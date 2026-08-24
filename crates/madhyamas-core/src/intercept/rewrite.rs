//! URL and header rewriting rules

use super::regex_cache;
use super::MatchCondition;
use crate::storage::InterceptStoreBackend;
use crate::traffic::{RequestData, ResponseData};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Direction for rewrite rules
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RewriteDirection {
    Request,
    Response,
    Both,
}

/// A rewrite rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteRule {
    /// Unique identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Condition to match
    pub condition: MatchCondition,
    /// Direction (request/response)
    pub direction: RewriteDirection,
    /// Rewrites to apply
    pub rewrites: Vec<RewriteAction>,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// When the rule was created
    pub created_at: DateTime<Utc>,
    /// Number of times this rule has been applied
    pub hit_count: u64,
}

impl RewriteRule {
    pub fn new(
        name: String,
        condition: MatchCondition,
        direction: RewriteDirection,
        rewrites: Vec<RewriteAction>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            condition,
            direction,
            rewrites,
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
            hit_count: 0,
        }
    }
}

/// Action to perform when rewriting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RewriteAction {
    /// Replace URL using regex
    UrlRewrite {
        pattern: String,
        replacement: String,
    },
    /// Add or set a header
    SetHeader { name: String, value: String },
    /// Remove a header
    RemoveHeader { name: String },
    /// Replace header value using regex
    HeaderRewrite {
        name: String,
        pattern: String,
        replacement: String,
    },
    /// Replace body using regex
    BodyRewrite {
        pattern: String,
        replacement: String,
    },
    /// Replace query parameter
    QueryParam { name: String, value: String },
    /// Remove query parameter
    RemoveQueryParam { name: String },
    /// Map to local file (for responses)
    MapToFile { path: String },
    /// Map to different URL (for requests)
    MapToUrl { url: String },
}

/// Manages rewrite rules
pub struct RewriteManager {
    /// Active rewrite rules
    rules: RwLock<Vec<RewriteRule>>,
    /// Optional SQLite persistence backend
    store: Option<Arc<dyn InterceptStoreBackend + Send + Sync>>,
}

impl RewriteManager {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            store: None,
        }
    }

    /// Attach a SQLite persistence backend.
    pub fn with_store(mut self, store: Arc<dyn InterceptStoreBackend + Send + Sync>) -> Self {
        self.store = Some(store);
        self
    }

    /// Add a rewrite rule
    pub async fn add_rule(&self, rule: RewriteRule) -> String {
        let id = rule.id.clone();
        if let Some(store) = &self.store {
            if let Err(e) = store.save_rewrite_rule(&rule).await {
                tracing::warn!("Failed to persist rewrite rule: {}", e);
            }
        }
        self.rules.write().push(rule);
        id
    }

    /// Remove a rewrite rule
    pub async fn remove_rule(&self, id: &str) -> bool {
        let removed = {
            let mut rules = self.rules.write();
            if let Some(pos) = rules.iter().position(|r| r.id == id) {
                rules.remove(pos);
                true
            } else {
                false
            }
        };
        if removed {
            if let Some(store) = &self.store {
                if let Err(e) = store.delete_rewrite_rule(id).await {
                    tracing::warn!("Failed to delete rewrite rule from store: {}", e);
                }
            }
        }
        removed
    }

    /// Get all rules
    pub fn get_rules(&self) -> Vec<RewriteRule> {
        self.rules.read().clone()
    }

    /// Get a specific rule
    pub fn get_rule(&self, id: &str) -> Option<RewriteRule> {
        self.rules.read().iter().find(|r| r.id == id).cloned()
    }

    /// Update a rule
    pub async fn update_rule(&self, id: &str, rule: RewriteRule) -> bool {
        let rule_to_save = {
            let mut rules = self.rules.write();
            if let Some(pos) = rules.iter().position(|r| r.id == id) {
                rules[pos] = rule.clone();
                Some(rule)
            } else {
                None
            }
        };
        if let Some(rule) = rule_to_save {
            if let Some(store) = &self.store {
                if let Err(e) = store.save_rewrite_rule(&rule).await {
                    tracing::warn!("Failed to persist rewrite rule update: {}", e);
                }
            }
            true
        } else {
            false
        }
    }

    /// Toggle a rule
    pub fn toggle_rule(&self, id: &str, enabled: bool) -> bool {
        let mut rules = self.rules.write();
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Apply rewrite rules to a request
    pub fn rewrite_request(&self, request: &mut RequestData) {
        let mut rules = self.rules.write();

        for rule in rules.iter_mut() {
            if !rule.enabled {
                continue;
            }
            if rule.direction != RewriteDirection::Request
                && rule.direction != RewriteDirection::Both
            {
                continue;
            }

            if !rule.condition.matches_request(
                &request.url,
                &request.method.to_string(),
                &request.headers,
                request.body.as_deref(),
                request.content_type.as_deref(),
            ) {
                continue;
            }

            rule.hit_count += 1;

            for action in &rule.rewrites {
                self.apply_request_action(request, action);
            }
        }
    }

    /// Apply rewrite rules to a response
    pub fn rewrite_response(&self, _request: &RequestData, response: &mut ResponseData) {
        let mut rules = self.rules.write();

        for rule in rules.iter_mut() {
            if !rule.enabled {
                continue;
            }
            if rule.direction != RewriteDirection::Response
                && rule.direction != RewriteDirection::Both
            {
                continue;
            }

            if !rule.condition.matches_response(
                response.status_code,
                &response.headers,
                response.body.as_deref(),
                response.content_type.as_deref(),
            ) {
                continue;
            }

            rule.hit_count += 1;

            for action in &rule.rewrites {
                self.apply_response_action(response, action);
            }
        }
    }

    fn apply_request_action(&self, request: &mut RequestData, action: &RewriteAction) {
        match action {
            RewriteAction::UrlRewrite {
                pattern,
                replacement,
            } => {
                let new_url = regex_cache::replace_all(pattern, &request.url, replacement);
                if new_url != request.url {
                    request.url = new_url;

                    // Update host and path
                    if let Ok(uri) = request.url.parse::<hyper::Uri>() {
                        if let Some(host) = uri.host() {
                            request.host = host.to_string();
                        }
                        if let Some(path) = uri.path_and_query() {
                            request.path = path.to_string();
                        }
                    }
                }
            }
            RewriteAction::SetHeader { name, value } => {
                request.headers.insert(name.clone(), value.clone());
            }
            RewriteAction::RemoveHeader { name } => {
                request.headers.remove(name);
            }
            RewriteAction::HeaderRewrite {
                name,
                pattern,
                replacement,
            } => {
                if let Some(value) = request.headers.get(name) {
                    let new_value = regex_cache::replace_all(pattern, value, replacement);
                    if new_value != *value {
                        request.headers.insert(name.clone(), new_value);
                    }
                }
            }
            RewriteAction::BodyRewrite {
                pattern,
                replacement,
            } => {
                if let Some(body) = &request.body {
                    if let Ok(body_str) = std::str::from_utf8(body) {
                        let new_body = regex_cache::replace_all(pattern, body_str, replacement);
                        if new_body != body_str {
                            request.body = Some(new_body.into_bytes());
                        }
                    }
                }
            }
            RewriteAction::QueryParam { name, value } => {
                // Add or replace query param
                let separator = if request.path.contains('?') { "&" } else { "?" };
                // Remove existing param if present
                let remove_pattern = format!(r"[?&]{}=[^&]*", regex::escape(name));
                let cleaned = regex_cache::replace_all(&remove_pattern, &request.path, "");
                request.path = format!("{}{}{}={}", cleaned, separator, name, value);
            }
            RewriteAction::RemoveQueryParam { name } => {
                let remove_pattern = format!(r"[?&]{}=[^&]*", regex::escape(name));
                request.path = regex_cache::replace_all(&remove_pattern, &request.path, "");
            }
            RewriteAction::MapToFile { .. } => {
                // Not applicable to requests
            }
            RewriteAction::MapToUrl { url } => {
                request.url = url.clone();
                if let Ok(uri) = url.parse::<hyper::Uri>() {
                    if let Some(host) = uri.host() {
                        request.host = host.to_string();
                    }
                    if let Some(path) = uri.path_and_query() {
                        request.path = path.to_string();
                    }
                }
            }
        }
    }

    fn apply_response_action(&self, response: &mut ResponseData, action: &RewriteAction) {
        match action {
            RewriteAction::SetHeader { name, value } => {
                response.headers.insert(name.clone(), value.clone());
            }
            RewriteAction::RemoveHeader { name } => {
                response.headers.remove(name);
            }
            RewriteAction::HeaderRewrite {
                name,
                pattern,
                replacement,
            } => {
                if let Some(value) = response.headers.get(name) {
                    let new_value = regex_cache::replace_all(pattern, value, replacement);
                    if new_value != *value {
                        response.headers.insert(name.clone(), new_value);
                    }
                }
            }
            RewriteAction::BodyRewrite {
                pattern,
                replacement,
            } => {
                if let Some(body) = &response.body {
                    if let Ok(body_str) = std::str::from_utf8(body) {
                        let new_body = regex_cache::replace_all(pattern, body_str, replacement);
                        if new_body != body_str {
                            response.body = Some(new_body.into_bytes());
                        }
                    }
                }
            }
            RewriteAction::MapToFile { path } => {
                if let Ok(content) = std::fs::read(path) {
                    response.body = Some(content);
                }
            }
            RewriteAction::UrlRewrite { .. }
            | RewriteAction::QueryParam { .. }
            | RewriteAction::RemoveQueryParam { .. }
            | RewriteAction::MapToUrl { .. } => {
                // Not applicable to responses
            }
        }
    }

    /// Clear all rules
    pub fn clear(&self) {
        self.rules.write().clear();
    }

    /// Import rules
    pub fn import_rules(&self, rules: Vec<RewriteRule>) {
        let mut current = self.rules.write();
        current.extend(rules);
    }

    /// Export rules
    pub fn export_rules(&self) -> Vec<RewriteRule> {
        self.rules.read().clone()
    }
}

impl Default for RewriteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::persistence::Persistable for RewriteManager {
    async fn save(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            let rules = self.rules.read().clone();
            for rule in &rules {
                store.save_rewrite_rule(rule).await?;
            }
        }
        Ok(())
    }

    async fn load(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            let loaded = store.load_rewrite_rules().await?;
            *self.rules.write() = loaded;
        }
        Ok(())
    }

    async fn clear(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            store.clear_rewrite_rules().await?;
        }
        self.rules.write().clear();
        Ok(())
    }

    fn size(&self) -> usize {
        self.rules.read().len()
    }
}

/// Common rewrite templates
pub struct RewriteTemplates;

impl RewriteTemplates {
    /// Redirect HTTP to HTTPS
    pub fn http_to_https() -> RewriteRule {
        RewriteRule::new(
            "HTTP to HTTPS".to_string(),
            MatchCondition::UrlPattern {
                pattern: r"^http://".to_string(),
            },
            RewriteDirection::Request,
            vec![RewriteAction::UrlRewrite {
                pattern: r"^http://".to_string(),
                replacement: "https://".to_string(),
            }],
        )
    }

    /// Add CORS headers to responses
    pub fn add_cors() -> RewriteRule {
        RewriteRule::new(
            "Add CORS Headers".to_string(),
            MatchCondition::All,
            RewriteDirection::Response,
            vec![
                RewriteAction::SetHeader {
                    name: "Access-Control-Allow-Origin".to_string(),
                    value: "*".to_string(),
                },
                RewriteAction::SetHeader {
                    name: "Access-Control-Allow-Methods".to_string(),
                    value: "GET, POST, PUT, DELETE, OPTIONS".to_string(),
                },
                RewriteAction::SetHeader {
                    name: "Access-Control-Allow-Headers".to_string(),
                    value: "*".to_string(),
                },
            ],
        )
    }

    /// Remove security headers (for testing)
    pub fn remove_security_headers() -> RewriteRule {
        RewriteRule::new(
            "Remove Security Headers".to_string(),
            MatchCondition::All,
            RewriteDirection::Response,
            vec![
                RewriteAction::RemoveHeader {
                    name: "Content-Security-Policy".to_string(),
                },
                RewriteAction::RemoveHeader {
                    name: "X-Frame-Options".to_string(),
                },
                RewriteAction::RemoveHeader {
                    name: "X-Content-Type-Options".to_string(),
                },
            ],
        )
    }

    /// Add authentication header
    pub fn add_auth_header(token: &str) -> RewriteRule {
        RewriteRule::new(
            "Add Auth Header".to_string(),
            MatchCondition::All,
            RewriteDirection::Request,
            vec![RewriteAction::SetHeader {
                name: "Authorization".to_string(),
                value: format!("Bearer {}", token),
            }],
        )
    }

    /// No Caching — prevent client and intermediary caching so that every
    /// request through the proxy always reaches the upstream server and
    /// returns the latest response.
    ///
    /// On **requests** the conditional-request headers are stripped so the
    /// server cannot answer `304 Not Modified`:
    /// - `If-Modified-Since`
    /// - `If-None-Match`
    ///
    /// On **responses** the caching headers are removed and explicit
    /// no-cache directives are added so the browser/proxy never serves a
    /// stale copy:
    /// - Remove `ETag`, `Last-Modified`, `Expires`
    /// - Set `Cache-Control: no-cache, no-store, must-revalidate`
    /// - Set `Pragma: no-cache`
    /// - Set `Expires: 0`
    pub fn no_caching() -> RewriteRule {
        RewriteRule::new(
            "No Caching".to_string(),
            MatchCondition::All,
            RewriteDirection::Both,
            vec![
                // Request: remove conditional request headers so the server
                // cannot return a 304 Not Modified.
                RewriteAction::RemoveHeader {
                    name: "If-Modified-Since".to_string(),
                },
                RewriteAction::RemoveHeader {
                    name: "If-None-Match".to_string(),
                },
                // Response: remove validators and expiration hints.
                RewriteAction::RemoveHeader {
                    name: "ETag".to_string(),
                },
                RewriteAction::RemoveHeader {
                    name: "Last-Modified".to_string(),
                },
                RewriteAction::RemoveHeader {
                    name: "Expires".to_string(),
                },
                // Response: add explicit no-cache directives.
                RewriteAction::SetHeader {
                    name: "Cache-Control".to_string(),
                    value: "no-cache, no-store, must-revalidate".to_string(),
                },
                RewriteAction::SetHeader {
                    name: "Pragma".to_string(),
                    value: "no-cache".to_string(),
                },
                RewriteAction::SetHeader {
                    name: "Expires".to_string(),
                    value: "0".to_string(),
                },
            ],
        )
    }

    /// Block Cookies — strip cookies from both directions of traffic so
    /// that the client never sends cookies and the server never sets them.
    ///
    /// On **requests** the `Cookie` header is removed, so the upstream
    /// server sees an unauthenticated, cookieless request.
    ///
    /// On **responses** the `Set-Cookie` header is removed, so the client
    /// never stores any cookies returned by the server.
    ///
    /// This is useful for testing how an application behaves for first-
    /// time/anonymous visitors, debugging login flows, or verifying that
    /// a site degrades gracefully without cookies.
    pub fn block_cookies() -> RewriteRule {
        RewriteRule::new(
            "Block Cookies".to_string(),
            MatchCondition::All,
            RewriteDirection::Both,
            vec![
                // Request: prevent the client from sending cookies.
                RewriteAction::RemoveHeader {
                    name: "Cookie".to_string(),
                },
                // Response: prevent the server from setting cookies.
                RewriteAction::RemoveHeader {
                    name: "Set-Cookie".to_string(),
                },
            ],
        )
    }
}

//! URL and header rewriting rules

use super::MatchCondition;
use crate::traffic::{RequestData, ResponseData};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
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
}

impl RewriteManager {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
        }
    }

    /// Add a rewrite rule
    pub fn add_rule(&self, rule: RewriteRule) -> String {
        let id = rule.id.clone();
        self.rules.write().push(rule);
        id
    }

    /// Remove a rewrite rule
    pub fn remove_rule(&self, id: &str) -> bool {
        let mut rules = self.rules.write();
        if let Some(pos) = rules.iter().position(|r| r.id == id) {
            rules.remove(pos);
            true
        } else {
            false
        }
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
    pub fn update_rule(&self, id: &str, rule: RewriteRule) -> bool {
        let mut rules = self.rules.write();
        if let Some(pos) = rules.iter().position(|r| r.id == id) {
            rules[pos] = rule;
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
                if let Ok(re) = regex::Regex::new(pattern) {
                    let new_url = re.replace_all(&request.url, replacement.as_str());
                    request.url = new_url.to_string();

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
                    if let Ok(re) = regex::Regex::new(pattern) {
                        let new_value = re.replace_all(value, replacement.as_str());
                        request.headers.insert(name.clone(), new_value.to_string());
                    }
                }
            }
            RewriteAction::BodyRewrite {
                pattern,
                replacement,
            } => {
                if let Some(body) = &request.body {
                    if let Ok(body_str) = std::str::from_utf8(body) {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            let new_body = re.replace_all(body_str, replacement.as_str());
                            request.body = Some(new_body.as_bytes().to_vec());
                        }
                    }
                }
            }
            RewriteAction::QueryParam { name, value } => {
                // Add or replace query param
                let separator = if request.path.contains('?') { "&" } else { "?" };
                // Remove existing param if present
                if let Ok(re) = regex::Regex::new(&format!(r"[?&]{}=[^&]*", regex::escape(name))) {
                    request.path = re.replace(&request.path, "").to_string();
                }
                request.path = format!("{}{}{}={}", request.path, separator, name, value);
            }
            RewriteAction::RemoveQueryParam { name } => {
                if let Ok(re) = regex::Regex::new(&format!(r"[?&]{}=[^&]*", regex::escape(name))) {
                    request.path = re.replace(&request.path, "").to_string();
                }
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
                    if let Ok(re) = regex::Regex::new(pattern) {
                        let new_value = re.replace_all(value, replacement.as_str());
                        response.headers.insert(name.clone(), new_value.to_string());
                    }
                }
            }
            RewriteAction::BodyRewrite {
                pattern,
                replacement,
            } => {
                if let Some(body) = &response.body {
                    if let Ok(body_str) = std::str::from_utf8(body) {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            let new_body = re.replace_all(body_str, replacement.as_str());
                            response.body = Some(new_body.as_bytes().to_vec());
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
}

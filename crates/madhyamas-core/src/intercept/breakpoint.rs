//! Breakpoint management for pausing and modifying traffic

use super::regex_cache;
use super::{InterceptDirection, MatchCondition, Modification};
use crate::storage::InterceptStoreBackend;
use crate::traffic::{RequestData, ResponseData};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use uuid::Uuid;

/// A breakpoint rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointRule {
    /// Unique identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Condition to match
    pub condition: MatchCondition,
    /// Direction (request/response)
    pub direction: InterceptDirection,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Order/priority (lower = higher priority)
    pub priority: u32,
}

impl BreakpointRule {
    pub fn new(name: String, condition: MatchCondition, direction: InterceptDirection) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            condition,
            direction,
            enabled: true,
            priority: 100,
        }
    }
}

/// State of a paused request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PausedTraffic {
    /// Unique ID for this paused item
    pub id: String,
    /// Traffic entry ID
    pub entry_id: String,
    /// Whether this is a request or response
    pub direction: InterceptDirection,
    /// The request data
    pub request: RequestData,
    /// The response data (if direction is Response)
    pub response: Option<ResponseData>,
    /// When it was paused
    pub paused_at: DateTime<Utc>,
    /// Rule that triggered the breakpoint
    pub rule_id: String,
    /// Current modifications applied
    pub modifications: Vec<Modification>,
}

/// Decision from user about a paused item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BreakpointDecision {
    /// Allow the traffic to continue
    Continue,
    /// Continue with modifications
    Modify { modifications: Vec<Modification> },
    /// Abort the request (return error)
    Abort,
    /// Respond with custom response
    Respond {
        status_code: u16,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
}

/// State of a breakpoint (waiting for user input)
pub struct BreakpointState {
    /// The paused traffic
    pub traffic: PausedTraffic,
    /// Channel to send the decision back
    pub tx: oneshot::Sender<BreakpointDecision>,
}

/// Manages breakpoints and paused traffic
pub struct BreakpointManager {
    /// Active breakpoint rules
    rules: RwLock<Vec<BreakpointRule>>,
    /// Currently paused traffic waiting for decision
    paused: RwLock<HashMap<String, BreakpointState>>,
    /// Maximum number of paused items
    max_paused: usize,
    /// Optional SQLite persistence backend
    store: Option<Arc<dyn InterceptStoreBackend + Send + Sync>>,
}

impl BreakpointManager {
    pub fn new(max_paused: usize) -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            paused: RwLock::new(HashMap::new()),
            max_paused,
            store: None,
        }
    }

    /// Attach a SQLite persistence backend.
    pub fn with_store(mut self, store: Arc<dyn InterceptStoreBackend + Send + Sync>) -> Self {
        self.store = Some(store);
        self
    }

    /// Add a breakpoint rule
    pub async fn add_rule(&self, rule: BreakpointRule) -> String {
        let id = rule.id.clone();
        if let Some(store) = &self.store {
            if let Err(e) = store.save_breakpoint_rule(&rule).await {
                tracing::warn!("Failed to persist breakpoint rule: {}", e);
            }
        }
        self.rules.write().push(rule);
        id
    }

    /// Remove a breakpoint rule
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
                if let Err(e) = store.delete_breakpoint_rule(id).await {
                    tracing::warn!("Failed to delete breakpoint rule from store: {}", e);
                }
            }
        }
        removed
    }

    /// Update a rule
    pub fn update_rule(&self, id: &str, rule: BreakpointRule) -> bool {
        let mut rules = self.rules.write();
        if let Some(pos) = rules.iter().position(|r| r.id == id) {
            rules[pos] = rule;
            true
        } else {
            false
        }
    }

    /// Check if a request should be breakpointed
    pub fn check_request(&self, request: &RequestData) -> Option<BreakpointRule> {
        let rules = self.rules.read();
        rules
            .iter()
            .filter(|r| {
                r.enabled
                    && (r.direction == InterceptDirection::Request
                        || r.direction == InterceptDirection::Both)
            })
            .find(|r| {
                r.condition.matches_request(
                    &request.url,
                    &request.method.to_string(),
                    &request.headers,
                    request.body.as_deref(),
                    request.content_type.as_deref(),
                )
            })
            .cloned()
    }

    /// Check if a response matches any breakpoint
    pub fn check_response(
        &self,
        _request: &RequestData,
        response: &ResponseData,
    ) -> Option<BreakpointRule> {
        let rules = self.rules.read();
        rules
            .iter()
            .filter(|r| {
                r.enabled
                    && (r.direction == InterceptDirection::Response
                        || r.direction == InterceptDirection::Both)
            })
            .find(|r| {
                r.condition.matches_response(
                    response.status_code,
                    &response.headers,
                    response.body.as_deref(),
                    response.content_type.as_deref(),
                )
            })
            .cloned()
    }

    /// Pause traffic and wait for decision
    pub async fn pause_and_wait(
        &self,
        entry_id: String,
        direction: InterceptDirection,
        request: RequestData,
        response: Option<ResponseData>,
        rule_id: String,
    ) -> BreakpointDecision {
        let (tx, rx) = oneshot::channel();

        let paused = PausedTraffic {
            id: Uuid::new_v4().to_string(),
            entry_id,
            direction,
            request,
            response,
            paused_at: Utc::now(),
            rule_id,
            modifications: Vec::new(),
        };

        let state = BreakpointState {
            traffic: paused,
            tx,
        };

        let paused_id = state.traffic.id.clone();

        // Store the paused state
        {
            let mut paused_map = self.paused.write();

            // Check if we're at capacity
            if paused_map.len() >= self.max_paused {
                // Remove the oldest one (just drop it - this will cancel that request)
                if let Some((_, oldest)) = paused_map.iter().next() {
                    let id_to_remove = oldest.traffic.id.clone();
                    let _ = oldest;
                    paused_map.remove(&id_to_remove);
                }
            }

            paused_map.insert(paused_id.clone(), state);
        }

        // Wait for decision
        match rx.await {
            Ok(decision) => decision,
            Err(_) => BreakpointDecision::Continue, // Default to continue if channel closed
        }
    }

    /// Get all paused traffic
    pub fn get_paused(&self) -> Vec<PausedTraffic> {
        self.paused
            .read()
            .values()
            .map(|s| s.traffic.clone())
            .collect()
    }

    /// Get a specific paused item
    pub fn get_paused_by_id(&self, id: &str) -> Option<PausedTraffic> {
        self.paused.read().get(id).map(|s| s.traffic.clone())
    }

    /// Resume a paused item with a decision
    pub fn resume(&self, id: &str, decision: BreakpointDecision) -> bool {
        let mut paused = self.paused.write();
        if let Some(state) = paused.remove(id) {
            let _ = state.tx.send(decision);
            true
        } else {
            false
        }
    }

    /// Apply modifications to request
    pub fn apply_request_modifications(request: &mut RequestData, modifications: &[Modification]) {
        for mod_ in modifications {
            match mod_ {
                Modification::SetHeader { name, value } => {
                    request.headers.insert(name.clone(), value.clone());
                }
                Modification::RemoveHeader { name } => {
                    request.headers.remove(name);
                }
                Modification::SetBody { content } => {
                    request.body = Some(content.as_bytes().to_vec());
                }
                Modification::SetBodyBase64 { content } => {
                    if let Ok(decoded) =
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content)
                    {
                        request.body = Some(decoded);
                    }
                }
                Modification::SetUrl { url } => {
                    request.url = url.clone();
                    // Update host and path from URL
                    if let Ok(uri) = url.parse::<hyper::Uri>() {
                        if let Some(host) = uri.host() {
                            request.host = host.to_string();
                        }
                        if let Some(path) = uri.path_and_query() {
                            request.path = path.to_string();
                        }
                    }
                }
                Modification::SetPath { path } => {
                    request.path = path.clone();
                }
                Modification::RegexReplace {
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
                Modification::UrlRegexReplace {
                    pattern,
                    replacement,
                } => {
                    let new_url = regex_cache::replace_all(pattern, &request.url, replacement);
                    if new_url != request.url {
                        request.url = new_url;
                    }
                }
                Modification::Delay { .. } | Modification::SetStatusCode { .. } => {
                    // Not applicable to requests
                }
            }
        }
    }

    /// Apply modifications to response
    pub fn apply_response_modifications(
        response: &mut ResponseData,
        modifications: &[Modification],
    ) {
        for mod_ in modifications {
            match mod_ {
                Modification::SetHeader { name, value } => {
                    response.headers.insert(name.clone(), value.clone());
                }
                Modification::RemoveHeader { name } => {
                    response.headers.remove(name);
                }
                Modification::SetBody { content } => {
                    response.body = Some(content.as_bytes().to_vec());
                }
                Modification::SetBodyBase64 { content } => {
                    if let Ok(decoded) =
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content)
                    {
                        response.body = Some(decoded);
                    }
                }
                Modification::SetStatusCode { code } => {
                    response.status_code = *code;
                }
                Modification::RegexReplace {
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
                Modification::SetUrl { .. }
                | Modification::SetPath { .. }
                | Modification::UrlRegexReplace { .. } => {
                    // Not applicable to responses
                }
                Modification::Delay { .. } => {
                    // Handled separately
                }
            }
        }
    }
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self::new(100)
    }
}

impl BreakpointManager {
    /// Get all rules
    pub fn get_rules(&self) -> Vec<BreakpointRule> {
        self.rules.read().clone()
    }

    /// Clear all rules
    pub fn clear(&self) {
        self.rules.write().clear();
    }
}

#[async_trait::async_trait]
impl crate::persistence::Persistable for BreakpointManager {
    async fn save(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            let rules = self.rules.read().clone();
            for rule in &rules {
                store.save_breakpoint_rule(rule).await?;
            }
        }
        Ok(())
    }

    async fn load(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            let loaded = store.load_breakpoint_rules().await?;
            *self.rules.write() = loaded;
        }
        Ok(())
    }

    async fn clear(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            store.clear_breakpoint_rules().await?;
        }
        self.rules.write().clear();
        Ok(())
    }

    fn size(&self) -> usize {
        self.rules.read().len()
    }
}

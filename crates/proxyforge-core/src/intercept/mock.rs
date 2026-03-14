//! Response mocking - serve custom responses for matched requests

use super::MatchCondition;
use crate::traffic::RequestData;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A mock response rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRule {
    /// Unique identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Condition to match requests
    pub condition: MatchCondition,
    /// Response to serve
    pub response: MockResponse,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// When the rule was created
    pub created_at: DateTime<Utc>,
    /// Number of times this mock has been used
    pub hit_count: u64,
}

impl MockRule {
    pub fn new(name: String, condition: MatchCondition, response: MockResponse) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            condition,
            response,
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
            hit_count: 0,
        }
    }
}

/// Mock response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (as string)
    pub body: Option<String>,
    /// Response body (base64 encoded binary)
    pub body_base64: Option<String>,
    /// Delay in milliseconds before responding
    pub delay_ms: Option<u64>,
    /// Load body from file
    pub body_file: Option<String>,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status_code: 200,
            headers: HashMap::new(),
            body: None,
            body_base64: None,
            delay_ms: None,
            body_file: None,
        }
    }
}

impl MockResponse {
    /// Get the response body as bytes
    pub fn body_bytes(&self) -> Option<Vec<u8>> {
        if let Some(body) = &self.body {
            Some(body.as_bytes().to_vec())
        } else if let Some(b64) = &self.body_base64 {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).ok()
        } else if let Some(path) = &self.body_file {
            std::fs::read(path).ok()
        } else {
            None
        }
    }
}

/// Manages mock responses
pub struct MockManager {
    /// Active mock rules
    rules: RwLock<Vec<MockRule>>,
}

impl MockManager {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
        }
    }

    /// Add a mock rule
    pub fn add_rule(&self, rule: MockRule) -> String {
        let id = rule.id.clone();
        self.rules.write().push(rule);
        id
    }

    /// Remove a mock rule
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
    pub fn get_rules(&self) -> Vec<MockRule> {
        self.rules.read().clone()
    }

    /// Get a specific rule
    pub fn get_rule(&self, id: &str) -> Option<MockRule> {
        self.rules.read().iter().find(|r| r.id == id).cloned()
    }

    /// Update a rule
    pub fn update_rule(&self, id: &str, rule: MockRule) -> bool {
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

    /// Check if a request matches any mock rule
    pub fn find_matching_mock(&self, request: &RequestData) -> Option<MockRule> {
        let mut rules = self.rules.write();
        let matching = rules
            .iter_mut()
            .filter(|r| r.enabled)
            .filter(|r| {
                r.condition.matches_request(
                    &request.url,
                    &request.method.to_string(),
                    &request.headers,
                    request.body.as_deref(),
                    request.content_type.as_deref(),
                )
            })
            .min_by_key(|r| r.priority);

        if let Some(rule) = matching {
            rule.hit_count += 1;
            Some(rule.clone())
        } else {
            None
        }
    }

    /// Clear all rules
    pub fn clear(&self) {
        self.rules.write().clear();
    }

    /// Import rules from JSON
    pub fn import_rules(&self, rules: Vec<MockRule>) {
        let mut current = self.rules.write();
        current.extend(rules);
    }

    /// Export rules to JSON
    pub fn export_rules(&self) -> Vec<MockRule> {
        self.rules.read().clone()
    }
}

impl Default for MockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-built mock templates
pub struct MockTemplates;

impl MockTemplates {
    /// Create a simple JSON response mock
    pub fn json_response(name: &str, url_pattern: &str, json: &str) -> MockRule {
        MockRule::new(
            name.to_string(),
            MatchCondition::UrlPattern {
                pattern: url_pattern.to_string(),
            },
            MockResponse {
                status_code: 200,
                headers: HashMap::from([(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )]),
                body: Some(json.to_string()),
                ..Default::default()
            },
        )
    }

    /// Create an error response mock
    pub fn error_response(
        name: &str,
        url_pattern: &str,
        status_code: u16,
        message: &str,
    ) -> MockRule {
        MockRule::new(
            name.to_string(),
            MatchCondition::UrlPattern {
                pattern: url_pattern.to_string(),
            },
            MockResponse {
                status_code,
                headers: HashMap::from([(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )]),
                body: Some(format!(r#"{{"error": "{}"}}"#, message)),
                ..Default::default()
            },
        )
    }

    /// Create a slow response mock
    pub fn slow_response(name: &str, url_pattern: &str, delay_ms: u64) -> MockRule {
        MockRule::new(
            name.to_string(),
            MatchCondition::UrlPattern {
                pattern: url_pattern.to_string(),
            },
            MockResponse {
                status_code: 200,
                delay_ms: Some(delay_ms),
                body: Some(r#"{"simulated": true}"#.to_string()),
                ..Default::default()
            },
        )
    }

    /// Create a 404 response mock
    pub fn not_found(name: &str, url_pattern: &str) -> MockRule {
        Self::error_response(name, url_pattern, 404, "Not Found")
    }

    /// Create a 500 response mock
    pub fn server_error(name: &str, url_pattern: &str) -> MockRule {
        Self::error_response(name, url_pattern, 500, "Internal Server Error")
    }

    /// Create a timeout mock (very long delay)
    pub fn timeout(name: &str, url_pattern: &str) -> MockRule {
        Self::slow_response(name, url_pattern, 30000) // 30 second delay
    }
}

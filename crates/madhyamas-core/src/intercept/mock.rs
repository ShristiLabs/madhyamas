//! Response mocking - serve custom responses for matched requests
//!
//! This module provides comprehensive mock response capabilities including:
//! - Dynamic response generation with template variables
//! - Response sequencing and scenarios
//! - Conditional response selection
//! - Probability-based responses
//! - Delay variance/jitter
//! - Mock collections/groups
//! - Response validation
//! - Hit analytics with history
//! - Scripting support

use super::MatchCondition;
use crate::traffic::RequestData;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Core Types
// ============================================================================

/// A mock response rule with full feature support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRule {
    /// Unique identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Description/documentation for the mock
    #[serde(default)]
    pub description: Option<String>,
    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Collection/group this mock belongs to
    #[serde(default)]
    pub collection_id: Option<String>,
    /// Condition to match requests
    pub condition: MatchCondition,
    /// Response configuration (single, sequence, or conditional)
    pub response_config: ResponseConfig,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// When the rule was created
    pub created_at: DateTime<Utc>,
    /// When the rule was last modified
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Number of times this mock has been used
    pub hit_count: u64,
    /// Expiration configuration
    #[serde(default)]
    pub expiration: Option<MockExpiration>,
    /// Version number for tracking changes
    #[serde(default = "default_version")]
    pub version: u32,
    /// Previous versions (for rollback)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_history: Vec<MockRuleVersion>,
    /// JSON Schema for response validation
    #[serde(default)]
    pub response_schema: Option<String>,
    /// Script to execute for dynamic response generation
    #[serde(default)]
    pub response_script: Option<String>,
}

fn default_version() -> u32 {
    1
}

/// Versioned snapshot of a mock rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRuleVersion {
    pub version: u32,
    pub response_config: ResponseConfig,
    pub condition: MatchCondition,
    pub saved_at: DateTime<Utc>,
    pub comment: Option<String>,
}

/// Expiration configuration for mocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockExpiration {
    /// Expire after a specific date/time
    DateTime { expires_at: DateTime<Utc> },
    /// Expire after N hits
    HitCount { max_hits: u64 },
    /// Expire after duration from creation
    Duration { duration_secs: u64 },
}

impl MockExpiration {
    pub fn is_expired(&self, rule: &MockRule) -> bool {
        match self {
            MockExpiration::DateTime { expires_at } => Utc::now() > *expires_at,
            MockExpiration::HitCount { max_hits } => rule.hit_count >= *max_hits,
            MockExpiration::Duration { duration_secs } => {
                let elapsed = Utc::now()
                    .signed_duration_since(rule.created_at)
                    .num_seconds();
                elapsed as u64 >= *duration_secs
            }
        }
    }
}

/// Response configuration - supports single, sequence, conditional, and probabilistic responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseConfig {
    /// Single static response
    Single { response: MockResponse },
    /// Sequence of responses (cycles through)
    Sequence {
        responses: Vec<MockResponse>,
        #[serde(default)]
        current_index: usize,
        /// Whether to cycle back to start or stop at last
        #[serde(default = "default_true")]
        cycle: bool,
    },
    /// Conditional responses based on request properties
    Conditional {
        conditions: Vec<ConditionalResponse>,
        default_response: MockResponse,
    },
    /// Probability-weighted responses
    Probabilistic {
        responses: Vec<ProbabilisticResponse>,
    },
}

fn default_true() -> bool {
    true
}

impl Default for ResponseConfig {
    fn default() -> Self {
        ResponseConfig::Single {
            response: MockResponse::default(),
        }
    }
}

/// A response with a condition for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalResponse {
    /// Condition to match (evaluated against request)
    pub condition: RequestCondition,
    /// Response to serve if condition matches
    pub response: MockResponse,
}

/// Condition evaluated against request data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestCondition {
    /// Match header value
    HeaderEquals { name: String, value: String },
    /// Match header with regex
    HeaderMatches { name: String, pattern: String },
    /// Match query parameter
    QueryParamEquals { name: String, value: String },
    /// Match JSON body path (using JSONPath syntax)
    JsonPathEquals {
        path: String,
        value: serde_json::Value,
    },
    /// Match JSON body path with regex
    JsonPathMatches { path: String, pattern: String },
    /// Match request body with regex
    BodyMatches { pattern: String },
    /// Combine conditions with AND
    And { conditions: Vec<RequestCondition> },
    /// Combine conditions with OR
    Or { conditions: Vec<RequestCondition> },
}

impl RequestCondition {
    /// Evaluate the condition against a request
    pub fn matches(&self, request: &RequestData) -> bool {
        match self {
            RequestCondition::HeaderEquals { name, value } => request
                .headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case(name) && v == value),
            RequestCondition::HeaderMatches { name, pattern } => {
                if let Ok(re) = Regex::new(pattern) {
                    request
                        .headers
                        .iter()
                        .any(|(k, v)| k.eq_ignore_ascii_case(name) && re.is_match(v))
                } else {
                    false
                }
            }
            RequestCondition::QueryParamEquals { name, value } => {
                if let Ok(url) = url::Url::parse(&request.url) {
                    url.query_pairs().any(|(k, v)| k == *name && v == *value)
                } else {
                    false
                }
            }
            RequestCondition::JsonPathEquals { path, value } => {
                if let Some(body) = &request.body {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                        match jsonpath_lib::select(&json, path) {
                            Ok(results) => results.contains(&value),
                            Err(_) => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            RequestCondition::JsonPathMatches { path, pattern } => {
                if let Some(body) = &request.body {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                        if let Ok(re) = Regex::new(pattern) {
                            match jsonpath_lib::select(&json, path) {
                                Ok(results) => results.iter().any(|r| {
                                    if let Some(s) = r.as_str() {
                                        re.is_match(s)
                                    } else {
                                        re.is_match(&r.to_string())
                                    }
                                }),
                                Err(_) => false,
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            RequestCondition::BodyMatches { pattern } => {
                if let Some(body) = &request.body {
                    if let Ok(body_str) = std::str::from_utf8(body) {
                        Regex::new(pattern)
                            .map(|re| re.is_match(body_str))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            RequestCondition::And { conditions } => conditions.iter().all(|c| c.matches(request)),
            RequestCondition::Or { conditions } => conditions.iter().any(|c| c.matches(request)),
        }
    }
}

/// A response with probability weight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilisticResponse {
    /// Weight (relative probability)
    pub weight: u32,
    /// Response to serve
    pub response: MockResponse,
}

impl MockRule {
    pub fn new(name: String, condition: MatchCondition, response: MockResponse) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            tags: Vec::new(),
            collection_id: None,
            condition,
            response_config: ResponseConfig::Single { response },
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            hit_count: 0,
            expiration: None,
            version: 1,
            version_history: Vec::new(),
            response_schema: None,
            response_script: None,
        }
    }

    /// Create a new mock with response config
    pub fn with_config(
        name: String,
        condition: MatchCondition,
        response_config: ResponseConfig,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            tags: Vec::new(),
            collection_id: None,
            condition,
            response_config,
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            hit_count: 0,
            expiration: None,
            version: 1,
            version_history: Vec::new(),
            response_schema: None,
            response_script: None,
        }
    }

    /// Check if this mock has expired
    pub fn is_expired(&self) -> bool {
        self.expiration
            .as_ref()
            .map(|e| e.is_expired(self))
            .unwrap_or(false)
    }

    /// Save current state to version history
    pub fn save_version(&mut self, comment: Option<String>) {
        let version_snapshot = MockRuleVersion {
            version: self.version,
            response_config: self.response_config.clone(),
            condition: self.condition.clone(),
            saved_at: Utc::now(),
            comment,
        };
        self.version_history.push(version_snapshot);
        self.version += 1;
        self.updated_at = Utc::now();
    }

    /// Rollback to a previous version
    pub fn rollback_to_version(&mut self, version: u32) -> bool {
        if let Some(snapshot) = self.version_history.iter().find(|v| v.version == version) {
            self.response_config = snapshot.response_config.clone();
            self.condition = snapshot.condition.clone();
            self.version += 1;
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Get the legacy response field for backward compatibility
    pub fn response(&self) -> MockResponse {
        match &self.response_config {
            ResponseConfig::Single { response } => response.clone(),
            ResponseConfig::Sequence { responses, .. } => {
                responses.first().cloned().unwrap_or_default()
            }
            ResponseConfig::Conditional {
                default_response, ..
            } => default_response.clone(),
            ResponseConfig::Probabilistic { responses } => responses
                .first()
                .map(|r| r.response.clone())
                .unwrap_or_default(),
        }
    }
}

/// Mock response configuration with full feature support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers (supports template variables)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body (as string, supports template variables)
    #[serde(default)]
    pub body: Option<String>,
    /// Response body (base64 encoded binary)
    #[serde(default)]
    pub body_base64: Option<String>,
    /// Delay in milliseconds before responding
    #[serde(default)]
    pub delay_ms: Option<u64>,
    /// Delay variance/jitter in milliseconds (actual delay = delay_ms ± variance)
    #[serde(default)]
    pub delay_variance_ms: Option<u64>,
    /// Load body from file
    #[serde(default)]
    pub body_file: Option<String>,
    /// Enable template variable processing
    #[serde(default)]
    pub template_enabled: bool,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status_code: 200,
            headers: HashMap::new(),
            body: None,
            body_base64: None,
            delay_ms: None,
            delay_variance_ms: None,
            body_file: None,
            template_enabled: false,
        }
    }
}

impl MockResponse {
    /// Get the response body as bytes (without template processing)
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

    /// Get the actual delay with variance applied
    pub fn actual_delay_ms(&self) -> Option<u64> {
        self.delay_ms.map(|base| {
            if let Some(variance) = self.delay_variance_ms {
                let jitter = rand::rng().random_range(0..=variance * 2) as i64 - variance as i64;
                (base as i64 + jitter).max(0) as u64
            } else {
                base
            }
        })
    }

    /// Process template variables in body and headers
    pub fn process_template(&self, request: &RequestData) -> MockResponse {
        if !self.template_enabled {
            return self.clone();
        }

        let mut result = self.clone();

        // Process body template
        if let Some(body) = &self.body {
            result.body = Some(TemplateEngine::process(body, request));
        }

        // Process header templates
        result.headers = self
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), TemplateEngine::process(v, request)))
            .collect();

        result
    }
}

// ============================================================================
// Template Engine
// ============================================================================

/// Template engine for dynamic response generation
pub struct TemplateEngine;

impl TemplateEngine {
    /// Process template variables in a string
    pub fn process(template: &str, request: &RequestData) -> String {
        let mut result = template.to_string();

        // Built-in variables
        result = result.replace("{{timestamp}}", &Utc::now().timestamp().to_string());
        result = result.replace(
            "{{timestamp_ms}}",
            &Utc::now().timestamp_millis().to_string(),
        );
        result = result.replace("{{uuid}}", &Uuid::new_v4().to_string());
        result = result.replace("{{date}}", &Utc::now().format("%Y-%m-%d").to_string());
        result = result.replace("{{time}}", &Utc::now().format("%H:%M:%S").to_string());
        result = result.replace(
            "{{datetime}}",
            &Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        );

        // Random values
        let mut rng = rand::rng();
        result = result.replace("{{random_int}}", &rng.random_range(0..1000000).to_string());
        result = result.replace(
            "{{random_float}}",
            &format!("{:.2}", rng.random_range(0.0..1.0)),
        );

        // Request properties
        result = result.replace("{{request.method}}", &request.method.to_string());
        result = result.replace("{{request.url}}", &request.url);
        if let Ok(url) = url::Url::parse(&request.url) {
            result = result.replace("{{request.path}}", url.path());
            result = result.replace("{{request.host}}", url.host_str().unwrap_or(""));
            result = result.replace("{{request.query}}", url.query().unwrap_or(""));
        }

        // Request headers: {{request.headers.X-Custom}}
        let header_re = Regex::new(r"\{\{request\.headers\.([^}]+)\}\}").unwrap();
        result = header_re
            .replace_all(&result, |caps: &regex::Captures| {
                let header_name = &caps[1];
                request
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(header_name))
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            })
            .to_string();

        // Query parameters: {{request.query.param_name}}
        let query_re = Regex::new(r"\{\{request\.query\.([^}]+)\}\}").unwrap();
        if let Ok(url) = url::Url::parse(&request.url) {
            result = query_re
                .replace_all(&result, |caps: &regex::Captures| {
                    let param_name = &caps[1];
                    url.query_pairs()
                        .find(|(k, _)| k == param_name)
                        .map(|(_, v)| v.to_string())
                        .unwrap_or_default()
                })
                .to_string();
        }

        // JSON body paths: {{request.body.path.to.value}}
        let body_re = Regex::new(r"\{\{request\.body\.([^}]+)\}\}").unwrap();
        if let Some(body) = &request.body {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                result = body_re
                    .replace_all(&result, |caps: &regex::Captures| {
                        let path = format!("$.{}", &caps[1]);
                        match jsonpath_lib::select(&json, &path) {
                            Ok(values) if !values.is_empty() => {
                                if let Some(s) = values[0].as_str() {
                                    s.to_string()
                                } else {
                                    values[0].to_string()
                                }
                            }
                            _ => String::new(),
                        }
                    })
                    .to_string();
            }
        }

        result
    }
}

// ============================================================================
// Mock Collection
// ============================================================================

/// A collection/group of mock rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockCollection {
    /// Unique identifier
    pub id: String,
    /// Collection name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether all mocks in this collection are enabled
    pub enabled: bool,
    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
    /// When the collection was created
    pub created_at: DateTime<Utc>,
    /// Color for UI display
    #[serde(default)]
    pub color: Option<String>,
}

impl MockCollection {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            enabled: true,
            tags: Vec::new(),
            created_at: Utc::now(),
            color: None,
        }
    }
}

// ============================================================================
// Hit Analytics
// ============================================================================

/// Record of a mock hit for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockHitRecord {
    /// Mock rule ID
    pub mock_id: String,
    /// When the hit occurred
    pub timestamp: DateTime<Utc>,
    /// Request URL
    pub request_url: String,
    /// Request method
    pub request_method: String,
    /// Response status code served
    pub response_status: u16,
    /// Response time in ms
    pub response_time_ms: u64,
    /// Which response was selected (for sequences/conditional)
    #[serde(default)]
    pub response_index: Option<usize>,
}

// ============================================================================
// Mock Manager
// ============================================================================

/// Manages mock responses with full feature support
pub struct MockManager {
    /// Active mock rules
    rules: RwLock<Vec<MockRule>>,
    /// Mock collections
    collections: RwLock<Vec<MockCollection>>,
    /// Hit history for analytics
    hit_history: RwLock<Vec<MockHitRecord>>,
    /// Maximum hit history entries to keep
    max_history_size: usize,
    /// Sequence indices for each rule (for sequence responses)
    sequence_indices: RwLock<HashMap<String, usize>>,
    /// Recording mode enabled
    recording_enabled: RwLock<bool>,
    /// Recorded mocks from live traffic
    recorded_mocks: RwLock<Vec<MockRule>>,
}

impl MockManager {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            collections: RwLock::new(Vec::new()),
            hit_history: RwLock::new(Vec::new()),
            max_history_size: 1000,
            sequence_indices: RwLock::new(HashMap::new()),
            recording_enabled: RwLock::new(false),
            recorded_mocks: RwLock::new(Vec::new()),
        }
    }

    /// Create with custom history size
    pub fn with_history_size(max_history_size: usize) -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            collections: RwLock::new(Vec::new()),
            hit_history: RwLock::new(Vec::new()),
            max_history_size,
            sequence_indices: RwLock::new(HashMap::new()),
            recording_enabled: RwLock::new(false),
            recorded_mocks: RwLock::new(Vec::new()),
        }
    }

    // ==================== Rule Management ====================

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
            self.sequence_indices.write().remove(id);
            true
        } else {
            false
        }
    }

    /// Get all rules
    pub fn get_rules(&self) -> Vec<MockRule> {
        self.rules.read().clone()
    }

    /// Get rules by collection
    pub fn get_rules_by_collection(&self, collection_id: &str) -> Vec<MockRule> {
        self.rules
            .read()
            .iter()
            .filter(|r| r.collection_id.as_deref() == Some(collection_id))
            .cloned()
            .collect()
    }

    /// Get rules by tag
    pub fn get_rules_by_tag(&self, tag: &str) -> Vec<MockRule> {
        self.rules
            .read()
            .iter()
            .filter(|r| r.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
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

    /// Update rule with version history
    pub fn update_rule_with_version(
        &self,
        id: &str,
        mut rule: MockRule,
        comment: Option<String>,
    ) -> bool {
        let mut rules = self.rules.write();
        if let Some(pos) = rules.iter().position(|r| r.id == id) {
            rule.save_version(comment);
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
            rule.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Duplicate a rule
    pub fn duplicate_rule(&self, id: &str, new_name: Option<String>) -> Option<String> {
        let rules = self.rules.read();
        if let Some(rule) = rules.iter().find(|r| r.id == id) {
            let mut new_rule = rule.clone();
            new_rule.id = Uuid::new_v4().to_string();
            new_rule.name = new_name.unwrap_or_else(|| format!("{} (copy)", rule.name));
            new_rule.created_at = Utc::now();
            new_rule.updated_at = Utc::now();
            new_rule.hit_count = 0;
            new_rule.version = 1;
            new_rule.version_history.clear();
            drop(rules);
            let id = new_rule.id.clone();
            self.rules.write().push(new_rule);
            Some(id)
        } else {
            None
        }
    }

    /// Rollback a rule to a previous version
    pub fn rollback_rule(&self, id: &str, version: u32) -> bool {
        let mut rules = self.rules.write();
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.rollback_to_version(version)
        } else {
            false
        }
    }

    // ==================== Collection Management ====================

    /// Add a collection
    pub fn add_collection(&self, collection: MockCollection) -> String {
        let id = collection.id.clone();
        self.collections.write().push(collection);
        id
    }

    /// Get all collections
    pub fn get_collections(&self) -> Vec<MockCollection> {
        self.collections.read().clone()
    }

    /// Get a specific collection
    pub fn get_collection(&self, id: &str) -> Option<MockCollection> {
        self.collections.read().iter().find(|c| c.id == id).cloned()
    }

    /// Update a collection
    pub fn update_collection(&self, id: &str, collection: MockCollection) -> bool {
        let mut collections = self.collections.write();
        if let Some(pos) = collections.iter().position(|c| c.id == id) {
            collections[pos] = collection;
            true
        } else {
            false
        }
    }

    /// Delete a collection (optionally delete rules in it)
    pub fn delete_collection(&self, id: &str, delete_rules: bool) -> bool {
        let mut collections = self.collections.write();
        if let Some(pos) = collections.iter().position(|c| c.id == id) {
            collections.remove(pos);
            if delete_rules {
                self.rules
                    .write()
                    .retain(|r| r.collection_id.as_deref() != Some(id));
            } else {
                // Just remove collection reference from rules
                for rule in self.rules.write().iter_mut() {
                    if rule.collection_id.as_deref() == Some(id) {
                        rule.collection_id = None;
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Toggle all rules in a collection
    pub fn toggle_collection(&self, id: &str, enabled: bool) -> usize {
        let mut count = 0;
        let mut rules = self.rules.write();
        for rule in rules.iter_mut() {
            if rule.collection_id.as_deref() == Some(id) {
                rule.enabled = enabled;
                rule.updated_at = Utc::now();
                count += 1;
            }
        }
        count
    }

    // ==================== Matching & Response Selection ====================

    /// Check if a request matches any mock rule and get the response
    pub fn find_matching_mock(&self, request: &RequestData) -> Option<MockRule> {
        let mut rules = self.rules.write();

        // Filter enabled, non-expired rules that match
        let matching = rules
            .iter_mut()
            .filter(|r| r.enabled && !r.is_expired())
            .filter(|r| {
                // Check if collection is enabled
                if let Some(collection_id) = &r.collection_id {
                    let collections = self.collections.read();
                    if let Some(collection) = collections.iter().find(|c| c.id == *collection_id) {
                        if !collection.enabled {
                            return false;
                        }
                    }
                }
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
            rule.updated_at = Utc::now();
            Some(rule.clone())
        } else {
            None
        }
    }

    /// Get the response for a matched rule (handles sequences, conditionals, probabilistic)
    pub fn get_response_for_rule(&self, rule: &MockRule, request: &RequestData) -> MockResponse {
        match &rule.response_config {
            ResponseConfig::Single { response } => response.process_template(request),
            ResponseConfig::Sequence {
                responses, cycle, ..
            } => {
                let mut indices = self.sequence_indices.write();
                let index = indices.entry(rule.id.clone()).or_insert(0);
                let response = responses.get(*index).cloned().unwrap_or_default();

                // Advance index
                *index += 1;
                if *index >= responses.len() {
                    if *cycle {
                        *index = 0;
                    } else {
                        *index = responses.len() - 1;
                    }
                }

                response.process_template(request)
            }
            ResponseConfig::Conditional {
                conditions,
                default_response,
            } => {
                for cond_resp in conditions {
                    if cond_resp.condition.matches(request) {
                        return cond_resp.response.process_template(request);
                    }
                }
                default_response.process_template(request)
            }
            ResponseConfig::Probabilistic { responses } => {
                let total_weight: u32 = responses.iter().map(|r| r.weight).sum();
                if total_weight == 0 {
                    return MockResponse::default();
                }

                let mut random_value = rand::rng().random_range(0..total_weight);

                for prob_resp in responses {
                    if random_value < prob_resp.weight {
                        return prob_resp.response.process_template(request);
                    }
                    random_value -= prob_resp.weight;
                }

                responses
                    .last()
                    .map(|r| r.response.process_template(request))
                    .unwrap_or_default()
            }
        }
    }

    /// Reset sequence index for a rule
    pub fn reset_sequence(&self, rule_id: &str) {
        self.sequence_indices.write().remove(rule_id);
    }

    /// Reset all sequence indices
    pub fn reset_all_sequences(&self) {
        self.sequence_indices.write().clear();
    }

    // ==================== Hit Analytics ====================

    /// Record a hit for analytics
    pub fn record_hit(
        &self,
        mock_id: &str,
        request: &RequestData,
        response_status: u16,
        response_time_ms: u64,
        response_index: Option<usize>,
    ) {
        let record = MockHitRecord {
            mock_id: mock_id.to_string(),
            timestamp: Utc::now(),
            request_url: request.url.clone(),
            request_method: request.method.to_string(),
            response_status,
            response_time_ms,
            response_index,
        };

        let mut history = self.hit_history.write();
        history.push(record);

        // Trim if over max size
        if history.len() > self.max_history_size {
            let excess = history.len() - self.max_history_size;
            history.drain(0..excess);
        }
    }

    /// Get hit history for a specific mock
    pub fn get_hit_history(&self, mock_id: &str) -> Vec<MockHitRecord> {
        self.hit_history
            .read()
            .iter()
            .filter(|h| h.mock_id == mock_id)
            .cloned()
            .collect()
    }

    /// Get all hit history
    pub fn get_all_hit_history(&self) -> Vec<MockHitRecord> {
        self.hit_history.read().clone()
    }

    /// Get hit statistics for a mock
    pub fn get_hit_stats(&self, mock_id: &str) -> MockHitStats {
        let history = self.hit_history.read();
        let hits: Vec<_> = history.iter().filter(|h| h.mock_id == mock_id).collect();

        if hits.is_empty() {
            return MockHitStats::default();
        }

        let total_hits = hits.len() as u64;
        let avg_response_time = hits.iter().map(|h| h.response_time_ms).sum::<u64>() / total_hits;
        let min_response_time = hits.iter().map(|h| h.response_time_ms).min().unwrap_or(0);
        let max_response_time = hits.iter().map(|h| h.response_time_ms).max().unwrap_or(0);
        let last_hit = hits.last().map(|h| h.timestamp);
        let first_hit = hits.first().map(|h| h.timestamp);

        MockHitStats {
            total_hits,
            avg_response_time_ms: avg_response_time,
            min_response_time_ms: min_response_time,
            max_response_time_ms: max_response_time,
            first_hit,
            last_hit,
        }
    }

    /// Clear hit history
    pub fn clear_hit_history(&self) {
        self.hit_history.write().clear();
    }

    // ==================== Recording ====================

    /// Enable/disable recording mode
    pub fn set_recording(&self, enabled: bool) {
        *self.recording_enabled.write() = enabled;
    }

    /// Check if recording is enabled
    pub fn is_recording(&self) -> bool {
        *self.recording_enabled.read()
    }

    /// Record a response from live traffic as a mock
    pub fn record_from_traffic(
        &self,
        request: &RequestData,
        response_status: u16,
        response_headers: HashMap<String, String>,
        response_body: Option<Vec<u8>>,
    ) -> String {
        let mock = MockRule::new(
            format!("Recorded: {} {}", request.method, request.url),
            MatchCondition::UrlPattern {
                pattern: regex::escape(&request.url),
            },
            MockResponse {
                status_code: response_status,
                headers: response_headers,
                body: response_body.and_then(|b| String::from_utf8(b).ok()),
                ..Default::default()
            },
        );

        let id = mock.id.clone();
        self.recorded_mocks.write().push(mock);
        id
    }

    /// Get recorded mocks
    pub fn get_recorded_mocks(&self) -> Vec<MockRule> {
        self.recorded_mocks.read().clone()
    }

    /// Clear recorded mocks
    pub fn clear_recorded_mocks(&self) {
        self.recorded_mocks.write().clear();
    }

    /// Promote recorded mocks to active rules
    pub fn promote_recorded_mocks(&self) -> usize {
        let recorded = self.recorded_mocks.write().drain(..).collect::<Vec<_>>();
        let count = recorded.len();
        self.rules.write().extend(recorded);
        count
    }

    // ==================== Import/Export ====================

    /// Clear all rules
    pub fn clear(&self) {
        self.rules.write().clear();
        self.sequence_indices.write().clear();
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

    /// Import from HAR format
    pub fn import_from_har(&self, har_json: &str) -> Result<usize, String> {
        let har: serde_json::Value =
            serde_json::from_str(har_json).map_err(|e| format!("Invalid HAR JSON: {}", e))?;

        let entries = har
            .get("log")
            .and_then(|log| log.get("entries"))
            .and_then(|e| e.as_array())
            .ok_or("Invalid HAR structure: missing log.entries")?;

        let mut count = 0;
        for entry in entries {
            if let (Some(request), Some(response)) = (entry.get("request"), entry.get("response")) {
                let url = request.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let method = request
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("GET");
                let status = response
                    .get("status")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(200) as u16;

                let mut headers = HashMap::new();
                if let Some(resp_headers) = response.get("headers").and_then(|h| h.as_array()) {
                    for header in resp_headers {
                        if let (Some(name), Some(value)) = (
                            header.get("name").and_then(|n| n.as_str()),
                            header.get("value").and_then(|v| v.as_str()),
                        ) {
                            headers.insert(name.to_string(), value.to_string());
                        }
                    }
                }

                let body = response
                    .get("content")
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                let mock = MockRule::new(
                    format!("HAR: {} {}", method, url),
                    MatchCondition::And {
                        conditions: vec![
                            MatchCondition::UrlPattern {
                                pattern: regex::escape(url),
                            },
                            MatchCondition::Method {
                                method: method.to_string(),
                            },
                        ],
                    },
                    MockResponse {
                        status_code: status,
                        headers,
                        body,
                        ..Default::default()
                    },
                );

                self.rules.write().push(mock);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Import from OpenAPI/Swagger format
    pub fn import_from_openapi(&self, openapi_json: &str) -> Result<usize, String> {
        let spec: serde_json::Value = serde_json::from_str(openapi_json)
            .map_err(|e| format!("Invalid OpenAPI JSON: {}", e))?;

        let paths = spec
            .get("paths")
            .and_then(|p| p.as_object())
            .ok_or("Invalid OpenAPI structure: missing paths")?;

        let mut count = 0;
        for (path, methods) in paths {
            if let Some(methods_obj) = methods.as_object() {
                for (method, operation) in methods_obj {
                    if let Some(responses) = operation.get("responses").and_then(|r| r.as_object())
                    {
                        for (status_code, response_def) in responses {
                            let status: u16 = status_code.parse().unwrap_or(200);

                            // Try to get example response
                            let body = response_def
                                .get("content")
                                .and_then(|c| c.get("application/json"))
                                .and_then(|j| j.get("example"))
                                .map(|e| serde_json::to_string_pretty(e).unwrap_or_default());

                            let mock = MockRule::new(
                                format!(
                                    "OpenAPI: {} {} -> {}",
                                    method.to_uppercase(),
                                    path,
                                    status_code
                                ),
                                MatchCondition::And {
                                    conditions: vec![
                                        MatchCondition::UrlPattern {
                                            pattern: path
                                                .replace("{", "(?P<")
                                                .replace("}", ">[^/]+)"),
                                        },
                                        MatchCondition::Method {
                                            method: method.to_uppercase(),
                                        },
                                    ],
                                },
                                MockResponse {
                                    status_code: status,
                                    headers: HashMap::from([(
                                        "Content-Type".to_string(),
                                        "application/json".to_string(),
                                    )]),
                                    body,
                                    ..Default::default()
                                },
                            );

                            self.rules.write().push(mock);
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Import from Postman collection format
    pub fn import_from_postman(&self, postman_json: &str) -> Result<usize, String> {
        let collection: serde_json::Value = serde_json::from_str(postman_json)
            .map_err(|e| format!("Invalid Postman JSON: {}", e))?;

        let items = collection
            .get("item")
            .and_then(|i| i.as_array())
            .ok_or("Invalid Postman structure: missing item array")?;

        let mut count = 0;
        count += self.import_postman_items(items)?;

        Ok(count)
    }

    fn import_postman_items(&self, items: &[serde_json::Value]) -> Result<usize, String> {
        let mut count = 0;

        for item in items {
            // Handle nested folders
            if let Some(nested_items) = item.get("item").and_then(|i| i.as_array()) {
                count += self.import_postman_items(nested_items)?;
                continue;
            }

            if let Some(request) = item.get("request") {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unnamed");
                let method = request
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("GET");

                let url = if let Some(url_obj) = request.get("url") {
                    if let Some(raw) = url_obj.get("raw").and_then(|r| r.as_str()) {
                        raw.to_string()
                    } else if let Some(url_str) = url_obj.as_str() {
                        url_str.to_string()
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                // Get example response if available
                let (status, body) =
                    if let Some(responses) = item.get("response").and_then(|r| r.as_array()) {
                        if let Some(first_response) = responses.first() {
                            let status = first_response
                                .get("code")
                                .and_then(|c| c.as_u64())
                                .unwrap_or(200) as u16;
                            let body = first_response
                                .get("body")
                                .and_then(|b| b.as_str())
                                .map(|s| s.to_string());
                            (status, body)
                        } else {
                            (200, None)
                        }
                    } else {
                        (200, None)
                    };

                let mock = MockRule::new(
                    format!("Postman: {}", name),
                    MatchCondition::And {
                        conditions: vec![
                            MatchCondition::UrlPattern {
                                pattern: regex::escape(&url),
                            },
                            MatchCondition::Method {
                                method: method.to_string(),
                            },
                        ],
                    },
                    MockResponse {
                        status_code: status,
                        headers: HashMap::from([(
                            "Content-Type".to_string(),
                            "application/json".to_string(),
                        )]),
                        body,
                        ..Default::default()
                    },
                );

                self.rules.write().push(mock);
                count += 1;
            }
        }

        Ok(count)
    }

    // ==================== Validation ====================

    /// Validate a mock response against its JSON schema
    pub fn validate_response(&self, rule_id: &str, response_body: &str) -> Result<(), String> {
        let rules = self.rules.read();
        let rule = rules
            .iter()
            .find(|r| r.id == rule_id)
            .ok_or("Rule not found")?;

        if let Some(schema_str) = &rule.response_schema {
            let schema: serde_json::Value = serde_json::from_str(schema_str)
                .map_err(|e| format!("Invalid schema JSON: {}", e))?;
            let response: serde_json::Value = serde_json::from_str(response_body)
                .map_err(|e| format!("Invalid response JSON: {}", e))?;

            // Basic JSON Schema validation (simplified)
            Self::validate_json_schema(&response, &schema)
        } else {
            Ok(())
        }
    }

    fn validate_json_schema(
        value: &serde_json::Value,
        schema: &serde_json::Value,
    ) -> Result<(), String> {
        // Type validation
        if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
            let actual_type = match value {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::String(_) => "string",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
            };

            if expected_type != actual_type
                && !(expected_type == "integer" && actual_type == "number")
            {
                return Err(format!(
                    "Type mismatch: expected {}, got {}",
                    expected_type, actual_type
                ));
            }
        }

        // Required properties
        if let (Some(required), Some(obj)) = (
            schema.get("required").and_then(|r| r.as_array()),
            value.as_object(),
        ) {
            for req in required {
                if let Some(prop_name) = req.as_str() {
                    if !obj.contains_key(prop_name) {
                        return Err(format!("Missing required property: {}", prop_name));
                    }
                }
            }
        }

        // Nested properties
        if let (Some(properties), Some(obj)) = (
            schema.get("properties").and_then(|p| p.as_object()),
            value.as_object(),
        ) {
            for (prop_name, prop_schema) in properties {
                if let Some(prop_value) = obj.get(prop_name) {
                    Self::validate_json_schema(prop_value, prop_schema)?;
                }
            }
        }

        // Array items
        if let (Some(items_schema), Some(arr)) = (schema.get("items"), value.as_array()) {
            for item in arr {
                Self::validate_json_schema(item, items_schema)?;
            }
        }

        Ok(())
    }

    // ==================== Testing/Preview ====================

    /// Test a mock rule against a sample request without affecting state
    pub fn test_mock(&self, rule_id: &str, request: &RequestData) -> Option<MockTestResult> {
        let rules = self.rules.read();
        let rule = rules.iter().find(|r| r.id == rule_id)?;

        let matches = rule.condition.matches_request(
            &request.url,
            &request.method.to_string(),
            &request.headers,
            request.body.as_deref(),
            request.content_type.as_deref(),
        );

        if matches {
            let response = self.get_response_for_rule(rule, request);
            Some(MockTestResult {
                matches: true,
                response: Some(response),
                match_details: "Condition matched".to_string(),
            })
        } else {
            Some(MockTestResult {
                matches: false,
                response: None,
                match_details: "Condition did not match".to_string(),
            })
        }
    }

    /// Preview what response would be returned for a request
    pub fn preview_response(&self, request: &RequestData) -> Option<MockPreviewResult> {
        let rules = self.rules.read();

        let matching: Vec<_> = rules
            .iter()
            .filter(|r| r.enabled && !r.is_expired())
            .filter(|r| {
                r.condition.matches_request(
                    &request.url,
                    &request.method.to_string(),
                    &request.headers,
                    request.body.as_deref(),
                    request.content_type.as_deref(),
                )
            })
            .collect();

        if matching.is_empty() {
            return None;
        }

        let selected = matching.iter().min_by_key(|r| r.priority)?;
        let response = self.get_response_for_rule(selected, request);

        Some(MockPreviewResult {
            matched_rule_id: selected.id.clone(),
            matched_rule_name: selected.name.clone(),
            response,
            other_matching_rules: matching
                .iter()
                .filter(|r| r.id != selected.id)
                .map(|r| (r.id.clone(), r.name.clone(), r.priority))
                .collect(),
        })
    }
}

impl Default for MockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for mock hits
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MockHitStats {
    pub total_hits: u64,
    pub avg_response_time_ms: u64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub first_hit: Option<DateTime<Utc>>,
    pub last_hit: Option<DateTime<Utc>>,
}

/// Result of testing a mock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockTestResult {
    pub matches: bool,
    pub response: Option<MockResponse>,
    pub match_details: String,
}

/// Result of previewing a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockPreviewResult {
    pub matched_rule_id: String,
    pub matched_rule_name: String,
    pub response: MockResponse,
    pub other_matching_rules: Vec<(String, String, u32)>, // (id, name, priority)
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

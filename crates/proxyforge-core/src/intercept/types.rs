//! Common types for interception features

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Action to take when a breakpoint is hit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BreakpointAction {
    /// Pause and wait for user decision
    Pause,
    /// Auto-forward after N seconds
    AutoForward { timeout_secs: u64 },
    /// Auto-respond with a mock response
    AutoRespond { mock_id: String },
}

/// Direction of interception
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InterceptDirection {
    /// Intercept requests before they reach the server
    Request,
    /// Intercept responses before they reach the client
    Response,
    /// Intercept both
    Both,
}

/// Condition for matching traffic
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MatchCondition {
    /// Match all traffic
    All,
    /// Match by URL regex pattern
    UrlPattern { pattern: String },
    /// Match by HTTP method
    Method { method: String },
    /// Match by status code range (e.g., "4xx", "5xx")
    StatusCode { range: String },
    /// Match by header presence/value
    Header { name: String, value: Option<String> },
    /// Match by request body content (regex)
    BodyPattern { pattern: String },
    /// Match by content type
    ContentType { pattern: String },
    /// Combine conditions with AND
    And { conditions: Vec<MatchCondition> },
    /// Combine conditions with OR
    Or { conditions: Vec<MatchCondition> },
    /// Negate a condition
    Not { condition: Box<MatchCondition> },
}

impl MatchCondition {
    /// Check if this condition matches a request
    pub fn matches_request(
        &self,
        url: &str,
        method: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
        content_type: Option<&str>,
    ) -> bool {
        match self {
            MatchCondition::All => true,
            MatchCondition::UrlPattern { pattern } => regex::Regex::new(pattern)
                .map(|re| re.is_match(url))
                .unwrap_or(false),
            MatchCondition::Method { method: m } => method.eq_ignore_ascii_case(m),
            MatchCondition::Header { name, value } => headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case(name)
                    && value.as_ref().map_or(true, |expected| v == expected)
            }),
            MatchCondition::BodyPattern { pattern } => body
                .and_then(|b| std::str::from_utf8(b).ok())
                .map(|body_str| {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(body_str))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            MatchCondition::ContentType { pattern } => content_type
                .map(|ct| {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(ct))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            MatchCondition::And { conditions } => conditions
                .iter()
                .all(|c| c.matches_request(url, method, headers, body, content_type)),
            MatchCondition::Or { conditions } => conditions
                .iter()
                .any(|c| c.matches_request(url, method, headers, body, content_type)),
            MatchCondition::Not { condition } => {
                !condition.matches_request(url, method, headers, body, content_type)
            }
            MatchCondition::StatusCode { .. } => {
                // Not applicable to requests
                false
            }
        }
    }

    /// Check if this condition matches a response
    pub fn matches_response(
        &self,
        status_code: u16,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
        content_type: Option<&str>,
    ) -> bool {
        match self {
            MatchCondition::All => true,
            MatchCondition::StatusCode { range } => {
                match range.as_str() {
                    "2xx" => (200..=299).contains(&status_code),
                    "3xx" => (300..=399).contains(&status_code),
                    "4xx" => (400..=499).contains(&status_code),
                    "5xx" => (500..=599).contains(&status_code),
                    _ => {
                        // Try parsing as exact code
                        range
                            .parse::<u16>()
                            .map(|c| c == status_code)
                            .unwrap_or(false)
                    }
                }
            }
            MatchCondition::Header { name, value } => headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case(name)
                    && value.as_ref().map_or(true, |expected| v == expected)
            }),
            MatchCondition::BodyPattern { pattern } => body
                .and_then(|b| std::str::from_utf8(b).ok())
                .map(|body_str| {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(body_str))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            MatchCondition::ContentType { pattern } => content_type
                .map(|ct| {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(ct))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            MatchCondition::And { conditions } => conditions
                .iter()
                .all(|c| c.matches_response(status_code, headers, body, content_type)),
            MatchCondition::Or { conditions } => conditions
                .iter()
                .any(|c| c.matches_response(status_code, headers, body, content_type)),
            MatchCondition::Not { condition } => {
                !condition.matches_response(status_code, headers, body, content_type)
            }
            MatchCondition::UrlPattern { .. } | MatchCondition::Method { .. } => {
                // Not applicable to responses directly (would need request context)
                false
            }
        }
    }
}

/// Modification to apply to a request or response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Modification {
    /// Add or replace a header
    SetHeader { name: String, value: String },
    /// Remove a header
    RemoveHeader { name: String },
    /// Replace body (string)
    SetBody { content: String },
    /// Replace body (base64 encoded binary)
    SetBodyBase64 { content: String },
    /// Replace URL (for requests only)
    SetUrl { url: String },
    /// Replace path (for requests only)
    SetPath { path: String },
    /// Set status code (for responses only)
    SetStatusCode { code: u16 },
    /// Apply regex replacement to body
    RegexReplace {
        pattern: String,
        replacement: String,
    },
    /// Apply regex replacement to URL
    UrlRegexReplace {
        pattern: String,
        replacement: String,
    },
    /// Delay by N milliseconds
    Delay { ms: u64 },
}

/// Result of an interception decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterceptDecision {
    /// Allow the traffic to continue
    Continue,
    /// Apply modifications and continue
    Modify { modifications: Vec<Modification> },
    /// Respond immediately without forwarding
    Respond {
        status_code: u16,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    },
    /// Close the connection
    Close,
}

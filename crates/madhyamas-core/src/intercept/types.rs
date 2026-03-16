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
    /// Match by header with regex pattern
    HeaderPattern { name: String, pattern: String },
    /// Match by request body content (regex)
    BodyPattern { pattern: String },
    /// Match by content type
    ContentType { pattern: String },
    /// Match by query parameter (exact match)
    QueryParam { name: String, value: Option<String> },
    /// Match by query parameter with regex
    QueryParamPattern { name: String, pattern: String },
    /// Match by JSON path in request body (using JSONPath syntax like $.user.id)
    JsonPath {
        path: String,
        value: serde_json::Value,
    },
    /// Match by JSON path with regex pattern
    JsonPathPattern { path: String, pattern: String },
    /// Match by GraphQL operation name
    GraphQLOperation { operation_name: String },
    /// Match by GraphQL operation type (query, mutation, subscription)
    GraphQLType { operation_type: String },
    /// Match by GraphQL variable value
    GraphQLVariable {
        name: String,
        value: serde_json::Value,
    },
    /// Match by URL path segment (e.g., /api/users/:id where :id is a path param)
    PathSegment { index: usize, value: String },
    /// Match by URL path with regex
    PathPattern { pattern: String },
    /// Match by host/domain
    Host { pattern: String },
    /// Match by port
    Port { port: u16 },
    /// Match by scheme (http/https)
    Scheme { scheme: String },
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
                k.eq_ignore_ascii_case(name) && value.as_ref().is_none_or(|expected| v == expected)
            }),
            MatchCondition::HeaderPattern { name, pattern } => headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case(name)
                    && regex::Regex::new(pattern)
                        .map(|re| re.is_match(v))
                        .unwrap_or(false)
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
            MatchCondition::QueryParam { name, value } => {
                if let Ok(parsed_url) = url::Url::parse(url) {
                    parsed_url.query_pairs().any(|(k, v)| {
                        k == *name && value.as_ref().is_none_or(|expected| v == *expected)
                    })
                } else {
                    false
                }
            }
            MatchCondition::QueryParamPattern { name, pattern } => {
                if let Ok(parsed_url) = url::Url::parse(url) {
                    parsed_url.query_pairs().any(|(k, v)| {
                        k == *name
                            && regex::Regex::new(pattern)
                                .map(|re| re.is_match(&v))
                                .unwrap_or(false)
                    })
                } else {
                    false
                }
            }
            MatchCondition::JsonPath { path, value } => body
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                .map(|json| {
                    jsonpath_lib::select(&json, path)
                        .map(|results| results.contains(&value))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            MatchCondition::JsonPathPattern { path, pattern } => body
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                .map(|json| {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        jsonpath_lib::select(&json, path)
                            .map(|results| {
                                results.iter().any(|r| {
                                    if let Some(s) = r.as_str() {
                                        re.is_match(s)
                                    } else {
                                        re.is_match(&r.to_string())
                                    }
                                })
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    }
                })
                .unwrap_or(false),
            MatchCondition::GraphQLOperation { operation_name } => body
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                .and_then(|json| {
                    json.get("operationName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .map(|op| op == *operation_name)
                .unwrap_or(false),
            MatchCondition::GraphQLType { operation_type } => body
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                .and_then(|json| {
                    json.get("query")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .map(|query| {
                    let query_lower = query.trim().to_lowercase();
                    match operation_type.to_lowercase().as_str() {
                        "query" => query_lower.starts_with("query") || query_lower.starts_with("{"),
                        "mutation" => query_lower.starts_with("mutation"),
                        "subscription" => query_lower.starts_with("subscription"),
                        _ => false,
                    }
                })
                .unwrap_or(false),
            MatchCondition::GraphQLVariable { name, value } => body
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                .and_then(|json| json.get("variables").cloned())
                .and_then(|vars| vars.get(name).cloned())
                .map(|v| v == *value)
                .unwrap_or(false),
            MatchCondition::PathSegment { index, value } => {
                if let Ok(parsed_url) = url::Url::parse(url) {
                    parsed_url
                        .path_segments()
                        .and_then(|segments| {
                            segments
                                .collect::<Vec<_>>()
                                .get(*index)
                                .map(|s| *s == value)
                        })
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            MatchCondition::PathPattern { pattern } => {
                if let Ok(parsed_url) = url::Url::parse(url) {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(parsed_url.path()))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            MatchCondition::Host { pattern } => {
                if let Ok(parsed_url) = url::Url::parse(url) {
                    parsed_url
                        .host_str()
                        .map(|host| {
                            regex::Regex::new(pattern)
                                .map(|re| re.is_match(host))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            MatchCondition::Port { port } => {
                if let Ok(parsed_url) = url::Url::parse(url) {
                    parsed_url.port().map(|p| p == *port).unwrap_or(false)
                } else {
                    false
                }
            }
            MatchCondition::Scheme { scheme } => {
                if let Ok(parsed_url) = url::Url::parse(url) {
                    parsed_url.scheme().eq_ignore_ascii_case(scheme)
                } else {
                    false
                }
            }
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
                k.eq_ignore_ascii_case(name) && value.as_ref().is_none_or(|expected| v == expected)
            }),
            MatchCondition::HeaderPattern { name, pattern } => headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case(name)
                    && regex::Regex::new(pattern)
                        .map(|re| re.is_match(v))
                        .unwrap_or(false)
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
            MatchCondition::JsonPath { path, value } => body
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                .map(|json| {
                    jsonpath_lib::select(&json, path)
                        .map(|results| results.contains(&value))
                        .unwrap_or(false)
                })
                .unwrap_or(false),
            MatchCondition::JsonPathPattern { path, pattern } => body
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                .map(|json| {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        jsonpath_lib::select(&json, path)
                            .map(|results| {
                                results.iter().any(|r| {
                                    if let Some(s) = r.as_str() {
                                        re.is_match(s)
                                    } else {
                                        re.is_match(&r.to_string())
                                    }
                                })
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    }
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
            // Not applicable to responses directly (would need request context)
            MatchCondition::UrlPattern { .. }
            | MatchCondition::Method { .. }
            | MatchCondition::QueryParam { .. }
            | MatchCondition::QueryParamPattern { .. }
            | MatchCondition::GraphQLOperation { .. }
            | MatchCondition::GraphQLType { .. }
            | MatchCondition::GraphQLVariable { .. }
            | MatchCondition::PathSegment { .. }
            | MatchCondition::PathPattern { .. }
            | MatchCondition::Host { .. }
            | MatchCondition::Port { .. }
            | MatchCondition::Scheme { .. } => false,
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

#[cfg(test)]
mod tests {
    use super::*;

    mod breakpoint_action_tests {
        use super::*;

        #[test]
        fn test_pause_action() {
            let action = BreakpointAction::Pause;
            let json = serde_json::to_string(&action).unwrap();
            assert!(json.contains("\"type\":\"pause\""));

            let decoded: BreakpointAction = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, BreakpointAction::Pause);
        }

        #[test]
        fn test_auto_forward_action() {
            let action = BreakpointAction::AutoForward { timeout_secs: 30 };
            let json = serde_json::to_string(&action).unwrap();
            assert!(json.contains("\"type\":\"auto_forward\""));
            assert!(json.contains("\"timeout_secs\":30"));

            let decoded: BreakpointAction = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, BreakpointAction::AutoForward { timeout_secs: 30 });
        }

        #[test]
        fn test_auto_respond_action() {
            let action = BreakpointAction::AutoRespond {
                mock_id: "mock-123".to_string(),
            };
            let json = serde_json::to_string(&action).unwrap();
            assert!(json.contains("\"type\":\"auto_respond\""));
            assert!(json.contains("\"mock_id\":\"mock-123\""));
        }
    }

    mod intercept_direction_tests {
        use super::*;

        #[test]
        fn test_direction_serialization() {
            assert_eq!(
                serde_json::to_string(&InterceptDirection::Request).unwrap(),
                "\"request\""
            );
            assert_eq!(
                serde_json::to_string(&InterceptDirection::Response).unwrap(),
                "\"response\""
            );
            assert_eq!(
                serde_json::to_string(&InterceptDirection::Both).unwrap(),
                "\"both\""
            );
        }

        #[test]
        fn test_direction_deserialization() {
            let req: InterceptDirection = serde_json::from_str("\"request\"").unwrap();
            assert_eq!(req, InterceptDirection::Request);

            let res: InterceptDirection = serde_json::from_str("\"response\"").unwrap();
            assert_eq!(res, InterceptDirection::Response);

            let both: InterceptDirection = serde_json::from_str("\"both\"").unwrap();
            assert_eq!(both, InterceptDirection::Both);
        }
    }

    mod match_condition_tests {
        use super::*;

        fn make_headers() -> HashMap<String, String> {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers.insert("Authorization".to_string(), "Bearer token123".to_string());
            headers
        }

        mod request_matching_tests {
            use super::*;

            #[test]
            fn test_match_all() {
                let condition = MatchCondition::All;
                assert!(condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_url_pattern() {
                let condition = MatchCondition::UrlPattern {
                    pattern: r"https://api\.example\.com/.*".to_string(),
                };

                assert!(condition.matches_request(
                    "https://api.example.com/users",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));

                assert!(!condition.matches_request(
                    "https://other.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_method() {
                let post_condition = MatchCondition::Method {
                    method: "POST".to_string(),
                };

                assert!(post_condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    None,
                    None
                ));

                assert!(post_condition.matches_request(
                    "https://example.com/api",
                    "post",
                    &HashMap::new(),
                    None,
                    None
                )); // case insensitive

                assert!(!post_condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_header_with_value() {
                let condition = MatchCondition::Header {
                    name: "Authorization".to_string(),
                    value: Some("Bearer token123".to_string()),
                };

                let headers = make_headers();
                assert!(condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &headers,
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_header_without_value() {
                let condition = MatchCondition::Header {
                    name: "Authorization".to_string(),
                    value: None,
                };

                let headers = make_headers();
                assert!(condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &headers,
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_header_missing() {
                let condition = MatchCondition::Header {
                    name: "X-Custom".to_string(),
                    value: None,
                };

                let headers = make_headers();
                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &headers,
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_body_pattern() {
                let condition = MatchCondition::BodyPattern {
                    pattern: r#"\"email\":\s*\".*@example\.com\""#.to_string(),
                };

                let body = br#"{"email": "user@example.com", "name": "Test"}"#;
                assert!(condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    Some(body),
                    None
                ));

                let other_body = br#"{"email": "user@other.com"}"#;
                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    Some(other_body),
                    None
                ));
            }

            #[test]
            fn test_match_body_pattern_no_body() {
                let condition = MatchCondition::BodyPattern {
                    pattern: "test".to_string(),
                };

                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_content_type() {
                let condition = MatchCondition::ContentType {
                    pattern: "application/json".to_string(),
                };

                assert!(condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    None,
                    Some("application/json")
                ));

                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    None,
                    Some("text/html")
                ));
            }

            #[test]
            fn test_match_and_condition() {
                let condition = MatchCondition::And {
                    conditions: vec![
                        MatchCondition::Method {
                            method: "POST".to_string(),
                        },
                        MatchCondition::ContentType {
                            pattern: "json".to_string(),
                        },
                    ],
                };

                assert!(condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    None,
                    Some("application/json")
                ));

                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    Some("application/json")
                ));

                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    None,
                    Some("text/html")
                ));
            }

            #[test]
            fn test_match_or_condition() {
                let condition = MatchCondition::Or {
                    conditions: vec![
                        MatchCondition::Method {
                            method: "POST".to_string(),
                        },
                        MatchCondition::Method {
                            method: "PUT".to_string(),
                        },
                    ],
                };

                assert!(condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    None,
                    None
                ));

                assert!(condition.matches_request(
                    "https://example.com/api",
                    "PUT",
                    &HashMap::new(),
                    None,
                    None
                ));

                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_not_condition() {
                let condition = MatchCondition::Not {
                    condition: Box::new(MatchCondition::Method {
                        method: "POST".to_string(),
                    }),
                };

                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "POST",
                    &HashMap::new(),
                    None,
                    None
                ));

                assert!(condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));
            }

            #[test]
            fn test_match_status_code_not_applicable() {
                let condition = MatchCondition::StatusCode {
                    range: "4xx".to_string(),
                };

                // Status code doesn't apply to requests
                assert!(!condition.matches_request(
                    "https://example.com/api",
                    "GET",
                    &HashMap::new(),
                    None,
                    None
                ));
            }
        }

        mod response_matching_tests {
            use super::*;

            #[test]
            fn test_match_all_response() {
                let condition = MatchCondition::All;
                assert!(condition.matches_response(200, &HashMap::new(), None, None));
                assert!(condition.matches_response(500, &HashMap::new(), None, None));
            }

            #[test]
            fn test_match_status_code_2xx() {
                let condition = MatchCondition::StatusCode {
                    range: "2xx".to_string(),
                };

                assert!(condition.matches_response(200, &HashMap::new(), None, None));
                assert!(condition.matches_response(201, &HashMap::new(), None, None));
                assert!(condition.matches_response(204, &HashMap::new(), None, None));
                assert!(!condition.matches_response(300, &HashMap::new(), None, None));
                assert!(!condition.matches_response(400, &HashMap::new(), None, None));
            }

            #[test]
            fn test_match_status_code_4xx() {
                let condition = MatchCondition::StatusCode {
                    range: "4xx".to_string(),
                };

                assert!(condition.matches_response(400, &HashMap::new(), None, None));
                assert!(condition.matches_response(404, &HashMap::new(), None, None));
                assert!(condition.matches_response(429, &HashMap::new(), None, None));
                assert!(!condition.matches_response(200, &HashMap::new(), None, None));
                assert!(!condition.matches_response(500, &HashMap::new(), None, None));
            }

            #[test]
            fn test_match_status_code_5xx() {
                let condition = MatchCondition::StatusCode {
                    range: "5xx".to_string(),
                };

                assert!(condition.matches_response(500, &HashMap::new(), None, None));
                assert!(condition.matches_response(502, &HashMap::new(), None, None));
                assert!(condition.matches_response(503, &HashMap::new(), None, None));
                assert!(!condition.matches_response(200, &HashMap::new(), None, None));
            }

            #[test]
            fn test_match_status_code_exact() {
                let condition = MatchCondition::StatusCode {
                    range: "404".to_string(),
                };

                assert!(condition.matches_response(404, &HashMap::new(), None, None));
                assert!(!condition.matches_response(400, &HashMap::new(), None, None));
                assert!(!condition.matches_response(500, &HashMap::new(), None, None));
            }

            #[test]
            fn test_match_response_header() {
                let condition = MatchCondition::Header {
                    name: "Content-Type".to_string(),
                    value: Some("application/json".to_string()),
                };

                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());

                assert!(condition.matches_response(200, &headers, None, None));
            }

            #[test]
            fn test_match_response_body() {
                let condition = MatchCondition::BodyPattern {
                    pattern: "error".to_string(),
                };

                let error_body = b"{\"error\": \"Not found\"}";
                assert!(condition.matches_response(404, &HashMap::new(), Some(error_body), None));

                let success_body = b"{\"success\": true}";
                assert!(!condition.matches_response(
                    200,
                    &HashMap::new(),
                    Some(success_body),
                    None
                ));
            }

            #[test]
            fn test_match_url_pattern_not_applicable_to_response() {
                let condition = MatchCondition::UrlPattern {
                    pattern: ".*".to_string(),
                };

                // URL pattern doesn't apply to responses directly
                assert!(!condition.matches_response(200, &HashMap::new(), None, None));
            }

            #[test]
            fn test_match_method_not_applicable_to_response() {
                let condition = MatchCondition::Method {
                    method: "GET".to_string(),
                };

                // Method doesn't apply to responses directly
                assert!(!condition.matches_response(200, &HashMap::new(), None, None));
            }

            #[test]
            fn test_match_response_and_condition() {
                let condition = MatchCondition::And {
                    conditions: vec![
                        MatchCondition::StatusCode {
                            range: "4xx".to_string(),
                        },
                        MatchCondition::ContentType {
                            pattern: "json".to_string(),
                        },
                    ],
                };

                assert!(condition.matches_response(
                    404,
                    &HashMap::new(),
                    None,
                    Some("application/json")
                ));
                assert!(!condition.matches_response(404, &HashMap::new(), None, Some("text/html")));
                assert!(!condition.matches_response(
                    200,
                    &HashMap::new(),
                    None,
                    Some("application/json")
                ));
            }
        }
    }

    mod modification_tests {
        use super::*;

        #[test]
        fn test_set_header_serialization() {
            let mod_ = Modification::SetHeader {
                name: "Authorization".to_string(),
                value: "Bearer token".to_string(),
            };
            let json = serde_json::to_string(&mod_).unwrap();
            assert!(json.contains("\"type\":\"set_header\""));
            assert!(json.contains("\"name\":\"Authorization\""));
        }

        #[test]
        fn test_remove_header_serialization() {
            let mod_ = Modification::RemoveHeader {
                name: "X-Custom".to_string(),
            };
            let json = serde_json::to_string(&mod_).unwrap();
            assert!(json.contains("\"type\":\"remove_header\""));
        }

        #[test]
        fn test_set_body_serialization() {
            let mod_ = Modification::SetBody {
                content: "{\"test\": true}".to_string(),
            };
            let json = serde_json::to_string(&mod_).unwrap();
            assert!(json.contains("\"type\":\"set_body\""));
        }

        #[test]
        fn test_set_status_code_serialization() {
            let mod_ = Modification::SetStatusCode { code: 201 };
            let json = serde_json::to_string(&mod_).unwrap();
            assert!(json.contains("\"type\":\"set_status_code\""));
            assert!(json.contains("\"code\":201"));
        }

        #[test]
        fn test_regex_replace_serialization() {
            let mod_ = Modification::RegexReplace {
                pattern: "old".to_string(),
                replacement: "new".to_string(),
            };
            let json = serde_json::to_string(&mod_).unwrap();
            assert!(json.contains("\"type\":\"regex_replace\""));
        }

        #[test]
        fn test_delay_serialization() {
            let mod_ = Modification::Delay { ms: 1000 };
            let json = serde_json::to_string(&mod_).unwrap();
            assert!(json.contains("\"type\":\"delay\""));
            assert!(json.contains("\"ms\":1000"));
        }
    }

    mod intercept_decision_tests {
        use super::*;

        #[test]
        fn test_continue_decision() {
            let decision = InterceptDecision::Continue;
            let json = serde_json::to_string(&decision).unwrap();
            assert!(json.contains("\"Continue\""));
        }

        #[test]
        fn test_modify_decision() {
            let decision = InterceptDecision::Modify {
                modifications: vec![Modification::SetHeader {
                    name: "X-Custom".to_string(),
                    value: "test".to_string(),
                }],
            };
            let json = serde_json::to_string(&decision).unwrap();
            assert!(json.contains("\"Modify\""));
        }

        #[test]
        fn test_respond_decision() {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());

            let decision = InterceptDecision::Respond {
                status_code: 200,
                headers: headers.clone(),
                body: Some(b"{\"success\":true}".to_vec()),
            };
            let json = serde_json::to_string(&decision).unwrap();
            assert!(json.contains("\"Respond\""));
            assert!(json.contains("\"status_code\":200"));
        }

        #[test]
        fn test_close_decision() {
            let decision = InterceptDecision::Close;
            let json = serde_json::to_string(&decision).unwrap();
            assert!(json.contains("\"Close\""));
        }
    }
}

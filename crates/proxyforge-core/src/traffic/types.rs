//! Traffic data types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/// HTTP method enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Connect,
    Trace,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Connect => write!(f, "CONNECT"),
            HttpMethod::Trace => write!(f, "TRACE"),
        }
    }
}

impl FromStr for HttpMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "PATCH" => Ok(HttpMethod::Patch),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            "CONNECT" => Ok(HttpMethod::Connect),
            "TRACE" => Ok(HttpMethod::Trace),
            _ => Err(format!("Unknown HTTP method: {}", s)),
        }
    }
}

impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(HttpMethod::Get)
    }
}

/// Request data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestData {
    /// HTTP method
    pub method: HttpMethod,
    /// Full URL
    pub url: String,
    /// Host name
    pub host: String,
    /// Path (including query string)
    pub path: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (as string for JSON serialization)
    #[serde(
        serialize_with = "serialize_body",
        deserialize_with = "deserialize_body"
    )]
    pub body: Option<Vec<u8>>,
    /// Content type
    pub content_type: Option<String>,
}

/// Response data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseData {
    /// HTTP status code
    pub status_code: u16,
    /// Status message
    pub status_message: Option<String>,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (as string for JSON serialization)
    #[serde(
        serialize_with = "serialize_body",
        deserialize_with = "deserialize_body"
    )]
    pub body: Option<Vec<u8>>,
    /// Content type
    pub content_type: Option<String>,
    /// Response time in milliseconds
    pub duration_ms: u64,
}

// Custom body serializer: converts Vec<u8> to String (UTF-8 or base64 for binary)
fn serialize_body<S>(body: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use base64::Engine;

    match body {
        Some(bytes) => {
            // Try UTF-8 first, fall back to base64 for binary data
            match String::from_utf8(bytes.clone()) {
                Ok(s) => serializer.serialize_str(&s),
                Err(_) => {
                    // Binary data - encode as base64 with prefix
                    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                    serializer.serialize_str(&format!("base64:{}", b64))
                }
            }
        }
        None => serializer.serialize_none(),
    }
}

// Custom body deserializer: converts String to Vec<u8>
fn deserialize_body<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use base64::Engine;

    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.map(|s| {
        // Check if it's base64 encoded
        if let Some(b64_data) = s.strip_prefix("base64:") {
            base64::engine::general_purpose::STANDARD
                .decode(b64_data)
                .unwrap_or_else(|_| s.into_bytes())
        } else {
            s.into_bytes()
        }
    }))
}

/// A complete traffic entry (request + response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficEntry {
    /// Unique identifier
    pub id: String,
    /// Session ID this entry belongs to
    pub session_id: String,
    /// Request data
    pub request: RequestData,
    /// Response data (None if request is pending)
    pub response: Option<ResponseData>,
    /// Timestamp when request was received
    #[serde(serialize_with = "serialize_datetime")]
    pub timestamp: DateTime<Utc>,
    /// Whether this entry has been modified (breakpoint)
    pub modified: bool,
    /// Notes/annotations
    pub notes: Option<String>,
}

fn serialize_datetime<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&dt.to_rfc3339())
}

impl TrafficEntry {
    /// Create a new traffic entry with a request
    pub fn new(session_id: &str, request: RequestData) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            request,
            response: None,
            timestamp: Utc::now(),
            modified: false,
            notes: None,
        }
    }

    /// Get the body size (request + response if available)
    pub fn total_size(&self) -> usize {
        let req_size = self.request.body.as_ref().map(|b| b.len()).unwrap_or(0);
        let res_size = self
            .response
            .as_ref()
            .and_then(|r| r.body.as_ref().map(|b| b.len()))
            .unwrap_or(0);
        req_size + res_size
    }

    /// Check if this is an HTTPS request
    pub fn is_https(&self) -> bool {
        self.request.url.starts_with("https://")
    }
}

/// Session for grouping traffic entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier
    pub id: String,
    /// Session name
    pub name: Option<String>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session
    pub fn new(name: Option<&str>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Filter for traffic queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficFilter {
    /// Filter by URL pattern (regex)
    pub url_pattern: Option<String>,
    /// Filter by HTTP method
    pub method: Option<HttpMethod>,
    /// Filter by status code range
    pub status_min: Option<u16>,
    pub status_max: Option<u16>,
    /// Search text in headers/body
    pub search: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Filter by file extension (e.g., ".js", ".css")
    pub file_type: Option<String>,
    /// Filter by request header (format: "key:value" or "key")
    pub header: Option<String>,
    /// Filter by cookie (name or value contains)
    pub cookie: Option<String>,
}

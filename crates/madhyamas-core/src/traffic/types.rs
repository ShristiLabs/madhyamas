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

impl RequestData {
    /// Total size of the request in bytes, computed from headers + body.
    pub fn size(&self) -> usize {
        headers_size(&self.headers) + self.body.as_ref().map(|b| b.len()).unwrap_or(0)
    }
}

/// Response data structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

impl ResponseData {
    /// Total size of the response in bytes, computed from headers + body.
    pub fn size(&self) -> usize {
        headers_size(&self.headers) + self.body.as_ref().map(|b| b.len()).unwrap_or(0)
    }
}

/// Compute the on-wire size of a header map in bytes.
/// Each header contributes `key: value\r\n` = key.len() + 2 + value.len() + 2.
fn headers_size(headers: &HashMap<String, String>) -> usize {
    headers
        .iter()
        .map(|(k, v)| k.len() + 2 + v.len() + 2)
        .sum()
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
    /// Total request size in bytes (headers + body).
    #[serde(default)]
    pub request_size: usize,
    /// Total response size in bytes (headers + body), if a response was received.
    #[serde(default)]
    pub response_size: Option<usize>,
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
        let request_size = request.size();
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            request,
            response: None,
            timestamp: Utc::now(),
            modified: false,
            notes: None,
            request_size,
            response_size: None,
        }
    }

    /// Get the total size (request + response if available)
    pub fn total_size(&self) -> usize {
        self.request_size
            + self.response_size.unwrap_or_else(|| {
                self.response.as_ref().map(|r| r.size()).unwrap_or(0)
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    mod http_method_tests {
        use super::*;

        #[test]
        fn test_display() {
            assert_eq!(HttpMethod::Get.to_string(), "GET");
            assert_eq!(HttpMethod::Post.to_string(), "POST");
            assert_eq!(HttpMethod::Put.to_string(), "PUT");
            assert_eq!(HttpMethod::Patch.to_string(), "PATCH");
            assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
            assert_eq!(HttpMethod::Head.to_string(), "HEAD");
            assert_eq!(HttpMethod::Options.to_string(), "OPTIONS");
            assert_eq!(HttpMethod::Connect.to_string(), "CONNECT");
            assert_eq!(HttpMethod::Trace.to_string(), "TRACE");
        }

        #[test]
        fn test_from_str() {
            assert_eq!("GET".parse::<HttpMethod>().unwrap(), HttpMethod::Get);
            assert_eq!("get".parse::<HttpMethod>().unwrap(), HttpMethod::Get);
            assert_eq!("POST".parse::<HttpMethod>().unwrap(), HttpMethod::Post);
            assert_eq!("post".parse::<HttpMethod>().unwrap(), HttpMethod::Post);
            assert_eq!("DELETE".parse::<HttpMethod>().unwrap(), HttpMethod::Delete);
        }

        #[test]
        fn test_from_str_invalid() {
            assert!("INVALID".parse::<HttpMethod>().is_err());
            assert!("".parse::<HttpMethod>().is_err());
        }

        #[test]
        fn test_from_str_case_insensitive() {
            assert_eq!(HttpMethod::from("get"), HttpMethod::Get);
            assert_eq!(HttpMethod::from("GET"), HttpMethod::Get);
            assert_eq!(HttpMethod::from("GeT"), HttpMethod::Get);
        }

        #[test]
        fn test_from_invalid_defaults_to_get() {
            assert_eq!(HttpMethod::from("invalid"), HttpMethod::Get);
            assert_eq!(HttpMethod::from(""), HttpMethod::Get);
        }

        #[test]
        fn test_serialize() {
            let method = HttpMethod::Post;
            let json = serde_json::to_string(&method).unwrap();
            assert_eq!(json, "\"POST\"");
        }

        #[test]
        fn test_deserialize() {
            let method: HttpMethod = serde_json::from_str("\"PUT\"").unwrap();
            assert_eq!(method, HttpMethod::Put);
        }
    }

    mod request_data_tests {
        use super::*;

        #[test]
        fn test_request_data_creation() {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());

            let request = RequestData {
                method: HttpMethod::Post,
                url: "https://example.com/api/test".to_string(),
                host: "example.com".to_string(),
                path: "/api/test".to_string(),
                headers: headers.clone(),
                body: Some(b"{\"key\":\"value\"}".to_vec()),
                content_type: Some("application/json".to_string()),
            };

            assert_eq!(request.method, HttpMethod::Post);
            assert_eq!(request.url, "https://example.com/api/test");
            assert_eq!(request.host, "example.com");
            assert_eq!(request.path, "/api/test");
            assert_eq!(
                request.headers.get("Content-Type"),
                Some(&"application/json".to_string())
            );
            assert!(request.body.is_some());
        }

        #[test]
        fn test_request_data_serialization() {
            let request = RequestData {
                method: HttpMethod::Get,
                url: "https://example.com/test".to_string(),
                host: "example.com".to_string(),
                path: "/test".to_string(),
                headers: HashMap::new(),
                body: None,
                content_type: None,
            };

            let json = serde_json::to_string(&request).unwrap();
            assert!(json.contains("\"method\":\"GET\""));
            assert!(json.contains("\"url\":\"https://example.com/test\""));
        }

        #[test]
        fn test_request_data_deserialization() {
            let json = r#"{
                "method": "POST",
                "url": "https://api.example.com/users",
                "host": "api.example.com",
                "path": "/users",
                "headers": {"Authorization": "Bearer token"},
                "body": "{\"name\":\"test\"}",
                "content_type": "application/json"
            }"#;

            let request: RequestData = serde_json::from_str(json).unwrap();
            assert_eq!(request.method, HttpMethod::Post);
            assert_eq!(request.body, Some(b"{\"name\":\"test\"}".to_vec()));
        }
    }

    mod response_data_tests {
        use super::*;

        #[test]
        fn test_response_data_creation() {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());

            let response = ResponseData {
                status_code: 200,
                status_message: Some("OK".to_string()),
                headers: headers.clone(),
                body: Some(b"{\"success\":true}".to_vec()),
                content_type: Some("application/json".to_string()),
                duration_ms: 150,
            };

            assert_eq!(response.status_code, 200);
            assert_eq!(response.status_message, Some("OK".to_string()));
            assert_eq!(response.duration_ms, 150);
        }

        #[test]
        fn test_response_data_serialization() {
            let response = ResponseData {
                status_code: 404,
                status_message: Some("Not Found".to_string()),
                headers: HashMap::new(),
                body: None,
                content_type: None,
                duration_ms: 50,
            };

            let json = serde_json::to_string(&response).unwrap();
            assert!(json.contains("\"status_code\":404"));
            assert!(json.contains("\"duration_ms\":50"));
        }
    }

    mod body_serialization_tests {
        use super::*;

        // Helper struct to test body serialization through the custom serializer
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct BodyWrapper {
            #[serde(
                serialize_with = "serialize_body",
                deserialize_with = "deserialize_body"
            )]
            body: Option<Vec<u8>>,
        }

        #[test]
        fn test_serialize_utf8_body() {
            let wrapper = BodyWrapper {
                body: Some(b"Hello, World!".to_vec()),
            };
            let json = serde_json::to_string(&wrapper).unwrap();
            assert!(json.contains("\"body\":\"Hello, World!\""));
        }

        #[test]
        fn test_serialize_none_body() {
            let wrapper = BodyWrapper { body: None };
            let json = serde_json::to_string(&wrapper).unwrap();
            assert!(json.contains("\"body\":null"));
        }

        #[test]
        fn test_serialize_binary_body_as_base64() {
            let wrapper = BodyWrapper {
                body: Some(vec![0x00, 0x01, 0x02, 0xFF, 0xFE]),
            };
            let json = serde_json::to_string(&wrapper).unwrap();
            assert!(json.contains("\"body\":\"base64:"));
        }

        #[test]
        fn test_deserialize_utf8_body() {
            let json = r#"{"body":"Test content"}"#;
            let wrapper: BodyWrapper = serde_json::from_str(json).unwrap();
            assert_eq!(wrapper.body, Some(b"Test content".to_vec()));
        }

        #[test]
        fn test_deserialize_base64_body() {
            let json = r#"{"body":"base64:SGVsbG8gV29ybGQ="}"#;
            let wrapper: BodyWrapper = serde_json::from_str(json).unwrap();
            assert_eq!(wrapper.body, Some(b"Hello World".to_vec()));
        }

        #[test]
        fn test_deserialize_null_body() {
            let json = r#"{"body":null}"#;
            let wrapper: BodyWrapper = serde_json::from_str(json).unwrap();
            assert!(wrapper.body.is_none());
        }

        #[test]
        fn test_roundtrip_utf8_body() {
            let original = BodyWrapper {
                body: Some("Test UTF-8 content with émojis 🎉".as_bytes().to_vec()),
            };
            let json = serde_json::to_string(&original).unwrap();
            let decoded: BodyWrapper = serde_json::from_str(&json).unwrap();
            assert_eq!(original, decoded);
        }

        #[test]
        fn test_roundtrip_binary_body() {
            let original = BodyWrapper {
                body: Some(vec![0x00, 0x01, 0x02, 0x80, 0xFF]),
            };
            let json = serde_json::to_string(&original).unwrap();
            let decoded: BodyWrapper = serde_json::from_str(&json).unwrap();
            assert_eq!(original, decoded);
        }
    }

    mod traffic_entry_tests {
        use super::*;

        fn create_test_request() -> RequestData {
            RequestData {
                method: HttpMethod::Get,
                url: "https://example.com/api/data".to_string(),
                host: "example.com".to_string(),
                path: "/api/data".to_string(),
                headers: HashMap::new(),
                body: Some(b"request body".to_vec()),
                content_type: None,
            }
        }

        fn create_test_response() -> ResponseData {
            ResponseData {
                status_code: 200,
                status_message: Some("OK".to_string()),
                headers: HashMap::new(),
                body: Some(b"response body".to_vec()),
                content_type: Some("application/json".to_string()),
                duration_ms: 100,
            }
        }

        #[test]
        fn test_traffic_entry_new() {
            let request = create_test_request();
            let entry = TrafficEntry::new("session-123", request.clone());

            assert!(!entry.id.is_empty());
            assert_eq!(entry.session_id, "session-123");
            assert_eq!(entry.request.method, HttpMethod::Get);
            assert!(entry.response.is_none());
            assert!(!entry.modified);
            assert!(entry.notes.is_none());
        }

        #[test]
        fn test_traffic_entry_total_size() {
            let request = create_test_request();
            let mut entry = TrafficEntry::new("session-1", request);

            // Without response, only request body size
            assert_eq!(entry.total_size(), 12); // "request body".len() = 12

            // With response
            entry.response = Some(create_test_response());
            assert_eq!(entry.total_size(), 25); // 12 + 13 ("response body".len() = 13)
        }

        #[test]
        fn test_traffic_entry_total_size_no_bodies() {
            let request = RequestData {
                method: HttpMethod::Get,
                url: "https://example.com/".to_string(),
                host: "example.com".to_string(),
                path: "/".to_string(),
                headers: HashMap::new(),
                body: None,
                content_type: None,
            };
            let entry = TrafficEntry::new("session-1", request);

            assert_eq!(entry.total_size(), 0);
        }

        #[test]
        fn test_traffic_entry_is_https() {
            let https_request = RequestData {
                method: HttpMethod::Get,
                url: "https://secure.example.com/api".to_string(),
                host: "secure.example.com".to_string(),
                path: "/api".to_string(),
                headers: HashMap::new(),
                body: None,
                content_type: None,
            };
            let https_entry = TrafficEntry::new("session-1", https_request);
            assert!(https_entry.is_https());

            let http_request = RequestData {
                method: HttpMethod::Get,
                url: "http://insecure.example.com/api".to_string(),
                host: "insecure.example.com".to_string(),
                path: "/api".to_string(),
                headers: HashMap::new(),
                body: None,
                content_type: None,
            };
            let http_entry = TrafficEntry::new("session-2", http_request);
            assert!(!http_entry.is_https());
        }

        #[test]
        fn test_traffic_entry_serialization() {
            let request = create_test_request();
            let entry = TrafficEntry::new("session-123", request);

            let json = serde_json::to_string(&entry).unwrap();
            assert!(json.contains("\"session_id\":\"session-123\""));
            assert!(json.contains("\"modified\":false"));

            let decoded: TrafficEntry = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.session_id, "session-123");
            assert_eq!(decoded.id, entry.id);
        }

        #[test]
        fn test_traffic_entry_with_response() {
            let request = create_test_request();
            let mut entry = TrafficEntry::new("session-1", request);
            entry.response = Some(create_test_response());
            entry.modified = true;
            entry.notes = Some("Test note".to_string());

            let json = serde_json::to_string(&entry).unwrap();
            let decoded: TrafficEntry = serde_json::from_str(&json).unwrap();

            assert!(decoded.response.is_some());
            assert_eq!(decoded.response.unwrap().status_code, 200);
            assert!(decoded.modified);
            assert_eq!(decoded.notes, Some("Test note".to_string()));
        }
    }

    mod session_tests {
        use super::*;

        #[test]
        fn test_session_new() {
            let session = Session::new(Some("My Session"));

            assert!(!session.id.is_empty());
            assert_eq!(session.name, Some("My Session".to_string()));
            assert_eq!(session.created_at, session.updated_at);
        }

        #[test]
        fn test_session_new_without_name() {
            let session = Session::new(None);

            assert!(!session.id.is_empty());
            assert!(session.name.is_none());
        }

        #[test]
        fn test_session_serialization() {
            let session = Session::new(Some("Test Session"));
            let json = serde_json::to_string(&session).unwrap();

            assert!(json.contains("\"name\":\"Test Session\""));

            let decoded: Session = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.name, Some("Test Session".to_string()));
            assert_eq!(decoded.id, session.id);
        }

        #[test]
        fn test_session_unique_ids() {
            let session1 = Session::new(None);
            let session2 = Session::new(None);

            assert_ne!(session1.id, session2.id);
        }
    }

    mod traffic_filter_tests {
        use super::*;

        #[test]
        fn test_traffic_filter_default() {
            let filter = TrafficFilter::default();

            assert!(filter.url_pattern.is_none());
            assert!(filter.method.is_none());
            assert!(filter.status_min.is_none());
            assert!(filter.status_max.is_none());
            assert!(filter.search.is_none());
            assert!(filter.limit.is_none());
            assert!(filter.offset.is_none());
            assert!(filter.file_type.is_none());
            assert!(filter.header.is_none());
            assert!(filter.cookie.is_none());
        }

        #[test]
        fn test_traffic_filter_with_fields() {
            let filter = TrafficFilter {
                url_pattern: Some("api".to_string()),
                method: Some(HttpMethod::Post),
                status_min: Some(200),
                status_max: Some(299),
                search: Some("error".to_string()),
                limit: Some(100),
                offset: Some(0),
                file_type: Some(".json".to_string()),
                header: Some("Authorization".to_string()),
                cookie: Some("session".to_string()),
            };

            assert_eq!(filter.url_pattern, Some("api".to_string()));
            assert_eq!(filter.method, Some(HttpMethod::Post));
            assert_eq!(filter.status_min, Some(200));
            assert_eq!(filter.status_max, Some(299));
            assert_eq!(filter.limit, Some(100));
        }

        #[test]
        fn test_traffic_filter_serialization() {
            let filter = TrafficFilter {
                method: Some(HttpMethod::Get),
                limit: Some(50),
                ..Default::default()
            };

            let json = serde_json::to_string(&filter).unwrap();
            assert!(json.contains("\"method\":\"GET\""));
            assert!(json.contains("\"limit\":50"));

            let decoded: TrafficFilter = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.method, Some(HttpMethod::Get));
            assert_eq!(decoded.limit, Some(50));
        }
    }
}

//! Request replay functionality

use crate::traffic::{HttpMethod, RequestData, ResponseData};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A saved request for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    /// Unique identifier
    pub id: String,
    /// Original traffic entry ID (if any)
    pub source_entry_id: Option<String>,
    /// Request name/description
    pub name: Option<String>,
    /// The request data
    pub request: RequestData,
    /// When it was saved
    pub saved_at: DateTime<Utc>,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Collection/folder it belongs to
    pub collection: Option<String>,
}

impl SavedRequest {
    pub fn from_traffic(entry_id: &str, request: RequestData) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_entry_id: Some(entry_id.to_string()),
            name: None,
            request,
            saved_at: Utc::now(),
            tags: Vec::new(),
            collection: None,
        }
    }

    pub fn new(name: Option<&str>, request: RequestData) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_entry_id: None,
            name: name.map(|s| s.to_string()),
            request,
            saved_at: Utc::now(),
            tags: Vec::new(),
            collection: None,
        }
    }
}

/// Result of a replay operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Unique identifier for this replay
    pub id: String,
    /// ID of the saved request that was replayed
    pub saved_request_id: String,
    /// The request that was sent (may be modified from original)
    pub request: RequestData,
    /// The response received
    pub response: Option<ResponseData>,
    /// Error message if replay failed
    pub error: Option<String>,
    /// When the replay was executed
    pub executed_at: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl ReplayResult {
    pub fn success(
        saved_request_id: &str,
        request: RequestData,
        response: ResponseData,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            saved_request_id: saved_request_id.to_string(),
            request,
            response: Some(response),
            error: None,
            executed_at: Utc::now(),
            duration_ms,
        }
    }

    pub fn error(saved_request_id: &str, request: RequestData, error: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            saved_request_id: saved_request_id.to_string(),
            request,
            response: None,
            error: Some(error),
            executed_at: Utc::now(),
            duration_ms: 0,
        }
    }
}

/// Manages saved requests and replay functionality
pub struct ReplayManager {
    /// Saved requests
    saved_requests: std::sync::Arc<parking_lot::RwLock<Vec<SavedRequest>>>,
    /// Replay history
    history: std::sync::Arc<parking_lot::RwLock<Vec<ReplayResult>>>,
}

impl ReplayManager {
    pub fn new() -> Self {
        Self {
            saved_requests: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
            history: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
    }

    /// Save a request for later replay
    pub fn save_request(&self, request: SavedRequest) -> String {
        let id = request.id.clone();
        self.saved_requests.write().push(request);
        id
    }

    /// Save from a traffic entry
    pub fn save_from_entry(
        &self,
        entry_id: &str,
        request: RequestData,
        name: Option<&str>,
    ) -> String {
        let mut saved = SavedRequest::from_traffic(entry_id, request);
        saved.name = name.map(|s| s.to_string());
        self.save_request(saved)
    }

    /// Remove a saved request
    pub fn remove_request(&self, id: &str) -> bool {
        let mut requests = self.saved_requests.write();
        if let Some(pos) = requests.iter().position(|r| r.id == id) {
            requests.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all saved requests
    pub fn get_saved_requests(&self) -> Vec<SavedRequest> {
        self.saved_requests.read().clone()
    }

    /// Get a specific saved request
    pub fn get_request(&self, id: &str) -> Option<SavedRequest> {
        self.saved_requests
            .read()
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }

    /// Update a saved request
    pub fn update_request(&self, id: &str, request: SavedRequest) -> bool {
        let mut requests = self.saved_requests.write();
        if let Some(pos) = requests.iter().position(|r| r.id == id) {
            requests[pos] = request;
            true
        } else {
            false
        }
    }

    /// Get replay history
    pub fn get_history(&self) -> Vec<ReplayResult> {
        self.history.read().clone()
    }

    /// Clear replay history
    pub fn clear_history(&self) {
        self.history.write().clear();
    }

    /// Replay a saved request
    pub async fn replay(
        &self,
        id: &str,
        modifications: Option<RequestModifications>,
    ) -> ReplayResult {
        let saved = match self.get_request(id) {
            Some(s) => s,
            None => {
                return ReplayResult::error(
                    id,
                    RequestData {
                        method: HttpMethod::Get,
                        url: String::new(),
                        host: String::new(),
                        path: String::new(),
                        headers: HashMap::new(),
                        body: None,
                        content_type: None,
                    },
                    "Saved request not found".to_string(),
                );
            }
        };

        // Clone and modify the request
        let mut request = saved.request.clone();
        if let Some(mods) = modifications {
            mods.apply(&mut request);
        }

        let start = std::time::Instant::now();

        // Execute the request
        match self.execute_request(&request).await {
            Ok(response) => {
                let result = ReplayResult::success(
                    id,
                    request.clone(),
                    response,
                    start.elapsed().as_millis() as u64,
                );
                self.history.write().push(result.clone());
                result
            }
            Err(e) => {
                let result = ReplayResult::error(id, request, e.to_string());
                self.history.write().push(result.clone());
                result
            }
        }
    }

    /// Execute an HTTP request
    async fn execute_request(&self, request: &RequestData) -> crate::Result<ResponseData> {
        let is_https = request.url.starts_with("https://");
        let port = if is_https { 443 } else { 80 };

        // Connect to the target server
        let addr = format!("{}:{}", request.host, port);
        let stream = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|e| crate::Error::Proxy(format!("Failed to connect to {}: {}", addr, e)))?;

        if is_https {
            self.execute_https_request(stream, request).await
        } else {
            self.execute_http_request(stream, request).await
        }
    }

    async fn execute_http_request(
        &self,
        mut stream: tokio::net::TcpStream,
        request: &RequestData,
    ) -> crate::Result<ResponseData> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Build and send request
        let request_bytes = self.build_request_bytes(request);
        stream.write_all(&request_bytes).await?;

        // Read response
        let mut response_buf = vec![0u8; 65536];
        let n = stream
            .read(&mut response_buf)
            .await
            .map_err(|e| crate::Error::Proxy(format!("Failed to read response: {}", e)))?;

        self.parse_response(&response_buf[..n])
    }

    async fn execute_https_request(
        &self,
        stream: tokio::net::TcpStream,
        request: &RequestData,
    ) -> crate::Result<ResponseData> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Create TLS connector that skips verification (for proxy use)
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(SkipServerVerification::new()))
            .with_no_client_auth();

        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let server_name = rustls::pki_types::ServerName::try_from(request.host.clone())
            .map_err(|e| crate::Error::Tls(format!("Invalid server name: {}", e)))?;

        let mut tls_stream = connector
            .connect(server_name, stream)
            .await
            .map_err(|e| crate::Error::Tls(format!("TLS connection failed: {}", e)))?;

        // Build and send request
        let request_bytes = self.build_request_bytes(request);
        tls_stream.write_all(&request_bytes).await?;

        // Read response
        let mut response_buf = vec![0u8; 65536];
        let n = tls_stream
            .read(&mut response_buf)
            .await
            .map_err(|e| crate::Error::Proxy(format!("Failed to read response: {}", e)))?;

        self.parse_response(&response_buf[..n])
    }

    fn build_request_bytes(&self, request: &RequestData) -> Vec<u8> {
        let mut req_str = format!("{} {} HTTP/1.1\r\n", request.method, request.path);

        // Add host header if not present
        if !request
            .headers
            .keys()
            .any(|k| k.eq_ignore_ascii_case("host"))
        {
            req_str.push_str(&format!("Host: {}\r\n", request.host));
        }

        for (key, value) in &request.headers {
            req_str.push_str(&format!("{}: {}\r\n", key, value));
        }

        req_str.push_str("Connection: close\r\n");
        req_str.push_str("\r\n");

        let mut bytes = req_str.into_bytes();
        if let Some(ref body) = request.body {
            bytes.extend(body);
        }

        bytes
    }

    fn parse_response(&self, data: &[u8]) -> crate::Result<ResponseData> {
        let response_str = String::from_utf8_lossy(data);
        let mut lines = response_str.lines();

        // Parse status line
        let status_line = lines.next().unwrap_or("");
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();

        if parts.len() < 2 {
            return Err(crate::Error::Proxy("Invalid response line".into()));
        }

        // Validate HTTP version
        if !parts[0].starts_with("HTTP/") {
            return Err(crate::Error::Proxy("Invalid HTTP response".into()));
        }

        let status_code: u16 = parts[1]
            .parse()
            .map_err(|_| crate::Error::Proxy("Invalid status code".into()))?;

        // Status message is everything after the status code (may contain spaces)
        let status_message = parts.get(2).map(|s| s.to_string());

        // Parse headers
        let mut headers = HashMap::new();
        let mut content_type = None;

        for line in lines {
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();

                if key.eq_ignore_ascii_case("content-type") {
                    content_type = Some(value.clone());
                }

                headers.insert(key, value);
            }
        }

        // Extract body
        let body = data
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| data[p + 4..].to_vec());

        Ok(ResponseData {
            status_code,
            status_message,
            headers,
            body,
            content_type,
            duration_ms: 0,
        })
    }
}

impl Default for ReplayManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Modifications to apply before replaying a request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestModifications {
    /// New URL
    pub url: Option<String>,
    /// New method
    pub method: Option<HttpMethod>,
    /// Headers to add/replace
    pub headers: HashMap<String, String>,
    /// Headers to remove
    pub remove_headers: Vec<String>,
    /// New body
    pub body: Option<String>,
}

impl RequestModifications {
    pub fn apply(&self, request: &mut RequestData) {
        if let Some(url) = &self.url {
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

        if let Some(method) = &self.method {
            request.method = *method;
        }

        for key in &self.remove_headers {
            request.headers.remove(key);
        }

        for (key, value) in &self.headers {
            request.headers.insert(key.clone(), value.clone());
        }

        if let Some(body) = &self.body {
            request.body = Some(body.as_bytes().to_vec());
        }
    }
}

/// Skip server certificate verification
#[derive(Debug)]
struct SkipServerVerification {
    supported_schemes: Vec<rustls::SignatureScheme>,
}

impl SkipServerVerification {
    fn new() -> Self {
        Self {
            supported_schemes: rustls::crypto::ring::default_provider()
                .signature_verification_algorithms
                .supported_schemes(),
        }
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.supported_schemes.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request() -> RequestData {
        RequestData {
            method: HttpMethod::Get,
            url: "https://api.example.com/users".to_string(),
            host: "api.example.com".to_string(),
            path: "/users".to_string(),
            headers: HashMap::new(),
            body: None,
            content_type: None,
        }
    }

    mod saved_request_tests {
        use super::*;

        #[test]
        fn test_from_traffic() {
            let request = create_test_request();
            let saved = SavedRequest::from_traffic("entry-123", request.clone());

            assert!(!saved.id.is_empty());
            assert_eq!(saved.source_entry_id, Some("entry-123".to_string()));
            assert!(saved.name.is_none());
            assert!(saved.tags.is_empty());
            assert!(saved.collection.is_none());
            assert_eq!(saved.request.method, HttpMethod::Get);
        }

        #[test]
        fn test_new() {
            let request = create_test_request();
            let saved = SavedRequest::new(Some("My Request"), request.clone());

            assert!(!saved.id.is_empty());
            assert!(saved.source_entry_id.is_none());
            assert_eq!(saved.name, Some("My Request".to_string()));
        }

        #[test]
        fn test_unique_ids() {
            let request = create_test_request();
            let saved1 = SavedRequest::new(None, request.clone());
            let saved2 = SavedRequest::new(None, request);
            assert_ne!(saved1.id, saved2.id);
        }

        #[test]
        fn test_serialization() {
            let request = create_test_request();
            let saved = SavedRequest {
                id: "test-id".to_string(),
                source_entry_id: Some("entry-456".to_string()),
                name: Some("Test Request".to_string()),
                request: request.clone(),
                saved_at: Utc::now(),
                tags: vec!["api".to_string(), "test".to_string()],
                collection: Some("API Tests".to_string()),
            };

            let json = serde_json::to_string(&saved).unwrap();
            assert!(json.contains("\"id\":\"test-id\""));
            assert!(json.contains("\"name\":\"Test Request\""));
            assert!(json.contains("\"tags\":[\"api\",\"test\"]"));
            assert!(json.contains("\"collection\":\"API Tests\""));

            let decoded: SavedRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.id, "test-id");
            assert_eq!(decoded.name, Some("Test Request".to_string()));
            assert_eq!(decoded.tags, vec!["api", "test"]);
        }
    }

    mod replay_result_tests {
        use super::*;

        #[test]
        fn test_success() {
            let request = create_test_request();
            let response = ResponseData {
                status_code: 200,
                status_message: Some("OK".to_string()),
                headers: HashMap::new(),
                body: Some(b"response body".to_vec()),
                content_type: Some("text/plain".to_string()),
                duration_ms: 150,
            };

            let result = ReplayResult::success("saved-123", request.clone(), response.clone(), 150);

            assert!(!result.id.is_empty());
            assert_eq!(result.saved_request_id, "saved-123");
            assert!(result.response.is_some());
            assert!(result.error.is_none());
            assert_eq!(result.duration_ms, 150);
        }

        #[test]
        fn test_error() {
            let request = create_test_request();
            let result = ReplayResult::error(
                "saved-456",
                request.clone(),
                "Connection failed".to_string(),
            );

            assert!(!result.id.is_empty());
            assert_eq!(result.saved_request_id, "saved-456");
            assert!(result.response.is_none());
            assert_eq!(result.error, Some("Connection failed".to_string()));
            assert_eq!(result.duration_ms, 0);
        }

        #[test]
        fn test_serialization() {
            let request = create_test_request();
            let response = ResponseData {
                status_code: 201,
                status_message: Some("Created".to_string()),
                headers: HashMap::new(),
                body: None,
                content_type: None,
                duration_ms: 50,
            };

            let result = ReplayResult::success("req-id", request, response, 50);
            let json = serde_json::to_string(&result).unwrap();

            assert!(json.contains("\"saved_request_id\":\"req-id\""));
            assert!(json.contains("\"duration_ms\":50"));

            let decoded: ReplayResult = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.saved_request_id, "req-id");
            assert_eq!(decoded.duration_ms, 50);
        }
    }

    mod request_modifications_tests {
        use super::*;

        #[test]
        fn test_default() {
            let mods = RequestModifications::default();

            assert!(mods.url.is_none());
            assert!(mods.method.is_none());
            assert!(mods.headers.is_empty());
            assert!(mods.remove_headers.is_empty());
            assert!(mods.body.is_none());
        }

        #[test]
        fn test_apply_url_change() {
            let mods = RequestModifications {
                url: Some("https://newhost.example.com/newpath".to_string()),
                ..Default::default()
            };

            let mut request = create_test_request();
            mods.apply(&mut request);

            assert_eq!(request.url, "https://newhost.example.com/newpath");
            assert_eq!(request.host, "newhost.example.com");
            assert_eq!(request.path, "/newpath");
        }

        #[test]
        fn test_apply_method_change() {
            let mods = RequestModifications {
                method: Some(HttpMethod::Post),
                ..Default::default()
            };

            let mut request = create_test_request();
            mods.apply(&mut request);

            assert_eq!(request.method, HttpMethod::Post);
        }

        #[test]
        fn test_apply_headers() {
            let mut headers = HashMap::new();
            headers.insert("Authorization".to_string(), "Bearer token".to_string());

            let mods = RequestModifications {
                headers,
                ..Default::default()
            };

            let mut request = create_test_request();
            mods.apply(&mut request);

            assert_eq!(
                request.headers.get("Authorization"),
                Some(&"Bearer token".to_string())
            );
        }

        #[test]
        fn test_apply_remove_headers() {
            let mods = RequestModifications {
                remove_headers: vec!["X-Remove-Me".to_string()],
                ..Default::default()
            };

            let mut request = create_test_request();
            request
                .headers
                .insert("X-Remove-Me".to_string(), "value".to_string());
            request
                .headers
                .insert("X-Keep-Me".to_string(), "keep".to_string());

            mods.apply(&mut request);

            assert!(!request.headers.contains_key("X-Remove-Me"));
            assert!(request.headers.contains_key("X-Keep-Me"));
        }

        #[test]
        fn test_apply_body_change() {
            let mods = RequestModifications {
                body: Some(r#"{"updated": true}"#.to_string()),
                ..Default::default()
            };

            let mut request = create_test_request();
            mods.apply(&mut request);

            let body = String::from_utf8(request.body.unwrap()).unwrap();
            assert_eq!(body, r#"{"updated": true}"#);
        }

        #[test]
        fn test_apply_multiple_modifications() {
            let mut headers = HashMap::new();
            headers.insert("X-Custom".to_string(), "value".to_string());

            let mods = RequestModifications {
                url: Some("https://modified.example.com/api".to_string()),
                method: Some(HttpMethod::Put),
                headers,
                remove_headers: vec!["X-Old".to_string()],
                body: Some(r#"{"test": 1}"#.to_string()),
            };

            let mut request = RequestData {
                method: HttpMethod::Get,
                url: "https://original.example.com/old".to_string(),
                host: "original.example.com".to_string(),
                path: "/old".to_string(),
                headers: {
                    let mut h = HashMap::new();
                    h.insert("X-Old".to_string(), "old-value".to_string());
                    h
                },
                body: None,
                content_type: None,
            };

            mods.apply(&mut request);

            assert_eq!(request.method, HttpMethod::Put);
            assert_eq!(request.url, "https://modified.example.com/api");
            assert_eq!(request.host, "modified.example.com");
            assert_eq!(request.path, "/api");
            assert!(request.headers.contains_key("X-Custom"));
            assert!(!request.headers.contains_key("X-Old"));
            assert!(request.body.is_some());
        }

        #[test]
        fn test_serialization() {
            let mut headers = HashMap::new();
            headers.insert("Auth".to_string(), "token".to_string());

            let mods = RequestModifications {
                url: Some("https://new.example.com".to_string()),
                method: Some(HttpMethod::Post),
                headers,
                remove_headers: vec!["Old".to_string()],
                body: Some("new body".to_string()),
            };

            let json = serde_json::to_string(&mods).unwrap();
            assert!(json.contains("\"url\":\"https://new.example.com\""));
            assert!(json.contains("\"method\":\"POST\""));

            let decoded: RequestModifications = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.url, Some("https://new.example.com".to_string()));
            assert_eq!(decoded.method, Some(HttpMethod::Post));
        }
    }

    mod replay_manager_tests {
        use super::*;

        #[test]
        fn test_new() {
            let manager = ReplayManager::new();
            assert!(manager.get_saved_requests().is_empty());
            assert!(manager.get_history().is_empty());
        }

        #[test]
        fn test_default() {
            let manager = ReplayManager::default();
            assert!(manager.get_saved_requests().is_empty());
        }

        #[test]
        fn test_save_request() {
            let manager = ReplayManager::new();
            let request = create_test_request();
            let saved = SavedRequest::new(Some("Test"), request);

            let id = manager.save_request(saved);
            assert!(!id.is_empty());
            assert_eq!(manager.get_saved_requests().len(), 1);
        }

        #[test]
        fn test_save_from_entry() {
            let manager = ReplayManager::new();
            let request = create_test_request();

            let id = manager.save_from_entry("entry-123", request, Some("From Entry"));
            assert!(!id.is_empty());

            let saved = manager.get_request(&id).unwrap();
            assert_eq!(saved.source_entry_id, Some("entry-123".to_string()));
            assert_eq!(saved.name, Some("From Entry".to_string()));
        }

        #[test]
        fn test_remove_request() {
            let manager = ReplayManager::new();
            let request = create_test_request();
            let saved = SavedRequest::new(None, request);

            let id = manager.save_request(saved);
            assert!(manager.remove_request(&id));
            assert!(manager.get_saved_requests().is_empty());
        }

        #[test]
        fn test_remove_nonexistent_request() {
            let manager = ReplayManager::new();
            assert!(!manager.remove_request("nonexistent"));
        }

        #[test]
        fn test_get_request() {
            let manager = ReplayManager::new();
            let request = create_test_request();
            let saved = SavedRequest::new(Some("Find Me"), request);

            let id = manager.save_request(saved);
            let found = manager.get_request(&id).unwrap();

            assert_eq!(found.name, Some("Find Me".to_string()));
        }

        #[test]
        fn test_get_nonexistent_request() {
            let manager = ReplayManager::new();
            assert!(manager.get_request("nonexistent").is_none());
        }

        #[test]
        fn test_update_request() {
            let manager = ReplayManager::new();
            let request = create_test_request();
            let saved = SavedRequest::new(Some("Original"), request.clone());

            let id = manager.save_request(saved);

            let mut updated = SavedRequest::new(Some("Updated"), request);
            updated.id = id.clone();
            assert!(manager.update_request(&id, updated));

            let found = manager.get_request(&id).unwrap();
            assert_eq!(found.name, Some("Updated".to_string()));
        }

        #[test]
        fn test_update_nonexistent_request() {
            let manager = ReplayManager::new();
            let request = create_test_request();
            let saved = SavedRequest::new(None, request);
            assert!(!manager.update_request("nonexistent", saved));
        }

        #[test]
        fn test_clear_history() {
            let manager = ReplayManager::new();

            // Add a result to history manually
            let request = create_test_request();
            let result = ReplayResult::error("test", request, "error".to_string());
            manager.history.write().push(result);

            assert!(!manager.get_history().is_empty());
            manager.clear_history();
            assert!(manager.get_history().is_empty());
        }

        #[test]
        fn test_build_request_bytes() {
            let manager = ReplayManager::new();

            let mut request = create_test_request();
            request
                .headers
                .insert("Authorization".to_string(), "Bearer token".to_string());

            let bytes = manager.build_request_bytes(&request);
            let request_str = String::from_utf8_lossy(&bytes);

            assert!(request_str.starts_with("GET /users HTTP/1.1\r\n"));
            assert!(request_str.contains("Host: api.example.com"));
            assert!(request_str.contains("Authorization: Bearer token"));
            assert!(request_str.contains("Connection: close"));
            assert!(request_str.ends_with("\r\n\r\n"));
        }

        #[test]
        fn test_build_request_bytes_with_body() {
            let manager = ReplayManager::new();

            let mut request = RequestData {
                method: HttpMethod::Post,
                url: "https://example.com/api".to_string(),
                host: "example.com".to_string(),
                path: "/api".to_string(),
                headers: HashMap::new(),
                body: Some(b"request body data".to_vec()),
                content_type: None,
            };

            let bytes = manager.build_request_bytes(&request);
            let body_start = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            let body = &bytes[body_start + 4..];

            assert_eq!(body, b"request body data");
        }

        #[test]
        fn test_parse_response() {
            let manager = ReplayManager::new();

            let response_data = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\n\r\n{\"status\":\"ok\"}";
            let response = manager.parse_response(response_data).unwrap();

            assert_eq!(response.status_code, 200);
            assert_eq!(response.status_message, Some("OK".to_string()));
            assert_eq!(
                response.headers.get("Content-Type"),
                Some(&"application/json".to_string())
            );
            assert_eq!(response.body, Some(br#"{"status":"ok"}"#.to_vec()));
        }

        #[test]
        fn test_parse_response_error_status() {
            let manager = ReplayManager::new();

            let response_data =
                b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nNot Found";
            let response = manager.parse_response(response_data).unwrap();

            assert_eq!(response.status_code, 404);
            assert_eq!(response.status_message, Some("Not Found".to_string()));
        }

        #[test]
        fn test_parse_response_invalid() {
            let manager = ReplayManager::new();

            let result = manager.parse_response(b"Invalid Response");
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_replay_nonexistent_request() {
            let manager = ReplayManager::new();

            let result = manager.replay("nonexistent-id", None).await;

            assert!(result.response.is_none());
            assert_eq!(result.error, Some("Saved request not found".to_string()));
            assert_eq!(result.saved_request_id, "nonexistent-id");
        }
    }
}

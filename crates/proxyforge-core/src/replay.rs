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
        let parts: Vec<&str> = status_line.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(crate::Error::Proxy("Invalid response line".into()));
        }

        let status_code: u16 = parts[1].parse().unwrap_or(0);
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

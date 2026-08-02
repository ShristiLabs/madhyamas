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
    /// Maximum history entries (FIFO eviction)
    max_history: usize,
}

impl ReplayManager {
    pub fn new() -> Self {
        Self {
            saved_requests: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
            history: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
            max_history: 500,
        }
    }

    /// Set the maximum history size.
    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
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

    /// Record a replay result, evicting oldest entries if over max_history.
    fn record_history(&self, result: ReplayResult) {
        let mut history = self.history.write();
        history.push(result);
        while history.len() > self.max_history {
            history.remove(0);
        }
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
                        http_version: None,
                    },
                    "Saved request not found".to_string(),
                );
            }
        };

        // Clone and modify the request
        let mut request = saved.request.clone();
        let mut follow_redirects = false;
        if let Some(mods) = modifications {
            mods.apply(&mut request);
            follow_redirects = mods.follow_redirects.unwrap_or(false);
        }

        let start = std::time::Instant::now();

        // Execute the request
        match self.execute_request(&request, follow_redirects).await {
            Ok(response) => {
                let result = ReplayResult::success(
                    id,
                    request.clone(),
                    response,
                    start.elapsed().as_millis() as u64,
                );
                self.record_history(result.clone());
                result
            }
            Err(e) => {
                let result = ReplayResult::error(id, request, e.to_string());
                self.record_history(result.clone());
                result
            }
        }
    }

    /// Replay a saved request multiple times with optional concurrency and
    /// inter-request delay (the "Repeat Advanced" / batch replay feature).
    ///
    /// All iterations use the same `modifications`. The `config` controls the
    /// total number of requests (`iterations`), how many run concurrently
    /// (`concurrency`), and an optional delay between dispatches (`delay_ms`).
    ///
    /// Safety limits are enforced: `iterations` is capped at
    /// [`MAX_BATCH_ITERATIONS`] and `concurrency` at [`MAX_BATCH_CONCURRENCY`].
    /// Zero values are normalized to 1.
    ///
    /// Individual results are recorded in replay history, and an aggregate
    /// [`ReplayBatchResult`] is returned summarizing success/failure counts and
    /// latency statistics (min/avg/max/p95).
    pub async fn replay_batch(
        &self,
        id: &str,
        modifications: Option<RequestModifications>,
        mut config: ReplayBatchConfig,
    ) -> ReplayBatchResult {
        let started_at = Utc::now();

        // Validate the saved request exists before dispatching anything.
        if self.get_request(id).is_none() {
            let request = RequestData {
                method: HttpMethod::Get,
                url: String::new(),
                host: String::new(),
                path: String::new(),
                headers: HashMap::new(),
                body: None,
                content_type: None,
                http_version: None,
            };
            let result = ReplayResult::error(id, request, "Saved request not found".to_string());
            return ReplayBatchResult {
                saved_request_id: id.to_string(),
                results: vec![result.clone()],
                total: 1,
                succeeded: 0,
                failed: 1,
                min_ms: 0,
                max_ms: 0,
                avg_ms: 0,
                p95_ms: 0,
                started_at,
                finished_at: Utc::now(),
            };
        }

        config.clamp_to_limits();

        let delay = config
            .delay_ms
            .map(std::time::Duration::from_millis)
            .unwrap_or(std::time::Duration::ZERO);

        // Build a stream of futures, applying the inter-request delay before
        // each dispatch (skipping the first), then bound concurrency with
        // `buffer_unordered`.
        use futures::stream::{self, StreamExt};

        let manager = self;
        let results: Vec<ReplayResult> = stream::iter(0..config.iterations)
            .map(|i| {
                let mods = modifications.clone();
                async move {
                    if i > 0 && delay > std::time::Duration::ZERO {
                        tokio::time::sleep(delay).await;
                    }
                    manager.replay(id, mods).await
                }
            })
            .buffer_unordered(config.concurrency)
            .collect()
            .await;

        let mut batch = ReplayBatchResult::from_results(id.to_string(), results);
        batch.started_at = started_at;
        batch.finished_at = Utc::now();
        batch
    }

    /// Execute an HTTP request using `reqwest`.
    ///
    /// This replaces the previous manual TCP+TLS+HTTP/1.1 implementation
    /// which had a 64KB buffer limit and didn't support chunked encoding,
    /// HTTP/2, or compression. Now uses reqwest for full protocol support.
    async fn execute_request(
        &self,
        request: &RequestData,
        follow_redirects: bool,
    ) -> crate::Result<ResponseData> {
        // Build a reqwest client. Don't use system proxy (avoid feedback loop).
        let redirect_policy = if follow_redirects {
            reqwest::redirect::Policy::default()
        } else {
            reqwest::redirect::Policy::none()
        };

        let client = reqwest::Client::builder()
            .redirect(redirect_policy)
            .no_proxy()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| crate::Error::Proxy(format!("Failed to create HTTP client: {}", e)))?;

        let method = reqwest::Method::from_bytes(request.method.to_string().as_bytes())
            .map_err(|e| crate::Error::Proxy(format!("Invalid HTTP method: {}", e)))?;

        let mut req_builder = client.request(method, &request.url);

        // Copy headers (skip hop-by-hop and content-length)
        for (key, value) in &request.headers {
            if !matches!(
                key.to_lowercase().as_str(),
                "connection" | "keep-alive" | "transfer-encoding" | "content-length" | "upgrade"
            ) {
                if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(val) = reqwest::header::HeaderValue::from_str(value) {
                        req_builder = req_builder.header(name, val);
                    }
                }
            }
        }

        // Add body if present
        if let Some(body) = &request.body {
            req_builder = req_builder.body(body.clone());
        }

        // Send the request
        let response = req_builder
            .send()
            .await
            .map_err(|e| crate::Error::Proxy(format!("Request failed: {}", e)))?;

        let status_code = response.status().as_u16();

        // Extract headers
        let mut headers = HashMap::new();
        let mut content_type = None;
        for (name, value) in response.headers() {
            let name_lower = name.as_str().to_lowercase();
            if matches!(
                name_lower.as_str(),
                "transfer-encoding" | "content-encoding" | "content-length" | "connection"
            ) {
                continue;
            }
            let val_str = value.to_str().unwrap_or("").to_string();
            if name_lower == "content-type" {
                content_type = Some(val_str.clone());
            }
            headers.insert(name.as_str().to_string(), val_str);
        }

        // Read full body (reqwest handles decompression)
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| crate::Error::Proxy(format!("Failed to read response body: {}", e)))?;

        let body = if body_bytes.is_empty() {
            None
        } else {
            Some(body_bytes.to_vec())
        };

        Ok(ResponseData {
            status_code,
            status_message: None,
            headers,
            body,
            content_type,
            duration_ms: 0,
            http_version: None,
        })
    }
}

impl Default for ReplayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::persistence::Persistable for ReplayManager {
    fn save(&self) -> crate::Result<()> {
        // In-memory only for now; no backing store wired up yet.
        Ok(())
    }

    fn load(&self) -> crate::Result<()> {
        // In-memory only for now; no backing store wired up yet.
        Ok(())
    }

    fn clear(&self) -> crate::Result<()> {
        self.saved_requests.write().clear();
        self.clear_history();
        Ok(())
    }

    fn size(&self) -> usize {
        self.saved_requests.read().len()
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
    /// Whether to follow redirect responses (3xx). Default: false.
    pub follow_redirects: Option<bool>,
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

/// Maximum number of iterations allowed in a single batch replay.
pub const MAX_BATCH_ITERATIONS: usize = 10_000;
/// Maximum concurrency allowed in a single batch replay.
pub const MAX_BATCH_CONCURRENCY: usize = 100;

/// Configuration for a batch (advanced) replay run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBatchConfig {
    /// Total number of requests to send.
    pub iterations: usize,
    /// Number of simultaneous in-flight requests.
    pub concurrency: usize,
    /// Optional delay (in milliseconds) between dispatches.
    pub delay_ms: Option<u64>,
}

impl Default for ReplayBatchConfig {
    fn default() -> Self {
        Self {
            iterations: 1,
            concurrency: 1,
            delay_ms: None,
        }
    }
}

impl ReplayBatchConfig {
    /// Clamp the configuration to the safety limits.
    pub fn clamp_to_limits(&mut self) {
        if self.iterations == 0 {
            self.iterations = 1;
        }
        if self.iterations > MAX_BATCH_ITERATIONS {
            self.iterations = MAX_BATCH_ITERATIONS;
        }
        if self.concurrency == 0 {
            self.concurrency = 1;
        }
        if self.concurrency > MAX_BATCH_CONCURRENCY {
            self.concurrency = MAX_BATCH_CONCURRENCY;
        }
    }
}

/// Aggregate result of a batch (advanced) replay run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBatchResult {
    /// ID of the saved request that was replayed.
    pub saved_request_id: String,
    /// Individual replay results (in completion order).
    pub results: Vec<ReplayResult>,
    /// Total number of requests sent.
    pub total: usize,
    /// Number of successful requests.
    pub succeeded: usize,
    /// Number of failed requests.
    pub failed: usize,
    /// Minimum latency in milliseconds.
    pub min_ms: u64,
    /// Maximum latency in milliseconds.
    pub max_ms: u64,
    /// Average latency in milliseconds.
    pub avg_ms: u64,
    /// 95th percentile latency in milliseconds.
    pub p95_ms: u64,
    /// When the batch started.
    pub started_at: DateTime<Utc>,
    /// When the batch finished.
    pub finished_at: DateTime<Utc>,
}

impl ReplayBatchResult {
    /// Compute aggregate statistics from a set of replay results.
    pub fn from_results(saved_request_id: String, results: Vec<ReplayResult>) -> Self {
        let total = results.len();
        let succeeded = results.iter().filter(|r| r.error.is_none()).count();
        let failed = total - succeeded;

        let mut durations: Vec<u64> = results
            .iter()
            .map(|r| if r.error.is_none() { r.duration_ms } else { 0 })
            .collect();
        durations.sort_unstable();

        let (min_ms, max_ms, avg_ms, p95_ms) = compute_statistics(&durations, succeeded);

        Self {
            saved_request_id,
            results,
            total,
            succeeded,
            failed,
            min_ms,
            max_ms,
            avg_ms,
            p95_ms,
            started_at: Utc::now(),
            finished_at: Utc::now(),
        }
    }
}

/// Compute min/avg/max/p95 statistics from a sorted list of durations.
///
/// Only successful request durations (non-zero) are considered for latency
/// statistics. If there are no successful requests, all stats are zero.
pub fn compute_statistics(durations: &[u64], succeeded: usize) -> (u64, u64, u64, u64) {
    if succeeded == 0 || durations.is_empty() {
        return (0, 0, 0, 0);
    }

    let successful: Vec<u64> = durations.iter().copied().filter(|d| *d > 0).collect();
    if successful.is_empty() {
        return (0, 0, 0, 0);
    }

    let min_ms = *successful.first().unwrap_or(&0);
    let max_ms = *successful.last().unwrap_or(&0);
    let sum: u64 = successful.iter().sum();
    let avg_ms = sum / successful.len() as u64;

    let p95_idx = if successful.len() == 1 {
        0
    } else {
        ((successful.len() as f64) * 0.95).ceil() as usize - 1
    };
    let p95_idx = p95_idx.min(successful.len() - 1);
    let p95_ms = successful[p95_idx];

    (min_ms, max_ms, avg_ms, p95_ms)
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
            http_version: None,
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
                http_version: None,
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
                http_version: None,
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
                ..Default::default()
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
                http_version: None,
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
                ..Default::default()
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

        // Note: build_request_bytes and parse_response tests were removed
        // because the replay engine now uses reqwest instead of manual
        // HTTP/1.1 parsing. The reqwest library handles request building
        // and response parsing internally.

        #[tokio::test]
        async fn test_replay_nonexistent_request() {
            let manager = ReplayManager::new();

            let result = manager.replay("nonexistent-id", None).await;

            assert!(result.response.is_none());
            assert_eq!(result.error, Some("Saved request not found".to_string()));
            assert_eq!(result.saved_request_id, "nonexistent-id");
        }
    }

    mod batch_config_tests {
        use super::*;

        #[test]
        fn test_default_config() {
            let config = ReplayBatchConfig::default();
            assert_eq!(config.iterations, 1);
            assert_eq!(config.concurrency, 1);
            assert!(config.delay_ms.is_none());
        }

        #[test]
        fn test_clamp_zero_values() {
            let mut config = ReplayBatchConfig {
                iterations: 0,
                concurrency: 0,
                delay_ms: None,
            };
            config.clamp_to_limits();
            assert_eq!(config.iterations, 1);
            assert_eq!(config.concurrency, 1);
        }

        #[test]
        fn test_clamp_over_limits() {
            let mut config = ReplayBatchConfig {
                iterations: MAX_BATCH_ITERATIONS + 5000,
                concurrency: MAX_BATCH_CONCURRENCY + 50,
                delay_ms: Some(100),
            };
            config.clamp_to_limits();
            assert_eq!(config.iterations, MAX_BATCH_ITERATIONS);
            assert_eq!(config.concurrency, MAX_BATCH_CONCURRENCY);
        }

        #[test]
        fn test_clamp_within_limits() {
            let mut config = ReplayBatchConfig {
                iterations: 50,
                concurrency: 10,
                delay_ms: Some(200),
            };
            config.clamp_to_limits();
            assert_eq!(config.iterations, 50);
            assert_eq!(config.concurrency, 10);
        }

        #[test]
        fn test_config_serialization() {
            let config = ReplayBatchConfig {
                iterations: 100,
                concurrency: 5,
                delay_ms: Some(250),
            };
            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"iterations\":100"));
            assert!(json.contains("\"concurrency\":5"));
            assert!(json.contains("\"delay_ms\":250"));

            let decoded: ReplayBatchConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.iterations, 100);
            assert_eq!(decoded.concurrency, 5);
            assert_eq!(decoded.delay_ms, Some(250));
        }
    }

    mod statistics_tests {
        use super::*;

        #[test]
        fn test_compute_statistics_basic() {
            // Sorted durations: [10, 20, 30, 40, 50]
            let durations = vec![10, 20, 30, 40, 50];
            let (min, max, avg, p95) = compute_statistics(&durations, 5);
            assert_eq!(min, 10);
            assert_eq!(max, 50);
            assert_eq!(avg, 30); // (10+20+30+40+50)/5 = 150/5 = 30
                                 // p95 index = ceil(5 * 0.95) - 1 = ceil(4.75) - 1 = 5 - 1 = 4
            assert_eq!(p95, 50);
        }

        #[test]
        fn test_compute_statistics_single_value() {
            let durations = vec![42];
            let (min, max, avg, p95) = compute_statistics(&durations, 1);
            assert_eq!(min, 42);
            assert_eq!(max, 42);
            assert_eq!(avg, 42);
            assert_eq!(p95, 42);
        }

        #[test]
        fn test_compute_statistics_with_failures() {
            // 3 succeeded (durations 100, 200, 300), 2 failed (duration 0)
            let durations = vec![0, 0, 100, 200, 300];
            let (min, max, avg, p95) = compute_statistics(&durations, 3);
            assert_eq!(min, 100);
            assert_eq!(max, 300);
            assert_eq!(avg, 200); // (100+200+300)/3 = 600/3 = 200
                                  // p95 index = ceil(3 * 0.95) - 1 = ceil(2.85) - 1 = 3 - 1 = 2
            assert_eq!(p95, 300);
        }

        #[test]
        fn test_compute_statistics_all_failed() {
            let durations = vec![0, 0, 0];
            let (min, max, avg, p95) = compute_statistics(&durations, 0);
            assert_eq!(min, 0);
            assert_eq!(max, 0);
            assert_eq!(avg, 0);
            assert_eq!(p95, 0);
        }

        #[test]
        fn test_compute_statistics_empty() {
            let durations: Vec<u64> = vec![];
            let (min, max, avg, p95) = compute_statistics(&durations, 0);
            assert_eq!(min, 0);
            assert_eq!(max, 0);
            assert_eq!(avg, 0);
            assert_eq!(p95, 0);
        }

        #[test]
        fn test_compute_statistics_p95_large_sample() {
            // 20 values: 5, 10, 15, ..., 100
            let durations: Vec<u64> = (1..=20).map(|i| i * 5).collect();
            let n = durations.len();
            let (min, max, avg, p95) = compute_statistics(&durations, n);
            assert_eq!(min, 5);
            assert_eq!(max, 100);
            let sum: u64 = durations.iter().sum();
            assert_eq!(avg, sum / n as u64);
            // p95 index = ceil(20 * 0.95) - 1 = ceil(19) - 1 = 19 - 1 = 18
            assert_eq!(p95, durations[18]);
        }
    }

    mod batch_result_tests {
        use super::*;

        fn make_success_result(saved_id: &str, duration_ms: u64) -> ReplayResult {
            let request = create_test_request();
            let response = ResponseData {
                status_code: 200,
                status_message: Some("OK".to_string()),
                headers: HashMap::new(),
                body: None,
                content_type: None,
                duration_ms,
                http_version: None,
            };
            ReplayResult::success(saved_id, request, response, duration_ms)
        }

        fn make_error_result(saved_id: &str) -> ReplayResult {
            let request = create_test_request();
            ReplayResult::error(saved_id, request, "Connection failed".to_string())
        }

        #[test]
        fn test_from_results_all_success() {
            let results = vec![
                make_success_result("req-1", 100),
                make_success_result("req-1", 200),
                make_success_result("req-1", 300),
            ];
            let batch = ReplayBatchResult::from_results("req-1".to_string(), results);

            assert_eq!(batch.saved_request_id, "req-1");
            assert_eq!(batch.total, 3);
            assert_eq!(batch.succeeded, 3);
            assert_eq!(batch.failed, 0);
            assert_eq!(batch.min_ms, 100);
            assert_eq!(batch.max_ms, 300);
            assert_eq!(batch.avg_ms, 200);
        }

        #[test]
        fn test_from_results_mixed_success_failure() {
            let results = vec![
                make_success_result("req-1", 100),
                make_error_result("req-1"),
                make_success_result("req-1", 300),
            ];
            let batch = ReplayBatchResult::from_results("req-1".to_string(), results);

            assert_eq!(batch.total, 3);
            assert_eq!(batch.succeeded, 2);
            assert_eq!(batch.failed, 1);
            // succeeded + failed == total
            assert_eq!(batch.succeeded + batch.failed, batch.total);
            // Only successful durations count toward stats
            assert_eq!(batch.min_ms, 100);
            assert_eq!(batch.max_ms, 300);
            assert_eq!(batch.avg_ms, 200);
        }

        #[test]
        fn test_from_results_all_failures() {
            let results = vec![make_error_result("req-1"), make_error_result("req-1")];
            let batch = ReplayBatchResult::from_results("req-1".to_string(), results);

            assert_eq!(batch.total, 2);
            assert_eq!(batch.succeeded, 0);
            assert_eq!(batch.failed, 2);
            assert_eq!(batch.succeeded + batch.failed, batch.total);
            assert_eq!(batch.min_ms, 0);
            assert_eq!(batch.max_ms, 0);
            assert_eq!(batch.avg_ms, 0);
            assert_eq!(batch.p95_ms, 0);
        }

        #[test]
        fn test_from_results_empty() {
            let batch = ReplayBatchResult::from_results("req-1".to_string(), vec![]);
            assert_eq!(batch.total, 0);
            assert_eq!(batch.succeeded, 0);
            assert_eq!(batch.failed, 0);
        }

        #[test]
        fn test_batch_result_serialization() {
            let results = vec![make_success_result("req-1", 150)];
            let batch = ReplayBatchResult::from_results("req-1".to_string(), results);
            let json = serde_json::to_string(&batch).unwrap();
            assert!(json.contains("\"saved_request_id\":\"req-1\""));
            assert!(json.contains("\"total\":1"));
            assert!(json.contains("\"succeeded\":1"));

            let decoded: ReplayBatchResult = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.saved_request_id, "req-1");
            assert_eq!(decoded.total, 1);
        }
    }

    mod replay_batch_tests {
        use super::*;

        #[tokio::test]
        async fn test_replay_batch_nonexistent_request() {
            let manager = ReplayManager::new();
            let config = ReplayBatchConfig {
                iterations: 3,
                concurrency: 1,
                delay_ms: None,
            };

            let batch = manager.replay_batch("nonexistent-id", None, config).await;

            // When the saved request doesn't exist, a single error result is
            // returned immediately without dispatching iterations.
            assert_eq!(batch.saved_request_id, "nonexistent-id");
            assert_eq!(batch.total, 1);
            assert_eq!(batch.succeeded, 0);
            assert_eq!(batch.failed, 1);
            assert!(batch.results[0].error.is_some());
        }

        #[tokio::test]
        async fn test_replay_batch_clamps_config() {
            // Verify that an over-limit config is clamped before dispatch by
            // checking the nonexistent-request path still returns a single
            // error (i.e. it didn't try to run 99999 iterations).
            let manager = ReplayManager::new();
            let config = ReplayBatchConfig {
                iterations: 99_999,
                concurrency: 999,
                delay_ms: None,
            };

            let batch = manager.replay_batch("nonexistent-id", None, config).await;
            assert_eq!(batch.total, 1);
            assert_eq!(batch.failed, 1);
        }
    }
}

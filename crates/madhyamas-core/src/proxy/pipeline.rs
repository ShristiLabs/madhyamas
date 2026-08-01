//! Shared HTTP request/response processing pipeline.
//!
//! This module contains the common logic used by both the plain HTTP and
//! HTTPS (TLS) proxy paths. The [`Pipeline`] struct borrows the various
//! managers and stores from [`crate::proxy::ProxyEngine`] and provides a
//! single [`Pipeline::process_request`] entry point that handles:
//!
//! - Rewrite rules
//! - Script and plugin hooks
//! - gRPC traffic detection and recording
//! - Mock responses
//! - Request/response breakpoints
//! - Traffic recording and broadcasting
//! - Upstream forwarding via `reqwest`
//!
//! The engine module remains responsible for connection management (TCP
//! accept, TLS handshake, WebSocket upgrade detection) and delegates the
//! per-request processing to this pipeline.

use crate::config::ProxyConfig;
use crate::extension::{ExtensionContext, ExtensionManager, ExtensionRequest, ExtensionResponse};
#[cfg(feature = "grpc")]
use crate::grpc::{is_grpc_content_type, is_grpc_path, parse_frame, GrpcDirection, GrpcManager};
use crate::intercept::{
    BlockListManager, BreakpointDecision, BreakpointManager, InterceptAction, InterceptHandler,
    MockManager, RewriteManager, ThrottleManager,
};
use crate::performance::{MemoryManager, MemoryPressure, MetricsCollector};
#[cfg(feature = "plugins")]
use crate::plugin::{PluginContext, PluginHook, PluginManager};
#[cfg(feature = "scripting")]
use crate::scripting::{ScriptContext, ScriptHook, ScriptRuntime};
use crate::traffic::{RequestData, ResponseData, TrafficEntry, TrafficStore};
use crate::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Outcome of processing a single request through the pipeline.
///
/// The caller (engine) uses this to decide whether to continue a keep-alive
/// loop or return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// The request was forwarded to the upstream server and the response was
    /// fully processed (stored, hooks run, etc.).
    Forwarded,
    /// A response was sent directly to the client without contacting the
    /// upstream server (e.g. a mock or breakpoint "respond" decision).
    Responded,
    /// The request was aborted by a breakpoint decision.
    Aborted,
}

/// Shared request/response processing pipeline.
///
/// Borrows the relevant managers and stores from the proxy engine for the
/// duration of processing one or more requests on a connection.
pub struct Pipeline<'a> {
    config: ProxyConfig,
    /// Shared, pooled HTTP client for upstream forwarding.
    http_client: reqwest::Client,
    traffic_store: &'a TrafficStore,
    traffic_tx: &'a broadcast::Sender<TrafficEntry>,
    mock_manager: Option<&'a Arc<MockManager>>,
    rewrite_manager: Option<&'a Arc<RewriteManager>>,
    breakpoint_manager: Option<&'a Arc<BreakpointManager>>,
    throttle_manager: Option<&'a Arc<ThrottleManager>>,
    block_list_manager: Option<&'a Arc<BlockListManager>>,
    #[cfg(feature = "grpc")]
    grpc_manager: Option<&'a Arc<GrpcManager>>,
    #[cfg(feature = "scripting")]
    script_runtime: Option<&'a Arc<ScriptRuntime>>,
    #[cfg(feature = "plugins")]
    plugin_manager: Option<&'a Arc<PluginManager>>,
    /// Unified extension manager (wraps scripting + plugins)
    extension_manager: Option<&'a Arc<ExtensionManager>>,
    /// Optional metrics collector for request/response counters and latency.
    metrics_collector: Option<&'a Arc<MetricsCollector>>,
    /// Optional memory manager for tracking traffic memory pressure.
    memory_manager: Option<&'a Arc<MemoryManager>>,
}

impl<'a> Pipeline<'a> {
    /// Create a new pipeline from the shared engine state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: ProxyConfig,
        http_client: reqwest::Client,
        traffic_store: &'a TrafficStore,
        traffic_tx: &'a broadcast::Sender<TrafficEntry>,
        mock_manager: Option<&'a Arc<MockManager>>,
        rewrite_manager: Option<&'a Arc<RewriteManager>>,
        breakpoint_manager: Option<&'a Arc<BreakpointManager>>,
        throttle_manager: Option<&'a Arc<ThrottleManager>>,
        block_list_manager: Option<&'a Arc<BlockListManager>>,
        #[cfg(feature = "grpc")] grpc_manager: Option<&'a Arc<GrpcManager>>,
        #[cfg(feature = "scripting")] script_runtime: Option<&'a Arc<ScriptRuntime>>,
        #[cfg(feature = "plugins")] plugin_manager: Option<&'a Arc<PluginManager>>,
        extension_manager: Option<&'a Arc<ExtensionManager>>,
        metrics_collector: Option<&'a Arc<MetricsCollector>>,
        memory_manager: Option<&'a Arc<MemoryManager>>,
    ) -> Self {
        Self {
            config,
            http_client,
            traffic_store,
            traffic_tx,
            mock_manager,
            rewrite_manager,
            breakpoint_manager,
            throttle_manager,
            block_list_manager,
            #[cfg(feature = "grpc")]
            grpc_manager,
            #[cfg(feature = "scripting")]
            script_runtime,
            #[cfg(feature = "plugins")]
            plugin_manager,
            extension_manager,
            metrics_collector,
            memory_manager,
        }
    }

    /// Record a response in the metrics collector (if attached). Accounts for
    /// both header and body bytes plus the request/response latency.
    fn record_response_metrics(&self, response: &ResponseData, duration: std::time::Duration) {
        if let Some(metrics) = self.metrics_collector {
            let body_bytes = response.body.as_ref().map(|b| b.len() as u64).unwrap_or(0);
            let header_bytes: u64 = response
                .headers
                .iter()
                .map(|(k, v)| (k.len() + v.len() + 4) as u64)
                .sum();
            metrics.record_response(body_bytes + header_bytes, duration);
        }
    }

    /// Check if a request should be excluded from traffic capture.
    /// Excludes Madhyamas's own API requests to prevent feedback loops.
    pub fn should_exclude_from_capture(&self, request: &RequestData) -> bool {
        let api_port = self.config.api_port;

        // Check if request is to Madhyamas's own API
        if let Some(host) = request.headers.get("Host").or(request.headers.get("host")) {
            // Match patterns like "localhost:3001", "127.0.0.1:3001", "10.0.0.37:3001"
            if host.ends_with(&format!(":{}", api_port)) {
                return true;
            }
        }

        // Also check the URL directly
        if request.url.contains(&format!(":{}/api/", api_port)) {
            return true;
        }

        false
    }

    /// Return all attached intercept handlers sorted by priority (ascending).
    ///
    /// This gives a uniform `Vec<Arc<dyn InterceptHandler>>` view of the
    /// mock, breakpoint, rewrite, and throttle managers. New interceptors
    /// that implement [`InterceptHandler`] can be added here without
    /// touching the pipeline's request/response loops.
    pub fn handlers(&self) -> Vec<Arc<dyn InterceptHandler>> {
        let mut handlers: Vec<Arc<dyn InterceptHandler>> = Vec::new();
        if let Some(m) = self.block_list_manager {
            handlers.push(m.clone() as Arc<dyn InterceptHandler>);
        }
        if let Some(m) = self.rewrite_manager {
            handlers.push(m.clone() as Arc<dyn InterceptHandler>);
        }
        if let Some(m) = self.mock_manager {
            handlers.push(m.clone() as Arc<dyn InterceptHandler>);
        }
        if let Some(m) = self.breakpoint_manager {
            handlers.push(m.clone() as Arc<dyn InterceptHandler>);
        }
        if let Some(m) = self.throttle_manager {
            handlers.push(m.clone() as Arc<dyn InterceptHandler>);
        }
        handlers.sort_by_key(|h| h.priority());
        handlers
    }

    /// Ship a short-circuit response to the client.
    ///
    /// This centralizes the "store request → store response → broadcast →
    /// write to client → record metrics" sequence that was previously
    /// duplicated in the mock-match and breakpoint-respond branches of
    /// [`process_request`](Self::process_request).
    ///
    /// Returns `Ok(RequestOutcome::Responded)` on success.
    async fn short_circuit_response<W>(
        &self,
        request_data: &RequestData,
        response: &ResponseData,
        client_stream: &mut W,
        log_tag: &str,
    ) -> crate::Result<RequestOutcome>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        let session_id = self.traffic_store.current_session_id();
        let entry = TrafficEntry::new(&session_id, request_data.clone());
        self.traffic_store.store_request(&entry)?;
        self.traffic_store.store_response(&entry.id, response)?;
        let _ = self.traffic_tx.send(entry);

        let response_bytes = self.build_response_bytes(response);
        client_stream.write_all(&response_bytes).await?;

        self.record_response_metrics(
            response,
            std::time::Duration::from_millis(response.duration_ms),
        );

        info!(
            "{} {} -> {} ({})",
            request_data.method, request_data.url, response.status_code, log_tag
        );
        Ok(RequestOutcome::Responded)
    }

    /// Process a single request through the full pipeline.
    ///
    /// This applies rewrite rules, runs hooks, checks for mocks and
    /// breakpoints, forwards to the upstream server (if not short-circuited),
    /// records traffic, and writes the response to `client_stream`.
    ///
    /// The caller is responsible for parsing the request and reading the full
    /// request body before calling this method.
    pub async fn process_request<W>(
        &self,
        request_data: &mut RequestData,
        client_stream: &mut W,
    ) -> crate::Result<RequestOutcome>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        // Record the incoming request in metrics (bytes = headers + body).
        if let Some(metrics) = self.metrics_collector {
            let body_bytes = request_data
                .body
                .as_ref()
                .map(|b| b.len() as u64)
                .unwrap_or(0);
            // Approximate header size for accounting.
            let header_bytes: u64 = request_data
                .headers
                .iter()
                .map(|(k, v)| (k.len() + v.len() + 4) as u64)
                .sum();
            metrics.record_request(body_bytes + header_bytes);
        }

        // Enforce memory limits: if the memory manager reports pressure,
        // log a warning so operators know traffic retention is constrained.
        // (Actual pruning of stored traffic is delegated to the store's own
        // max_requests limit; the memory manager tracks aggregate pressure.)
        if let Some(memory) = self.memory_manager {
            match memory.check_memory() {
                MemoryPressure::Cleanup { target_bytes } => {
                    warn!(
                        "Memory pressure: cleanup recommended (target {} bytes) for {}",
                        target_bytes, request_data.url
                    );
                }
                MemoryPressure::Pressure => {
                    debug!("Memory pressure elevated for {}", request_data.url);
                }
                MemoryPressure::Normal => {}
            }
        }

        // Check block list (priority 5 — before rewrites, mocks, breakpoints).
        // A blocked request is short-circuited immediately without forwarding
        // upstream or running any other intercept handlers.
        if let Some(block_list_manager) = self.block_list_manager {
            let action = block_list_manager.on_request(request_data).await;
            if let InterceptAction::Respond(response) = action {
                debug!(
                    "Request blocked by block list: {} for {}",
                    request_data.host, request_data.url
                );
                if let Some(metrics) = self.metrics_collector {
                    metrics.record_mock_hit();
                }
                return self
                    .short_circuit_response(request_data, &response, client_stream, "blocked")
                    .await;
            }
        }

        // Apply rewrite rules to request
        if let Some(rewrite_manager) = self.rewrite_manager {
            rewrite_manager.rewrite_request(request_data);
        }

        // Run script and plugin request hooks
        #[cfg(any(feature = "scripting", feature = "plugins"))]
        self.run_request_hooks(request_data);

        // Detect and record gRPC traffic
        #[cfg(feature = "grpc")]
        let grpc_stream = self.detect_and_record_grpc_request(request_data);

        // Check for mock response
        if let Some(mock_manager) = self.mock_manager {
            if let Some(mock) = mock_manager.find_matching_mock(request_data) {
                debug!("Mock matched: {} for {}", mock.name, request_data.url);

                if let Some(metrics) = self.metrics_collector {
                    metrics.record_mock_hit();
                }

                if let Some(throttle_manager) = self.throttle_manager {
                    throttle_manager.apply_latency().await;
                }

                let mut response = self.build_mock_response(&mock.response()).await;
                // Mocks are protocol-agnostic; inherit the downstream HTTP
                // version from the request so the traffic list shows the
                // correct protocol label.
                response.http_version = request_data.http_version.clone();

                return self
                    .short_circuit_response(request_data, &response, client_stream, "mocked")
                    .await;
            }
        }

        // Check for breakpoint on request
        if let Some(breakpoint_manager) = self.breakpoint_manager {
            if let Some(rule) = breakpoint_manager.check_request(request_data) {
                debug!("Breakpoint hit: {} for {}", rule.name, request_data.url);

                if let Some(metrics) = self.metrics_collector {
                    metrics.record_breakpoint_hit();
                }

                let session_id = self.traffic_store.current_session_id();
                let entry = TrafficEntry::new(&session_id, request_data.clone());
                let entry_id = entry.id.clone();

                let decision = breakpoint_manager
                    .pause_and_wait(
                        entry_id,
                        crate::intercept::InterceptDirection::Request,
                        request_data.clone(),
                        None,
                        rule.id.clone(),
                    )
                    .await;

                match decision {
                    BreakpointDecision::Abort => {
                        warn!("Request aborted by breakpoint: {}", request_data.url);
                        return Ok(RequestOutcome::Aborted);
                    }
                    BreakpointDecision::Continue => {}
                    BreakpointDecision::Modify { modifications } => {
                        BreakpointManager::apply_request_modifications(
                            request_data,
                            &modifications,
                        );
                    }
                    BreakpointDecision::Respond {
                        status_code,
                        headers,
                        body,
                    } => {
                        let response = ResponseData {
                            status_code,
                            status_message: None,
                            headers,
                            body: body.map(|b| b.into_bytes()),
                            content_type: Some("application/json".to_string()),
                            duration_ms: 0,
                            http_version: request_data.http_version.clone(),
                        };

                        return self
                            .short_circuit_response(
                                request_data,
                                &response,
                                client_stream,
                                "breakpoint response",
                            )
                            .await;
                    }
                }
            }
        }

        // Skip storing Madhyamas's own API requests to prevent feedback loops
        let should_capture = !self.should_exclude_from_capture(request_data);

        // Store the request (if not excluded)
        let session_id = self.traffic_store.current_session_id();
        let entry = TrafficEntry::new(&session_id, request_data.clone());
        if should_capture {
            self.traffic_store.store_request(&entry)?;
            // Broadcast to WebSocket clients
            let _ = self.traffic_tx.send(entry.clone());
        }

        // Apply throttle latency if enabled
        if let Some(throttle_manager) = self.throttle_manager {
            throttle_manager.apply_latency().await;
        }

        // Forward to upstream server
        let start = std::time::Instant::now();

        match self.forward_via_reqwest(request_data, client_stream).await {
            Ok(mut response) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                response.duration_ms = duration_ms;

                // Apply rewrite rules to response
                if let Some(rewrite_manager) = self.rewrite_manager {
                    rewrite_manager.rewrite_response(request_data, &mut response);
                }

                // Run script and plugin response hooks
                #[cfg(any(feature = "scripting", feature = "plugins"))]
                self.run_response_hooks(request_data, &response);

                // Record gRPC response frames
                #[cfg(feature = "grpc")]
                self.record_grpc_response(request_data, &response, grpc_stream.as_ref());

                // Check for breakpoint on response
                if let Some(breakpoint_manager) = self.breakpoint_manager {
                    if let Some(rule) = breakpoint_manager.check_response(request_data, &response) {
                        debug!(
                            "Breakpoint hit on response: {} for {}",
                            rule.name, request_data.url
                        );

                        if let Some(metrics) = self.metrics_collector {
                            metrics.record_breakpoint_hit();
                        }

                        let decision = breakpoint_manager
                            .pause_and_wait(
                                entry.id.clone(),
                                crate::intercept::InterceptDirection::Response,
                                request_data.clone(),
                                Some(response.clone()),
                                rule.id.clone(),
                            )
                            .await;

                        match decision {
                            BreakpointDecision::Abort => {
                                warn!("Response aborted by breakpoint: {}", request_data.url);
                                return Ok(RequestOutcome::Aborted);
                            }
                            BreakpointDecision::Continue => {}
                            BreakpointDecision::Modify { modifications } => {
                                BreakpointManager::apply_response_modifications(
                                    &mut response,
                                    &modifications,
                                );
                            }
                            BreakpointDecision::Respond { .. } => {
                                // Already have the response, just continue
                            }
                        }
                    }
                }

                // Store the response (if not excluded)
                if should_capture {
                    self.traffic_store.store_response(&entry.id, &response)?;
                }

                // Record as mock if recording is enabled
                if let Some(mock_manager) = self.mock_manager {
                    if mock_manager.is_recording() {
                        mock_manager.record_from_traffic(
                            request_data,
                            response.status_code,
                            response.headers.clone(),
                            response.body.clone(),
                        );
                        debug!("Recorded mock for: {}", request_data.url);
                    }
                }

                info!(
                    "{} {} -> {} ({}ms)",
                    request_data.method, request_data.url, response.status_code, duration_ms
                );

                // Record the response in metrics (bytes + latency).
                self.record_response_metrics(
                    &response,
                    std::time::Duration::from_millis(duration_ms),
                );
            }
            Err(e) => {
                warn!("Failed to forward request to {}: {}", request_data.url, e);
                // Record the error in metrics.
                if let Some(metrics) = self.metrics_collector {
                    metrics.record_error();
                }
                // Store error response so request doesn't remain in "pending" state.
                // Include the full request details (method, URL, headers) in the
                // error body so the user can diagnose the failure from the UI.
                if should_capture {
                    let error_detail = format!(
                        "Proxy error forwarding request to upstream server.\n\n\
                         Request: {} {}\n\
                         Error: {}\n\n\
                         Request headers:\n{}",
                        request_data.method,
                        request_data.url,
                        e,
                        request_data
                            .headers
                            .iter()
                            .map(|(k, v)| format!("  {}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    let error_response = ResponseData {
                        status_code: 502,
                        status_message: Some("Bad Gateway".to_string()),
                        headers: std::collections::HashMap::new(),
                        body: Some(error_detail.into_bytes()),
                        content_type: Some("text/plain".to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                        http_version: request_data.http_version.clone(),
                    };
                    self.traffic_store
                        .store_response(&entry.id, &error_response)?;
                }
            }
        }

        Ok(RequestOutcome::Forwarded)
    }

    /// Parse HTTP request from bytes
    pub fn parse_http_request(
        &self,
        data: &[u8],
        host: &str,
        port: u16,
    ) -> crate::Result<RequestData> {
        let request_str = String::from_utf8_lossy(data);
        let mut lines = request_str.lines();

        // Parse request line
        let request_line = lines.next().unwrap_or("");
        let parts: Vec<&str> = request_line.split_whitespace().collect();

        if parts.len() < 3 {
            return Err(Error::Proxy("Invalid request line".into()));
        }

        let method = parts[0];
        let path = parts[1];

        // Parse headers
        let mut headers = std::collections::HashMap::new();
        let mut content_length = 0;
        let mut content_type = None;

        for line in lines {
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();

                if key.eq_ignore_ascii_case("content-length") {
                    content_length = value.parse().unwrap_or(0);
                } else if key.eq_ignore_ascii_case("content-type") {
                    content_type = Some(value.clone());
                }

                headers.insert(key, value);
            }
        }

        // Detect chunked transfer encoding
        let is_chunked = headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("transfer-encoding") && v.eq_ignore_ascii_case("chunked")
        });

        // Find where headers end and body begins
        let header_end = data
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| p + 4);

        // Extract body if present
        let body = if content_length > 0 {
            // Content-Length based body: only take up to content_length bytes
            // (the initial read may contain less than the full body, or may
            // contain pipelined request data beyond the body)
            if let Some(start) = header_end {
                let available = data.len().saturating_sub(start);
                let take = available.min(content_length);
                Some(data[start..start + take].to_vec())
            } else {
                None
            }
        } else if is_chunked {
            // Chunked transfer encoding: extract raw chunked data after headers.
            // The body may be incomplete in the initial read; the caller will
            // read more data and decode the chunks.
            header_end.map(|start| data[start..].to_vec())
        } else {
            None
        };

        // In HTTP proxy mode, path contains full URL - use it directly
        let (url, actual_path) = if path.starts_with("http://") || path.starts_with("https://") {
            // path is already a full URL, extract just the path component
            if let Ok(uri) = path.parse::<hyper::Uri>() {
                let uri_path = uri
                    .path_and_query()
                    .map(|p| p.to_string())
                    .unwrap_or("/".to_string());
                (path.to_string(), uri_path)
            } else {
                (path.to_string(), path.to_string())
            }
        } else {
            // path is just a path component
            let scheme = if port == 443 { "https" } else { "http" };
            (format!("{}://{}{}", scheme, host, path), path.to_string())
        };

        Ok(RequestData {
            method: method.into(),
            url,
            host: host.to_string(),
            path: actual_path,
            headers,
            body,
            content_type,
            http_version: Some("HTTP/1.1".to_string()),
        })
    }

    /// Read the full request body from the client, handling both
    /// Content-Length and chunked transfer encoding. The initial body
    /// bytes (from the first read) are supplemented with additional
    /// reads as needed.
    pub async fn read_full_request_body<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut R,
        body: Option<Vec<u8>>,
        headers: &std::collections::HashMap<String, String>,
    ) -> crate::Result<Option<Vec<u8>>> {
        // Check for chunked transfer encoding
        let is_chunked = headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("transfer-encoding") && v.eq_ignore_ascii_case("chunked")
        });

        if is_chunked {
            // For chunked: read raw data until we see the 0\r\n\r\n terminator
            let mut raw = body.unwrap_or_default();

            // Check if we already have the terminator
            let has_terminator = |buf: &[u8]| {
                // Look for "0\r\n\r\n" which marks the end of chunked data
                buf.windows(5).any(|w| w == b"0\r\n\r\n")
            };

            while !has_terminator(&raw) {
                let mut chunk = vec![0u8; 65536];
                match reader.read(&mut chunk).await {
                    Ok(0) => break, // client closed
                    Ok(n) => raw.extend_from_slice(&chunk[..n]),
                    Err(e) => {
                        return Err(Error::Proxy(format!(
                            "Failed to read chunked request body: {}",
                            e
                        )))
                    }
                }
            }

            // Decode chunked encoding into raw body bytes
            let decoded = Self::decode_chunked(&raw);
            Ok(Some(decoded))
        } else {
            // Content-Length based: read until we have the full body
            let content_length: usize = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, v)| v.parse().ok())
                .unwrap_or(0);

            if content_length == 0 {
                return Ok(body);
            }

            let mut buf = body.unwrap_or_default();

            // Read more if we don't have the full body yet
            while buf.len() < content_length {
                let mut chunk = vec![0u8; content_length - buf.len()];
                match reader.read(&mut chunk).await {
                    Ok(0) => break, // client closed
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    Err(e) => {
                        return Err(Error::Proxy(format!("Failed to read request body: {}", e)))
                    }
                }
            }

            // Truncate to content_length in case we over-read (pipelined requests)
            buf.truncate(content_length);
            Ok(Some(buf))
        }
    }

    /// Forward a request to the upstream server using `reqwest`.
    ///
    /// This replaces the previous manual TCP+TLS+HTTP/1.1 parsing and provides:
    /// - HTTP/2 support (via ALPN negotiation)
    /// - Chunked transfer encoding
    /// - gzip/deflate/brotli decompression (handled by reqwest)
    /// - Connection pooling / keep-alive
    /// - Proper header handling (case-insensitive, duplicate headers)
    ///
    /// The response is read in full, stored as `ResponseData`, then
    /// re-serialized as HTTP/1.1 and written to the client stream.
    pub async fn forward_via_reqwest<W>(
        &self,
        request_data: &RequestData,
        client_stream: &mut W,
    ) -> crate::Result<ResponseData>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        // Use the shared, pooled HTTP client from the engine. This reuses
        // TCP/TLS connections across requests (connection pooling), enables
        // HTTP/2 multiplexing, and TLS session resumption — all critical for
        // compatibility with servers that rate-limit or reject clients that
        // open a new connection for every request.
        //
        // The client is configured with:
        // - No redirects (proxy returns 3xx to client)
        // - No system proxy (avoid feedback loop)
        // - No auto-decompression (we store raw compressed body + Content-Encoding)
        // - 120s timeout, 90s idle pool timeout, 20 idle conns per host
        let client = &self.http_client;

        // Build the reqwest request from RequestData
        let method = reqwest::Method::from_bytes(request_data.method.to_string().as_bytes())
            .map_err(|e| Error::Proxy(format!("Invalid HTTP method: {}", e)))?;

        let mut req_builder = client.request(method, &request_data.url);

        // Copy headers, skipping hop-by-hop headers that are forbidden in
        // both HTTP/1.1 and HTTP/2.  HTTP/2 (RFC 7540 §8.1.2.2) forbids
        // connection-specific headers: Connection, Keep-Alive, Proxy-Connection,
        // Transfer-Encoding, Upgrade, TE (except "trailers"), and any header
        // named in the Connection header's value.  We also strip:
        // - `Host` — reqwest sets `:authority` from the URL; sending both
        //   can cause HTTP/2 PROTOCOL_ERROR resets.
        // We preserve `Accept-Encoding` so the upstream server decides
        // whether to compress, and we preserve `Content-Encoding` in the
        // response so the frontend can toggle decompression.
        for (key, value) in &request_data.headers {
            let key_lower = key.to_lowercase();
            if !matches!(
                key_lower.as_str(),
                "connection"
                    | "keep-alive"
                    | "transfer-encoding"
                    | "upgrade"
                    | "content-length"
                    | "proxy-connection"
                    | "proxy-authenticate"
                    | "proxy-authorization"
                    | "te"
                    | "trailers"
                    | "host" // reqwest sets :authority from URL
            ) {
                if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(val) = reqwest::header::HeaderValue::from_str(value) {
                        req_builder = req_builder.header(name, val);
                    }
                }
            }
        }

        // Add body if present
        if let Some(body) = &request_data.body {
            req_builder = req_builder.body(body.clone());
        }

        // Send the request
        let response = match req_builder.send().await {
            Ok(r) => r,
            Err(e) => {
                // Log the full error chain for debugging upstream failures
                tracing::error!("Upstream request to {} failed: {:?}", request_data.url, e);
                return Err(Error::Proxy(format!("Upstream request failed: {}", e)));
            }
        };

        // Extract response data
        let status_code = response.status().as_u16();
        let response_headers = {
            let mut headers = std::collections::HashMap::new();
            for (name, value) in response.headers() {
                // Skip hop-by-hop headers and content-length (we recompute
                // it in build_response_bytes). We KEEP content-encoding so
                // the frontend can toggle decompression and the client
                // receives the original compressed response.
                let name_lower = name.as_str().to_lowercase();
                if !matches!(
                    name_lower.as_str(),
                    "transfer-encoding" | "content-length" | "connection" | "keep-alive"
                ) {
                    headers.insert(
                        name.as_str().to_string(),
                        value.to_str().unwrap_or("").to_string(),
                    );
                }
            }
            headers
        };

        let content_type = response_headers.get("content-type").cloned();

        // Read the raw response body (not decompressed — reqwest
        // auto-decompression is disabled).
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| Error::Proxy(format!("Failed to read response body: {}", e)))?;

        let body = if body_bytes.is_empty() {
            None
        } else {
            Some(body_bytes.to_vec())
        };

        let response_data = ResponseData {
            status_code,
            status_message: None,
            headers: response_headers,
            body: body.clone(),
            content_type,
            duration_ms: 0, // Set by caller
            // The response is delivered to the client over the same protocol
            // the request arrived on, so mirror the downstream HTTP version.
            http_version: request_data.http_version.clone(),
        };

        // Build HTTP/1.1 response bytes and write to client
        let response_bytes = self.build_response_bytes(&response_data);
        client_stream
            .write_all(&response_bytes)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to write response to client: {}", e)))?;

        Ok(response_data)
    }

    /// Build a mock response from mock configuration
    pub async fn build_mock_response(
        &self,
        mock_response: &crate::intercept::MockResponse,
    ) -> ResponseData {
        // Apply delay if specified
        if let Some(delay_ms) = mock_response.delay_ms {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }

        ResponseData {
            status_code: mock_response.status_code,
            status_message: None,
            headers: mock_response.headers.clone(),
            body: mock_response.body_bytes(),
            content_type: mock_response.headers.get("Content-Type").cloned(),
            duration_ms: mock_response.delay_ms.unwrap_or(0),
            // Inherit the downstream protocol version from the request at the
            // call site (mocks are protocol-agnostic).
            http_version: None,
        }
    }

    /// Build HTTP response bytes from ResponseData
    pub fn build_response_bytes(&self, response: &ResponseData) -> Vec<u8> {
        let mut bytes = format!(
            "HTTP/1.1 {} {}\r\n",
            response.status_code,
            response.status_message.as_deref().unwrap_or("OK")
        )
        .into_bytes();

        // Add headers
        for (key, value) in &response.headers {
            bytes.extend(format!("{}: {}\r\n", key, value).as_bytes());
        }

        // Add content-length if body exists
        if let Some(ref body) = response.body {
            bytes.extend(format!("Content-Length: {}\r\n", body.len()).as_bytes());
        }

        bytes.extend(b"\r\n");

        // Add body
        if let Some(ref body) = response.body {
            bytes.extend(body);
        }

        bytes
    }

    /// Build HTTP request bytes (used for WebSocket upstream connections)
    #[allow(dead_code)]
    pub fn build_http_request(&self, request_data: &RequestData) -> Vec<u8> {
        let mut request = format!("{} {} HTTP/1.1\r\n", request_data.method, request_data.path);

        for (key, value) in &request_data.headers {
            // Skip hop-by-hop headers and content-length (we recompute it
            // from the actual body to avoid mismatches when the body has
            // been modified by rewrites/breakpoints or decoded from chunked)
            if !matches!(
                key.to_lowercase().as_str(),
                "connection" | "keep-alive" | "transfer-encoding" | "upgrade" | "content-length"
            ) {
                request.push_str(&format!("{}: {}\r\n", key, value));
            }
        }

        // Add Content-Length based on the actual body length
        if let Some(ref body) = request_data.body {
            request.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }

        request.push_str("Connection: close\r\n");
        request.push_str("\r\n");

        let mut bytes = request.into_bytes();

        if let Some(ref body) = request_data.body {
            bytes.extend(body);
        }

        bytes
    }

    /// Parse HTTP response from raw bytes (kept for WebSocket upstream parsing)
    #[allow(dead_code)]
    pub fn parse_http_response(&self, data: &[u8]) -> crate::Result<ResponseData> {
        let response_str = String::from_utf8_lossy(data);
        let mut lines = response_str.lines();

        // Parse status line
        let status_line = lines.next().unwrap_or("");
        let parts: Vec<&str> = status_line.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(Error::Proxy("Invalid response line".into()));
        }

        let status_code: u16 = parts[1].parse().unwrap_or(0);
        let status_message = parts.get(2).map(|s| s.to_string());

        // Parse headers
        let mut headers = std::collections::HashMap::new();
        let mut content_type = None;
        let mut is_chunked = false;
        let mut content_length: Option<usize> = None;

        for line in lines {
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();

                if key.eq_ignore_ascii_case("content-type") {
                    content_type = Some(value.clone());
                } else if key.eq_ignore_ascii_case("transfer-encoding") {
                    if value.eq_ignore_ascii_case("chunked") {
                        is_chunked = true;
                    }
                } else if key.eq_ignore_ascii_case("content-length") {
                    content_length = value.parse().ok();
                }

                headers.insert(key, value);
            }
        }

        // Extract body (everything after \r\n\r\n)
        let body = data
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| &data[p + 4..]);

        // Process body based on transfer encoding
        let body = if let Some(raw_body) = body {
            if is_chunked {
                // Decode chunked transfer encoding and remove the header
                // so the stored response is consistent (decoded body + no TE)
                let decoded = Self::decode_chunked(raw_body);
                // Remove Transfer-Encoding header (case-insensitive)
                headers.retain(|k, _| !k.eq_ignore_ascii_case("transfer-encoding"));
                // Add Content-Length based on decoded body
                headers.insert("Content-Length".to_string(), decoded.len().to_string());
                Some(decoded)
            } else if let Some(cl) = content_length {
                // Truncate to Content-Length in case extra data was read
                let take = raw_body.len().min(cl);
                Some(raw_body[..take].to_vec())
            } else {
                Some(raw_body.to_vec())
            }
        } else {
            None
        };

        // Note: we do NOT decompress the body here. The raw compressed
        // body is stored with the Content-Encoding header preserved, so
        // the frontend can toggle between compressed and decompressed views.
        // (This method is currently dead code — kept for WebSocket upstream
        // parsing if needed in the future.)

        Ok(ResponseData {
            status_code,
            status_message,
            headers,
            body,
            content_type,
            duration_ms: 0, // Will be set by caller
            http_version: Some("HTTP/1.1".to_string()),
        })
    }

    /// Decompress response body based on Content-Encoding header.
    /// On success, removes the Content-Encoding header and updates
    /// Content-Length to match the decompressed body. Returns the
    /// decompressed body, or the original body if decompression fails
    /// or no encoding is present.
    #[allow(dead_code)]
    fn decompress_body(
        content_encoding: Option<&str>,
        body: Vec<u8>,
        out_headers: &mut std::collections::HashMap<String, String>,
    ) -> Option<Vec<u8>> {
        let encoding = match content_encoding {
            Some(e) => e,
            None => return Some(body), // no encoding, return as-is
        };

        let decompressed = match encoding {
            "gzip" | "x-gzip" => {
                use std::io::Read;
                let mut decoder = flate2::read::GzDecoder::new(&body[..]);
                let mut out = Vec::with_capacity(body.len() * 4);
                match decoder.read_to_end(&mut out) {
                    Ok(_) => Some(out),
                    Err(e) => {
                        debug!("Failed to decompress gzip body: {}", e);
                        None
                    }
                }
            }
            "deflate" => {
                use std::io::Read;
                // Try zlib-wrapped deflate first (most common), then raw deflate
                let mut out = Vec::with_capacity(body.len() * 4);
                let result = flate2::read::ZlibDecoder::new(&body[..])
                    .read_to_end(&mut out)
                    .or_else(|_| {
                        out.clear();
                        flate2::read::DeflateDecoder::new(&body[..]).read_to_end(&mut out)
                    });
                match result {
                    Ok(_) => Some(out),
                    Err(e) => {
                        debug!("Failed to decompress deflate body: {}", e);
                        None
                    }
                }
            }
            "br" => {
                let mut decoder = brotli::Decompressor::new(&body[..], 4096);
                let mut out = Vec::with_capacity(body.len() * 4);
                use std::io::Read;
                match decoder.read_to_end(&mut out) {
                    Ok(_) => Some(out),
                    Err(e) => {
                        debug!("Failed to decompress brotli body: {}", e);
                        None
                    }
                }
            }
            _ => {
                // Unknown encoding — leave body as-is
                return Some(body);
            }
        };

        match decompressed {
            Some(dec) => {
                // Remove Content-Encoding header and update Content-Length
                out_headers.retain(|k, _| !k.eq_ignore_ascii_case("content-encoding"));
                out_headers.insert("Content-Length".to_string(), dec.len().to_string());
                Some(dec)
            }
            None => Some(body), // decompression failed, keep original
        }
    }

    /// Decode HTTP chunked transfer encoding into the raw body bytes
    fn decode_chunked(data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            // Find the end of the chunk size line
            let line_end = match data[pos..].windows(2).position(|w| w == b"\r\n") {
                Some(p) => pos + p,
                None => break,
            };

            // Parse chunk size (hex, may have extensions after ';')
            let size_str = std::str::from_utf8(&data[pos..line_end]).unwrap_or("");
            let chunk_size =
                usize::from_str_radix(size_str.split(';').next().unwrap_or("0").trim(), 16)
                    .unwrap_or(0);

            pos = line_end + 2; // skip \r\n after size

            if chunk_size == 0 {
                break; // last chunk
            }

            // Copy chunk data (bounded by available data)
            let end = (pos + chunk_size).min(data.len());
            result.extend_from_slice(&data[pos..end]);
            pos = end;

            // Skip trailing \r\n after chunk data
            if pos + 2 <= data.len() && &data[pos..pos + 2] == b"\r\n" {
                pos += 2;
            }
        }

        result
    }

    /// Run script and plugin request hooks (on_request) before the request is
    /// forwarded to the upstream server.
    ///
    /// When an [`ExtensionManager`] is attached, hooks are dispatched
    /// through it (which internally calls the scripting runtime and plugin
    /// manager via adapter trait objects).  Otherwise the legacy direct
    /// calls to `script_runtime` and `plugin_manager` are used.
    #[cfg(any(feature = "scripting", feature = "plugins"))]
    fn run_request_hooks(&self, request_data: &RequestData) {
        // Unified extension manager path.
        if let Some(ext_mgr) = self.extension_manager {
            let mut ctx = build_extension_context(request_data, None, "on_request");
            ext_mgr.on_request(&mut ctx);
            return;
        }

        // Legacy direct-call path.
        // Script on_request hook
        if let Some(script_runtime) = self.script_runtime {
            let session_id = self.traffic_store.current_session_id();
            let request_id = uuid::Uuid::new_v4().to_string();
            let mut context = ScriptContext::new(&request_id, &session_id, ScriptHook::OnRequest)
                .with_request(request_data);
            let results = script_runtime.execute_hook(ScriptHook::OnRequest.as_str(), &mut context);
            for result in &results {
                if let Some(ref err) = result.error {
                    debug!("Script on_request error: {}", err);
                } else {
                    debug!(
                        "Script on_request executed (modified={}, continue={})",
                        result.modified, result.continue_
                    );
                }
            }
        }

        // Plugin on_request hook
        if let Some(plugin_manager) = self.plugin_manager {
            if plugin_manager.is_enabled() {
                let session_id = self.traffic_store.current_session_id();
                let request_id = uuid::Uuid::new_v4().to_string();
                let mut context =
                    PluginContext::new("", PluginHook::OnRequest).with_request(request_data);
                context.request_id = Some(request_id);
                context.session_id = Some(session_id);
                let results = plugin_manager.execute_hook(PluginHook::OnRequest, context);
                for (plugin_id, result) in &results {
                    if let Some(ref err) = result.error {
                        debug!("Plugin {} on_request error: {}", plugin_id, err);
                    } else {
                        debug!("Plugin {} on_request executed", plugin_id);
                    }
                }
            }
        }
    }

    /// Run script and plugin response hooks (on_response) after a response is
    /// received from the upstream server.
    ///
    /// When an [`ExtensionManager`] is attached, hooks are dispatched
    /// through it.  Otherwise the legacy direct calls are used.
    #[cfg(any(feature = "scripting", feature = "plugins"))]
    fn run_response_hooks(&self, request_data: &RequestData, response: &ResponseData) {
        // Unified extension manager path.
        if let Some(ext_mgr) = self.extension_manager {
            let mut ctx = build_extension_context(request_data, Some(response), "on_response");
            ext_mgr.on_response(&mut ctx);
            return;
        }

        // Legacy direct-call path.
        // Script on_response hook
        if let Some(script_runtime) = self.script_runtime {
            let session_id = self.traffic_store.current_session_id();
            let request_id = uuid::Uuid::new_v4().to_string();
            let mut context = ScriptContext::new(&request_id, &session_id, ScriptHook::OnResponse)
                .with_request(request_data)
                .with_response(response);
            let results =
                script_runtime.execute_hook(ScriptHook::OnResponse.as_str(), &mut context);
            for result in &results {
                if let Some(ref err) = result.error {
                    debug!("Script on_response error: {}", err);
                } else {
                    debug!(
                        "Script on_response executed (modified={}, continue={})",
                        result.modified, result.continue_
                    );
                }
            }
        }

        // Plugin on_response hook
        if let Some(plugin_manager) = self.plugin_manager {
            if plugin_manager.is_enabled() {
                let session_id = self.traffic_store.current_session_id();
                let request_id = uuid::Uuid::new_v4().to_string();
                let mut context = PluginContext::new("", PluginHook::OnResponse)
                    .with_request(request_data)
                    .with_response(response);
                context.request_id = Some(request_id);
                context.session_id = Some(session_id);
                let results = plugin_manager.execute_hook(PluginHook::OnResponse, context);
                for (plugin_id, result) in &results {
                    if let Some(ref err) = result.error {
                        debug!("Plugin {} on_response error: {}", plugin_id, err);
                    } else {
                        debug!("Plugin {} on_response executed", plugin_id);
                    }
                }
            }
        }
    }

    /// Detect gRPC traffic on a request and register the connection/stream and
    /// record request frames with the gRPC manager. Returns the
    /// `(connection_id, stream_id)` so the response path can reuse the same
    /// stream, or `None` if no gRPC manager is attached or the request is not
    /// gRPC.
    #[cfg(feature = "grpc")]
    fn detect_and_record_grpc_request(
        &self,
        request_data: &RequestData,
    ) -> Option<(String, String)> {
        let grpc_manager = self.grpc_manager?;

        let content_type = request_data.content_type.as_deref();
        let is_grpc = is_grpc_content_type(content_type) || is_grpc_path(&request_data.path);
        if !is_grpc {
            return None;
        }

        debug!(
            "gRPC request detected: {} {}",
            request_data.method, request_data.url
        );

        let conn_id = grpc_manager.register_connection("client", &request_data.host);
        let stream_id = grpc_manager.register_stream(&conn_id, Some(&request_data.path));

        // Record request metadata (HTTP/2 headers / pseudo-headers)
        grpc_manager.update_stream_metadata(
            &stream_id,
            GrpcDirection::Request,
            request_data.headers.clone(),
        );

        // Parse and record gRPC frames from the request body
        if let Some(ref body) = request_data.body {
            let mut offset = 0;
            while offset < body.len() {
                match parse_frame(
                    &body[offset..],
                    &stream_id,
                    &conn_id,
                    GrpcDirection::Request,
                ) {
                    Ok(Some((frame, consumed))) => {
                        grpc_manager.record_frame(frame);
                        offset += consumed;
                    }
                    _ => break,
                }
            }
        }

        Some((conn_id, stream_id))
    }

    /// Record gRPC response frames for a detected gRPC stream. If no stream
    /// info is provided (the request wasn't detected as gRPC), a new stream is
    /// registered only when the response itself looks like gRPC.
    #[cfg(feature = "grpc")]
    fn record_grpc_response(
        &self,
        request_data: &RequestData,
        response: &ResponseData,
        stream: Option<&(String, String)>,
    ) {
        let grpc_manager = match self.grpc_manager {
            Some(m) => m,
            None => return,
        };

        let (conn_id, stream_id) = match stream {
            Some((c, s)) => (c.clone(), s.clone()),
            None => {
                let content_type = response.content_type.as_deref();
                if !is_grpc_content_type(content_type) && !is_grpc_path(&request_data.path) {
                    return;
                }
                debug!(
                    "gRPC response detected (no request stream): {} {}",
                    request_data.method, request_data.url
                );
                let conn_id = grpc_manager.register_connection("client", &request_data.host);
                let stream_id = grpc_manager.register_stream(&conn_id, Some(&request_data.path));
                (conn_id, stream_id)
            }
        };

        // Record response metadata
        grpc_manager.update_stream_metadata(
            &stream_id,
            GrpcDirection::Response,
            response.headers.clone(),
        );

        // Parse and record gRPC frames from the response body
        if let Some(ref body) = response.body {
            let mut offset = 0;
            while offset < body.len() {
                match parse_frame(
                    &body[offset..],
                    &stream_id,
                    &conn_id,
                    GrpcDirection::Response,
                ) {
                    Ok(Some((frame, consumed))) => {
                        grpc_manager.record_frame(frame);
                        offset += consumed;
                    }
                    _ => break,
                }
            }
        }
    }
}

/// Build an [`ExtensionContext`] from pipeline request/response data.
#[cfg(any(feature = "scripting", feature = "plugins"))]
fn build_extension_context(
    request_data: &RequestData,
    response: Option<&ResponseData>,
    hook: &'static str,
) -> ExtensionContext {
    let request = ExtensionRequest {
        method: request_data.method.to_string(),
        url: request_data.url.clone(),
        host: request_data.host.clone(),
        path: request_data.path.clone(),
        headers: request_data.headers.clone(),
        body: request_data.body.clone(),
        content_type: request_data.content_type.clone(),
    };

    let response = response.map(|r| ExtensionResponse {
        status_code: r.status_code,
        status_message: r.status_message.clone(),
        headers: r.headers.clone(),
        body: r.body.clone(),
        content_type: r.content_type.clone(),
        duration_ms: r.duration_ms,
    });

    ExtensionContext {
        request_id: uuid::Uuid::new_v4().to_string(),
        session_id: String::new(),
        hook,
        request: Some(request),
        response,
        data: std::collections::HashMap::new(),
        timestamp: chrono::Utc::now(),
    }
}

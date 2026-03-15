//! Main proxy engine

use crate::config::ProxyConfig;
use crate::intercept::{
    BreakpointDecision, BreakpointManager, MockManager, RewriteManager, ThrottleManager,
};
use crate::tls::CertificateManager;
use crate::traffic::{RequestData, ResponseData, TrafficEntry, TrafficStore};
use crate::websocket::{is_websocket_upgrade, WsDirection, WsManager, WsMessageType, WsPayload};
use crate::Error;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Proxy engine state
pub struct ProxyEngine {
    config: ProxyConfig,
    cert_manager: Arc<CertificateManager>,
    traffic_store: Arc<TrafficStore>,
    mock_manager: Option<Arc<MockManager>>,
    rewrite_manager: Option<Arc<RewriteManager>>,
    breakpoint_manager: Option<Arc<BreakpointManager>>,
    throttle_manager: Option<Arc<ThrottleManager>>,
    /// WebSocket traffic manager
    ws_manager: Option<Arc<WsManager>>,
    /// Channel to broadcast traffic updates to WebSocket clients
    traffic_tx: broadcast::Sender<TrafficEntry>,
    /// Whether the proxy is running
    running: RwLock<bool>,
}

impl ProxyEngine {
    /// Create a new proxy engine
    pub async fn new(
        config: ProxyConfig,
        cert_manager: Arc<CertificateManager>,
        traffic_store: Arc<TrafficStore>,
    ) -> crate::Result<Arc<Self>> {
        let (traffic_tx, _) = broadcast::channel(1024);

        Ok(Arc::new(Self {
            config,
            cert_manager,
            traffic_store,
            mock_manager: None,
            rewrite_manager: None,
            breakpoint_manager: None,
            throttle_manager: None,
            ws_manager: None,
            traffic_tx,
            running: RwLock::new(false),
        }))
    }

    /// Check if a request should be excluded from traffic capture
    /// Excludes Madhyamas's own API requests to prevent feedback loops
    fn should_exclude_from_capture(&self, request: &RequestData) -> bool {
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

    /// Set the mock manager
    pub fn with_mock_manager(mut self: Arc<Self>, manager: Arc<MockManager>) -> Arc<Self> {
        Arc::get_mut(&mut self).unwrap().mock_manager = Some(manager);
        self
    }

    /// Set the rewrite manager
    pub fn with_rewrite_manager(mut self: Arc<Self>, manager: Arc<RewriteManager>) -> Arc<Self> {
        Arc::get_mut(&mut self).unwrap().rewrite_manager = Some(manager);
        self
    }

    /// Set the breakpoint manager
    pub fn with_breakpoint_manager(
        mut self: Arc<Self>,
        manager: Arc<BreakpointManager>,
    ) -> Arc<Self> {
        Arc::get_mut(&mut self).unwrap().breakpoint_manager = Some(manager);
        self
    }

    /// Set the throttle manager
    pub fn with_throttle_manager(mut self: Arc<Self>, manager: Arc<ThrottleManager>) -> Arc<Self> {
        Arc::get_mut(&mut self).unwrap().throttle_manager = Some(manager);
        self
    }

    /// Set the WebSocket manager
    pub fn with_ws_manager(mut self: Arc<Self>, manager: Arc<WsManager>) -> Arc<Self> {
        Arc::get_mut(&mut self).unwrap().ws_manager = Some(manager);
        self
    }

    /// Start the proxy server
    pub async fn start(self: Arc<Self>) -> crate::Result<()> {
        let addr: SocketAddr = self
            .config
            .proxy_addr()
            .parse()
            .map_err(|e| Error::Proxy(format!("Invalid proxy address: {}", e)))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to bind proxy port: {}", e)))?;

        *self.running.write() = true;
        info!("Proxy server listening on {}", addr);

        loop {
            let (client_socket, client_addr) = listener
                .accept()
                .await
                .map_err(|e| Error::Proxy(format!("Failed to accept connection: {}", e)))?;

            let engine = self.clone();
            tokio::spawn(async move {
                if let Err(e) = engine.handle_connection(client_socket).await {
                    debug!("Connection error from {}: {}", client_addr, e);
                }
            });
        }
    }

    /// Handle an incoming connection
    async fn handle_connection(&self, mut client_socket: TcpStream) -> crate::Result<()> {
        // Peek first to determine request type without consuming
        let mut peek_buf = [0u8; 1024];
        let n = client_socket
            .peek(&mut peek_buf)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to peek connection: {}", e)))?;

        if n == 0 {
            return Ok(());
        }

        let request_str = String::from_utf8_lossy(&peek_buf[..n]);

        if request_str.starts_with("CONNECT ") {
            // For CONNECT, we must consume the full CONNECT request from the buffer
            // before starting TLS handshake. Read until we find \r\n\r\n.
            let mut buf = vec![0u8; 8192];
            let n = client_socket
                .read(&mut buf)
                .await
                .map_err(|e| Error::Proxy(format!("Failed to read CONNECT request: {}", e)))?;

            if n == 0 {
                return Ok(());
            }

            let connect_str = String::from_utf8_lossy(&buf[..n]);
            // HTTPS tunneling
            self.handle_https_tunnel(client_socket, &connect_str).await
        } else {
            // For HTTP, read the full request data
            let mut buf = vec![0u8; 65536];
            let n = client_socket
                .read(&mut buf)
                .await
                .map_err(|e| Error::Proxy(format!("Failed to read HTTP request: {}", e)))?;

            if n == 0 {
                return Ok(());
            }

            // Regular HTTP proxy
            self.handle_http_proxy(client_socket, &buf[..n]).await
        }
    }

    /// Handle HTTPS CONNECT request
    async fn handle_https_tunnel(
        &self,
        mut client_socket: TcpStream,
        request_str: &str,
    ) -> crate::Result<()> {
        // Parse CONNECT request: "CONNECT host:port HTTP/1.1"
        let first_line = request_str.lines().next().unwrap_or("");
        let parts: Vec<&str> = first_line.split_whitespace().collect();

        if parts.len() < 2 || parts[0] != "CONNECT" {
            return Err(Error::Proxy("Invalid CONNECT request".into()));
        }

        let target = parts[1];
        let (host, port) = if target.contains(':') {
            let parts: Vec<&str> = target.split(':').collect();
            (parts[0], parts[1].parse::<u16>().unwrap_or(443))
        } else {
            (target, 443)
        };

        info!("HTTPS CONNECT: {}:{}", host, port);

        // Generate certificate for this host
        let cert = self.cert_manager.generate_cert_for_host(host)?;

        // Send 200 Connection Established
        let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
        client_socket.write_all(response.as_bytes()).await?;

        // Perform TLS handshake with client
        let tls_config = self.create_tls_server_config(&cert)?;
        let acceptor = tokio_rustls::TlsAcceptor::from(tls_config);
        let mut tls_stream = acceptor
            .accept(client_socket)
            .await
            .map_err(|e| Error::Tls(format!("TLS handshake failed: {}", e)))?;

        // Now we can intercept the actual HTTP request over TLS
        self.handle_tls_request(&mut tls_stream, host, port).await
    }

    /// Create TLS server config with the generated certificate
    fn create_tls_server_config(
        &self,
        cert: &crate::tls::GeneratedCert,
    ) -> crate::Result<Arc<rustls::ServerConfig>> {
        let cert_chain = rustls_pemfile::certs(&mut std::io::Cursor::new(&cert.certificate))
            .filter_map(|c| c.ok())
            .collect::<Vec<_>>();

        let private_key = rustls_pemfile::private_key(&mut std::io::Cursor::new(&cert.private_key))
            .map_err(|e| Error::Tls(format!("Failed to parse private key: {}", e)))?
            .ok_or_else(|| Error::Tls("No private key found".into()))?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| Error::Tls(format!("Failed to create TLS config: {}", e)))?;

        Ok(Arc::new(config))
    }

    /// Handle TLS-wrapped HTTP requests (loops for HTTP/1.1 keep-alive)
    async fn handle_tls_request(
        &self,
        tls_stream: &mut tokio_rustls::server::TlsStream<TcpStream>,
        host: &str,
        port: u16,
    ) -> crate::Result<()> {
        let mut buf = vec![0u8; 65536];

        loop {
            // Read the next HTTP request from the TLS stream
            let n = match tls_stream.read(&mut buf).await {
                Ok(0) => {
                    debug!("TLS client closed connection to {}", host);
                    return Ok(());
                }
                Ok(n) => n,
                Err(e) => {
                    debug!("TLS read finished for {}: {}", host, e);
                    return Ok(());
                }
            };

            let mut request_data = match self.parse_http_request(&buf[..n], host, port) {
                Ok(data) => data,
                Err(e) => {
                    debug!("Failed to parse request on keep-alive connection: {}", e);
                    return Ok(());
                }
            };

            // Check for WebSocket upgrade (breaks the keep-alive loop)
            if is_websocket_upgrade(&request_data.headers) {
                return self
                    .handle_websocket_upgrade_tls(tls_stream, &request_data, host, port)
                    .await;
            }

            // Determine if the client wants to close after this request
            let connection_close = request_data
                .headers
                .get("Connection")
                .or_else(|| request_data.headers.get("connection"))
                .map(|v| v.eq_ignore_ascii_case("close"))
                .unwrap_or(false);

            // Apply rewrite rules to request
            if let Some(ref rewrite_manager) = self.rewrite_manager {
                rewrite_manager.rewrite_request(&mut request_data);
            }

            // Check for mock response
            let mut handled = false;
            if let Some(ref mock_manager) = self.mock_manager {
                if let Some(mock) = mock_manager.find_matching_mock(&request_data) {
                    debug!("Mock matched: {} for {}", mock.name, request_data.url);

                    if let Some(ref throttle_manager) = self.throttle_manager {
                        throttle_manager.apply_latency().await;
                    }

                    let response = self.build_mock_response(&mock.response).await;

                    let session_id = self.traffic_store.current_session_id();
                    let entry = TrafficEntry::new(&session_id, request_data.clone());
                    self.traffic_store.store_request(&entry)?;
                    self.traffic_store.store_response(&entry.id, &response)?;
                    let _ = self.traffic_tx.send(entry);

                    let response_bytes = self.build_response_bytes(&response);
                    tls_stream.write_all(&response_bytes).await?;

                    info!(
                        "{} {} -> {} (mocked)",
                        request_data.method, request_data.url, response.status_code
                    );
                    handled = true;
                }
            }

            if !handled {
                // Check for breakpoint on request
                if let Some(ref breakpoint_manager) = self.breakpoint_manager {
                    if let Some(rule) = breakpoint_manager.check_request(&request_data) {
                        debug!("Breakpoint hit: {} for {}", rule.name, request_data.url);

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
                                return Ok(());
                            }
                            BreakpointDecision::Continue => {}
                            BreakpointDecision::Modify { modifications } => {
                                BreakpointManager::apply_request_modifications(
                                    &mut request_data,
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
                                };

                                let session_id = self.traffic_store.current_session_id();
                                let entry = TrafficEntry::new(&session_id, request_data.clone());
                                self.traffic_store.store_request(&entry)?;
                                self.traffic_store.store_response(&entry.id, &response)?;
                                let _ = self.traffic_tx.send(entry);

                                let response_bytes = self.build_response_bytes(&response);
                                tls_stream.write_all(&response_bytes).await?;

                                info!(
                                    "{} {} -> {} (breakpoint response)",
                                    request_data.method, request_data.url, status_code
                                );
                                handled = true;
                            }
                        }
                    }
                }
            }

            if !handled {
                // Skip storing Madhyamas's own API requests to prevent feedback loops
                let should_capture = !self.should_exclude_from_capture(&request_data);

                // Store the request (if not excluded)
                let session_id = self.traffic_store.current_session_id();
                let entry = TrafficEntry::new(&session_id, request_data.clone());
                if should_capture {
                    self.traffic_store.store_request(&entry)?;
                    // Broadcast to WebSocket clients
                    let _ = self.traffic_tx.send(entry.clone());
                }

                // Apply throttle latency if enabled
                if let Some(ref throttle_manager) = self.throttle_manager {
                    throttle_manager.apply_latency().await;
                }

                // Forward to upstream server
                let start = std::time::Instant::now();

                match self.forward_https_request(&request_data, tls_stream).await {
                    Ok(mut response) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        response.duration_ms = duration_ms;

                        // Apply rewrite rules to response
                        if let Some(ref rewrite_manager) = self.rewrite_manager {
                            rewrite_manager.rewrite_response(&request_data, &mut response);
                        }

                        // Check for breakpoint on response
                        if let Some(ref breakpoint_manager) = self.breakpoint_manager {
                            if let Some(rule) =
                                breakpoint_manager.check_response(&request_data, &response)
                            {
                                debug!(
                                    "Breakpoint hit on response: {} for {}",
                                    rule.name, request_data.url
                                );

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
                                        warn!(
                                            "Response aborted by breakpoint: {}",
                                            request_data.url
                                        );
                                        return Ok(());
                                    }
                                    BreakpointDecision::Continue => {}
                                    BreakpointDecision::Modify { modifications } => {
                                        BreakpointManager::apply_response_modifications(
                                            &mut response,
                                            &modifications,
                                        );
                                    }
                                    BreakpointDecision::Respond { .. } => {}
                                }
                            }
                        }

                        // Store the response (if not excluded)
                        if should_capture {
                            self.traffic_store.store_response(&entry.id, &response)?;
                        }
                        info!(
                            "{} {} -> {} ({}ms)",
                            request_data.method,
                            request_data.url,
                            response.status_code,
                            duration_ms
                        );
                    }
                    Err(e) => {
                        warn!("Failed to forward request to {}: {}", request_data.url, e);
                        return Ok(());
                    }
                }
            }

            // If the client sent Connection: close, stop the keep-alive loop
            if connection_close {
                debug!("Client requested connection close for {}", host);
                return Ok(());
            }
        }
    }

    /// Handle regular HTTP proxy request
    async fn handle_http_proxy(
        &self,
        mut client_socket: TcpStream,
        initial_data: &[u8],
    ) -> crate::Result<()> {
        let request_str = String::from_utf8_lossy(initial_data);
        let first_line = request_str.lines().next().unwrap_or("");
        let parts: Vec<&str> = first_line.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(Error::Proxy("Invalid HTTP request".into()));
        }

        let method = parts[0];
        let url = parts[1];

        // Parse URL
        let parsed_url = url
            .parse::<hyper::Uri>()
            .map_err(|e| Error::Proxy(format!("Invalid URL: {}", e)))?;

        let host = parsed_url.host().unwrap_or("");
        let port = parsed_url.port_u16().unwrap_or(80);

        info!("HTTP {} {}", method, url);

        // Create request data
        let mut request_data = self.parse_http_request(initial_data, host, port)?;

        // Check for WebSocket upgrade
        if is_websocket_upgrade(&request_data.headers) {
            return self
                .handle_websocket_upgrade_http(&mut client_socket, &request_data, host, port)
                .await;
        }

        // Apply rewrite rules to request
        if let Some(ref rewrite_manager) = self.rewrite_manager {
            rewrite_manager.rewrite_request(&mut request_data);
        }

        // Check for mock response
        if let Some(ref mock_manager) = self.mock_manager {
            if let Some(mock) = mock_manager.find_matching_mock(&request_data) {
                debug!("Mock matched: {} for {}", mock.name, request_data.url);

                // Apply throttle latency if enabled
                if let Some(ref throttle_manager) = self.throttle_manager {
                    throttle_manager.apply_latency().await;
                }

                // Build mock response
                let response = self.build_mock_response(&mock.response).await;

                // Store the request
                let session_id = self.traffic_store.current_session_id();
                let entry = TrafficEntry::new(&session_id, request_data.clone());
                self.traffic_store.store_request(&entry)?;
                self.traffic_store.store_response(&entry.id, &response)?;
                let _ = self.traffic_tx.send(entry);

                // Send response to client
                let response_bytes = self.build_response_bytes(&response);
                client_socket.write_all(&response_bytes).await?;

                info!(
                    "{} {} -> {} (mocked)",
                    request_data.method, request_data.url, response.status_code
                );
                return Ok(());
            }
        }

        // Check for breakpoint on request
        if let Some(ref breakpoint_manager) = self.breakpoint_manager {
            if let Some(rule) = breakpoint_manager.check_request(&request_data) {
                debug!("Breakpoint hit: {} for {}", rule.name, request_data.url);

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
                        return Ok(());
                    }
                    BreakpointDecision::Continue => {}
                    BreakpointDecision::Modify { modifications } => {
                        BreakpointManager::apply_request_modifications(
                            &mut request_data,
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
                        };

                        let session_id = self.traffic_store.current_session_id();
                        let entry = TrafficEntry::new(&session_id, request_data.clone());
                        self.traffic_store.store_request(&entry)?;
                        self.traffic_store.store_response(&entry.id, &response)?;
                        let _ = self.traffic_tx.send(entry);

                        let response_bytes = self.build_response_bytes(&response);
                        client_socket.write_all(&response_bytes).await?;

                        info!(
                            "{} {} -> {} (breakpoint response)",
                            request_data.method, request_data.url, status_code
                        );
                        return Ok(());
                    }
                }
            }
        }

        // Skip storing Madhyamas's own API requests to prevent feedback loops
        let should_capture = !self.should_exclude_from_capture(&request_data);

        // Store the request (if not excluded)
        let session_id = self.traffic_store.current_session_id();
        let entry = TrafficEntry::new(&session_id, request_data.clone());
        if should_capture {
            self.traffic_store.store_request(&entry)?;
            // Broadcast to WebSocket clients
            let _ = self.traffic_tx.send(entry.clone());
        }

        // Apply throttle latency if enabled
        if let Some(ref throttle_manager) = self.throttle_manager {
            throttle_manager.apply_latency().await;
        }

        // Forward to upstream
        let start = std::time::Instant::now();

        match self
            .forward_http_request(&request_data, &mut client_socket)
            .await
        {
            Ok(mut response) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                response.duration_ms = duration_ms;

                // Apply rewrite rules to response
                if let Some(ref rewrite_manager) = self.rewrite_manager {
                    rewrite_manager.rewrite_response(&request_data, &mut response);
                }

                // Check for breakpoint on response
                if let Some(ref breakpoint_manager) = self.breakpoint_manager {
                    if let Some(rule) = breakpoint_manager.check_response(&request_data, &response)
                    {
                        debug!(
                            "Breakpoint hit on response: {} for {}",
                            rule.name, request_data.url
                        );

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
                                return Ok(());
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

                if should_capture {
                    self.traffic_store.store_response(&entry.id, &response)?;
                }
                info!(
                    "{} {} -> {} ({}ms)",
                    request_data.method, request_data.url, response.status_code, duration_ms
                );
            }
            Err(e) => {
                warn!("Failed to forward HTTP request: {}", e);
            }
        }

        Ok(())
    }

    /// Parse HTTP request from bytes
    fn parse_http_request(&self, data: &[u8], host: &str, port: u16) -> crate::Result<RequestData> {
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

        // Extract body if present
        let body = if content_length > 0 {
            let header_end = data
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .map(|p| p + 4);

            if let Some(start) = header_end {
                let body_data = &data[start..];
                Some(body_data.to_vec())
            } else {
                None
            }
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
        })
    }

    /// Forward HTTPS request to upstream server
    async fn forward_https_request(
        &self,
        request_data: &RequestData,
        client_stream: &mut tokio_rustls::server::TlsStream<TcpStream>,
    ) -> crate::Result<ResponseData> {
        // Connect to upstream server
        let upstream_socket = TcpStream::connect((&request_data.host[..], 443))
            .await
            .map_err(|e| Error::Proxy(format!("Failed to connect to upstream: {}", e)))?;

        // Create TLS connector
        let tls_config = self.create_tls_client_config();
        let connector = tokio_rustls::TlsConnector::from(tls_config);
        let server_name = rustls::pki_types::ServerName::try_from(request_data.host.clone())
            .map_err(|e| Error::Tls(format!("Invalid server name: {}", e)))?;

        let mut upstream_stream = connector
            .connect(server_name, upstream_socket)
            .await
            .map_err(|e| Error::Tls(format!("TLS connection to upstream failed: {}", e)))?;

        // Build and send request
        let request_bytes = self.build_http_request(request_data);
        upstream_stream.write_all(&request_bytes).await?;

        // Read ALL response data (loop until upstream closes connection)
        let mut response_buf = Vec::new();
        let mut chunk = vec![0u8; 65536];
        loop {
            match upstream_stream.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => response_buf.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }

        if response_buf.is_empty() {
            return Err(Error::Proxy("Empty response from upstream".into()));
        }

        // Parse response (for storage)
        let response_data = self.parse_http_response(&response_buf)?;

        // Forward complete response to client
        client_stream.write_all(&response_buf).await?;

        Ok(response_data)
    }

    /// Forward HTTP request to upstream server
    async fn forward_http_request(
        &self,
        request_data: &RequestData,
        client_stream: &mut TcpStream,
    ) -> crate::Result<ResponseData> {
        // Connect to upstream server
        let upstream_socket = TcpStream::connect((&request_data.host[..], 80))
            .await
            .map_err(|e| Error::Proxy(format!("Failed to connect to upstream: {}", e)))?;

        let (mut upstream_read, mut upstream_write) = upstream_socket.into_split();

        // Send request
        let request_bytes = self.build_http_request(request_data);
        upstream_write.write_all(&request_bytes).await?;

        // Read ALL response data (loop until upstream closes connection)
        let mut response_buf = Vec::new();
        let mut chunk = vec![0u8; 65536];
        loop {
            match upstream_read.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => response_buf.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }

        if response_buf.is_empty() {
            return Err(Error::Proxy("Empty response from upstream".into()));
        }

        // Parse response (for storage)
        let response_data = self.parse_http_response(&response_buf)?;

        // Forward complete response to client
        client_stream.write_all(&response_buf).await?;

        Ok(response_data)
    }

    /// Build HTTP request bytes
    fn build_http_request(&self, request_data: &RequestData) -> Vec<u8> {
        let mut request = format!("{} {} HTTP/1.1\r\n", request_data.method, request_data.path);

        for (key, value) in &request_data.headers {
            // Skip hop-by-hop headers
            if !matches!(
                key.to_lowercase().as_str(),
                "connection" | "keep-alive" | "transfer-encoding" | "upgrade"
            ) {
                request.push_str(&format!("{}: {}\r\n", key, value));
            }
        }

        request.push_str("Connection: close\r\n");
        request.push_str("\r\n");

        let mut bytes = request.into_bytes();

        if let Some(ref body) = request_data.body {
            bytes.extend(body);
        }

        bytes
    }

    /// Parse HTTP response
    fn parse_http_response(&self, data: &[u8]) -> crate::Result<ResponseData> {
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
            duration_ms: 0, // Will be set by caller
        })
    }

    /// Create TLS client config for connecting to upstream servers
    fn create_tls_client_config(&self) -> Arc<rustls::ClientConfig> {
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification::new()))
            .with_no_client_auth();

        Arc::new(config)
    }

    /// Subscribe to traffic updates
    pub fn subscribe(&self) -> broadcast::Receiver<TrafficEntry> {
        self.traffic_tx.subscribe()
    }

    /// Check if proxy is running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    /// Build a mock response from mock configuration
    async fn build_mock_response(
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
        }
    }

    /// Build HTTP response bytes from ResponseData
    fn build_response_bytes(&self, response: &ResponseData) -> Vec<u8> {
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

    /// Handle WebSocket upgrade over TLS connection
    async fn handle_websocket_upgrade_tls(
        &self,
        client_stream: &mut tokio_rustls::server::TlsStream<TcpStream>,
        request_data: &RequestData,
        host: &str,
        port: u16,
    ) -> crate::Result<()> {
        info!("WebSocket upgrade detected (TLS): {}", request_data.url);

        // Connect to upstream WebSocket server
        let upstream_socket = TcpStream::connect((host, port))
            .await
            .map_err(|e| Error::Proxy(format!("Failed to connect to upstream: {}", e)))?;

        // Create TLS connector for upstream
        let tls_config = self.create_tls_client_config();
        let connector = tokio_rustls::TlsConnector::from(tls_config);
        let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
            .map_err(|e| Error::Tls(format!("Invalid server name: {}", e)))?;

        let mut upstream_stream = connector
            .connect(server_name, upstream_socket)
            .await
            .map_err(|e| Error::Tls(format!("TLS connection to upstream failed: {}", e)))?;

        // Forward the WebSocket upgrade request to upstream
        let upgrade_request = self.build_websocket_upgrade_request(request_data);
        upstream_stream.write_all(&upgrade_request).await?;

        // Read the upgrade response
        let mut response_buf = vec![0u8; 4096];
        let n = upstream_stream
            .read(&mut response_buf)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to read upstream response: {}", e)))?;

        // Parse response to verify 101 Switching Protocols
        let response_str = String::from_utf8_lossy(&response_buf[..n]);
        if !response_str.starts_with("HTTP/1.1 101") {
            warn!(
                "WebSocket upgrade failed: {}",
                response_str.lines().next().unwrap_or("")
            );
            client_stream.write_all(&response_buf[..n]).await?;
            return Ok(());
        }

        // Forward the upgrade response to client
        client_stream.write_all(&response_buf[..n]).await?;

        // Create connection tracking
        if let Some(ref ws_manager) = self.ws_manager {
            let id = ws_manager.create_connection(
                &request_data.url,
                host,
                &request_data.path,
                request_data.headers.clone(),
            );

            // Parse response headers
            let response_headers = self.parse_response_headers(&response_buf[..n]);
            ws_manager.complete_connection(&id, response_headers, None);
        }

        info!(
            "WebSocket connection established (TLS): {}",
            request_data.url
        );

        // Simple bidirectional copy
        let (mut client_rd, mut client_wr) = tokio::io::split(client_stream.get_mut().0);
        let (mut upstream_rd, mut upstream_wr) = tokio::io::split(upstream_stream.get_mut().0);

        let client_to_server = async {
            if let Err(e) = tokio::io::copy(&mut client_rd, &mut upstream_wr).await {
                warn!("Error copying client to server: {}", e);
            }
        };

        let server_to_client = async {
            if let Err(e) = tokio::io::copy(&mut upstream_rd, &mut client_wr).await {
                warn!("Error copying server to client: {}", e);
            }
        };

        tokio::select! {
            _ = client_to_server => {},
            _ = server_to_client => {},
        }

        Ok(())
    }

    /// Handle WebSocket upgrade over plain HTTP connection
    async fn handle_websocket_upgrade_http(
        &self,
        client_socket: &mut TcpStream,
        request_data: &RequestData,
        host: &str,
        port: u16,
    ) -> crate::Result<()> {
        info!("WebSocket upgrade detected (HTTP): {}", request_data.url);

        // Connect to upstream WebSocket server
        let mut upstream_socket = TcpStream::connect((host, port))
            .await
            .map_err(|e| Error::Proxy(format!("Failed to connect to upstream: {}", e)))?;

        // Forward the WebSocket upgrade request to upstream
        let upgrade_request = self.build_websocket_upgrade_request(request_data);
        upstream_socket.write_all(&upgrade_request).await?;

        // Read the upgrade response
        let mut response_buf = vec![0u8; 4096];
        let n = upstream_socket
            .read(&mut response_buf)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to read upstream response: {}", e)))?;

        // Parse response to verify 101 Switching Protocols
        let response_str = String::from_utf8_lossy(&response_buf[..n]);
        if !response_str.starts_with("HTTP/1.1 101") {
            warn!(
                "WebSocket upgrade failed: {}",
                response_str.lines().next().unwrap_or("")
            );
            client_socket.write_all(&response_buf[..n]).await?;
            return Ok(());
        }

        // Forward the upgrade response to client
        client_socket.write_all(&response_buf[..n]).await?;

        // Create connection tracking
        if let Some(ref ws_manager) = self.ws_manager {
            let id = ws_manager.create_connection(
                &request_data.url,
                host,
                &request_data.path,
                request_data.headers.clone(),
            );

            // Parse response headers
            let response_headers = self.parse_response_headers(&response_buf[..n]);
            ws_manager.complete_connection(&id, response_headers, None);
        }

        info!(
            "WebSocket connection established (HTTP): {}",
            request_data.url
        );

        // Simple bidirectional copy
        let (mut client_rd, mut client_wr) = tokio::io::split(client_socket);
        let (mut upstream_rd, mut upstream_wr) = tokio::io::split(&mut upstream_socket);

        let client_to_server = async {
            if let Err(e) = tokio::io::copy(&mut client_rd, &mut upstream_wr).await {
                warn!("Error copying client to server: {}", e);
            }
        };

        let server_to_client = async {
            if let Err(e) = tokio::io::copy(&mut upstream_rd, &mut client_wr).await {
                warn!("Error copying server to client: {}", e);
            }
        };

        tokio::select! {
            _ = client_to_server => {},
            _ = server_to_client => {},
        }

        Ok(())
    }

    /// Build WebSocket upgrade request
    fn build_websocket_upgrade_request(&self, request_data: &RequestData) -> Vec<u8> {
        let mut request = format!("GET {} HTTP/1.1\r\n", request_data.path);

        for (key, value) in &request_data.headers {
            request.push_str(&format!("{}: {}\r\n", key, value));
        }

        request.push_str("\r\n");
        request.into_bytes()
    }

    /// Parse response headers from buffer
    fn parse_response_headers(&self, data: &[u8]) -> std::collections::HashMap<String, String> {
        let mut headers = std::collections::HashMap::new();
        let response_str = String::from_utf8_lossy(data);

        for line in response_str.lines().skip(1) {
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        headers
    }

    /// Forward WebSocket frames between client and server
    #[allow(dead_code)]
    async fn forward_websocket_frames(
        &self,
        mut client_read: tokio::io::ReadHalf<tokio::net::TcpStream>,
        _client_write: tokio::io::WriteHalf<tokio::net::TcpStream>,
        mut upstream_read: tokio::io::ReadHalf<tokio::net::TcpStream>,
        mut upstream_write: tokio::io::WriteHalf<tokio::net::TcpStream>,
        conn_id: Option<&str>,
    ) -> crate::Result<()> {
        let mut client_buf = vec![0u8; 65536];
        let mut upstream_buf = vec![0u8; 65536];

        loop {
            tokio::select! {
                // Client to server
                result = client_read.read(&mut client_buf) => {
                    match result {
                        Ok(0) => {
                            info!("WebSocket client disconnected");
                            if let Some(id) = conn_id {
                                if let Some(ref ws_manager) = self.ws_manager {
                                    ws_manager.close_connection(id);
                                }
                            }
                            break;
                        }
                        Ok(n) => {
                            // Record message if tracking is enabled
                            if let (Some(id), Some(ref ws_manager)) = (conn_id, &self.ws_manager) {
                                self.record_ws_frame(id, WsDirection::Send, &client_buf[..n], ws_manager);
                            }

                            // Forward to upstream
                            if let Err(e) = upstream_write.write_all(&client_buf[..n]).await {
                                warn!("Failed to forward WebSocket frame to upstream: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Error reading from WebSocket client: {}", e);
                            break;
                        }
                    }
                }

                // Server to client
                result = upstream_read.read(&mut upstream_buf) => {
                    match result {
                        Ok(0) => {
                            info!("WebSocket server disconnected");
                            if let Some(id) = conn_id {
                                if let Some(ref ws_manager) = self.ws_manager {
                                    ws_manager.close_connection(id);
                                }
                            }
                            break;
                        }
                        Ok(n) => {
                            // Record message if tracking is enabled
                            if let (Some(id), Some(ref ws_manager)) = (conn_id, &self.ws_manager) {
                                self.record_ws_frame(id, WsDirection::Receive, &upstream_buf[..n], ws_manager);
                            }

                            // Forward to client (we need to re-acquire the write handle)
                            // For now, just log - this is a simplified implementation
                            debug!("Received {} bytes from WebSocket server", n);
                        }
                        Err(e) => {
                            warn!("Error reading from WebSocket server: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Record a WebSocket frame
    #[allow(dead_code)]
    fn record_ws_frame(
        &self,
        conn_id: &str,
        direction: WsDirection,
        data: &[u8],
        ws_manager: &WsManager,
    ) {
        // Parse frame header to get message type
        if let Some((fin, opcode, _payload_len, _header_len)) =
            crate::websocket::WsFrameParser::parse_header(data)
        {
            let msg_type = crate::websocket::WsFrameParser::message_type_from_opcode(opcode);

            // Create payload
            let payload = match msg_type {
                WsMessageType::Text => {
                    if let Ok(text) = std::str::from_utf8(data) {
                        WsPayload::text(text.to_string())
                    } else {
                        WsPayload::binary(data.to_vec())
                    }
                }
                WsMessageType::Binary => WsPayload::binary(data.to_vec()),
                _ => WsPayload::binary(data.to_vec()),
            };

            ws_manager.record_message(conn_id, direction, msg_type, payload);

            if fin {
                debug!("WebSocket {:?} frame: {} bytes", direction, data.len());
            }
        }
    }
}

/// Skip server certificate verification (for proxy use)
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

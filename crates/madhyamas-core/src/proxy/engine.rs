//! Main proxy engine
//!
//! This module is focused on connection management: accepting TCP connections,
//! performing TLS handshakes, and detecting WebSocket upgrades. The shared
//! HTTP request/response processing logic (rewrites, mocks, breakpoints,
//! traffic recording, upstream forwarding) lives in [`crate::proxy::pipeline`].

use crate::auto_save::AutoSaveManager;
use crate::config::ProxyConfig;
use crate::extension::ExtensionManager;
#[cfg(feature = "grpc")]
use crate::grpc::GrpcManager;
use crate::intercept::{
    BlockListManager, BreakpointManager, MockManager, RewriteManager, ThrottleManager,
};
use crate::mirror::MirrorWriter;
use crate::performance::{MemoryManager, MetricsCollector, PerformanceMonitor};
#[cfg(feature = "plugins")]
use crate::plugin::PluginManager;
use crate::proxy::pipeline::{Pipeline, RequestOutcome};
#[cfg(feature = "scripting")]
use crate::scripting::ScriptRuntime;
use crate::storage::TrafficStoreBackend;
use crate::tls::CertificateManager;
use crate::traffic::{RequestData, TrafficEntry};
use crate::websocket::{
    is_websocket_upgrade, WsDirection, WsFrameParser, WsManager, WsMessageType, WsPayload,
};
use crate::Error;
use bytes::Bytes;
use futures::StreamExt;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Trait for validating proxy-level authentication (Phase 9.6).
///
/// When `--proxy-auth` is enabled, the proxy engine calls
/// [`ProxyAuthValidator::validate`] with the credentials extracted from
/// the `Proxy-Authorization` or `X-API-Key` header before forwarding a
/// request. If validation fails, the proxy returns `407 Proxy
/// Authentication Required`.
///
/// The enterprise crate implements this trait via its `AuthManager` (JWT
/// + API key validation). In the OSS tier, no validator is set and proxy
/// auth is not enforced.
#[async_trait::async_trait]
pub trait ProxyAuthValidator: Send + Sync {
    /// Validate proxy credentials. Returns `Ok(())` if the credentials are
    /// valid, or an error message describing why they were rejected.
    async fn validate(&self, credentials: &ProxyCredentials) -> Result<(), String>;
}

/// Credentials extracted from a proxy request for auth validation
/// (Phase 9.6).
#[derive(Debug, Clone)]
pub enum ProxyCredentials {
    /// `Proxy-Authorization: Basic <base64(user:pass)>` — the decoded
    /// `username:password` string.
    ProxyBasicAuth(String),
    /// `X-API-Key: <key>` — the raw API key value.
    ApiKey(String),
    /// `Proxy-Authorization: Bearer <jwt>` — the raw JWT token.
    ProxyBearer(String),
}

/// Proxy engine state
pub struct ProxyEngine {
    /// Shared, live-updatable configuration. The same `Arc<RwLock<ProxyConfig>>`
    /// is held by the API layer so that config changes (e.g. passthrough domains
    /// added via the web UI) are immediately visible to the proxy engine.
    config: Arc<RwLock<ProxyConfig>>,
    cert_manager: Arc<CertificateManager>,
    traffic_store: Arc<dyn TrafficStoreBackend + Send + Sync>,
    /// Shared HTTP client for upstream forwarding. Reused across all requests
    /// for connection pooling, TLS session resumption, and HTTP/2 multiplexing.
    /// Creating a new client per request (as done previously) causes many
    /// servers to rate-limit or reject connections, and prevents HTTP/2
    /// stream multiplexing.
    http_client: reqwest::Client,
    mock_manager: OnceLock<Arc<MockManager>>,
    rewrite_manager: OnceLock<Arc<RewriteManager>>,
    breakpoint_manager: OnceLock<Arc<BreakpointManager>>,
    throttle_manager: OnceLock<Arc<ThrottleManager>>,
    /// Block list manager (blocks requests to matching domains)
    block_list_manager: OnceLock<Arc<BlockListManager>>,
    /// WebSocket traffic manager
    ws_manager: OnceLock<Arc<WsManager>>,
    /// gRPC traffic manager
    #[cfg(feature = "grpc")]
    grpc_manager: OnceLock<Arc<GrpcManager>>,
    /// JavaScript scripting runtime
    #[cfg(feature = "scripting")]
    script_runtime: OnceLock<Arc<ScriptRuntime>>,
    /// Plugin manager
    #[cfg(feature = "plugins")]
    plugin_manager: OnceLock<Arc<PluginManager>>,
    /// Unified extension manager (wraps scripting + plugins)
    extension_manager: OnceLock<Arc<ExtensionManager>>,
    /// Metrics collector (request/response counts, latency histogram, etc.)
    metrics_collector: OnceLock<Arc<MetricsCollector>>,
    /// Memory manager (tracks traffic memory usage and GC pressure)
    memory_manager: OnceLock<Arc<MemoryManager>>,
    /// Performance monitor (background health checks and alerting)
    performance_monitor: OnceLock<Arc<PerformanceMonitor>>,
    /// Auto Save manager (periodic session backup and rotation).
    /// Only started when `auto_save.enabled` is true in the config.
    auto_save_manager: OnceLock<Arc<AutoSaveManager>>,
    /// Channel to broadcast traffic updates to WebSocket clients
    traffic_tx: broadcast::Sender<TrafficEntry>,
    /// Whether the proxy is running
    running: RwLock<bool>,
    /// Optional proxy auth validator (Phase 9.6). When set, proxy
    /// CONNECT/HTTP requests must supply valid credentials or receive
    /// `407 Proxy Authentication Required`. `None` in OSS mode or when
    /// `--proxy-auth` is not enabled.
    proxy_auth_validator: OnceLock<Arc<dyn ProxyAuthValidator>>,
}

impl ProxyEngine {
    /// Create a new proxy engine
    pub async fn new(
        config: Arc<RwLock<ProxyConfig>>,
        cert_manager: Arc<CertificateManager>,
        traffic_store: Arc<dyn TrafficStoreBackend + Send + Sync>,
    ) -> crate::Result<Arc<Self>> {
        let (traffic_tx, _) = broadcast::channel(1024);

        // Build a shared HTTP client for all upstream forwarding.
        //
        // Key settings:
        // - Upstream proxy chaining: when `upstream_proxy` is enabled in the
        //   config, we attach a `reqwest::Proxy` so all outbound HTTP/HTTPS
        //   requests are forwarded through the configured corporate/SOCKS5
        //   proxy. Otherwise we call `no_proxy()` to avoid picking up system
        //   proxy environment variables (which could create a feedback loop
        //   where the proxy forwards to itself).
        // - `redirect(Policy::none())`: the proxy must return 3xx responses to
        //   the client; it should not silently follow redirects upstream.
        // - No auto-decompression: we store the raw compressed body and
        //   preserve Content-Encoding so the frontend can toggle views.
        // - Connection pool: reqwest reuses TCP/TLS connections across
        //   requests to the same host, enabling HTTP/2 multiplexing and TLS
        //   session resumption. This is critical for compatibility — many
        //   servers reject or rate-limit clients that open a new connection
        //   for every request.
        // - Generous timeout (120s) for slow APIs / large downloads.
        let upstream_cfg = config.read().upstream_proxy.clone();
        let mut client_builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(120))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(20)
            .gzip(false)
            .deflate(false)
            .brotli(false);

        if let Some(proxy_url) = upstream_cfg.proxy_url()? {
            info!(
                "Upstream proxy chaining enabled: {} (protocol={}, auth={})",
                proxy_url,
                upstream_cfg.protocol,
                if upstream_cfg.auth_enabled() {
                    "yes"
                } else {
                    "no"
                }
            );
            let mut proxy = reqwest::Proxy::all(&proxy_url)
                .map_err(|e| Error::Proxy(format!("Failed to configure upstream proxy: {}", e)))?;
            if upstream_cfg.auth_enabled() {
                proxy = proxy.basic_auth(
                    upstream_cfg.auth_username.as_deref().unwrap_or(""),
                    upstream_cfg.auth_password.as_deref().unwrap_or(""),
                );
            }
            client_builder = client_builder.proxy(proxy);
        } else {
            // No upstream proxy configured — explicitly disable system proxy
            // detection to avoid feedback loops and unexpected routing.
            client_builder = client_builder.no_proxy();
        }

        let http_client = client_builder
            .build()
            .map_err(|e| Error::Proxy(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Arc::new(Self {
            config,
            cert_manager,
            traffic_store,
            http_client,
            mock_manager: OnceLock::new(),
            rewrite_manager: OnceLock::new(),
            breakpoint_manager: OnceLock::new(),
            throttle_manager: OnceLock::new(),
            block_list_manager: OnceLock::new(),
            ws_manager: OnceLock::new(),
            #[cfg(feature = "grpc")]
            grpc_manager: OnceLock::new(),
            #[cfg(feature = "scripting")]
            script_runtime: OnceLock::new(),
            #[cfg(feature = "plugins")]
            plugin_manager: OnceLock::new(),
            extension_manager: OnceLock::new(),
            metrics_collector: OnceLock::new(),
            memory_manager: OnceLock::new(),
            performance_monitor: OnceLock::new(),
            auto_save_manager: OnceLock::new(),
            traffic_tx,
            running: RwLock::new(false),
            proxy_auth_validator: OnceLock::new(),
        }))
    }

    /// Build a [`Pipeline`] borrowing the shared engine state for processing
    /// one or more requests on a connection.
    fn pipeline(&self) -> Pipeline<'_> {
        // Snapshot the current config so the pipeline sees live updates
        // (e.g. passthrough domains added via the API) while still borrowing
        // the other shared state for the connection's lifetime.
        let config = self.config.read().clone();
        Pipeline::new(
            config,
            self.http_client.clone(),
            &*self.traffic_store,
            &self.traffic_tx,
            self.mock_manager.get(),
            self.rewrite_manager.get(),
            self.breakpoint_manager.get(),
            self.throttle_manager.get(),
            self.block_list_manager.get(),
            #[cfg(feature = "grpc")]
            self.grpc_manager.get(),
            #[cfg(feature = "scripting")]
            self.script_runtime.get(),
            #[cfg(feature = "plugins")]
            self.plugin_manager.get(),
            self.extension_manager.get(),
            self.metrics_collector.get(),
            self.memory_manager.get(),
        )
    }

    /// Set the mock manager
    pub fn with_mock_manager(self: Arc<Self>, manager: Arc<MockManager>) -> Arc<Self> {
        let _ = self.mock_manager.set(manager);
        self
    }

    /// Set the rewrite manager
    pub fn with_rewrite_manager(self: Arc<Self>, manager: Arc<RewriteManager>) -> Arc<Self> {
        let _ = self.rewrite_manager.set(manager);
        self
    }

    /// Set the breakpoint manager
    pub fn with_breakpoint_manager(self: Arc<Self>, manager: Arc<BreakpointManager>) -> Arc<Self> {
        let _ = self.breakpoint_manager.set(manager);
        self
    }

    /// Set the throttle manager
    pub fn with_throttle_manager(self: Arc<Self>, manager: Arc<ThrottleManager>) -> Arc<Self> {
        let _ = self.throttle_manager.set(manager);
        self
    }

    /// Set the block list manager
    pub fn with_block_list_manager(self: Arc<Self>, manager: Arc<BlockListManager>) -> Arc<Self> {
        let _ = self.block_list_manager.set(manager);
        self
    }

    /// Set the WebSocket manager
    pub fn with_ws_manager(self: Arc<Self>, manager: Arc<WsManager>) -> Arc<Self> {
        let _ = self.ws_manager.set(manager);
        self
    }

    /// Set the gRPC manager
    #[cfg(feature = "grpc")]
    pub fn with_grpc_manager(self: Arc<Self>, manager: Arc<GrpcManager>) -> Arc<Self> {
        let _ = self.grpc_manager.set(manager);
        self
    }

    /// Set the script runtime
    #[cfg(feature = "scripting")]
    pub fn with_script_runtime(self: Arc<Self>, runtime: Arc<ScriptRuntime>) -> Arc<Self> {
        let _ = self.script_runtime.set(runtime);
        self
    }

    /// Set the plugin manager
    #[cfg(feature = "plugins")]
    pub fn with_plugin_manager(self: Arc<Self>, manager: Arc<PluginManager>) -> Arc<Self> {
        let _ = self.plugin_manager.set(manager);
        self
    }

    /// Set the unified extension manager.
    pub fn with_extension_manager(self: Arc<Self>, manager: Arc<ExtensionManager>) -> Arc<Self> {
        let _ = self.extension_manager.set(manager);
        self
    }

    /// Set the metrics collector.
    ///
    /// When attached, the pipeline records every request/response (and
    /// WebSocket open/close) so that `MetricsCollector::snapshot()` reflects
    /// live traffic.
    pub fn with_metrics_collector(self: Arc<Self>, collector: Arc<MetricsCollector>) -> Arc<Self> {
        let _ = self.metrics_collector.set(collector);
        self
    }

    /// Set the memory manager.
    ///
    /// When attached, the pipeline checks memory pressure on each request and
    /// the performance monitor includes memory usage in its alerts.
    pub fn with_memory_manager(self: Arc<Self>, manager: Arc<MemoryManager>) -> Arc<Self> {
        let _ = self.memory_manager.set(manager);
        self
    }

    /// Set the performance monitor and start its background monitoring task.
    ///
    /// The monitor periodically inspects the metrics collector and memory
    /// manager (when attached) and emits alerts when thresholds are exceeded.
    /// The task runs until the monitor is dropped.
    pub fn with_performance_monitor(
        self: Arc<Self>,
        monitor: Arc<PerformanceMonitor>,
    ) -> Arc<Self> {
        // Start the background monitoring task if both the metrics collector
        // and memory manager are already attached. Otherwise the monitor will
        // simply not have data to inspect until they are attached (in which
        // case the caller should call `monitor.start_monitoring` manually).
        if let (Some(metrics), Some(memory)) =
            (self.metrics_collector.get(), self.memory_manager.get())
        {
            monitor.start_monitoring(metrics.clone(), memory.clone(), Duration::from_secs(30));
        }
        let _ = self.performance_monitor.set(monitor);
        self
    }

    /// Set the Auto Save manager and start its background task if enabled.
    ///
    /// The manager runs a `tokio::time::interval` background task that
    /// periodically exports the current session to a backup directory. The
    /// task is stopped automatically when the manager is dropped (graceful
    /// shutdown via a `oneshot` channel).
    pub fn with_auto_save_manager(self: Arc<Self>, manager: Arc<AutoSaveManager>) -> Arc<Self> {
        manager.clone().start();
        let _ = self.auto_save_manager.set(manager);
        self
    }

    /// Attach a mirror writer and register it with the traffic store so that
    /// captured responses are written to disk following the URL path
    /// structure. The writer holds a live-updatable config shared with the
    /// API layer so runtime changes take effect immediately.
    pub fn with_mirror_writer(self: Arc<Self>, writer: Arc<MirrorWriter>) -> Arc<Self> {
        self.traffic_store.set_mirror_writer(writer);
        self
    }

    /// Attach a proxy auth validator (Phase 9.6). When set, proxy
    /// CONNECT/HTTP requests must supply valid credentials or receive
    /// `407 Proxy Authentication Required`.
    pub fn with_proxy_auth_validator(
        self: Arc<Self>,
        validator: Arc<dyn ProxyAuthValidator>,
    ) -> Arc<Self> {
        let _ = self.proxy_auth_validator.set(validator);
        self
    }

    /// Get the metrics collector, if attached.
    pub fn metrics_collector(&self) -> Option<&Arc<MetricsCollector>> {
        self.metrics_collector.get()
    }

    /// Get the memory manager, if attached.
    pub fn memory_manager(&self) -> Option<&Arc<MemoryManager>> {
        self.memory_manager.get()
    }

    /// Get the performance monitor, if attached.
    pub fn performance_monitor(&self) -> Option<&Arc<PerformanceMonitor>> {
        self.performance_monitor.get()
    }

    /// Get the Auto Save manager, if attached.
    pub fn auto_save_manager(&self) -> Option<&Arc<AutoSaveManager>> {
        self.auto_save_manager.get()
    }

    /// Start the proxy server
    pub async fn start(self: Arc<Self>) -> crate::Result<()> {
        let addr: SocketAddr = self
            .config
            .read()
            .proxy_addr()
            .parse()
            .map_err(|e| Error::Proxy(format!("Invalid proxy address: {}", e)))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to bind proxy port: {}", e)))?;

        *self.running.write() = true;
        info!("Proxy server listening on {}", addr);

        // Optionally start a SOCKS5 listener on a separate port. SOCKS5 is a
        // blind TCP tunnel (RFC 1928); it runs alongside the HTTP/HTTPS proxy
        // listener and shares the traffic store so SOCKS connections are
        // visible in the web UI. See [`crate::proxy::socks`] for details.
        //
        // We spawn the SOCKS server in its own task because `serve_socks5`
        // owns its accept loop and must run concurrently with the HTTP proxy
        // accept loop below. The shared components are cloned out of `self`
        // so the task is `'static` and doesn't borrow the engine.
        if self.config.read().enable_socks {
            let socks_config = Arc::new(self.config.read().clone());
            let socks_traffic_store = self.traffic_store.clone();
            let socks_traffic_tx = self.traffic_tx.clone();
            let socks_engine = self.clone();
            tokio::spawn(async move {
                let ctx = crate::proxy::socks::SocksContext {
                    config: socks_config,
                    traffic_store: socks_traffic_store,
                    traffic_tx: socks_traffic_tx,
                };
                if let Err(e) = crate::proxy::socks::serve_socks5(ctx).await {
                    tracing::error!("SOCKS5 server error: {}", e);
                }
                drop(socks_engine); // keep engine alive for the task's lifetime
            });
        }

        loop {
            let (mut client_socket, client_addr) = listener
                .accept()
                .await
                .map_err(|e| Error::Proxy(format!("Failed to accept connection: {}", e)))?;

            // IP access control (allowlist). The check runs on every accept
            // using a live snapshot of the config, so API updates to
            // `allowed_ips` (`PATCH /api/config`) take effect immediately for
            // new connections without a restart. Loopback addresses are
            // always allowed (enforced inside `is_allowed`) so a
            // locally-started proxy can never be locked out.
            //
            // Rejected connections are dropped (TCP close) without spawning
            // a handler task, matching Charles Proxy's behavior.
            let reject = {
                let cfg = self.config.read();
                if !cfg.access_control_enabled() {
                    false
                } else {
                    match cfg.access_control_list() {
                        Ok(acl) => !acl.is_allowed(client_addr.ip()),
                        Err(e) => {
                            // A malformed allowlist entry should never reach
                            // here (validated at config load / API patch
                            // time), but if it does we fail closed for
                            // non-localhost traffic and log the error.
                            warn!(
                                "Access control list parse error: {}. Rejecting connection from {}",
                                e, client_addr
                            );
                            !client_addr.ip().is_loopback()
                        }
                    }
                }
            };
            if reject {
                warn!(
                    "Connection from {} rejected by IP access control",
                    client_addr
                );
                let _ = client_socket.shutdown().await;
                continue;
            }

            let engine = self.clone();
            tokio::spawn(async move {
                // Track active connections for metrics.
                if let Some(metrics) = engine.metrics_collector.get() {
                    metrics.connection_opened();
                }
                if let Err(e) = engine.handle_connection(client_socket).await {
                    debug!("Connection error from {}: {}", client_addr, e);
                }
                if let Some(metrics) = engine.metrics_collector.get() {
                    metrics.connection_closed();
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

        // Phase 9.6: proxy auth check. When a proxy auth validator is
        // configured, extract credentials from the request headers and
        // validate them before processing. Unauthenticated requests
        // receive a 407 response.
        if let Some(validator) = self.proxy_auth_validator.get() {
            if let Err(msg) = self.check_proxy_auth(&request_str, validator).await {
                let response = format!(
                    "HTTP/1.1 407 Proxy Authentication Required\r\n\
                     Proxy-Authenticate: Basic realm=\"madhyamas\"\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\
                     \r\n\
                     {{\"error\":\"proxy_auth_required\",\"message\":\"{}\"}}",
                    msg.len(),
                    msg
                );
                let _ = client_socket.write_all(response.as_bytes()).await;
                return Ok(());
            }
        }

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

    /// Extract and validate proxy auth credentials from the raw request
    /// string (Phase 9.6). Checks `Proxy-Authorization` and `X-API-Key`
    /// headers. Returns `Ok(())` when authenticated, or `Err(message)`
    /// when credentials are missing or invalid.
    async fn check_proxy_auth(
        &self,
        request_str: &str,
        validator: &Arc<dyn ProxyAuthValidator>,
    ) -> Result<(), String> {
        let headers = parse_connect_headers(request_str);
        // Try Proxy-Authorization header first.
        if let Some(auth_val) = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("proxy-authorization"))
            .map(|(_, v)| v)
        {
            if let Some(credentials) = parse_proxy_authorization(auth_val) {
                return validator.validate(&credentials).await;
            }
        }
        // Try X-API-Key header.
        if let Some(key_val) = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-api-key"))
            .map(|(_, v)| v)
        {
            return validator
                .validate(&ProxyCredentials::ApiKey(key_val.clone()))
                .await;
        }
        Err("No proxy credentials provided".to_string())
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

        // Parse CONNECT request headers (everything after the request line).
        // These headers (User-Agent, Proxy-Authorization, etc.) are captured
        // so they can be included in traffic entries for passthrough and SSL
        // error cases, giving the user maximum visibility into the connection
        // attempt even when the actual HTTP request is not visible.
        let connect_headers = parse_connect_headers(request_str);

        info!("HTTPS CONNECT: {}:{}", host, port);

        // Check if this host is in the SSL passthrough exclusion list.
        // If so, tunnel the connection directly without TLS interception.
        // We read the shared config so live updates from the API are honored.
        if self.config.read().should_passthrough(host) {
            info!("SSL passthrough for {}:{}", host, port);
            return self
                .handle_passthrough_tunnel(client_socket, host, port, &connect_headers)
                .await;
        }

        // Generate certificate for this host
        let cert = self.cert_manager.generate_cert_for_host(host)?;

        // Send 200 Connection Established
        let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
        client_socket.write_all(response.as_bytes()).await?;

        // Perform TLS handshake with client.
        //
        // If the handshake fails (e.g. the client doesn't trust our CA
        // certificate — common with Android apps that use certificate
        // pinning or don't have the CA installed), we record a traffic
        // entry with a 502 error so the failed attempt is visible in the
        // web UI. Without this, the request would be completely invisible.
        let tls_config = self.create_tls_server_config(&cert)?;
        let acceptor = tokio_rustls::TlsAcceptor::from(tls_config);
        let mut tls_stream = match acceptor.accept(client_socket).await {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    "TLS handshake failed for {}:{} — the client likely does \
                     not trust the proxy CA certificate (common with Android \
                     apps using cert pinning). Error: {}",
                    host, port, e
                );

                // Record a traffic entry so the failed attempt is visible.
                // Include the CONNECT request headers for debugging context.
                let session_id = self.traffic_store.current_session_id();
                let entry = TrafficEntry::new(
                    &session_id,
                    RequestData {
                        method: crate::traffic::HttpMethod::Connect,
                        url: format!("https://{}:{}/", host, port),
                        host: host.to_string(),
                        path: format!(":{}", port),
                        headers: connect_headers.clone(),
                        body: None,
                        content_type: None,
                        http_version: Some("HTTP/1.1".to_string()),
                    },
                );
                let _ = self.traffic_store.store_request(&entry).await;
                let _ = self
                    .traffic_store
                    .store_response(
                        &entry.id,
                        &crate::traffic::ResponseData {
                            status_code: 502,
                            status_message: Some("Bad Gateway (TLS Handshake Failed)".to_string()),
                            headers: std::collections::HashMap::new(),
                            body: Some(
                                format!(
                                    "TLS handshake failed for {}:{}.\n\n\
                                 The client does not trust the proxy CA certificate.\n\
                                 This is common with apps using certificate pinning.\n\n\
                                 Error: {}\n\n\
                                 CONNECT request headers:\n{}",
                                    host,
                                    port,
                                    e,
                                    connect_headers
                                        .iter()
                                        .map(|(k, v)| format!("  {}: {}", k, v))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                )
                                .into_bytes(),
                            ),
                            content_type: Some("text/plain".to_string()),
                            duration_ms: 0,
                            http_version: Some("HTTP/1.1".to_string()),
                        },
                    )
                    .await;
                let _ = self.traffic_tx.send(entry);

                return Err(Error::Tls(format!("TLS handshake failed: {}", e)));
            }
        };

        // Inspect the ALPN protocol negotiated during the handshake. When
        // HTTP/2 downstream support is enabled we advertise both `h2` and
        // `http/1.1`; the negotiated protocol determines which request loop
        // handles the connection. See [`Self::create_tls_server_config`].
        let negotiated_alpn = tls_stream
            .get_ref()
            .1
            .alpn_protocol()
            .and_then(|p| std::str::from_utf8(p).ok())
            .map(|s| s.to_string());

        match negotiated_alpn.as_deref() {
            Some("h2") => {
                info!("ALPN negotiated h2 (HTTP/2) for {}:{}", host, port);
                // Hand the TLS stream to the HTTP/2 frame parser. This path
                // multiplexes streams through the same interception pipeline
                // used by HTTP/1.1, and is required for gRPC interception.
                return self.handle_h2_connection(tls_stream, host, port).await;
            }
            Some("http/1.1") => {
                debug!("ALPN negotiated http/1.1 for {}:{}", host, port);
            }
            other => {
                debug!(
                    "ALPN negotiation result for {}:{}: {:?} (no protocol or unknown)",
                    host, port, other
                );
            }
        }

        // Now we can intercept the actual HTTP request over TLS
        self.handle_tls_request(&mut tls_stream, host, port).await
    }

    /// Handle an HTTPS CONNECT request in SSL passthrough mode.
    ///
    /// Instead of performing a TLS handshake with the client and intercepting
    /// the decrypted traffic, we tunnel the raw TCP connection directly to the
    /// upstream server. The client's TLS session goes through untouched.
    ///
    /// We still record a traffic entry (flagged as `is_passthrough`) so the
    /// connection is visible in the web UI, but we cannot inspect the
    /// request/response contents.
    async fn handle_passthrough_tunnel(
        &self,
        mut client_socket: TcpStream,
        host: &str,
        port: u16,
        connect_headers: &std::collections::HashMap<String, String>,
    ) -> crate::Result<()> {
        // Send 200 Connection Established so the client starts TLS
        let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
        client_socket.write_all(response.as_bytes()).await?;

        // Record a passthrough traffic entry so the connection is visible.
        // Include the CONNECT request headers for debugging context — since
        // the actual HTTP request is encrypted inside the TLS tunnel, these
        // headers are the only metadata we can capture.
        let session_id = self.traffic_store.current_session_id();
        let mut entry = TrafficEntry::new(
            &session_id,
            RequestData {
                method: crate::traffic::HttpMethod::Connect,
                url: format!("https://{}:{}/", host, port),
                host: host.to_string(),
                path: format!(":{}", port),
                headers: connect_headers.clone(),
                body: None,
                content_type: None,
                http_version: Some("HTTP/1.1".to_string()),
            },
        );
        entry.is_passthrough = true;
        let _ = self.traffic_store.store_request(&entry).await;
        let _ = self.traffic_tx.send(entry.clone());

        // Connect to the upstream server.
        //
        // When upstream proxy chaining is active and the target host is not
        // in the bypass list, we tunnel through the upstream proxy using
        // HTTP CONNECT or SOCKS5 (depending on the configured protocol).
        // Otherwise we connect directly to the target.
        let upstream_addr = format!("{}:{}", host, port);
        let config_snapshot = self.config.read().clone();
        let use_upstream_proxy = config_snapshot.upstream_proxy_active()
            && !config_snapshot.should_bypass_upstream(host);

        let mut upstream_socket = if use_upstream_proxy {
            info!(
                "Passthrough: tunneling {} via upstream proxy {}:{} ({})",
                upstream_addr,
                config_snapshot.upstream_proxy.host,
                config_snapshot.upstream_proxy.port,
                config_snapshot.upstream_proxy.protocol
            );
            match crate::proxy::upstream_proxy::connect_through_upstream(
                &config_snapshot.upstream_proxy,
                host,
                port,
            )
            .await
            {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        "Passthrough: upstream proxy connect to {} failed: {}",
                        upstream_addr, e
                    );
                    let _ = self
                        .traffic_store
                        .store_response(
                            &entry.id,
                            &crate::traffic::ResponseData {
                                status_code: 502,
                                status_message: Some(
                                    "Bad Gateway (Upstream Proxy Connect Failed)".to_string(),
                                ),
                                headers: std::collections::HashMap::new(),
                                body: Some(
                                    format!(
                                    "SSL passthrough connection through upstream proxy failed.\n\n\
                                     Target: {}\n\
                                     Upstream proxy: {}:{} ({})\n\
                                     Error: {}\n\n\
                                     CONNECT request headers:\n{}",
                                    upstream_addr,
                                    config_snapshot.upstream_proxy.host,
                                    config_snapshot.upstream_proxy.port,
                                    config_snapshot.upstream_proxy.protocol,
                                    e,
                                    connect_headers
                                        .iter()
                                        .map(|(k, v)| format!("  {}: {}", k, v))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                )
                                    .into_bytes(),
                                ),
                                content_type: Some("text/plain".to_string()),
                                duration_ms: 0,
                                http_version: Some("HTTP/1.1".to_string()),
                            },
                        )
                        .await;
                    let _ = self.traffic_tx.send(entry);
                    return Err(Error::Proxy(format!(
                        "Upstream proxy connect failed: {}",
                        e
                    )));
                }
            }
        } else {
            match tokio::time::timeout(Duration::from_secs(30), TcpStream::connect(&upstream_addr))
                .await
            {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    warn!("Passthrough: failed to connect to {}: {}", upstream_addr, e);
                    let _ = self
                        .traffic_store
                        .store_response(
                            &entry.id,
                            &crate::traffic::ResponseData {
                                status_code: 502,
                                status_message: Some(
                                    "Bad Gateway (Passthrough Connect Failed)".to_string(),
                                ),
                                headers: std::collections::HashMap::new(),
                                body: Some(
                                    format!(
                                        "SSL passthrough connection failed.\n\n\
                                 Target: {}\n\
                                 Error: {}\n\n\
                                 CONNECT request headers:\n{}",
                                        upstream_addr,
                                        e,
                                        connect_headers
                                            .iter()
                                            .map(|(k, v)| format!("  {}: {}", k, v))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    )
                                    .into_bytes(),
                                ),
                                content_type: Some("text/plain".to_string()),
                                duration_ms: 0,
                                http_version: Some("HTTP/1.1".to_string()),
                            },
                        )
                        .await;
                    let _ = self.traffic_tx.send(entry);
                    return Err(Error::Proxy(format!("Passthrough connect failed: {}", e)));
                }
                Err(_) => {
                    warn!("Passthrough: timeout connecting to {}", upstream_addr);
                    let _ = self
                        .traffic_store
                        .store_response(
                            &entry.id,
                            &crate::traffic::ResponseData {
                                status_code: 504,
                                status_message: Some("Gateway Timeout (Passthrough)".to_string()),
                                headers: std::collections::HashMap::new(),
                                body: Some(
                                    format!(
                                        "SSL passthrough connection timed out.\n\n\
                                 Target: {}\n\
                                 Timeout: 30 seconds\n\n\
                                 CONNECT request headers:\n{}",
                                        upstream_addr,
                                        connect_headers
                                            .iter()
                                            .map(|(k, v)| format!("  {}: {}", k, v))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    )
                                    .into_bytes(),
                                ),
                                content_type: Some("text/plain".to_string()),
                                duration_ms: 30000,
                                http_version: Some("HTTP/1.1".to_string()),
                            },
                        )
                        .await;
                    let _ = self.traffic_tx.send(entry);
                    return Err(Error::Proxy("Passthrough connect timeout".into()));
                }
            }
        };

        // Record successful connection with a 200 response.
        // Include a descriptive body explaining what happened (visible in
        // the traffic detail view).
        let _ = self
            .traffic_store
            .store_response(
                &entry.id,
                &crate::traffic::ResponseData {
                    status_code: 200,
                    status_message: Some("Connection Established (SSL Passthrough)".to_string()),
                    headers: std::collections::HashMap::new(),
                    body: Some(
                        format!(
                            "SSL Passthrough — connection tunneled directly to {}.\n\n\
                         The TLS session was not intercepted; request and response\n\
                         contents (URL path, query parameters, headers, body) are\n\
                         not visible because they are encrypted inside the tunnel.\n\n\
                         CONNECT request headers:\n{}",
                            upstream_addr,
                            connect_headers
                                .iter()
                                .map(|(k, v)| format!("  {}: {}", k, v))
                                .collect::<Vec<_>>()
                                .join("\n")
                        )
                        .into_bytes(),
                    ),
                    content_type: Some("text/plain".to_string()),
                    duration_ms: 0,
                    http_version: Some("HTTP/1.1".to_string()),
                },
            )
            .await;
        let _ = self.traffic_tx.send(entry);

        // Bidirectional byte forwarding: client ↔ upstream
        // We split both sockets and copy in both directions simultaneously.
        let (mut client_rx, mut client_tx) = client_socket.split();
        let (mut upstream_rx, mut upstream_tx) = upstream_socket.split();

        let client_to_upstream = async {
            let mut buf = vec![0u8; 8192];
            loop {
                match client_rx.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if upstream_tx.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = upstream_tx.shutdown().await;
        };

        let upstream_to_client = async {
            let mut buf = vec![0u8; 8192];
            loop {
                match upstream_rx.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if client_tx.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = client_tx.shutdown().await;
        };

        // Run both directions concurrently until either side closes
        tokio::try_join!(
            tokio::time::timeout(Duration::from_secs(300), client_to_upstream),
            tokio::time::timeout(Duration::from_secs(300), upstream_to_client),
        )
        .ok();

        debug!("Passthrough tunnel closed for {}:{}", host, port);
        Ok(())
    }

    /// Create TLS server config with the generated certificate.
    ///
    /// # ALPN advertisement
    ///
    /// When `enable_h2_downstream` is **disabled** (the default), only
    /// `http/1.1` is advertised. This preserves the historical behaviour:
    /// the proxy cannot parse HTTP/2 frames on the client-facing side, so
    /// advertising `h2` would cause modern clients to negotiate HTTP/2 and
    /// then send frames the proxy interprets as binary garbage (502 errors).
    ///
    /// When `enable_h2_downstream` is **enabled**, both `h2` and `http/1.1`
    /// are advertised (with `h2` listed first so ALPN-aware clients prefer
    /// HTTP/2). The proxy then parses HTTP/2 frames via the `h2` crate in
    /// [`Self::handle_h2_connection`]. HTTP/1.1-only clients automatically
    /// fall back to `http/1.1`. This is required for gRPC interception,
    /// since gRPC mandates HTTP/2.
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

        let mut config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| Error::Tls(format!("Failed to create TLS config: {}", e)))?;

        if self.config.read().enable_h2_downstream {
            // h2 first (preferred), http/1.1 as fallback for older clients.
            config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        } else {
            // Only http/1.1 — the proxy cannot parse HTTP/2 frames when the
            // feature is disabled.
            config.alpn_protocols = vec![b"http/1.1".to_vec()];
        }

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
        let pipeline = self.pipeline();
        // One correlation id per client connection: every request on this
        // keep-alive connection carries the same connection_id in logs.
        let connection_id = uuid::Uuid::new_v4().to_string();

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

            let mut request_data = match pipeline.parse_http_request(&buf[..n], host, port) {
                Ok(data) => data,
                Err(e) => {
                    debug!("Failed to parse request on keep-alive connection: {}", e);
                    return Ok(());
                }
            };

            // Read the full request body from the TLS stream. The initial
            // read above may not have captured the entire body, which would
            // cause the upstream to wait forever and time out.
            {
                let headers = request_data.headers.clone();
                request_data.body = pipeline
                    .read_full_request_body(tls_stream, request_data.body.take(), &headers)
                    .await?;
            }

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

            // Process the request through the shared pipeline (rewrites,
            // hooks, mocks, breakpoints, upstream forwarding, recording).
            let outcome = pipeline
                .process_request_with_conn(&mut request_data, tls_stream, &connection_id)
                .await?;

            // A breakpoint abort terminates the keep-alive loop
            if outcome == RequestOutcome::Aborted {
                return Ok(());
            }

            // If the client sent Connection: close, stop the keep-alive loop
            if connection_close {
                debug!("Client requested connection close for {}", host);
                return Ok(());
            }
        }
    }

    /// Handle an HTTP/2 connection on an already-established TLS stream.
    ///
    /// This is invoked by [`Self::handle_https_tunnel`] when ALPN negotiates
    /// `h2`. The `h2` crate performs HTTP/2 framing, multiplexing, and flow
    /// control on the TLS stream. Each accepted stream is converted into a
    /// [`RequestData`] (with `http_version = "HTTP/2"`), run through the same
    /// shared [`Pipeline`] used by HTTP/1.1 (rewrites, mocks, breakpoints,
    /// upstream forwarding, traffic recording), and the resulting response is
    /// written back over the same h2 stream via [`H2ResponseWriter`].
    ///
    /// Streams are handled in independent tasks so concurrent HTTP/2 requests
    /// (and gRPC calls) are processed in parallel — the defining feature of
    /// HTTP/2 multiplexing.
    async fn handle_h2_connection(
        &self,
        tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        host: &str,
        port: u16,
    ) -> crate::Result<()> {
        // Perform the HTTP/2 server handshake (client preface + settings
        // exchange) on the TLS stream. The h2 crate owns framing/flow-control
        // from here on.
        let mut h2_conn = match h2::server::handshake(tls_stream).await {
            Ok(c) => c,
            Err(e) => {
                warn!("HTTP/2 handshake failed for {}:{}: {}", host, port, e);
                return Err(Error::Proxy(format!("h2 handshake failed: {}", e)));
            }
        };

        info!("HTTP/2 connection established for {}:{}", host, port);

        // Accept streams until the client closes the connection. `Connection`
        // implements `futures::Stream` (via the `stream` feature on the h2
        // crate), so we drive it with `.next().await`.
        while let Some(stream_result) = h2_conn.next().await {
            let (request, mut respond) = match stream_result {
                Ok(pair) => pair,
                Err(e) => {
                    warn!("h2 stream accept error for {}:{}: {}", host, port, e);
                    continue;
                }
            };

            // Snapshot all shared state by cloning the Arcs so the spawned
            // per-stream task is `'static` and self-contained. The pipeline
            // borrows from these local Arcs for the task's lifetime.
            let traffic_store = self.traffic_store.clone();
            let traffic_tx = self.traffic_tx.clone();
            let http_client = self.http_client.clone();
            let config = self.config.read().clone();
            let mock_manager = self.mock_manager.get().cloned();
            let rewrite_manager = self.rewrite_manager.get().cloned();
            let breakpoint_manager = self.breakpoint_manager.get().cloned();
            let throttle_manager = self.throttle_manager.get().cloned();
            let block_list_manager = self.block_list_manager.get().cloned();
            #[cfg(feature = "grpc")]
            let grpc_manager = self.grpc_manager.get().cloned();
            #[cfg(feature = "scripting")]
            let script_runtime = self.script_runtime.get().cloned();
            #[cfg(feature = "plugins")]
            let plugin_manager = self.plugin_manager.get().cloned();
            let extension_manager = self.extension_manager.get().cloned();
            let metrics_collector = self.metrics_collector.get().cloned();
            let memory_manager = self.memory_manager.get().cloned();

            let host_owned = host.to_string();

            tokio::spawn(async move {
                // Build a fresh pipeline borrowing from the task-local Arcs.
                let pipeline = Pipeline::new(
                    config,
                    http_client,
                    &*traffic_store,
                    &traffic_tx,
                    mock_manager.as_ref(),
                    rewrite_manager.as_ref(),
                    breakpoint_manager.as_ref(),
                    throttle_manager.as_ref(),
                    block_list_manager.as_ref(),
                    #[cfg(feature = "grpc")]
                    grpc_manager.as_ref(),
                    #[cfg(feature = "scripting")]
                    script_runtime.as_ref(),
                    #[cfg(feature = "plugins")]
                    plugin_manager.as_ref(),
                    extension_manager.as_ref(),
                    metrics_collector.as_ref(),
                    memory_manager.as_ref(),
                );

                if let Err(e) =
                    process_h2_stream(request, &mut respond, &host_owned, port, &pipeline).await
                {
                    warn!(
                        "h2 stream processing failed for {}:{}: {}",
                        host_owned, port, e
                    );
                    // Best-effort reset so the client sees a clean stream error.
                    respond.send_reset(h2::Reason::INTERNAL_ERROR);
                }
            });
        }

        // Drive the connection to completion so any in-flight response frames
        // queued by spawned tasks are flushed before we return. `poll_closed`
        // advances the internal h2 state machine (flushing queued frames)
        // and returns Ready when the underlying connection is fully closed.
        use std::future::poll_fn;
        if let Err(e) = poll_fn(|cx| h2_conn.poll_closed(cx)).await {
            debug!("h2 connection close for {}:{}: {}", host, port, e);
        }
        debug!("HTTP/2 connection closed for {}:{}", host, port);
        Ok(())
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

        let pipeline = self.pipeline();

        // Create request data
        let mut request_data = pipeline.parse_http_request(initial_data, host, port)?;

        // Read the full request body from the client. The initial read in
        // handle_connection may not have captured the entire body (especially
        // for POST/PUT with large bodies), which would cause the upstream to
        // wait forever for the remaining bytes and time out.
        {
            let headers = request_data.headers.clone();
            request_data.body = pipeline
                .read_full_request_body(&mut client_socket, request_data.body.take(), &headers)
                .await?;
        }

        // Check for WebSocket upgrade
        if is_websocket_upgrade(&request_data.headers) {
            return self
                .handle_websocket_upgrade_http(&mut client_socket, &request_data, host, port)
                .await;
        }

        // Process the request through the shared pipeline (rewrites, hooks,
        // mocks, breakpoints, upstream forwarding, recording). Non
        // keep-alive path: one connection id for this single request.
        let connection_id = uuid::Uuid::new_v4().to_string();
        pipeline
            .process_request_with_conn(&mut request_data, &mut client_socket, &connection_id)
            .await?;

        Ok(())
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

    /// Handle WebSocket upgrade over TLS connection
    async fn handle_websocket_upgrade_tls(
        &self,
        client_stream: &mut tokio_rustls::server::TlsStream<TcpStream>,
        request_data: &RequestData,
        host: &str,
        port: u16,
    ) -> crate::Result<()> {
        info!("WebSocket upgrade detected (TLS): {}", request_data.url);

        // Connect to upstream WebSocket server. When upstream proxy chaining
        // is active and the target is not bypassed, tunnel through the proxy.
        let config_snapshot = self.config.read().clone();
        let use_upstream_proxy = config_snapshot.upstream_proxy_active()
            && !config_snapshot.should_bypass_upstream(host);
        let upstream_socket = if use_upstream_proxy {
            crate::proxy::upstream_proxy::connect_through_upstream(
                &config_snapshot.upstream_proxy,
                host,
                port,
            )
            .await
            .map_err(|e| Error::Proxy(format!("Failed to connect via upstream proxy: {}", e)))?
        } else {
            TcpStream::connect((host, port))
                .await
                .map_err(|e| Error::Proxy(format!("Failed to connect to upstream: {}", e)))?
        };

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
        if let Some(ws_manager) = self.ws_manager.get() {
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

        // Track WebSocket connection for metrics.
        if let Some(metrics) = self.metrics_collector.get() {
            metrics.websocket_opened();
        }

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

        // WebSocket connection has closed.
        if let Some(metrics) = self.metrics_collector.get() {
            metrics.websocket_closed();
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

        // Connect to upstream WebSocket server. When upstream proxy chaining
        // is active and the target is not bypassed, tunnel through the proxy.
        let config_snapshot = self.config.read().clone();
        let use_upstream_proxy = config_snapshot.upstream_proxy_active()
            && !config_snapshot.should_bypass_upstream(host);
        let mut upstream_socket = if use_upstream_proxy {
            crate::proxy::upstream_proxy::connect_through_upstream(
                &config_snapshot.upstream_proxy,
                host,
                port,
            )
            .await
            .map_err(|e| Error::Proxy(format!("Failed to connect via upstream proxy: {}", e)))?
        } else {
            TcpStream::connect((host, port))
                .await
                .map_err(|e| Error::Proxy(format!("Failed to connect to upstream: {}", e)))?
        };

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
        if let Some(ws_manager) = self.ws_manager.get() {
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

        // Track WebSocket connection for metrics.
        if let Some(metrics) = self.metrics_collector.get() {
            metrics.websocket_opened();
        }

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

        // WebSocket connection has closed.
        if let Some(metrics) = self.metrics_collector.get() {
            metrics.websocket_closed();
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
        mut client_write: tokio::io::WriteHalf<tokio::net::TcpStream>,
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
                                if let Some(ws_manager) = self.ws_manager.get() {
                                    ws_manager.close_connection(id);
                                }
                            }
                            break;
                        }
                        Ok(n) => {
                            let data = &client_buf[..n];

                            // Record message if tracking is enabled
                            if let (Some(id), Some(ws_manager)) = (conn_id, self.ws_manager.get()) {
                                self.record_ws_frame(id, WsDirection::Send, data, ws_manager);
                            }

                            // Auto-reply to Ping frames from the client with a Pong
                            // sent back to the client. The proxy acts as a server
                            // towards the client, so the Pong is sent unmasked.
                            for payload in extract_ping_payloads(data) {
                                let pong = build_pong_frame(&payload, false);
                                if let Err(e) = client_write.write_all(&pong).await {
                                    warn!("Failed to send WebSocket Pong to client: {}", e);
                                    break;
                                }
                                debug!(
                                    "Sent auto-reply Pong ({} bytes) to WebSocket client",
                                    pong.len()
                                );
                            }

                            // Forward the original frame (including the Ping) to upstream
                            if let Err(e) = upstream_write.write_all(data).await {
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
                                if let Some(ws_manager) = self.ws_manager.get() {
                                    ws_manager.close_connection(id);
                                }
                            }
                            break;
                        }
                        Ok(n) => {
                            let data = &upstream_buf[..n];

                            // Record message if tracking is enabled
                            if let (Some(id), Some(ws_manager)) = (conn_id, self.ws_manager.get()) {
                                self.record_ws_frame(id, WsDirection::Receive, data, ws_manager);
                            }

                            // Auto-reply to Ping frames from the server with a Pong
                            // sent back to the server. The proxy acts as a client
                            // towards the server, so the Pong must be masked.
                            for payload in extract_ping_payloads(data) {
                                let pong = build_pong_frame(&payload, true);
                                if let Err(e) = upstream_write.write_all(&pong).await {
                                    warn!("Failed to send WebSocket Pong to upstream: {}", e);
                                    break;
                                }
                                debug!(
                                    "Sent auto-reply Pong ({} bytes) to WebSocket server",
                                    pong.len()
                                );
                            }

                            // Forward the original frame (including the Ping) to the client
                            if let Err(e) = client_write.write_all(data).await {
                                warn!("Failed to forward WebSocket frame to client: {}", e);
                                break;
                            }
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

/// Parse headers from a CONNECT request string.
///
/// A CONNECT request looks like:
/// ```text
/// CONNECT example.com:443 HTTP/1.1
/// Host: example.com:443
/// User-Agent: curl/7.88.1
/// Proxy-Connection: Keep-Alive
///
/// ```
///
/// This function extracts all headers (lines after the request line, up to
/// the first blank line) into a `HashMap<String, String>`. The header names
/// are preserved as-is (case preserved) for fidelity.
fn parse_connect_headers(request_str: &str) -> std::collections::HashMap<String, String> {
    let mut headers = std::collections::HashMap::new();
    for line in request_str.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            break; // End of headers
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() {
                headers.insert(key, value);
            }
        }
    }
    headers
}

/// Parse a `Proxy-Authorization` header value into [`ProxyCredentials`]
/// (Phase 9.6). Supports `Basic <base64>` and `Bearer <token>` schemes.
/// Returns `None` when the scheme is unrecognized or the value is
/// malformed.
fn parse_proxy_authorization(value: &str) -> Option<ProxyCredentials> {
    let (scheme, rest) = value.split_once(' ')?;
    let rest = rest.trim();
    match scheme.to_ascii_lowercase().as_str() {
        "basic" => {
            use base64::Engine;
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(rest)
                .ok()?;
            let decoded_str = String::from_utf8(decoded).ok()?;
            Some(ProxyCredentials::ProxyBasicAuth(decoded_str))
        }
        "bearer" => Some(ProxyCredentials::ProxyBearer(rest.to_string())),
        _ => None,
    }
}

/// WebSocket Ping opcode (RFC 6455).
const WS_OPCODE_PING: u8 = 0x9;

/// Extract the unmasked payloads of all complete WebSocket Ping frames
/// contained in `data`.
///
/// A single TCP read may coalesce multiple WebSocket frames or contain a
/// partial frame. This walks through as many *complete* frames as are
/// present and returns the decoded payloads of any Ping (opcode `0x9`)
/// frames. Trailing partial frames are ignored (they will be re-read on the
/// next iteration once more bytes arrive).
fn extract_ping_payloads(data: &[u8]) -> Vec<Vec<u8>> {
    let mut pings = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let remaining = &data[offset..];
        let (fin, opcode, payload_len, header_len) = match WsFrameParser::parse_header(remaining) {
            Some(h) => h,
            None => break, // Not enough bytes for a header yet.
        };
        let total = match (header_len as u64).checked_add(payload_len) {
            Some(t) => t as usize,
            None => break,
        };
        if remaining.len() < total {
            // Partial frame: wait for more data on the next read.
            break;
        }

        if opcode == WS_OPCODE_PING && fin {
            let second_byte = remaining[1];
            let masked = (second_byte & 0x80) != 0;
            let mask_len = if masked { 4 } else { 0 };
            let base_header_len = header_len - mask_len;
            let payload = &remaining[header_len..total];
            if masked {
                let mask = [
                    remaining[base_header_len],
                    remaining[base_header_len + 1],
                    remaining[base_header_len + 2],
                    remaining[base_header_len + 3],
                ];
                pings.push(WsFrameParser::decode_masked(payload, mask));
            } else {
                pings.push(payload.to_vec());
            }
        }

        offset += total;
    }
    pings
}

/// Build a WebSocket Pong frame (opcode `0x0A`) carrying `payload`.
///
/// Per RFC 6455, frames sent from a client to a server must be masked, while
/// frames sent from a server to a client must not be masked. The proxy acts
/// as a server towards the client and as a client towards the upstream
/// server, so:
///
/// - When sending a Pong to the client, pass `mask = false`.
/// - When sending a Pong to the server, pass `mask = true`.
fn build_pong_frame(payload: &[u8], mask: bool) -> Vec<u8> {
    use rand::Rng;

    let mut frame = vec![0x8A]; // FIN=1, opcode=0x0A (Pong)
    let mask_flag: u8 = if mask { 0x80 } else { 0x00 };
    let len = payload.len();
    if len < 126 {
        frame.push(mask_flag | len as u8);
    } else if len <= 65535 {
        frame.push(mask_flag | 126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(mask_flag | 127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    if mask {
        let masking_key: [u8; 4] = rand::rng().random::<u32>().to_be_bytes();
        frame.extend_from_slice(&masking_key);
        let masked_payload: Vec<u8> = payload
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ masking_key[i % 4])
            .collect();
        frame.extend_from_slice(&masked_payload);
    } else {
        frame.extend_from_slice(payload);
    }

    frame
}

// ─── HTTP/2 downstream support ──────────────────────────────────────────────

/// Process a single HTTP/2 stream accepted from the client.
///
/// Converts the `h2::RecvStream` request into a protocol-agnostic
/// [`RequestData`] (tagged `http_version = "HTTP/2"`), runs it through the
/// shared [`Pipeline`] (rewrites, mocks, breakpoints, upstream forwarding,
/// traffic recording), and writes the resulting response back to the client
/// over the same h2 stream.
///
/// The pipeline serializes responses as HTTP/1.1 bytes (its existing
/// contract); [`H2ResponseWriter`] buffers those bytes and [`H2ResponseWriter::finalize`]
/// translates them back into HTTP/2 frames. This adapter lets the entire
/// HTTP/1.1 interception pipeline be reused for HTTP/2 without duplication.
async fn process_h2_stream(
    request: http::Request<h2::RecvStream>,
    respond: &mut h2::server::SendResponse<Bytes>,
    fallback_host: &str,
    port: u16,
    pipeline: &Pipeline<'_>,
) -> crate::Result<()> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers_map = request.headers().clone();

    // Determine the target host: prefer the URI host, then the HTTP/2
    // `:authority` pseudo-header, then the CONNECT target host passed in.
    let host = uri
        .host()
        .map(|h| h.to_string())
        .or_else(|| {
            headers_map
                .get(":authority")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| fallback_host.to_string());

    // Path (including query string). HTTP/2 always sends `:path`.
    let path = uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());

    // Copy regular headers, skipping HTTP/2 pseudo-headers (names starting
    // with `:`). Pseudo-headers have no place in the stored `RequestData`
    // and would confuse the upstream reqwest client.
    let mut headers = std::collections::HashMap::new();
    let mut content_type = None;
    for (name, value) in &headers_map {
        let name_str = name.as_str();
        if name_str.starts_with(':') {
            continue;
        }
        let value_str = value.to_str().unwrap_or("");
        if name_str.eq_ignore_ascii_case("content-type") {
            content_type = Some(value_str.to_string());
        }
        headers.insert(name_str.to_string(), value_str.to_string());
    }

    // Read the full request body from the h2 stream, releasing flow-control
    // capacity as we consume data so the client's window stays open for
    // large uploads (otherwise the stream deadlocks).
    let mut body_stream = request.into_body();
    let mut body = Vec::new();
    while let Some(chunk_result) = body_stream.data().await {
        match chunk_result {
            Ok(chunk) => {
                let len = chunk.len();
                body.extend_from_slice(&chunk);
                let _ = body_stream.flow_control().release_capacity(len);
            }
            Err(e) => {
                warn!("h2 recv data error for {}:{}: {}", host, port, e);
                break;
            }
        }
    }

    let url = format!("https://{}{}", host, path);

    let mut request_data = RequestData {
        method: method.as_str().into(),
        url,
        host,
        path,
        headers,
        body: if body.is_empty() { None } else { Some(body) },
        content_type,
        http_version: Some("HTTP/2".to_string()),
    };

    // Run the request through the shared pipeline. The pipeline writes the
    // HTTP/1.1-serialized response into the H2ResponseWriter buffer; we then
    // translate it back into h2 frames in `finalize`.
    let mut writer = H2ResponseWriter::new();
    let _outcome = pipeline
        .process_request(&mut request_data, &mut writer)
        .await?;

    writer.finalize(respond).await
}

/// An [`tokio::io::AsyncWrite`] adapter that buffers the HTTP/1.1 response
/// bytes produced by the pipeline and later translates them into HTTP/2
/// frames via [`H2ResponseWriter::finalize`].
///
/// The pipeline's existing response path serializes responses as HTTP/1.1
/// (`build_response_bytes`) and writes them via `AsyncWrite`. By buffering
/// those bytes here and re-parsing them, the entire HTTP/1.1 interception
/// pipeline (mocks, breakpoints, rewrites, upstream forwarding) is reused
/// for HTTP/2 streams without any duplication of that logic.
struct H2ResponseWriter {
    buffer: Vec<u8>,
}

impl H2ResponseWriter {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Translate the buffered HTTP/1.1 response into HTTP/2 frames and send
    /// them to the client over the h2 stream.
    async fn finalize(self, respond: &mut h2::server::SendResponse<Bytes>) -> crate::Result<()> {
        // If the pipeline wrote nothing (e.g. upstream forwarding failed
        // without producing a client-facing response — see the error branch
        // of `process_request`), send a 502 so the h2 stream is closed
        // cleanly rather than left hanging.
        if self.buffer.is_empty() {
            let resp = http::Response::builder()
                .status(502)
                .header("content-type", "text/plain")
                .body(())
                .map_err(|e| Error::Proxy(format!("h2 response build: {}", e)))?;
            let mut stream = respond
                .send_response(resp, false)
                .map_err(|e| Error::Proxy(format!("h2 send_response: {}", e)))?;
            stream
                .send_data(
                    Bytes::from_static(b"Bad Gateway (upstream forwarding failed)"),
                    true,
                )
                .map_err(|e| Error::Proxy(format!("h2 send_data: {}", e)))?;
            return Ok(());
        }

        let (status, headers, body) = parse_http1_response(&self.buffer);
        let mut builder = http::Response::builder().status(status);
        for (name, value) in &headers {
            let lower = name.to_lowercase();
            // Strip hop-by-hop and length headers — HTTP/2 frames carry their
            // own length and forbid connection-specific headers
            // (RFC 7540 §8.1.2.2).
            if matches!(
                lower.as_str(),
                "content-length" | "transfer-encoding" | "connection" | "keep-alive"
            ) {
                continue;
            }
            if let (Ok(n), Ok(v)) = (
                http::HeaderName::from_bytes(name.as_bytes()),
                http::HeaderValue::from_str(value),
            ) {
                builder = builder.header(n, v);
            }
        }

        let body_bytes = body.unwrap_or_default();
        let end_stream = body_bytes.is_empty();
        let resp = builder
            .body(())
            .map_err(|e| Error::Proxy(format!("h2 response build: {}", e)))?;
        let mut stream = respond
            .send_response(resp, end_stream)
            .map_err(|e| Error::Proxy(format!("h2 send_response: {}", e)))?;
        if !body_bytes.is_empty() {
            stream
                .send_data(Bytes::from(body_bytes), true)
                .map_err(|e| Error::Proxy(format!("h2 send_data: {}", e)))?;
        }
        Ok(())
    }
}

impl tokio::io::AsyncWrite for H2ResponseWriter {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.get_mut().buffer.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

/// Parse a minimal HTTP/1.1 response (as produced by the pipeline's
/// `build_response_bytes`) into `(status_code, headers, body)`.
///
/// This is the inverse of `Pipeline::build_response_bytes` and is used to
/// recover the structured response from the bytes buffered in
/// [`H2ResponseWriter`] so it can be re-encoded as HTTP/2 frames.
fn parse_http1_response(buf: &[u8]) -> (u16, Vec<(String, String)>, Option<Vec<u8>>) {
    let header_end = buf.windows(4).position(|w| w == b"\r\n\r\n");
    let (head, body) = match header_end {
        Some(p) => (&buf[..p], Some(&buf[p + 4..])),
        None => (buf, None),
    };
    let head_str = String::from_utf8_lossy(head);
    let mut lines = head_str.lines();
    let status_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    let status: u16 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(502);

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    (status, headers, body.map(|b| b.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http1_response_basic() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello";
        let (status, headers, body) = parse_http1_response(raw);
        assert_eq!(status, 200);
        assert_eq!(headers.len(), 2);
        assert_eq!(
            headers.iter().find(|(k, _)| k == "Content-Type"),
            Some(&("Content-Type".to_string(), "text/plain".to_string()))
        );
        assert_eq!(body, Some(b"hello".to_vec()));
    }

    #[test]
    fn test_parse_http1_response_no_body() {
        let raw = b"HTTP/1.1 204 No Content\r\n\r\n";
        let (status, headers, body) = parse_http1_response(raw);
        assert_eq!(status, 204);
        assert!(headers.is_empty());
        assert_eq!(body, Some(Vec::new()));
    }

    #[test]
    fn test_parse_http1_response_error_status() {
        let raw = b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\n\r\nBad Gateway";
        let (status, _, body) = parse_http1_response(raw);
        assert_eq!(status, 502);
        assert_eq!(body, Some(b"Bad Gateway".to_vec()));
    }

    #[test]
    fn test_parse_http1_response_no_header_terminator() {
        // No \r\n\r\n — entire buffer is treated as head, body is None.
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain";
        let (status, headers, body) = parse_http1_response(raw);
        assert_eq!(status, 200);
        assert_eq!(headers.len(), 1);
        assert_eq!(body, None);
    }

    #[test]
    fn test_parse_http1_response_empty_buffer() {
        let (status, headers, body) = parse_http1_response(b"");
        assert_eq!(status, 502); // Default fallback
        assert!(headers.is_empty());
        assert_eq!(body, None);
    }

    #[test]
    fn test_h2_response_writer_buffers_writes() {
        use tokio::io::AsyncWriteExt;

        let mut writer = H2ResponseWriter::new();
        // Simulate the pipeline writing an HTTP/1.1 response.
        let data = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true}";
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            writer.write_all(data).await.unwrap();
        });

        // The buffer should contain exactly what was written.
        assert_eq!(writer.buffer, data);
    }
}

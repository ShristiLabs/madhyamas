//! Madhyamas API - REST and WebSocket API for the web UI

pub mod auth;
pub mod embedded_assets;
#[cfg(feature = "enterprise")]
pub mod enterprise_handlers;
pub mod error;
pub mod handlers;
pub mod intercept_handlers;
#[cfg(feature = "enterprise")]
pub mod middleware;
pub mod routes;
#[cfg(any(feature = "grpc", feature = "scripting", feature = "plugins"))]
pub mod tools_handlers;
pub mod validation;
pub mod ws;

pub use auth::{
    AuditError, AuditEvent, AuditEventType, AuditFilter, AuditSink, AuthError, AuthMethod,
    AuthProvider, Authorizer, Identity, Permission, ResourceType,
};

use axum::Router;
#[cfg(feature = "enterprise")]
use madhyamas_core::enterprise::AuthManager;
#[cfg(feature = "plugins")]
use madhyamas_core::plugin::PluginRegistry;
#[cfg(feature = "grpc")]
use madhyamas_core::GrpcManager;
#[cfg(feature = "plugins")]
use madhyamas_core::PluginManager;
#[cfg(feature = "scripting")]
use madhyamas_core::ScriptRuntime;
use madhyamas_core::{
    AutoSaveManager, BlockListManager, BreakpointManager, CertificateManager, InterceptStore,
    LogHandle, MirrorWriter, MockManager, ProxyConfig, ReplayManager, RewriteManager,
    SessionManager, ThrottleManager, TrafficStore, WsManager,
};
use parking_lot::RwLock;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// Returns true if the given Origin header value points at a localhost or
/// private-network address. This prevents arbitrary websites from reading
/// captured traffic (cookies, auth headers) via the local API.
fn is_safe_origin(value: &axum::http::HeaderValue) -> bool {
    let s = match value.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    // Parse origin URL: scheme://host[:port]
    if let Ok(url) = url::Url::parse(s) {
        match url.host() {
            Some(url::Host::Ipv4(ip)) => {
                // localhost, loopback, private ranges (10/172.16/192.168)
                ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified()
            }
            Some(url::Host::Ipv6(ip)) => {
                ip.is_loopback() || ip.is_unspecified() || ip.is_unicast_link_local()
            }
            Some(url::Host::Domain(d)) => d == "localhost" || d.ends_with(".localhost"),
            None => false,
        }
    } else {
        false
    }
}

/// API state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub traffic_store: Arc<TrafficStore>,
    pub cert_manager: Option<Arc<CertificateManager>>,
    pub breakpoint_manager: Arc<BreakpointManager>,
    pub mock_manager: Arc<MockManager>,
    pub rewrite_manager: Arc<RewriteManager>,
    pub throttle_manager: Arc<ThrottleManager>,
    pub block_list_manager: Arc<BlockListManager>,
    pub replay_manager: Arc<ReplayManager>,
    #[cfg(feature = "grpc")]
    pub grpc_manager: Arc<GrpcManager>,
    #[cfg(feature = "scripting")]
    pub script_runtime: Arc<ScriptRuntime>,
    #[cfg(feature = "plugins")]
    pub plugin_manager: Arc<PluginManager>,
    #[cfg(feature = "plugins")]
    pub plugin_registry: Arc<tokio::sync::Mutex<PluginRegistry>>,
    pub ws_manager: Arc<WsManager>,
    pub session_manager: Arc<SessionManager>,
    pub intercept_store: Option<Arc<InterceptStore>>,
    pub proxy_config: Option<Arc<RwLock<ProxyConfig>>>,
    /// Auto Save manager (periodic session backup). Optional — only set
    /// when the proxy engine is running with Auto Save enabled.
    pub autosave_manager: Option<Arc<AutoSaveManager>>,
    /// Mirror writer (saves response bodies to disk). Optional — only set
    /// when the proxy engine is running with mirroring enabled.
    pub mirror_writer: Option<Arc<MirrorWriter>>,
    /// Log rotation handle. Optional — only set when the proxy server is
    /// running (not in CLI/MCP modes). Enables `GET /api/logs`,
    /// `POST /api/logs/rotate`, and `PATCH /api/logs`.
    pub log_handle: Option<Arc<LogHandle>>,
    /// Optional enterprise auth manager. When present and enterprise
    /// features are enabled, JWT authentication is enforced on protected
    /// routes.
    #[cfg(feature = "enterprise")]
    pub auth_service: Option<Arc<AuthManager>>,
    /// Pluggable authentication provider (trait object). `None` in the
    /// simple/OSS tier; `Some` in the enterprise tier once the enterprise
    /// crate injects its `AuthManager`-backed implementation (Phase 1b).
    pub auth_provider: Option<Arc<dyn AuthProvider + Send + Sync>>,
    /// Pluggable authorization checker (trait object). `None` in the
    /// simple/OSS tier (allow-all); `Some` in the enterprise tier once the
    /// enterprise crate injects its `RbacManager`-backed implementation
    /// (Phase 1b).
    pub authorizer: Option<Arc<dyn Authorizer + Send + Sync>>,
    /// Pluggable audit sink (trait object). `None` in the simple/OSS tier
    /// (audit events dropped); `Some` in the enterprise tier once the
    /// enterprise crate injects its `AuditLogger`-backed implementation
    /// (Phase 1b).
    pub audit_sink: Option<Arc<dyn AuditSink + Send + Sync>>,
}

impl AppState {
    pub fn new(traffic_store: Arc<TrafficStore>) -> Self {
        let session_manager = Arc::new(SessionManager::new(traffic_store.clone()));
        Self {
            traffic_store,
            cert_manager: None,
            breakpoint_manager: Arc::new(BreakpointManager::default()),
            mock_manager: Arc::new(MockManager::default()),
            rewrite_manager: Arc::new(RewriteManager::default()),
            throttle_manager: Arc::new(ThrottleManager::default()),
            block_list_manager: Arc::new(BlockListManager::default()),
            replay_manager: Arc::new(ReplayManager::default()),
            #[cfg(feature = "grpc")]
            grpc_manager: Arc::new(GrpcManager::default()),
            #[cfg(feature = "scripting")]
            script_runtime: Arc::new(ScriptRuntime::default()),
            #[cfg(feature = "plugins")]
            plugin_manager: Arc::new(PluginManager::default()),
            #[cfg(feature = "plugins")]
            plugin_registry: Arc::new(tokio::sync::Mutex::new(PluginRegistry::new())),
            ws_manager: Arc::new(WsManager::new()),
            session_manager,
            intercept_store: None,
            proxy_config: None,
            autosave_manager: None,
            mirror_writer: None,
            log_handle: None,
            #[cfg(feature = "enterprise")]
            auth_service: None,
            auth_provider: None,
            authorizer: None,
            audit_sink: None,
        }
    }

    pub fn with_cert_manager(mut self, cert_manager: Arc<CertificateManager>) -> Self {
        self.cert_manager = Some(cert_manager);
        self
    }

    pub fn with_breakpoint_manager(mut self, manager: Arc<BreakpointManager>) -> Self {
        self.breakpoint_manager = manager;
        self
    }

    pub fn with_mock_manager(mut self, manager: Arc<MockManager>) -> Self {
        self.mock_manager = manager;
        self
    }

    pub fn with_rewrite_manager(mut self, manager: Arc<RewriteManager>) -> Self {
        self.rewrite_manager = manager;
        self
    }

    pub fn with_throttle_manager(mut self, manager: Arc<ThrottleManager>) -> Self {
        self.throttle_manager = manager;
        self
    }

    pub fn with_block_list_manager(mut self, manager: Arc<BlockListManager>) -> Self {
        self.block_list_manager = manager;
        self
    }

    pub fn with_replay_manager(mut self, manager: Arc<ReplayManager>) -> Self {
        self.replay_manager = manager;
        self
    }

    #[cfg(feature = "grpc")]
    pub fn with_grpc_manager(mut self, manager: Arc<GrpcManager>) -> Self {
        self.grpc_manager = manager;
        self
    }

    #[cfg(feature = "scripting")]
    pub fn with_script_runtime(mut self, runtime: Arc<ScriptRuntime>) -> Self {
        self.script_runtime = runtime;
        self
    }

    #[cfg(feature = "plugins")]
    pub fn with_plugin_manager(mut self, manager: Arc<PluginManager>) -> Self {
        self.plugin_manager = manager;
        self
    }

    pub fn with_ws_manager(mut self, manager: Arc<WsManager>) -> Self {
        self.ws_manager = manager;
        self
    }

    pub fn with_session_manager(mut self, manager: Arc<SessionManager>) -> Self {
        self.session_manager = manager;
        self
    }

    pub fn with_intercept_store(mut self, store: Arc<InterceptStore>) -> Self {
        self.intercept_store = Some(store);
        self
    }

    pub fn with_proxy_config(mut self, config: Arc<RwLock<ProxyConfig>>) -> Self {
        self.proxy_config = Some(config);
        self
    }

    /// Attach the Auto Save manager so the API layer can query/update the
    /// live Auto Save configuration and trigger manual snapshots.
    pub fn with_autosave_manager(mut self, manager: Arc<AutoSaveManager>) -> Self {
        self.autosave_manager = Some(manager);
        self
    }

    /// Attach the mirror writer so the API layer can query/update the live
    /// mirror configuration and statistics.
    pub fn with_mirror_writer(mut self, writer: Arc<MirrorWriter>) -> Self {
        self.mirror_writer = Some(writer);
        self
    }

    /// Attach the log rotation handle so the API layer can query log status,
    /// trigger on-demand rotation, and update the rotation config at runtime.
    pub fn with_log_handle(mut self, handle: LogHandle) -> Self {
        self.log_handle = Some(Arc::new(handle));
        self
    }

    /// Attach an enterprise auth manager. When set, enterprise routes
    /// enforce JWT authentication (see [`middleware::auth_middleware`]).
    #[cfg(feature = "enterprise")]
    pub fn with_auth_service(mut self, auth_service: Arc<AuthManager>) -> Self {
        self.auth_service = Some(auth_service);
        self
    }

    /// Attach a pluggable authentication provider. When set, the API layer
    /// can validate JWT/API-key credentials via the [`AuthProvider`] trait
    /// without depending on enterprise concrete types.
    pub fn with_auth_provider(mut self, provider: Arc<dyn AuthProvider + Send + Sync>) -> Self {
        self.auth_provider = Some(provider);
        self
    }

    /// Attach a pluggable authorization checker. When set, the API layer
    /// can enforce RBAC via the [`Authorizer`] trait without depending on
    /// enterprise concrete types. When unset, authorization is allow-all.
    pub fn with_authorizer(mut self, authorizer: Arc<dyn Authorizer + Send + Sync>) -> Self {
        self.authorizer = Some(authorizer);
        self
    }

    /// Attach a pluggable audit sink. When set, the API layer can record
    /// and query audit events via the [`AuditSink`] trait without depending
    /// on enterprise concrete types. When unset, audit events are dropped.
    pub fn with_audit_sink(mut self, sink: Arc<dyn AuditSink + Send + Sync>) -> Self {
        self.audit_sink = Some(sink);
        self
    }
}

/// Rate-limiting configuration for the API server.
///
/// Rate limiting is **disabled by default**. Madhyamas is a local debugging
/// tool, and the web UI's TanStack Query fires many parallel requests on
/// page load, which can easily exhaust a low burst budget. Enable rate
/// limiting only when the API is exposed to a less-trusted network.
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Enable or disable rate limiting.
    pub enabled: bool,
    /// Maximum requests per second per peer IP.
    pub requests_per_second: u32,
    /// Maximum burst size (tokens that can accumulate when idle).
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: 600,
            burst_size: 1000,
        }
    }
}

impl RateLimitConfig {
    /// Create a disabled config.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Create an enabled config with the given parameters.
    pub fn enabled(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            enabled: true,
            requests_per_second,
            burst_size,
        }
    }
}

/// Create the API router.
///
/// `rate_limit` controls whether the [`tower_governor`] rate-limiting layer
/// is applied. When [`RateLimitConfig::enabled`] is `false` (the default),
/// no rate limiting is applied.
pub fn create_router(state: AppState, rate_limit: RateLimitConfig) -> Router<()> {
    let state = Arc::new(state);

    let mut router = Router::new()
        // Top-level health check for quick status
        .route("/health", axum::routing::get(|| async { "OK" }))
        .nest("/api", routes::create_routes())
        // Serve embedded web assets (compiled into the binary via rust-embed).
        // Falls back to disk-based serving via MADHYAMAS_WEB_DIR for dev.
        .fallback(embedded_assets::embedded_fallback)
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin, _| is_safe_origin(origin)))
                .allow_methods(Any)
                .allow_headers(Any),
        )
        // Security headers
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::X_FRAME_OPTIONS,
            axum::http::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::REFERRER_POLICY,
            axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        // Limit request bodies to 10MB to prevent OOM from large payloads
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024));

    // Rate limiting is opt-in. When enabled, apply the governor layer with
    // the configured requests-per-second and burst size.
    if rate_limit.enabled {
        tracing::info!(
            "API rate limiting enabled: {} req/s, burst {}",
            rate_limit.requests_per_second,
            rate_limit.burst_size
        );
        router = router.layer(tower_governor::GovernorLayer::new(
            tower_governor::governor::GovernorConfigBuilder::default()
                .const_per_second(rate_limit.requests_per_second as u64)
                .burst_size(rate_limit.burst_size)
                .finish()
                .unwrap(),
        ));
    } else {
        tracing::debug!("API rate limiting disabled (default)");
    }

    // Log only method, URI, and status — exclude headers and body to
    // avoid leaking sensitive data (cookies, auth headers) into logs.
    router = router.layer(TraceLayer::new_for_http().make_span_with(
        |request: &axum::http::Request<_>| {
            tracing::info_span!(
                "api",
                method = %request.method(),
                uri = %request.uri(),
            )
        },
    ));

    router.with_state(state)
}

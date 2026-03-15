//! ProxyForge API - REST and WebSocket API for the web UI

pub mod handlers;
pub mod intercept_handlers;
pub mod phase3_handlers;
pub mod phase4_handlers;
pub mod routes;
pub mod ws;

use axum::Router;
use proxyforge_core::{
    BreakpointManager, CertificateManager, GrpcManager, InterceptStore, MockManager, PluginManager,
    ProxyConfig, ReplayManager, RewriteManager, ScriptRuntime, SessionManager, ThrottleManager,
    TrafficStore, WsManager,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

/// API state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub traffic_store: Arc<TrafficStore>,
    pub cert_manager: Option<Arc<CertificateManager>>,
    pub breakpoint_manager: Arc<BreakpointManager>,
    pub mock_manager: Arc<MockManager>,
    pub rewrite_manager: Arc<RewriteManager>,
    pub throttle_manager: Arc<ThrottleManager>,
    pub replay_manager: Arc<ReplayManager>,
    pub grpc_manager: Arc<GrpcManager>,
    pub script_runtime: Arc<ScriptRuntime>,
    pub plugin_manager: Arc<PluginManager>,
    pub ws_manager: Arc<WsManager>,
    pub session_manager: Arc<SessionManager>,
    pub intercept_store: Option<Arc<InterceptStore>>,
    pub proxy_config: Option<Arc<ProxyConfig>>,
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
            replay_manager: Arc::new(ReplayManager::default()),
            grpc_manager: Arc::new(GrpcManager::default()),
            script_runtime: Arc::new(ScriptRuntime::default()),
            plugin_manager: Arc::new(PluginManager::default()),
            ws_manager: Arc::new(WsManager::new()),
            session_manager,
            intercept_store: None,
            proxy_config: None,
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

    pub fn with_replay_manager(mut self, manager: Arc<ReplayManager>) -> Self {
        self.replay_manager = manager;
        self
    }

    pub fn with_grpc_manager(mut self, manager: Arc<GrpcManager>) -> Self {
        self.grpc_manager = manager;
        self
    }

    pub fn with_script_runtime(mut self, runtime: Arc<ScriptRuntime>) -> Self {
        self.script_runtime = runtime;
        self
    }

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

    pub fn with_proxy_config(mut self, config: Arc<ProxyConfig>) -> Self {
        self.proxy_config = Some(config);
        self
    }
}

/// Create the API router
pub fn create_router(state: AppState) -> Router<()> {
    let state = Arc::new(state);

    Router::new()
        // Top-level health check for quick status
        .route("/health", axum::routing::get(|| async { "OK" }))
        .nest("/api", routes::create_routes())
        // Serve static files from web/dist
        .fallback_service(ServeDir::new("web/dist").fallback(ServeDir::new("web/dist/index.html")))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

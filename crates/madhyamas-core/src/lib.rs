//! Madhyamas Core - HTTP/HTTPS debugging proxy engine

pub mod config;
pub mod enterprise;
pub mod grpc;
pub mod intercept;
pub mod performance;
pub mod persistence;
pub mod plugin;
pub mod proxy;
pub mod replay;
pub mod scripting;
pub mod session;
pub mod tls;
pub mod traffic;
pub mod websocket;

// Re-exports from config
pub use config::ProxyConfig;

// Re-exports from traffic
pub use traffic::{HttpMethod, RequestData, ResponseData, Session, TrafficEntry, TrafficFilter, TrafficStore};

// Re-exports from tls
pub use tls::CertificateManager;

// Re-exports from proxy
pub use proxy::ProxyEngine;

// Re-exports from websocket
pub use websocket::{WsConnection, WsDirection, WsFilter, WsManager, WsMessage, WsMessageType};

// Re-exports from intercept
pub use intercept::{
    BreakpointAction, BreakpointDecision, BreakpointManager, BreakpointRule,
    InterceptDecision, InterceptDirection, MatchCondition, MockManager, MockResponse,
    MockRule, MockTemplates, PausedTraffic, RewriteAction, RewriteDirection,
    RewriteManager, RewriteRule, RewriteTemplates, ThrottleManager, ThrottleProfile,
};

// Re-exports from persistence
pub use persistence::InterceptStore;

// Re-exports from grpc
pub use grpc::{GrpcConnection, GrpcDirection, GrpcFilter, GrpcFrame, GrpcManager, GrpcStream};

// Re-exports from scripting
pub use scripting::{Script, ScriptConfig, ScriptRuntime, ScriptTemplates};

// Re-exports from session
pub use session::{SessionExport, SessionManager, SessionMetadata, SessionPreset, SessionSummary};

// Re-exports from replay
pub use replay::{ReplayManager, ReplayResult, RequestModifications, SavedRequest};

// Re-exports from plugin
pub use plugin::PluginManager;

/// Result type
pub type Result<T> = std::result::Result<T, Error>;

/// Error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TLS error: {0}")]
    Tls(String),
    #[error("Certificate error: {0}")]
    Certificate(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Proxy error: {0}")]
    Proxy(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Channel error: {0}")]
    Channel(String),
}

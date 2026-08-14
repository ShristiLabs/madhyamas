//! Madhyamas Core - HTTP/HTTPS debugging proxy engine

pub mod access_control;
pub mod auto_save;
pub mod config;
pub mod error;
pub mod extension;
#[cfg(feature = "grpc")]
pub mod grpc;
pub mod intercept;
pub mod log_rotation;
pub mod mirror;
pub mod performance;
pub mod persistence;
#[cfg(feature = "plugins")]
pub mod plugin;
pub mod proxy;
pub mod replay;
#[cfg(feature = "scripting")]
pub mod scripting;
pub mod session;
pub mod storage;
pub mod tls;
pub mod traffic;
pub mod websocket;

// Re-exports from access_control
pub use access_control::AccessControlList;

// Re-exports from auto_save
pub use auto_save::AutoSaveManager;

// Re-exports from config
pub use config::{
    AutoSaveConfig, LogConfig, LogRotation, MirrorConfig, ProxyConfig, UpstreamProxyConfig,
};

// Re-exports from log_rotation
pub use log_rotation::{ArchivedLog, LogHandle, RotatingFileWriter};

// Re-exports from mirror
pub use mirror::{MirrorStats, MirrorWriter};

// Re-exports from traffic
pub use traffic::{
    create_traffic_event_channel, CaptureStats, FocusHost, HttpMethod, ImportResult, RequestData,
    ResponseData, Session, TrafficEntry, TrafficEntrySnapshot, TrafficEvent, TrafficFilter,
    TrafficStore, TrafficSubscriptionFilter, WsClientMessage, WsServerMessage,
    TRAFFIC_EVENT_CHANNEL_CAPACITY,
};

// Re-exports from tls
pub use tls::CertificateManager;

// Re-exports from proxy
pub use proxy::ProxyEngine;

// Re-exports from websocket
pub use websocket::{
    WsConnection, WsDirection, WsFilter, WsFragmentReassembler, WsManager, WsMessage, WsMessageType,
};

// Re-exports from intercept
pub use intercept::{
    matches_pattern as block_list_matches_pattern, BlockListEntry, BlockListManager,
    BlockListStats, BreakpointAction, BreakpointDecision, BreakpointManager, BreakpointRule,
    ConditionalResponse, InterceptAction, InterceptDecision, InterceptDirection, InterceptHandler,
    MatchCondition, MockCollection, MockExpiration, MockHitRecord, MockHitStats, MockManager,
    MockPreviewResult, MockResponse, MockRule, MockRuleVersion, MockTemplates, MockTestResult,
    PausedTraffic, ProbabilisticResponse, RequestCondition, ResponseConfig, RewriteAction,
    RewriteDirection, RewriteManager, RewriteRule, RewriteTemplates, ThrottleManager,
    ThrottleProfile,
};

// Re-exports from persistence
pub use persistence::Persistable;

// Re-exports from extension
#[cfg(feature = "plugins")]
pub use extension::PluginExtension;
#[cfg(feature = "scripting")]
pub use extension::ScriptExtension;
pub use extension::{
    Extension, ExtensionContext, ExtensionManager, ExtensionRequest, ExtensionResponse,
    ExtensionResult,
};

// Re-exports from grpc
#[cfg(feature = "grpc")]
pub use grpc::{GrpcConnection, GrpcDirection, GrpcFilter, GrpcFrame, GrpcManager, GrpcStream};

// Re-exports from scripting
#[cfg(feature = "scripting")]
pub use scripting::{
    Script, ScriptConfig, ScriptErrorPolicy, ScriptMatch, ScriptRuntime, ScriptTemplates,
    UpdateScriptFields,
};

// Re-exports from session
pub use session::{SessionExport, SessionManager, SessionMetadata, SessionPreset, SessionSummary};

// Re-exports from replay
pub use replay::{
    ReplayBatchConfig, ReplayBatchResult, ReplayManager, ReplayResult, RequestModifications,
    SavedRequest,
};

// Re-exports from plugin
#[cfg(all(feature = "plugins", feature = "wasm-runtime"))]
pub use plugin::HotReloader;
#[cfg(all(feature = "plugins", feature = "wasm-runtime"))]
pub use plugin::WasmRuntime;
#[cfg(feature = "plugins")]
pub use plugin::{
    bytes_to_hex, generate_keypair, hex_to_bytes, sign_package, verify_package, PluginCapability,
    PluginError, PluginEventBus, PluginInstaller, PluginKeypair, PluginManager, PluginManifest,
    PluginPersistence, PluginRegistry, PluginSettingField, PluginSettingType, PluginSettingsSchema,
    PluginState, PluginStats, PluginTemplate, PluginTemplates, TemplateId,
};

// Re-exports from performance
pub use performance::{
    Alert, AlertConfig, AlertLevel, GarbageCollectionConfig, HealthCheck, HealthStatus,
    MemoryManager, MemoryPressure, MemoryStats, Metrics, MetricsCollector, PerformanceMonitor,
    PerformanceStats,
};

// Re-exports from storage (Phase 2b async backend traits)
#[cfg(feature = "plugins")]
pub use storage::PluginStoreBackend;
#[cfg(feature = "scripting")]
pub use storage::ScriptStoreBackend;
pub use storage::{
    ConfigStoreBackend, InterceptStoreBackend, SqliteConfigStore, SqliteInterceptStore,
    TrafficStoreBackend,
};

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
    #[error("SQLx database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Proxy error: {0}")]
    Proxy(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Channel error: {0}")]
    Channel(String),
}

//! Plugin types and definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin manifest (madhyamas-plugin.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin ID (unique identifier)
    pub id: String,
    /// Plugin name
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Plugin description
    pub description: Option<String>,
    /// Author name
    pub author: Option<String>,
    /// Plugin homepage URL
    pub homepage: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Minimum Madhyamas version required
    pub min_version: Option<String>,
    /// Maximum Madhyamas version supported
    pub max_version: Option<String>,
    /// Plugin license
    pub license: Option<String>,
    /// Plugin dependencies (other plugins)
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// Hooks the plugin subscribes to
    #[serde(default)]
    pub hooks: Vec<String>,
    /// Plugin settings schema
    pub settings: Option<PluginSettingsSchema>,
    /// Whether the plugin is enabled by default
    #[serde(default)]
    pub enabled_by_default: bool,
    /// Capability flags declared by the plugin. The host enforces these at
    /// link time (e.g. only `Network` plugins get the `http_fetch` host
    /// function). Defaults to an empty list.
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
    /// Whether the plugin requires network access (host `http_fetch`).
    /// Convenience shortcut for the `Network` capability. Defaults to false.
    #[serde(default)]
    pub network: bool,
    /// Maximum WASM linear memory pages (1 page = 64 KiB). Defaults to 64
    /// (4 MiB). Only honored when the `wasm-runtime` feature is enabled.
    #[serde(default = "default_memory_pages")]
    pub max_memory_pages: u32,
    /// WASM fuel limit (approximate instruction budget) per hook invocation.
    /// Defaults to 10_000_000. Only honored with `wasm-runtime`.
    #[serde(default = "default_fuel_limit")]
    pub fuel_limit: u64,
    /// Interval in seconds for the `on_timer` hook. `None` (default) means
    /// the timer hook is not scheduled.
    #[serde(default)]
    pub timer_interval_seconds: Option<u64>,
    /// Optional Ed25519 public key (hex) of the trusted publisher. When
    /// present, installed plugin packages must be signed with the
    /// corresponding private key. See `docs/PLUGIN_SECURITY.md`.
    #[serde(default)]
    pub publisher_public_key: Option<String>,
    /// Optional list of tags used by the registry for search/discovery.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Environment variable names this plugin may receive via `${ENV:VAR}`
    /// substitution in its settings. Deny by default: an empty list means no
    /// env placeholders are expanded. Never grants raw process-environment
    /// access. See `docs/PLUGIN_SECURITY.md`.
    #[serde(default)]
    pub env_grants: Vec<String>,
    /// Secret names this plugin may receive via `${SECRET:name}`
    /// substitution in its settings. Deny by default. Each substitution is
    /// audit-logged in the enterprise tier.
    #[serde(default)]
    pub secret_grants: Vec<String>,
    /// Optional declarative UI panels. Each panel is rendered in the web UI's
    /// plugin detail view. Panels can display static content, settings forms,
    /// live data, or custom widgets.
    #[serde(default)]
    pub panels: Vec<PluginPanel>,
}

fn default_memory_pages() -> u32 {
    64
}

fn default_fuel_limit() -> u64 {
    10_000_000
}

/// Plugin settings schema for UI generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSettingsSchema {
    pub fields: Vec<PluginSettingField>,
}

/// A single plugin setting field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSettingField {
    pub key: String,
    pub label: String,
    pub description: Option<String>,
    pub field_type: PluginSettingType,
    pub default: Option<serde_json::Value>,
    pub required: bool,
    pub options: Option<Vec<String>>, // For select/radio types
}

/// Plugin setting field types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginSettingType {
    String,
    Number,
    Boolean,
    Select,
    MultiSelect,
    Color,
    Url,
    Path,
    Json,
}

/// A declarative UI panel that a plugin can render in the web UI.
///
/// Panels are defined in the plugin manifest and rendered by the frontend
/// based on the `kind` field. This allows plugins to provide custom UI
/// without shipping JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPanel {
    /// Panel identifier (unique within the plugin).
    pub id: String,
    /// Display title shown in the panel header.
    pub title: String,
    /// Panel kind — determines how the frontend renders it.
    pub kind: PluginPanelKind,
    /// Panel contents (depends on `kind`).
    #[serde(default)]
    pub content: PluginPanelContent,
    /// Optional icon name (Lucide icon identifier, e.g. "shield", "globe").
    #[serde(default)]
    pub icon: Option<String>,
    /// Display order (lower = first). Defaults to 0.
    #[serde(default)]
    pub order: i32,
}

/// The kind of a plugin panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPanelKind {
    /// Static markdown content (rendered as read-only documentation).
    Markdown,
    /// A settings form generated from the plugin's settings schema.
    Settings,
    /// A live data table showing invocation logs.
    Logs,
    /// A custom HTML/JS widget (rendered in a sandboxed iframe).
    Widget,
    /// A stats/metrics dashboard (shows plugin statistics).
    Stats,
}

/// The content of a plugin panel, depending on its kind.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPanelContent {
    /// Markdown text (for `Markdown` kind).
    #[serde(default)]
    pub markdown: Option<String>,
    /// Widget HTML (for `Widget` kind). Rendered in a sandboxed iframe.
    #[serde(default)]
    pub html: Option<String>,
    /// Widget JavaScript (for `Widget` kind). Injected into the iframe.
    #[serde(default)]
    pub script: Option<String>,
    /// Additional data passed to the panel (key-value).
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

/// Loaded plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Current state
    pub state: PluginState,
    /// Plugin settings (user-configured)
    pub settings: HashMap<String, serde_json::Value>,
    /// Path to plugin directory
    pub path: String,
    /// When the plugin was loaded
    pub loaded_at: DateTime<Utc>,
    /// Error message if state is Error
    pub error: Option<String>,
}

impl Plugin {
    pub fn from_manifest(manifest: PluginManifest, path: &str) -> Self {
        Self {
            state: if manifest.enabled_by_default {
                PluginState::Enabled
            } else {
                PluginState::Loaded
            },
            settings: HashMap::new(),
            path: path.to_string(),
            loaded_at: Utc::now(),
            error: None,
            manifest,
        }
    }

    /// Check if plugin is enabled
    pub fn is_enabled(&self) -> bool {
        matches!(self.state, PluginState::Enabled | PluginState::Running)
    }
}

/// Plugin state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// Plugin is loaded but not initialized
    Loaded,
    /// Plugin is enabled and ready
    Enabled,
    /// Plugin is currently running (has active operations)
    Running,
    /// Plugin is disabled
    Disabled,
    /// Plugin encountered an error
    Error,
    /// Plugin is being unloaded
    Unloading,
}

/// Plugin error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginError {
    /// Failed to load plugin
    LoadError { message: String },
    /// Failed to initialize plugin
    InitError { message: String },
    /// Plugin execution error
    ExecutionError { message: String },
    /// Configuration error
    ConfigError { message: String },
    /// Dependency not found
    DependencyError {
        plugin_id: String,
        required_version: String,
    },
    /// Version mismatch
    VersionError { required: String, actual: String },
    /// Plugin not found
    NotFound { plugin_id: String },
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadError { message } => write!(f, "Load error: {}", message),
            Self::InitError { message } => write!(f, "Init error: {}", message),
            Self::ExecutionError { message } => write!(f, "Execution error: {}", message),
            Self::ConfigError { message } => write!(f, "Config error: {}", message),
            Self::DependencyError {
                plugin_id,
                required_version,
            } => {
                write!(
                    f,
                    "Missing dependency: {} ({})",
                    plugin_id, required_version
                )
            }
            Self::VersionError { required, actual } => {
                write!(
                    f,
                    "Version mismatch: required {}, actual {}",
                    required, actual
                )
            }
            Self::NotFound { plugin_id } => write!(f, "Plugin not found: {}", plugin_id),
        }
    }
}

impl std::error::Error for PluginError {}

/// Plugin statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginStats {
    /// Number of times plugin was invoked
    pub invocations: u64,
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Number of errors
    pub errors: u64,
    /// Last invocation time
    pub last_invoked: Option<DateTime<Utc>>,
}

/// Plugin capability flags
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Plugin can intercept requests
    InterceptRequest,
    /// Plugin can intercept responses
    InterceptResponse,
    /// Plugin can intercept WebSocket messages
    InterceptWebSocket,
    /// Plugin can intercept gRPC messages
    InterceptGrpc,
    /// Plugin provides UI panels
    UiPanel,
    /// Plugin provides custom themes
    Theme,
    /// Plugin provides export formats
    ExportFormat,
    /// Plugin provides import formats
    ImportFormat,
    /// Plugin requires network access (host `http_fetch`)
    Network,
}

impl PluginCapability {
    /// Parse a capability from its snake_case string name.
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        Some(match s {
            "intercept_request" => Self::InterceptRequest,
            "intercept_response" => Self::InterceptResponse,
            "intercept_websocket" => Self::InterceptWebSocket,
            "intercept_grpc" => Self::InterceptGrpc,
            "ui_panel" => Self::UiPanel,
            "theme" => Self::Theme,
            "export_format" => Self::ExportFormat,
            "import_format" => Self::ImportFormat,
            "network" => Self::Network,
            _ => return None,
        })
    }
}

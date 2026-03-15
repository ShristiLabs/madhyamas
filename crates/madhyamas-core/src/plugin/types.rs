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
    pub dependencies: HashMap<String, String>,
    /// Hooks the plugin subscribes to
    pub hooks: Vec<String>,
    /// Plugin settings schema
    pub settings: Option<PluginSettingsSchema>,
    /// Whether the plugin is enabled by default
    pub enabled_by_default: bool,
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
}

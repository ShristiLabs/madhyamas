//! Plugin hooks and execution context

use crate::traffic::{RequestData, ResponseData};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin hook types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginHook {
    /// Called when plugin is loaded
    OnLoad,
    /// Called when plugin is enabled
    OnEnable,
    /// Called when plugin is disabled
    OnDisable,
    /// Called when plugin is unloaded
    OnUnload,
    /// Called before request is forwarded
    OnRequest,
    /// Called after response is received
    OnResponse,
    /// Called for WebSocket messages
    OnWebSocket,
    /// Called for gRPC messages
    OnGrpc,
    /// Called when settings change
    OnSettingsChange,
    /// Called periodically (configurable interval)
    OnTimer,
}

impl PluginHook {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OnLoad => "on_load",
            Self::OnEnable => "on_enable",
            Self::OnDisable => "on_disable",
            Self::OnUnload => "on_unload",
            Self::OnRequest => "on_request",
            Self::OnResponse => "on_response",
            Self::OnWebSocket => "on_websocket",
            Self::OnGrpc => "on_grpc",
            Self::OnSettingsChange => "on_settings_change",
            Self::OnTimer => "on_timer",
        }
    }

    /// The WASM export name the host looks up when dispatching this hook.
    pub fn export_name(&self) -> &'static str {
        self.as_str()
    }

    /// Stable numeric id used by the WASM ABI (`__madhyamas_hook`'s `hook_id`
    /// argument). Do not renumber existing variants — guest SDKs depend on
    /// these values.
    pub fn export_id(&self) -> i32 {
        match self {
            Self::OnLoad => 0,
            Self::OnEnable => 1,
            Self::OnDisable => 2,
            Self::OnUnload => 3,
            Self::OnRequest => 4,
            Self::OnResponse => 5,
            Self::OnWebSocket => 6,
            Self::OnGrpc => 7,
            Self::OnSettingsChange => 8,
            Self::OnTimer => 9,
        }
    }

    /// Parse a hook from its numeric export id (inverse of [`export_id`]).
    pub fn from_export_id(id: i32) -> Option<Self> {
        Some(match id {
            0 => Self::OnLoad,
            1 => Self::OnEnable,
            2 => Self::OnDisable,
            3 => Self::OnUnload,
            4 => Self::OnRequest,
            5 => Self::OnResponse,
            6 => Self::OnWebSocket,
            7 => Self::OnGrpc,
            8 => Self::OnSettingsChange,
            9 => Self::OnTimer,
            _ => return None,
        })
    }

    /// Parse a hook from its snake_case string name.
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        Some(match s {
            "on_load" => Self::OnLoad,
            "on_enable" => Self::OnEnable,
            "on_disable" => Self::OnDisable,
            "on_unload" => Self::OnUnload,
            "on_request" => Self::OnRequest,
            "on_response" => Self::OnResponse,
            "on_websocket" => Self::OnWebSocket,
            "on_grpc" => Self::OnGrpc,
            "on_settings_change" => Self::OnSettingsChange,
            "on_timer" => Self::OnTimer,
            _ => return None,
        })
    }
}

impl std::fmt::Display for PluginHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Plugin execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// Plugin ID
    pub plugin_id: String,
    /// Request ID (if applicable)
    pub request_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Current hook
    pub hook: String,
    /// Request data (for request/response hooks)
    pub request: Option<PluginRequest>,
    /// Response data (for response hooks)
    pub response: Option<PluginResponse>,
    /// Plugin settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Shared state between hooks
    pub state: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PluginContext {
    pub fn new(plugin_id: &str, hook: PluginHook) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            request_id: None,
            session_id: None,
            hook: hook.as_str().to_string(),
            request: None,
            response: None,
            settings: HashMap::new(),
            state: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_request(mut self, req: &RequestData) -> Self {
        self.request = Some(PluginRequest::from(req));
        self
    }

    pub fn with_response(mut self, resp: &ResponseData) -> Self {
        self.response = Some(PluginResponse::from(resp));
        self
    }
}

/// Request data for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
}

impl From<&RequestData> for PluginRequest {
    fn from(req: &RequestData) -> Self {
        Self {
            method: req.method.to_string(),
            url: req.url.clone(),
            host: req.host.clone(),
            path: req.path.clone(),
            headers: req.headers.clone(),
            // Clone the body — plugin execution is currently a no-op, so this
            // is never exercised. When plugins are implemented, consider using
            // Arc<Vec<u8>> with a custom serde wrapper to avoid cloning.
            body: req.body.clone(),
            content_type: req.content_type.clone(),
        }
    }
}

/// Response data for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    pub status_code: u16,
    pub status_message: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
    pub duration_ms: u64,
}

impl From<&ResponseData> for PluginResponse {
    fn from(resp: &ResponseData) -> Self {
        Self {
            status_code: resp.status_code,
            status_message: resp.status_message.clone(),
            headers: resp.headers.clone(),
            body: resp.body.clone(),
            content_type: resp.content_type.clone(),
            duration_ms: resp.duration_ms,
        }
    }
}

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    /// Whether the plugin handled the hook
    pub handled: bool,
    /// Whether to continue to next plugin
    pub continue_: bool,
    /// Whether data was modified
    pub modified: bool,
    /// Modified request (if applicable)
    pub request: Option<PluginRequest>,
    /// Modified response (if applicable)
    pub response: Option<PluginResponse>,
    /// Error message if failed
    pub error: Option<String>,
    /// Log messages
    pub logs: Vec<String>,
    /// Custom response to return
    pub custom_response: Option<PluginResponse>,
}

impl Default for PluginResult {
    fn default() -> Self {
        Self {
            handled: false,
            continue_: true,
            modified: false,
            request: None,
            response: None,
            error: None,
            logs: Vec::new(),
            custom_response: None,
        }
    }
}

impl PluginResult {
    /// Create a continue result
    pub fn cont() -> Self {
        Self::default()
    }

    /// Create a modified result
    pub fn modified() -> Self {
        Self {
            modified: true,
            ..Default::default()
        }
    }

    /// Create an error result
    pub fn error(message: &str) -> Self {
        Self {
            handled: true,
            continue_: false,
            error: Some(message.to_string()),
            ..Default::default()
        }
    }

    /// Create a stop result with custom response
    pub fn respond(status: u16, body: &str) -> Self {
        Self {
            handled: true,
            continue_: false,
            custom_response: Some(PluginResponse {
                status_code: status,
                status_message: None,
                headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
                body: Some(body.as_bytes().to_vec()),
                content_type: Some("text/plain".to_string()),
                duration_ms: 0,
            }),
            ..Default::default()
        }
    }
}

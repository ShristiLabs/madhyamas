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

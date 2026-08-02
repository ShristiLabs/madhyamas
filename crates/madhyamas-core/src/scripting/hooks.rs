//! Script hooks and execution context

use crate::traffic::{RequestData, ResponseData};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Available script hooks
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScriptHook {
    /// Called before a request is forwarded
    OnRequest,
    /// Called after a response is received
    OnResponse,
    /// Called when a WebSocket message is sent/received
    OnWebSocketMessage,
    /// Called when a gRPC message is sent/received
    OnGrpcMessage,
    /// Called on traffic store
    OnTrafficStore,
    /// Called when a session starts
    OnSessionStart,
    /// Called when a session ends
    OnSessionEnd,
}

impl ScriptHook {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OnRequest => "on_request",
            Self::OnResponse => "on_response",
            Self::OnWebSocketMessage => "on_websocket_message",
            Self::OnGrpcMessage => "on_grpc_message",
            Self::OnTrafficStore => "on_traffic_store",
            Self::OnSessionStart => "on_session_start",
            Self::OnSessionEnd => "on_session_end",
        }
    }
}

impl std::fmt::Display for ScriptHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ScriptHook {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "on_request" => Ok(Self::OnRequest),
            "on_response" => Ok(Self::OnResponse),
            "on_websocket_message" => Ok(Self::OnWebSocketMessage),
            "on_grpc_message" => Ok(Self::OnGrpcMessage),
            "on_traffic_store" => Ok(Self::OnTrafficStore),
            "on_session_start" => Ok(Self::OnSessionStart),
            "on_session_end" => Ok(Self::OnSessionEnd),
            other => Err(format!("Unknown hook: {other}")),
        }
    }
}

/// Script execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptContext {
    /// Request ID
    pub request_id: String,
    /// Session ID
    pub session_id: String,
    /// Current request (if applicable)
    pub request: Option<RequestContext>,
    /// Current response (if applicable)
    pub response: Option<ResponseContext>,
    /// WebSocket message (if applicable)
    pub websocket: Option<WebSocketContext>,
    /// gRPC message (if applicable)
    pub grpc: Option<GrpcContext>,
    /// Custom data that can be passed between hooks
    pub data: HashMap<String, serde_json::Value>,
    /// Hook that triggered this context
    pub hook: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ScriptContext {
    pub fn new(request_id: &str, session_id: &str, hook: ScriptHook) -> Self {
        Self {
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            request: None,
            response: None,
            websocket: None,
            grpc: None,
            data: HashMap::new(),
            hook: hook.as_str().to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_request(mut self, request: &RequestData) -> Self {
        self.request = Some(RequestContext::from(request));
        self
    }

    pub fn with_response(mut self, response: &ResponseData) -> Self {
        self.response = Some(ResponseContext::from(response));
        self
    }
}

/// Request context exposed to scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub content_type: Option<String>,
    pub query: HashMap<String, String>,
}

impl From<&RequestData> for RequestContext {
    fn from(req: &RequestData) -> Self {
        // Parse query parameters
        let query = if let Ok(uri) = req.url.parse::<hyper::Uri>() {
            uri.query()
                .map(|q| {
                    url::form_urlencoded::parse(q.as_bytes())
                        .into_owned()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            method: req.method.to_string(),
            url: req.url.clone(),
            host: req.host.clone(),
            path: req.path.clone(),
            headers: req.headers.clone(),
            body: req
                .body
                .as_ref()
                .and_then(|b| std::str::from_utf8(b).ok().map(|s| s.to_string())),
            content_type: req.content_type.clone(),
            query,
        }
    }
}

/// Response context exposed to scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContext {
    pub status_code: u16,
    pub status_message: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub content_type: Option<String>,
    pub duration_ms: u64,
}

impl From<&ResponseData> for ResponseContext {
    fn from(resp: &ResponseData) -> Self {
        Self {
            status_code: resp.status_code,
            status_message: resp.status_message.clone(),
            headers: resp.headers.clone(),
            body: resp
                .body
                .as_ref()
                .and_then(|b| std::str::from_utf8(b).ok().map(|s| s.to_string())),
            content_type: resp.content_type.clone(),
            duration_ms: resp.duration_ms,
        }
    }
}

/// WebSocket context for scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketContext {
    pub connection_id: String,
    pub direction: String,
    pub message_type: String,
    pub payload: String,
    pub is_binary: bool,
}

/// gRPC context for scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcContext {
    pub stream_id: String,
    pub service: Option<String>,
    pub method: Option<String>,
    pub direction: String,
    pub message: Option<String>,
}

/// Result from script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResult {
    /// Whether the request/response was modified
    pub modified: bool,
    /// Whether to continue processing (false = stop and use response)
    pub continue_: bool,
    /// Custom response to return (if continue is false)
    pub response: Option<ScriptResponse>,
    /// Error message if script failed
    pub error: Option<String>,
    /// Console output from script
    pub console: Vec<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Modified request (present when `modified` is true and the script
    /// changed the request object on an `on_request` hook)
    pub modified_request: Option<RequestContext>,
    /// Modified response (present when `modified` is true and the script
    /// changed the response object on an `on_response` hook)
    pub modified_response: Option<ResponseContext>,
}

impl Default for ScriptResult {
    fn default() -> Self {
        Self {
            modified: false,
            continue_: true,
            response: None,
            error: None,
            console: Vec::new(),
            duration_ms: 0,
            modified_request: None,
            modified_response: None,
        }
    }
}

/// Response from script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl ScriptResponse {
    /// Convert to ResponseData
    pub fn to_response_data(&self) -> ResponseData {
        ResponseData {
            status_code: self.status_code,
            status_message: None,
            headers: self.headers.clone(),
            body: Some(self.body.as_bytes().to_vec()),
            content_type: self.headers.get("Content-Type").cloned(),
            duration_ms: 0,
            http_version: None,
        }
    }
}

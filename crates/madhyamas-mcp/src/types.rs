//! MCP protocol types

use serde::{Deserialize, Serialize};

/// Authentication method for MCP tool API calls.
///
/// In enterprise mode (with `--enable-auth`), the Madhyamas API rejects
/// unauthenticated requests with HTTP 401. The MCP server attaches the
/// configured credentials to every outbound request so tools continue to
/// work behind the auth middleware. In OSS mode (or when auth is disabled),
/// [`McpAuth::None`] sends no credentials — the API server ignores them.
#[derive(Debug, Clone)]
pub enum McpAuth {
    /// No authentication (OSS mode or auth disabled).
    None,
    /// API key authentication (`X-API-Key` header).
    ApiKey(String),
    /// JWT authentication (`Authorization: Bearer` header).
    Jwt(String),
}

/// MCP transport mode.
///
/// The MCP server can run over stdio (the default, for local AI agent
/// integration) or over HTTP (for remote / multi-instance deployments).
/// HTTP transport accepts JSON-RPC POST requests on a single endpoint.
#[derive(Debug, Clone, Default)]
pub enum McpTransport {
    /// Standard stdio transport (line-delimited JSON-RPC over stdin/stdout).
    #[default]
    Stdio,
    /// Streamable HTTP transport (JSON-RPC over HTTP POST on the given port).
    Http { port: u16 },
}

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Madhyamas API URL
    pub api_url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Authentication method for API calls.
    pub auth: McpAuth,
    /// Transport mode (stdio or HTTP).
    pub transport: McpTransport,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:3001".to_string(),
            timeout_secs: 30,
            auth: McpAuth::None,
            transport: McpTransport::Stdio,
        }
    }
}

impl McpConfig {
    /// Build the HTTP auth header pairs for this configuration.
    ///
    /// Returns an empty vector when no authentication is configured
    /// ([`McpAuth::None`]). The returned pairs are applied as default
    /// headers on the MCP server's HTTP client so every tool request
    /// carries the credentials automatically.
    pub fn auth_headers(&self) -> Vec<(String, String)> {
        match &self.auth {
            McpAuth::None => vec![],
            McpAuth::ApiKey(key) => vec![("X-API-Key".to_string(), key.clone())],
            McpAuth::Jwt(token) => {
                vec![("Authorization".to_string(), format!("Bearer {}", token))]
            }
        }
    }
}

/// MCP error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum McpError {
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

impl madhyamas_core::error::AppError for McpError {
    fn error_code(&self) -> &str {
        match self {
            McpError::JsonRpc(_) => "MCP_JSON_RPC",
            McpError::Http(_) => "MCP_HTTP",
            McpError::ToolExecution(_) => "MCP_TOOL_EXECUTION",
            McpError::InvalidParams(_) => "MCP_INVALID_PARAMS",
            McpError::NotFound(_) => "MCP_NOT_FOUND",
            McpError::Parse(_) => "MCP_PARSE",
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            // Transient transport failures may succeed on retry.
            McpError::Http(_) | McpError::JsonRpc(_) => true,
            // Invalid input, missing resources, parse failures, and tool
            // execution errors are unlikely to resolve without changes.
            McpError::ToolExecution(_)
            | McpError::InvalidParams(_)
            | McpError::NotFound(_)
            | McpError::Parse(_) => false,
        }
    }
}

// ============ JSON-RPC 2.0 Types ============

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ============ MCP Protocol Types ============

/// MCP server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    pub resources: Option<ResourcesCapability>,
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Tool annotations (MCP spec hints + enterprise permission hint).
///
/// The `readOnlyHint`, `destructiveHint`, and `idempotentHint` fields
/// follow the MCP specification's tool annotations format. The
/// `required_permission` field is a Madhyamas extension that hints at
/// the RBAC permission needed to execute the tool against an enterprise
/// API server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations {
    /// If true, the tool does not modify its environment.
    #[serde(rename = "readOnlyHint", skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    /// If true, the tool may perform irreversible changes.
    #[serde(rename = "destructiveHint", skip_serializing_if = "Option::is_none")]
    pub destructive: Option<bool>,
    /// If true, repeated calls with the same arguments produce the same result.
    #[serde(rename = "idempotentHint", skip_serializing_if = "Option::is_none")]
    pub idempotent: Option<bool>,
    /// RBAC permission required to execute this tool (enterprise tier).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_permission: Option<String>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    /// Tool annotations (MCP spec hints + enterprise permission).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Content block for tool results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: ResourceContents },
}

/// Resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContents {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub text: String,
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

/// Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// List tools result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// List resources result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResult {
    pub resources: Vec<Resource>,
}

/// Read resource result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContents>,
}

/// Resource template definition (for `resources/templates/list`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplate {
    pub uri_template: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// List resource templates result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesResult {
    pub resource_templates: Vec<ResourceTemplate>,
}

/// A prompt argument definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Prompt definition (for `prompts/list`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub arguments: Vec<PromptArgument>,
}

/// List prompts result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsResult {
    pub prompts: Vec<Prompt>,
}

/// A message in a prompt result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: ContentBlock,
}

/// Get prompt result (for `prompts/get`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

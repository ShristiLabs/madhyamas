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

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Madhyamas API URL
    pub api_url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Authentication method for API calls.
    pub auth: McpAuth,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:3001".to_string(),
            timeout_secs: 30,
            auth: McpAuth::None,
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

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_auth_none_headers() {
        let config = McpConfig {
            api_url: "http://localhost".to_string(),
            timeout_secs: 5,
            auth: McpAuth::None,
        };
        assert!(config.auth_headers().is_empty());
        assert!(matches!(McpAuth::None, McpAuth::None));
    }

    #[test]
    fn test_mcp_auth_api_key_headers() {
        let config = McpConfig {
            api_url: "http://localhost".to_string(),
            timeout_secs: 5,
            auth: McpAuth::ApiKey("secret-key-123".to_string()),
        };
        let headers = config.auth_headers();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "X-API-Key");
        assert_eq!(headers[0].1, "secret-key-123");
        assert!(!matches!(McpAuth::ApiKey("x".to_string()), McpAuth::None));
    }

    #[test]
    fn test_mcp_auth_jwt_headers() {
        let config = McpConfig {
            api_url: "http://localhost".to_string(),
            timeout_secs: 5,
            auth: McpAuth::Jwt("jwt-token-456".to_string()),
        };
        let headers = config.auth_headers();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "Authorization");
        assert_eq!(headers[0].1, "Bearer jwt-token-456");
        assert!(!matches!(McpAuth::Jwt("x".to_string()), McpAuth::None));
    }

    #[test]
    fn test_mcp_config_default_auth_none() {
        let config = McpConfig::default();
        assert!(config.auth_headers().is_empty());
        assert!(matches!(config.auth, McpAuth::None));
    }
}

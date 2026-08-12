//! WebSocket traffic MCP tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, api_result_void, get_id};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

/// List all WebSocket connections.
pub struct ListWsConnectionsTool;

#[async_trait::async_trait]
impl McpTool for ListWsConnectionsTool {
    fn name(&self) -> &str {
        "madhyamas_list_ws_connections"
    }
    fn description(&self) -> &str {
        "List all captured WebSocket connections observed by the proxy."
    }
    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/ws-traffic/connections", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get a specific WebSocket connection.
pub struct GetWsConnectionTool;

#[async_trait::async_trait]
impl McpTool for GetWsConnectionTool {
    fn name(&self) -> &str {
        "madhyamas_get_ws_connection"
    }
    fn description(&self) -> &str {
        "Get details of a specific WebSocket connection by ID."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "WebSocket connection ID" }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let resp = client
            .get(format!("{}/api/ws-traffic/connections/{}", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get WebSocket messages with optional filtering.
pub struct GetWsMessagesTool;

#[async_trait::async_trait]
impl McpTool for GetWsMessagesTool {
    fn name(&self) -> &str {
        "madhyamas_get_ws_messages"
    }
    fn description(&self) -> &str {
        "Get captured WebSocket messages with optional filtering by \
         connection ID, direction, message type, and text search."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "connection_id": { "type": "string", "description": "Filter by connection ID" },
                "direction": { "type": "string", "enum": ["send", "receive"], "description": "Filter by direction" },
                "message_type": { "type": "string", "enum": ["text", "binary", "ping", "pong", "close"], "description": "Filter by message type" },
                "search": { "type": "string", "description": "Search in message payloads" },
                "limit": { "type": "integer", "description": "Maximum number of results" },
                "offset": { "type": "integer", "description": "Offset for pagination" }
            }
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let mut params = Vec::new();
        for key in [
            "connection_id",
            "direction",
            "message_type",
            "search",
            "limit",
            "offset",
        ] {
            if let Some(v) = arguments.get(key) {
                if let Some(s) = v.as_str() {
                    params.push(format!("{}={}", key, s));
                } else if let Some(n) = v.as_u64() {
                    params.push(format!("{}={}", key, n));
                }
            }
        }
        let path = if params.is_empty() {
            format!("{}/api/ws-traffic/messages", api_url)
        } else {
            format!("{}/api/ws-traffic/messages?{}", api_url, params.join("&"))
        };
        let resp = client
            .get(&path)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Clear all WebSocket traffic.
pub struct ClearWsTrafficTool;

#[async_trait::async_trait]
impl McpTool for ClearWsTrafficTool {
    fn name(&self) -> &str {
        "madhyamas_clear_ws_traffic"
    }
    fn description(&self) -> &str {
        "Clear all captured WebSocket messages and closed connections. \
         This action cannot be undone."
    }
    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .post(format!("{}/api/ws-traffic/clear", api_url))
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, "WebSocket traffic cleared").await
    }
}

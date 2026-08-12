//! Block list MCP tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, api_result_void, get_id};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

/// List all block list entries.
pub struct ListBlockListTool;

#[async_trait::async_trait]
impl McpTool for ListBlockListTool {
    fn name(&self) -> &str {
        "madhyamas_list_blocklist"
    }
    fn description(&self) -> &str {
        "List all block list entries. Block list entries block requests \
         matching a domain/pattern and return a configurable response \
         instead of forwarding upstream."
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
            .get(format!("{}/api/blocklist", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get block list statistics.
pub struct GetBlockListStatsTool;

#[async_trait::async_trait]
impl McpTool for GetBlockListStatsTool {
    fn name(&self) -> &str {
        "madhyamas_get_blocklist_stats"
    }
    fn description(&self) -> &str {
        "Get block list summary statistics (total entries, enabled count, \
         total hits)."
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
            .get(format!("{}/api/blocklist/stats", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Create a block list entry.
pub struct CreateBlockListEntryTool;

#[async_trait::async_trait]
impl McpTool for CreateBlockListEntryTool {
    fn name(&self) -> &str {
        "madhyamas_create_blocklist_entry"
    }
    fn description(&self) -> &str {
        "Create a block list entry to block requests matching a domain or \
         pattern. Supports exact domains, wildcard subdomains (*.example.com), \
         and globs (*ads*). Returns a configurable status code (default 403)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Domain or wildcard pattern to block"
                },
                "note": {
                    "type": "string",
                    "description": "Optional note describing why this entry exists"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the entry is enabled (default: true)"
                },
                "status_code": {
                    "type": "integer",
                    "description": "HTTP status code to return (default: 403)"
                },
                "response_body": {
                    "type": "string",
                    "description": "Response body to return when blocked"
                },
                "content_type": {
                    "type": "string",
                    "description": "Content-Type header for the block response"
                }
            },
            "required": ["pattern"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .post(format!("{}/api/blocklist", api_url))
            .json(arguments)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get a specific block list entry.
pub struct GetBlockListEntryTool;

#[async_trait::async_trait]
impl McpTool for GetBlockListEntryTool {
    fn name(&self) -> &str {
        "madhyamas_get_blocklist_entry"
    }
    fn description(&self) -> &str {
        "Get details of a specific block list entry by ID."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Block list entry ID" }
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
            .get(format!("{}/api/blocklist/{}", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Update a block list entry.
pub struct UpdateBlockListEntryTool;

#[async_trait::async_trait]
impl McpTool for UpdateBlockListEntryTool {
    fn name(&self) -> &str {
        "madhyamas_update_blocklist_entry"
    }
    fn description(&self) -> &str {
        "Update an existing block list entry. Provide the full entry object \
         with modified fields."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Block list entry ID" },
                "entry": { "type": "object", "description": "Full block list entry object with updates" }
            },
            "required": ["id", "entry"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let entry = arguments
            .get("entry")
            .ok_or_else(|| McpError::InvalidParams("entry is required".to_string()))?;
        let resp = client
            .put(format!("{}/api/blocklist/{}", api_url, id))
            .json(entry)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, "Block list entry updated").await
    }
}

/// Delete a block list entry.
pub struct DeleteBlockListEntryTool;

#[async_trait::async_trait]
impl McpTool for DeleteBlockListEntryTool {
    fn name(&self) -> &str {
        "madhyamas_delete_blocklist_entry"
    }
    fn description(&self) -> &str {
        "Delete a block list entry by ID."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Block list entry ID" }
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
            .delete(format!("{}/api/blocklist/{}", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, &format!("Block list entry {} deleted", id)).await
    }
}

/// Toggle a block list entry.
pub struct ToggleBlockListEntryTool;

#[async_trait::async_trait]
impl McpTool for ToggleBlockListEntryTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_blocklist_entry"
    }
    fn description(&self) -> &str {
        "Enable or disable a block list entry."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Block list entry ID" },
                "enabled": { "type": "boolean", "description": "true to enable, false to disable" }
            },
            "required": ["id", "enabled"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let enabled = arguments
            .get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| McpError::InvalidParams("enabled is required".to_string()))?;
        let resp = client
            .post(format!("{}/api/blocklist/{}/toggle", api_url, id))
            .json(&json!({ "enabled": enabled }))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(
            resp,
            &format!(
                "Block list entry {} {}",
                id,
                if enabled { "enabled" } else { "disabled" }
            ),
        )
        .await
    }
}

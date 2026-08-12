//! Breakpoint tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, api_result_void, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// Create a breakpoint rule
pub(super) async fn create_breakpoint(
    client: &Client,
    api_url: &str,
    url_pattern: &str,
    method: Option<&str>,
    direction: Option<&str>,
    enabled: Option<bool>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/breakpoints", api_url);

    let mut bp_config = json!({
        "url_pattern": url_pattern,
    });

    if let Some(m) = method {
        bp_config["method"] = Value::String(m.to_string());
    }
    if let Some(d) = direction {
        bp_config["direction"] = Value::String(d.to_string());
    }
    if let Some(e) = enabled {
        bp_config["enabled"] = Value::Bool(e);
    }

    let response = client
        .post(&url)
        .json(&bp_config)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let breakpoint: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(json!({
        "success": true,
        "message": format!("Breakpoint created for pattern: {}", url_pattern),
        "breakpoint": breakpoint
    }))
}

/// List all breakpoint rules
pub(super) async fn list_breakpoints(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/breakpoints", api_url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let breakpoints: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(breakpoints)
}

/// Delete a breakpoint rule
pub(super) async fn delete_breakpoint(
    client: &Client,
    api_url: &str,
    breakpoint_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/breakpoints/{}", api_url, breakpoint_id);

    let response = client
        .delete(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({
        "success": true,
        "message": "Breakpoint deleted"
    }))
}

/// Get paused traffic (traffic currently held at breakpoints)
#[allow(dead_code)]
pub(super) async fn get_paused_traffic(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/breakpoints/paused", api_url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let paused: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(paused)
}

/// Resume paused traffic
#[allow(dead_code)]
pub(super) async fn resume_paused_traffic(
    client: &Client,
    api_url: &str,
    paused_id: &str,
    action: &str, // "continue" or "abort"
    modifications: Option<Value>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/breakpoints/paused/{}/resume", api_url, paused_id);

    let mut body = json!({
        "action": action,
    });
    if let Some(mods) = modifications {
        body["modifications"] = mods;
    }

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({
        "success": true,
        "message": format!("Traffic {}", action),
    }))
}

// ============ Trait-based tool structs ============

/// List all breakpoint rules currently configured.
pub struct ListBreakpointsTool;

#[async_trait::async_trait]
impl McpTool for ListBreakpointsTool {
    fn name(&self) -> &str {
        "madhyamas_list_breakpoints"
    }
    fn description(&self) -> &str {
        "List all breakpoint rules currently configured."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let result = list_breakpoints(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Create a breakpoint rule to pause traffic matching a pattern.
pub struct CreateBreakpointTool;

#[async_trait::async_trait]
impl McpTool for CreateBreakpointTool {
    fn name(&self) -> &str {
        "madhyamas_create_breakpoint"
    }
    fn description(&self) -> &str {
        "Create a breakpoint rule to pause traffic matching a pattern. Paused traffic can be inspected and modified before proceeding."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url_pattern": {
                    "type": "string",
                    "description": "URL pattern to match"
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                    "description": "HTTP method to match"
                },
                "direction": {
                    "type": "string",
                    "enum": ["request", "response", "both"],
                    "description": "Which direction to intercept (default: request)"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether breakpoint is enabled immediately (default: true)"
                }
            },
            "required": ["url_pattern"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let url_pattern = arguments
            .get("url_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("url_pattern is required".to_string()))?;
        let method = arguments.get("method").and_then(|v| v.as_str());
        let direction = arguments.get("direction").and_then(|v| v.as_str());
        let enabled = arguments.get("enabled").and_then(|v| v.as_bool());
        let result =
            create_breakpoint(client, api_url, url_pattern, method, direction, enabled).await?;
        Ok(json_text(&result))
    }
}

/// Delete a breakpoint rule.
pub struct DeleteBreakpointTool;

#[async_trait::async_trait]
impl McpTool for DeleteBreakpointTool {
    fn name(&self) -> &str {
        "madhyamas_delete_breakpoint"
    }
    fn description(&self) -> &str {
        "Delete a breakpoint rule."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the breakpoint rule to delete"
                }
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
        let result = delete_breakpoint(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

/// Get details of a specific breakpoint rule.
pub struct GetBreakpointTool;

#[async_trait::async_trait]
impl McpTool for GetBreakpointTool {
    fn name(&self) -> &str {
        "madhyamas_get_breakpoint"
    }
    fn description(&self) -> &str {
        "Get details of a specific breakpoint rule by ID."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the breakpoint rule to retrieve"
                }
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
            .get(format!("{}/api/breakpoints/{}", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// List all traffic paused by breakpoints.
pub struct ListPausedTrafficTool;

#[async_trait::async_trait]
impl McpTool for ListPausedTrafficTool {
    fn name(&self) -> &str {
        "madhyamas_list_paused_traffic"
    }
    fn description(&self) -> &str {
        "List all traffic currently paused by breakpoints. Paused traffic \
         can be inspected and then resumed (continued or aborted)."
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
            .get(format!("{}/api/breakpoints/paused", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get a specific paused item.
pub struct GetPausedItemTool;

#[async_trait::async_trait]
impl McpTool for GetPausedItemTool {
    fn name(&self) -> &str {
        "madhyamas_get_paused_item"
    }
    fn description(&self) -> &str {
        "Get details of a specific paused traffic item by ID, including \
         request headers and body."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Paused item ID" }
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
            .get(format!("{}/api/breakpoints/paused/{}", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Resume a paused item.
pub struct ResumePausedItemTool;

#[async_trait::async_trait]
impl McpTool for ResumePausedItemTool {
    fn name(&self) -> &str {
        "madhyamas_resume_paused_item"
    }
    fn description(&self) -> &str {
        "Resume a paused traffic item. Use action='continue' to allow the \
         request to proceed, or action='abort' to abort it."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Paused item ID" },
                "action": {
                    "type": "string",
                    "enum": ["continue", "abort"],
                    "description": "Action to take: continue or abort"
                }
            },
            "required": ["id", "action"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let action = arguments
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("action is required".to_string()))?;
        let resp = client
            .post(format!("{}/api/breakpoints/paused/{}/resume", api_url, id))
            .json(&json!({ "action": action }))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, &format!("Paused item {} {}", id, action)).await
    }
}

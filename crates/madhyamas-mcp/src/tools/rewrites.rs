//! Rewrite rule tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// List all rewrite rules
pub(super) async fn list_rewrites(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/rewrites", api_url);

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

    let rewrites: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(rewrites)
}

/// Create a rewrite rule
#[allow(clippy::too_many_arguments)]
pub(super) async fn create_rewrite(
    client: &Client,
    api_url: &str,
    name: &str,
    condition: Value,
    direction: &str,
    rewrites: Value,
    enabled: Option<bool>,
    priority: Option<u32>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/rewrites", api_url);

    let mut body = json!({
        "name": name,
        "condition": condition,
        "direction": direction,
        "rewrites": rewrites,
    });

    if let Some(e) = enabled {
        body["enabled"] = Value::Bool(e);
    }
    if let Some(p) = priority {
        body["priority"] = Value::Number(p.into());
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

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(result)
}

/// Delete a rewrite rule
pub(super) async fn delete_rewrite(
    client: &Client,
    api_url: &str,
    rewrite_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/rewrites/{}", api_url, rewrite_id);

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
        "message": "Rewrite rule deleted"
    }))
}

/// Toggle a rewrite rule on/off
pub(super) async fn toggle_rewrite(
    client: &Client,
    api_url: &str,
    rewrite_id: &str,
    enabled: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/rewrites/{}/toggle", api_url, rewrite_id);

    let response = client
        .post(&url)
        .json(&json!({ "enabled": enabled }))
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
        "message": format!("Rewrite rule {}", if enabled { "enabled" } else { "disabled" })
    }))
}

/// Get rewrite templates
pub(super) async fn get_rewrite_templates(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/rewrites/templates", api_url);

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

    let templates: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(templates)
}

// ============ Trait-based tool structs ============

#[derive(Debug, Clone, serde::Deserialize)]
struct CreateRewriteArgs {
    name: String,
    condition: Value,
    direction: String,
    rewrites: Value,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    priority: Option<u32>,
}

pub struct ListRewritesTool;

#[async_trait::async_trait]
impl McpTool for ListRewritesTool {
    fn name(&self) -> &str {
        "madhyamas_list_rewrites"
    }

    fn description(&self) -> &str {
        "List all URL/header rewrite rules currently configured."
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
        let result = list_rewrites(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct CreateRewriteTool;

#[async_trait::async_trait]
impl McpTool for CreateRewriteTool {
    fn name(&self) -> &str {
        "madhyamas_create_rewrite"
    }

    fn description(&self) -> &str {
        "Create a rewrite rule to modify URLs, headers, or bodies of matching requests/responses."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the rewrite rule"
                },
                "condition": {
                    "type": "object",
                    "description": "Match condition (e.g., {\"type\": \"url_pattern\", \"pattern\": \"https://api.example.com/.*\"})"
                },
                "direction": {
                    "type": "string",
                    "enum": ["request", "response", "both"],
                    "description": "Which direction to apply rewrites (default: request)"
                },
                "rewrites": {
                    "type": "array",
                    "items": {"type": "object"},
                    "description": "List of rewrite actions to apply (e.g., {\"type\": \"set_header\", \"name\": \"X-Custom\", \"value\": \"test\"})"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the rule is enabled (default: true)"
                },
                "priority": {
                    "type": "integer",
                    "description": "Priority (lower = higher priority, default: 100)"
                }
            },
            "required": ["name", "condition", "direction", "rewrites"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let args: CreateRewriteArgs = serde_json::from_value(arguments.clone())
            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
        let result = create_rewrite(
            client,
            api_url,
            &args.name,
            args.condition,
            &args.direction,
            args.rewrites,
            args.enabled,
            args.priority,
        )
        .await?;
        Ok(json_text(&result))
    }
}

pub struct DeleteRewriteTool;

#[async_trait::async_trait]
impl McpTool for DeleteRewriteTool {
    fn name(&self) -> &str {
        "madhyamas_delete_rewrite"
    }

    fn description(&self) -> &str {
        "Delete a rewrite rule."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the rewrite rule to delete"
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
        let result = delete_rewrite(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct ToggleRewriteTool;

#[async_trait::async_trait]
impl McpTool for ToggleRewriteTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_rewrite"
    }

    fn description(&self) -> &str {
        "Enable or disable a rewrite rule."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the rewrite rule to toggle"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "true to enable, false to disable"
                }
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
        let result = toggle_rewrite(client, api_url, &id, enabled).await?;
        Ok(json_text(&result))
    }
}

pub struct GetRewriteTemplatesTool;

#[async_trait::async_trait]
impl McpTool for GetRewriteTemplatesTool {
    fn name(&self) -> &str {
        "madhyamas_get_rewrite_templates"
    }

    fn description(&self) -> &str {
        "Get predefined rewrite rule templates for common scenarios."
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
        let result = get_rewrite_templates(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Update an existing rewrite rule with new configuration.
pub struct UpdateRewriteTool;

#[async_trait::async_trait]
impl McpTool for UpdateRewriteTool {
    fn name(&self) -> &str {
        "madhyamas_update_rewrite"
    }

    fn description(&self) -> &str {
        "Update an existing rewrite rule with new configuration. The id, created_at, and hit_count fields are preserved from the existing rule."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the rewrite rule to update"
                },
                "name": {
                    "type": "string",
                    "description": "Name for the rewrite rule"
                },
                "condition": {
                    "type": "object",
                    "description": "Match condition (e.g., {\"type\": \"url_pattern\", \"pattern\": \"https://api.example.com/.*\"})"
                },
                "direction": {
                    "type": "string",
                    "enum": ["request", "response", "both"],
                    "description": "Which direction to apply rewrites"
                },
                "rewrites": {
                    "type": "array",
                    "items": {"type": "object"},
                    "description": "List of rewrite actions to apply"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the rule is enabled"
                },
                "priority": {
                    "type": "integer",
                    "description": "Priority (lower = higher priority, default: 100)"
                }
            },
            "required": ["id", "name", "condition", "direction", "rewrites"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let mut body = json!({});
        if let Some(name) = arguments.get("name").and_then(|v| v.as_str()) {
            body["name"] = Value::String(name.to_string());
        }
        if let Some(condition) = arguments.get("condition") {
            body["condition"] = condition.clone();
        }
        if let Some(direction) = arguments.get("direction").and_then(|v| v.as_str()) {
            body["direction"] = Value::String(direction.to_string());
        }
        if let Some(rewrites) = arguments.get("rewrites") {
            body["rewrites"] = rewrites.clone();
        }
        if let Some(enabled) = arguments.get("enabled").and_then(|v| v.as_bool()) {
            body["enabled"] = Value::Bool(enabled);
        }
        if let Some(priority) = arguments.get("priority").and_then(|v| v.as_u64()) {
            body["priority"] = Value::Number(priority.into());
        }
        let resp = client
            .put(format!("{}/api/rewrites/{}", api_url, id))
            .json(&body)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Enable or disable multiple rewrite rules in a single request.
pub struct BatchToggleRewritesTool;

#[async_trait::async_trait]
impl McpTool for BatchToggleRewritesTool {
    fn name(&self) -> &str {
        "madhyamas_batch_toggle_rewrites"
    }

    fn description(&self) -> &str {
        "Enable or disable multiple rewrite rules in a single request."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of rewrite rule IDs to toggle"
                },
                "enabled": { "type": "boolean", "description": "true to enable, false to disable" }
            },
            "required": ["ids", "enabled"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .post(format!("{}/api/rewrites/batch-toggle", api_url))
            .json(arguments)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

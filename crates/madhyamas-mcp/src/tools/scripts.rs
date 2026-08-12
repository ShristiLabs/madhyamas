//! Script tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, api_result_void, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// List all scripts
pub(super) async fn list_scripts(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts", api_url);

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

    let scripts: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(scripts)
}

/// Create a new script
pub(super) async fn create_script(
    client: &Client,
    api_url: &str,
    name: &str,
    source: &str,
    hook: Option<&str>,
    enabled: Option<bool>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts", api_url);

    let hooks = match hook {
        Some(h) => vec![Value::String(h.to_string())],
        None => vec![],
    };

    let mut body = json!({
        "name": name,
        "source": source,
        "hooks": hooks,
    });

    if let Some(e) = enabled {
        body["enabled"] = Value::Bool(e);
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

/// Get a specific script
pub(super) async fn get_script(
    client: &Client,
    api_url: &str,
    script_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts/{}", api_url, script_id);

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

    let script: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(script)
}

/// Update a script
pub(super) async fn update_script(
    client: &Client,
    api_url: &str,
    script_id: &str,
    script: Value,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts/{}", api_url, script_id);

    let response = client
        .put(&url)
        .json(&script)
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
        "message": "Script updated"
    }))
}

/// Delete a script
pub(super) async fn delete_script(
    client: &Client,
    api_url: &str,
    script_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts/{}", api_url, script_id);

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
        "message": "Script deleted"
    }))
}

/// Toggle a script on/off
pub(super) async fn toggle_script(
    client: &Client,
    api_url: &str,
    script_id: &str,
    enabled: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts/{}/toggle", api_url, script_id);

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
        "message": format!("Script {}", if enabled { "enabled" } else { "disabled" })
    }))
}

/// Get script templates
pub(super) async fn get_script_templates(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts/templates", api_url);

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

/// Test (dry-run) a script against a sample context
pub(super) async fn test_script(
    client: &Client,
    api_url: &str,
    source: &str,
    hook: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts/test", api_url);

    let body = json!({
        "source": source,
        "hook": hook,
    });

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

/// Validate a script's source code (syntax check)
pub(super) async fn validate_script(
    client: &Client,
    api_url: &str,
    source: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/scripts/validate", api_url);

    let body = json!({ "source": source });

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

/// Get execution history for a specific script
pub(super) async fn get_script_history(
    client: &Client,
    api_url: &str,
    script_id: &str,
    limit: Option<usize>,
) -> Result<Value, McpError> {
    let url = match limit {
        Some(l) => format!("{}/api/scripts/{}/history?limit={}", api_url, script_id, l),
        None => format!("{}/api/scripts/{}/history", api_url, script_id),
    };

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

    let history: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(history)
}

// ============ Trait-based tool structs ============

pub struct ListScriptsTool;

#[async_trait::async_trait]
impl McpTool for ListScriptsTool {
    fn name(&self) -> &str {
        "madhyamas_list_scripts"
    }
    fn description(&self) -> &str {
        "List all registered scripts."
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
        let result = list_scripts(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct CreateScriptTool;

#[async_trait::async_trait]
impl McpTool for CreateScriptTool {
    fn name(&self) -> &str {
        "madhyamas_create_script"
    }
    fn description(&self) -> &str {
        "Create a new script that runs on specified request/response hooks."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the script"
                },
                "source": {
                    "type": "string",
                    "description": "The script source code"
                },
                "hook": {
                    "type": "string",
                    "description": "Hook to attach the script to (e.g., on_request, on_response)"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the script is enabled immediately (default: true)"
                }
            },
            "required": ["name", "source"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let name = arguments
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("name is required".to_string()))?;
        let source = arguments
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("source is required".to_string()))?;
        let hook = arguments.get("hook").and_then(|v| v.as_str());
        let enabled = arguments.get("enabled").and_then(|v| v.as_bool());
        let result = create_script(client, api_url, name, source, hook, enabled).await?;
        Ok(json_text(&result))
    }
}

pub struct GetScriptTool;

#[async_trait::async_trait]
impl McpTool for GetScriptTool {
    fn name(&self) -> &str {
        "madhyamas_get_script"
    }
    fn description(&self) -> &str {
        "Get details of a specific script."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the script to retrieve"
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
        let result = get_script(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct UpdateScriptTool;

#[async_trait::async_trait]
impl McpTool for UpdateScriptTool {
    fn name(&self) -> &str {
        "madhyamas_update_script"
    }
    fn description(&self) -> &str {
        "Update an existing script with new source/configuration."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the script to update"
                },
                "script": {
                    "type": "object",
                    "description": "The full script object to update"
                }
            },
            "required": ["id", "script"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let script = arguments
            .get("script")
            .cloned()
            .ok_or_else(|| McpError::InvalidParams("script is required".to_string()))?;
        let result = update_script(client, api_url, &id, script).await?;
        Ok(json_text(&result))
    }
}

pub struct DeleteScriptTool;

#[async_trait::async_trait]
impl McpTool for DeleteScriptTool {
    fn name(&self) -> &str {
        "madhyamas_delete_script"
    }
    fn description(&self) -> &str {
        "Delete a script."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the script to delete"
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
        let result = delete_script(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct ToggleScriptTool;

#[async_trait::async_trait]
impl McpTool for ToggleScriptTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_script"
    }
    fn description(&self) -> &str {
        "Enable or disable a script."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the script to toggle"
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
        let result = toggle_script(client, api_url, &id, enabled).await?;
        Ok(json_text(&result))
    }
}

pub struct GetScriptTemplatesTool;

#[async_trait::async_trait]
impl McpTool for GetScriptTemplatesTool {
    fn name(&self) -> &str {
        "madhyamas_get_script_templates"
    }
    fn description(&self) -> &str {
        "Get predefined script templates for common scenarios."
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
        let result = get_script_templates(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct TestScriptTool;

#[async_trait::async_trait]
impl McpTool for TestScriptTool {
    fn name(&self) -> &str {
        "madhyamas_test_script"
    }
    fn description(&self) -> &str {
        "Test (dry-run) a script against a sample request/response context without affecting live traffic or recording history."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "The script source code to test"
                },
                "hook": {
                    "type": "string",
                    "description": "Hook to test against (e.g. on_request, on_response)",
                    "enum": ["on_request", "on_response", "on_websocket_message", "on_grpc_message", "on_traffic_store", "on_session_start", "on_session_end"]
                }
            },
            "required": ["source", "hook"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let source = arguments
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("source is required".to_string()))?;
        let hook = arguments
            .get("hook")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("hook is required".to_string()))?;
        let result = test_script(client, api_url, source, hook).await?;
        Ok(json_text(&result))
    }
}

pub struct ValidateScriptTool;

#[async_trait::async_trait]
impl McpTool for ValidateScriptTool {
    fn name(&self) -> &str {
        "madhyamas_validate_script"
    }
    fn description(&self) -> &str {
        "Validate a script's syntax without executing it. Returns whether the source is valid and any parse errors."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "The script source code to validate"
                }
            },
            "required": ["source"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let source = arguments
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("source is required".to_string()))?;
        let result = validate_script(client, api_url, source).await?;
        Ok(json_text(&result))
    }
}

pub struct GetScriptHistoryTool;

#[async_trait::async_trait]
impl McpTool for GetScriptHistoryTool {
    fn name(&self) -> &str {
        "madhyamas_get_script_history"
    }
    fn description(&self) -> &str {
        "Get execution history for a specific script, showing recent runs with success/failure status, duration, console output, and errors."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the script to get history for"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of history entries to return (default: 50)"
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
        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize);
        let result = get_script_history(client, api_url, &id, limit).await?;
        Ok(json_text(&result))
    }
}

pub struct ReorderScriptTool;

#[async_trait::async_trait]
impl McpTool for ReorderScriptTool {
    fn name(&self) -> &str {
        "madhyamas_reorder_script"
    }
    fn description(&self) -> &str {
        "Reorder a script by changing its priority. Lower priority values \
         run earlier in the script chain."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Script ID" },
                "priority": { "type": "integer", "description": "New priority position" }
            },
            "required": ["id", "priority"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let priority = arguments
            .get("priority")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| McpError::InvalidParams("priority is required".to_string()))?;
        let resp = client
            .post(format!("{}/api/scripts/{}/reorder", api_url, id))
            .json(&json!({ "priority": priority }))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(
            resp,
            &format!("Script {} reordered to priority {}", id, priority),
        )
        .await
    }
}

pub struct ScriptMatchPreviewTool;

#[async_trait::async_trait]
impl McpTool for ScriptMatchPreviewTool {
    fn name(&self) -> &str {
        "madhyamas_script_match_preview"
    }
    fn description(&self) -> &str {
        "Preview which scripts would match a given request without \
         actually executing them."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to test" },
                "method": { "type": "string", "description": "HTTP method (default: GET)" }
            },
            "required": ["url"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .post(format!("{}/api/scripts/match-preview", api_url))
            .json(arguments)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

pub struct GetScriptHistoryAllTool;

#[async_trait::async_trait]
impl McpTool for GetScriptHistoryAllTool {
    fn name(&self) -> &str {
        "madhyamas_get_script_history_all"
    }
    fn description(&self) -> &str {
        "Get execution history across all scripts, showing recent runs \
         with success/failure status, duration, and errors."
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
            .get(format!("{}/api/scripts/history", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

pub struct ClearScriptHistoryTool;

#[async_trait::async_trait]
impl McpTool for ClearScriptHistoryTool {
    fn name(&self) -> &str {
        "madhyamas_clear_script_history"
    }
    fn description(&self) -> &str {
        "Clear execution history for a specific script."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Script ID" }
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
            .delete(format!("{}/api/scripts/{}/history", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, &format!("History cleared for script {}", id)).await
    }
}

pub struct GetScriptConfigTool;

#[async_trait::async_trait]
impl McpTool for GetScriptConfigTool {
    fn name(&self) -> &str {
        "madhyamas_get_script_config"
    }
    fn description(&self) -> &str {
        "Get the global script runtime configuration (timeout, memory \
         limit, console capture settings)."
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
            .get(format!("{}/api/scripts/config", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

pub struct UpdateScriptConfigTool;

#[async_trait::async_trait]
impl McpTool for UpdateScriptConfigTool {
    fn name(&self) -> &str {
        "madhyamas_update_script_config"
    }
    fn description(&self) -> &str {
        "Update the global script runtime configuration (timeout, memory \
         limit, console capture). Only provided fields are updated."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "timeout_ms": { "type": "integer", "description": "Execution timeout in milliseconds" },
                "memory_limit_mb": { "type": "integer", "description": "Memory limit in MB" },
                "capture_console": { "type": "boolean", "description": "Enable console output capture" }
            }
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .put(format!("{}/api/scripts/config", api_url))
            .json(arguments)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

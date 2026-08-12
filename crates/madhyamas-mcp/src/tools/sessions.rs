//! Session management tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// List all sessions
pub(super) async fn list_sessions(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/sessions", api_url);

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

    let sessions: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(sessions)
}

/// Create a new session
pub(super) async fn create_session(
    client: &Client,
    api_url: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/sessions", api_url);

    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = Value::String(n.to_string());
    }
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
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

    let session: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(session)
}

/// Get a specific session
#[allow(dead_code)]
pub(super) async fn get_session(
    client: &Client,
    api_url: &str,
    session_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/sessions/{}", api_url, session_id);

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

    let session: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(session)
}

/// Delete a session
#[allow(dead_code)]
pub(super) async fn delete_session(
    client: &Client,
    api_url: &str,
    session_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/sessions/{}", api_url, session_id);

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
        "message": "Session deleted"
    }))
}

/// Export a session
#[allow(dead_code)]
pub(super) async fn export_session(
    client: &Client,
    api_url: &str,
    session_id: &str,
    format: Option<&str>,
) -> Result<Value, McpError> {
    let fmt = format.unwrap_or("har");
    let url = format!(
        "{}/api/sessions/{}/export?format={}",
        api_url, session_id, fmt
    );

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

    let exported: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(exported)
}

/// Import a session
#[allow(dead_code)]
pub(super) async fn import_session(
    client: &Client,
    api_url: &str,
    session_data: Value,
) -> Result<Value, McpError> {
    let url = format!("{}/api/sessions/import", api_url);

    let response = client
        .post(&url)
        .json(&session_data)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let session: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(session)
}

/// Switch active session
pub(super) async fn switch_session(
    client: &Client,
    api_url: &str,
    session_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/sessions/{}/switch", api_url, session_id);

    let response = client
        .post(&url)
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
        "message": format!("Switched to session {}", session_id)
    }))
}

// ============ Trait-based tool structs ============

/// List all capture sessions.
pub struct ListSessionsTool;

#[async_trait::async_trait]
impl McpTool for ListSessionsTool {
    fn name(&self) -> &str {
        "madhyamas_list_sessions"
    }

    fn description(&self) -> &str {
        "List all debugging sessions."
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
        let sessions = list_sessions(client, api_url).await?;
        Ok(vec![ContentBlock::Text {
            text: format_sessions(&sessions),
        }])
    }
}

fn format_sessions(sessions: &Value) -> String {
    let arr = match sessions.as_array() {
        Some(a) => a,
        None => return "No sessions found.".to_string(),
    };

    if arr.is_empty() {
        return "No sessions found.".to_string();
    }

    let mut out = format!("Found {} session(s):\n\n", arr.len());
    for s in arr {
        let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = s
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("(unnamed)");
        let count = s.get("request_count").and_then(|v| v.as_u64()).unwrap_or(0);
        out.push_str(&format!("  • {} — {} ({} requests)\n", id, name, count));
    }
    out
}

/// Create a new debugging session.
pub struct CreateSessionTool;

#[async_trait::async_trait]
impl McpTool for CreateSessionTool {
    fn name(&self) -> &str {
        "madhyamas_create_session"
    }

    fn description(&self) -> &str {
        "Create a new debugging session. Sessions help organize captured traffic."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the session"
                },
                "description": {
                    "type": "string",
                    "description": "Description of the session"
                }
            }
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
            .map(|s| s.to_string());
        let description = arguments
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let result =
            create_session(client, api_url, name.as_deref(), description.as_deref()).await?;
        Ok(json_text(&result))
    }
}

/// Switch the active debugging session.
pub struct SwitchSessionTool;

#[async_trait::async_trait]
impl McpTool for SwitchSessionTool {
    fn name(&self) -> &str {
        "madhyamas_switch_session"
    }

    fn description(&self) -> &str {
        "Switch the active debugging session."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the session to switch to"
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
        let result = switch_session(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

/// Export a session in HAR or cURL format.
pub struct ExportSessionTool;

#[async_trait::async_trait]
impl McpTool for ExportSessionTool {
    fn name(&self) -> &str {
        "madhyamas_export_session"
    }

    fn description(&self) -> &str {
        "Export a session in HAR or cURL format for sharing or backup."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the session to export"
                },
                "format": {
                    "type": "string",
                    "enum": ["har", "curl"],
                    "description": "Export format (default: har)"
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
        let format = arguments
            .get("format")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let result = export_session(client, api_url, &id, format.as_deref()).await?;
        Ok(json_text(&result))
    }
}

/// Import a session from HAR format or previously exported data.
pub struct ImportSessionTool;

#[async_trait::async_trait]
impl McpTool for ImportSessionTool {
    fn name(&self) -> &str {
        "madhyamas_import_session"
    }

    fn description(&self) -> &str {
        "Import a session from HAR format or previously exported data."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "session_data": {
                    "type": "object",
                    "description": "The session data to import (HAR format or Madhyamas export)"
                }
            },
            "required": ["session_data"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let session_data = arguments
            .get("session_data")
            .ok_or_else(|| McpError::InvalidParams("session_data is required".to_string()))?
            .clone();
        let result = import_session(client, api_url, session_data).await?;
        Ok(json_text(&result))
    }
}

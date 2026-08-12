//! Mock response tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, api_result_void, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============
// Internal helpers (existing free functions, kept as pub(super))
// ============

/// Create a simple mock rule (legacy API)
#[allow(clippy::too_many_arguments)]
pub(super) async fn create_mock(
    client: &Client,
    api_url: &str,
    url_pattern: &str,
    method: Option<&str>,
    status_code: Option<u16>,
    headers: Option<Value>,
    body: Option<Value>,
    delay_ms: Option<u64>,
    enabled: Option<bool>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks", api_url);

    // Build the condition from URL pattern
    let condition = if let Some(m) = method {
        json!({
            "type": "and",
            "conditions": [
                {"type": "url_pattern", "pattern": url_pattern},
                {"type": "method", "method": m}
            ]
        })
    } else {
        json!({
            "type": "url_pattern",
            "pattern": url_pattern
        })
    };

    // Build the mock response
    let mut response = json!({
        "status_code": status_code.unwrap_or(200),
        "headers": headers.unwrap_or(json!({})),
    });
    if let Some(b) = body {
        response["body"] = b;
    }
    if let Some(d) = delay_ms {
        response["delay_ms"] = Value::Number(d.into());
    }

    let mock_config = json!({
        "name": url_pattern,
        "condition": condition,
        "response": response,
        "enabled": enabled.unwrap_or(true),
    });

    let resp = client
        .post(&url)
        .json(&mock_config)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let mock: Value = resp
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(mock)
}

/// Create an advanced mock rule with full configuration
#[allow(clippy::too_many_arguments)]
pub(super) async fn create_advanced_mock(
    client: &Client,
    api_url: &str,
    name: &str,
    condition: Value,
    response_config: Value,
    description: Option<&str>,
    tags: Option<Vec<String>>,
    collection_id: Option<&str>,
    enabled: Option<bool>,
    priority: Option<u32>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/advanced", api_url);

    let mut mock_config = json!({
        "name": name,
        "condition": condition,
        "response_config": response_config,
    });

    if let Some(desc) = description {
        mock_config["description"] = Value::String(desc.to_string());
    }
    if let Some(t) = tags {
        mock_config["tags"] = Value::Array(t.into_iter().map(Value::String).collect());
    }
    if let Some(cid) = collection_id {
        mock_config["collection_id"] = Value::String(cid.to_string());
    }
    if let Some(e) = enabled {
        mock_config["enabled"] = Value::Bool(e);
    }
    if let Some(p) = priority {
        mock_config["priority"] = Value::Number(p.into());
    }

    let response = client
        .post(&url)
        .json(&mock_config)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let mock: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(mock)
}

/// Update an existing mock rule
pub(super) async fn update_mock(
    client: &Client,
    api_url: &str,
    mock_id: &str,
    mock: Value,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}", api_url, mock_id);

    let response = client
        .put(&url)
        .json(&mock)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({ "success": true, "message": "Mock updated" }))
}

/// List all mock rules
pub(super) async fn list_mocks(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks", api_url);

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

    let mocks: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(mocks)
}

/// Get a specific mock rule
pub(super) async fn get_mock(
    client: &Client,
    api_url: &str,
    mock_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}", api_url, mock_id);

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

    let mock: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(mock)
}

/// Delete a mock rule
pub(super) async fn delete_mock(
    client: &Client,
    api_url: &str,
    mock_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}", api_url, mock_id);

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

    Ok(json!({ "success": true, "message": "Mock deleted" }))
}

/// Toggle a mock rule on/off
pub(super) async fn toggle_mock(
    client: &Client,
    api_url: &str,
    mock_id: &str,
    enabled: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}/toggle", api_url, mock_id);

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

    let mock: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(json!({
        "success": true,
        "message": format!("Mock rule {}", if enabled { "enabled" } else { "disabled" }),
        "mock": mock
    }))
}

/// Get mock templates (predefined mock responses)
#[allow(dead_code)]
pub(super) async fn get_mock_templates(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/templates", api_url);

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

// ============================================================================
// Mock Collections
// ============================================================================

/// List all mock collections
pub(super) async fn list_collections(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/collections", api_url);

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

    let collections: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(collections)
}

/// Create a mock collection
pub(super) async fn create_collection(
    client: &Client,
    api_url: &str,
    name: &str,
    description: Option<&str>,
    tags: Option<Vec<String>>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/collections", api_url);

    let mut collection = json!({ "name": name });
    if let Some(desc) = description {
        collection["description"] = Value::String(desc.to_string());
    }
    if let Some(t) = tags {
        collection["tags"] = Value::Array(t.into_iter().map(Value::String).collect());
    }

    let response = client
        .post(&url)
        .json(&collection)
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

/// Delete a mock collection
pub(super) async fn delete_collection(
    client: &Client,
    api_url: &str,
    collection_id: &str,
    delete_rules: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/collections/{}", api_url, collection_id);

    let response = client
        .delete(&url)
        .json(&json!({ "delete_rules": delete_rules }))
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({ "success": true, "message": "Collection deleted" }))
}

/// Toggle a mock collection (enable/disable all rules)
pub(super) async fn toggle_collection(
    client: &Client,
    api_url: &str,
    collection_id: &str,
    enabled: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/collections/{}/toggle", api_url, collection_id);

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

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(result)
}

// ============================================================================
// Mock Analytics
// ============================================================================

/// Get hit analytics for all mocks
pub(super) async fn get_analytics(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/analytics", api_url);

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

    let analytics: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(analytics)
}

/// Get hit history for a specific mock
pub(super) async fn get_hit_history(
    client: &Client,
    api_url: &str,
    mock_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}/analytics/history", api_url, mock_id);

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

// ============================================================================
// Mock Testing & Preview
// ============================================================================

/// Test a mock rule against a sample request
pub(super) async fn test_mock(
    client: &Client,
    api_url: &str,
    mock_id: &str,
    request: Value,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}/test", api_url, mock_id);

    let response = client
        .post(&url)
        .json(&json!({ "request": request }))
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

/// Preview which mock would match a request
pub(super) async fn preview_mock_match(
    client: &Client,
    api_url: &str,
    request: Value,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/preview", api_url);

    let response = client
        .post(&url)
        .json(&json!({ "request": request }))
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

// ============================================================================
// Mock Import/Export
// ============================================================================

/// Export all mocks
pub(super) async fn export_mocks(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/export", api_url);

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

    let mocks: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(mocks)
}

/// Import mocks from HAR, OpenAPI, or Postman format
pub(super) async fn import_mocks(
    client: &Client,
    api_url: &str,
    format: &str,
    data: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/import", api_url);

    let response = client
        .post(&url)
        .json(&json!({
            "format": format,
            "data": data
        }))
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

// ============================================================================
// Mock Recording
// ============================================================================

/// Set mock recording mode
pub(super) async fn set_recording(
    client: &Client,
    api_url: &str,
    enabled: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/recording", api_url);

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

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(result)
}

/// Get recording status
pub(super) async fn get_recording_status(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/recording", api_url);

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

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(result)
}

/// Get recorded mocks
pub(super) async fn get_recorded_mocks(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/recording/recorded", api_url);

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

    let mocks: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(mocks)
}

/// Promote recorded mocks to active rules
pub(super) async fn promote_recorded_mocks(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/recording/promote", api_url);

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

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(result)
}

// ============================================================================
// Mock Versioning & Duplication
// ============================================================================

/// Duplicate a mock rule
pub(super) async fn duplicate_mock(
    client: &Client,
    api_url: &str,
    mock_id: &str,
    new_name: Option<&str>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}/duplicate", api_url, mock_id);

    let mut body = json!({});
    if let Some(name) = new_name {
        body["new_name"] = Value::String(name.to_string());
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

/// Rollback a mock rule to a previous version
pub(super) async fn rollback_mock(
    client: &Client,
    api_url: &str,
    mock_id: &str,
    version: u32,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}/rollback", api_url, mock_id);

    let response = client
        .post(&url)
        .json(&json!({ "version": version }))
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({ "success": true, "message": format!("Rolled back to version {}", version) }))
}

/// Get version history for a mock rule
pub(super) async fn get_mock_versions(
    client: &Client,
    api_url: &str,
    mock_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/mocks/{}/versions", api_url, mock_id);

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

    let versions: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(versions)
}

// ============================================================================
// Trait-based tool structs
// ============================================================================

/// List all mock rules currently configured.
pub struct ListMocksTool;

#[async_trait::async_trait]
impl McpTool for ListMocksTool {
    fn name(&self) -> &str {
        "madhyamas_list_mocks"
    }
    fn description(&self) -> &str {
        "List all mock rules currently configured."
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
        let result = list_mocks(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Create a mock rule to intercept and replace responses.
pub struct CreateMockTool;

#[async_trait::async_trait]
impl McpTool for CreateMockTool {
    fn name(&self) -> &str {
        "madhyamas_create_mock"
    }
    fn description(&self) -> &str {
        "Create a mock rule to intercept and replace responses. Useful for testing error handling, edge cases, or offline development."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url_pattern": {
                    "type": "string",
                    "description": "URL pattern to match (supports wildcards and regex)"
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                    "description": "HTTP method to match"
                },
                "status_code": {
                    "type": "integer",
                    "description": "HTTP status code to return (default: 200)"
                },
                "headers": {
                    "type": "object",
                    "description": "Response headers to return"
                },
                "body": {
                    "description": "Response body to return"
                },
                "delay_ms": {
                    "type": "integer",
                    "description": "Optional delay before responding (for testing slow connections)"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the mock is enabled immediately (default: true)"
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
        let status_code = arguments
            .get("status_code")
            .and_then(|v| v.as_u64())
            .map(|v| v as u16);
        let headers = arguments.get("headers").cloned();
        let body = arguments.get("body").cloned();
        let delay_ms = arguments.get("delay_ms").and_then(|v| v.as_u64());
        let enabled = arguments.get("enabled").and_then(|v| v.as_bool());
        let result = create_mock(
            client,
            api_url,
            url_pattern,
            method,
            status_code,
            headers,
            body,
            delay_ms,
            Some(enabled.unwrap_or(true)),
        )
        .await?;
        Ok(json_text(&result))
    }
}

/// Delete a mock rule.
pub struct DeleteMockTool;

#[async_trait::async_trait]
impl McpTool for DeleteMockTool {
    fn name(&self) -> &str {
        "madhyamas_delete_mock"
    }
    fn description(&self) -> &str {
        "Delete a mock rule."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule to delete"
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
        let result = delete_mock(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

/// Enable or disable a mock rule.
pub struct ToggleMockTool;

#[async_trait::async_trait]
impl McpTool for ToggleMockTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_mock"
    }
    fn description(&self) -> &str {
        "Enable or disable a mock rule."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule to toggle"
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
        let result = toggle_mock(client, api_url, &id, enabled).await?;
        Ok(json_text(&result))
    }
}

/// Create an advanced mock rule with full configuration.
pub struct CreateAdvancedMockTool;

#[async_trait::async_trait]
impl McpTool for CreateAdvancedMockTool {
    fn name(&self) -> &str {
        "madhyamas_create_advanced_mock"
    }
    fn description(&self) -> &str {
        "Create an advanced mock rule with full configuration including response sequences, conditional responses, or probabilistic responses. Use this for complex mocking scenarios."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the mock rule"
                },
                "condition": {
                    "type": "object",
                    "description": "Match condition (e.g., {\"type\": \"url_pattern\", \"pattern\": \"https://api.example.com/.*\"})"
                },
                "response_config": {
                    "type": "object",
                    "description": "Response configuration. Can be: Single {\"type\": \"single\", \"response\": {...}}, Sequence {\"type\": \"sequence\", \"responses\": [...]}, Conditional {\"type\": \"conditional\", \"conditions\": [...]}, or Probabilistic {\"type\": \"probabilistic\", \"responses\": [...]}"
                },
                "description": {
                    "type": "string",
                    "description": "Optional description/documentation"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tags for organization"
                },
                "collection_id": {
                    "type": "string",
                    "description": "Collection to add this mock to"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the mock is enabled (default: true)"
                },
                "priority": {
                    "type": "integer",
                    "description": "Priority (lower = higher priority, default: 100)"
                }
            },
            "required": ["name", "condition", "response_config"]
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
        let condition = arguments
            .get("condition")
            .cloned()
            .ok_or_else(|| McpError::InvalidParams("condition is required".to_string()))?;
        let response_config = arguments
            .get("response_config")
            .cloned()
            .ok_or_else(|| McpError::InvalidParams("response_config is required".to_string()))?;
        let description = arguments.get("description").and_then(|v| v.as_str());
        let tags = arguments.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });
        let collection_id = arguments.get("collection_id").and_then(|v| v.as_str());
        let enabled = arguments.get("enabled").and_then(|v| v.as_bool());
        let priority = arguments
            .get("priority")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let result = create_advanced_mock(
            client,
            api_url,
            name,
            condition,
            response_config,
            description,
            tags,
            collection_id,
            enabled,
            priority,
        )
        .await?;
        Ok(json_text(&result))
    }
}

/// Update an existing mock rule with new configuration.
pub struct UpdateMockTool;

#[async_trait::async_trait]
impl McpTool for UpdateMockTool {
    fn name(&self) -> &str {
        "madhyamas_update_mock"
    }
    fn description(&self) -> &str {
        "Update an existing mock rule with new configuration."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule to update"
                },
                "mock": {
                    "type": "object",
                    "description": "The full mock rule object to update"
                }
            },
            "required": ["id", "mock"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let mock = arguments
            .get("mock")
            .cloned()
            .ok_or_else(|| McpError::InvalidParams("mock is required".to_string()))?;
        let result = update_mock(client, api_url, &id, mock).await?;
        Ok(json_text(&result))
    }
}

/// Get details of a specific mock rule.
pub struct GetMockTool;

#[async_trait::async_trait]
impl McpTool for GetMockTool {
    fn name(&self) -> &str {
        "madhyamas_get_mock"
    }
    fn description(&self) -> &str {
        "Get details of a specific mock rule."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule to retrieve"
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
        let result = get_mock(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

/// Duplicate an existing mock rule with a new name.
pub struct DuplicateMockTool;

#[async_trait::async_trait]
impl McpTool for DuplicateMockTool {
    fn name(&self) -> &str {
        "madhyamas_duplicate_mock"
    }
    fn description(&self) -> &str {
        "Duplicate an existing mock rule with a new name."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule to duplicate"
                },
                "new_name": {
                    "type": "string",
                    "description": "Optional new name for the duplicate"
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
        let new_name = arguments.get("new_name").and_then(|v| v.as_str());
        let result = duplicate_mock(client, api_url, &id, new_name).await?;
        Ok(json_text(&result))
    }
}

/// Rollback a mock rule to a previous version.
pub struct RollbackMockTool;

#[async_trait::async_trait]
impl McpTool for RollbackMockTool {
    fn name(&self) -> &str {
        "madhyamas_rollback_mock"
    }
    fn description(&self) -> &str {
        "Rollback a mock rule to a previous version."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule to rollback"
                },
                "version": {
                    "type": "integer",
                    "description": "The version number to rollback to"
                }
            },
            "required": ["id", "version"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let version = arguments
            .get("version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| McpError::InvalidParams("version is required".to_string()))?
            as u32;
        let result = rollback_mock(client, api_url, &id, version).await?;
        Ok(json_text(&result))
    }
}

/// Get version history for a mock rule.
pub struct GetMockVersionsTool;

#[async_trait::async_trait]
impl McpTool for GetMockVersionsTool {
    fn name(&self) -> &str {
        "madhyamas_get_mock_versions"
    }
    fn description(&self) -> &str {
        "Get version history for a mock rule."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule"
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
        let result = get_mock_versions(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

/// List all mock collections.
pub struct ListMockCollectionsTool;

#[async_trait::async_trait]
impl McpTool for ListMockCollectionsTool {
    fn name(&self) -> &str {
        "madhyamas_list_mock_collections"
    }
    fn description(&self) -> &str {
        "List all mock collections. Collections help organize related mock rules."
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
        let result = list_collections(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Create a new mock collection for organizing related mock rules.
pub struct CreateMockCollectionTool;

#[async_trait::async_trait]
impl McpTool for CreateMockCollectionTool {
    fn name(&self) -> &str {
        "madhyamas_create_mock_collection"
    }
    fn description(&self) -> &str {
        "Create a new mock collection for organizing related mock rules."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the collection"
                },
                "description": {
                    "type": "string",
                    "description": "Optional description"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tags for the collection"
                }
            },
            "required": ["name"]
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
        let description = arguments.get("description").and_then(|v| v.as_str());
        let tags = arguments.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });
        let result = create_collection(client, api_url, name, description, tags).await?;
        Ok(json_text(&result))
    }
}

/// Delete a mock collection.
pub struct DeleteMockCollectionTool;

#[async_trait::async_trait]
impl McpTool for DeleteMockCollectionTool {
    fn name(&self) -> &str {
        "madhyamas_delete_mock_collection"
    }
    fn description(&self) -> &str {
        "Delete a mock collection. Optionally delete all rules in the collection."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the collection to delete"
                },
                "delete_rules": {
                    "type": "boolean",
                    "description": "Whether to also delete all rules in this collection (default: false)"
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
        let delete_rules = arguments
            .get("delete_rules")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let result = delete_collection(client, api_url, &id, delete_rules).await?;
        Ok(json_text(&result))
    }
}

/// Enable or disable all mock rules in a collection.
pub struct ToggleMockCollectionTool;

#[async_trait::async_trait]
impl McpTool for ToggleMockCollectionTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_mock_collection"
    }
    fn description(&self) -> &str {
        "Enable or disable all mock rules in a collection."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the collection to toggle"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "true to enable all, false to disable all"
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
        let result = toggle_collection(client, api_url, &id, enabled).await?;
        Ok(json_text(&result))
    }
}

/// Get details of a specific mock collection by ID.
pub struct GetMockCollectionTool;

#[async_trait::async_trait]
impl McpTool for GetMockCollectionTool {
    fn name(&self) -> &str {
        "madhyamas_get_mock_collection"
    }
    fn description(&self) -> &str {
        "Get details of a specific mock collection by ID."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the collection to retrieve"
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
            .get(format!("{}/api/mocks/collections/{}", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Update a mock collection's metadata (name, description, enabled, tags).
///
/// Only the provided fields are updated; omitted fields retain their
/// existing values.
pub struct UpdateMockCollectionTool;

#[async_trait::async_trait]
impl McpTool for UpdateMockCollectionTool {
    fn name(&self) -> &str {
        "madhyamas_update_mock_collection"
    }
    fn description(&self) -> &str {
        "Update a mock collection's metadata. Only provided fields are updated; omitted fields retain their existing values."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the collection to update"
                },
                "name": {
                    "type": "string",
                    "description": "New name for the collection"
                },
                "description": {
                    "type": "string",
                    "description": "New description for the collection"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether the collection is enabled"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tags for the collection"
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
        let mut body = json!({});
        if let Some(name) = arguments.get("name").and_then(|v| v.as_str()) {
            body["name"] = Value::String(name.to_string());
        }
        if let Some(desc) = arguments.get("description").and_then(|v| v.as_str()) {
            body["description"] = Value::String(desc.to_string());
        }
        if let Some(enabled) = arguments.get("enabled").and_then(|v| v.as_bool()) {
            body["enabled"] = Value::Bool(enabled);
        }
        if let Some(tags) = arguments.get("tags").and_then(|v| v.as_array()) {
            body["tags"] = Value::Array(
                tags.iter()
                    .filter_map(|v| v.as_str().map(|s| Value::String(s.to_string())))
                    .collect(),
            );
        }
        let resp = client
            .put(format!("{}/api/mocks/collections/{}", api_url, id))
            .json(&body)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get hit analytics for all mock rules.
pub struct GetMockAnalyticsTool;

#[async_trait::async_trait]
impl McpTool for GetMockAnalyticsTool {
    fn name(&self) -> &str {
        "madhyamas_get_mock_analytics"
    }
    fn description(&self) -> &str {
        "Get hit analytics for all mock rules, including hit counts and history."
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
        let result = get_analytics(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Get detailed hit history for a specific mock rule.
pub struct GetMockHitHistoryTool;

#[async_trait::async_trait]
impl McpTool for GetMockHitHistoryTool {
    fn name(&self) -> &str {
        "madhyamas_get_mock_hit_history"
    }
    fn description(&self) -> &str {
        "Get detailed hit history for a specific mock rule."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule"
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
        let result = get_hit_history(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

/// Test a mock rule against a sample request.
pub struct TestMockTool;

#[async_trait::async_trait]
impl McpTool for TestMockTool {
    fn name(&self) -> &str {
        "madhyamas_test_mock"
    }
    fn description(&self) -> &str {
        "Test a mock rule against a sample request to see if it matches and what response would be returned."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the mock rule to test"
                },
                "request": {
                    "type": "object",
                    "description": "Sample request data with url, method, headers, body"
                }
            },
            "required": ["id", "request"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let request = arguments
            .get("request")
            .cloned()
            .ok_or_else(|| McpError::InvalidParams("request is required".to_string()))?;
        let result = test_mock(client, api_url, &id, request).await?;
        Ok(json_text(&result))
    }
}

/// Preview which mock rule would match a given request.
pub struct PreviewMockMatchTool;

#[async_trait::async_trait]
impl McpTool for PreviewMockMatchTool {
    fn name(&self) -> &str {
        "madhyamas_preview_mock_match"
    }
    fn description(&self) -> &str {
        "Preview which mock rule would match a given request without actually intercepting traffic."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "request": {
                    "type": "object",
                    "description": "Request data to test against all mocks"
                }
            },
            "required": ["request"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let request = arguments
            .get("request")
            .cloned()
            .ok_or_else(|| McpError::InvalidParams("request is required".to_string()))?;
        let result = preview_mock_match(client, api_url, request).await?;
        Ok(json_text(&result))
    }
}

/// Export all mock rules as JSON for backup or sharing.
pub struct ExportMocksTool;

#[async_trait::async_trait]
impl McpTool for ExportMocksTool {
    fn name(&self) -> &str {
        "madhyamas_export_mocks"
    }
    fn description(&self) -> &str {
        "Export all mock rules as JSON for backup or sharing."
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
        let result = export_mocks(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Import mock rules from HAR, OpenAPI, or Postman format.
pub struct ImportMocksTool;

#[async_trait::async_trait]
impl McpTool for ImportMocksTool {
    fn name(&self) -> &str {
        "madhyamas_import_mocks"
    }
    fn description(&self) -> &str {
        "Import mock rules from HAR, OpenAPI, or Postman format."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["har", "openapi", "postman"],
                    "description": "Import format"
                },
                "data": {
                    "type": "string",
                    "description": "The data to import (JSON string)"
                }
            },
            "required": ["format", "data"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let format = arguments
            .get("format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("format is required".to_string()))?;
        let data = arguments
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("data is required".to_string()))?;
        let result = import_mocks(client, api_url, format, data).await?;
        Ok(json_text(&result))
    }
}

/// Enable or disable mock recording mode.
pub struct SetMockRecordingTool;

#[async_trait::async_trait]
impl McpTool for SetMockRecordingTool {
    fn name(&self) -> &str {
        "madhyamas_set_mock_recording"
    }
    fn description(&self) -> &str {
        "Enable or disable mock recording mode. When enabled, responses are captured as potential mock rules."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "true to enable recording, false to disable"
                }
            },
            "required": ["enabled"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let enabled = arguments
            .get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| McpError::InvalidParams("enabled is required".to_string()))?;
        let result = set_recording(client, api_url, enabled).await?;
        Ok(json_text(&result))
    }
}

/// Get current mock recording status.
pub struct GetMockRecordingStatusTool;

#[async_trait::async_trait]
impl McpTool for GetMockRecordingStatusTool {
    fn name(&self) -> &str {
        "madhyamas_get_mock_recording_status"
    }
    fn description(&self) -> &str {
        "Get current mock recording status."
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
        let result = get_recording_status(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Get all mock rules that have been recorded from live traffic.
pub struct GetRecordedMocksTool;

#[async_trait::async_trait]
impl McpTool for GetRecordedMocksTool {
    fn name(&self) -> &str {
        "madhyamas_get_recorded_mocks"
    }
    fn description(&self) -> &str {
        "Get all mock rules that have been recorded from live traffic."
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
        let result = get_recorded_mocks(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Promote all recorded mocks to active mock rules.
pub struct PromoteRecordedMocksTool;

#[async_trait::async_trait]
impl McpTool for PromoteRecordedMocksTool {
    fn name(&self) -> &str {
        "madhyamas_promote_recorded_mocks"
    }
    fn description(&self) -> &str {
        "Promote all recorded mocks to active mock rules."
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
        let result = promote_recorded_mocks(client, api_url).await?;
        Ok(json_text(&result))
    }
}

// ============================================================================
// Tools merged from modern_tools/mocks.rs (direct HTTP calls via helpers)
// ============================================================================

/// List available predefined mock templates for quick creation.
pub struct GetMockTemplatesTool;

#[async_trait::async_trait]
impl McpTool for GetMockTemplatesTool {
    fn name(&self) -> &str {
        "madhyamas_get_mock_templates"
    }
    fn description(&self) -> &str {
        "List available predefined mock templates for quick creation."
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
            .get(format!("{}/api/mocks/templates", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Enable or disable multiple mock rules in a single request.
pub struct BatchToggleMocksTool;

#[async_trait::async_trait]
impl McpTool for BatchToggleMocksTool {
    fn name(&self) -> &str {
        "madhyamas_batch_toggle_mocks"
    }
    fn description(&self) -> &str {
        "Enable or disable multiple mock rules in a single request."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of mock rule IDs to toggle"
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
            .post(format!("{}/api/mocks/batch-toggle", api_url))
            .json(arguments)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Clear all recorded mock candidates.
pub struct ClearMockRecordingTool;

#[async_trait::async_trait]
impl McpTool for ClearMockRecordingTool {
    fn name(&self) -> &str {
        "madhyamas_clear_mock_recording"
    }
    fn description(&self) -> &str {
        "Clear all mock candidates that have been recorded from live traffic."
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
            .post(format!("{}/api/mocks/recording/clear", api_url))
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, "Mock recording cleared").await
    }
}

/// Clear all mock hit history.
pub struct ClearMockHistoryTool;

#[async_trait::async_trait]
impl McpTool for ClearMockHistoryTool {
    fn name(&self) -> &str {
        "madhyamas_clear_mock_history"
    }
    fn description(&self) -> &str {
        "Clear all mock hit history and analytics data."
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
            .post(format!("{}/api/mocks/history/clear", api_url))
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, "Mock hit history cleared").await
    }
}

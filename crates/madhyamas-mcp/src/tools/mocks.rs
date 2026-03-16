//! Mock response tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// Create a simple mock rule (legacy API)
#[allow(clippy::too_many_arguments)]
pub async fn create_mock(
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
pub async fn create_advanced_mock(
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
pub async fn update_mock(
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
pub async fn list_mocks(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn get_mock(client: &Client, api_url: &str, mock_id: &str) -> Result<Value, McpError> {
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
pub async fn delete_mock(client: &Client, api_url: &str, mock_id: &str) -> Result<Value, McpError> {
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
pub async fn toggle_mock(
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
pub async fn get_mock_templates(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn list_collections(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn create_collection(
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
pub async fn delete_collection(
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
pub async fn toggle_collection(
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
pub async fn get_analytics(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn get_hit_history(
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
pub async fn test_mock(
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
pub async fn preview_mock_match(
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
pub async fn export_mocks(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn import_mocks(
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
pub async fn set_recording(
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
pub async fn get_recording_status(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn get_recorded_mocks(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn promote_recorded_mocks(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn duplicate_mock(
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
pub async fn rollback_mock(
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
pub async fn get_mock_versions(
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

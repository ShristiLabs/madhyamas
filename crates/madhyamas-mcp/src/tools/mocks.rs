//! Mock response tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// Create a mock rule
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

    let mut mock_config = json!({
        "url_pattern": url_pattern,
    });

    if let Some(m) = method {
        mock_config["method"] = Value::String(m.to_string());
    }
    if let Some(s) = status_code {
        mock_config["status_code"] = Value::Number(s.into());
    }
    if let Some(h) = headers {
        mock_config["headers"] = h;
    }
    if let Some(b) = body {
        mock_config["body"] = b;
    }
    if let Some(d) = delay_ms {
        mock_config["delay_ms"] = Value::Number(d.into());
    }
    if let Some(e) = enabled {
        mock_config["enabled"] = Value::Bool(e);
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

/// List all mock rules
pub async fn list_mocks(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
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
pub async fn get_mock(
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
pub async fn delete_mock(
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
pub async fn get_mock_templates(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
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

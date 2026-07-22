//! Rewrite rule tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// List all rewrite rules
pub async fn list_rewrites(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn create_rewrite(
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
pub async fn delete_rewrite(
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
pub async fn toggle_rewrite(
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
pub async fn get_rewrite_templates(client: &Client, api_url: &str) -> Result<Value, McpError> {
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

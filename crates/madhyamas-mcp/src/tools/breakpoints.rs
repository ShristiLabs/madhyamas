//! Breakpoint tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// Create a breakpoint rule
pub async fn create_breakpoint(
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
pub async fn list_breakpoints(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn delete_breakpoint(
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
pub async fn get_paused_traffic(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn resume_paused_traffic(
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

//! Throttle tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// Get the current throttle profile
pub async fn get_throttle(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/throttle", api_url);

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

    let profile: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(profile)
}

/// Set the throttle profile
pub async fn set_throttle(
    client: &Client,
    api_url: &str,
    profile: Value,
    enabled: Option<bool>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/throttle", api_url);

    let mut body = json!({ "profile": profile });
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

    Ok(json!({
        "success": true,
        "message": "Throttle profile set"
    }))
}

/// Enable or disable throttling
pub async fn set_throttle_enabled(
    client: &Client,
    api_url: &str,
    enabled: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/throttle/enabled", api_url);

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
        "message": format!("Throttling {}", if enabled { "enabled" } else { "disabled" })
    }))
}

/// Get available throttle presets
pub async fn get_throttle_presets(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/throttle/presets", api_url);

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

    let presets: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(presets)
}

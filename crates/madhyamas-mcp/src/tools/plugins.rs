//! Plugin tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// List all plugins
pub async fn list_plugins(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins", api_url);

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

    let plugins: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(plugins)
}

/// Get a specific plugin
pub async fn get_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}", api_url, plugin_id);

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

    let plugin: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(plugin)
}

/// Enable a plugin
pub async fn enable_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/enable", api_url, plugin_id);

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
        "message": format!("Plugin {} enabled", plugin_id)
    }))
}

/// Disable a plugin
pub async fn disable_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/disable", api_url, plugin_id);

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
        "message": format!("Plugin {} disabled", plugin_id)
    }))
}

/// Get statistics for a plugin
pub async fn get_plugin_stats(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/stats", api_url, plugin_id);

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

    let stats: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(stats)
}

/// Reload all plugins
pub async fn reload_plugins(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/reload", api_url);

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
        "message": "Plugins reloaded"
    }))
}

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

/// Install a plugin from a URL or registry id.
pub async fn install_plugin(
    client: &Client,
    api_url: &str,
    source: &str,
    target: &str,
    checksum: Option<&str>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/install", api_url);
    let body = match source {
        "registry" => json!({ "source": "registry", "id": target }),
        _ => json!({ "source": "url", "url": target, "checksum": checksum }),
    };

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

/// Uninstall a plugin.
pub async fn uninstall_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/uninstall", api_url, plugin_id);

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
        "message": format!("Plugin {} uninstalled", plugin_id)
    }))
}

/// Search the plugin registry.
pub async fn search_registry(
    client: &Client,
    api_url: &str,
    query: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/registry/search?q={}", api_url, query);

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

    let results: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(results)
}

/// List all registry entries.
pub async fn list_registry(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/registry", api_url);

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

    let entries: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(entries)
}

/// Get a plugin's settings schema.
pub async fn get_plugin_schema(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/schema", api_url, plugin_id);

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

    let schema: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(schema)
}

/// Get a plugin's current settings.
pub async fn get_plugin_settings(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/settings", api_url, plugin_id);

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

    let settings: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(settings)
}

/// Update a plugin's settings.
pub async fn update_plugin_settings(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
    settings: Value,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/settings", api_url, plugin_id);

    let response = client
        .put(&url)
        .json(&settings)
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
        "message": format!("Settings updated for plugin {}", plugin_id)
    }))
}

/// Get a plugin's recent invocation logs.
pub async fn get_plugin_logs(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
    limit: u32,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/logs?limit={}", api_url, plugin_id, limit);

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

    let logs: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(logs)
}

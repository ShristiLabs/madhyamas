//! Script tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// List all scripts
pub async fn list_scripts(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn create_script(
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
pub async fn get_script(
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
pub async fn update_script(
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
pub async fn delete_script(
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
pub async fn toggle_script(
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
pub async fn get_script_templates(client: &Client, api_url: &str) -> Result<Value, McpError> {
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

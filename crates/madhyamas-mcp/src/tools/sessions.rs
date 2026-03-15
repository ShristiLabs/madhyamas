//! Session management tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// List all sessions
pub async fn list_sessions(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub async fn create_session(
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
pub async fn get_session(
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
pub async fn delete_session(
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
pub async fn export_session(
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
pub async fn import_session(
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
pub async fn switch_session(
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

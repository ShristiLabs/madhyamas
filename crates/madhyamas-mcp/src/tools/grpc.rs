//! gRPC inspection tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// Get gRPC connections
pub async fn get_grpc_connections(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/grpc/connections", api_url);

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

    let connections: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(connections)
}

/// Get gRPC streams
pub async fn get_grpc_streams(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/grpc/streams", api_url);

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

    let streams: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(streams)
}

/// Get gRPC frames, optionally filtered
pub async fn get_grpc_frames(
    client: &Client,
    api_url: &str,
    filter: Option<&str>,
) -> Result<Value, McpError> {
    let mut url = format!("{}/api/grpc/frames", api_url);
    if let Some(f) = filter {
        url.push_str(&format!("?filter={}", f));
    }

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

    let frames: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(frames)
}

/// Get gRPC statistics
pub async fn get_grpc_stats(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/grpc/stats", api_url);

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

/// Clear all captured gRPC frames
pub async fn clear_grpc_frames(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/grpc/clear", api_url);

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
        "message": "gRPC frames cleared"
    }))
}

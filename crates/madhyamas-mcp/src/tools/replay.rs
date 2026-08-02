//! Replay tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// Replay a captured request
pub async fn replay_request(
    client: &Client,
    api_url: &str,
    traffic_id: &str,
    modifications: Option<Value>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/replay/execute/{}", api_url, traffic_id);

    let body = modifications.unwrap_or(json!({}));

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

/// Replay a saved request multiple times with concurrency, iterations, and
/// delay (the "Repeat Advanced" / batch replay feature).
pub async fn replay_request_advanced(
    client: &Client,
    api_url: &str,
    traffic_id: &str,
    modifications: Option<Value>,
    iterations: usize,
    concurrency: usize,
    delay_ms: Option<u64>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/replay/execute/{}/batch", api_url, traffic_id);

    let mut config = json!({
        "iterations": iterations,
        "concurrency": concurrency,
    });
    if let Some(delay) = delay_ms {
        config["delay_ms"] = Value::Number(delay.into());
    }

    let mut body = json!({ "config": config });
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

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(result)
}

/// Save a request for later replay
pub async fn save_request(
    client: &Client,
    api_url: &str,
    traffic_id: &str,
    name: Option<&str>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/replay/saved", api_url);

    let mut body = json!({
        "traffic_id": traffic_id,
    });
    if let Some(n) = name {
        body["name"] = Value::String(n.to_string());
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

    let saved: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(saved)
}

/// List saved requests
pub async fn list_saved_requests(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/replay/saved", api_url);

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

    let saved: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(saved)
}

/// Delete a saved request
#[allow(dead_code)]
pub async fn delete_saved_request(
    client: &Client,
    api_url: &str,
    request_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/replay/saved/{}", api_url, request_id);

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
        "message": "Saved request deleted"
    }))
}

/// Get replay history
#[allow(dead_code)]
pub async fn get_replay_history(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/replay/history", api_url);

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

/// Export request as cURL command
pub async fn export_curl(
    client: &Client,
    api_url: &str,
    traffic_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/export/curl/{}", api_url, traffic_id);

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

    let curl: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(curl)
}

/// Format replay result for AI analysis
pub fn format_replay_result(result: &Value) -> String {
    let mut output = String::new();
    output.push_str("# Replay Result\n\n");

    if let Some(obj) = result.as_object() {
        if let Some(status) = obj.get("status_code").and_then(|v| v.as_u64()) {
            output.push_str(&format!("**Status**: {}\n", status));
        }
        if let Some(duration) = obj.get("duration_ms").and_then(|v| v.as_u64()) {
            output.push_str(&format!("**Duration**: {}ms\n", duration));
        }
        if let Some(headers) = obj.get("response_headers") {
            output.push_str("\n**Response Headers**:\n");
            output.push_str(&serde_json::to_string_pretty(headers).unwrap_or_default());
            output.push('\n');
        }
        if let Some(body) = obj.get("response_body") {
            output.push_str("\n**Response Body**:\n");
            output.push_str(&serde_json::to_string_pretty(body).unwrap_or_default());
        }
    }

    output
}

/// Format a batch replay result for AI analysis
pub fn format_batch_replay_result(result: &Value) -> String {
    let mut output = String::new();
    output.push_str("# Batch Replay Result\n\n");

    if let Some(obj) = result.as_object() {
        if let Some(total) = obj.get("total").and_then(|v| v.as_u64()) {
            output.push_str(&format!("**Total requests**: {}\n", total));
        }
        if let Some(succeeded) = obj.get("succeeded").and_then(|v| v.as_u64()) {
            output.push_str(&format!("**Succeeded**: {}\n", succeeded));
        }
        if let Some(failed) = obj.get("failed").and_then(|v| v.as_u64()) {
            output.push_str(&format!("**Failed**: {}\n", failed));
        }
        output.push_str("\n**Latency Statistics (ms)**:\n");
        if let Some(min) = obj.get("min_ms").and_then(|v| v.as_u64()) {
            output.push_str(&format!("- min: {}\n", min));
        }
        if let Some(avg) = obj.get("avg_ms").and_then(|v| v.as_u64()) {
            output.push_str(&format!("- avg: {}\n", avg));
        }
        if let Some(max) = obj.get("max_ms").and_then(|v| v.as_u64()) {
            output.push_str(&format!("- max: {}\n", max));
        }
        if let Some(p95) = obj.get("p95_ms").and_then(|v| v.as_u64()) {
            output.push_str(&format!("- p95: {}\n", p95));
        }
    }

    output
}

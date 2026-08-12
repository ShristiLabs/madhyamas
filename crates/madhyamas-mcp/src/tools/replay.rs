//! Replay tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result_void, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// Replay a captured request
pub(super) async fn replay_request(
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
pub(super) async fn replay_request_advanced(
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
pub(super) async fn save_request(
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
pub(super) async fn list_saved_requests(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub(super) async fn delete_saved_request(
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
pub(super) async fn get_replay_history(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub(super) async fn export_curl(
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
pub(super) fn format_replay_result(result: &Value) -> String {
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
pub(super) fn format_batch_replay_result(result: &Value) -> String {
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

// ============ Trait-based tool structs ============

/// Replay a saved request with optional edit-then-repeat.
pub struct ReplayRequestTool;

#[async_trait::async_trait]
impl McpTool for ReplayRequestTool {
    fn name(&self) -> &str {
        "madhyamas_replay_request"
    }

    fn description(&self) -> &str {
        "Replay a saved request with optional edit-then-repeat. Supports modifying the URL, method, headers, body, and redirect behavior before replaying. Useful for debugging, testing different scenarios, or re-running requests with modified payloads."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the saved request to replay"
                },
                "modifications": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "Override the request URL"
                        },
                        "method": {
                            "type": "string",
                            "description": "Override the HTTP method (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)"
                        },
                        "headers": {
                            "type": "object",
                            "description": "Headers to add or replace (key-value pairs)"
                        },
                        "remove_headers": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Header names to remove from the request"
                        },
                        "body": {
                            "type": "string",
                            "description": "New request body (raw text)"
                        },
                        "follow_redirects": {
                            "type": "boolean",
                            "description": "Whether to follow 3xx redirect responses (default: false)"
                        }
                    },
                    "description": "Optional modifications to apply before replaying (edit-then-repeat)"
                }
            },
            "required": ["id"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let modifications = arguments.get("modifications").cloned();
        let result = replay_request(client, api_url, &id, modifications).await?;
        Ok(vec![ContentBlock::Text {
            text: format_replay_result(&result),
        }])
    }
}

/// Replay a saved request multiple times with concurrency, iterations, and delay.
pub struct ReplayAdvancedTool;

#[async_trait::async_trait]
impl McpTool for ReplayAdvancedTool {
    fn name(&self) -> &str {
        "madhyamas_replay_advanced"
    }

    fn description(&self) -> &str {
        "Replay a saved request multiple times with concurrency, iterations, and inter-request delay (batch/advanced replay). Returns aggregate statistics including success/failure counts and latency percentiles (min/avg/max/p95). Useful for basic load testing and performance benchmarking. Safety limits: iterations capped at 10,000 and concurrency at 100."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the saved request to replay"
                },
                "iterations": {
                    "type": "integer",
                    "description": "Total number of requests to send (max 10,000, default: 1)",
                    "minimum": 1
                },
                "concurrency": {
                    "type": "integer",
                    "description": "Number of simultaneous in-flight requests (max 100, default: 1)",
                    "minimum": 1
                },
                "delay_ms": {
                    "type": "integer",
                    "description": "Optional delay between requests in milliseconds",
                    "minimum": 0
                },
                "modifications": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "Override the request URL"
                        },
                        "method": {
                            "type": "string",
                            "description": "Override the HTTP method (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)"
                        },
                        "headers": {
                            "type": "object",
                            "description": "Headers to add or replace (key-value pairs)"
                        },
                        "remove_headers": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Header names to remove from the request"
                        },
                        "body": {
                            "type": "string",
                            "description": "New request body (raw text)"
                        },
                        "follow_redirects": {
                            "type": "boolean",
                            "description": "Whether to follow 3xx redirect responses (default: false)"
                        }
                    },
                    "description": "Optional modifications to apply before replaying (applied to all iterations)"
                }
            },
            "required": ["id", "iterations", "concurrency"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let iterations = arguments
            .get("iterations")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| McpError::InvalidParams("iterations is required".to_string()))?
            as usize;
        let concurrency = arguments
            .get("concurrency")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| McpError::InvalidParams("concurrency is required".to_string()))?
            as usize;
        let delay_ms = arguments.get("delay_ms").and_then(|v| v.as_u64());
        let modifications = arguments.get("modifications").cloned();
        let result = replay_request_advanced(
            client,
            api_url,
            &id,
            modifications,
            iterations,
            concurrency,
            delay_ms,
        )
        .await?;
        Ok(vec![ContentBlock::Text {
            text: format_batch_replay_result(&result),
        }])
    }
}

/// Save a request for later replay.
pub struct SaveRequestTool;

#[async_trait::async_trait]
impl McpTool for SaveRequestTool {
    fn name(&self) -> &str {
        "madhyamas_save_request"
    }

    fn description(&self) -> &str {
        "Save a request for later replay. Useful for creating a library of test requests."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "traffic_id": {
                    "type": "string",
                    "description": "The ID of the traffic entry to save"
                },
                "name": {
                    "type": "string",
                    "description": "Optional name for the saved request"
                }
            },
            "required": ["traffic_id"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let traffic_id = arguments
            .get("traffic_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("traffic_id is required".to_string()))?;
        let name = arguments.get("name").and_then(|v| v.as_str());
        let result = save_request(client, api_url, traffic_id, name).await?;
        Ok(json_text(&result))
    }
}

/// List all saved requests available for replay.
pub struct ListSavedRequestsTool;

#[async_trait::async_trait]
impl McpTool for ListSavedRequestsTool {
    fn name(&self) -> &str {
        "madhyamas_list_saved_requests"
    }

    fn description(&self) -> &str {
        "List all saved requests available for replay."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let result = list_saved_requests(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Export a specific request as a cURL command.
pub struct ExportCurlTool;

#[async_trait::async_trait]
impl McpTool for ExportCurlTool {
    fn name(&self) -> &str {
        "madhyamas_export_curl"
    }

    fn description(&self) -> &str {
        "Export a specific request as a cURL command. Useful for reproducing API calls in a terminal."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the traffic entry to export as cURL"
                }
            },
            "required": ["id"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = export_curl(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

/// Clear all replay history.
pub struct ClearReplayHistoryTool;

#[async_trait::async_trait]
impl McpTool for ClearReplayHistoryTool {
    fn name(&self) -> &str {
        "madhyamas_clear_replay_history"
    }

    fn description(&self) -> &str {
        "Clear all replay history entries."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .delete(format!("{}/api/replay/history", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result_void(resp, "Replay history cleared").await
    }
}

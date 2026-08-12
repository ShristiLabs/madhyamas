//! gRPC inspection tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::json_text;
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// Get gRPC connections
pub(super) async fn get_grpc_connections(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
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
pub(super) async fn get_grpc_streams(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub(super) async fn get_grpc_frames(
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
pub(super) async fn get_grpc_stats(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub(super) async fn clear_grpc_frames(client: &Client, api_url: &str) -> Result<Value, McpError> {
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

// ============ Trait-based tool structs ============

#[derive(Debug, Clone, serde::Deserialize)]
struct GrpcFramesArgs {
    #[serde(default)]
    filter: Option<String>,
}

pub struct GetGrpcConnectionsTool;

#[async_trait::async_trait]
impl McpTool for GetGrpcConnectionsTool {
    fn name(&self) -> &str {
        "madhyamas_get_grpc_connections"
    }

    fn description(&self) -> &str {
        "List all captured gRPC connections."
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
        let result = get_grpc_connections(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct GetGrpcStreamsTool;

#[async_trait::async_trait]
impl McpTool for GetGrpcStreamsTool {
    fn name(&self) -> &str {
        "madhyamas_get_grpc_streams"
    }

    fn description(&self) -> &str {
        "List all gRPC streams observed by the proxy."
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
        let result = get_grpc_streams(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct GetGrpcFramesTool;

#[async_trait::async_trait]
impl McpTool for GetGrpcFramesTool {
    fn name(&self) -> &str {
        "madhyamas_get_grpc_frames"
    }

    fn description(&self) -> &str {
        "Get captured gRPC frames, optionally filtered."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "Optional filter expression for frames"
                }
            }
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let args: GrpcFramesArgs = serde_json::from_value(arguments.clone())
            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
        let result = get_grpc_frames(client, api_url, args.filter.as_deref()).await?;
        Ok(json_text(&result))
    }
}

pub struct GetGrpcStatsTool;

#[async_trait::async_trait]
impl McpTool for GetGrpcStatsTool {
    fn name(&self) -> &str {
        "madhyamas_get_grpc_stats"
    }

    fn description(&self) -> &str {
        "Get aggregated gRPC traffic statistics."
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
        let result = get_grpc_stats(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct ClearGrpcTool;

#[async_trait::async_trait]
impl McpTool for ClearGrpcTool {
    fn name(&self) -> &str {
        "madhyamas_clear_grpc"
    }

    fn description(&self) -> &str {
        "Clear all captured gRPC frames and reset statistics."
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
        let result = clear_grpc_frames(client, api_url).await?;
        Ok(json_text(&result))
    }
}

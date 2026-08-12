//! Throttle tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::json_text;
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// Get the current throttle profile
pub(super) async fn get_throttle(client: &Client, api_url: &str) -> Result<Value, McpError> {
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
pub(super) async fn set_throttle(
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
pub(super) async fn set_throttle_enabled(
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
pub(super) async fn get_throttle_presets(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
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

// ============ Trait-based tool structs ============

#[derive(Debug, Clone, serde::Deserialize)]
struct SetThrottleArgs {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    download_bps: Option<u64>,
    #[serde(default)]
    upload_bps: Option<u64>,
    #[serde(default)]
    delay_ms: Option<u64>,
    #[serde(default)]
    jitter_ms: Option<u64>,
    #[serde(default)]
    packet_loss_percent: Option<u8>,
    #[serde(default)]
    enabled: Option<bool>,
}

pub struct GetThrottleTool;

#[async_trait::async_trait]
impl McpTool for GetThrottleTool {
    fn name(&self) -> &str {
        "madhyamas_get_throttle"
    }

    fn description(&self) -> &str {
        "Get the current network throttle profile, including download/upload bandwidth limits, latency, jitter, and packet loss."
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
        let result = get_throttle(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct SetThrottleTool;

#[async_trait::async_trait]
impl McpTool for SetThrottleTool {
    fn name(&self) -> &str {
        "madhyamas_set_throttle"
    }

    fn description(&self) -> &str {
        "Set a custom network throttle profile to simulate slow or unreliable network conditions. Optionally enable/disable throttling at the same time."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "download_bps": {
                    "type": "integer",
                    "description": "Download bandwidth in bytes per second (0 = unlimited)"
                },
                "upload_bps": {
                    "type": "integer",
                    "description": "Upload bandwidth in bytes per second (0 = unlimited)"
                },
                "delay_ms": {
                    "type": "integer",
                    "description": "Latency in milliseconds"
                },
                "jitter_ms": {
                    "type": "integer",
                    "description": "Jitter in milliseconds"
                },
                "packet_loss_percent": {
                    "type": "integer",
                    "description": "Packet loss percentage (0-100)"
                },
                "name": {
                    "type": "string",
                    "description": "Profile name"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Whether to enable throttling immediately"
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
        let args: SetThrottleArgs = serde_json::from_value(arguments.clone())
            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
        let profile = json!({
            "name": args.name.unwrap_or_else(|| "Custom".to_string()),
            "download_bps": args.download_bps.unwrap_or(0),
            "upload_bps": args.upload_bps.unwrap_or(0),
            "latency_ms": args.delay_ms.unwrap_or(0),
            "jitter_ms": args.jitter_ms.unwrap_or(0),
            "packet_loss_percent": args.packet_loss_percent.unwrap_or(0),
        });
        let result = set_throttle(client, api_url, profile, args.enabled).await?;
        Ok(json_text(&result))
    }
}

pub struct ToggleThrottleTool;

#[async_trait::async_trait]
impl McpTool for ToggleThrottleTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_throttle"
    }

    fn description(&self) -> &str {
        "Enable or disable network throttling without changing the active profile."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "true to enable throttling, false to disable"
                }
            },
            "required": ["enabled"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let enabled = arguments
            .get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| McpError::InvalidParams("enabled is required".to_string()))?;
        let result = set_throttle_enabled(client, api_url, enabled).await?;
        Ok(json_text(&result))
    }
}

pub struct GetThrottlePresetsTool;

#[async_trait::async_trait]
impl McpTool for GetThrottlePresetsTool {
    fn name(&self) -> &str {
        "madhyamas_get_throttle_presets"
    }

    fn description(&self) -> &str {
        "List available predefined throttle profiles (e.g., GPRS, EDGE, 3G, 4G LTE) for quick network simulation."
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
        let result = get_throttle_presets(client, api_url).await?;
        Ok(json_text(&result))
    }
}

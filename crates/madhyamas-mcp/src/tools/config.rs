//! Proxy configuration and capture mode MCP tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::json_text;
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

/// Get the current proxy configuration.
pub struct GetConfigTool;

#[async_trait::async_trait]
impl McpTool for GetConfigTool {
    fn name(&self) -> &str {
        "madhyamas_get_config"
    }

    fn description(&self) -> &str {
        "Get current Madhyamas configuration including proxy port, API port, \
         host, HTTPS interception status, and max requests."
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
        let url = format!("{}/api/config", api_url);
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        let config: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(json_text(&config))
    }
}

/// Update runtime proxy configuration.
pub struct UpdateConfigTool;

#[async_trait::async_trait]
impl McpTool for UpdateConfigTool {
    fn name(&self) -> &str {
        "madhyamas_update_config"
    }

    fn description(&self) -> &str {
        "Update runtime Madhyamas configuration. Only specified fields will \
         be updated."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "intercept_https": {
                    "type": "boolean",
                    "description": "Enable or disable HTTPS interception"
                },
                "max_requests": {
                    "type": "integer",
                    "description": "Maximum number of requests to keep in memory"
                },
                "verbose": {
                    "type": "boolean",
                    "description": "Enable or disable verbose logging"
                },
                "public_ip": {
                    "type": ["string", "null"],
                    "description": "Public IP address to display (null to auto-detect)"
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
        let url = format!("{}/api/config", api_url);

        let mut payload = serde_json::Map::new();
        if let Some(intercept) = arguments.get("intercept_https").and_then(|v| v.as_bool()) {
            payload.insert("intercept_https".to_string(), Value::Bool(intercept));
        }
        if let Some(max_req) = arguments.get("max_requests").and_then(|v| v.as_u64()) {
            payload.insert("max_requests".to_string(), Value::Number(max_req.into()));
        }
        if let Some(verbose) = arguments.get("verbose").and_then(|v| v.as_bool()) {
            payload.insert("verbose".to_string(), Value::Bool(verbose));
        }
        if let Some(ip) = arguments.get("public_ip") {
            if !ip.is_null() {
                payload.insert("public_ip".to_string(), ip.clone());
            }
        }

        let resp = client
            .patch(&url)
            .json(&Value::Object(payload))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        let result: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(json_text(&result))
    }
}

/// Get current capture mode status.
pub struct GetCaptureStatusTool;

#[async_trait::async_trait]
impl McpTool for GetCaptureStatusTool {
    fn name(&self) -> &str {
        "madhyamas_get_capture_status"
    }

    fn description(&self) -> &str {
        "Get current capture mode status. Returns whether traffic is being \
         recorded (recording mode) or just forwarded without recording \
         (passthrough mode)."
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
        let url = format!("{}/api/capture", api_url);
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        let result: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(json_text(&result))
    }
}

/// Toggle capture mode between recording and passthrough.
pub struct ToggleCaptureTool;

#[async_trait::async_trait]
impl McpTool for ToggleCaptureTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_capture"
    }

    fn description(&self) -> &str {
        "Toggle capture mode between recording and passthrough. In \
         passthrough mode, the proxy forwards traffic but does not record \
         it to the database."
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
        let url = format!("{}/api/capture/toggle", api_url);
        let resp = client
            .post(&url)
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        let result: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(json_text(&result))
    }
}

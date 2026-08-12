//! Mirror tool MCP tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

/// Get the current mirror status and statistics.
pub struct GetMirrorStatusTool;

#[async_trait::async_trait]
impl McpTool for GetMirrorStatusTool {
    fn name(&self) -> &str {
        "madhyamas_get_mirror_status"
    }

    fn description(&self) -> &str {
        "Get the current mirror tool status, configuration, and statistics \
         (files written, bytes written). The mirror tool saves response \
         bodies to disk following the URL path structure."
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
        let url = format!("{}/api/mirror", api_url);
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

        let status: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(vec![ContentBlock::Text {
            text: format_mirror_status(&status),
        }])
    }
}

/// Toggle mirroring on or off.
pub struct ToggleMirrorTool;

#[async_trait::async_trait]
impl McpTool for ToggleMirrorTool {
    fn name(&self) -> &str {
        "madhyamas_toggle_mirror"
    }

    fn description(&self) -> &str {
        "Toggle the mirror tool on or off. When enabled, response bodies are \
         saved to disk following the URL path structure."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "Whether to enable (true) or disable (false) mirroring"
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

        let url = format!("{}/api/mirror/toggle", api_url);
        let resp = client
            .post(&url)
            .json(&json!({ "enabled": enabled }))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        Ok(vec![ContentBlock::Text {
            text: format!("Mirror {}", if enabled { "enabled" } else { "disabled" }),
        }])
    }
}

/// Update the mirror configuration.
pub struct UpdateMirrorConfigTool;

#[async_trait::async_trait]
impl McpTool for UpdateMirrorConfigTool {
    fn name(&self) -> &str {
        "madhyamas_update_mirror_config"
    }

    fn description(&self) -> &str {
        "Update the mirror tool configuration (output directory, host \
         filter, save request bodies). Only provided fields are updated."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "Enable or disable mirroring"
                },
                "output_dir": {
                    "type": "string",
                    "description": "Directory where mirrored files are written"
                },
                "host_filter": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Host patterns to mirror (empty or null for all hosts)"
                },
                "save_request_bodies": {
                    "type": "boolean",
                    "description": "Whether to also save request bodies"
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
        let url = format!("{}/api/mirror/config", api_url);
        let resp = client
            .patch(&url)
            .json(arguments)
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

        Ok(vec![ContentBlock::Text {
            text: format_mirror_status(&config),
        }])
    }
}

fn format_mirror_status(status: &Value) -> String {
    let enabled = status
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let output_dir = status
        .get("output_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("(default)");
    let files = status
        .get("files_written")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let bytes = status
        .get("bytes_written")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut out = format!("Mirror: {}\n", if enabled { "enabled" } else { "disabled" });
    out.push_str(&format!("  Output dir: {}\n", output_dir));
    out.push_str(&format!("  Files written: {}\n", files));
    out.push_str(&format!("  Bytes written: {}\n", bytes));

    if let Some(filter) = status.get("host_filter").and_then(|v| v.as_array()) {
        if !filter.is_empty() {
            let patterns: Vec<&str> = filter.iter().filter_map(|v| v.as_str()).collect();
            out.push_str(&format!("  Host filter: {}\n", patterns.join(", ")));
        } else {
            out.push_str("  Host filter: (all hosts)\n");
        }
    }

    let save_req = status
        .get("save_request_bodies")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    out.push_str(&format!(
        "  Save request bodies: {}",
        if save_req { "yes" } else { "no" }
    ));

    out
}

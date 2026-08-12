//! Log rotation MCP tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

/// Get the current log rotation status (config, current file, archived files).
pub struct GetLogStatusTool;

#[async_trait::async_trait]
impl McpTool for GetLogStatusTool {
    fn name(&self) -> &str {
        "madhyamas_get_log_status"
    }

    fn description(&self) -> &str {
        "Get the current log rotation status: configuration, current log \
         file path and size, and the list of archived (rotated) log files."
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
        let url = format!("{}/api/logs", api_url);
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
            text: format_log_status(&status),
        }])
    }
}

/// Trigger an immediate (on-demand) log file rotation.
pub struct RotateLogsTool;

#[async_trait::async_trait]
impl McpTool for RotateLogsTool {
    fn name(&self) -> &str {
        "madhyamas_rotate_logs"
    }

    fn description(&self) -> &str {
        "Rotate the current log file immediately (on-demand). The current \
         madhyamas.log is renamed with a timestamp suffix and a fresh file \
         is opened. Archived files are pruned to max_files."
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
        let url = format!("{}/api/logs/rotate", api_url);
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

        let rotated_to = result
            .get("rotated_to")
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown)");
        Ok(vec![ContentBlock::Text {
            text: format!(
                "Log rotated to: {}\n\n{}",
                rotated_to,
                format_log_status(&result)
            ),
        }])
    }
}

/// Update the log rotation configuration.
pub struct UpdateLogConfigTool;

#[async_trait::async_trait]
impl McpTool for UpdateLogConfigTool {
    fn name(&self) -> &str {
        "madhyamas_update_log_config"
    }

    fn description(&self) -> &str {
        "Update the log rotation configuration (enabled, rotation mode, \
         max_files, max_file_size_mb, json_format). Only provided fields \
         are updated. Rotation mode changes take effect on next restart; \
         size/max_files take effect immediately."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "description": "Enable or disable file logging"
                },
                "rotation": {
                    "type": "object",
                    "description": "Rotation mode: {\"mode\": \"never\"|\"hourly\"|\"daily\"} or {\"mode\": \"size\", \"size_mb\": <n>}",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["never", "hourly", "daily", "size"],
                            "description": "Rotation mode"
                        },
                        "size_mb": {
                            "type": "integer",
                            "description": "Size in MB (required when mode=size)"
                        }
                    }
                },
                "max_files": {
                    "type": "integer",
                    "description": "Maximum number of archived log files to keep"
                },
                "max_file_size_mb": {
                    "type": "integer",
                    "description": "Hard per-file size cap in MB (safety net for time-based rotation)"
                },
                "json_format": {
                    "type": "boolean",
                    "description": "Use structured JSON log format (takes effect on restart)"
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
        let url = format!("{}/api/logs", api_url);
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

        let result: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(vec![ContentBlock::Text {
            text: format_log_config_result(&result),
        }])
    }
}

fn format_log_status(status: &Value) -> String {
    let enabled = status
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let rotation = status
        .get("rotation")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let max_files = status
        .get("max_files")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let max_size = status
        .get("max_file_size_mb")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let json_fmt = status
        .get("json_format")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let log_dir = status
        .get("log_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("(default)");

    let mut out = format!(
        "Log rotation: {}\n",
        if enabled { "enabled" } else { "disabled" }
    );
    out.push_str(&format!("  Rotation:       {}\n", rotation));
    out.push_str(&format!("  Max files:      {}\n", max_files));
    out.push_str(&format!("  Max file size:  {} MB\n", max_size));
    out.push_str(&format!(
        "  JSON format:    {}\n",
        if json_fmt { "yes" } else { "no" }
    ));
    out.push_str(&format!("  Log dir:        {}\n", log_dir));

    if let Some(current) = status.get("current_file") {
        let path = current.get("path").and_then(|v| v.as_str()).unwrap_or("?");
        let size = current
            .get("size_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        out.push_str(&format!("\nCurrent file: {}\n", path));
        out.push_str(&format!("  Size: {}\n", format_log_bytes(size)));
    }

    if let Some(archived) = status.get("archived_files").and_then(|v| v.as_array()) {
        out.push_str(&format!("\nArchived files ({}):\n", archived.len()));
        if archived.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for f in archived {
                let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let size = f.get("size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                out.push_str(&format!("  {} ({})\n", name, format_log_bytes(size)));
            }
        }
    }

    out
}

fn format_log_config_result(result: &Value) -> String {
    let mut out = "Log configuration updated.\n".to_string();
    out.push_str(&format!(
        "  Enabled:       {}\n",
        if result
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(&format!(
        "  Rotation:      {}\n",
        result
            .get("rotation")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
    ));
    out.push_str(&format!(
        "  Max files:     {}\n",
        result
            .get("max_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "  Max file size: {} MB\n",
        result
            .get("max_file_size_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "  JSON format:   {}\n",
        if result
            .get("json_format")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            "yes"
        } else {
            "no"
        }
    ));
    if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
        out.push_str(&format!("\n{}\n", msg));
    }
    out
}

fn format_log_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

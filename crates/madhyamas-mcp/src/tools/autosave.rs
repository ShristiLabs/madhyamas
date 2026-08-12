//! Auto Save MCP tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::api_result;
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

/// Get Auto Save configuration.
pub struct GetAutoSaveConfigTool;

#[async_trait::async_trait]
impl McpTool for GetAutoSaveConfigTool {
    fn name(&self) -> &str {
        "madhyamas_get_autosave_config"
    }
    fn description(&self) -> &str {
        "Get the current Auto Save configuration (enabled, interval, \
         export format, output directory, max backups, rotation settings)."
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
            .get(format!("{}/api/autosave", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Update Auto Save configuration.
pub struct UpdateAutoSaveConfigTool;

#[async_trait::async_trait]
impl McpTool for UpdateAutoSaveConfigTool {
    fn name(&self) -> &str {
        "madhyamas_update_autosave_config"
    }
    fn description(&self) -> &str {
        "Update the Auto Save configuration. Only provided fields are \
         updated. Auto Save periodically exports the current session as \
         HAR or Session format to a backup directory."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean", "description": "Enable or disable Auto Save" },
                "interval_seconds": { "type": "integer", "description": "Seconds between snapshots" },
                "export_format": { "type": "string", "enum": ["har", "session"], "description": "Export format" },
                "output_dir": { "type": "string", "description": "Backup directory path" },
                "max_backups": { "type": "integer", "description": "Number of backups to keep" },
                "rotate_after_requests": { "type": "integer", "description": "Rotate session after N requests" },
                "rotate_after_minutes": { "type": "integer", "description": "Rotate session after N minutes" }
            }
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .patch(format!("{}/api/autosave", api_url))
            .json(arguments)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Trigger an immediate Auto Save snapshot.
pub struct TriggerAutoSaveSnapshotTool;

#[async_trait::async_trait]
impl McpTool for TriggerAutoSaveSnapshotTool {
    fn name(&self) -> &str {
        "madhyamas_trigger_autosave_snapshot"
    }
    fn description(&self) -> &str {
        "Trigger an immediate Auto Save snapshot (save now) without \
         waiting for the next interval."
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
            .post(format!("{}/api/autosave/snapshot", api_url))
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

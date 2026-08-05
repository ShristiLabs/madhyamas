//! Trait-based MCP tools.
//!
//! Each struct in this module implements [`super::McpTool`] and is
//! registered in [`default_registry`].  Adding a new tool is as simple as
//! defining a struct here and pushing it into the registry builder — no
//! edits to `registry.rs` or `executor.rs` are required.

use reqwest::Client;
use serde_json::{json, Value};

use super::tool_trait::{DynToolRegistry, McpTool};
use crate::types::{ContentBlock, McpError};

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

/// List all capture sessions.
pub struct ListSessionsTool;

#[async_trait::async_trait]
impl McpTool for ListSessionsTool {
    fn name(&self) -> &str {
        "madhyamas_list_sessions"
    }

    fn description(&self) -> &str {
        "List all capture sessions with their metadata (name, description, \
         request count, timestamps)."
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
        let url = format!("{}/api/sessions", api_url);
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

        let sessions: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(vec![ContentBlock::Text {
            text: format_sessions(&sessions),
        }])
    }
}

fn format_sessions(sessions: &Value) -> String {
    let arr = match sessions.as_array() {
        Some(a) => a,
        None => return "No sessions found.".to_string(),
    };

    if arr.is_empty() {
        return "No sessions found.".to_string();
    }

    let mut out = format!("Found {} session(s):\n\n", arr.len());
    for s in arr {
        let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = s
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("(unnamed)");
        let count = s.get("request_count").and_then(|v| v.as_u64()).unwrap_or(0);
        out.push_str(&format!("  • {} — {} ({} requests)\n", id, name, count));
    }
    out
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Get the current proxy configuration.
pub struct GetConfigTool;

#[async_trait::async_trait]
impl McpTool for GetConfigTool {
    fn name(&self) -> &str {
        "madhyamas_get_config"
    }

    fn description(&self) -> &str {
        "Get the current proxy configuration (ports, HTTPS interception, \
         body size limits, etc.)."
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

        Ok(vec![ContentBlock::Text {
            text: format!("{}", config),
        }])
    }
}

// ---------------------------------------------------------------------------
// Certificate
// ---------------------------------------------------------------------------

/// Get the CA certificate information for the proxy.
pub struct GetCertInfoTool;

#[async_trait::async_trait]
impl McpTool for GetCertInfoTool {
    fn name(&self) -> &str {
        "madhyamas_get_cert_info"
    }

    fn description(&self) -> &str {
        "Get the proxy's CA certificate details (subject, issuer, validity, \
         download URL) needed to configure browsers/clients for HTTPS \
         interception."
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
        let url = format!("{}/api/cert/ca", api_url);
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

        let cert: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(vec![ContentBlock::Text {
            text: format!(
                "CA Certificate:\n  Subject: {}\n  Issuer: {}\n  Valid: {} to {}\n  Download: {}/api/cert/ca\n",
                cert.get("subject").and_then(|v| v.as_str()).unwrap_or("?"),
                cert.get("issuer").and_then(|v| v.as_str()).unwrap_or("?"),
                cert.get("not_before").and_then(|v| v.as_str()).unwrap_or("?"),
                cert.get("not_after").and_then(|v| v.as_str()).unwrap_or("?"),
                api_url,
            ),
        }])
    }
}

// ---------------------------------------------------------------------------
// Registry builder
// ---------------------------------------------------------------------------

/// Build a [`DynToolRegistry`] pre-populated with all trait-based tools.
///
/// Call this once at server startup and merge the result with the legacy
/// static registry.
pub fn default_registry() -> DynToolRegistry {
    let mut reg = DynToolRegistry::new();
    reg.register(Box::new(ListSessionsTool));
    reg.register(Box::new(GetConfigTool));
    reg.register(Box::new(GetCertInfoTool));
    reg.register(Box::new(ListFocusHostsTool));
    reg.register(Box::new(AddFocusHostTool));
    reg.register(Box::new(RemoveFocusHostTool));
    reg.register(Box::new(ClearFocusHostsTool));
    reg.register(Box::new(GetMirrorStatusTool));
    reg.register(Box::new(ToggleMirrorTool));
    reg.register(Box::new(UpdateMirrorConfigTool));
    reg.register(Box::new(GetLogStatusTool));
    reg.register(Box::new(RotateLogsTool));
    reg.register(Box::new(UpdateLogConfigTool));
    reg
}

// ---------------------------------------------------------------------------
// Focus hosts
// ---------------------------------------------------------------------------

/// List all focus host patterns.
pub struct ListFocusHostsTool;

#[async_trait::async_trait]
impl McpTool for ListFocusHostsTool {
    fn name(&self) -> &str {
        "madhyamas_list_focus_hosts"
    }

    fn description(&self) -> &str {
        "List all focus host patterns. Focused hosts are highlighted in the \
         traffic view. Patterns support exact hostnames, wildcard subdomains \
         (*.example.com), and globs (*api*)."
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
        let url = format!("{}/api/focus", api_url);
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

        let hosts: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(vec![ContentBlock::Text {
            text: format_focus_hosts(&hosts),
        }])
    }
}

/// Add a focus host pattern.
pub struct AddFocusHostTool;

#[async_trait::async_trait]
impl McpTool for AddFocusHostTool {
    fn name(&self) -> &str {
        "madhyamas_add_focus_host"
    }

    fn description(&self) -> &str {
        "Add a focus host pattern to highlight matching traffic. Supports \
         exact hostnames (api.example.com), wildcard subdomains \
         (*.example.com), and globs (*api*)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Host pattern to focus on"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let pattern = arguments
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("pattern is required".to_string()))?;

        let url = format!("{}/api/focus", api_url);
        let resp = client
            .post(&url)
            .json(&json!({ "pattern": pattern }))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        let host: Value = resp
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(vec![ContentBlock::Text {
            text: format!(
                "Added focus host: {} (pattern: {})",
                host.get("id").and_then(|v| v.as_str()).unwrap_or("?"),
                host.get("pattern").and_then(|v| v.as_str()).unwrap_or("?"),
            ),
        }])
    }
}

/// Remove a focus host by ID.
pub struct RemoveFocusHostTool;

#[async_trait::async_trait]
impl McpTool for RemoveFocusHostTool {
    fn name(&self) -> &str {
        "madhyamas_remove_focus_host"
    }

    fn description(&self) -> &str {
        "Remove a focus host pattern by its ID."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Focus host ID to remove"
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
        let id = arguments
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("id is required".to_string()))?;

        let safe_id = super::sanitize_id(id)?;
        let url = format!("{}/api/focus/{}", api_url, safe_id);
        let resp = client
            .delete(&url)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        Ok(vec![ContentBlock::Text {
            text: format!("Removed focus host: {}", safe_id),
        }])
    }
}

/// Clear all focus hosts.
pub struct ClearFocusHostsTool;

#[async_trait::async_trait]
impl McpTool for ClearFocusHostsTool {
    fn name(&self) -> &str {
        "madhyamas_clear_focus_hosts"
    }

    fn description(&self) -> &str {
        "Clear all focus host patterns."
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
        let url = format!("{}/api/focus", api_url);
        let resp = client
            .delete(&url)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        Ok(vec![ContentBlock::Text {
            text: "Cleared all focus hosts.".to_string(),
        }])
    }
}

// ---------------------------------------------------------------------------
// Mirror tool
// ---------------------------------------------------------------------------

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

fn format_focus_hosts(hosts: &Value) -> String {
    let arr = match hosts.as_array() {
        Some(a) => a,
        None => return "No focus hosts found.".to_string(),
    };

    if arr.is_empty() {
        return "No focus hosts found.".to_string();
    }

    let mut out = format!("Found {} focus host(s):\n\n", arr.len());
    for h in arr {
        let id = h.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let pattern = h.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
        out.push_str(&format!("  • {} — {}\n", id, pattern));
    }
    out
}

// ---------------------------------------------------------------------------
// Log rotation
// ---------------------------------------------------------------------------

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

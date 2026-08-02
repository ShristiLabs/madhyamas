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

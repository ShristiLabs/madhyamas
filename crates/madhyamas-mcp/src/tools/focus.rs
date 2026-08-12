//! Focus host MCP tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

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

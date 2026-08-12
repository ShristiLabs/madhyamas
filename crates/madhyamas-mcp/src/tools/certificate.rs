//! Certificate information MCP tool.

use reqwest::Client;
use serde_json::{json, Value};

use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

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

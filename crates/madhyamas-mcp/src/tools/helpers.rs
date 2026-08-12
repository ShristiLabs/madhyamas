//! Shared helpers for MCP tool implementations.
//!
//! These utilities reduce boilerplate across tool implementations:
//! - [`get_id`] extracts and sanitizes an `id` field from tool arguments.
//! - [`api_result`] parses a successful HTTP response as JSON.
//! - [`api_result_void`] handles 204 No Content responses with a custom message.
//! - [`json_text`] wraps a `serde_json::Value` into a `ContentBlock::Text`.

use serde_json::Value;

use crate::types::{ContentBlock, McpError};

/// Extract and sanitize an `id` field from arguments.
pub(super) fn get_id(arguments: &Value) -> Result<String, McpError> {
    let id = arguments
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidParams("id is required".to_string()))?;
    super::sanitize_id(id)
}

/// Parse a successful response as JSON, or return an error.
pub(super) async fn api_result(resp: reqwest::Response) -> Result<Vec<ContentBlock>, McpError> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }
    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(vec![ContentBlock::Text {
            text: "OK".to_string(),
        }]);
    }
    let value: Value = resp
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(json_text(&value))
}

/// Handle a void response (204 No Content) with a custom success message.
pub(super) async fn api_result_void(
    resp: reqwest::Response,
    msg: &str,
) -> Result<Vec<ContentBlock>, McpError> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }
    Ok(vec![ContentBlock::Text {
        text: msg.to_string(),
    }])
}

/// Wrap a JSON `Value` into a `ContentBlock::Text` with pretty formatting.
pub(super) fn json_text(value: &Value) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: serde_json::to_string_pretty(value).unwrap_or_default(),
    }]
}

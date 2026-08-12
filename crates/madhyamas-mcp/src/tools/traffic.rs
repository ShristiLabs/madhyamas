//! Traffic inspection tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// Advanced traffic filter parameters
#[derive(Debug, Default)]
pub(super) struct TrafficFilter {
    pub filter: Option<String>,
    pub method: Option<String>,
    pub status: Option<u16>,
    pub file_type: Option<String>,
    pub header: Option<String>,
    pub cookie: Option<String>,
    pub search: Option<String>,
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
    pub min_time: Option<u64>,
    pub max_time: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Get captured traffic with advanced filtering
pub(super) async fn get_traffic_filtered(
    client: &Client,
    api_url: &str,
    filter: TrafficFilter,
) -> Result<Value, McpError> {
    let mut url = format!("{}/api/traffic", api_url);
    let mut params = Vec::new();

    if let Some(ref f) = filter.filter {
        params.push(("filter", f.clone()));
    }
    if let Some(ref m) = filter.method {
        params.push(("method", m.clone()));
    }
    if let Some(s) = filter.status {
        params.push(("status", s.to_string()));
    }
    if let Some(ref ft) = filter.file_type {
        params.push(("file_type", ft.clone()));
    }
    if let Some(ref h) = filter.header {
        params.push(("header", h.clone()));
    }
    if let Some(ref c) = filter.cookie {
        params.push(("cookie", c.clone()));
    }
    if let Some(ref s) = filter.search {
        params.push(("search", s.clone()));
    }
    if let Some(ms) = filter.min_size {
        params.push(("min_size", ms.to_string()));
    }
    if let Some(ms) = filter.max_size {
        params.push(("max_size", ms.to_string()));
    }
    if let Some(mt) = filter.min_time {
        params.push(("min_time", mt.to_string()));
    }
    if let Some(mt) = filter.max_time {
        params.push(("max_time", mt.to_string()));
    }
    if let Some(l) = filter.limit {
        params.push(("limit", l.to_string()));
    }
    if let Some(o) = filter.offset {
        params.push(("offset", o.to_string()));
    }

    if !params.is_empty() {
        url.push('?');
        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        url.push_str(&query);
    }

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

    let traffic: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(traffic)
}

/// Get a specific traffic entry
pub(super) async fn get_traffic_entry(
    client: &Client,
    api_url: &str,
    entry_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/traffic/{}", api_url, entry_id);

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

    let entry: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(entry)
}

/// Clear all traffic
pub(super) async fn clear_traffic(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/traffic/clear", api_url);

    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({ "success": true, "message": "Traffic cleared" }))
}

/// Import traffic from a HAR JSON document into a new session.
///
/// `har` is the full HAR object (`{ "log": { ... } }`). When `session_name`
/// is provided it is used as the new session's name; otherwise the server
/// defaults to `"Imported HAR"`. When `switch_session` is true the active
/// session is switched to the newly created one.
pub(super) async fn import_har(
    client: &Client,
    api_url: &str,
    har: Value,
    session_name: Option<&str>,
    switch_session: bool,
) -> Result<Value, McpError> {
    let url = format!("{}/api/traffic/import/har", api_url);

    let body = json!({
        "har": har,
        "session_name": session_name,
        "switch_session": switch_session,
    });

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

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(result)
}

/// Get traffic count
pub(super) async fn get_traffic_count(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/traffic/count", api_url);

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

    let count: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(count)
}

/// Search traffic by content
pub(super) async fn search_traffic(
    client: &Client,
    api_url: &str,
    query: &str,
) -> Result<Value, McpError> {
    let url = format!(
        "{}/api/traffic?search={}",
        api_url,
        urlencoding::encode(query)
    );

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

    let traffic: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(traffic)
}

/// Format traffic summary for AI analysis
pub(super) fn format_traffic_summary(traffic: &Value) -> String {
    let mut summary = String::new();
    summary.push_str("# Traffic Summary\n\n");

    if let Some(items) = traffic.as_array() {
        summary.push_str(&format!("Total requests: {}\n\n", items.len()));

        for (i, entry) in items.iter().enumerate() {
            if let Some(obj) = entry.as_object() {
                let method = obj.get("method").and_then(|v| v.as_str()).unwrap_or("?");
                let url = obj.get("url").and_then(|v| v.as_str()).unwrap_or("?");
                let status = obj.get("status_code").and_then(|v| v.as_u64()).unwrap_or(0);
                let duration = obj.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);

                summary.push_str(&format!(
                    "{}. **{}** {} - Status: {} ({}ms)\n",
                    i + 1,
                    method,
                    url,
                    status,
                    duration
                ));
            }
        }
    } else if let Some(obj) = traffic.as_object() {
        // Single entry
        if let Some(items) = obj.get("items").and_then(|v| v.as_array()) {
            summary.push_str(&format!("Total requests: {}\n\n", items.len()));

            for (i, entry) in items.iter().enumerate() {
                if let Some(obj) = entry.as_object() {
                    let method = obj.get("method").and_then(|v| v.as_str()).unwrap_or("?");
                    let url = obj.get("url").and_then(|v| v.as_str()).unwrap_or("?");
                    let status = obj.get("status_code").and_then(|v| v.as_u64()).unwrap_or(0);
                    let duration = obj.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);

                    summary.push_str(&format!(
                        "{}. **{}** {} - Status: {} ({}ms)\n",
                        i + 1,
                        method,
                        url,
                        status,
                        duration
                    ));
                }
            }
        }
    }

    summary
}

/// Format a single traffic entry for detailed analysis
pub(super) fn format_traffic_detail(entry: &Value) -> String {
    let mut detail = String::new();

    if let Some(obj) = entry.as_object() {
        detail.push_str("# Traffic Entry Details\n\n");

        // Request info
        detail.push_str("## Request\n");
        if let Some(method) = obj.get("method").and_then(|v| v.as_str()) {
            detail.push_str(&format!("- **Method**: {}\n", method));
        }
        if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
            detail.push_str(&format!("- **URL**: {}\n", url));
        }
        if let Some(headers) = obj.get("request_headers") {
            detail.push_str(&format!(
                "- **Headers**: ```json\n{}\n```\n",
                serde_json::to_string_pretty(headers).unwrap_or_default()
            ));
        }
        if let Some(body) = obj.get("request_body") {
            detail.push_str(&format!(
                "- **Body**: ```json\n{}\n```\n",
                serde_json::to_string_pretty(body).unwrap_or_default()
            ));
        }

        // Response info
        detail.push_str("\n## Response\n");
        if let Some(status) = obj.get("status_code").and_then(|v| v.as_u64()) {
            detail.push_str(&format!("- **Status**: {}\n", status));
        }
        if let Some(headers) = obj.get("response_headers") {
            detail.push_str(&format!(
                "- **Headers**: ```json\n{}\n```\n",
                serde_json::to_string_pretty(headers).unwrap_or_default()
            ));
        }
        if let Some(body) = obj.get("response_body") {
            detail.push_str(&format!(
                "- **Body**: ```json\n{}\n```\n",
                serde_json::to_string_pretty(body).unwrap_or_default()
            ));
        }

        // Timing
        if let Some(duration) = obj.get("duration_ms").and_then(|v| v.as_u64()) {
            detail.push_str(&format!("\n- **Duration**: {}ms\n", duration));
        }
    }

    detail
}

// ============ Trait-based tool structs ============

/// List captured HTTP traffic with advanced filtering.
pub struct GetTrafficTool;

#[async_trait::async_trait]
impl McpTool for GetTrafficTool {
    fn name(&self) -> &str {
        "madhyamas_get_traffic"
    }
    fn description(&self) -> &str {
        "List captured HTTP traffic with advanced filtering. Returns a summary of requests including method, URL, status code, and timing."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "Filter expression to match URLs (supports wildcards)"
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                    "description": "Filter by HTTP method"
                },
                "status": {
                    "type": "integer",
                    "description": "Filter by HTTP status code (e.g., 200, 404, 500)"
                },
                "file_type": {
                    "type": "string",
                    "description": "Filter by file type/extension (e.g., json, html, css, js, png)"
                },
                "header": {
                    "type": "string",
                    "description": "Filter by header (format: 'key:value' or just 'key')"
                },
                "cookie": {
                    "type": "string",
                    "description": "Filter by cookie (format: 'name=value' or just 'name')"
                },
                "search": {
                    "type": "string",
                    "description": "Search in request/response bodies"
                },
                "min_size": {
                    "type": "integer",
                    "description": "Filter by minimum response size in bytes"
                },
                "max_size": {
                    "type": "integer",
                    "description": "Filter by maximum response size in bytes"
                },
                "min_time": {
                    "type": "integer",
                    "description": "Filter by minimum response time in milliseconds"
                },
                "max_time": {
                    "type": "integer",
                    "description": "Filter by maximum response time in milliseconds"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Offset for pagination"
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
        let filter = TrafficFilter {
            filter: arguments
                .get("filter")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            method: arguments
                .get("method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            status: arguments
                .get("status")
                .and_then(|v| v.as_u64())
                .map(|s| s as u16),
            file_type: arguments
                .get("file_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            header: arguments
                .get("header")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            cookie: arguments
                .get("cookie")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            search: arguments
                .get("search")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            min_size: arguments
                .get("min_size")
                .and_then(|v| v.as_u64())
                .map(|s| s as usize),
            max_size: arguments
                .get("max_size")
                .and_then(|v| v.as_u64())
                .map(|s| s as usize),
            min_time: arguments.get("min_time").and_then(|v| v.as_u64()),
            max_time: arguments.get("max_time").and_then(|v| v.as_u64()),
            limit: arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|s| s as usize),
            offset: arguments
                .get("offset")
                .and_then(|v| v.as_u64())
                .map(|s| s as usize),
        };
        let result = get_traffic_filtered(client, api_url, filter).await?;
        Ok(vec![ContentBlock::Text {
            text: format_traffic_summary(&result),
        }])
    }
}

/// Get detailed information about a specific traffic entry.
pub struct GetTrafficEntryTool;

#[async_trait::async_trait]
impl McpTool for GetTrafficEntryTool {
    fn name(&self) -> &str {
        "madhyamas_get_traffic_entry"
    }
    fn description(&self) -> &str {
        "Get detailed information about a specific traffic entry, including full request/response headers and bodies."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the traffic entry to retrieve"
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
        let id = get_id(arguments)?;
        let result = get_traffic_entry(client, api_url, &id).await?;
        Ok(vec![ContentBlock::Text {
            text: format_traffic_detail(&result),
        }])
    }
}

/// Search captured traffic by content.
pub struct SearchTrafficTool;

#[async_trait::async_trait]
impl McpTool for SearchTrafficTool {
    fn name(&self) -> &str {
        "madhyamas_search_traffic"
    }
    fn description(&self) -> &str {
        "Search captured traffic by content (headers, bodies, URLs). Useful for finding specific API calls or patterns."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query string"
                }
            },
            "required": ["query"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("query is required".to_string()))?;
        let result = search_traffic(client, api_url, query).await?;
        Ok(vec![ContentBlock::Text {
            text: format_traffic_summary(&result),
        }])
    }
}

/// Get the total count of captured traffic entries.
pub struct GetTrafficCountTool;

#[async_trait::async_trait]
impl McpTool for GetTrafficCountTool {
    fn name(&self) -> &str {
        "madhyamas_get_traffic_count"
    }
    fn description(&self) -> &str {
        "Get the total count of captured traffic entries."
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
        let result = get_traffic_count(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Clear all captured traffic.
pub struct ClearTrafficTool;

#[async_trait::async_trait]
impl McpTool for ClearTrafficTool {
    fn name(&self) -> &str {
        "madhyamas_clear_traffic"
    }
    fn description(&self) -> &str {
        "Clear all captured traffic. This action cannot be undone."
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
        let result = clear_traffic(client, api_url).await?;
        Ok(json_text(&result))
    }
}

/// Import traffic from a HAR JSON document into a new session.
pub struct ImportHarTool;

#[async_trait::async_trait]
impl McpTool for ImportHarTool {
    fn name(&self) -> &str {
        "madhyamas_import_har"
    }
    fn description(&self) -> &str {
        "Import traffic from a HAR (HTTP Archive) JSON document into a new session. Each log.entries[] entry is converted into a traffic entry. Invalid entries are skipped. Useful for loading traffic captured by other tools (Chrome DevTools, Charles, Fiddler)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "har": {
                    "type": "object",
                    "description": "The full HAR JSON document (must contain a 'log' object with an 'entries' array)"
                },
                "session_name": {
                    "type": "string",
                    "description": "Optional name for the newly created session (default: 'Imported HAR')"
                },
                "switch_session": {
                    "type": "boolean",
                    "description": "Switch the active session to the newly created one after import (default: false)"
                }
            },
            "required": ["har"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let har = arguments
            .get("har")
            .ok_or_else(|| McpError::InvalidParams("har is required".to_string()))?
            .clone();
        let session_name = arguments
            .get("session_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let switch_session = arguments
            .get("switch_session")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let result = import_har(
            client,
            api_url,
            har,
            session_name.as_deref(),
            switch_session,
        )
        .await?;
        Ok(json_text(&result))
    }
}

/// Get script execution traces for a traffic entry.
pub struct GetTrafficScriptTracesTool;

#[async_trait::async_trait]
impl McpTool for GetTrafficScriptTracesTool {
    fn name(&self) -> &str {
        "madhyamas_get_traffic_script_traces"
    }
    fn description(&self) -> &str {
        "Get script execution traces for a specific traffic entry, showing \
         which scripts ran on the request and their results."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Traffic entry ID" }
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
        let id = get_id(arguments)?;
        let resp = client
            .get(format!("{}/api/traffic/{}/script-traces", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

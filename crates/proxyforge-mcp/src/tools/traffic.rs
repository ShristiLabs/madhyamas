//! Traffic inspection tools

use reqwest::Client;
use serde_json::{json, Value};

use crate::types::McpError;

/// Get captured traffic
pub async fn get_traffic(
    client: &Client,
    api_url: &str,
    filter: Option<&str>,
    method: Option<&str>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Value, McpError> {
    let mut url = format!("{}/api/traffic", api_url);
    let mut params = Vec::new();

    if let Some(f) = filter {
        params.push(("filter", f.to_string()));
    }
    if let Some(m) = method {
        params.push(("method", m.to_string()));
    }
    if let Some(l) = limit {
        params.push(("limit", l.to_string()));
    }
    if let Some(o) = offset {
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
pub async fn get_traffic_entry(
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
pub async fn clear_traffic(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
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

/// Get traffic count
pub async fn get_traffic_count(
    client: &Client,
    api_url: &str,
) -> Result<Value, McpError> {
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
pub async fn search_traffic(
    client: &Client,
    api_url: &str,
    query: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/traffic?search={}", api_url, urlencoding::encode(query));

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
pub fn format_traffic_summary(traffic: &Value) -> String {
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
                    i + 1, method, url, status, duration
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
                        i + 1, method, url, status, duration
                    ));
                }
            }
        }
    }

    summary
}

/// Format a single traffic entry for detailed analysis
pub fn format_traffic_detail(entry: &Value) -> String {
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
            detail.push_str(&format!("- **Headers**: ```json\n{}\n```\n", serde_json::to_string_pretty(headers).unwrap_or_default()));
        }
        if let Some(body) = obj.get("request_body") {
            detail.push_str(&format!("- **Body**: ```json\n{}\n```\n", serde_json::to_string_pretty(body).unwrap_or_default()));
        }

        // Response info
        detail.push_str("\n## Response\n");
        if let Some(status) = obj.get("status_code").and_then(|v| v.as_u64()) {
            detail.push_str(&format!("- **Status**: {}\n", status));
        }
        if let Some(headers) = obj.get("response_headers") {
            detail.push_str(&format!("- **Headers**: ```json\n{}\n```\n", serde_json::to_string_pretty(headers).unwrap_or_default()));
        }
        if let Some(body) = obj.get("response_body") {
            detail.push_str(&format!("- **Body**: ```json\n{}\n```\n", serde_json::to_string_pretty(body).unwrap_or_default()));
        }

        // Timing
        if let Some(duration) = obj.get("duration_ms").and_then(|v| v.as_u64()) {
            detail.push_str(&format!("\n- **Duration**: {}ms\n", duration));
        }
    }

    detail
}

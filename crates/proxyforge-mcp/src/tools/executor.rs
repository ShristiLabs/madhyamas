//! Tool executor

use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use crate::types::{ContentBlock, McpError};
use super::traffic;
use super::mocks;
use super::breakpoints;
use super::replay;
use super::sessions;

/// Tool executor that handles tool calls
pub struct ToolExecutor {
    api_url: String,
    client: Client,
}

impl ToolExecutor {
    pub fn new(api_url: String, client: Client) -> Self {
        Self { api_url, client }
    }

    /// Execute a tool by name
    pub async fn execute(&self, tool_name: &str, arguments: Value) -> Result<Vec<ContentBlock>, McpError> {
        match tool_name {
            // Traffic tools
            "proxyforge_get_traffic" => {
                let args: TrafficArgs = self.parse_args(&arguments)?;
                let result = traffic::get_traffic(
                    &self.client,
                    &self.api_url,
                    args.filter.as_deref(),
                    args.method.as_deref(),
                    args.limit,
                    args.offset,
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: traffic::format_traffic_summary(&result),
                }])
            }

            "proxyforge_get_traffic_entry" => {
                let args: EntryArgs = self.parse_args(&arguments)?;
                let result = traffic::get_traffic_entry(
                    &self.client,
                    &self.api_url,
                    &args.id,
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: traffic::format_traffic_detail(&result),
                }])
            }

            "proxyforge_search_traffic" => {
                let args: SearchArgs = self.parse_args(&arguments)?;
                let result = traffic::search_traffic(
                    &self.client,
                    &self.api_url,
                    &args.query,
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: traffic::format_traffic_summary(&result),
                }])
            }

            "proxyforge_get_traffic_count" => {
                let result = traffic::get_traffic_count(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_clear_traffic" => {
                let result = traffic::clear_traffic(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Mock tools
            "proxyforge_create_mock" => {
                let args: MockCreateArgs = self.parse_args(&arguments)?;
                let result = mocks::create_mock(
                    &self.client,
                    &self.api_url,
                    &args.url_pattern,
                    args.method.as_deref(),
                    args.status_code,
                    args.headers,
                    args.body,
                    args.delay_ms,
                    Some(args.enabled),
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_list_mocks" => {
                let result = mocks::list_mocks(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_delete_mock" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result = mocks::delete_mock(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_toggle_mock" => {
                let args: ToggleArgs = self.parse_args(&arguments)?;
                let result = mocks::toggle_mock(&self.client, &self.api_url, &args.id, args.enabled).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Breakpoint tools
            "proxyforge_list_breakpoints" => {
                let result = breakpoints::list_breakpoints(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_create_breakpoint" => {
                let args: BreakpointCreateArgs = self.parse_args(&arguments)?;
                let result = breakpoints::create_breakpoint(
                    &self.client,
                    &self.api_url,
                    &args.url_pattern,
                    args.method.as_deref(),
                    args.direction.as_deref(),
                    Some(args.enabled),
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_delete_breakpoint" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result = breakpoints::delete_breakpoint(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Replay tools
            "proxyforge_replay_request" => {
                let args: ReplayArgs = self.parse_args(&arguments)?;
                let result = replay::replay_request(
                    &self.client,
                    &self.api_url,
                    &args.id,
                    args.modifications,
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: replay::format_replay_result(&result),
                }])
            }

            "proxyforge_save_request" => {
                let args: SaveRequestArgs = self.parse_args(&arguments)?;
                let result = replay::save_request(
                    &self.client,
                    &self.api_url,
                    &args.traffic_id,
                    args.name.as_deref(),
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_list_saved_requests" => {
                let result = replay::list_saved_requests(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_export_curl" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result = replay::export_curl(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: result.to_string(),
                }])
            }

            // Session tools
            "proxyforge_list_sessions" => {
                let result = sessions::list_sessions(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_create_session" => {
                let args: SessionCreateArgs = self.parse_args(&arguments)?;
                let result = sessions::create_session(
                    &self.client,
                    &self.api_url,
                    args.name.as_deref(),
                    args.description.as_deref(),
                ).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "proxyforge_switch_session" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result = sessions::switch_session(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Configuration
            "proxyforge_get_config" => {
                let result = self.get_config().await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            _ => Err(McpError::NotFound(format!("Unknown tool: {}", tool_name))),
        }
    }

    /// Parse arguments from JSON value
    fn parse_args<T: DeserializeOwned>(&self, value: &Value) -> Result<T, McpError> {
        serde_json::from_value(value.clone())
            .map_err(|e| McpError::InvalidParams(e.to_string()))
    }

    /// Get traffic list (for resource access)
    pub async fn get_traffic(
        &self,
        filter: Option<&str>,
        method: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Value, McpError> {
        traffic::get_traffic(&self.client, &self.api_url, filter, method, limit, offset).await
    }

    /// Get sessions list (for resource access)
    pub async fn get_sessions(&self) -> Result<Value, McpError> {
        sessions::list_sessions(&self.client, &self.api_url).await
    }

    /// Get proxy configuration
    pub async fn get_config(&self) -> Result<Value, McpError> {
        let url = format!("{}/api/config", self.api_url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        let config: Value = response
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))?;

        Ok(config)
    }

}

// ============ Argument Types ============

#[derive(Debug, Clone, Deserialize)]
struct TrafficArgs {
    #[serde(default)]
    filter: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct EntryArgs {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchArgs {
    query: String,
}

#[derive(Debug, Clone, Deserialize)]
struct IdArgs {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MockCreateArgs {
    url_pattern: String,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    status_code: Option<u16>,
    #[serde(default)]
    headers: Option<Value>,
    #[serde(default)]
    body: Option<Value>,
    #[serde(default)]
    delay_ms: Option<u64>,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Deserialize)]
struct ToggleArgs {
    id: String,
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct BreakpointCreateArgs {
    url_pattern: String,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ReplayArgs {
    id: String,
    #[serde(default)]
    modifications: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct SaveRequestArgs {
    traffic_id: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SessionCreateArgs {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

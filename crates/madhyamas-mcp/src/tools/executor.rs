//! Tool executor

use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use super::breakpoints;
use super::mocks;
use super::replay;
use super::sessions;
use super::traffic;
use crate::types::{ContentBlock, McpError};

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
    pub async fn execute(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        match tool_name {
            // Traffic tools
            "madhyamas_get_traffic" => {
                let args: TrafficArgs = self.parse_args(&arguments)?;
                let filter = traffic::TrafficFilter {
                    filter: args.filter,
                    method: args.method,
                    status: args.status,
                    file_type: args.file_type,
                    header: args.header,
                    cookie: args.cookie,
                    search: args.search,
                    min_size: args.min_size,
                    max_size: args.max_size,
                    min_time: args.min_time,
                    max_time: args.max_time,
                    limit: args.limit,
                    offset: args.offset,
                };
                let result =
                    traffic::get_traffic_filtered(&self.client, &self.api_url, filter).await?;
                Ok(vec![ContentBlock::Text {
                    text: traffic::format_traffic_summary(&result),
                }])
            }

            "madhyamas_get_traffic_entry" => {
                let args: EntryArgs = self.parse_args(&arguments)?;
                let result =
                    traffic::get_traffic_entry(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: traffic::format_traffic_detail(&result),
                }])
            }

            "madhyamas_search_traffic" => {
                let args: SearchArgs = self.parse_args(&arguments)?;
                let result =
                    traffic::search_traffic(&self.client, &self.api_url, &args.query).await?;
                Ok(vec![ContentBlock::Text {
                    text: traffic::format_traffic_summary(&result),
                }])
            }

            "madhyamas_get_traffic_count" => {
                let result = traffic::get_traffic_count(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_clear_traffic" => {
                let result = traffic::clear_traffic(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Mock tools
            "madhyamas_create_mock" => {
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
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_list_mocks" => {
                let result = mocks::list_mocks(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_delete_mock" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result = mocks::delete_mock(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_toggle_mock" => {
                let args: ToggleArgs = self.parse_args(&arguments)?;
                let result =
                    mocks::toggle_mock(&self.client, &self.api_url, &args.id, args.enabled).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Breakpoint tools
            "madhyamas_list_breakpoints" => {
                let result = breakpoints::list_breakpoints(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_create_breakpoint" => {
                let args: BreakpointCreateArgs = self.parse_args(&arguments)?;
                let result = breakpoints::create_breakpoint(
                    &self.client,
                    &self.api_url,
                    &args.url_pattern,
                    args.method.as_deref(),
                    args.direction.as_deref(),
                    Some(args.enabled),
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_delete_breakpoint" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    breakpoints::delete_breakpoint(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Replay tools
            "madhyamas_replay_request" => {
                let args: ReplayArgs = self.parse_args(&arguments)?;
                let result = replay::replay_request(
                    &self.client,
                    &self.api_url,
                    &args.id,
                    args.modifications,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: replay::format_replay_result(&result),
                }])
            }

            "madhyamas_save_request" => {
                let args: SaveRequestArgs = self.parse_args(&arguments)?;
                let result = replay::save_request(
                    &self.client,
                    &self.api_url,
                    &args.traffic_id,
                    args.name.as_deref(),
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_list_saved_requests" => {
                let result = replay::list_saved_requests(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_export_curl" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result = replay::export_curl(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: result.to_string(),
                }])
            }

            // Session tools
            "madhyamas_list_sessions" => {
                let result = sessions::list_sessions(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_create_session" => {
                let args: SessionCreateArgs = self.parse_args(&arguments)?;
                let result = sessions::create_session(
                    &self.client,
                    &self.api_url,
                    args.name.as_deref(),
                    args.description.as_deref(),
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_switch_session" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    sessions::switch_session(&self.client, &self.api_url, &args.id).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Configuration
            "madhyamas_get_config" => {
                let result = self.get_config().await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_update_config" => {
                let args: UpdateConfigArgs = self.parse_args(&arguments)?;
                let result = self.update_config(args).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Capture mode
            "madhyamas_get_capture_status" => {
                let result = self.get_capture_status().await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_toggle_capture" => {
                let result = self.toggle_capture().await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            _ => Err(McpError::NotFound(format!("Unknown tool: {}", tool_name))),
        }
    }

    /// Parse arguments from JSON value
    fn parse_args<T: DeserializeOwned>(&self, value: &Value) -> Result<T, McpError> {
        serde_json::from_value(value.clone()).map_err(|e| McpError::InvalidParams(e.to_string()))
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

        let response = self
            .client
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

    /// Update proxy configuration
    pub async fn update_config(&self, args: UpdateConfigArgs) -> Result<Value, McpError> {
        let url = format!("{}/api/config", self.api_url);

        let mut payload = serde_json::Map::new();
        if let Some(intercept) = args.intercept_https {
            payload.insert("intercept_https".to_string(), Value::Bool(intercept));
        }
        if let Some(max_req) = args.max_requests {
            payload.insert("max_requests".to_string(), Value::Number(max_req.into()));
        }
        if let Some(verbose) = args.verbose {
            payload.insert("verbose".to_string(), Value::Bool(verbose));
        }
        if let Some(ip) = args.public_ip {
            payload.insert("public_ip".to_string(), ip);
        }

        let response = self
            .client
            .patch(&url)
            .json(&Value::Object(payload))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        response
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))
    }

    /// Get capture status
    pub async fn get_capture_status(&self) -> Result<Value, McpError> {
        let url = format!("{}/api/capture", self.api_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        response
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))
    }

    /// Toggle capture mode
    pub async fn toggle_capture(&self) -> Result<Value, McpError> {
        let url = format!("{}/api/capture/toggle", self.api_url);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }

        response
            .json()
            .await
            .map_err(|e| McpError::Parse(e.to_string()))
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
    status: Option<u16>,
    #[serde(default)]
    file_type: Option<String>,
    #[serde(default)]
    header: Option<String>,
    #[serde(default)]
    cookie: Option<String>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    min_size: Option<usize>,
    #[serde(default)]
    max_size: Option<usize>,
    #[serde(default)]
    min_time: Option<u64>,
    #[serde(default)]
    max_time: Option<u64>,
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

fn default_true() -> bool {
    true
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfigArgs {
    #[serde(default)]
    intercept_https: Option<bool>,
    #[serde(default)]
    max_requests: Option<usize>,
    #[serde(default)]
    verbose: Option<bool>,
    #[serde(default)]
    public_ip: Option<Value>,
}

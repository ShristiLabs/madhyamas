//! Tool executor

use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

use super::breakpoints;
use super::grpc;
use super::mocks;
use super::plugins;
use super::replay;
use super::rewrites;
use super::sanitize_id;
use super::scripts;
use super::sessions;
use super::throttle;
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

    /// Borrow the underlying HTTP client (used by trait-based tools).
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Borrow the API URL (used by trait-based tools).
    pub fn api_url(&self) -> &str {
        &self.api_url
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
                let result = traffic::get_traffic_entry(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                )
                .await?;
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

            "madhyamas_import_har" => {
                let args: ImportHarArgs = self.parse_args(&arguments)?;
                let result = traffic::import_har(
                    &self.client,
                    &self.api_url,
                    args.har,
                    args.session_name.as_deref(),
                    args.switch_session.unwrap_or(false),
                )
                .await?;
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
                let result =
                    mocks::delete_mock(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_toggle_mock" => {
                let args: ToggleArgs = self.parse_args(&arguments)?;
                let result = mocks::toggle_mock(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.enabled,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Advanced Mock tools
            "madhyamas_create_advanced_mock" => {
                let args: AdvancedMockCreateArgs = self.parse_args(&arguments)?;
                let result = mocks::create_advanced_mock(
                    &self.client,
                    &self.api_url,
                    &args.name,
                    args.condition,
                    args.response_config,
                    args.description.as_deref(),
                    args.tags,
                    args.collection_id.as_deref(),
                    args.enabled,
                    args.priority,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_update_mock" => {
                let args: UpdateMockArgs = self.parse_args(&arguments)?;
                let result = mocks::update_mock(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.mock,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_get_mock" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    mocks::get_mock(&self.client, &self.api_url, &sanitize_id(&args.id)?).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_duplicate_mock" => {
                let args: DuplicateMockArgs = self.parse_args(&arguments)?;
                let result = mocks::duplicate_mock(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.new_name.as_deref(),
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_rollback_mock" => {
                let args: RollbackMockArgs = self.parse_args(&arguments)?;
                let result = mocks::rollback_mock(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.version,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_get_mock_versions" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    mocks::get_mock_versions(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Mock Collections
            "madhyamas_list_mock_collections" => {
                let result = mocks::list_collections(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_create_mock_collection" => {
                let args: CreateCollectionArgs = self.parse_args(&arguments)?;
                let result = mocks::create_collection(
                    &self.client,
                    &self.api_url,
                    &args.name,
                    args.description.as_deref(),
                    args.tags,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_delete_mock_collection" => {
                let args: DeleteCollectionArgs = self.parse_args(&arguments)?;
                let result = mocks::delete_collection(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.delete_rules.unwrap_or(false),
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_toggle_mock_collection" => {
                let args: ToggleCollectionArgs = self.parse_args(&arguments)?;
                let result = mocks::toggle_collection(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.enabled,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Mock Analytics
            "madhyamas_get_mock_analytics" => {
                let result = mocks::get_analytics(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_get_mock_hit_history" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    mocks::get_hit_history(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Mock Testing & Preview
            "madhyamas_test_mock" => {
                let args: TestMockArgs = self.parse_args(&arguments)?;
                let result = mocks::test_mock(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.request,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_preview_mock_match" => {
                let args: PreviewMockArgs = self.parse_args(&arguments)?;
                let result =
                    mocks::preview_mock_match(&self.client, &self.api_url, args.request).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Mock Import/Export
            "madhyamas_export_mocks" => {
                let result = mocks::export_mocks(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_import_mocks" => {
                let args: ImportMocksArgs = self.parse_args(&arguments)?;
                let result =
                    mocks::import_mocks(&self.client, &self.api_url, &args.format, &args.data)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Mock Recording
            "madhyamas_set_mock_recording" => {
                let args: RecordingArgs = self.parse_args(&arguments)?;
                let result =
                    mocks::set_recording(&self.client, &self.api_url, args.enabled).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_get_mock_recording_status" => {
                let result = mocks::get_recording_status(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_get_recorded_mocks" => {
                let result = mocks::get_recorded_mocks(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_promote_recorded_mocks" => {
                let result = mocks::promote_recorded_mocks(&self.client, &self.api_url).await?;
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
                let result = breakpoints::delete_breakpoint(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                )
                .await?;
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
                    &sanitize_id(&args.id)?,
                    args.modifications,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: replay::format_replay_result(&result),
                }])
            }

            "madhyamas_replay_advanced" => {
                let args: ReplayAdvancedArgs = self.parse_args(&arguments)?;
                let result = replay::replay_request_advanced(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.modifications,
                    args.iterations,
                    args.concurrency,
                    args.delay_ms,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: replay::format_batch_replay_result(&result),
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
                let result =
                    replay::export_curl(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
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
                    sessions::switch_session(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_export_session" => {
                let args: ExportSessionArgs = self.parse_args(&arguments)?;
                let result = sessions::export_session(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.format.as_deref(),
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            "madhyamas_import_session" => {
                let args: ImportSessionArgs = self.parse_args(&arguments)?;
                let result =
                    sessions::import_session(&self.client, &self.api_url, args.session_data)
                        .await?;
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

            // Throttle tools
            "madhyamas_get_throttle" => {
                let result = throttle::get_throttle(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_set_throttle" => {
                let args: SetThrottleArgs = self.parse_args(&arguments)?;
                let profile = json!({
                    "name": args.name.unwrap_or_else(|| "Custom".to_string()),
                    "download_bps": args.download_bps.unwrap_or(0),
                    "upload_bps": args.upload_bps.unwrap_or(0),
                    "latency_ms": args.delay_ms.unwrap_or(0),
                    "jitter_ms": args.jitter_ms.unwrap_or(0),
                    "packet_loss_percent": args.packet_loss_percent.unwrap_or(0),
                });
                let result =
                    throttle::set_throttle(&self.client, &self.api_url, profile, args.enabled)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_toggle_throttle" => {
                let args: ToggleArgs = self.parse_args(&arguments)?;
                let result =
                    throttle::set_throttle_enabled(&self.client, &self.api_url, args.enabled)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_throttle_presets" => {
                let result = throttle::get_throttle_presets(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Rewrite tools
            "madhyamas_list_rewrites" => {
                let result = rewrites::list_rewrites(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_create_rewrite" => {
                let args: CreateRewriteArgs = self.parse_args(&arguments)?;
                let result = rewrites::create_rewrite(
                    &self.client,
                    &self.api_url,
                    &args.name,
                    args.condition,
                    &args.direction,
                    args.rewrites,
                    args.enabled,
                    args.priority,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_delete_rewrite" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    rewrites::delete_rewrite(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_toggle_rewrite" => {
                let args: ToggleArgs = self.parse_args(&arguments)?;
                let result = rewrites::toggle_rewrite(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.enabled,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_rewrite_templates" => {
                let result = rewrites::get_rewrite_templates(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // gRPC tools
            "madhyamas_get_grpc_connections" => {
                let result = grpc::get_grpc_connections(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_grpc_streams" => {
                let result = grpc::get_grpc_streams(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_grpc_frames" => {
                let args: GrpcFramesArgs = self.parse_args(&arguments)?;
                let result =
                    grpc::get_grpc_frames(&self.client, &self.api_url, args.filter.as_deref())
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_grpc_stats" => {
                let result = grpc::get_grpc_stats(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_clear_grpc" => {
                let result = grpc::clear_grpc_frames(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Script tools
            "madhyamas_list_scripts" => {
                let result = scripts::list_scripts(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_create_script" => {
                let args: CreateScriptArgs = self.parse_args(&arguments)?;
                let result = scripts::create_script(
                    &self.client,
                    &self.api_url,
                    &args.name,
                    &args.source,
                    args.hook.as_deref(),
                    args.enabled,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_script" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    scripts::get_script(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_update_script" => {
                let args: UpdateScriptArgs = self.parse_args(&arguments)?;
                let result = scripts::update_script(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.script,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_delete_script" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    scripts::delete_script(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_toggle_script" => {
                let args: ToggleArgs = self.parse_args(&arguments)?;
                let result = scripts::toggle_script(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.enabled,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_script_templates" => {
                let result = scripts::get_script_templates(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_test_script" => {
                let args: TestScriptArgs = self.parse_args(&arguments)?;
                let result =
                    scripts::test_script(&self.client, &self.api_url, &args.source, &args.hook)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_validate_script" => {
                let args: ValidateScriptArgs = self.parse_args(&arguments)?;
                let result =
                    scripts::validate_script(&self.client, &self.api_url, &args.source).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_script_history" => {
                let args: ScriptHistoryArgs = self.parse_args(&arguments)?;
                let result = scripts::get_script_history(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.limit,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }

            // Plugin tools
            "madhyamas_list_plugins" => {
                let result = plugins::list_plugins(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_plugin" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    plugins::get_plugin(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_enable_plugin" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    plugins::enable_plugin(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_disable_plugin" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    plugins::disable_plugin(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_plugin_stats" => {
                let args: IdArgs = self.parse_args(&arguments)?;
                let result =
                    plugins::get_plugin_stats(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_reload_plugins" => {
                let result = plugins::reload_plugins(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_install_plugin" => {
                let args = self.parse_args::<InstallPluginArgs>(&arguments)?;
                let result = plugins::install_plugin(
                    &self.client,
                    &self.api_url,
                    &args.source,
                    &args.target,
                    args.checksum.as_deref(),
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_uninstall_plugin" => {
                let args = self.parse_args::<PluginIdArgs>(&arguments)?;
                let result =
                    plugins::uninstall_plugin(&self.client, &self.api_url, &sanitize_id(&args.id)?)
                        .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_search_registry" => {
                let args = self.parse_args::<SearchArgs>(&arguments)?;
                let result =
                    plugins::search_registry(&self.client, &self.api_url, &args.query).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_list_registry" => {
                let result = plugins::list_registry(&self.client, &self.api_url).await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_plugin_schema" => {
                let args = self.parse_args::<PluginIdArgs>(&arguments)?;
                let result = plugins::get_plugin_schema(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_plugin_settings" => {
                let args = self.parse_args::<PluginIdArgs>(&arguments)?;
                let result = plugins::get_plugin_settings(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_update_plugin_settings" => {
                let args = self.parse_args::<UpdatePluginSettingsArgs>(&arguments)?;
                let result = plugins::update_plugin_settings(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.settings,
                )
                .await?;
                Ok(vec![ContentBlock::Text {
                    text: serde_json::to_string_pretty(&result).unwrap_or_default(),
                }])
            }
            "madhyamas_get_plugin_logs" => {
                let args = self.parse_args::<PluginLogsArgs>(&arguments)?;
                let result = plugins::get_plugin_logs(
                    &self.client,
                    &self.api_url,
                    &sanitize_id(&args.id)?,
                    args.limit.unwrap_or(50),
                )
                .await?;
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
struct PluginIdArgs {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct InstallPluginArgs {
    #[serde(default = "default_source")]
    source: String,
    target: String,
    #[serde(default)]
    checksum: Option<String>,
}

fn default_source() -> String {
    "url".to_string()
}

#[derive(Debug, Clone, Deserialize)]
struct UpdatePluginSettingsArgs {
    id: String,
    settings: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct PluginLogsArgs {
    id: String,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct ImportHarArgs {
    har: Value,
    #[serde(default)]
    session_name: Option<String>,
    #[serde(default)]
    switch_session: Option<bool>,
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
struct ReplayAdvancedArgs {
    id: String,
    iterations: usize,
    concurrency: usize,
    #[serde(default)]
    delay_ms: Option<u64>,
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

// Advanced Mock Arguments
#[derive(Debug, Clone, Deserialize)]
struct AdvancedMockCreateArgs {
    name: String,
    condition: Value,
    response_config: Value,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    collection_id: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    priority: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdateMockArgs {
    id: String,
    mock: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct DuplicateMockArgs {
    id: String,
    #[serde(default)]
    new_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RollbackMockArgs {
    id: String,
    version: u32,
}

// Collection Arguments
#[derive(Debug, Clone, Deserialize)]
struct CreateCollectionArgs {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct DeleteCollectionArgs {
    id: String,
    #[serde(default)]
    delete_rules: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct ToggleCollectionArgs {
    id: String,
    enabled: bool,
}

// Testing Arguments
#[derive(Debug, Clone, Deserialize)]
struct TestMockArgs {
    id: String,
    request: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct PreviewMockArgs {
    request: Value,
}

// Import Arguments
#[derive(Debug, Clone, Deserialize)]
struct ImportMocksArgs {
    format: String,
    data: String,
}

// Recording Arguments
#[derive(Debug, Clone, Deserialize)]
struct RecordingArgs {
    enabled: bool,
}

// Session Export/Import Arguments
#[derive(Debug, Clone, Deserialize)]
struct ExportSessionArgs {
    id: String,
    #[serde(default)]
    format: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ImportSessionArgs {
    session_data: Value,
}

// Throttle Arguments
#[derive(Debug, Clone, Deserialize)]
struct SetThrottleArgs {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    download_bps: Option<u64>,
    #[serde(default)]
    upload_bps: Option<u64>,
    #[serde(default)]
    delay_ms: Option<u64>,
    #[serde(default)]
    jitter_ms: Option<u64>,
    #[serde(default)]
    packet_loss_percent: Option<u8>,
    #[serde(default)]
    enabled: Option<bool>,
}

// Rewrite Arguments
#[derive(Debug, Clone, Deserialize)]
struct CreateRewriteArgs {
    name: String,
    condition: Value,
    direction: String,
    rewrites: Value,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    priority: Option<u32>,
}

// gRPC Arguments
#[derive(Debug, Clone, Deserialize)]
struct GrpcFramesArgs {
    #[serde(default)]
    filter: Option<String>,
}

// Script Arguments
#[derive(Debug, Clone, Deserialize)]
struct CreateScriptArgs {
    name: String,
    source: String,
    #[serde(default)]
    hook: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdateScriptArgs {
    id: String,
    script: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct TestScriptArgs {
    source: String,
    hook: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ValidateScriptArgs {
    source: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptHistoryArgs {
    id: String,
    #[serde(default)]
    limit: Option<usize>,
}

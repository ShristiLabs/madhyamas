//! Phase 3 handlers: gRPC, Scripting, Plugins

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use madhyamas_core::{
    GrpcDirection, GrpcFilter, Script, ScriptMatch, ScriptTemplates,
};
use serde::Deserialize;
use std::sync::Arc;

// ============== gRPC Handlers ==============

/// Get all gRPC connections
pub async fn get_grpc_connections(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.grpc_manager.get_connections())
}

/// Get all gRPC streams
pub async fn get_grpc_streams(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.grpc_manager.get_streams())
}

/// Query parameters for gRPC frames
#[derive(Debug, Deserialize)]
pub struct GrpcFilterQuery {
    pub service: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub direction: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status_code: Option<i32>,
}

/// Get gRPC frames with filtering
pub async fn get_grpc_frames(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GrpcFilterQuery>,
) -> impl IntoResponse {
    let filter = GrpcFilter {
        service: query.service,
        method: query.method,
        path_pattern: query.path,
        direction: query
            .direction
            .as_deref()
            .and_then(|d| match d.to_lowercase().as_str() {
                "request" => Some(GrpcDirection::Request),
                "response" => Some(GrpcDirection::Response),
                _ => None,
            }),
        search: query.search,
        limit: query.limit,
        offset: query.offset,
        status_code: query.status_code,
    };
    Json(state.grpc_manager.get_frames(&filter))
}

/// Get gRPC statistics
pub async fn get_grpc_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.grpc_manager.stats())
}

/// Clear all gRPC frames
pub async fn clear_grpc_frames(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.grpc_manager.clear();
    StatusCode::NO_CONTENT
}

// ============== Script Handlers ==============

/// Get all scripts
pub async fn get_scripts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.script_runtime.get_scripts())
}

/// Get a single script
pub async fn get_script(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.script_runtime.get_script(&id) {
        Some(script) => Json(script).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Script not found" })),
        )
            .into_response(),
    }
}

/// Create script request
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateScriptRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1))]
    pub source: String,
    pub description: Option<String>,
    pub hooks: Vec<String>,
    /// Optional declarative match filter.  When set, the script only fires
    /// on requests matching all specified fields.
    #[serde(default)]
    pub match_filter: Option<ScriptMatch>,
}

/// Create a new script
pub async fn create_script(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateScriptRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let mut script = Script::new(req.name, req.source);
    script.description = req.description;
    script.hooks = req.hooks;
    script.match_filter = req.match_filter;
    let id = state.script_runtime.register_script(script.clone());
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id, "script": script })),
    )
        .into_response()
}

/// Update script request (partial update — only provided fields are applied).
#[derive(Debug, Deserialize)]
pub struct UpdateScriptRequest {
    pub source: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub hooks: Option<Vec<String>>,
    /// Set to `Some(None)` to clear the match filter, or `Some(Some(filter))`
    /// to update it.  Omit to leave unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_filter: Option<Option<ScriptMatch>>,
    /// Update the script priority (lower runs first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
}

/// Update a script
pub async fn update_script(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScriptRequest>,
) -> impl IntoResponse {
    if let Some(ref source) = req.source {
        if source.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "source must not be empty" })),
            )
                .into_response();
        }
    }
    let fields = madhyamas_core::UpdateScriptFields {
        source: req.source,
        name: req.name,
        description: req.description,
        hooks: req.hooks,
        match_filter: req.match_filter,
        priority: req.priority,
    };
    if state.script_runtime.update_script_fields(&id, fields) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Script not found" })),
        )
            .into_response()
    }
}

/// Delete a script
pub async fn delete_script(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.script_runtime.remove_script(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Script not found" })),
        )
            .into_response()
    }
}

/// Toggle script request
#[derive(Debug, Deserialize)]
pub struct ToggleScriptRequest {
    pub enabled: bool,
}

/// Toggle a script on/off
pub async fn toggle_script(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleScriptRequest>,
) -> impl IntoResponse {
    if state.script_runtime.toggle_script(&id, req.enabled) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Script not found" })),
        )
            .into_response()
    }
}

/// Reorder script request — move a script up (run earlier) or down (run later).
#[derive(Debug, Deserialize)]
pub struct ReorderScriptRequest {
    /// `"up"` to run earlier (lower priority), `"down"` to run later.
    pub direction: String,
}

/// Reorder a script by swapping its priority with the adjacent script.
pub async fn reorder_script(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ReorderScriptRequest>,
) -> impl IntoResponse {
    if state.script_runtime.reorder_script(&id, &req.direction) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Script not found or cannot move in that direction"
            })),
        )
            .into_response()
    }
}

/// Match-preview request — check which scripts would fire for a given request.
#[derive(Debug, Deserialize)]
pub struct MatchPreviewRequest {
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    /// Optional: filter by hook (e.g. "on_request").  If omitted, checks
    /// all hooks.
    #[serde(default)]
    pub hook: Option<String>,
}

/// Match-preview response item.
#[derive(Debug, serde::Serialize)]
pub struct MatchPreviewItem {
    pub id: String,
    pub name: String,
    pub priority: u32,
    pub enabled: bool,
    pub hooks: Vec<String>,
    pub match_filter: Option<ScriptMatch>,
}

/// Preview which scripts would match a given request, without executing them.
pub async fn match_preview_scripts(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MatchPreviewRequest>,
) -> impl IntoResponse {
    let all_scripts = state.script_runtime.get_scripts();
    let mut matching: Vec<MatchPreviewItem> = all_scripts
        .into_iter()
        .filter(|s| {
            // If a hook filter is provided, only include scripts that
            // subscribe to that hook.
            if let Some(ref hook) = req.hook {
                if !s.hooks.iter().any(|h| h == hook) {
                    return false;
                }
            }
            // Check the match filter.  An empty/None filter matches all.
            if let Some(ref filter) = s.match_filter {
                if !filter.is_empty() {
                    return filter.matches(&req.method, &req.host, &req.path, &req.url);
                }
            }
            true
        })
        .map(|s| MatchPreviewItem {
            id: s.id,
            name: s.name,
            priority: s.priority,
            enabled: s.enabled,
            hooks: s.hooks,
            match_filter: s.match_filter,
        })
        .collect();
    // Sort by priority (execution order) so the caller sees the order
    // in which scripts would fire.
    matching.sort_by_key(|m| m.priority);
    Json(matching).into_response()
}

/// Get script execution traces for a specific traffic entry.  Returns all
/// script executions (across all scripts) that were recorded for the given
/// traffic entry ID, in execution order.  Each trace includes the script
/// ID, duration, success/error status, console output, and hook name.
///
/// Route: `GET /api/traffic/{id}/script-traces`
pub async fn get_traffic_script_traces(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let traces = state
        .script_runtime
        .get_executions_for_traffic_entry(&id, 200);

    // Enrich each trace with the script name (if the script still exists).
    #[derive(serde::Serialize)]
    struct ScriptTrace {
        script_id: String,
        script_name: Option<String>,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
        console: Vec<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
        traffic_entry_id: Option<String>,
        hook: Option<String>,
    }

    let enriched: Vec<ScriptTrace> = traces
        .into_iter()
        .map(|exec| {
            let script_name = state
                .script_runtime
                .get_script(&exec.script_id)
                .map(|s| s.name);
            ScriptTrace {
                script_id: exec.script_id,
                script_name,
                duration_ms: exec.duration_ms,
                success: exec.success,
                error: exec.error,
                console: exec.console,
                timestamp: exec.timestamp,
                traffic_entry_id: exec.traffic_entry_id,
                hook: exec.hook,
            }
        })
        .collect();

    Json(enriched).into_response()
}

/// Get script templates
pub async fn get_script_templates() -> impl IntoResponse {
    Json(vec![
        ScriptTemplates::log_requests(),
        ScriptTemplates::add_cors(),
        ScriptTemplates::block_domains(),
        ScriptTemplates::modify_headers(),
        ScriptTemplates::mock_api(),
        ScriptTemplates::inject_latency(),
        ScriptTemplates::rewrite_url(),
        ScriptTemplates::inject_auth_token(),
        ScriptTemplates::modify_json_response(),
        ScriptTemplates::override_status_code(),
        ScriptTemplates::cache_buster(),
        ScriptTemplates::strip_response_headers(),
        ScriptTemplates::conditional_mock(),
    ])
}

/// Get script configuration
pub async fn get_script_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.script_runtime.config())
}

/// Partial update for script configuration — only provided fields are
/// applied; the rest are left unchanged.
#[derive(Debug, Deserialize)]
pub struct UpdateScriptConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_console: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_network: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_error: Option<madhyamas_core::ScriptErrorPolicy>,
}

/// Update script configuration (partial update — only provided fields
/// are applied).
pub async fn update_script_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateScriptConfigRequest>,
) -> impl IntoResponse {
    let mut config = state.script_runtime.config();
    if let Some(v) = req.timeout_ms {
        config.timeout_ms = v;
    }
    if let Some(v) = req.max_memory_bytes {
        config.max_memory_bytes = v;
    }
    if let Some(v) = req.enable_console {
        config.enable_console = v;
    }
    if let Some(v) = req.allow_network {
        config.allow_network = v;
    }
    if let Some(v) = req.allow_fs {
        config.allow_fs = v;
    }
    if let Some(v) = req.on_error {
        config.on_error = v;
    }
    state.script_runtime.set_config(config.clone());
    Json(config)
}

/// Query parameters for script history
#[derive(Debug, Deserialize)]
pub struct ScriptHistoryQuery {
    pub limit: Option<usize>,
}

/// Get execution history for a specific script
pub async fn get_script_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<ScriptHistoryQuery>,
) -> impl IntoResponse {
    match state.script_runtime.get_script(&id) {
        Some(_) => {
            let history = state
                .script_runtime
                .get_script_history(&id, query.limit.unwrap_or(50));
            Json(history).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Script not found" })),
        )
            .into_response(),
    }
}

/// Get execution history for all scripts (enriched with script names).
pub async fn get_scripts_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ScriptHistoryQuery>,
) -> impl IntoResponse {
    let history = state.script_runtime.get_history(query.limit);

    // Enrich each execution with the script name (if the script still
    // exists) so the UI can display a human-readable label instead of
    // just the script ID.
    #[derive(serde::Serialize)]
    struct HistoryEntry {
        script_id: String,
        script_name: Option<String>,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
        console: Vec<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
        traffic_entry_id: Option<String>,
        hook: Option<String>,
    }

    let enriched: Vec<HistoryEntry> = history
        .into_iter()
        .map(|exec| {
            let script_name = state
                .script_runtime
                .get_script(&exec.script_id)
                .map(|s| s.name);
            HistoryEntry {
                script_id: exec.script_id,
                script_name,
                duration_ms: exec.duration_ms,
                success: exec.success,
                error: exec.error,
                console: exec.console,
                timestamp: exec.timestamp,
                traffic_entry_id: exec.traffic_entry_id,
                hook: exec.hook,
            }
        })
        .collect();

    Json(enriched)
}

/// Clear execution history (all scripts or a specific script)
pub async fn clear_script_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.script_runtime.get_script(&id) {
        Some(_) => {
            // Clear history for this specific script only (in-memory +
            // DB). Other scripts' execution history is preserved.
            state.script_runtime.clear_script_history(&id);
            StatusCode::NO_CONTENT.into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Script not found" })),
        )
            .into_response(),
    }
}

/// Test script request — dry-run a script against a sample context without
/// affecting live traffic or recording history.
#[derive(Debug, Deserialize)]
pub struct TestScriptRequest {
    /// Script source code to test
    pub source: String,
    /// Hook to test (e.g. "on_request", "on_response")
    pub hook: String,
    /// Optional sample request (defaults to a simple GET /)
    #[serde(default)]
    pub request: Option<serde_json::Value>,
    /// Optional sample response (for on_response hooks)
    #[serde(default)]
    pub response: Option<serde_json::Value>,
}

/// Test (dry-run) a script against a sample context.
pub async fn test_script(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestScriptRequest>,
) -> impl IntoResponse {
    use madhyamas_core::scripting::{RequestContext, ResponseContext, ScriptContext};
    use std::str::FromStr;

    let hook = match madhyamas_core::scripting::ScriptHook::from_str(&req.hook) {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };

    // Build the script context from the request or defaults.
    let mut ctx = ScriptContext::new("test-req", "test-sess", hook);

    let hook_str = ctx.hook.clone();
    if let Some(req_val) = req.request {
        if let Ok(req_ctx) = serde_json::from_value::<RequestContext>(req_val) {
            ctx.request = Some(req_ctx);
        }
    } else if hook_str == "on_request" || hook_str == "on_response" {
        // Default sample request for request/response hooks.
        ctx.request = Some(RequestContext {
            method: "GET".to_string(),
            url: "https://api.example.com/v1/users?id=42".to_string(),
            host: "api.example.com".to_string(),
            path: "/v1/users".to_string(),
            headers: std::collections::HashMap::from([
                ("Accept".to_string(), "application/json".to_string()),
                ("User-Agent".to_string(), "Madhyamas-Test".to_string()),
            ]),
            body: None,
            content_type: None,
            query: std::collections::HashMap::from([("id".to_string(), "42".to_string())]),
        });
    }

    if let Some(resp_val) = req.response {
        if let Ok(resp_ctx) = serde_json::from_value::<ResponseContext>(resp_val) {
            ctx.response = Some(resp_ctx);
        }
    } else if hook_str == "on_response" {
        // Default sample response for response hooks.
        ctx.response = Some(ResponseContext {
            status_code: 200,
            status_message: Some("OK".to_string()),
            headers: std::collections::HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body: Some(r#"{"status":"ok","data":[]}"#.to_string()),
            content_type: Some("application/json".to_string()),
            duration_ms: 42,
        });
    }

    let result = state.script_runtime.test_script(&req.source, &ctx);
    Json(result).into_response()
}

/// Validate script source code without executing it.
#[derive(Debug, Deserialize)]
pub struct ValidateScriptRequest {
    pub source: String,
}

/// Validate a script's source code (syntax check).
pub async fn validate_script(Json(req): Json<ValidateScriptRequest>) -> impl IntoResponse {
    match madhyamas_core::ScriptRuntime::validate(&req.source) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "valid": true }))).into_response(),
        Err(e) => (
            StatusCode::OK,
            Json(serde_json::json!({ "valid": false, "error": e })),
        )
            .into_response(),
    }
}

// ============== Plugin Handlers ==============

/// Get all plugins
pub async fn get_plugins(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.plugin_manager.get_plugins())
}

/// Get a single plugin
pub async fn get_plugin(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager.get_plugin(&id) {
        Some(plugin) => Json(plugin).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Plugin not found" })),
        )
            .into_response(),
    }
}

/// Enable a plugin
pub async fn enable_plugin(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager.enable_plugin(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Plugin not found or dependency error" })),
        )
            .into_response(),
    }
}

/// Disable a plugin
pub async fn disable_plugin(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager.disable_plugin(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Plugin not found" })),
        )
            .into_response(),
    }
}

/// Get plugin statistics
pub async fn get_plugin_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager.get_stats(&id) {
        Some(stats) => Json(stats).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Plugin not found" })),
        )
            .into_response(),
    }
}

/// Reload all plugins
pub async fn reload_plugins(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.plugin_manager.reload_all() {
        Ok(count) => (
            StatusCode::OK,
            Json(serde_json::json!({ "reloaded": count })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

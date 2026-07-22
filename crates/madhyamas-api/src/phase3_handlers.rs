//! Phase 3 handlers: gRPC, Scripting, Plugins

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use madhyamas_core::{GrpcDirection, GrpcFilter, Script, ScriptConfig, ScriptTemplates};
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
    let id = state.script_runtime.register_script(script.clone());
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id, "script": script })),
    )
        .into_response()
}

/// Update script request
#[derive(Debug, Deserialize, validator::Validate)]
pub struct UpdateScriptRequest {
    #[validate(length(min = 1))]
    pub source: String,
}

/// Update a script
pub async fn update_script(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScriptRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    if state.script_runtime.update_script(&id, req.source) {
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

/// Get script templates
pub async fn get_script_templates() -> impl IntoResponse {
    Json(vec![
        ScriptTemplates::log_requests(),
        ScriptTemplates::add_cors(),
        ScriptTemplates::block_domains(),
        ScriptTemplates::modify_headers(),
        ScriptTemplates::mock_api(),
    ])
}

/// Get script configuration
pub async fn get_script_config() -> impl IntoResponse {
    Json(ScriptConfig::default())
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

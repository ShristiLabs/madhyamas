//! API handlers for interception features (breakpoints, mocks, rewrites, throttling, replay)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use madhyamas_core::{
    BreakpointDecision, BreakpointRule, InterceptDirection, MatchCondition, MockResponse, MockRule,
    RequestModifications, RewriteAction, RewriteDirection, RewriteRule, SavedRequest,
    ThrottleProfile,
};
use serde::Deserialize;
use std::sync::Arc;

use super::handlers::ErrorResponse;
use super::AppState;

// ============================================================================
// Breakpoints
// ============================================================================

/// Get all breakpoint rules
pub async fn get_breakpoint_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rules = state.breakpoint_manager.get_rules();
    Json(rules)
}

/// Create a breakpoint rule
#[derive(Debug, Deserialize)]
pub struct CreateBreakpointRequest {
    pub name: String,
    pub condition: MatchCondition,
    pub direction: InterceptDirection,
    pub enabled: Option<bool>,
    pub priority: Option<u32>,
}

pub async fn create_breakpoint_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBreakpointRequest>,
) -> impl IntoResponse {
    let mut rule = BreakpointRule::new(req.name, req.condition, req.direction);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }

    let id = state.breakpoint_manager.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id })))
}

/// Get a specific breakpoint rule
pub async fn get_breakpoint_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let rules = state.breakpoint_manager.get_rules();
    match rules.into_iter().find(|r| r.id == id) {
        Some(rule) => Json(rule).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Rule not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Delete a breakpoint rule
pub async fn delete_breakpoint_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.breakpoint_manager.remove_rule(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Rule not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Get all paused traffic
pub async fn get_paused_traffic(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let paused = state.breakpoint_manager.get_paused();
    Json(paused)
}

/// Get a specific paused item
pub async fn get_paused_item(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.breakpoint_manager.get_paused_by_id(&id) {
        Some(paused) => Json(paused).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Paused item not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Resume a paused item
#[derive(Debug, Deserialize)]
pub struct ResumeRequest {
    pub action: BreakpointDecision,
}

pub async fn resume_paused_item(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ResumeRequest>,
) -> impl IntoResponse {
    if state.breakpoint_manager.resume(&id, req.action) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Paused item not found".to_string(),
            }),
        )
            .into_response()
    }
}

// ============================================================================
// Mocks
// ============================================================================

/// Get all mock rules
pub async fn get_mock_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rules = state.mock_manager.get_rules();
    Json(rules)
}

/// Create a mock rule
#[derive(Debug, Deserialize)]
pub struct CreateMockRequest {
    pub name: String,
    pub condition: MatchCondition,
    pub response: MockResponse,
    pub enabled: Option<bool>,
    pub priority: Option<u32>,
}

pub async fn create_mock_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMockRequest>,
) -> impl IntoResponse {
    let mut rule = MockRule::new(req.name, req.condition, req.response);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }

    let id = state.mock_manager.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id })))
}

/// Get a specific mock rule
pub async fn get_mock_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.mock_manager.get_rule(&id) {
        Some(rule) => Json(rule).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Mock rule not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Update a mock rule
pub async fn update_mock_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(rule): Json<MockRule>,
) -> impl IntoResponse {
    if state.mock_manager.update_rule(&id, rule) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Mock rule not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Delete a mock rule
pub async fn delete_mock_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.mock_manager.remove_rule(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Mock rule not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Toggle a mock rule
#[derive(Debug, Deserialize)]
pub struct ToggleRequest {
    pub enabled: bool,
}

pub async fn toggle_mock_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    if state.mock_manager.toggle_rule(&id, req.enabled) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Mock rule not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Get mock templates
pub async fn get_mock_templates() -> impl IntoResponse {
    Json(vec![
        serde_json::json!({
            "name": "JSON Response",
            "description": "Return a JSON response",
            "template": {
                "status_code": 200,
                "headers": { "Content-Type": "application/json" },
                "body": "{ \"message\": \"Hello, World!\" }"
            }
        }),
        serde_json::json!({
            "name": "404 Not Found",
            "description": "Return a 404 error",
            "template": {
                "status_code": 404,
                "headers": { "Content-Type": "application/json" },
                "body": "{ \"error\": \"Not Found\" }"
            }
        }),
        serde_json::json!({
            "name": "500 Server Error",
            "description": "Return a 500 error",
            "template": {
                "status_code": 500,
                "headers": { "Content-Type": "application/json" },
                "body": "{ \"error\": \"Internal Server Error\" }"
            }
        }),
        serde_json::json!({
            "name": "Slow Response",
            "description": "Return a delayed response",
            "template": {
                "status_code": 200,
                "headers": {},
                "body": "{}",
                "delay_ms": 3000
            }
        }),
    ])
}

// ============================================================================
// Rewrites
// ============================================================================

/// Get all rewrite rules
pub async fn get_rewrite_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rules = state.rewrite_manager.get_rules();
    Json(rules)
}

/// Create a rewrite rule
#[derive(Debug, Deserialize)]
pub struct CreateRewriteRequest {
    pub name: String,
    pub condition: MatchCondition,
    pub direction: RewriteDirection,
    pub rewrites: Vec<RewriteAction>,
    pub enabled: Option<bool>,
    pub priority: Option<u32>,
}

pub async fn create_rewrite_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRewriteRequest>,
) -> impl IntoResponse {
    let mut rule = RewriteRule::new(req.name, req.condition, req.direction, req.rewrites);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }

    let id = state.rewrite_manager.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id })))
}

/// Get a specific rewrite rule
pub async fn get_rewrite_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.rewrite_manager.get_rule(&id) {
        Some(rule) => Json(rule).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Rewrite rule not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Delete a rewrite rule
pub async fn delete_rewrite_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.rewrite_manager.remove_rule(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Rewrite rule not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Toggle a rewrite rule
pub async fn toggle_rewrite_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    if state.rewrite_manager.toggle_rule(&id, req.enabled) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Rewrite rule not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Get rewrite templates
pub async fn get_rewrite_templates() -> impl IntoResponse {
    Json(vec![
        serde_json::json!({
            "name": "Add CORS Headers",
            "description": "Add Access-Control-Allow-Origin headers to responses",
            "template": {
                "direction": "response",
                "rewrites": [
                    { "type": "set_header", "name": "Access-Control-Allow-Origin", "value": "*" },
                    { "type": "set_header", "name": "Access-Control-Allow-Methods", "value": "GET, POST, PUT, DELETE, OPTIONS" }
                ]
            }
        }),
        serde_json::json!({
            "name": "HTTP to HTTPS",
            "description": "Redirect HTTP requests to HTTPS",
            "template": {
                "direction": "request",
                "rewrites": [
                    { "type": "url_rewrite", "pattern": "^http://", "replacement": "https://" }
                ]
            }
        }),
        serde_json::json!({
            "name": "Add Auth Header",
            "description": "Add Authorization header to requests",
            "template": {
                "direction": "request",
                "rewrites": [
                    { "type": "set_header", "name": "Authorization", "value": "Bearer YOUR_TOKEN" }
                ]
            }
        }),
        serde_json::json!({
            "name": "Remove Security Headers",
            "description": "Remove CSP and other security headers for testing",
            "template": {
                "direction": "response",
                "rewrites": [
                    { "type": "remove_header", "name": "Content-Security-Policy" },
                    { "type": "remove_header", "name": "X-Frame-Options" }
                ]
            }
        }),
    ])
}

// ============================================================================
// Throttling
// ============================================================================

/// Get current throttle profile
pub async fn get_throttle_profile(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let profile = state.throttle_manager.get_profile();
    let enabled = state.throttle_manager.is_enabled();
    Json(serde_json::json!({
        "profile": profile,
        "enabled": enabled
    }))
}

/// Set throttle profile
#[derive(Debug, Deserialize)]
pub struct SetThrottleRequest {
    pub profile: ThrottleProfile,
    pub enabled: Option<bool>,
}

pub async fn set_throttle_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetThrottleRequest>,
) -> impl IntoResponse {
    state.throttle_manager.set_profile(req.profile);
    if let Some(enabled) = req.enabled {
        state.throttle_manager.set_enabled(enabled);
    }
    StatusCode::OK
}

/// Enable/disable throttling
pub async fn set_throttle_enabled(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    state.throttle_manager.set_enabled(req.enabled);
    StatusCode::OK
}

/// Get available throttle presets
pub async fn get_throttle_presets() -> impl IntoResponse {
    Json(ThrottleProfile::all())
}

// ============================================================================
// Replay
// ============================================================================

/// Get all saved requests
pub async fn get_saved_requests(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let requests = state.replay_manager.get_saved_requests();
    Json(requests)
}

/// Save a request for replay
#[derive(Debug, Deserialize)]
pub struct SaveRequestPayload {
    pub entry_id: Option<String>,
    pub request: madhyamas_core::RequestData,
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub collection: Option<String>,
}

pub async fn save_request(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveRequestPayload>,
) -> impl IntoResponse {
    let mut saved = match req.entry_id {
        Some(entry_id) => SavedRequest::from_traffic(&entry_id, req.request),
        None => SavedRequest::new(req.name.as_deref(), req.request),
    };

    saved.name = req.name;
    if let Some(tags) = req.tags {
        saved.tags = tags;
    }
    if let Some(collection) = req.collection {
        saved.collection = Some(collection);
    }

    let id = state.replay_manager.save_request(saved);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id })))
}

/// Get a specific saved request
pub async fn get_saved_request(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.replay_manager.get_request(&id) {
        Some(request) => Json(request).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Saved request not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Delete a saved request
pub async fn delete_saved_request(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.replay_manager.remove_request(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Saved request not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Replay a saved request
#[derive(Debug, Deserialize)]
pub struct ReplayRequest {
    pub modifications: Option<RequestModifications>,
}

pub async fn replay_request(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ReplayRequest>,
) -> impl IntoResponse {
    let result = state.replay_manager.replay(&id, req.modifications).await;
    Json(result)
}

/// Get replay history
pub async fn get_replay_history(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let history = state.replay_manager.get_history();
    Json(history)
}

/// Clear replay history
pub async fn clear_replay_history(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.replay_manager.clear_history();
    StatusCode::NO_CONTENT
}

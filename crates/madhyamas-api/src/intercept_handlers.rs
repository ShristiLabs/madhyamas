//! API handlers for interception features (breakpoints, mocks, rewrites, throttling, replay)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use madhyamas_core::{
    BlockListEntry, BreakpointDecision, BreakpointRule, InterceptDirection, MatchCondition,
    MockCollection, MockResponse, MockRule, RequestData, RequestModifications, ResponseConfig,
    RewriteAction, RewriteDirection, RewriteRule, SavedRequest, ThrottleProfile,
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
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateBreakpointRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(custom(function = "super::validation::validate_match_condition"))]
    pub condition: MatchCondition,
    pub direction: InterceptDirection,
    pub enabled: Option<bool>,
    pub priority: Option<u32>,
}

pub async fn create_breakpoint_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBreakpointRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let mut rule = BreakpointRule::new(req.name, req.condition, req.direction);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }

    let id = state.breakpoint_manager.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
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
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateMockRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(custom(function = "super::validation::validate_match_condition"))]
    pub condition: MatchCondition,
    #[validate(custom(function = "super::validation::validate_mock_response"))]
    pub response: MockResponse,
    pub enabled: Option<bool>,
    pub priority: Option<u32>,
}

pub async fn create_mock_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMockRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let mut rule = MockRule::new(req.name, req.condition, req.response);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }

    let id = state.mock_manager.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
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
    // Validate the incoming rule without requiring a Validate impl on the
    // core MockRule type.
    if rule.name.trim().is_empty() {
        return super::error::ApiError::bad_request("name cannot be empty").into_response();
    }
    if let Err(e) = super::validation::validate_match_condition(&rule.condition) {
        return super::error::ApiError::bad_request(e.to_string()).into_response();
    }
    if let Err(e) = super::validation::validate_response_config(&rule.response_config) {
        return super::error::ApiError::bad_request(e.to_string()).into_response();
    }
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

/// Batch toggle multiple mock rules at once
#[derive(Debug, Deserialize)]
pub struct BatchToggleRequest {
    pub ids: Vec<String>,
    pub enabled: bool,
}

pub async fn batch_toggle_mocks(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchToggleRequest>,
) -> impl IntoResponse {
    let mut updated = 0;
    let mut not_found: Vec<String> = Vec::new();
    for id in &req.ids {
        if state.mock_manager.toggle_rule(id, req.enabled) {
            updated += 1;
        } else {
            not_found.push(id.clone());
        }
    }
    Json(serde_json::json!({
        "updated": updated,
        "not_found": not_found,
        "total": req.ids.len(),
    }))
    .into_response()
}

/// Batch toggle multiple rewrite rules at once
pub async fn batch_toggle_rewrites(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchToggleRequest>,
) -> impl IntoResponse {
    let mut updated = 0;
    let mut not_found: Vec<String> = Vec::new();
    for id in &req.ids {
        if state.rewrite_manager.toggle_rule(id, req.enabled) {
            updated += 1;
        } else {
            not_found.push(id.clone());
        }
    }
    Json(serde_json::json!({
        "updated": updated,
        "not_found": not_found,
        "total": req.ids.len(),
    }))
    .into_response()
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
// Mock Collections
// ============================================================================

/// Get all mock collections
pub async fn get_mock_collections(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let collections = state.mock_manager.get_collections();
    Json(collections)
}

/// Create a mock collection
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateCollectionRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub async fn create_mock_collection(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCollectionRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let mut collection = MockCollection::new(req.name);
    if let Some(desc) = req.description {
        collection.description = Some(desc);
    }
    if let Some(tags) = req.tags {
        collection.tags = tags;
    }
    let id = state.mock_manager.add_collection(collection);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

/// Get a specific mock collection
pub async fn get_mock_collection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.mock_manager.get_collection(&id) {
        Some(collection) => Json(collection).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Collection not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Delete a mock collection
#[derive(Debug, Deserialize)]
pub struct DeleteCollectionRequest {
    pub delete_rules: Option<bool>,
}

pub async fn delete_mock_collection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<Option<DeleteCollectionRequest>>,
) -> impl IntoResponse {
    let delete_rules = req.and_then(|r| r.delete_rules).unwrap_or(false);
    if state.mock_manager.delete_collection(&id, delete_rules) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Collection not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Toggle a mock collection (enable/disable all rules in collection)
pub async fn toggle_mock_collection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    let count = state.mock_manager.toggle_collection(&id, req.enabled);
    if count > 0 {
        Json(serde_json::json!({ "toggled": count })).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Collection not found or no rules in collection".to_string(),
            }),
        )
            .into_response()
    }
}

// ============================================================================
// Mock Hit Analytics
// ============================================================================

/// Get hit history for all mocks
pub async fn get_mock_analytics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let history = state.mock_manager.get_all_hit_history();
    Json(history)
}

/// Get hit statistics for a specific mock
pub async fn get_mock_rule_analytics(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let stats = state.mock_manager.get_hit_stats(&id);
    Json(stats)
}

/// Get hit history for a specific mock
pub async fn get_mock_hit_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let history = state.mock_manager.get_hit_history(&id);
    Json(history)
}

/// Clear all hit history
pub async fn clear_mock_hit_history(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.mock_manager.clear_hit_history();
    StatusCode::NO_CONTENT
}

// ============================================================================
// Mock Testing & Preview
// ============================================================================

/// Test a mock rule against a sample request
#[derive(Debug, Deserialize)]
pub struct TestMockRequest {
    pub request: RequestData,
}

pub async fn test_mock_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<TestMockRequest>,
) -> impl IntoResponse {
    // Check if rule exists and matches the request
    if let Some(rule) = state.mock_manager.get_rule(&id) {
        let body_str = req
            .request
            .body
            .as_ref()
            .and_then(|b| std::str::from_utf8(b).ok());
        let matches = rule.condition.matches_request(
            &req.request.url,
            &req.request.method.to_string(),
            &req.request.headers,
            req.request.body.as_deref(),
            body_str,
        );
        Json(serde_json::json!({
            "matches": matches,
            "rule_id": id,
            "rule_name": rule.name
        }))
        .into_response()
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

/// Preview which mock would match a request
pub async fn preview_mock_match(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestMockRequest>,
) -> impl IntoResponse {
    match state.mock_manager.find_matching_mock(&req.request) {
        Some(rule) => Json(serde_json::json!({
            "matched": true,
            "rule_id": rule.id,
            "rule_name": rule.name,
            "response": rule.response()
        }))
        .into_response(),
        None => Json(serde_json::json!({
            "matched": false,
            "message": "No mock rule matches this request"
        }))
        .into_response(),
    }
}

// ============================================================================
// Mock Import/Export
// ============================================================================

/// Export mocks as JSON
pub async fn export_mocks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rules = state.mock_manager.export_rules();
    Json(rules)
}

/// Import mocks from HAR format
#[derive(Debug, Deserialize, validator::Validate)]
pub struct ImportMocksRequest {
    #[validate(length(min = 1))]
    pub format: String, // "har", "openapi", "postman"
    #[validate(length(min = 1))]
    pub data: String,
}

pub async fn import_mocks(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportMocksRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let result = match req.format.as_str() {
        "har" => state.mock_manager.import_from_har(&req.data),
        "openapi" => state.mock_manager.import_from_openapi(&req.data),
        "postman" => state.mock_manager.import_from_postman(&req.data),
        _ => Err(format!("Unsupported format: {}", req.format)),
    };

    match result {
        Ok(count) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "imported": count,
                "format": req.format
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Import failed: {}", e),
            }),
        )
            .into_response(),
    }
}

// ============================================================================
// Mock Recording
// ============================================================================

/// Set recording mode
#[derive(Debug, Deserialize)]
pub struct RecordingRequest {
    pub enabled: bool,
}

pub async fn set_mock_recording(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RecordingRequest>,
) -> impl IntoResponse {
    state.mock_manager.set_recording(req.enabled);
    Json(serde_json::json!({ "recording": req.enabled }))
}

/// Get recording status
pub async fn get_mock_recording_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let is_recording = state.mock_manager.is_recording();
    Json(serde_json::json!({ "recording": is_recording }))
}

/// Get recorded mocks
pub async fn get_recorded_mocks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mocks = state.mock_manager.get_recorded_mocks();
    Json(mocks)
}

/// Promote recorded mocks to active rules
pub async fn promote_recorded_mocks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let count = state.mock_manager.promote_recorded_mocks();
    Json(serde_json::json!({ "promoted": count }))
}

/// Clear recorded mocks
pub async fn clear_recorded_mocks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.mock_manager.clear_recorded_mocks();
    StatusCode::NO_CONTENT
}

// ============================================================================
// Mock Versioning
// ============================================================================

/// Duplicate a mock rule
#[derive(Debug, Deserialize)]
pub struct DuplicateRequest {
    pub new_name: Option<String>,
}

pub async fn duplicate_mock_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<Option<DuplicateRequest>>,
) -> impl IntoResponse {
    let new_name = req.and_then(|r| r.new_name);
    match state.mock_manager.duplicate_rule(&id, new_name) {
        Some(new_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "id": new_id })),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Mock rule not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Rollback a mock rule to a previous version
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub version: u32,
}

pub async fn rollback_mock_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<RollbackRequest>,
) -> impl IntoResponse {
    if state.mock_manager.rollback_rule(&id, req.version) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Mock rule or version not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Get version history for a mock rule
pub async fn get_mock_version_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.mock_manager.get_rule(&id) {
        Some(rule) => Json(rule.version_history).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Mock rule not found".to_string(),
            }),
        )
            .into_response(),
    }
}

// ============================================================================
// Enhanced Mock Creation
// ============================================================================

/// Create a mock rule with advanced configuration
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateAdvancedMockRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    #[validate(custom(function = "super::validation::validate_match_condition"))]
    pub condition: MatchCondition,
    #[validate(custom(function = "super::validation::validate_response_config"))]
    pub response_config: ResponseConfig,
    pub enabled: Option<bool>,
    pub priority: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub collection_id: Option<String>,
}

pub async fn create_advanced_mock_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAdvancedMockRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let mut rule = MockRule::with_config(req.name, req.condition, req.response_config);
    if let Some(desc) = req.description {
        rule.description = Some(desc);
    }
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }
    if let Some(tags) = req.tags {
        rule.tags = tags;
    }
    if let Some(collection_id) = req.collection_id {
        rule.collection_id = Some(collection_id);
    }

    let id = state.mock_manager.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
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
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateRewriteRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(custom(function = "super::validation::validate_match_condition"))]
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
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let mut rule = RewriteRule::new(req.name, req.condition, req.direction, req.rewrites);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }

    let id = state.rewrite_manager.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

/// Update a rewrite rule
#[derive(Debug, Deserialize, validator::Validate)]
pub struct UpdateRewriteRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(custom(function = "super::validation::validate_match_condition"))]
    pub condition: MatchCondition,
    pub direction: RewriteDirection,
    pub rewrites: Vec<RewriteAction>,
    pub enabled: Option<bool>,
    pub priority: Option<u32>,
}

pub async fn update_rewrite_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRewriteRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }

    // Preserve immutable fields (id, created_at, hit_count) from the
    // existing rule so callers can't overwrite them via the update payload.
    let existing = match state.rewrite_manager.get_rule(&id) {
        Some(rule) => rule,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Rewrite rule not found".to_string(),
                }),
            )
                .into_response();
        }
    };

    let mut rule = RewriteRule::new(req.name, req.condition, req.direction, req.rewrites);
    rule.id = existing.id;
    rule.created_at = existing.created_at;
    rule.hit_count = existing.hit_count;
    rule.enabled = req.enabled.unwrap_or(existing.enabled);
    rule.priority = req.priority.unwrap_or(existing.priority);

    if state.rewrite_manager.update_rule(&id, rule) {
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
        serde_json::json!({
            "name": "No Caching",
            "description": "Prevent client caching by stripping cache-related headers and adding no-cache directives. Ensures you always see the latest version.",
            "template": {
                "direction": "both",
                "rewrites": [
                    { "type": "remove_header", "name": "If-Modified-Since" },
                    { "type": "remove_header", "name": "If-None-Match" },
                    { "type": "remove_header", "name": "ETag" },
                    { "type": "remove_header", "name": "Last-Modified" },
                    { "type": "remove_header", "name": "Expires" },
                    { "type": "set_header", "name": "Cache-Control", "value": "no-cache, no-store, must-revalidate" },
                    { "type": "set_header", "name": "Pragma", "value": "no-cache" },
                    { "type": "set_header", "name": "Expires", "value": "0" }
                ]
            }
        }),
        serde_json::json!({
            "name": "Block Cookies",
            "description": "Strip Cookie and Set-Cookie headers from both requests and responses. Useful for testing how a site behaves for anonymous/first-time visitors.",
            "template": {
                "direction": "both",
                "rewrites": [
                    { "type": "remove_header", "name": "Cookie" },
                    { "type": "remove_header", "name": "Set-Cookie" }
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
#[derive(Debug, Deserialize, validator::Validate)]
pub struct SetThrottleRequest {
    #[validate(custom(function = "super::validation::validate_throttle_profile"))]
    pub profile: ThrottleProfile,
    pub enabled: Option<bool>,
}

pub async fn set_throttle_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetThrottleRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    state.throttle_manager.set_profile(req.profile);
    if let Some(enabled) = req.enabled {
        state.throttle_manager.set_enabled(enabled);
    }
    StatusCode::OK.into_response()
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
#[derive(Debug, Deserialize, validator::Validate)]
pub struct SaveRequestPayload {
    pub entry_id: Option<String>,
    pub request: madhyamas_core::RequestData,
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub collection: Option<String>,
}

pub async fn save_request(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveRequestPayload>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
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
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
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

// ============================================================================
// Block List
// ============================================================================

/// Get all block list entries
pub async fn get_block_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let entries = state.block_list_manager.get_entries();
    Json(entries)
}

/// Get block list summary statistics
pub async fn get_block_list_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let stats = state.block_list_manager.stats();
    Json(stats)
}

/// Create a block list entry
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateBlockListEntryRequest {
    #[validate(length(min = 1, max = 255))]
    pub pattern: String,
    pub note: Option<String>,
    pub enabled: Option<bool>,
    pub status_code: Option<u16>,
    pub response_body: Option<String>,
    pub content_type: Option<String>,
}

pub async fn create_block_list_entry(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBlockListEntryRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let mut entry = BlockListEntry::new(req.pattern);
    if let Some(note) = req.note {
        entry.note = Some(note);
    }
    if let Some(enabled) = req.enabled {
        entry.enabled = enabled;
    }
    if let Some(status_code) = req.status_code {
        entry.status_code = status_code;
    }
    if let Some(response_body) = req.response_body {
        entry.response_body = response_body;
    }
    if let Some(content_type) = req.content_type {
        entry.content_type = content_type;
    }

    let id = state.block_list_manager.add_entry(entry);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

/// Get a specific block list entry
pub async fn get_block_list_entry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.block_list_manager.get_entry(&id) {
        Some(entry) => Json(entry).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Block list entry not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Update a block list entry
pub async fn update_block_list_entry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(entry): Json<BlockListEntry>,
) -> impl IntoResponse {
    if entry.pattern.trim().is_empty() {
        return super::error::ApiError::bad_request("pattern cannot be empty").into_response();
    }
    if state.block_list_manager.update_entry(&id, entry) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Block list entry not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Delete a block list entry
pub async fn delete_block_list_entry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.block_list_manager.remove_entry(&id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Block list entry not found".to_string(),
            }),
        )
            .into_response()
    }
}

/// Toggle a block list entry
pub async fn toggle_block_list_entry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    if state.block_list_manager.toggle_entry(&id, req.enabled) {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Block list entry not found".to_string(),
            }),
        )
            .into_response()
    }
}

//! API handlers for interception features (breakpoints, mocks, rewrites, throttling, replay)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use madhyamas_core::{
    BlockListEntry, BreakpointDecision, BreakpointRule, InterceptDirection, MatchCondition,
    MockCollection, MockResponse, MockRule, ReplayBatchConfig, RequestData, RequestModifications,
    ResponseConfig, RewriteAction, RewriteDirection, RewriteRule, SavedRequest, ThrottleProfile,
};
use serde::Deserialize;
use std::sync::Arc;

use super::auth::{AuditEvent, AuditEventType, RuleActor};
use super::handlers::ErrorResponse;
use super::AppState;

// ============================================================================
// Device-scope enforcement helpers (issue #109)
// ============================================================================

/// Deserialize helper for the `device_id: Option<Option<String>>` request
/// fields: without it serde maps BOTH a missing key and an explicit
/// `null` to `None`, collapsing "use the default scope" (absent) into
/// "explicitly global" (`null`) — the two cases the #109 enforcement
/// must distinguish (agents get the former, are rejected on the latter).
fn double_option<'de, D>(de: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(de).map(Some)
}

/// Resolve the effective device scope for a rule the principal is
/// creating (issue #109).
///
/// - **Agent keys** (device-bound, `RuleActor.device_id = Some(X)`)
///   create rules scoped to their parent device by **default**. An
///   explicit global (`null`) or a foreign device ID is rejected —
///   agents cannot create global or cross-device rules.
/// - **Every other principal** (JWT, user API key, OSS unauthenticated)
///   passes the requested scope through unchanged (`None` = global).
///
/// The `requested` triple-Option distinguishes a missing field (`None`,
/// default applies), an explicit `null` (`Some(None)`, global) and an
/// explicit device ID (`Some(Some(id))`).
fn resolve_rule_scope(
    actor: Option<&RuleActor>,
    requested: Option<Option<&str>>,
) -> Result<Option<String>, (StatusCode, String)> {
    match actor.and_then(|a| a.device_id.as_deref()) {
        Some(parent) => match requested {
            // Absent or explicitly the parent device: scoped to parent.
            None => Ok(Some(parent.to_string())),
            Some(Some(id)) if id == parent => Ok(Some(parent.to_string())),
            Some(None) => Err((
                StatusCode::FORBIDDEN,
                "agent keys cannot create global rules; rules are scoped to the parent device"
                    .to_string(),
            )),
            Some(Some(_)) => Err((
                StatusCode::FORBIDDEN,
                "agent keys may only create rules scoped to their parent device".to_string(),
            )),
        },
        None => Ok(requested.flatten().map(|id| id.to_string())),
    }
}

/// Whether a rule with the given device scope is visible to the actor
/// (issue #109). Non-agent principals see everything; an agent key sees
/// only rules scoped to its parent device — global and other-device
/// rules are hidden (and mutations on them return 404, indistinguishable
/// from a missing rule).
fn rule_visible(actor: Option<&RuleActor>, rule_device: Option<&str>) -> bool {
    match actor.and_then(|a| a.device_id.as_deref()) {
        Some(parent) => rule_device == Some(parent),
        None => true,
    }
}

/// The forced device scope for agent-key writes that must not change a
/// rule's scope (full-replace updates, imports, promotions). `None` for
/// non-agent principals — they manage every scope.
fn forced_scope(actor: Option<&RuleActor>) -> Option<String> {
    actor.and_then(|a| a.device_id.clone())
}

/// Identity of the rule a mutation touched, for
/// [`audit_rule_mutation`].
struct RuleRef<'a> {
    rule_type: &'a str,
    rule_id: &'a str,
    rule_name: Option<&'a str>,
}

/// Emit an audit event for a rule mutation (issue #109): the acting
/// principal's user/key and the rule's device scope ride on the event.
/// Fire-and-forget — a failed audit write must not fail the mutation,
/// and the OSS tier has no sink at all.
fn audit_rule_mutation(
    state: &AppState,
    actor: Option<&RuleActor>,
    event_type: AuditEventType,
    action: &str,
    rule: RuleRef<'_>,
    device_id: Option<&str>,
) {
    if let Some(sink) = &state.audit_sink {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("action".to_string(), serde_json::json!(action));
        metadata.insert("rule_type".to_string(), serde_json::json!(rule.rule_type));
        metadata.insert("rule_id".to_string(), serde_json::json!(rule.rule_id));
        if let Some(name) = rule.rule_name {
            metadata.insert("rule_name".to_string(), serde_json::json!(name));
        }
        metadata.insert("device_id".to_string(), serde_json::json!(device_id));
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp: chrono::Utc::now(),
            user_id: actor.and_then(|a| a.user_id.clone()),
            api_key_id: actor.and_then(|a| a.key_id.clone()),
            client_ip: None,
            description: format!("{} {} rule", action, rule.rule_type),
            metadata,
        };
        let sink = Arc::clone(sink);
        tokio::spawn(async move {
            if let Err(e) = sink.log_event(event).await {
                tracing::warn!("rule-mutation audit write failed: {}", e);
            }
        });
    }
}

/// 404 response for a rule the actor cannot see (global or another
/// device's, for agent keys) — identical to a missing rule.
fn rule_not_found(what: &str) -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("{} not found", what),
        }),
    )
        .into_response()
}

// ============================================================================
// Breakpoints
// ============================================================================

/// Get all breakpoint rules
pub async fn get_breakpoint_rules(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    let rules = state.breakpoint_manager.get_rules();
    let visible: Vec<BreakpointRule> = rules
        .into_iter()
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()))
        .collect();
    Json(visible)
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
    /// Device scope (issue #109). Absent: agent keys default to their
    /// parent device, other principals to global. Explicit `null`
    /// (global) is rejected for agent keys; a foreign device ID is
    /// rejected for agent keys.
    #[serde(default, deserialize_with = "double_option")]
    pub device_id: Option<Option<String>>,
}

pub async fn create_breakpoint_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<CreateBreakpointRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let device_scope = match resolve_rule_scope(
        actor.as_deref(),
        req.device_id.as_ref().map(|inner| inner.as_deref()),
    ) {
        Ok(scope) => scope,
        Err((status, msg)) => {
            return (status, Json(ErrorResponse { error: msg })).into_response();
        }
    };
    let mut rule = BreakpointRule::new(req.name, req.condition, req.direction);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }
    rule.device_id = device_scope.clone();

    let id = state.breakpoint_manager.add_rule(rule).await;
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::BreakpointCreated,
        "create",
        RuleRef {
            rule_type: "breakpoint",
            rule_id: &id,
            rule_name: None,
        },
        device_scope.as_deref(),
    );
    super::pubsub::notify(
        &state.event_publisher,
        madhyamas_core::CHANNEL_INTERCEPT_EVENT,
        "breakpoint-created",
    );
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

/// Get a specific breakpoint rule
pub async fn get_breakpoint_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let rules = state.breakpoint_manager.get_rules();
    match rules
        .into_iter()
        .find(|r| r.id == id && rule_visible(actor.as_deref(), r.device_id.as_deref()))
    {
        Some(rule) => Json(rule).into_response(),
        None => rule_not_found("Rule"),
    }
}

/// Delete a breakpoint rule
pub async fn delete_breakpoint_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let existing = state.breakpoint_manager.get_rules();
    let visible = existing
        .iter()
        .find(|r| r.id == id && rule_visible(actor.as_deref(), r.device_id.as_deref()));
    let Some(rule) = visible else {
        return rule_not_found("Rule");
    };
    let (rule_name, rule_device) = (rule.name.clone(), rule.device_id.clone());
    if state.breakpoint_manager.remove_rule(&id).await {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::BreakpointDeleted,
            "delete",
            RuleRef {
                rule_type: "breakpoint",
                rule_id: &id,
                rule_name: Some(&rule_name),
            },
            rule_device.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "breakpoint-deleted",
        );
        StatusCode::NO_CONTENT.into_response()
    } else {
        rule_not_found("Rule")
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
pub async fn get_mock_rules(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    let rules = state.mock_manager.get_rules();
    let visible: Vec<MockRule> = rules
        .into_iter()
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()))
        .collect();
    Json(visible)
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
    /// Device scope (issue #109). Absent: agent keys default to their
    /// parent device, other principals to global. Explicit `null`
    /// (global) is rejected for agent keys; a foreign device ID is
    /// rejected for agent keys.
    #[serde(default, deserialize_with = "double_option")]
    pub device_id: Option<Option<String>>,
}

pub async fn create_mock_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<CreateMockRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let device_scope = match resolve_rule_scope(
        actor.as_deref(),
        req.device_id.as_ref().map(|inner| inner.as_deref()),
    ) {
        Ok(scope) => scope,
        Err((status, msg)) => {
            return (status, Json(ErrorResponse { error: msg })).into_response();
        }
    };
    let mut rule = MockRule::new(req.name, req.condition, req.response);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }
    rule.device_id = device_scope.clone();

    let id = state.mock_manager.add_rule(rule).await;
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::MockCreated,
        "create",
        RuleRef {
            rule_type: "mock",
            rule_id: &id,
            rule_name: None,
        },
        device_scope.as_deref(),
    );
    super::pubsub::notify(
        &state.event_publisher,
        madhyamas_core::CHANNEL_INTERCEPT_EVENT,
        "mock-created",
    );
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

/// Get a specific mock rule
pub async fn get_mock_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.mock_manager.get_rule(&id) {
        Some(rule) if rule_visible(actor.as_deref(), rule.device_id.as_deref()) => {
            Json(rule).into_response()
        }
        _ => rule_not_found("Mock rule"),
    }
}

/// Update a mock rule
pub async fn update_mock_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    mut rule: Json<MockRule>,
) -> impl IntoResponse {
    // Visibility (issue #109): agent keys may only update their parent
    // device's rules; global and other-device rules 404.
    let existing_scope = match state.mock_manager.get_rule(&id) {
        Some(existing) if rule_visible(actor.as_deref(), existing.device_id.as_deref()) => {
            existing.device_id.clone()
        }
        _ => return rule_not_found("Mock rule"),
    };
    // Agent keys cannot change a rule's device scope through a
    // full-replace update — the scope is forced back to the parent
    // device. Other principals: an explicit device_id in the payload
    // sets that scope; an absent or null field PRESERVES the existing
    // scope (the full-rule body cannot distinguish the two). To
    // globalize a scoped mock the owner recreates it (rewrite rules,
    // whose update request carries the tri-state field, can change
    // scope freely).
    rule.device_id = match forced_scope(actor.as_deref()) {
        Some(forced) => Some(forced),
        None => rule.device_id.take().or(existing_scope),
    };
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
    let (rule_id, rule_name, rule_device) = (id.clone(), rule.name.clone(), rule.device_id.clone());
    if state.mock_manager.update_rule(&id, rule.0) {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "update",
            RuleRef {
                rule_type: "mock",
                rule_id: &rule_id,
                rule_name: Some(&rule_name),
            },
            rule_device.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "mock-updated",
        );
        StatusCode::OK.into_response()
    } else {
        rule_not_found("Mock rule")
    }
}

/// Delete a mock rule
pub async fn delete_mock_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let visible = state
        .mock_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    let Some(rule) = visible else {
        return rule_not_found("Mock rule");
    };
    let (rule_name, rule_device) = (rule.name.clone(), rule.device_id.clone());
    if state.mock_manager.remove_rule(&id).await {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::MockDeleted,
            "delete",
            RuleRef {
                rule_type: "mock",
                rule_id: &id,
                rule_name: Some(&rule_name),
            },
            rule_device.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "mock-deleted",
        );
        StatusCode::NO_CONTENT.into_response()
    } else {
        rule_not_found("Mock rule")
    }
}

/// Toggle a mock rule
#[derive(Debug, Deserialize)]
pub struct ToggleRequest {
    pub enabled: bool,
}

pub async fn toggle_mock_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    let visible = state
        .mock_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    let Some(rule) = visible else {
        return rule_not_found("Mock rule");
    };
    if state.mock_manager.toggle_rule(&id, req.enabled) {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "toggle",
            RuleRef {
                rule_type: "mock",
                rule_id: &id,
                rule_name: Some(&rule.name),
            },
            rule.device_id.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "mock-toggled",
        );
        StatusCode::OK.into_response()
    } else {
        rule_not_found("Mock rule")
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
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<BatchToggleRequest>,
) -> impl IntoResponse {
    let mut updated = 0;
    let mut not_found: Vec<String> = Vec::new();
    for id in &req.ids {
        // Device axis (issue #109): foreign/global ids behave exactly
        // like missing ids for agent keys.
        let visible = state
            .mock_manager
            .get_rule(id)
            .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
        match (visible, state.mock_manager.toggle_rule(id, req.enabled)) {
            (Some(rule), true) => {
                updated += 1;
                audit_rule_mutation(
                    &state,
                    actor.as_deref(),
                    AuditEventType::Custom,
                    "batch_toggle",
                    RuleRef {
                        rule_type: "mock",
                        rule_id: id,
                        rule_name: Some(&rule.name),
                    },
                    rule.device_id.as_deref(),
                );
            }
            _ => not_found.push(id.clone()),
        }
    }
    Json(serde_json::json!({
        "updated": updated,
        "not_found": not_found,
        "total": req.ids.len(),
    }))
    .into_response()
}

// ============================================================================
// Create mocks from captured traffic
// ============================================================================

/// Create a mock rule from one or more captured traffic entries.
/// The mock's condition matches the request URL (regex-escaped) and the
/// response is replayed from the captured response.
#[derive(Debug, Deserialize)]
pub struct CreateMockFromTrafficRequest {
    /// Traffic entry IDs to create mocks from.
    pub entry_ids: Vec<String>,
    /// Optional name prefix. If omitted, uses "Mock: {METHOD} {URL}".
    pub name_prefix: Option<String>,
    /// Whether the created mock rules should be enabled (default: true).
    pub enabled: Option<bool>,
}

pub async fn create_mock_from_traffic(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<CreateMockFromTrafficRequest>,
) -> impl IntoResponse {
    if req.entry_ids.is_empty() {
        return super::error::ApiError::bad_request("entry_ids cannot be empty").into_response();
    }

    // Default scoping (issue #109): agent keys create parent-device-
    // scoped mocks; other principals create global mocks as before.
    let device_scope = forced_scope(actor.as_deref());

    let mut created_ids: Vec<String> = Vec::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();
    let enabled = req.enabled.unwrap_or(true);

    for entry_id in &req.entry_ids {
        match state.traffic_store.get_by_id(entry_id).await {
            Ok(Some(entry)) => {
                let request = &entry.request;
                let response = match &entry.response {
                    Some(r) => r,
                    None => {
                        errors.push(serde_json::json!({
                            "entry_id": entry_id,
                            "error": "No response captured for this entry",
                        }));
                        continue;
                    }
                };

                let name = match &req.name_prefix {
                    Some(prefix) => format!("{}: {} {}", prefix, request.method, request.url),
                    None => format!("Mock: {} {}", request.method, request.url),
                };

                let condition = MatchCondition::UrlPattern {
                    pattern: regex::escape(&request.url),
                };

                let mock_response = MockResponse {
                    status_code: response.status_code,
                    headers: response.headers.clone(),
                    body: response
                        .body
                        .as_ref()
                        .and_then(|b| String::from_utf8(b.clone()).ok()),
                    ..Default::default()
                };

                let mut rule = MockRule::new(name, condition, mock_response);
                rule.enabled = enabled;
                rule.device_id = device_scope.clone();

                let id = state.mock_manager.add_rule(rule).await;
                audit_rule_mutation(
                    &state,
                    actor.as_deref(),
                    AuditEventType::MockCreated,
                    "create_from_traffic",
                    RuleRef {
                        rule_type: "mock",
                        rule_id: &id,
                        rule_name: None,
                    },
                    device_scope.as_deref(),
                );
                created_ids.push(id);
            }
            Ok(None) => {
                errors.push(serde_json::json!({
                    "entry_id": entry_id,
                    "error": "Traffic entry not found",
                }));
            }
            Err(e) => {
                errors.push(serde_json::json!({
                    "entry_id": entry_id,
                    "error": e.to_string(),
                }));
            }
        }
    }

    let status = if created_ids.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::CREATED
    };

    (
        status,
        Json(serde_json::json!({
            "created": created_ids.len(),
            "ids": created_ids,
            "errors": errors,
            "total": req.entry_ids.len(),
        })),
    )
        .into_response()
}

/// Batch toggle multiple rewrite rules at once
pub async fn batch_toggle_rewrites(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<BatchToggleRequest>,
) -> impl IntoResponse {
    let mut updated = 0;
    let mut not_found: Vec<String> = Vec::new();
    for id in &req.ids {
        // Device axis (issue #109): foreign/global ids behave exactly
        // like missing ids for agent keys.
        let visible = state
            .rewrite_manager
            .get_rule(id)
            .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
        match (visible, state.rewrite_manager.toggle_rule(id, req.enabled)) {
            (Some(rule), true) => {
                updated += 1;
                audit_rule_mutation(
                    &state,
                    actor.as_deref(),
                    AuditEventType::Custom,
                    "batch_toggle",
                    RuleRef {
                        rule_type: "rewrite",
                        rule_id: id,
                        rule_name: Some(&rule.name),
                    },
                    rule.device_id.as_deref(),
                );
            }
            _ => not_found.push(id.clone()),
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
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<Option<DeleteCollectionRequest>>,
) -> impl IntoResponse {
    // Collections are global grouping objects (issue #109 leaves them
    // unscoped), so a deletion that cascades to member rules — which can
    // span every device scope — is owner/JWT territory. Agent keys may
    // delete only the collection shell, never its rules.
    let delete_rules = req.and_then(|r| r.delete_rules).unwrap_or(false);
    if delete_rules && actor.as_deref().is_some_and(|a| a.device_id.is_some()) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "agent keys cannot delete collection rules; delete rules individually"
                    .to_string(),
            }),
        )
            .into_response();
    }
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

/// Update a mock collection's metadata (name, description, enabled, tags).
///
/// Accepts a partial body: only the provided fields are updated; omitted
/// fields retain their existing values. The `id`, `created_at` fields are
/// preserved from the existing collection.
#[derive(Debug, Deserialize)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub tags: Option<Vec<String>>,
}

pub async fn update_mock_collection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCollectionRequest>,
) -> impl IntoResponse {
    let existing = match state.mock_manager.get_collection(&id) {
        Some(c) => c,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Collection not found".to_string(),
                }),
            )
                .into_response();
        }
    };

    let mut updated = existing.clone();
    if let Some(name) = req.name {
        updated.name = name;
    }
    if let Some(desc) = req.description {
        updated.description = Some(desc);
    }
    if let Some(enabled) = req.enabled {
        updated.enabled = enabled;
    }
    if let Some(tags) = req.tags {
        updated.tags = tags;
    }

    if state.mock_manager.update_collection(&id, updated.clone()) {
        Json(updated).into_response()
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
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    // Collections group rules across device scopes (issue #109); toggling
    // one flips every member rule, so it is owner/JWT territory.
    if actor.as_deref().is_some_and(|a| a.device_id.is_some()) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "agent keys cannot toggle collections; toggle rules individually"
                    .to_string(),
            }),
        )
            .into_response();
    }
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
pub async fn get_mock_analytics(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    let history = state.mock_manager.get_all_hit_history();
    // Device axis (issue #109): agent keys see hit records only for their
    // own device's rules — global and other-device rules' existence and
    // activity must not be disclosed through analytics.
    let history = match actor.as_deref().and_then(|a| a.device_id.as_deref()) {
        Some(_) => {
            let visible_ids: std::collections::HashSet<String> = state
                .mock_manager
                .get_rules()
                .into_iter()
                .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()))
                .map(|r| r.id)
                .collect();
            history
                .into_iter()
                .filter(|h| visible_ids.contains(&h.mock_id))
                .collect()
        }
        None => history,
    };
    Json(history)
}

/// Get hit statistics for a specific mock
pub async fn get_mock_rule_analytics(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Device axis (issue #109): stats for an invisible rule 404 — the
    // same existence-hiding as every other per-rule read.
    let visible = state
        .mock_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    if visible.is_none() {
        return rule_not_found("Mock rule");
    }
    let stats = state.mock_manager.get_hit_stats(&id);
    Json(stats).into_response()
}

/// Get hit history for a specific mock
pub async fn get_mock_hit_history(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Device axis (issue #109): history for an invisible rule 404.
    let visible = state
        .mock_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    if visible.is_none() {
        return rule_not_found("Mock rule");
    }
    let history = state.mock_manager.get_hit_history(&id);
    Json(history).into_response()
}

/// Clear all hit history
pub async fn clear_mock_hit_history(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    // Device axis (issue #109): clearing wipes analytics across every
    // rule scope — owner/JWT territory, like every other cross-scope
    // mutation.
    if actor.as_deref().is_some_and(|a| a.device_id.is_some()) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "agent keys cannot clear hit history across all rules".to_string(),
            }),
        )
            .into_response();
    }
    state.mock_manager.clear_hit_history();
    StatusCode::NO_CONTENT.into_response()
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
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<TestMockRequest>,
) -> impl IntoResponse {
    // Check if rule exists and matches the request
    if let Some(rule) = state
        .mock_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()))
    {
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
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<TestMockRequest>,
) -> impl IntoResponse {
    // The preview is an unattributed context (no device on the test
    // request), so device-scoped rules never match. Global rules DO
    // match — but they are invisible to agent keys (issue #109), so a
    // global match is reported as "no match" rather than disclosing the
    // rule to a device-bound principal.
    match state
        .mock_manager
        .find_matching_mock(&req.request, None)
        .filter(|rule| rule_visible(actor.as_deref(), rule.device_id.as_deref()))
    {
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
pub async fn export_mocks(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    // Device axis (issue #109): agent keys export only their parent
    // device's rules — global and other-device rules never leave.
    let rules: Vec<MockRule> = state
        .mock_manager
        .export_rules()
        .into_iter()
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()))
        .collect();
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
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<ImportMocksRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    // Default scoping (issue #109): imported rules land in the agent's
    // parent-device namespace; other principals import global rules.
    let device_scope = forced_scope(actor.as_deref());
    let result = match req.format.as_str() {
        "har" => state
            .mock_manager
            .import_from_har(&req.data, device_scope.clone()),
        "openapi" => state
            .mock_manager
            .import_from_openapi(&req.data, device_scope.clone()),
        "postman" => state
            .mock_manager
            .import_from_postman(&req.data, device_scope.clone()),
        _ => Err(format!("Unsupported format: {}", req.format)),
    };

    match result {
        Ok(count) => {
            audit_rule_mutation(
                &state,
                actor.as_deref(),
                AuditEventType::Custom,
                "import",
                RuleRef {
                    rule_type: "mock",
                    rule_id: &format!("{}:batch", req.format),
                    rule_name: None,
                },
                device_scope.as_deref(),
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "imported": count,
                    "format": req.format
                })),
            )
                .into_response()
        }
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
pub async fn promote_recorded_mocks(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    // Default scoping (issue #109): agent keys promote into their parent
    // device's namespace.
    let device_scope = forced_scope(actor.as_deref());
    let count = state
        .mock_manager
        .promote_recorded_mocks(device_scope.clone());
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::Custom,
        "promote",
        RuleRef {
            rule_type: "mock",
            rule_id: "recorded:batch",
            rule_name: None,
        },
        device_scope.as_deref(),
    );
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
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<Option<DuplicateRequest>>,
) -> impl IntoResponse {
    let new_name = req.and_then(|r| r.new_name);
    let visible = state
        .mock_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    let Some(source) = visible else {
        return rule_not_found("Mock rule");
    };
    let (source_name, source_device) = (source.name.clone(), source.device_id.clone());
    match state.mock_manager.duplicate_rule(&id, new_name) {
        Some(new_id) => {
            // The duplicate inherits the source's device scope.
            audit_rule_mutation(
                &state,
                actor.as_deref(),
                AuditEventType::MockCreated,
                "duplicate",
                RuleRef {
                    rule_type: "mock",
                    rule_id: &new_id,
                    rule_name: Some(&source_name),
                },
                source_device.as_deref(),
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({ "id": new_id })),
            )
                .into_response()
        }
        None => rule_not_found("Mock rule"),
    }
}

/// Rollback a mock rule to a previous version
#[derive(Debug, Deserialize)]
pub struct RollbackRequest {
    pub version: u32,
}

pub async fn rollback_mock_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<RollbackRequest>,
) -> impl IntoResponse {
    let visible = state
        .mock_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    let Some(rule) = visible else {
        return rule_not_found("Mock rule or version");
    };
    if state.mock_manager.rollback_rule(&id, req.version) {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "rollback",
            RuleRef {
                rule_type: "mock",
                rule_id: &id,
                rule_name: Some(&rule.name),
            },
            rule.device_id.as_deref(),
        );
        StatusCode::OK.into_response()
    } else {
        rule_not_found("Mock rule or version")
    }
}

/// Get version history for a mock rule
pub async fn get_mock_version_history(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.mock_manager.get_rule(&id) {
        Some(rule) if rule_visible(actor.as_deref(), rule.device_id.as_deref()) => {
            Json(rule.version_history).into_response()
        }
        _ => rule_not_found("Mock rule"),
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
    /// Device scope (issue #109); same semantics as
    /// [`CreateMockRequest::device_id`].
    #[serde(default, deserialize_with = "double_option")]
    pub device_id: Option<Option<String>>,
}

pub async fn create_advanced_mock_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<CreateAdvancedMockRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let device_scope = match resolve_rule_scope(
        actor.as_deref(),
        req.device_id.as_ref().map(|inner| inner.as_deref()),
    ) {
        Ok(scope) => scope,
        Err((status, msg)) => {
            return (status, Json(ErrorResponse { error: msg })).into_response();
        }
    };
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
    rule.device_id = device_scope.clone();

    let id = state.mock_manager.add_rule(rule).await;
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::MockCreated,
        "create",
        RuleRef {
            rule_type: "mock",
            rule_id: &id,
            rule_name: None,
        },
        device_scope.as_deref(),
    );
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

// ============================================================================
// Rewrites
// ============================================================================

/// Get all rewrite rules
pub async fn get_rewrite_rules(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    let rules = state.rewrite_manager.get_rules();
    let visible: Vec<RewriteRule> = rules
        .into_iter()
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()))
        .collect();
    Json(visible)
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
    /// Device scope (issue #109); same semantics as
    /// [`CreateMockRequest::device_id`].
    #[serde(default, deserialize_with = "double_option")]
    pub device_id: Option<Option<String>>,
}

pub async fn create_rewrite_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<CreateRewriteRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let device_scope = match resolve_rule_scope(
        actor.as_deref(),
        req.device_id.as_ref().map(|inner| inner.as_deref()),
    ) {
        Ok(scope) => scope,
        Err((status, msg)) => {
            return (status, Json(ErrorResponse { error: msg })).into_response();
        }
    };
    let mut rule = RewriteRule::new(req.name, req.condition, req.direction, req.rewrites);
    if let Some(enabled) = req.enabled {
        rule.enabled = enabled;
    }
    if let Some(priority) = req.priority {
        rule.priority = priority;
    }
    rule.device_id = device_scope.clone();

    let id = state.rewrite_manager.add_rule(rule).await;
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::Custom,
        "create",
        RuleRef {
            rule_type: "rewrite",
            rule_id: &id,
            rule_name: None,
        },
        device_scope.as_deref(),
    );
    super::pubsub::notify(
        &state.event_publisher,
        madhyamas_core::CHANNEL_INTERCEPT_EVENT,
        "rewrite-created",
    );
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
    /// Device scope (issue #109). Absent keeps the existing scope for
    /// non-agent principals; explicit values follow create semantics.
    #[serde(default, deserialize_with = "double_option")]
    pub device_id: Option<Option<String>>,
}

pub async fn update_rewrite_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRewriteRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }

    // Preserve immutable fields (id, created_at, hit_count) from the
    // existing rule so callers can't overwrite them via the update payload.
    let existing = match state.rewrite_manager.get_rule(&id) {
        Some(rule) if rule_visible(actor.as_deref(), rule.device_id.as_deref()) => rule,
        _ => return rule_not_found("Rewrite rule"),
    };

    // Device scope (issue #109): agent keys keep the rule pinned to the
    // parent device (no scope change via update); other principals keep
    // the existing scope when the field is absent and may change it
    // explicitly otherwise.
    let device_scope = match forced_scope(actor.as_deref()) {
        Some(forced) => Some(forced),
        None => match req.device_id.as_ref().map(|inner| inner.as_deref()) {
            None => existing.device_id.clone(),
            Some(Some(id)) => Some(id.to_string()),
            Some(None) => None,
        },
    };

    let mut rule = RewriteRule::new(req.name, req.condition, req.direction, req.rewrites);
    rule.id = existing.id;
    rule.created_at = existing.created_at;
    rule.hit_count = existing.hit_count;
    rule.enabled = req.enabled.unwrap_or(existing.enabled);
    rule.priority = req.priority.unwrap_or(existing.priority);
    rule.device_id = device_scope.clone();

    if state.rewrite_manager.update_rule(&id, rule).await {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "update",
            RuleRef {
                rule_type: "rewrite",
                rule_id: &id,
                rule_name: None,
            },
            device_scope.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "rewrite-updated",
        );
        StatusCode::OK.into_response()
    } else {
        rule_not_found("Rewrite rule")
    }
}

/// Get a specific rewrite rule
pub async fn get_rewrite_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.rewrite_manager.get_rule(&id) {
        Some(rule) if rule_visible(actor.as_deref(), rule.device_id.as_deref()) => {
            Json(rule).into_response()
        }
        _ => rule_not_found("Rewrite rule"),
    }
}

/// Delete a rewrite rule
pub async fn delete_rewrite_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let visible = state
        .rewrite_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    let Some(rule) = visible else {
        return rule_not_found("Rewrite rule");
    };
    if state.rewrite_manager.remove_rule(&id).await {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "delete",
            RuleRef {
                rule_type: "rewrite",
                rule_id: &id,
                rule_name: Some(&rule.name),
            },
            rule.device_id.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "rewrite-deleted",
        );
        StatusCode::NO_CONTENT.into_response()
    } else {
        rule_not_found("Rewrite rule")
    }
}

/// Toggle a rewrite rule
pub async fn toggle_rewrite_rule(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    let visible = state
        .rewrite_manager
        .get_rule(&id)
        .filter(|r| rule_visible(actor.as_deref(), r.device_id.as_deref()));
    let Some(rule) = visible else {
        return rule_not_found("Rewrite rule");
    };
    if state.rewrite_manager.toggle_rule(&id, req.enabled) {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "toggle",
            RuleRef {
                rule_type: "rewrite",
                rule_id: &id,
                rule_name: Some(&rule.name),
            },
            rule.device_id.as_deref(),
        );
        StatusCode::OK.into_response()
    } else {
        rule_not_found("Rewrite rule")
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
pub async fn get_throttle_profile(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    let profile = state.throttle_manager.get_profile();
    let enabled = state.throttle_manager.is_enabled();
    // Device axis (issue #109): an agent key sees the profile only when
    // it is scoped to the agent's parent device; the global (owner-set)
    // profile is invisible, reported as no profile / disabled.
    let (visible, effective_enabled) =
        if rule_visible(actor.as_deref(), profile.device_id.as_deref()) {
            (profile, enabled)
        } else {
            (ThrottleProfile::none(), false)
        };
    Json(serde_json::json!({
        "profile": visible,
        "enabled": effective_enabled
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
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<SetThrottleRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    // The throttle profile is a single active row (singleton). Setting a
    // device-scoped profile replaces whatever profile was active —
    // global included; the owner can re-set a global profile at any time
    // (issue #109). Agent keys always write their parent-device scope;
    // other principals honor the payload's `device_id` (absent = global).
    let mut profile = req.profile;
    profile.device_id = match forced_scope(actor.as_deref()) {
        Some(forced) => Some(forced),
        None => profile.device_id.take(),
    };
    let device_scope = profile.device_id.clone();
    state.throttle_manager.set_profile(profile).await;
    if let Some(enabled) = req.enabled {
        state.throttle_manager.set_enabled(enabled).await;
    }
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::Custom,
        "set_profile",
        RuleRef {
            rule_type: "throttle",
            rule_id: "singleton",
            rule_name: None,
        },
        device_scope.as_deref(),
    );
    super::pubsub::notify(
        &state.event_publisher,
        madhyamas_core::CHANNEL_INTERCEPT_EVENT,
        "throttle-updated",
    );
    StatusCode::OK.into_response()
}

/// Enable/disable throttling
pub async fn set_throttle_enabled(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    // Device axis (issue #109): toggling the singleton flips the active
    // profile wherever it is scoped. An agent key may only toggle a
    // profile scoped to its own device — toggling a global (owner-set)
    // profile would mutate a rule the agent cannot see.
    let profile = state.throttle_manager.get_profile();
    if !rule_visible(actor.as_deref(), profile.device_id.as_deref()) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "the active throttle profile is not scoped to this agent's device"
                    .to_string(),
            }),
        )
            .into_response();
    }
    state.throttle_manager.set_enabled(req.enabled).await;
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::Custom,
        "set_enabled",
        RuleRef {
            rule_type: "throttle",
            rule_id: "singleton",
            rule_name: Some(&profile.name),
        },
        profile.device_id.as_deref(),
    );
    super::pubsub::notify(
        &state.event_publisher,
        madhyamas_core::CHANNEL_INTERCEPT_EVENT,
        "throttle-toggled",
    );
    StatusCode::OK.into_response()
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

/// Save multiple traffic entries as replay requests in one batch.
#[derive(Debug, Deserialize)]
pub struct BatchSaveFromTrafficRequest {
    /// Traffic entry IDs to save.
    pub entry_ids: Vec<String>,
    /// Optional name prefix. If omitted, uses "{METHOD} {URL}".
    pub name_prefix: Option<String>,
}

pub async fn batch_save_requests_from_traffic(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchSaveFromTrafficRequest>,
) -> impl IntoResponse {
    if req.entry_ids.is_empty() {
        return super::error::ApiError::bad_request("entry_ids cannot be empty").into_response();
    }

    let mut saved_ids: Vec<String> = Vec::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();

    for entry_id in &req.entry_ids {
        match state.traffic_store.get_by_id(entry_id).await {
            Ok(Some(entry)) => {
                let name = match &req.name_prefix {
                    Some(prefix) => {
                        format!("{}: {} {}", prefix, entry.request.method, entry.request.url)
                    }
                    None => format!("{} {}", entry.request.method, entry.request.url),
                };
                let saved = SavedRequest::from_traffic(entry_id, entry.request.clone());
                let mut saved = saved;
                saved.name = Some(name);
                let id = state.replay_manager.save_request(saved);
                saved_ids.push(id);
            }
            Ok(None) => {
                errors.push(serde_json::json!({
                    "entry_id": entry_id,
                    "error": "Traffic entry not found",
                }));
            }
            Err(e) => {
                errors.push(serde_json::json!({
                    "entry_id": entry_id,
                    "error": e.to_string(),
                }));
            }
        }
    }

    let status = if saved_ids.is_empty() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::CREATED
    };

    (
        status,
        Json(serde_json::json!({
            "saved": saved_ids.len(),
            "ids": saved_ids,
            "errors": errors,
            "total": req.entry_ids.len(),
        })),
    )
        .into_response()
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

/// Replay a saved request multiple times with concurrency and delay (batch
/// replay / "Repeat Advanced").
#[derive(Debug, Deserialize)]
pub struct ReplayBatchRequest {
    pub modifications: Option<RequestModifications>,
    pub config: ReplayBatchConfig,
}

pub async fn replay_request_batch(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ReplayBatchRequest>,
) -> impl IntoResponse {
    let result = state
        .replay_manager
        .replay_batch(&id, req.modifications, req.config)
        .await;
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
pub async fn get_block_list(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    let entries = state.block_list_manager.get_entries();
    let visible: Vec<BlockListEntry> = entries
        .into_iter()
        .filter(|e| rule_visible(actor.as_deref(), e.device_id.as_deref()))
        .collect();
    Json(visible)
}

/// Get block list summary statistics
pub async fn get_block_list_stats(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
) -> impl IntoResponse {
    // Device axis (issue #109): agent keys get stats over their visible
    // entries only.
    let entries = state.block_list_manager.get_entries();
    let visible: Vec<BlockListEntry> = entries
        .into_iter()
        .filter(|e| rule_visible(actor.as_deref(), e.device_id.as_deref()))
        .collect();
    let total = visible.len();
    let enabled = visible.iter().filter(|e| e.enabled).count();
    Json(serde_json::json!({
        "total": total,
        "enabled": enabled,
        "disabled": total - enabled,
        "total_hits": visible.iter().map(|e| e.hit_count).sum::<u64>(),
    }))
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
    /// Device scope (issue #109); same semantics as
    /// [`CreateMockRequest::device_id`].
    #[serde(default, deserialize_with = "double_option")]
    pub device_id: Option<Option<String>>,
}

pub async fn create_block_list_entry(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Json(req): Json<CreateBlockListEntryRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let device_scope = match resolve_rule_scope(
        actor.as_deref(),
        req.device_id.as_ref().map(|inner| inner.as_deref()),
    ) {
        Ok(scope) => scope,
        Err((status, msg)) => {
            return (status, Json(ErrorResponse { error: msg })).into_response();
        }
    };
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
    entry.device_id = device_scope.clone();

    let id = state.block_list_manager.add_entry(entry).await;
    audit_rule_mutation(
        &state,
        actor.as_deref(),
        AuditEventType::Custom,
        "create",
        RuleRef {
            rule_type: "blocklist",
            rule_id: &id,
            rule_name: None,
        },
        device_scope.as_deref(),
    );
    super::pubsub::notify(
        &state.event_publisher,
        madhyamas_core::CHANNEL_INTERCEPT_EVENT,
        "blocklist-created",
    );
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response()
}

/// Get a specific block list entry
pub async fn get_block_list_entry(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.block_list_manager.get_entry(&id) {
        Some(entry) if rule_visible(actor.as_deref(), entry.device_id.as_deref()) => {
            Json(entry).into_response()
        }
        _ => rule_not_found("Block list entry"),
    }
}

/// Update a block list entry
pub async fn update_block_list_entry(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    mut entry: Json<BlockListEntry>,
) -> impl IntoResponse {
    // Visibility (issue #109): agent keys may only update their parent
    // device's entries; global and other-device entries 404.
    let existing_scope = match state.block_list_manager.get_entry(&id) {
        Some(existing) if rule_visible(actor.as_deref(), existing.device_id.as_deref()) => {
            existing.device_id.clone()
        }
        _ => return rule_not_found("Block list entry"),
    };
    // Full-replace update: agent keys keep the parent-device scope
    // (unchangeable). Other principals: an explicit device_id in the
    // payload sets that scope; an absent or null field PRESERVES the
    // existing scope (the full-entry body cannot distinguish the two).
    // To globalize a scoped entry the owner recreates it.
    entry.device_id = match forced_scope(actor.as_deref()) {
        Some(forced) => Some(forced),
        None => entry.device_id.take().or(existing_scope),
    };
    if entry.pattern.trim().is_empty() {
        return super::error::ApiError::bad_request("pattern cannot be empty").into_response();
    }
    let (entry_id, entry_device) = (id.clone(), entry.device_id.clone());
    if state.block_list_manager.update_entry(&id, entry.0).await {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "update",
            RuleRef {
                rule_type: "blocklist",
                rule_id: &entry_id,
                rule_name: None,
            },
            entry_device.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "blocklist-updated",
        );
        StatusCode::OK.into_response()
    } else {
        rule_not_found("Block list entry")
    }
}

/// Delete a block list entry
pub async fn delete_block_list_entry(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let visible = state
        .block_list_manager
        .get_entry(&id)
        .filter(|e| rule_visible(actor.as_deref(), e.device_id.as_deref()));
    let Some(entry) = visible else {
        return rule_not_found("Block list entry");
    };
    if state.block_list_manager.remove_entry(&id).await {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "delete",
            RuleRef {
                rule_type: "blocklist",
                rule_id: &id,
                rule_name: entry.note.as_deref(),
            },
            entry.device_id.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "blocklist-deleted",
        );
        StatusCode::NO_CONTENT.into_response()
    } else {
        rule_not_found("Block list entry")
    }
}

/// Toggle a block list entry
pub async fn toggle_block_list_entry(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<RuleActor>>,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> impl IntoResponse {
    let visible = state
        .block_list_manager
        .get_entry(&id)
        .filter(|e| rule_visible(actor.as_deref(), e.device_id.as_deref()));
    let Some(entry) = visible else {
        return rule_not_found("Block list entry");
    };
    if state
        .block_list_manager
        .toggle_entry(&id, req.enabled)
        .await
    {
        audit_rule_mutation(
            &state,
            actor.as_deref(),
            AuditEventType::Custom,
            "toggle",
            RuleRef {
                rule_type: "blocklist",
                rule_id: &id,
                rule_name: entry.note.as_deref(),
            },
            entry.device_id.as_deref(),
        );
        super::pubsub::notify(
            &state.event_publisher,
            madhyamas_core::CHANNEL_INTERCEPT_EVENT,
            "blocklist-toggled",
        );
        StatusCode::OK.into_response()
    } else {
        rule_not_found("Block list entry")
    }
}

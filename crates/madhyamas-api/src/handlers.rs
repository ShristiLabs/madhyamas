//! API handlers

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use madhyamas_core::{ProxyConfig, TrafficFilter, WsFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::ws::handle_ws;
use super::AppState;

/// Query parameters for traffic listing
#[derive(Debug, Deserialize)]
pub struct TrafficQuery {
    pub url: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search: Option<String>,
    pub file_type: Option<String>,
    pub header: Option<String>,
    pub cookie: Option<String>,
}

/// Get all traffic entries
pub async fn get_traffic(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TrafficQuery>,
) -> impl IntoResponse {
    // Parse status code filter (e.g., "2xx", "4xx", "5xx")
    let (status_min, status_max) = query
        .status_code
        .and_then(|s| match s.as_str() {
            "2xx" => Some((Some(200), Some(299))),
            "3xx" => Some((Some(300), Some(399))),
            "4xx" => Some((Some(400), Some(499))),
            "5xx" => Some((Some(500), Some(599))),
            _ => None,
        })
        .unwrap_or((None, None));

    let filter = TrafficFilter {
        url_pattern: query.url,
        method: query.method.and_then(|m| m.parse().ok()),
        status_min,
        status_max,
        limit: query.limit,
        offset: query.offset,
        search: query.search,
        file_type: query.file_type,
        header: query.header,
        cookie: query.cookie,
    };

    match state.traffic_store.get_traffic(&filter) {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get a single traffic entry
pub async fn get_traffic_entry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.traffic_store.get_by_id(&id) {
        Ok(Some(entry)) => Json(entry).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Entry not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Clear all traffic for current session
pub async fn clear_traffic(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.traffic_store.clear_traffic() {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get traffic count
pub async fn get_traffic_count(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.traffic_store.count() {
        Ok(count) => Json(serde_json::json!({ "count": count })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Session response
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Get all sessions
pub async fn get_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // For now, just return the current session
    let session_id = state.traffic_store.current_session_id();
    Json(vec![serde_json::json!({
        "id": session_id,
        "name": "Default Session",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339()
    })])
}

/// Create a new session
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateSessionRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    match state.traffic_store.create_session(req.name.as_deref()) {
        Ok(session) => Json(session).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Get a specific session
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.session_manager.get_session(&id) {
        Ok(Some(session)) => Json(session).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Session not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Delete a session
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.session_manager.delete_session(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Export session as HAR
pub async fn export_har(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let session_id = state.traffic_store.current_session_id();

    match state.traffic_store.export_har(&session_id) {
        Ok(har) => Json(har).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Export request as cURL
pub async fn export_curl(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.traffic_store.get_by_id(&id) {
        Ok(Some(entry)) => {
            let curl = generate_curl(&entry.request);
            Json(serde_json::json!({ "curl": curl })).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Entry not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Generate cURL command from request
fn generate_curl(req: &madhyamas_core::RequestData) -> String {
    let mut parts = vec![format!("curl -X {} '{}{}'", req.method, req.host, req.path)];

    for (key, value) in &req.headers {
        if !matches!(
            key.to_lowercase().as_str(),
            "host" | "content-length" | "connection"
        ) {
            parts.push(format!(
                "-H '{}{}: {}'",
                if parts.len() > 1 { "  " } else { "" },
                key,
                value
            ));
        }
    }

    if let Some(ref body) = req.body {
        if let Ok(body_str) = std::str::from_utf8(body) {
            parts.push(format!(
                "  -d '{}{}'",
                if body_str.contains('\n') { "\n" } else { "" },
                body_str.replace('\'', "'\\''")
            ));
        }
    }

    parts.join(" \\\n")
}

/// Get current configuration
pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Use actual runtime config if available, otherwise fall back to default
    let config = state
        .proxy_config
        .as_ref()
        .map(|c| c.read().clone())
        .unwrap_or_else(ProxyConfig::default);

    // Determine which IP to display:
    // 1. If public_ip is configured, use it
    // 2. Otherwise, try to detect private IP from network interfaces
    // 3. Fall back to host value
    let display_host = if let Some(ref public_ip) = config.public_ip {
        public_ip.clone()
    } else if let Some(detected_ip) = ProxyConfig::detect_private_ip() {
        detected_ip
    } else {
        config.host.clone()
    };

    Json(serde_json::json!({
        "proxy_port": config.proxy_port,
        "api_port": config.api_port,
        "host": display_host,
        "public_ip": config.public_ip,
        "intercept_https": config.intercept_https,
        "max_requests": config.max_requests,
        "max_body_size": config.max_body_size
    }))
}

/// Get current capture status
pub async fn get_capture_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let enabled = state.traffic_store.is_capture_enabled();
    Json(serde_json::json!({
        "capture_enabled": enabled,
        "mode": if enabled { "recording" } else { "passthrough" }
    }))
}

/// Toggle traffic capture on/off (passthrough mode)
pub async fn toggle_capture(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let new_state = !state.traffic_store.is_capture_enabled();
    state.traffic_store.set_capture_enabled(new_state);
    Json(serde_json::json!({
        "capture_enabled": new_state,
        "mode": if new_state { "recording" } else { "passthrough" }
    }))
}

/// Request body for updating runtime configuration
#[derive(Debug, Deserialize, validator::Validate)]
pub struct PatchConfigRequest {
    pub intercept_https: Option<bool>,
    #[validate(range(min = 0, max = 1_000_000))]
    pub max_requests: Option<usize>,
    pub verbose: Option<bool>,
    pub public_ip: Option<serde_json::Value>,
    #[validate(range(min = 0, max = 1_073_741_824))]
    pub max_body_size: Option<usize>,
}

/// Update runtime configuration fields
pub async fn patch_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PatchConfigRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    let Some(proxy_config) = state.proxy_config.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Runtime config not available" })),
        )
            .into_response();
    };

    // Acquire write lock and mutate the live config in place.
    let mut config = proxy_config.write();

    if let Some(v) = req.intercept_https {
        config.intercept_https = v;
    }
    if let Some(v) = req.max_requests {
        config.max_requests = v;
    }
    if let Some(v) = req.verbose {
        config.verbose = v;
    }
    if let Some(v) = req.public_ip {
        config.public_ip = v.as_str().map(|s| s.to_string());
    }
    if let Some(v) = req.max_body_size {
        config.max_body_size = v;
        // Apply to the traffic store immediately
        state.traffic_store.set_max_body_size(v);
    }

    // Snapshot the updated config for the response (still holding the lock).
    let resp = serde_json::json!({
        "proxy_port": config.proxy_port,
        "api_port": config.api_port,
        "host": config.host,
        "public_ip": config.public_ip,
        "intercept_https": config.intercept_https,
        "max_requests": config.max_requests,
        "max_body_size": config.max_body_size,
        "verbose": config.verbose
    });

    (StatusCode::OK, Json(resp)).into_response()
}

/// WebSocket handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state.traffic_store.clone()))
}

/// Get CA certificate for HTTPS interception
pub async fn get_ca_certificate(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match &state.cert_manager {
        Some(cert_manager) => {
            let cert_pem = cert_manager.ca_certificate_pem().to_vec();
            (
                [
                    (
                        axum::http::header::CONTENT_TYPE,
                        "application/x-x509-ca-cert",
                    ),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"madhyamas-ca.crt\"",
                    ),
                    (
                        axum::http::header::CACHE_CONTROL,
                        "no-cache, no-store, must-revalidate",
                    ),
                ],
                cert_pem,
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Certificate manager not available. HTTPS interception may be disabled."
                    .to_string(),
            }),
        )
            .into_response(),
    }
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ==================== Session Management ====================

/// Export a session
pub async fn export_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.session_manager.export_session(&id) {
        Ok(export) => Json(export).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Switch to a different session
pub async fn switch_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.traffic_store.switch_session(&id) {
        Ok(()) => Json(serde_json::json!({ "success": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Import a session
pub async fn import_session(
    State(state): State<Arc<AppState>>,
    Json(export): Json<madhyamas_core::SessionExport>,
) -> impl IntoResponse {
    match state.session_manager.import_session(export) {
        Ok(session) => Json(session).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

// ==================== WebSocket Traffic ====================

/// Get all WebSocket connections
pub async fn get_ws_connections(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.ws_manager.get_connections())
}

/// Get a specific WebSocket connection
pub async fn get_ws_connection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.ws_manager.get_connection(&id) {
        Some(conn) => Json::<madhyamas_core::WsConnection>(conn).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Connection not found".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Query parameters for WebSocket messages
#[derive(Debug, Deserialize)]
pub struct WsMessagesQuery {
    pub connection_id: Option<String>,
    pub direction: Option<String>,
    pub message_type: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Get WebSocket messages
pub async fn get_ws_messages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsMessagesQuery>,
) -> impl IntoResponse {
    let filter = WsFilter {
        connection_id: query.connection_id,
        direction: query
            .direction
            .and_then(|d| match d.to_lowercase().as_str() {
                "send" => Some(madhyamas_core::WsDirection::Send),
                "receive" => Some(madhyamas_core::WsDirection::Receive),
                _ => None,
            }),
        message_type: query
            .message_type
            .and_then(|t| match t.to_lowercase().as_str() {
                "text" => Some(madhyamas_core::WsMessageType::Text),
                "binary" => Some(madhyamas_core::WsMessageType::Binary),
                "ping" => Some(madhyamas_core::WsMessageType::Ping),
                "pong" => Some(madhyamas_core::WsMessageType::Pong),
                "close" => Some(madhyamas_core::WsMessageType::Close),
                _ => None,
            }),
        search: query.search,
        limit: query.limit,
        offset: query.offset,
    };

    Json(state.ws_manager.get_messages(&filter))
}

/// Clear all WebSocket traffic
pub async fn clear_ws_traffic(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.ws_manager.clear_messages();
    state.ws_manager.clear_closed_connections();
    StatusCode::NO_CONTENT
}

// ==================== Persistence ====================

/// Export all rules
pub async fn export_all_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match &state.intercept_store {
        Some(store) => match store.export_all() {
            Ok(json) => Json(serde_json::json!({ "data": json })).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Persistence not enabled".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Import all rules
#[derive(Debug, Deserialize, validator::Validate)]
pub struct ImportRulesRequest {
    #[validate(length(min = 1))]
    pub data: String,
}

pub async fn import_all_rules(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportRulesRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    match &state.intercept_store {
        Some(store) => match store.import_all(&req.data) {
            Ok(()) => Json(serde_json::json!({ "success": true })).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Persistence not enabled".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Save all rules from managers to store
pub async fn save_all_rules(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // CSRF protection: require a custom header that browsers won't send
    // in a cross-origin simple request. This prevents arbitrary websites
    // from triggering a save-all-rules via a form POST.
    if headers.get("x-madhyamas-confirm").is_none() {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Missing X-Madhyamas-Confirm header".to_string(),
            }),
        )
            .into_response();
    }

    match &state.intercept_store {
        Some(store) => {
            // Save mock rules
            for rule in state.mock_manager.get_rules() {
                if let Err(e) = store.save_mock_rule(&rule) {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            // Save rewrite rules
            for rule in state.rewrite_manager.get_rules() {
                if let Err(e) = store.save_rewrite_rule(&rule) {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            // Save breakpoint rules
            for rule in state.breakpoint_manager.get_rules() {
                if let Err(e) = store.save_breakpoint_rule(&rule) {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            // Save throttle profile
            {
                let profile = state.throttle_manager.get_profile();
                if let Err(e) =
                    store.save_throttle_profile(&profile, state.throttle_manager.is_enabled())
                {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            Json(serde_json::json!({ "success": true })).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Persistence not enabled".to_string(),
            }),
        )
            .into_response(),
    }
}

/// Load all rules from store to managers
pub async fn load_all_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match &state.intercept_store {
        Some(store) => {
            // Load mock rules
            match store.load_mock_rules() {
                Ok(rules) => {
                    state.mock_manager.clear();
                    state.mock_manager.import_rules(rules);
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            // Load rewrite rules
            match store.load_rewrite_rules() {
                Ok(rules) => {
                    state.rewrite_manager.clear();
                    for rule in rules {
                        state.rewrite_manager.add_rule(rule);
                    }
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            // Load breakpoint rules
            match store.load_breakpoint_rules() {
                Ok(rules) => {
                    state.breakpoint_manager.clear();
                    for rule in rules {
                        state.breakpoint_manager.add_rule(rule);
                    }
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            // Load throttle profile
            match store.load_throttle_profile() {
                Ok(Some((profile, enabled))) => {
                    state.throttle_manager.set_profile(profile);
                    state.throttle_manager.set_enabled(enabled);
                }
                Ok(None) => {}
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }

            Json(serde_json::json!({ "success": true })).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Persistence not enabled".to_string(),
            }),
        )
            .into_response(),
    }
}

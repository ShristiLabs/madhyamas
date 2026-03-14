//! API handlers

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use proxyforge_core::{ProxyConfig, TrafficFilter, WsFilter};
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
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub name: Option<String>,
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
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
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "id": id,
        "name": "Session",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339()
    }))
}

/// Delete a session
pub async fn delete_session(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
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
fn generate_curl(req: &proxyforge_core::RequestData) -> String {
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
pub async fn get_config() -> impl IntoResponse {
    let config = ProxyConfig::default();
    Json(serde_json::json!({
        "proxy_port": config.proxy_port,
        "api_port": config.api_port,
        "host": config.host,
        "intercept_https": config.intercept_https,
        "max_requests": config.max_requests
    }))
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
                [(
                    axum::http::header::CONTENT_DISPOSITION,
                    "attachment; filename=\"proxyforge-ca.pem\"",
                )],
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
    Json(export): Json<proxyforge_core::SessionExport>,
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
        Some(conn) => Json::<proxyforge_core::WsConnection>(conn).into_response(),
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
                "send" => Some(proxyforge_core::WsDirection::Send),
                "receive" => Some(proxyforge_core::WsDirection::Receive),
                _ => None,
            }),
        message_type: query
            .message_type
            .and_then(|t| match t.to_lowercase().as_str() {
                "text" => Some(proxyforge_core::WsMessageType::Text),
                "binary" => Some(proxyforge_core::WsMessageType::Binary),
                "ping" => Some(proxyforge_core::WsMessageType::Ping),
                "pong" => Some(proxyforge_core::WsMessageType::Pong),
                "close" => Some(proxyforge_core::WsMessageType::Close),
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
#[derive(Debug, Deserialize)]
pub struct ImportRulesRequest {
    pub data: String,
}

pub async fn import_all_rules(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportRulesRequest>,
) -> impl IntoResponse {
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
pub async fn save_all_rules(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

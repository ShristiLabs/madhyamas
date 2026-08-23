//! API handlers

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use madhyamas_core::{
    AccessControlList, PaginatedTraffic, ProxyConfig, TrafficCursor, TrafficFilter, WsFilter,
};
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
    /// Filter by passthrough: "true" = only passthrough, "false" = only intercepted
    pub is_passthrough: Option<String>,
    /// Filter by host pattern (substring match)
    pub host: Option<String>,
    /// Phase 10b.3: cursor for cursor-based pagination. When provided,
    /// OFFSET is ignored and keyset pagination is used. The response
    /// format changes to `PaginatedTraffic` (with `next_cursor`).
    pub cursor: Option<String>,
    /// Phase 10b.4: when "false", omit body columns from the response
    /// to reduce payload size. Defaults to "true" (include bodies).
    pub include_bodies: Option<String>,
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

    let include_bodies = query
        .include_bodies
        .as_deref()
        .map(|s| !s.eq_ignore_ascii_case("false"))
        .unwrap_or(true);

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
        is_passthrough: query.is_passthrough.and_then(|s| match s.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }),
        host: query.host,
        cursor: query.cursor,
        include_bodies: Some(include_bodies),
    };

    match state.traffic_store.get_traffic(&filter).await {
        Ok(entries) => {
            // Phase 10b.3: when cursor pagination is used, return
            // PaginatedTraffic with next_cursor. Otherwise, return a
            // plain array for backward compatibility.
            if filter.cursor.is_some() {
                let next_cursor = entries.last().map(TrafficCursor::from_entry);
                let paginated = PaginatedTraffic {
                    entries,
                    next_cursor,
                };
                Json(paginated).into_response()
            } else {
                Json(entries).into_response()
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Query parameters for fetching a single traffic entry
#[derive(Debug, Deserialize)]
pub struct TrafficEntryQuery {
    /// When `true`, decompress the response body on the fly using the
    /// response's `Content-Encoding` header and return the decompressed
    /// bytes with the encoding header removed. Useful for encodings the
    /// browser cannot decompress client-side (e.g. zstd).
    pub decompressed: Option<String>,
}

/// Get a single traffic entry
pub async fn get_traffic_entry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<TrafficEntryQuery>,
) -> impl IntoResponse {
    let decompress = query
        .decompressed
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    match state.traffic_store.get_by_id(&id).await {
        Ok(Some(mut entry)) => {
            if decompress {
                if let Some(response) = entry.response.as_mut() {
                    if let Some(body) = response.body.take() {
                        let content_encoding = response
                            .headers
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("content-encoding"))
                            .map(|(_, v)| v.clone());
                        match content_encoding {
                            Some(encoding) => {
                                // decompress_body returns the original body on
                                // failure, so the response body is never lost.
                                response.body =
                                    madhyamas_core::proxy::pipeline::Pipeline::decompress_body(
                                        Some(encoding.as_str()),
                                        body,
                                        &mut response.headers,
                                    );
                            }
                            None => {
                                response.body = Some(body);
                            }
                        }
                    }
                }
            }
            Json(entry).into_response()
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

/// Clear all traffic for current session
pub async fn clear_traffic(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.traffic_store.clear_traffic().await {
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
    match state.traffic_store.count().await {
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

/// Health check handler for `/api/health`. Verifies database connectivity
/// via `TrafficStoreBackend::ping` so the instance is not reported healthy
/// before schema initialization completes. Returns `200 "OK"` when the
/// database is ready, `503 "Database not ready"` otherwise. Unauthenticated
/// — intended for Docker/nginx health probes.
pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.traffic_store.ping().await {
        Ok(()) => (StatusCode::OK, "OK").into_response(),
        Err(e) => {
            tracing::error!("Health check failed: database not ready: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "Database not ready").into_response()
        }
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
    match state
        .traffic_store
        .create_session(req.name.as_deref())
        .await
    {
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
    match state.session_manager.get_session(&id).await {
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
    match state.session_manager.delete_session(&id).await {
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

    match state.traffic_store.export_har(&session_id).await {
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

/// Request body for importing traffic from a HAR file.
#[derive(Debug, Deserialize)]
pub struct HarImportRequest {
    /// HAR JSON document (the full `{ "log": { ... } }` object).
    pub har: serde_json::Value,
    /// Optional name for the newly created session. Defaults to
    /// `"Imported HAR"` when omitted.
    #[serde(default)]
    pub session_name: Option<String>,
    /// When true, switch the active session to the newly created one after
    /// a successful import. Defaults to false.
    #[serde(default)]
    pub switch_session: bool,
}

/// Import traffic from a HAR file into a new session.
pub async fn import_traffic_har(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HarImportRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate_har_import(&req.har) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response();
    }

    match state
        .traffic_store
        .import_har(&req.har, req.session_name.as_deref())
        .await
    {
        Ok(result) => {
            if req.switch_session {
                let _ = state.traffic_store.switch_session(&result.session_id).await;
            }
            Json(result).into_response()
        }
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
    match state.traffic_store.get_by_id(&id).await {
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
        "max_body_size": config.max_body_size,
        "max_total_size_mb": config.max_total_size_mb,
        "capture_request_bodies": config.capture_request_bodies,
        "capture_response_bodies": config.capture_response_bodies,
        "ignored_domains": config.ignored_domains,
        "passthrough_domains": config.passthrough_domains,
        "enable_h2_downstream": config.enable_h2_downstream,
        "enable_socks": config.enable_socks,
        "socks_port": config.socks_port(),
        "socks_auth_enabled": config.socks_auth_enabled(),
        "socks_auth_username": config.socks_auth_username,
        "upstream_proxy": {
            "enabled": config.upstream_proxy.enabled,
            "protocol": config.upstream_proxy.protocol,
            "host": config.upstream_proxy.host,
            "port": config.upstream_proxy.port,
            "auth_enabled": config.upstream_proxy.auth_enabled(),
            "auth_username": config.upstream_proxy.auth_username,
            "no_proxy_hosts": config.upstream_proxy.no_proxy_hosts,
        },
        "access_control_enabled": config.access_control_enabled(),
        "allowed_ips": config.allowed_ips,
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

/// Get recording quota statistics (entry count, total size, limits, usage).
pub async fn get_capture_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.traffic_store.get_capture_stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
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

    /// Maximum total recording size in megabytes. Set to `null` to disable
    /// the total-size limit. Applied to the traffic store immediately.
    pub max_total_size_mb: Option<serde_json::Value>,

    /// Whether to capture request bodies. Applied to the traffic store
    /// immediately.
    pub capture_request_bodies: Option<bool>,

    /// Whether to capture response bodies. Applied to the traffic store
    /// immediately.
    pub capture_response_bodies: Option<bool>,

    /// Domains whose traffic should not be recorded (capture ignore list).
    /// Supports suffix and wildcard matching (e.g. `*.example.com`).
    /// Applied to the traffic store immediately.
    pub ignored_domains: Option<Vec<String>>,

    /// Domains to exclude from TLS interception (SSL passthrough)
    pub passthrough_domains: Option<Vec<String>>,
    /// Enable HTTP/2 downstream (client-facing) support via ALPN h2 negotiation.
    /// When enabled, the proxy advertises both h2 and http/1.1 and can parse
    /// HTTP/2 frames, enabling gRPC interception. Requires restart to take
    /// effect (ALPN advertisement is set at TLS config creation time).
    pub enable_h2_downstream: Option<bool>,

    /// Enable the SOCKS5 proxy listener. Requires restart to take effect
    /// (the SOCKS TCP listener is bound at startup).
    pub enable_socks: Option<bool>,

    /// Port for the SOCKS5 listener. Requires restart to take effect.
    #[validate(range(min = 1, max = 65535))]
    pub socks_port: Option<u16>,

    /// Username for SOCKS5 username/password auth. Set to null/None to
    /// disable auth. When set, --socks-password must also be provided.
    /// Requires restart to take effect.
    pub socks_auth_username: Option<serde_json::Value>,

    /// Password for SOCKS5 username/password auth. Ignored unless
    /// socks_auth_username is set. Requires restart to take effect.
    pub socks_auth_password: Option<serde_json::Value>,

    /// Upstream (external) proxy chaining configuration.
    ///
    /// When provided, updates the upstream proxy settings. The HTTP
    /// forwarding path (reqwest) picks up the change on the next request
    /// (the client is rebuilt when the engine detects a config change).
    /// Raw TCP tunneling (CONNECT/passthrough) reads the live config on
    /// each connection, so changes take effect immediately for new
    /// connections.
    ///
    /// Note: changing the upstream proxy protocol/host/port requires a
    /// restart for the reqwest client to pick up the new proxy (the
    /// client is built once at engine startup). The bypass list and
    /// auth credentials are read live and don't require a restart.
    pub upstream_proxy: Option<UpstreamProxyPatch>,

    /// IP allowlist for the proxy and API listeners.
    ///
    /// When set to a non-empty list, only connections from the listed IP
    /// addresses or CIDR ranges are accepted (loopback is always allowed).
    /// Set to an empty array to disable access control (allow all).
    /// Each entry is validated as an IP address or CIDR range; invalid
    /// entries cause a `400 Bad Request`. Changes take effect immediately
    /// for new connections (the proxy accept loop reads the live config).
    pub allowed_ips: Option<Vec<String>>,
}

/// Patchable subset of [`UpstreamProxyConfig`].
///
/// The `auth_password` field is write-only (never returned in GET
/// responses) to avoid leaking credentials. Set it to `null` to clear
/// existing credentials.
#[derive(Debug, Deserialize, validator::Validate)]
pub struct UpstreamProxyPatch {
    pub enabled: Option<bool>,
    pub protocol: Option<String>,
    pub host: Option<String>,
    #[validate(range(min = 0, max = 65535))]
    pub port: Option<u16>,
    /// Username for upstream proxy auth. Set to null to clear.
    pub auth_username: Option<serde_json::Value>,
    /// Password for upstream proxy auth. Set to null to clear.
    /// Never returned in GET responses.
    pub auth_password: Option<serde_json::Value>,
    pub no_proxy_hosts: Option<Vec<String>>,
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
        // Apply to the traffic store immediately (entry-count limit)
        state.traffic_store.set_max_entries(v);
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
    if let Some(v) = req.max_total_size_mb {
        match v {
            serde_json::Value::Null => {
                config.max_total_size_mb = None;
                state.traffic_store.set_max_total_size_bytes(0);
            }
            serde_json::Value::Number(n) => {
                if let Some(mb) = n.as_u64() {
                    config.max_total_size_mb = Some(mb as usize);
                    state
                        .traffic_store
                        .set_max_total_size_bytes(mb as usize * 1024 * 1024);
                } else {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "max_total_size_mb must be a non-negative integer or null",
                        })),
                    )
                        .into_response();
                }
            }
            other => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "max_total_size_mb must be a non-negative integer or null",
                        "value": other,
                    })),
                )
                    .into_response();
            }
        }
    }
    if let Some(v) = req.capture_request_bodies {
        config.capture_request_bodies = v;
        state.traffic_store.set_capture_request_bodies(v);
    }
    if let Some(v) = req.capture_response_bodies {
        config.capture_response_bodies = v;
        state.traffic_store.set_capture_response_bodies(v);
    }
    if let Some(domains) = req.ignored_domains {
        // Normalize: trim, lowercase, deduplicate, drop empties
        let mut cleaned: Vec<String> = domains
            .iter()
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty())
            .collect();
        cleaned.sort();
        cleaned.dedup();
        config.ignored_domains = cleaned.clone();
        state.traffic_store.set_ignored_domains(cleaned);
    }
    if let Some(domains) = req.passthrough_domains {
        // Normalize: trim, lowercase, deduplicate, drop empties
        let mut cleaned: Vec<String> = domains
            .iter()
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty())
            .collect();
        cleaned.sort();
        cleaned.dedup();
        config.passthrough_domains = cleaned;
    }
    if let Some(v) = req.enable_h2_downstream {
        config.enable_h2_downstream = v;
        // Note: the ALPN advertisement is baked into the TLS ServerConfig at
        // creation time. The change takes effect on new TLS handshakes after
        // the config is re-read. Existing connections are unaffected.
    }
    if let Some(v) = req.enable_socks {
        config.enable_socks = v;
        // The SOCKS TCP listener is bound at startup; this change takes
        // effect after a restart.
    }
    if let Some(v) = req.socks_port {
        config.socks_port = Some(v);
        // Requires restart to rebind the SOCKS listener.
    }
    // socks_auth_username / socks_auth_password accept either a string or
    // null (to clear auth). A non-string, non-null value is rejected.
    if let Some(v) = req.socks_auth_username {
        match v {
            serde_json::Value::Null => {
                config.socks_auth_username = None;
                config.socks_auth_password = None;
            }
            serde_json::Value::String(s) => {
                config.socks_auth_username = Some(s);
            }
            other => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "socks_auth_username must be a string or null",
                        "value": other
                    })),
                )
                    .into_response();
            }
        }
    }
    if let Some(v) = req.socks_auth_password {
        match v {
            serde_json::Value::Null => {
                // Only clear the password; keep username unless explicitly cleared.
                config.socks_auth_password = None;
            }
            serde_json::Value::String(s) => {
                config.socks_auth_password = Some(s);
            }
            other => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "socks_auth_password must be a string or null",
                        "value": other
                    })),
                )
                    .into_response();
            }
        }
    }

    // Upstream proxy patching. Each field is optional; only provided
    // fields are mutated. The auth_password is write-only (never echoed
    // back in the response). Protocol changes are validated against the
    // allowed set (http/https/socks5).
    if let Some(patch) = req.upstream_proxy {
        if let Some(v) = patch.enabled {
            config.upstream_proxy.enabled = v;
        }
        if let Some(v) = patch.protocol {
            let normalized = v.trim().to_lowercase();
            if !matches!(normalized.as_str(), "http" | "https" | "socks5") {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "upstream_proxy.protocol must be one of: http, https, socks5",
                        "value": v
                    })),
                )
                    .into_response();
            }
            config.upstream_proxy.protocol = normalized;
        }
        if let Some(v) = patch.host {
            config.upstream_proxy.host = v.trim().to_string();
        }
        if let Some(v) = patch.port {
            config.upstream_proxy.port = v;
        }
        if let Some(v) = patch.auth_username {
            match v {
                serde_json::Value::Null => {
                    config.upstream_proxy.auth_username = None;
                    config.upstream_proxy.auth_password = None;
                }
                serde_json::Value::String(s) => {
                    config.upstream_proxy.auth_username = Some(s);
                }
                other => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "upstream_proxy.auth_username must be a string or null",
                            "value": other
                        })),
                    )
                        .into_response();
                }
            }
        }
        if let Some(v) = patch.auth_password {
            match v {
                serde_json::Value::Null => {
                    config.upstream_proxy.auth_password = None;
                }
                serde_json::Value::String(s) => {
                    config.upstream_proxy.auth_password = Some(s);
                }
                other => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "upstream_proxy.auth_password must be a string or null",
                            "value": other
                        })),
                    )
                        .into_response();
                }
            }
        }
        if let Some(v) = patch.no_proxy_hosts {
            // Normalize: trim, lowercase, deduplicate, drop empties
            let mut cleaned: Vec<String> = v
                .iter()
                .map(|d| d.trim().to_lowercase())
                .filter(|d| !d.is_empty())
                .collect();
            cleaned.sort();
            cleaned.dedup();
            config.upstream_proxy.no_proxy_hosts = cleaned;
        }
    }

    // IP access control (allowlist). Validate every entry as an IP address
    // or CIDR range before applying so a bad entry doesn't silently break
    // the proxy. Normalization: trim, drop empties, deduplicate (case is
    // preserved for IPv6 but entries are compared as strings). An empty
    // array disables access control (allow all).
    if let Some(entries) = req.allowed_ips {
        let mut cleaned: Vec<String> = entries
            .iter()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty())
            .collect();
        cleaned.sort();
        cleaned.dedup();
        // Validate by constructing an AccessControlList. This catches
        // malformed IPs and out-of-range CIDR prefixes before they reach
        // the accept loop.
        if let Err(e) = AccessControlList::new(&cleaned) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid allowed_ips entry",
                    "message": e.to_string(),
                })),
            )
                .into_response();
        }
        config.allowed_ips = cleaned;
    }

    // Snapshot the updated config for the response (still holding the lock).
    // Note: auth_password is intentionally omitted to avoid leaking secrets.
    let resp = serde_json::json!({
        "proxy_port": config.proxy_port,
        "api_port": config.api_port,
        "host": config.host,
        "public_ip": config.public_ip,
        "intercept_https": config.intercept_https,
        "max_requests": config.max_requests,
        "max_body_size": config.max_body_size,
        "max_total_size_mb": config.max_total_size_mb,
        "capture_request_bodies": config.capture_request_bodies,
        "capture_response_bodies": config.capture_response_bodies,
        "ignored_domains": config.ignored_domains,
        "verbose": config.verbose,
        "passthrough_domains": config.passthrough_domains,
        "enable_h2_downstream": config.enable_h2_downstream,
        "enable_socks": config.enable_socks,
        "socks_port": config.socks_port(),
        "socks_auth_enabled": config.socks_auth_enabled(),
        "socks_auth_username": config.socks_auth_username,
        "upstream_proxy": {
            "enabled": config.upstream_proxy.enabled,
            "protocol": config.upstream_proxy.protocol,
            "host": config.upstream_proxy.host,
            "port": config.upstream_proxy.port,
            "auth_enabled": config.upstream_proxy.auth_enabled(),
            "auth_username": config.upstream_proxy.auth_username,
            "no_proxy_hosts": config.upstream_proxy.no_proxy_hosts,
        },
        "access_control_enabled": config.access_control_enabled(),
        "allowed_ips": config.allowed_ips,
    });

    // Persist the updated config to disk so it survives restarts.
    // We clone here because `save()` takes `&self` and we're holding a
    // write lock — cloning avoids extending the lock scope across I/O.
    let config_snapshot = config.clone();
    drop(config); // release the write lock before disk I/O
    if let Err(e) = config_snapshot.save() {
        tracing::warn!("Failed to persist config to disk: {}", e);
    }

    // Notify other instances (multi-instance mode) that config changed so
    // they can reload from the shared store. No-op in single-instance mode.
    super::pubsub::notify(
        &state.event_publisher,
        madhyamas_core::CHANNEL_CONFIG_EVENT,
        "config-changed",
    );

    (StatusCode::OK, Json(resp)).into_response()
}

// ==================== Auto Save ====================

/// Request body for updating Auto Save configuration via `PATCH /api/autosave`.
#[derive(Debug, Deserialize)]
pub struct PatchAutoSaveRequest {
    pub enabled: Option<bool>,
    pub interval_seconds: Option<u64>,
    pub export_format: Option<String>,
    pub output_dir: Option<String>,
    pub max_backups: Option<usize>,
    /// Set to `null` to disable request-based rotation.
    pub rotate_after_requests: Option<serde_json::Value>,
    /// Set to `null` to disable time-based rotation.
    pub rotate_after_minutes: Option<serde_json::Value>,
}

/// Get the current Auto Save configuration.
pub async fn get_autosave_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Prefer the live AutoSaveManager config (reflects runtime changes),
    // then fall back to the proxy config, then to defaults.
    let cfg = if let Some(mgr) = state.autosave_manager.as_ref() {
        mgr.config().read().clone()
    } else if let Some(pc) = state.proxy_config.as_ref() {
        pc.read().auto_save.clone()
    } else {
        madhyamas_core::AutoSaveConfig::default()
    };

    Json(serde_json::json!({
        "enabled": cfg.enabled,
        "interval_seconds": cfg.interval_seconds,
        "export_format": cfg.export_format,
        "output_dir": cfg.output_dir,
        "max_backups": cfg.max_backups,
        "rotate_after_requests": cfg.rotate_after_requests,
        "rotate_after_minutes": cfg.rotate_after_minutes,
    }))
}

/// Update the Auto Save configuration at runtime.
///
/// Changes are applied to the live [`AutoSaveManager`] config (when
/// attached) and persisted to the proxy config so they survive restarts.
/// Enabling/disabling or changing the interval requires a restart for the
/// background task to pick up the new schedule (the task reads the config
/// at start time).
pub async fn update_autosave_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PatchAutoSaveRequest>,
) -> impl IntoResponse {
    // Resolve the target config: live manager config or proxy config.
    let (enabled, interval, format, output_dir, max_backups, rotate_req, rotate_min) = {
        let current = if let Some(mgr) = state.autosave_manager.as_ref() {
            mgr.config().read().clone()
        } else if let Some(pc) = state.proxy_config.as_ref() {
            pc.read().auto_save.clone()
        } else {
            madhyamas_core::AutoSaveConfig::default()
        };
        (
            req.enabled.unwrap_or(current.enabled),
            req.interval_seconds.unwrap_or(current.interval_seconds),
            req.export_format
                .unwrap_or_else(|| current.export_format.clone()),
            req.output_dir.unwrap_or_else(|| current.output_dir.clone()),
            req.max_backups.unwrap_or(current.max_backups),
            req.rotate_after_requests,
            req.rotate_after_minutes,
        )
    };

    // Validate export format.
    let format_normalized = format.trim().to_lowercase();
    if !matches!(format_normalized.as_str(), "har" | "session") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "export_format must be one of: har, session",
                "value": format
            })),
        )
            .into_response();
    }

    // Validate interval.
    if interval == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "interval_seconds must be greater than 0" })),
        )
            .into_response();
    }

    // Parse optional rotation fields (accept integer or null).
    let rotate_after_requests = match rotate_req {
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(n)) => {
            if let Some(v) = n.as_u64() {
                Some(v as usize)
            } else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "rotate_after_requests must be a non-negative integer or null"
                    })),
                )
                    .into_response();
            }
        }
        Some(other) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "rotate_after_requests must be a non-negative integer or null",
                    "value": other
                })),
            )
                .into_response();
        }
        None => None,
    };

    let rotate_after_minutes = match rotate_min {
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(n)) => {
            if let Some(v) = n.as_u64() {
                Some(v)
            } else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "rotate_after_minutes must be a non-negative integer or null"
                    })),
                )
                    .into_response();
            }
        }
        Some(other) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "rotate_after_minutes must be a non-negative integer or null",
                    "value": other
                })),
            )
                .into_response();
        }
        None => None,
    };

    let new_cfg = madhyamas_core::AutoSaveConfig {
        enabled,
        interval_seconds: interval,
        export_format: format_normalized,
        output_dir: output_dir.trim().to_string(),
        max_backups,
        rotate_after_requests,
        rotate_after_minutes,
    };

    // Apply to the live AutoSaveManager config (if attached).
    if let Some(mgr) = state.autosave_manager.as_ref() {
        *mgr.config().write() = new_cfg.clone();
    }

    // Persist to the proxy config so changes survive restarts.
    if let Some(pc) = state.proxy_config.as_ref() {
        let snapshot = {
            let mut config = pc.write();
            config.auto_save = new_cfg.clone();
            config.clone()
        };
        if let Err(e) = snapshot.save() {
            tracing::warn!("Failed to persist auto-save config to disk: {}", e);
        }
    }

    let resp = serde_json::json!({
        "enabled": new_cfg.enabled,
        "interval_seconds": new_cfg.interval_seconds,
        "export_format": new_cfg.export_format,
        "output_dir": new_cfg.output_dir,
        "max_backups": new_cfg.max_backups,
        "rotate_after_requests": new_cfg.rotate_after_requests,
        "rotate_after_minutes": new_cfg.rotate_after_minutes,
    });

    (StatusCode::OK, Json(resp)).into_response()
}

/// Trigger an immediate Auto Save snapshot (manual "save now").
pub async fn trigger_autosave_snapshot(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let Some(mgr) = state.autosave_manager.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Auto Save manager not available".to_string(),
            }),
        )
            .into_response();
    };

    let cfg = mgr.config().read().clone();
    match mgr.save_snapshot(&cfg).await {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": "Snapshot saved",
            "output_dir": cfg.output_dir,
        }))
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

/// Query parameters for the WebSocket upgrade endpoint. Browsers cannot
/// set custom headers on a WebSocket handshake, so the JWT is passed via
/// the `?token=` query parameter (or the `Sec-WebSocket-Protocol`
/// subprotocol header). See Phase 9.1.
#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// WebSocket handler (Phase 9.1: auth on upgrade).
///
/// In enterprise mode with auth enabled (`AppState::auth_provider` is
/// `Some`), the WebSocket upgrade is rejected with `401 Unauthorized`
/// unless a valid JWT is supplied. Browsers cannot set custom headers on
/// the WS handshake, so the token is accepted from:
/// 1. `?token=` query parameter, or
/// 2. `Sec-WebSocket-Protocol` subprotocol header (the first protocol
///    value is treated as the token).
///
/// In OSS mode (or when auth is disabled — `auth_provider` is `None`),
/// all connections are allowed (unchanged behavior).
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsAuthQuery>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    // If an auth provider is configured, validate the token before
    // allowing the upgrade. This runs inside the handler (not as
    // middleware) because the WebSocketUpgrade extractor must consume
    // the connection — the enterprise auth middleware cannot reject it
    // before the extractor runs.
    if let Some(ref auth_provider) = state.auth_provider {
        if auth_provider.auth_required() {
            // Extract the token: query param first, then subprotocol header.
            let token = query.token.or_else(|| {
                headers
                    .get("sec-websocket-protocol")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.split(',').next())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            });

            let token = match token {
                Some(t) => t,
                None => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({
                            "error": "unauthorized",
                            "message": "WebSocket authentication required: provide ?token= or Sec-WebSocket-Protocol header"
                        })),
                    )
                        .into_response();
                }
            };

            if let Err(err) = auth_provider.validate_token(&token).await {
                tracing::debug!("WebSocket auth rejected: {err}");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "unauthorized",
                        "message": err.to_string(),
                    })),
                )
                    .into_response();
            }
        }
    }

    ws.on_upgrade(move |socket| {
        let cross_rx = state.cross_instance_sender.as_ref().map(|s| s.subscribe());
        handle_ws(socket, state.traffic_store.clone(), cross_rx)
    })
    .into_response()
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
    match state.session_manager.export_session(&id).await {
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
    match state.traffic_store.switch_session(&id).await {
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
    match state.session_manager.import_session(export).await {
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
        Some(store) => match store.export_all().await {
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
        Some(store) => match store.import_all(&req.data).await {
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
                if let Err(e) = store.save_mock_rule(&rule).await {
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
                if let Err(e) = store.save_rewrite_rule(&rule).await {
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
                if let Err(e) = store.save_breakpoint_rule(&rule).await {
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
                if let Err(e) = store
                    .save_throttle_profile(&profile, state.throttle_manager.is_enabled())
                    .await
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

            // Save block list entries
            for entry in state.block_list_manager.get_entries() {
                if let Err(e) = store.save_block_list_entry(&entry).await {
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
            match store.load_mock_rules().await {
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
            match store.load_rewrite_rules().await {
                Ok(rules) => {
                    state.rewrite_manager.clear();
                    for rule in rules {
                        state.rewrite_manager.add_rule(rule).await;
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
            match store.load_breakpoint_rules().await {
                Ok(rules) => {
                    state.breakpoint_manager.clear();
                    for rule in rules {
                        state.breakpoint_manager.add_rule(rule).await;
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
            match store.load_throttle_profile().await {
                Ok(Some((profile, enabled))) => {
                    state.throttle_manager.set_profile(profile).await;
                    state.throttle_manager.set_enabled(enabled).await;
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

            // Load block list entries
            match store.load_block_list_entries().await {
                Ok(entries) => {
                    state.block_list_manager.clear().await;
                    for entry in entries {
                        state.block_list_manager.add_entry(entry).await;
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

// ============================================================================
// Focus hosts
// ============================================================================

/// Get all focus host patterns
pub async fn get_focus_hosts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.traffic_store.list_focus_hosts().await {
        Ok(hosts) => Json(hosts).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Request body for creating a focus host
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateFocusHostRequest {
    #[validate(length(min = 1, max = 255))]
    pub pattern: String,
}

/// Add a focus host pattern
pub async fn add_focus_host(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateFocusHostRequest>,
) -> impl IntoResponse {
    if let Err(e) = super::validation::validate(&req) {
        return e.into_response();
    }
    match state.traffic_store.add_focus_host(&req.pattern).await {
        Ok(host) => (StatusCode::CREATED, Json(host)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Remove a focus host by ID
pub async fn remove_focus_host(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.traffic_store.remove_focus_host(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Focus host not found".to_string(),
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

/// Clear all focus hosts
pub async fn clear_focus_hosts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.traffic_store.clear_focus_hosts().await {
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

// ============================================================================
// Mirror tool
// ============================================================================

/// Get the current mirror configuration and statistics.
pub async fn get_mirror_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Prefer the live MirrorWriter config (reflects runtime changes),
    // then fall back to the proxy config, then to defaults.
    let (cfg, stats) = if let Some(mgr) = state.mirror_writer.as_ref() {
        let c = mgr.config().read().clone();
        let s = mgr.stats();
        (c, s)
    } else if let Some(pc) = state.proxy_config.as_ref() {
        let c = pc.read().mirror.clone();
        let s = madhyamas_core::MirrorStats {
            enabled: c.enabled,
            output_dir: c.output_dir.clone(),
            files_written: 0,
            bytes_written: 0,
        };
        (c, s)
    } else {
        let c = madhyamas_core::MirrorConfig::default();
        let s = madhyamas_core::MirrorStats {
            enabled: c.enabled,
            output_dir: c.output_dir.clone(),
            files_written: 0,
            bytes_written: 0,
        };
        (c, s)
    };

    Json(serde_json::json!({
        "enabled": cfg.enabled,
        "output_dir": cfg.output_dir,
        "host_filter": cfg.host_filter,
        "save_request_bodies": cfg.save_request_bodies,
        "files_written": stats.files_written,
        "bytes_written": stats.bytes_written,
    }))
}

/// Request body for toggling the mirror on/off.
#[derive(Debug, Deserialize)]
pub struct ToggleMirrorRequest {
    pub enabled: bool,
}

/// Toggle mirroring on or off at runtime.
///
/// This updates the live [`MirrorWriter`] config (when attached) and
/// persists the change to the proxy config so it survives restarts.
pub async fn toggle_mirror(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ToggleMirrorRequest>,
) -> impl IntoResponse {
    // Apply to the live MirrorWriter config (if attached).
    if let Some(mgr) = state.mirror_writer.as_ref() {
        mgr.config().write().enabled = req.enabled;
    }

    // Persist to the proxy config so changes survive restarts.
    if let Some(pc) = state.proxy_config.as_ref() {
        let snapshot = {
            let mut config = pc.write();
            config.mirror.enabled = req.enabled;
            config.clone()
        };
        if let Err(e) = snapshot.save() {
            tracing::warn!("Failed to persist mirror config to disk: {}", e);
        }
    }

    Json(serde_json::json!({
        "enabled": req.enabled,
        "message": if req.enabled { "Mirroring enabled" } else { "Mirroring disabled" },
    }))
}

/// Partial update payload for PATCH /api/mirror/config.
#[derive(Debug, Deserialize)]
pub struct PatchMirrorConfigRequest {
    pub enabled: Option<bool>,
    pub output_dir: Option<String>,
    /// Set to null to clear the host filter (mirror all hosts).
    pub host_filter: Option<serde_json::Value>,
    pub save_request_bodies: Option<bool>,
}

/// Update the mirror configuration at runtime.
///
/// Changes are applied to the live [`MirrorWriter`] config (when attached)
/// and persisted to the proxy config so they survive restarts.
pub async fn update_mirror_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PatchMirrorConfigRequest>,
) -> impl IntoResponse {
    // Resolve the target config: live writer config or proxy config.
    let (mut enabled, mut output_dir, host_filter, mut save_req) = {
        let current = if let Some(mgr) = state.mirror_writer.as_ref() {
            mgr.config().read().clone()
        } else if let Some(pc) = state.proxy_config.as_ref() {
            pc.read().mirror.clone()
        } else {
            madhyamas_core::MirrorConfig::default()
        };
        (
            current.enabled,
            current.output_dir,
            current.host_filter,
            current.save_request_bodies,
        )
    };

    if let Some(v) = req.enabled {
        enabled = v;
    }
    if let Some(ref dir) = req.output_dir {
        output_dir = dir.trim().to_string();
    }
    if let Some(v) = req.save_request_bodies {
        save_req = v;
    }

    // Parse host_filter (accept array of strings or null to clear).
    let host_filter = match req.host_filter {
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Array(arr)) => {
            let mut patterns = Vec::new();
            for item in arr {
                match item {
                    serde_json::Value::String(s) => {
                        let trimmed = s.trim().to_string();
                        if !trimmed.is_empty() {
                            patterns.push(trimmed);
                        }
                    }
                    _ => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": "host_filter must be an array of strings or null"
                            })),
                        )
                            .into_response();
                    }
                }
            }
            if patterns.is_empty() {
                None
            } else {
                Some(patterns)
            }
        }
        Some(other) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "host_filter must be an array of strings or null",
                    "value": other
                })),
            )
                .into_response();
        }
        None => host_filter,
    };

    let new_cfg = madhyamas_core::MirrorConfig {
        enabled,
        output_dir,
        host_filter: host_filter.clone(),
        save_request_bodies: save_req,
    };

    // Apply to the live MirrorWriter config (if attached).
    if let Some(mgr) = state.mirror_writer.as_ref() {
        *mgr.config().write() = new_cfg.clone();
    }

    // Persist to the proxy config so changes survive restarts.
    if let Some(pc) = state.proxy_config.as_ref() {
        let snapshot = {
            let mut config = pc.write();
            config.mirror = new_cfg.clone();
            config.clone()
        };
        if let Err(e) = snapshot.save() {
            tracing::warn!("Failed to persist mirror config to disk: {}", e);
        }
    }

    let resp = serde_json::json!({
        "enabled": new_cfg.enabled,
        "output_dir": new_cfg.output_dir,
        "host_filter": new_cfg.host_filter,
        "save_request_bodies": new_cfg.save_request_bodies,
    });

    (StatusCode::OK, Json(resp)).into_response()
}

// =============================================================================
// Log rotation
// =============================================================================

/// Get the current log rotation status: config, current file path/size, and
/// the list of archived (rotated) log files.
pub async fn get_log_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let Some(handle) = state.log_handle.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Log rotation not available (proxy server mode required)"
            })),
        )
            .into_response();
    };
    let mut status = handle.status_json();
    if let Some(obj) = status.as_object_mut() {
        if let Some(pc) = state.proxy_config.as_ref() {
            obj.insert(
                "debug_logging".to_string(),
                debug_logging_json(&pc.read().debug_logging),
            );
        }
    }
    Json(status).into_response()
}

/// Serialize the debug logging section for API responses.
fn debug_logging_json(cfg: &madhyamas_core::DebugLogConfig) -> serde_json::Value {
    serde_json::json!({
        "enabled": cfg.enabled,
        "level": cfg.level.as_str(),
        "host_filter": cfg.host_filter,
        "redact_headers": cfg.redact_headers,
        "redact_bodies": cfg.redact_bodies,
    })
}

/// Trigger an immediate (on-demand) log file rotation.
///
/// The current `madhyamas.log` is renamed to `madhyamas.log.<timestamp>` and a
/// fresh file is opened. Archived files are pruned to `max_files`. Returns the
/// new archived file path and the updated status.
pub async fn rotate_logs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let Some(handle) = state.log_handle.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Log rotation not available (proxy server mode required)"
            })),
        )
            .into_response();
    };
    match handle.rotate_now() {
        Ok(archive_path) => {
            let mut status = handle.status_json();
            if let Some(obj) = status.as_object_mut() {
                obj.insert(
                    "rotated_to".to_string(),
                    serde_json::Value::String(archive_path.to_string_lossy().to_string()),
                );
            }
            (StatusCode::OK, Json(status)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to rotate log file: {}", e),
            })),
        )
            .into_response(),
    }
}

/// Partial update payload for `PATCH /api/logs`.
#[derive(Debug, Deserialize)]
pub struct PatchLogConfigRequest {
    pub enabled: Option<bool>,
    /// Rotation mode. Accepts `{"mode": "never"|"hourly"|"daily"}` or
    /// `{"mode": "size", "size_mb": <n>}`.
    pub rotation: Option<serde_json::Value>,
    pub max_files: Option<usize>,
    pub max_file_size_mb: Option<u64>,
    pub json_format: Option<bool>,
    /// Proxied-traffic debug logging settings (runtime-toggleable, no
    /// restart required). Applied to the shared proxy config and persisted.
    pub debug_logging: Option<PatchDebugLogConfigRequest>,
}

/// Partial update payload for the `debug_logging` section of
/// `PATCH /api/logs`.
#[derive(Debug, Deserialize)]
pub struct PatchDebugLogConfigRequest {
    pub enabled: Option<bool>,
    /// Verbosity: `"summary"`, `"headers"`, or `"full"`.
    pub level: Option<String>,
    /// Host patterns to log (empty/omitted = all hosts).
    pub host_filter: Option<Vec<String>>,
    /// Header names to redact (case-insensitive).
    pub redact_headers: Option<Vec<String>>,
    /// When `true`, bodies are never logged (size placeholder only).
    pub redact_bodies: Option<bool>,
}

/// Update the log rotation configuration at runtime.
///
/// Changes are applied to the live [`LogHandle`] and persisted to the proxy
/// config so they survive restarts. Note: changing `rotation` mode or
/// `json_format` takes effect on the next restart (the active subscriber
/// layer is installed once at startup); `max_files` and `max_file_size_mb`
/// take effect immediately.
pub async fn update_log_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PatchLogConfigRequest>,
) -> impl IntoResponse {
    let Some(handle) = state.log_handle.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Log rotation not available (proxy server mode required)"
            })),
        )
            .into_response();
    };

    // Start from the current live config.
    let mut cfg = handle.config();

    if let Some(v) = req.enabled {
        cfg.enabled = v;
    }
    if let Some(v) = req.max_files {
        cfg.max_files = v;
    }
    if let Some(v) = req.max_file_size_mb {
        cfg.max_file_size_mb = v;
    }
    if let Some(v) = req.json_format {
        cfg.json_format = v;
    }
    if let Some(rotation_val) = req.rotation {
        match parse_rotation(&rotation_val) {
            Ok(r) => cfg.rotation = r,
            Err(msg) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": msg, "value": rotation_val })),
                )
                    .into_response();
            }
        }
    }

    // Compute the proxied-traffic debug logging section (runtime-toggleable,
    // no restart required) BEFORE applying anything, so an invalid payload
    // leaves the live state untouched.
    let mut debug_cfg = state
        .proxy_config
        .as_ref()
        .map(|pc| pc.read().debug_logging.clone())
        .unwrap_or_default();
    if let Some(req_debug) = req.debug_logging {
        if let Some(v) = req_debug.enabled {
            debug_cfg.enabled = v;
        }
        if let Some(ref level) = req_debug.level {
            match madhyamas_core::DebugLogLevel::parse(level) {
                Some(l) => debug_cfg.level = l,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("invalid debug_logging.level: `{}` (expected summary, headers, or full)", level),
                        })),
                    )
                        .into_response();
                }
            }
        }
        if let Some(v) = req_debug.host_filter {
            debug_cfg.host_filter = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(v) = req_debug.redact_headers {
            debug_cfg.redact_headers = v;
        }
        if let Some(v) = req_debug.redact_bodies {
            debug_cfg.redact_bodies = v;
        }
    }

    // Apply to the live handle.
    handle.update_config(cfg.clone());

    // Persist to the proxy config so changes survive restarts.
    if let Some(pc) = state.proxy_config.as_ref() {
        let snapshot = {
            let mut config = pc.write();
            config.log_config = cfg.clone();
            config.debug_logging = debug_cfg.clone();
            config.clone()
        };
        if let Err(e) = snapshot.save() {
            tracing::warn!("Failed to persist log config to disk: {}", e);
        }
    }

    let resp = serde_json::json!({
        "enabled": cfg.enabled,
        "rotation": cfg.rotation.label(),
        "max_files": cfg.max_files,
        "max_file_size_mb": cfg.max_file_size_mb,
        "json_format": cfg.json_format,
        "debug_logging": debug_logging_json(&debug_cfg),
        "message": "Log configuration updated (rotation mode/json_format changes take effect on next restart; size/max_files and debug_logging take effect immediately)",
    });
    (StatusCode::OK, Json(resp)).into_response()
}

/// Parse a rotation JSON value into a [`madhyamas_core::LogRotation`].
///
/// Accepts:
/// - `{"mode": "never"}` / `{"mode": "hourly"}` / `{"mode": "daily"}`
/// - `{"mode": "size", "size_mb": <n>}`
fn parse_rotation(val: &serde_json::Value) -> Result<madhyamas_core::LogRotation, String> {
    let obj = val
        .as_object()
        .ok_or_else(|| "rotation must be an object with a \"mode\" field".to_string())?;
    let mode = obj
        .get("mode")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "rotation.mode is required (never|hourly|daily|size)".to_string())?;
    match mode {
        "never" => Ok(madhyamas_core::LogRotation::Never),
        "hourly" => Ok(madhyamas_core::LogRotation::Hourly),
        "daily" => Ok(madhyamas_core::LogRotation::Daily),
        "size" => {
            let size_mb = obj
                .get("size_mb")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "rotation.size_mb is required when mode=size".to_string())?;
            if size_mb == 0 {
                return Err("rotation.size_mb must be > 0".to_string());
            }
            Ok(madhyamas_core::LogRotation::SizeMB { size_mb })
        }
        other => Err(format!("unknown rotation mode: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use madhyamas_core::log_rotation::RotatingFileWriter;
    use madhyamas_core::{DebugLogConfig, DebugLogLevel, LogConfig, ProxyConfig};

    async fn make_state() -> AppState {
        let store = madhyamas_core::TrafficStore::new(":memory:")
            .await
            .expect("in-memory store");
        let dir = std::env::temp_dir().join(format!(
            "madhyamas-log-tests-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let writer = RotatingFileWriter::new(&dir, LogConfig::default()).unwrap();
        AppState::new(store).with_log_handle(madhyamas_core::LogHandle::new(writer))
    }

    /// Invoke a handler and return (status, JSON body).
    async fn respond(
        resp: axum::response::Response,
    ) -> (axum::http::StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn update_log_config_applies_full_debug_logging_section() {
        let state = Arc::new(make_state().await);
        let req: PatchLogConfigRequest = serde_json::from_str(
            r#"{"debug_logging": {
                "enabled": true,
                "level": "full",
                "host_filter": ["*.example.com"],
                "redact_headers": ["X-Secret"],
                "redact_bodies": true
            }}"#,
        )
        .unwrap();

        let (status, body) = respond(
            update_log_config(State(state), Json(req))
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        let d = &body["debug_logging"];
        assert_eq!(d["enabled"], true);
        assert_eq!(d["level"], "full");
        assert_eq!(d["host_filter"], serde_json::json!(["*.example.com"]));
        assert_eq!(d["redact_headers"], serde_json::json!(["X-Secret"]));
        assert_eq!(d["redact_bodies"], true);
    }

    #[tokio::test]
    async fn update_log_config_partial_debug_logging_keeps_defaults() {
        let state = Arc::new(make_state().await);
        let req: PatchLogConfigRequest =
            serde_json::from_str(r#"{"debug_logging": {"enabled": true}}"#).unwrap();

        let (status, body) = respond(
            update_log_config(State(state), Json(req))
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        let d = &body["debug_logging"];
        // Untouched fields keep the defaults.
        assert_eq!(d["level"], "summary");
        assert_eq!(d["host_filter"], serde_json::Value::Null);
        assert_eq!(
            d["redact_headers"],
            serde_json::json!(["Authorization", "Cookie", "Set-Cookie"])
        );
        assert_eq!(d["redact_bodies"], false);
    }

    #[tokio::test]
    async fn update_log_config_rejects_invalid_debug_level() {
        let state = Arc::new(make_state().await);
        let req: PatchLogConfigRequest =
            serde_json::from_str(r#"{"debug_logging": {"enabled": true, "level": "verbose"}}"#)
                .unwrap();

        let (status, body) = respond(
            update_log_config(State(state), Json(req))
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("verbose"));
    }

    #[tokio::test]
    async fn update_log_config_normalizes_empty_host_filter_to_null() {
        let state = Arc::new(make_state().await);
        let req: PatchLogConfigRequest =
            serde_json::from_str(r#"{"debug_logging": {"enabled": true, "host_filter": []}}"#)
                .unwrap();

        let (status, body) = respond(
            update_log_config(State(state), Json(req))
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(
            body["debug_logging"]["host_filter"],
            serde_json::Value::Null
        );
    }

    #[tokio::test]
    async fn update_log_config_without_debug_logging_section_is_noop() {
        let state = Arc::new(make_state().await);
        let req: PatchLogConfigRequest = serde_json::from_str(r#"{"max_files": 3}"#).unwrap();

        let (status, body) = respond(
            update_log_config(State(state), Json(req))
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body["debug_logging"]["enabled"], false);
    }

    #[tokio::test]
    async fn get_log_status_includes_debug_logging_section() {
        let store = madhyamas_core::TrafficStore::new(":memory:")
            .await
            .expect("in-memory store");
        let dir = std::env::temp_dir().join(format!(
            "madhyamas-log-tests-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let writer = RotatingFileWriter::new(&dir, LogConfig::default()).unwrap();
        let cfg = ProxyConfig {
            debug_logging: DebugLogConfig {
                enabled: true,
                level: DebugLogLevel::Headers,
                host_filter: Some(vec!["api.example.com".to_string()]),
                redact_headers: vec!["Authorization".to_string()],
                redact_bodies: true,
            },
            ..ProxyConfig::default()
        };
        let state = Arc::new(
            AppState::new(store)
                .with_log_handle(madhyamas_core::LogHandle::new(writer))
                .with_proxy_config(Arc::new(parking_lot::RwLock::new(cfg))),
        );

        let (status, body) = respond(get_log_status(State(state)).await.into_response()).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body["debug_logging"]["enabled"], true);
        assert_eq!(body["debug_logging"]["level"], "headers");
        assert_eq!(
            body["debug_logging"]["host_filter"],
            serde_json::json!(["api.example.com"])
        );
        assert_eq!(body["debug_logging"]["redact_bodies"], true);
    }

    #[test]
    fn patch_debug_log_request_deserializes_partial_payloads() {
        let req: PatchDebugLogConfigRequest =
            serde_json::from_str(r#"{"level": "headers"}"#).unwrap();
        assert_eq!(req.level.as_deref(), Some("headers"));
        assert_eq!(req.enabled, None);
        assert_eq!(req.host_filter, None);
        assert_eq!(req.redact_headers, None);
        assert_eq!(req.redact_bodies, None);
    }

    #[test]
    fn debug_logging_json_serializes_all_fields() {
        let cfg = DebugLogConfig {
            enabled: true,
            level: DebugLogLevel::Full,
            host_filter: Some(vec!["a.com".to_string()]),
            redact_headers: vec!["Authorization".to_string()],
            redact_bodies: true,
        };
        let v = debug_logging_json(&cfg);
        assert_eq!(v["enabled"], true);
        assert_eq!(v["level"], "full");
        assert_eq!(v["host_filter"], serde_json::json!(["a.com"]));
        assert_eq!(v["redact_headers"], serde_json::json!(["Authorization"]));
        assert_eq!(v["redact_bodies"], true);
    }
}

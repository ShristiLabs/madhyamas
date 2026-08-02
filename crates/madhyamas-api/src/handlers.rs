//! API handlers

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use madhyamas_core::{AccessControlList, ProxyConfig, TrafficFilter, WsFilter};
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
        is_passthrough: query.is_passthrough.and_then(|s| match s.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }),
        host: query.host,
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

    match state.traffic_store.get_by_id(&id) {
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
    {
        Ok(result) => {
            if req.switch_session {
                let _ = state.traffic_store.switch_session(&result.session_id);
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
    match state.traffic_store.get_capture_stats() {
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

            // Save block list entries
            for entry in state.block_list_manager.get_entries() {
                if let Err(e) = store.save_block_list_entry(&entry) {
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

            // Load block list entries
            match store.load_block_list_entries() {
                Ok(entries) => {
                    state.block_list_manager.clear();
                    for entry in entries {
                        state.block_list_manager.add_entry(entry);
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
    match state.traffic_store.list_focus_hosts() {
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
    match state.traffic_store.add_focus_host(&req.pattern) {
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
    match state.traffic_store.remove_focus_host(&id) {
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
    match state.traffic_store.clear_focus_hosts() {
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

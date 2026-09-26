//! Enterprise API handlers - authentication, user management, RBAC, audit
//! logs, performance monitoring, and onboarding.
//!
//! Persistent data (users, API keys, auth sessions, audit events) is served
//! from an [`EnterpriseStore`] injected via [`axum::Extension`]. The auth
//! manager ([`AuthManager`]) is likewise injected so login/token handlers can
//! issue JWTs. This keeps [`madhyamas_api::AppState`] free of any dependency
//! on this crate.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use madhyamas_api::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::audit::{AuditEventType, AuditFilter};
use crate::credentials::{hash_password, validate_password_complexity, verify_password};
use crate::store::{ApiKeyRecord, AuthSession, EnterpriseStore, UserUpdate};
use crate::{
    ApiKey, AuditEvent, AuthManager, JwtClaims, License, Permission, RbacManager, ResourceType,
    User, UserRole, UserStatus,
};

// ============== Stub Types ==============

/// Metrics snapshot (stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub requests_total: u64,
    pub requests_success: u64,
    pub requests_failed: u64,
    pub avg_latency_ms: f64,
    pub requests_per_second: f64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            requests_total: 0,
            requests_success: 0,
            requests_failed: 0,
            avg_latency_ms: 0.0,
            requests_per_second: 0.0,
        }
    }
}

/// Metrics collector (stub)
#[derive(Debug, Default)]
pub struct MetricsCollector;

impl MetricsCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn snapshot(&self) -> Metrics {
        Metrics::default()
    }
}

/// Health check status (stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub healthy: bool,
    pub version: String,
    pub uptime_secs: u64,
    pub memory_usage_mb: u64,
    pub active_connections: u64,
    pub details: HashMap<String, String>,
    /// License status summary (Phase 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<LicenseHealth>,
    /// Overall status: "ok", "degraded", or "error" (Phase 6d).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Dependency health statuses (Phase 6d).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Dependencies>,
    /// Deployment tier: "enterprise" or "community" (Phase 7a).
    pub tier: String,
    /// Authentication mode: "none", "local", "oidc", "header", or "ldap".
    pub auth_mode: String,
    /// Whether authentication is required for API access.
    pub auth_required: bool,
}

/// Dependency health statuses for the detailed health check (Phase 6d).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    pub database: String,
    pub redis: String,
    pub license: String,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_secs: 0,
            memory_usage_mb: 0,
            active_connections: 0,
            details: HashMap::new(),
            license: None,
            status: None,
            dependencies: None,
            tier: "enterprise".to_string(),
            auth_mode: "local".to_string(),
            auth_required: false,
        }
    }
}

/// License status embedded in the health-check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseHealth {
    pub licensed: bool,
    /// RFC 3339 expiry timestamp (omitted when unlicensed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Role info (stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

// ============================================================================
// Performance & Metrics Handlers
// ============================================================================

/// Get current metrics
pub async fn get_metrics(State(_state): State<Arc<AppState>>) -> Json<Metrics> {
    let collector = MetricsCollector::new();
    Json(collector.snapshot())
}

/// Cluster-wide metrics response (Phase 6e). Aggregates per-instance metrics
/// from all active instances registered in Redis.
#[derive(Debug, Serialize)]
pub struct ClusterMetricsResponse {
    pub instances: Vec<InstanceSummary>,
    pub total_active_connections: u64,
    pub total_request_count: u64,
    pub avg_cpu_usage: f64,
    pub avg_memory_usage_mb: f64,
}

/// Per-instance summary in the cluster metrics response.
#[derive(Debug, Serialize)]
pub struct InstanceSummary {
    pub instance_id: String,
    pub addr: String,
    pub last_heartbeat: i64,
    pub status: String,
    pub cpu_usage: f64,
    pub memory_usage_mb: u64,
    pub active_connections: u64,
    pub request_count: u64,
    pub uptime_secs: u64,
}

/// `GET /api/metrics/cluster` — aggregate metrics from all instances via
/// Redis. Admin-only (requires authentication). Returns 503 when Redis is
/// not configured (single-instance mode).
pub async fn get_cluster_metrics(
    State(_state): State<Arc<AppState>>,
    Extension(redis): Extension<Option<Arc<crate::RedisState>>>,
) -> Result<Json<ClusterMetricsResponse>, StatusCode> {
    let rs = redis.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let instances = rs
        .list_instances_with_metrics()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let summaries: Vec<InstanceSummary> = instances
        .iter()
        .map(|info| {
            let m = info.metrics.clone().unwrap_or_default();
            InstanceSummary {
                instance_id: info.instance_id.clone(),
                addr: info.addr.clone(),
                last_heartbeat: info.last_heartbeat,
                status: "active".to_string(),
                cpu_usage: m.cpu_usage,
                memory_usage_mb: m.memory_usage_mb,
                active_connections: m.active_connections,
                request_count: m.request_count,
                uptime_secs: m.uptime_secs,
            }
        })
        .collect();
    let total_active_connections = summaries.iter().map(|s| s.active_connections).sum();
    let total_request_count = summaries.iter().map(|s| s.request_count).sum();
    let count = summaries.len().max(1);
    let avg_cpu_usage = summaries.iter().map(|s| s.cpu_usage).sum::<f64>() / count as f64;
    let avg_memory_usage_mb = summaries
        .iter()
        .map(|s| s.memory_usage_mb as f64)
        .sum::<f64>()
        / count as f64;
    Ok(Json(ClusterMetricsResponse {
        instances: summaries,
        total_active_connections,
        total_request_count,
        avg_cpu_usage,
        avg_memory_usage_mb,
    }))
}

/// Instances list response (Phase 6e).
#[derive(Debug, Serialize)]
pub struct InstancesResponse {
    pub instances: Vec<InstanceStatus>,
}

/// Per-instance status in the instances list response.
#[derive(Debug, Serialize)]
pub struct InstanceStatus {
    pub instance_id: String,
    pub addr: String,
    pub last_heartbeat: i64,
    pub status: String,
}

/// `GET /api/instances` — list all active instances registered in Redis.
/// Admin-only. Returns 503 when Redis is not configured.
pub async fn get_instances(
    State(_state): State<Arc<AppState>>,
    Extension(redis): Extension<Option<Arc<crate::RedisState>>>,
) -> Result<Json<InstancesResponse>, StatusCode> {
    let rs = redis.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let instances = rs
        .list_instances()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let statuses = instances
        .iter()
        .map(|info| InstanceStatus {
            instance_id: info.instance_id.clone(),
            addr: info.addr.clone(),
            last_heartbeat: info.last_heartbeat,
            status: "active".to_string(),
        })
        .collect();
    Ok(Json(InstancesResponse {
        instances: statuses,
    }))
}

/// Get health check. Includes a license status summary when the enterprise
/// tier is active (Phase 3) and dependency checks (database, Redis, license)
/// for load-balancer health probes (Phase 6d).
pub async fn get_health_check(
    State(state): State<Arc<AppState>>,
    Extension(license): Extension<Option<License>>,
    Extension(redis): Extension<Option<Arc<crate::RedisState>>>,
    Extension(auth): Extension<Arc<AuthManager>>,
) -> Json<HealthCheck> {
    // Database: ping the traffic store to verify the connection is alive
    // and the schema is initialized. This prevents the load balancer from
    // routing traffic to an instance whose schema initialization has not
    // completed (issue #10).
    let db_status = match state.traffic_store.ping().await {
        Ok(()) => "ok".to_string(),
        Err(e) => {
            tracing::warn!("Health check database ping failed: {e}");
            "error".to_string()
        }
    };

    // Redis: "ok" if connected and PING succeeds, "error" on failure,
    // "not_configured" when --redis-url was not provided.
    let redis_status = match &redis {
        Some(rs) => match rs.ping().await {
            Ok(()) => "ok".to_string(),
            Err(e) => {
                tracing::warn!("Health check Redis ping failed: {e}");
                "error".to_string()
            }
        },
        None => "not_configured".to_string(),
    };

    // License: "ok" if present and not expired, "expired" if past expiry,
    // "not_configured" when no license file was provided.
    let license_status = match &license {
        Some(l) if l.is_expired() => "expired".to_string(),
        Some(_) => "ok".to_string(),
        None => "not_configured".to_string(),
    };

    let deps = Dependencies {
        database: db_status,
        redis: redis_status,
        license: license_status,
    };

    // Overall status: "ok" when all deps are ok or not_configured;
    // "degraded" when any dep is not ok but none is "error";
    // "error" when any dep is "error".
    let overall = if deps.database == "error" || deps.redis == "error" || deps.license == "error" {
        "error"
    } else if deps.database != "ok"
        || (deps.redis != "ok" && deps.redis != "not_configured")
        || (deps.license != "ok" && deps.license != "not_configured")
    {
        "degraded"
    } else {
        "ok"
    };

    Json(HealthCheck {
        healthy: overall == "ok",
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        memory_usage_mb: 0,
        active_connections: 0,
        details: Default::default(),
        license: Some(license_health(&license)),
        status: Some(overall.to_string()),
        dependencies: Some(deps),
        tier: "enterprise".to_string(),
        auth_mode: "local".to_string(),
        auth_required: auth.require_auth(),
    })
}

/// Build a [`LicenseHealth`] summary from the optional verified license.
fn license_health(license: &Option<License>) -> LicenseHealth {
    match license {
        Some(l) => LicenseHealth {
            licensed: true,
            expires_at: Some(l.claims.expires_at.to_rfc3339()),
        },
        None => LicenseHealth {
            licensed: false,
            expires_at: None,
        },
    }
}

/// License info response returned by `GET /api/license`.
///
/// When a license is present, all claim fields are included. When no license
/// was provided at startup, only `licensed: false` is returned (the endpoint
/// is informational, not a gate).
#[derive(Debug, Serialize)]
pub struct LicenseInfoResponse {
    pub licensed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
}

/// `GET /api/license` — return the active license info (public, no auth).
///
/// Returns `200` with `{ "licensed": true, ... }` when a verified license is
/// active, or `200` with `{ "licensed": false }` when running in unlicensed
/// enterprise mode.
pub async fn get_license_info(
    State(_state): State<Arc<AppState>>,
    Extension(license): Extension<Option<License>>,
) -> Json<LicenseInfoResponse> {
    match license {
        Some(l) => Json(LicenseInfoResponse {
            licensed: true,
            license_id: Some(l.claims.license_id.clone()),
            customer: Some(l.claims.customer.clone()),
            plan: Some(l.claims.plan.clone()),
            seats: Some(l.claims.seats),
            instance_id: Some(l.claims.instance_id.clone()),
            issued_at: Some(l.claims.issued_at.to_rfc3339()),
            expires_at: Some(l.claims.expires_at.to_rfc3339()),
            features: Some(l.claims.features.clone()),
            verified_at: Some(l.verified_at.to_rfc3339()),
        }),
        None => Json(LicenseInfoResponse {
            licensed: false,
            license_id: None,
            customer: None,
            plan: None,
            seats: None,
            instance_id: None,
            issued_at: None,
            expires_at: None,
            features: None,
            verified_at: None,
        }),
    }
}

/// Performance stats response
#[derive(Debug, Serialize)]
pub struct PerformanceStatsResponse {
    pub metrics: Metrics,
    pub memory: MemoryInfoResponse,
    pub pool: PoolStatsResponse,
}

#[derive(Debug, Serialize)]
pub struct MemoryInfoResponse {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct PoolStatsResponse {
    pub total_connections: u64,
    pub idle_connections: u64,
    pub active_connections: u64,
    pub pending_requests: u64,
}

/// Get performance stats
pub async fn get_performance_stats(
    State(_state): State<Arc<AppState>>,
) -> Json<PerformanceStatsResponse> {
    Json(PerformanceStatsResponse {
        metrics: MetricsCollector::new().snapshot(),
        memory: MemoryInfoResponse {
            used_bytes: 0,
            total_bytes: 0,
            usage_percent: 0.0,
        },
        pool: PoolStatsResponse {
            total_connections: 0,
            idle_connections: 0,
            active_connections: 0,
            pending_requests: 0,
        },
    })
}

// ============================================================================
// Authentication Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserInfo,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    /// Effective feature scopes when the caller authenticated with an API
    /// key (issue #107): the stored grants expanded to their taxonomy
    /// equivalents (see [`crate::auth::effective_scopes`]). `None` for
    /// JWT principals — their authorization is role-based. Consumed by the
    /// MCP server to filter its tool list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}

/// Login handler. Looks up the user by username in the enterprise store,
/// verifies the password against the stored Argon2id hash, issues an access
/// JWT plus a longer-lived refresh token via the injected [`AuthManager`],
/// creates a persisted auth session, and records a login audit event.
pub async fn login(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(auth): Extension<Arc<AuthManager>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let (user, password_hash) = store
        .get_user_credentials(&req.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let matched = verify_password(&req.password, &password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !matched {
        audit.log(
            AuditEvent::new(AuditEventType::Login, "login failed: bad credentials")
                .with_user(user.id.clone()),
        );
        return Err(StatusCode::UNAUTHORIZED);
    }
    let role = user.role.as_label().to_string();
    let (token, refresh_token, session_id, expires_at) = auth
        .generate_token_pair(&user.id, &role)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = chrono::Utc::now();
    let expires_dt = chrono::DateTime::<chrono::Utc>::from_timestamp(expires_at, 0)
        .unwrap_or(now + chrono::Duration::hours(1));
    let session = AuthSession {
        id: session_id.clone(),
        user_id: user.id.clone(),
        jwt_jti: session_id.clone(),
        created_at: now.to_rfc3339(),
        expires_at: expires_dt.to_rfc3339(),
        last_activity: now.to_rfc3339(),
        revoked: false,
    };
    store
        .create_session(&session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // Update last_login timestamp (best-effort; does not fail the login).
    let _ = store
        .update_user(
            &user.id,
            &UserUpdate {
                last_login: Some(now.timestamp()),
                ..Default::default()
            },
        )
        .await;
    audit.log(AuditEvent::new(AuditEventType::Login, "user logged in").with_user(user.id.clone()));
    Ok(Json(LoginResponse {
        token,
        refresh_token,
        user: UserInfo {
            id: user.id.clone(),
            username: user.username,
            email: user.email.unwrap_or_default(),
            role,
            scopes: None,
        },
        expires_at,
    }))
}

/// Logout handler. Revokes the authenticated user's session in the store so
/// the token can no longer be refreshed or used past idle timeout.
pub async fn logout(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
) -> Result<StatusCode, StatusCode> {
    if let Some(ref sid) = claims.session_id {
        store
            .revoke_session(sid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    audit.log(
        AuditEvent::new(AuditEventType::Logout, "user logged out")
            .with_user(claims.user_id.clone()),
    );
    Ok(StatusCode::OK)
}

/// Get current user. Returns the authenticated user's profile from the store.
pub async fn get_current_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
) -> Result<Json<UserInfo>, StatusCode> {
    let user = store
        .get_user(&claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(UserInfo {
        id: user.id,
        username: user.username,
        email: user.email.unwrap_or_default(),
        role: user.role.as_label().to_string(),
        // Key principals report their effective (taxonomy-expanded)
        // scopes so API-key clients — notably the MCP server — can adapt
        // to what the key may do (issue #107 tool filtering).
        scopes: claims.scopes.clone(),
    }))
}

/// Validate JWT token. Verifies the token via the injected [`AuthManager`]
/// and returns its claims.
pub async fn validate_token(
    State(_state): State<Arc<AppState>>,
    Extension(auth): Extension<Arc<AuthManager>>,
    Json(token): Json<String>,
) -> Result<Json<JwtClaims>, StatusCode> {
    let claims = auth
        .validate_jwt(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(Json(claims))
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

/// Refresh token endpoint. Validates the supplied refresh token, checks the
/// associated session is still active in the store, and issues a fresh
/// access + refresh token pair. This route is public (no `Authorization`
/// header required) — it authenticates via the refresh token itself.
pub async fn refresh_token(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(auth): Extension<Arc<AuthManager>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, StatusCode> {
    let claims = auth
        .validate_refresh_token(&req.refresh_token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    // Confirm the session is still valid (not revoked / expired).
    if let Some(sid) = &claims.sid {
        let session = store
            .get_session(sid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        match session {
            Some(s) if !s.revoked => {}
            _ => return Err(StatusCode::UNAUTHORIZED),
        }
    }
    let role = claims.role.clone();
    let (token, refresh_token, _session_id, expires_at) = auth
        .generate_token_pair(&claims.sub, &role)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(RefreshResponse {
        token,
        refresh_token,
        expires_at,
    }))
}

// ============================================================================
// API Key Handlers
// ============================================================================

/// Get all API keys for current user
pub async fn get_api_keys(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
) -> Result<Json<Vec<ApiKey>>, StatusCode> {
    let records = store
        .list_api_keys(&claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        records
            .into_iter()
            .map(|r| ApiKey {
                id: r.id,
                user_id: r.user_id,
                key: r.key_prefix,
                name: r.name,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc).timestamp())
                    .unwrap_or(0),
                expires_at: r.expires_at.as_deref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp())
                }),
                is_active: true,
                last_used: r.last_used_at.as_deref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp())
                }),
                scopes: serde_json::from_str(&r.scopes).unwrap_or_default(),
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub expires_in_days: Option<i64>,
}

/// Create new API key. Generates a key, hashes it with SHA-256, persists the
/// record with scopes, and returns the plaintext key once to the caller.
pub async fn create_api_key(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKey>, StatusCode> {
    let api_key = ApiKey::generate(&claims.user_id, &req.name);
    let now = chrono::Utc::now();
    let expires_at = req.expires_in_days.map(|d| now + chrono::Duration::days(d));
    let key_hash = crate::auth::hash_api_key(&api_key.key);
    let scopes_json = serde_json::to_string(&req.scopes).unwrap_or_else(|_| "[]".into());
    let record = ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: api_key.user_id.clone(),
        name: api_key.name.clone(),
        key_hash,
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: scopes_json,
        expires_at: expires_at.map(|t| t.to_rfc3339()),
        last_used_at: None,
        created_at: now.to_rfc3339(),
    };
    store
        .create_api_key(&record)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut response_key = api_key.clone();
    response_key.scopes = req.scopes.clone();
    audit.log(
        AuditEvent::new(AuditEventType::ApiKeyCreated, "API key created")
            .with_user(claims.user_id.clone())
            .with_metadata("key_name", serde_json::json!(req.name)),
    );
    Ok(Json(response_key))
}

/// Revoke API key
pub async fn revoke_api_key(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path(key_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    store
        .revoke_api_key(&key_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::ApiKeyRevoked, "API key revoked")
            .with_user(claims.user_id.clone())
            .with_metadata("key_id", serde_json::json!(key_id)),
    );
    Ok(StatusCode::OK)
}

// ============================================================================
// Device Management Handlers (issue #104)
// ============================================================================

/// A registered device principal, as returned by the devices API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub owner_user_id: String,
    pub install_uuid: Option<String>,
    pub mac_address: Option<String>,
    /// `active` or `revoked`.
    pub status: String,
    /// Unix seconds.
    pub created_at: i64,
    /// Unix seconds of the last proxy-auth event for this device
    /// (device CONNECT heartbeat), if any.
    pub last_seen: Option<i64>,
}

impl From<crate::store::DeviceRecord> for Device {
    fn from(r: crate::store::DeviceRecord) -> Self {
        Self {
            id: r.id,
            name: r.name,
            owner_user_id: r.owner_user_id,
            install_uuid: r.install_uuid,
            mac_address: r.mac_address,
            status: r.status,
            created_at: rfc3339_to_unix(&r.created_at),
            last_seen: r.last_seen.as_deref().and_then(rfc3339_to_unix_opt),
        }
    }
}

fn rfc3339_to_unix(s: &str) -> i64 {
    rfc3339_to_unix_opt(s).unwrap_or(0)
}

fn rfc3339_to_unix_opt(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp())
}

/// A device together with its show-once credential — returned by create
/// and rotate. The plaintext `mdy_dev_...` key is displayed exactly once
/// (mirroring API-key creation) and never persisted or shown again.
#[derive(Debug, Serialize)]
pub struct DeviceWithKey {
    pub device: Device,
    /// Plaintext per-device credential — shown once, never stored.
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    /// Human-readable device name (required so traffic views are
    /// readable from the first captured entry).
    pub name: String,
    #[serde(default)]
    pub install_uuid: Option<String>,
    #[serde(default)]
    pub mac_address: Option<String>,
}

/// List the current user's registered devices.
pub async fn get_devices(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
) -> Result<Json<Vec<Device>>, StatusCode> {
    let records = store
        .list_devices(&claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(records.into_iter().map(Device::from).collect()))
}

/// Register a device and mint its initial per-device credential.
/// The plaintext key is returned exactly once (show-once semantics).
pub async fn create_device(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Json(req): Json<CreateDeviceRequest>,
) -> Result<Json<DeviceWithKey>, StatusCode> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let now = chrono::Utc::now();
    let record = crate::store::DeviceRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        owner_user_id: claims.user_id.clone(),
        install_uuid: req.install_uuid,
        mac_address: req.mac_address,
        status: "active".to_string(),
        created_at: now.to_rfc3339(),
        last_seen: None,
    };
    store
        .create_device(&record)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = mint_device_key(&store, &record.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::DeviceRegistered, "Device registered")
            .with_user(claims.user_id.clone())
            .with_metadata("device_id", serde_json::json!(record.id))
            .with_metadata("device_name", serde_json::json!(record.name)),
    );
    Ok(Json(DeviceWithKey {
        device: Device::from(record),
        key,
    }))
}

/// Rotate a device's credential: the previous key is deactivated and a
/// new one is minted. The plaintext key is returned exactly once.
/// Agent keys are deliberately NOT touched: their binding is
/// referential (`parent_device_id`), not key material, so agents keep
/// working across rotation (issue #108).
pub async fn rotate_device_key(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path(device_id): Path<String>,
) -> Result<Json<DeviceWithKey>, StatusCode> {
    let device = load_owned_device(&store, &claims, &device_id).await?;
    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = mint_device_key(&store, &device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::DeviceKeyRotated, "Device key rotated")
            .with_user(claims.user_id.clone())
            .with_metadata("device_id", serde_json::json!(device.id)),
    );
    Ok(Json(DeviceWithKey {
        device: Device::from(device),
        key,
    }))
}

/// Revoke a device: its credentials are deactivated, its agent keys are
/// cascaded a revoke (issue #108 — the device is gone, debugging it is
/// meaningless), and its status flips to `revoked`, so subsequent
/// CONNECTs with its key are rejected.
pub async fn revoke_device(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path(device_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let device = load_owned_device(&store, &claims, &device_id).await?;
    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .revoke_enrollment_tokens_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .revoke_agent_keys_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .update_device_status(&device.id, "revoked")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::DeviceRevoked, "Device revoked")
            .with_user(claims.user_id.clone())
            .with_metadata("device_id", serde_json::json!(device.id))
            .with_metadata("deleted", serde_json::json!(false)),
    );
    Ok(StatusCode::OK)
}

/// Delete a device: revokes its credentials and agent keys (audit
/// `DeviceRevoked`) and removes the device record.
pub async fn delete_device(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path(device_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let device = load_owned_device(&store, &claims, &device_id).await?;
    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .revoke_enrollment_tokens_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .revoke_agent_keys_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .delete_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::DeviceRevoked, "Device deleted")
            .with_user(claims.user_id.clone())
            .with_metadata("device_id", serde_json::json!(device.id))
            .with_metadata("deleted", serde_json::json!(true)),
    );
    Ok(StatusCode::OK)
}

/// A device together with a freshly issued enrollment token — returned by
/// the enrollment-token endpoint (issue #106). The plaintext
/// `mdy_enroll_...` token is displayed exactly once (in the QR payload)
/// and never persisted or shown again; `expires_at` lets the dialog count
/// down to the 15-minute TTL.
#[derive(Debug, Serialize)]
pub struct DeviceWithEnrollmentToken {
    pub device: Device,
    /// Plaintext enrollment token — carried by the QR, never stored.
    pub token: String,
    /// RFC 3339 timestamp after which redemption is rejected.
    pub expires_at: String,
}

/// Issue a short-lived, single-use enrollment token for a device
/// (issue #106). The onboarding QR carries this token instead of the
/// long-lived key, so a photographed QR expires. The token is redeemable
/// exactly once within its TTL via the public enroll endpoint.
pub async fn issue_device_enrollment_token(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path(device_id): Path<String>,
) -> Result<Json<DeviceWithEnrollmentToken>, StatusCode> {
    let device = load_owned_device(&store, &claims, &device_id).await?;
    if device.status != "active" {
        return Err(StatusCode::CONFLICT);
    }
    let now = chrono::Utc::now();
    // Opportunistic cleanup: expired tokens are dead rows; prune them so
    // the table stays bounded. Best-effort — never fails the issuance.
    let _ = store
        .delete_expired_enrollment_tokens(&now.to_rfc3339())
        .await;
    let record = crate::store::EnrollmentTokenRecord {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: device.id.clone(),
        token_hash: String::new(),
        token_prefix: String::new(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::seconds(crate::auth::ENROLLMENT_TOKEN_TTL_SECS))
            .to_rfc3339(),
        redeemed_at: None,
        revoked_at: None,
    };
    let (token, record) = mint_enrollment_token(&store, record)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(
            AuditEventType::DeviceEnrollmentIssued,
            "Device enrollment token issued",
        )
        .with_user(claims.user_id.clone())
        .with_metadata("device_id", serde_json::json!(device.id))
        .with_metadata("expires_at", serde_json::json!(record.expires_at)),
    );
    Ok(Json(DeviceWithEnrollmentToken {
        device: Device::from(device),
        token,
        expires_at: record.expires_at,
    }))
}

#[derive(Debug, Deserialize)]
pub struct EnrollDeviceRequest {
    /// The `mdy_enroll_...` token from the QR payload.
    pub token: String,
}

/// Redeem an enrollment token for the real long-lived device credential
/// (issue #106). This is the exchange a smart client (companion app)
/// performs after scanning the QR; it is a PUBLIC endpoint because the
/// token itself is the credential — the device has no web session yet.
///
/// Redemption is atomic and single-use: the store stamps `redeemed_at`
/// only when the token is unredeemed, unrevoked, and unexpired, so
/// concurrent attempts race on the update and exactly one wins. On
/// success the device's existing keys are retired (one live credential
/// per device — the create-time show-once key dies here) and a fresh
/// `mdy_dev_` key is minted and returned exactly once. Neither the token
/// nor the key is ever logged or audited — only the device ID.
pub async fn enroll_device(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    Json(req): Json<EnrollDeviceRequest>,
) -> Result<Json<DeviceWithKey>, StatusCode> {
    let token = req.token.trim();
    if !crate::auth::is_enrollment_token(token) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let now = chrono::Utc::now().to_rfc3339();
    let token_hash = crate::auth::hash_api_key(token);
    // Atomic single-use + TTL + not-revoked enforcement (rows-affected
    // compare-and-set): `false` means unknown, expired, already used, or
    // revoked — indistinguishable by design to avoid token enumeration.
    let redeemed = store
        .redeem_enrollment_token(&token_hash, &now)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !redeemed {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let record = store
        .get_enrollment_token_by_hash(&token_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let device = store
        .get_device(&record.device_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if device.status != "active" {
        return Err(StatusCode::UNAUTHORIZED);
    }
    // One live credential per device: retire the create-time show-once
    // key (and anything older) — mirrors rotate semantics, and kills the
    // photographed-dialog credential the moment the real device enrolls.
    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = mint_device_key(&store, &device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(
            AuditEventType::DeviceEnrolled,
            "Device enrolled via enrollment token",
        )
        .with_user(device.owner_user_id.clone())
        .with_metadata("device_id", serde_json::json!(device.id))
        .with_metadata("via", serde_json::json!("enrollment_token")),
    );
    Ok(Json(DeviceWithKey {
        device: Device::from(device),
        key,
    }))
}

/// Persist an enrollment token record with its hash filled in, returning
/// the plaintext token alongside the completed record.
async fn mint_enrollment_token(
    store: &Arc<dyn EnterpriseStore>,
    mut record: crate::store::EnrollmentTokenRecord,
) -> std::result::Result<(String, crate::store::EnrollmentTokenRecord), crate::store::StoreError> {
    let token = crate::auth::generate_enrollment_token();
    record.token_hash = crate::auth::hash_api_key(&token);
    record.token_prefix = token.chars().take(12).collect();
    store.create_enrollment_token(&record).await?;
    Ok((token, record))
}

// ============================================================================
// Device-Derived Agent Key Handlers (issue #108)
// ============================================================================

/// The issue #107 feature-scope taxonomy agent keys may draw from. A
/// device-derived agent key is minted with a user-picked subset of these;
/// anything else (including the user-key `*` wildcard) is rejected —
/// deny-by-default is the point of the picker.
pub const AGENT_KEY_TAXONOMY: &[&str] = &[
    "traffic:read",
    "traffic:export",
    "sessions:read",
    "mocks:read",
    "mocks:write",
    "rewrites:read",
    "rewrites:write",
    "breakpoints:read",
    "breakpoints:write",
    "blocklist:read",
    "blocklist:write",
    "throttle:read",
    "throttle:write",
    "replay:execute",
    "config:read",
    "config:write",
];

/// One-click scope presets from the minting-flow design (docs/
/// CREDENTIAL_ONBOARDING.md *Minting flow and agent UX*). Presets are
/// shortcuts, never the only choice — the request may always add or
/// remove individual scopes.
pub const AGENT_KEY_PRESETS: &[(&str, &[&str])] = &[
    ("read-only-agent", &["traffic:read", "config:read"]),
    (
        "intercept-agent",
        &[
            "traffic:read",
            "config:read",
            "mocks:read",
            "mocks:write",
            "rewrites:read",
            "rewrites:write",
            "breakpoints:read",
            "breakpoints:write",
            "blocklist:read",
            "blocklist:write",
            "throttle:read",
            "throttle:write",
        ],
    ),
];

/// Expand a preset name to its scope list, or `None` when unknown.
pub fn agent_key_preset_scopes(preset: &str) -> Option<Vec<String>> {
    AGENT_KEY_PRESETS
        .iter()
        .find(|(name, _)| *name == preset)
        .map(|(_, scopes)| scopes.iter().map(|s| s.to_string()).collect())
}

/// An agent key row as returned by the devices API — metadata only, no
/// secret material (the hash never leaves the store; the plaintext was
/// shown once at mint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentKey {
    pub id: String,
    /// Device this key is scoped to.
    pub parent_device_id: String,
    /// Owning user (device owner, denormalized at mint).
    pub owner_user_id: String,
    pub name: String,
    /// Non-secret preview of the `mdy_agent_...` key.
    pub key_prefix: String,
    pub scopes: Vec<String>,
    /// Unix seconds.
    pub created_at: i64,
    /// Unix seconds, `None` = never expires.
    pub expires_at: Option<i64>,
    /// `active` or `revoked`.
    pub status: String,
    /// Unix seconds of the last request made with this key.
    pub last_used: Option<i64>,
}

impl From<crate::store::AgentKeyRecord> for AgentKey {
    fn from(r: crate::store::AgentKeyRecord) -> Self {
        Self {
            id: r.id,
            parent_device_id: r.parent_device_id,
            owner_user_id: r.owner_user_id,
            name: r.name,
            key_prefix: r.key_prefix,
            scopes: serde_json::from_str(&r.scopes).unwrap_or_default(),
            created_at: rfc3339_to_unix(&r.created_at),
            expires_at: r.expires_at.as_deref().and_then(rfc3339_to_unix_opt),
            status: if r.revoked_at.is_some() {
                "revoked".to_string()
            } else {
                "active".to_string()
            },
            last_used: r.last_used_at.as_deref().and_then(rfc3339_to_unix_opt),
        }
    }
}

/// A freshly minted agent key together with its show-once plaintext.
#[derive(Debug, Serialize)]
pub struct AgentKeyWithSecret {
    pub key: AgentKey,
    /// Plaintext `mdy_agent_...` credential — shown once, never stored.
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentKeyRequest {
    /// Optional human label for the device's agent list.
    #[serde(default)]
    pub name: Option<String>,
    /// Preset shortcut (`read-only-agent` / `intercept-agent`); unioned
    /// with `scopes`.
    #[serde(default)]
    pub preset: Option<String>,
    /// Explicit feature scopes from the #107 taxonomy.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Optional expiry in days (> 0).
    pub expires_in_days: Option<i64>,
}

/// Validate and collect the effective scope set for a mint request:
/// preset-expanded ∪ explicit, every entry checked against the taxonomy
/// (`*` and unknown scopes rejected), at least one scope required.
/// Returns the deduplicated scope list or the offending scope.
fn validate_agent_scopes(req: &CreateAgentKeyRequest) -> Result<Vec<String>, String> {
    let mut scopes: Vec<String> = Vec::new();
    if let Some(ref preset) = req.preset {
        match agent_key_preset_scopes(preset) {
            Some(preset_scopes) => scopes.extend(preset_scopes),
            None => return Err(format!("unknown preset: {preset}")),
        }
    }
    for scope in &req.scopes {
        if !AGENT_KEY_TAXONOMY.contains(&scope.as_str()) {
            return Err(format!(
                "scope not in the agent-key taxonomy: {scope} \
                 (allowed: {})",
                AGENT_KEY_TAXONOMY.join(", ")
            ));
        }
        scopes.push(scope.clone());
    }
    // Deduplicate (preset ∪ explicit may overlap) with a stable order.
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty() {
        return Err("at least one scope is required".to_string());
    }
    Ok(scopes)
}

/// Mint a device-derived agent key (issue #108).
///
/// `POST /api/devices/{id}/agent-keys` — JWT-only (the whole `/devices`
/// surface rejects API keys, so a device key can never self-mint). The
/// key is **referentially** bound to the device: the row carries
/// `parent_device_id`, the material is independent random, and rotating
/// the device key later does not disturb it. Scopes are validated
/// against the #107 taxonomy (presets are unioned in); expiry is
/// optional. The plaintext is returned exactly once.
pub async fn create_agent_key(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path(device_id): Path<String>,
    Json(req): Json<CreateAgentKeyRequest>,
) -> Result<Json<AgentKeyWithSecret>, StatusCode> {
    let device = load_owned_device(&store, &claims, &device_id).await?;
    if device.status != "active" {
        return Err(StatusCode::CONFLICT);
    }
    let scopes = validate_agent_scopes(&req).map_err(|_| StatusCode::BAD_REQUEST)?;
    let now = chrono::Utc::now();
    let expires_at = req
        .expires_in_days
        .filter(|d| *d > 0)
        .map(|d| now + chrono::Duration::days(d));
    let plaintext = crate::auth::generate_agent_key();
    let record = crate::store::AgentKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        parent_device_id: device.id.clone(),
        owner_user_id: device.owner_user_id.clone(),
        name: req.name.unwrap_or_default().trim().to_string(),
        key_hash: crate::auth::hash_api_key(&plaintext),
        key_prefix: plaintext.chars().take(12).collect(),
        scopes: serde_json::to_string(&scopes).unwrap_or_else(|_| "[]".into()),
        created_at: now.to_rfc3339(),
        expires_at: expires_at.map(|t| t.to_rfc3339()),
        revoked_at: None,
        last_used_at: None,
    };
    store
        .create_agent_key(&record)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::ApiKeyCreated, "Agent key minted for device")
            .with_user(claims.user_id.clone())
            .with_metadata("parent_device_id", serde_json::json!(device.id))
            .with_metadata("key_kind", serde_json::json!("agent"))
            .with_metadata(
                "key_name",
                serde_json::json!(if record.name.is_empty() {
                    agent_key_display_name(&record)
                } else {
                    record.name.clone()
                }),
            ),
    );
    Ok(Json(AgentKeyWithSecret {
        key: AgentKey::from(record),
        secret: plaintext,
    }))
}

/// Non-secret display label for audit rows when the mint carried no name.
fn agent_key_display_name(record: &crate::store::AgentKeyRecord) -> String {
    format!("{}…", record.key_prefix)
}

/// List a device's agent keys (metadata only).
pub async fn list_agent_keys(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path(device_id): Path<String>,
) -> Result<Json<Vec<AgentKey>>, StatusCode> {
    let device = load_owned_device(&store, &claims, &device_id).await?;
    let records = store
        .list_agent_keys(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(records.into_iter().map(AgentKey::from).collect()))
}

/// Revoke one agent key. The parent device, its device key, and sibling
/// agent keys are untouched — killing one noisy agent must not disturb
/// the device (docs/CREDENTIAL_ONBOARDING.md security properties).
pub async fn revoke_agent_key(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Path((device_id, key_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let device = load_owned_device(&store, &claims, &device_id).await?;
    // The key must belong to this device — a mismatched pair is a 404,
    // not a revoke of someone else's key.
    let records = store
        .list_agent_keys(&device.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !records.iter().any(|r| r.id == key_id) {
        return Err(StatusCode::NOT_FOUND);
    }
    store
        .revoke_agent_key(&key_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::ApiKeyRevoked, "Agent key revoked")
            .with_user(claims.user_id.clone())
            .with_metadata("parent_device_id", serde_json::json!(device.id))
            .with_metadata("key_kind", serde_json::json!("agent")),
    );
    Ok(StatusCode::OK)
}

/// Mint and persist a new `mdy_dev_` credential for `device_id`,
/// returning the plaintext once.
async fn mint_device_key(
    store: &Arc<dyn EnterpriseStore>,
    device_id: &str,
) -> std::result::Result<String, crate::store::StoreError> {
    let key = crate::auth::generate_device_key();
    let record = crate::store::DeviceKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: device_id.to_string(),
        key_hash: crate::auth::hash_api_key(&key),
        key_prefix: key.chars().take(12).collect(),
        created_at: chrono::Utc::now().to_rfc3339(),
        revoked_at: None,
        last_used_at: None,
    };
    store.create_device_key(&record).await?;
    Ok(key)
}

/// Load a device and enforce ownership: the owner or an admin may act on
/// it, anyone else gets `403`; a missing device is `404`.
async fn load_owned_device(
    store: &Arc<dyn EnterpriseStore>,
    claims: &crate::middleware::AuthUser,
    device_id: &str,
) -> Result<crate::store::DeviceRecord, StatusCode> {
    let device = store
        .get_device(device_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let is_owner = device.owner_user_id == claims.user_id;
    let is_admin = claims.role == "admin";
    if !is_owner && !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(device)
}

// ============================================================================
// User Management Handlers
// ============================================================================

/// Get all users
pub async fn get_users(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let users = store
        .list_users()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(users))
}

/// Get user by ID
pub async fn get_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Path(user_id): Path<String>,
) -> Result<Json<User>, StatusCode> {
    let user = store
        .get_user(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(user))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: UserRole,
}

/// Create user. Hashes the password with Argon2id, persists the user with the
/// hash in the dedicated `password_hash` column, and returns the created
/// record (without credential material). Passwords must pass the complexity
/// check (Phase 9.9) before hashing.
pub async fn create_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<User>, (StatusCode, Json<serde_json::Value>)> {
    if store
        .get_user_by_username(&req.username)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal server error"})),
            )
        })?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "username already exists"})),
        ));
    }
    validate_password_complexity(&req.password).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid password", "message": e.to_string()})),
        )
    })?;
    let password_hash = hash_password(&req.password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "password hashing failed"})),
        )
    })?;
    let user = User::new(
        uuid::Uuid::new_v4().to_string(),
        req.username.clone(),
        Some(req.email.clone()),
        req.role,
        req.username.clone(),
        UserStatus::Active,
    );
    store
        .create_user(&user, &password_hash)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to create user"})),
            )
        })?;
    audit.log(
        AuditEvent::new(AuditEventType::Custom, "user created")
            .with_metadata("username", serde_json::json!(req.username))
            .with_metadata("user_id", serde_json::json!(user.id)),
    );
    Ok(Json(user))
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub password: Option<String>,
}

/// Update user. Applies partial updates (email, role, status, password) and
/// returns the updated record. When a new password is supplied it must pass
/// the complexity check (Phase 9.9) and is hashed with Argon2id before being
/// persisted.
pub async fn update_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<User>, (StatusCode, Json<serde_json::Value>)> {
    let password_hash = match req.password {
        Some(ref pw) if !pw.is_empty() => {
            validate_password_complexity(pw).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "invalid password",
                        "message": e.to_string()
                    })),
                )
            })?;
            Some(hash_password(pw).map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "password hashing failed"})),
                )
            })?)
        }
        _ => None,
    };
    let updates = UserUpdate {
        username: None,
        email: req.email,
        password_hash,
        role: req.role.map(|r| r.as_label().to_string()),
        status: req.status.map(|s| {
            match s {
                UserStatus::Active => "active",
                UserStatus::Inactive => "inactive",
                UserStatus::Suspended => "suspended",
                UserStatus::PendingVerification => "pending_verification",
            }
            .to_string()
        }),
        preferences: None,
        last_login: None,
    };
    store.update_user(&user_id, &updates).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "failed to update user"})),
        )
    })?;
    let user = store
        .get_user(&user_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal server error"})),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "user not found"})),
        ))?;
    audit.log(
        AuditEvent::new(AuditEventType::Custom, "user updated")
            .with_metadata("user_id", serde_json::json!(user_id)),
    );
    Ok(Json(user))
}

/// Delete user
pub async fn delete_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    Path(user_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    store
        .delete_user(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    audit.log(
        AuditEvent::new(AuditEventType::Custom, "user deleted")
            .with_metadata("user_id", serde_json::json!(user_id)),
    );
    Ok(StatusCode::OK)
}

// ============================================================================
// RBAC Handlers
// ============================================================================

/// Get all roles with their permissions from the RBAC matrix.
pub async fn get_roles(State(_state): State<Arc<AppState>>) -> Json<Vec<Role>> {
    let rbac = RbacManager::new();
    let roles = vec![
        UserRole::Admin,
        UserRole::User,
        UserRole::Viewer,
        UserRole::ReadOnly,
    ];
    Json(
        roles
            .into_iter()
            .map(|role| {
                let perms: Vec<String> = rbac
                    .get_permissions(&role)
                    .into_iter()
                    .map(|(rt, p)| format!("{:?}:{:?}", rt, p))
                    .collect();
                Role {
                    name: role.as_label().to_string(),
                    description: role_description(&role),
                    permissions: perms,
                }
            })
            .collect(),
    )
}

fn role_description(role: &UserRole) -> String {
    match role {
        UserRole::Admin => "Full access to all resources and administrative actions".to_string(),
        UserRole::User => {
            "Read and write access to traffic, mocks, rewrites, breakpoints".to_string()
        }
        UserRole::Viewer => "Read-only access to all resources".to_string(),
        UserRole::ReadOnly => "Read-only access (same as viewer)".to_string(),
    }
}

/// Get all permissions for a given role (query param `?role=<label>`).
/// If no role is specified, returns all known permission labels.
pub async fn get_permissions(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<PermissionQuery>,
) -> Json<Vec<String>> {
    let rbac = RbacManager::new();
    if let Some(ref role_label) = query.role {
        let role = UserRole::from_label(role_label);
        let perms: Vec<String> = rbac
            .get_permissions(&role)
            .into_iter()
            .map(|(rt, p)| format!("{:?}:{:?}", rt, p))
            .collect();
        return Json(perms);
    }
    // No role specified — return the full set of distinct permission labels.
    let mut all: Vec<String> = Vec::new();
    for role in [
        UserRole::Admin,
        UserRole::User,
        UserRole::Viewer,
        UserRole::ReadOnly,
    ] {
        for (rt, p) in rbac.get_permissions(&role) {
            let label = format!("{:?}:{:?}", rt, p);
            if !all.contains(&label) {
                all.push(label);
            }
        }
    }
    Json(all)
}

#[derive(Debug, Deserialize)]
pub struct PermissionQuery {
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CheckPermissionRequest {
    pub user_id: String,
    pub resource: String,
    pub permission: String,
}

/// Check if a user has a permission. Looks up the user's role from the store
/// and checks it against the RBAC matrix.
pub async fn check_permission(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Json(req): Json<CheckPermissionRequest>,
) -> Result<Json<bool>, StatusCode> {
    let user = store
        .get_user(&req.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let rbac = RbacManager::new();
    let resource_type = parse_resource_type(&req.resource);
    let permission = parse_permission(&req.permission);
    let (rt, perm) = match (resource_type, permission) {
        (Some(rt), Some(p)) => (rt, p),
        _ => return Ok(Json(false)),
    };
    Ok(Json(rbac.has_permission(&user.role, rt, perm)))
}

fn parse_resource_type(s: &str) -> Option<ResourceType> {
    match s.to_lowercase().as_str() {
        "traffic" => Some(ResourceType::Traffic),
        "session" => Some(ResourceType::Session),
        "mock" => Some(ResourceType::Mock),
        "rewrite" => Some(ResourceType::Rewrite),
        "breakpoint" => Some(ResourceType::Breakpoint),
        "script" => Some(ResourceType::Script),
        "plugin" => Some(ResourceType::Plugin),
        "config" => Some(ResourceType::Config),
        _ => None,
    }
}

fn parse_permission(s: &str) -> Option<Permission> {
    match s.to_lowercase().as_str() {
        "read" => Some(Permission::Read),
        "write" => Some(Permission::Write),
        "delete" => Some(Permission::Delete),
        "execute" => Some(Permission::Execute),
        _ => None,
    }
}

// ============================================================================
// Audit Log Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub event_types: Option<String>,
    pub user_id: Option<String>,
    pub resource: Option<String>,
    pub success: Option<bool>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

fn audit_filter_from_query(query: &AuditQuery) -> AuditFilter {
    AuditFilter {
        event_type: query.event_types.as_deref().map(parse_event_type),
        user_id: query.user_id.clone(),
        start_time: query.start_time.map(|t| {
            chrono::DateTime::from_timestamp(t, 0)
                .unwrap_or_default()
                .with_timezone(&chrono::Utc)
        }),
        end_time: query.end_time.map(|t| {
            chrono::DateTime::from_timestamp(t, 0)
                .unwrap_or_default()
                .with_timezone(&chrono::Utc)
        }),
        limit: query.limit,
        offset: query.offset,
    }
}

fn parse_event_type(label: &str) -> AuditEventType {
    match label {
        "login" => AuditEventType::Login,
        "logout" => AuditEventType::Logout,
        "api_key_created" => AuditEventType::ApiKeyCreated,
        "api_key_revoked" => AuditEventType::ApiKeyRevoked,
        "device_registered" => AuditEventType::DeviceRegistered,
        "device_key_rotated" => AuditEventType::DeviceKeyRotated,
        "device_revoked" => AuditEventType::DeviceRevoked,
        "device_enrollment_issued" => AuditEventType::DeviceEnrollmentIssued,
        "device_enrolled" => AuditEventType::DeviceEnrolled,
        "traffic_exported" => AuditEventType::TrafficExported,
        "session_created" => AuditEventType::SessionCreated,
        "session_deleted" => AuditEventType::SessionDeleted,
        "mock_created" => AuditEventType::MockCreated,
        "mock_deleted" => AuditEventType::MockDeleted,
        "breakpoint_created" => AuditEventType::BreakpointCreated,
        "breakpoint_deleted" => AuditEventType::BreakpointDeleted,
        "config_changed" => AuditEventType::ConfigChanged,
        _ => AuditEventType::Custom,
    }
}

/// Get audit events
pub async fn get_audit_events(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEvent>>, StatusCode> {
    let filter = audit_filter_from_query(&query);
    let events = store
        .query_audit_events(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(events))
}

/// Audit statistics
#[derive(Debug, Serialize)]
pub struct AuditStatsResponse {
    pub total_events: u64,
    pub events_today: u64,
    pub events_by_type: HashMap<String, u64>,
    pub top_users: Vec<String>,
    pub error_count: u64,
}

/// Get audit statistics
pub async fn get_audit_stats(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
) -> Result<Json<AuditStatsResponse>, StatusCode> {
    let stats = store
        .get_audit_stats()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(AuditStatsResponse {
        total_events: stats.total_events as u64,
        events_today: stats.events_today as u64,
        events_by_type: stats
            .events_by_type
            .into_iter()
            .map(|(k, v)| (k, v as u64))
            .collect(),
        top_users: Vec::new(),
        error_count: 0,
    }))
}

/// Export audit events
pub async fn export_audit_events(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEvent>>, StatusCode> {
    let filter = audit_filter_from_query(&query);
    let events = store
        .query_audit_events(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(events))
}

/// Clear audit events
pub async fn clear_audit_events(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
) -> Result<StatusCode, StatusCode> {
    store
        .clear_audit_events()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

// ============================================================================
// Onboarding Handlers
// ============================================================================

#[derive(Debug, Serialize)]
pub struct OnboardingStatus {
    pub completed: bool,
    pub current_step: u32,
    pub total_steps: u32,
    pub steps: Vec<OnboardingStep>,
}

#[derive(Debug, Serialize)]
pub struct OnboardingStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub optional: bool,
}

/// Get onboarding status
pub async fn get_onboarding_status(State(_state): State<Arc<AppState>>) -> Json<OnboardingStatus> {
    Json(OnboardingStatus {
        completed: false,
        current_step: 1,
        total_steps: 6,
        steps: vec![
            OnboardingStep {
                id: "welcome".to_string(),
                title: "Welcome to Madhyamas".to_string(),
                description: "Get started with Madhyamas".to_string(),
                completed: false,
                optional: false,
            },
            OnboardingStep {
                id: "certificate".to_string(),
                title: "Install Certificate".to_string(),
                description: "Install the root CA certificate to intercept HTTPS traffic"
                    .to_string(),
                completed: false,
                optional: false,
            },
            OnboardingStep {
                id: "proxy".to_string(),
                title: "Configure Proxy".to_string(),
                description: "Set up your browser or app to use the proxy".to_string(),
                completed: false,
                optional: false,
            },
            OnboardingStep {
                id: "device".to_string(),
                title: "Connect a Device".to_string(),
                description: "Register a phone or tablet and scan its QR to capture its \
                              traffic under its own identity"
                    .to_string(),
                completed: false,
                optional: true,
            },
            OnboardingStep {
                id: "features".to_string(),
                title: "Explore Features".to_string(),
                description: "Learn about breakpoints, mocks, and more".to_string(),
                completed: false,
                optional: true,
            },
            OnboardingStep {
                id: "tips".to_string(),
                title: "Pro Tips".to_string(),
                description: "Tips and tricks for power users".to_string(),
                completed: false,
                optional: true,
            },
        ],
    })
}

#[derive(Debug, Deserialize)]
pub struct CompleteOnboardingStepRequest {
    pub step_id: String,
}

/// Complete onboarding step
pub async fn complete_onboarding_step(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<CompleteOnboardingStepRequest>,
) -> StatusCode {
    StatusCode::OK
}

/// Skip onboarding
pub async fn skip_onboarding(State(_state): State<Arc<AppState>>) -> StatusCode {
    StatusCode::OK
}

// ============================================================================
// Configuration Export/Import
// ============================================================================

/// Export all configuration
pub async fn export_config(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "settings": {},
        "rules": {},
    }))
}

#[derive(Debug, Deserialize)]
pub struct ImportConfigRequest {
    pub config: serde_json::Value,
}

/// Import configuration
pub async fn import_config(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<ImportConfigRequest>,
) -> StatusCode {
    StatusCode::OK
}

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
use crate::credentials::{hash_password, verify_password};
use crate::store::{ApiKeyRecord, AuthSession, EnterpriseStore, UserUpdate};
use crate::{
    ApiKey, AuditEvent, AuthManager, JwtClaims, License, Permission, User, UserRole, UserStatus,
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

/// Get health check. Includes a license status summary when the enterprise
/// tier is active (Phase 3).
pub async fn get_health_check(
    State(_state): State<Arc<AppState>>,
    Extension(license): Extension<Option<License>>,
) -> Json<HealthCheck> {
    Json(HealthCheck {
        healthy: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        memory_usage_mb: 0,
        active_connections: 0,
        details: Default::default(),
        license: Some(license_health(&license)),
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
    if let Some(sid) = &claims.0 .0.sid {
        store
            .revoke_session(sid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    audit.log(
        AuditEvent::new(AuditEventType::Logout, "user logged out")
            .with_user(claims.0 .0.sub.clone()),
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
        .get_user(&claims.0 .0.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(UserInfo {
        id: user.id,
        username: user.username,
        email: user.email.unwrap_or_default(),
        role: user.role.as_label().to_string(),
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
        .list_api_keys(&claims.0 .0.sub)
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
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_in_days: Option<i64>,
}

/// Create new API key. Generates a key, persists a hashed record, and returns
/// the plaintext key once to the caller.
pub async fn create_api_key(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    claims: axum::Extension<crate::middleware::AuthUser>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKey>, StatusCode> {
    let api_key = ApiKey::generate(&claims.0 .0.sub, &req.name);
    let now = chrono::Utc::now();
    let expires_at = req.expires_in_days.map(|d| now + chrono::Duration::days(d));
    let key_hash = hash_password(&api_key.key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let record = ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: api_key.user_id.clone(),
        name: api_key.name.clone(),
        key_hash,
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: "[]".to_string(),
        expires_at: expires_at.map(|t| t.to_rfc3339()),
        last_used_at: None,
        created_at: now.to_rfc3339(),
    };
    store
        .create_api_key(&record)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(api_key))
}

/// Revoke API key
pub async fn revoke_api_key(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Path(key_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    store
        .revoke_api_key(&key_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
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
/// record (without credential material).
pub async fn create_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<User>, StatusCode> {
    if store
        .get_user_by_username(&req.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some()
    {
        return Err(StatusCode::CONFLICT);
    }
    let password_hash =
        hash_password(&req.password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
/// returns the updated record. When a new password is supplied it is hashed
/// with Argon2id before being persisted.
pub async fn update_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<User>, StatusCode> {
    let password_hash = match req.password {
        Some(ref pw) if !pw.is_empty() => {
            Some(hash_password(pw).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
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
    store
        .update_user(&user_id, &updates)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user = store
        .get_user(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(user))
}

/// Delete user
pub async fn delete_user(
    State(_state): State<Arc<AppState>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Path(user_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    store
        .delete_user(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

// ============================================================================
// RBAC Handlers
// ============================================================================

/// Get all roles
pub async fn get_roles(State(_state): State<Arc<AppState>>) -> Json<Vec<Role>> {
    Json(Vec::new())
}

/// Get all permissions
pub async fn get_permissions(State(_state): State<Arc<AppState>>) -> Json<Vec<Permission>> {
    Json(Vec::new())
}

#[derive(Debug, Deserialize)]
pub struct CheckPermissionRequest {
    pub user_id: String,
    pub permission: String,
    pub resource: Option<String>,
}

/// Check if user has permission
pub async fn check_permission(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<CheckPermissionRequest>,
) -> Json<bool> {
    Json(false)
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
        total_steps: 5,
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

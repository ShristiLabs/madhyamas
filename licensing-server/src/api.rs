//! Axum API handlers for the licensing server.
//!
//! Exposes license issuance, retrieval, revocation, verification, seat
//! tracking, customer portal auth, team management, Stripe billing, and admin
//! portal endpoints under `/api`. The legacy `X-Admin-Key` endpoints remain
//! for backward compatibility with the proxy binary's seat-tracking flow; new
//! customer-facing and admin-facing endpoints use JWT auth (Phase 12b/12d).

use crate::auth::{self, TokenKind};
use crate::db::{self, DbError, LicenseRow};
use crate::license::{self, LicenseClaims, LicenseFile, LicenseSigner, SignError};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Shared application state passed to all handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub signer: std::sync::Arc<LicenseSigner>,
    pub public_key: VerifyingKey,
    pub admin_key: String,
    /// Optional Stripe API key. When `None`, billing endpoints return 503.
    pub stripe_api_key: Option<String>,
    /// Optional Stripe webhook signing secret (for verifying webhook payloads).
    pub stripe_webhook_secret: Option<String>,
}

/// API error type — maps domain errors to HTTP status codes.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("database error: {0}")]
    Db(#[from] DbError),
    #[error("signing error: {0}")]
    Sign(#[from] SignError),
    #[error("auth error: {0}")]
    Auth(#[from] auth::AuthError),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    #[allow(dead_code)]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Db(DbError::LicenseNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("not found: {id}"))
            }
            ApiError::Db(DbError::SeatNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("seat not found: {id}"))
            }
            ApiError::Db(DbError::SeatLimitReached { active, limit }) => (
                StatusCode::CONFLICT,
                format!("seat limit reached: active={active}, limit={limit}"),
            ),
            ApiError::Db(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Sign(SignError::Expired { expires_at }) => {
                (StatusCode::OK, format!("license expired at {expires_at}"))
            }
            ApiError::Sign(SignError::InvalidSignature) => {
                (StatusCode::OK, "invalid signature".to_string())
            }
            ApiError::Sign(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Auth(_) => (StatusCode::UNAUTHORIZED, "invalid credentials".to_string()),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_string()),
            ApiError::ServiceUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
        };
        let body = Json(serde_json::json!({ "error": message }));
        (status, body).into_response()
    }
}

/// Build the API router with all endpoints.
pub fn router(state: AppState) -> Router {
    Router::new()
        // Legacy admin-key endpoints (12a) — kept for backward compat.
        .route("/api/licenses", post(create_license).get(list_licenses))
        .route("/api/licenses/verify", post(verify_license))
        .route("/api/licenses/{id}", get(get_license))
        .route("/api/licenses/{id}/revoke", post(revoke_license))
        .route("/api/seats/register", post(register_seat))
        .route("/api/seats/heartbeat", post(heartbeat_seat))
        .route("/api/seats/deregister", post(deregister_seat))
        .route("/api/seats/{license_id}", get(list_seats))
        // Customer auth (12b.2)
        .route("/api/auth/register", post(register_customer))
        .route("/api/auth/login", post(login_customer))
        .route("/api/auth/me", get(get_me))
        // Customer license dashboard (12b.3)
        .route("/api/customer/licenses", get(customer_list_licenses))
        .route("/api/customer/licenses/{id}", get(customer_get_license))
        .route("/api/customer/seats/{license_id}", get(customer_list_seats))
        // Customer team management (12b.4)
        .route(
            "/api/customer/team",
            get(list_team).post(invite_team_member),
        )
        .route("/api/customer/team/{id}", post(remove_team_member))
        // Customer billing (12c.5)
        .route("/api/customer/billing", get(customer_billing))
        // Stripe billing (12c.1, 12c.2, 12c.5)
        .route("/api/billing/checkout", post(stripe_checkout))
        .route("/api/billing/webhook", post(stripe_webhook))
        .route("/api/billing/portal", post(stripe_portal))
        // Admin auth (12d.1)
        .route("/api/admin/login", post(admin_login))
        // Admin customer management (12d.2)
        .route("/api/admin/customers", get(admin_list_customers))
        .route("/api/admin/customers/{id}", get(admin_get_customer))
        .route(
            "/api/admin/customers/{id}/suspend",
            post(admin_suspend_customer),
        )
        .route(
            "/api/admin/customers/{id}/activate",
            post(admin_activate_customer),
        )
        // Admin license management (12d.3)
        .route("/api/admin/licenses", post(admin_create_license))
        .route(
            "/api/admin/licenses/{id}/revoke",
            post(admin_revoke_license),
        )
        .route(
            "/api/admin/licenses/{id}/extend",
            post(admin_extend_license),
        )
        // Admin dashboard (12d.4)
        .route("/api/admin/dashboard", get(admin_dashboard))
        .route("/health", get(health))
        .with_state(state)
}

/// Check the X-Admin-Key header against the configured admin key.
fn check_admin(headers: &HeaderMap, admin_key: &str) -> Result<(), ApiError> {
    let provided = headers
        .get("X-Admin-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    if provided == admin_key {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// ---------------------------------------------------------------------------
// License issuance / retrieval / revocation
// ---------------------------------------------------------------------------

/// Request body for `POST /api/licenses`.
#[derive(Debug, Deserialize)]
pub struct CreateLicenseRequest {
    pub customer_id: String,
    /// Optional customer display name for the license claims. Defaults to
    /// `customer_id` when absent.
    pub customer_name: Option<String>,
    pub plan: String,
    pub seats: u32,
    pub expires_at: DateTime<Utc>,
    pub features: Vec<String>,
    pub instance_id: Option<String>,
}

/// Create a license: sign the claims with Ed25519, store the record, and
/// return the complete [`LicenseFile`].
pub async fn create_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateLicenseRequest>,
) -> Result<Json<LicenseFile>, ApiError> {
    check_admin(&headers, &state.admin_key)?;

    if req.seats == 0 {
        return Err(ApiError::BadRequest("seats must be > 0".to_string()));
    }
    if req.customer_id.is_empty() {
        return Err(ApiError::BadRequest("customer_id is required".to_string()));
    }

    let license_id = format!("lic_{}", random_id(12));
    let customer = req.customer_name.unwrap_or_else(|| req.customer_id.clone());
    let now = Utc::now();

    let claims = LicenseClaims {
        license_id: license_id.clone(),
        customer,
        plan: req.plan.clone(),
        seats: req.seats,
        instance_id: req.instance_id.clone().unwrap_or_default(),
        issued_at: now,
        expires_at: req.expires_at,
        features: req.features.clone(),
    };

    let file = state.signer.sign_license(&claims)?;

    let row = LicenseRow {
        id: Uuid::new_v4(),
        customer_id: req.customer_id.clone(),
        license_id: license_id.clone(),
        plan: req.plan,
        seats: req.seats as i32,
        instance_id: req.instance_id,
        issued_at: now,
        expires_at: req.expires_at,
        features: serde_json::to_value(&req.features).unwrap_or(serde_json::json!([])),
        status: "active".to_string(),
        signature: file.signature.clone(),
    };

    db::insert_license(&state.pool, &row).await?;

    db::insert_audit(
        &state.pool,
        "license.issued",
        None,
        &format!(
            "license {license_id} issued for customer {}",
            req.customer_id
        ),
        serde_json::json!({ "license_id": license_id, "plan": row.plan, "seats": row.seats }),
    )
    .await
    .ok();

    Ok(Json(file))
}

/// `GET /api/licenses/:id` — retrieve a license by its string ID.
pub async fn get_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<LicenseRow>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    let row = db::get_license_by_id(&state.pool, &id).await?;
    Ok(Json(row))
}

/// Query parameters for `GET /api/licenses`.
#[derive(Debug, Deserialize)]
pub struct ListLicensesQuery {
    pub customer_id: Option<String>,
}

/// `GET /api/licenses` — list licenses, optionally filtered by customer_id.
pub async fn list_licenses(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListLicensesQuery>,
) -> Result<Json<Vec<LicenseRow>>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    let rows = db::list_licenses(&state.pool, q.customer_id.as_deref()).await?;
    Ok(Json(rows))
}

/// `POST /api/licenses/:id/revoke` — revoke a license.
pub async fn revoke_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<LicenseRow>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    let row = db::revoke_license(&state.pool, &id).await?;
    db::insert_audit(
        &state.pool,
        "license.revoked",
        None,
        &format!("license {id} revoked"),
        serde_json::json!({ "license_id": id }),
    )
    .await
    .ok();
    Ok(Json(row))
}

// ---------------------------------------------------------------------------
// License verification
// ---------------------------------------------------------------------------

/// Response body for `POST /api/licenses/verify`.
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub reason: Option<String>,
}

/// `POST /api/licenses/verify` — verify a license file's signature and expiry.
pub async fn verify_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(file): Json<LicenseFile>,
) -> Result<Json<VerifyResponse>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    match license::verify_license(&file, &state.public_key) {
        Ok(()) => Ok(Json(VerifyResponse {
            valid: true,
            reason: None,
        })),
        Err(SignError::Expired { expires_at }) => Ok(Json(VerifyResponse {
            valid: false,
            reason: Some(format!("license expired at {expires_at}")),
        })),
        Err(SignError::InvalidSignature) => Ok(Json(VerifyResponse {
            valid: false,
            reason: Some("invalid signature".to_string()),
        })),
        Err(e) => Err(ApiError::Sign(e)),
    }
}

// ---------------------------------------------------------------------------
// Seat tracking
// ---------------------------------------------------------------------------

/// Request body for `POST /api/seats/register`.
#[derive(Debug, Deserialize)]
pub struct RegisterSeatRequest {
    pub license_id: String,
    pub instance_id: String,
    /// Optional address (host:port) of the registering instance. Stored in
    /// audit metadata but not persisted as a column in Phase 12a.
    pub address: Option<String>,
}

/// Response body for seat registration.
#[derive(Debug, Serialize)]
pub struct SeatOperationResponse {
    pub success: bool,
    pub active_seats: i64,
    pub seat_limit: i32,
}

/// `POST /api/seats/register` — register an instance against a license.
pub async fn register_seat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterSeatRequest>,
) -> Result<Json<SeatOperationResponse>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    let license_row = db::get_license_by_id(&state.pool, &req.license_id).await?;
    if license_row.status != "active" {
        return Err(ApiError::Conflict(format!(
            "license {} is not active (status: {})",
            req.license_id, license_row.status
        )));
    }
    let active = db::register_seat(
        &state.pool,
        license_row.id,
        &req.instance_id,
        license_row.seats,
    )
    .await?;

    db::insert_audit(
        &state.pool,
        "seat.registered",
        None,
        &format!(
            "instance {} registered for license {}",
            req.instance_id, req.license_id
        ),
        serde_json::json!({ "instance_id": req.instance_id, "address": req.address }),
    )
    .await
    .ok();

    Ok(Json(SeatOperationResponse {
        success: true,
        active_seats: active,
        seat_limit: license_row.seats,
    }))
}

/// Request body for `POST /api/seats/heartbeat`.
#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub instance_id: String,
}

/// `POST /api/seats/heartbeat` — refresh a seat's heartbeat.
pub async fn heartbeat_seat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    db::heartbeat_seat(&state.pool, &req.instance_id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// Request body for `POST /api/seats/deregister`.
#[derive(Debug, Deserialize)]
pub struct DeregisterSeatRequest {
    pub instance_id: String,
}

/// `POST /api/seats/deregister` — deregister an instance.
pub async fn deregister_seat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DeregisterSeatRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    db::deregister_seat(&state.pool, &req.instance_id).await?;
    db::insert_audit(
        &state.pool,
        "seat.deregistered",
        None,
        &format!("instance {} deregistered", req.instance_id),
        serde_json::json!({ "instance_id": req.instance_id }),
    )
    .await
    .ok();
    Ok(Json(serde_json::json!({ "success": true })))
}

/// `GET /api/seats/:license_id` — list seats for a license.
pub async fn list_seats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(license_id): Path<String>,
) -> Result<Json<Vec<db::SeatRow>>, ApiError> {
    check_admin(&headers, &state.admin_key)?;
    let license_row = db::get_license_by_id(&state.pool, &license_id).await?;
    let seats = db::list_seats(&state.pool, license_row.id).await?;
    Ok(Json(seats))
}

// ---------------------------------------------------------------------------
// Customer auth (Phase 12b.2)
// ---------------------------------------------------------------------------

/// Request body for `POST /api/auth/register`.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub company_name: String,
}

/// Auth response carrying a JWT.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub account_id: Uuid,
}

/// Register a new customer account + customer record and return a JWT.
pub async fn register_customer(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    if req.email.is_empty() || !req.email.contains('@') {
        return Err(ApiError::BadRequest("valid email is required".to_string()));
    }
    if req.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }
    if req.company_name.is_empty() {
        return Err(ApiError::BadRequest("company_name is required".to_string()));
    }

    let password_hash = auth::hash_password(&req.password)
        .map_err(|e| ApiError::BadRequest(format!("password hashing failed: {e}")))?;

    let account_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();

    // Insert account; if email is a duplicate, return a conflict error.
    db::insert_account(
        &state.pool,
        account_id,
        &req.company_name,
        &req.email,
        &password_hash,
    )
    .await
    .map_err(|e| match e {
        DbError::Sqlx(sqlx::Error::Database(ref dbe)) if dbe.is_unique_violation() => {
            ApiError::Conflict("email already registered".to_string())
        }
        other => ApiError::Db(other),
    })?;

    db::insert_customer(
        &state.pool,
        customer_id,
        account_id,
        &req.company_name,
        &req.email,
    )
    .await?;

    let token = auth::issue_customer_token(account_id)?;

    db::insert_audit(
        &state.pool,
        "customer.registered",
        Some(account_id),
        &format!("customer registered: {}", req.email),
        serde_json::json!({ "account_id": account_id, "customer_id": customer_id }),
    )
    .await
    .ok();

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse { token, account_id }),
    ))
}

/// Request body for `POST /api/auth/login`.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login a customer and return a JWT.
pub async fn login_customer(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let account = db::get_account_by_email(&state.pool, &req.email).await?;
    if account.status != "active" {
        return Err(ApiError::Forbidden);
    }
    auth::verify_password(&req.password, &account.password_hash)
        .map_err(|_| ApiError::Unauthorized)?;
    let token = auth::issue_customer_token(account.id)?;
    Ok(Json(AuthResponse {
        token,
        account_id: account.id,
    }))
}

/// Response for `GET /api/auth/me`.
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub account_id: Uuid,
    pub email: String,
    pub name: String,
    pub status: String,
    pub customer: Option<db::CustomerRow>,
}

/// Get the current account info (requires customer JWT).
pub async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, ApiError> {
    let claims = require_customer_token(&headers)?;
    let account_id = parse_uuid(&claims.sub)?;
    let account = db::get_account_by_id(&state.pool, account_id).await?;
    let customer = db::get_customer_by_account(&state.pool, account_id)
        .await
        .ok();
    Ok(Json(MeResponse {
        account_id: account.id,
        email: account.email,
        name: account.name,
        status: account.status,
        customer,
    }))
}

// ---------------------------------------------------------------------------
// Customer license dashboard (Phase 12b.3)
// ---------------------------------------------------------------------------

/// List the current customer's licenses.
pub async fn customer_list_licenses(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LicenseRow>>, ApiError> {
    let claims = require_customer_token(&headers)?;
    let account_id = parse_uuid(&claims.sub)?;
    let customer = db::get_customer_by_account(&state.pool, account_id).await?;
    // The licenses table uses a TEXT customer_id; we use the customer UUID string.
    let rows = db::list_licenses(&state.pool, Some(&customer.id.to_string())).await?;
    Ok(Json(rows))
}

/// Get a single license detail (must own the license).
pub async fn customer_get_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<LicenseRow>, ApiError> {
    let claims = require_customer_token(&headers)?;
    let account_id = parse_uuid(&claims.sub)?;
    let customer = db::get_customer_by_account(&state.pool, account_id).await?;
    let row = db::get_license_by_id(&state.pool, &id).await?;
    if row.customer_id != customer.id.to_string() {
        return Err(ApiError::Forbidden);
    }
    Ok(Json(row))
}

/// List seats for a license (must own the license).
pub async fn customer_list_seats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(license_id): Path<String>,
) -> Result<Json<Vec<db::SeatRow>>, ApiError> {
    let claims = require_customer_token(&headers)?;
    let account_id = parse_uuid(&claims.sub)?;
    let customer = db::get_customer_by_account(&state.pool, account_id).await?;
    let license_row = db::get_license_by_id(&state.pool, &license_id).await?;
    if license_row.customer_id != customer.id.to_string() {
        return Err(ApiError::Forbidden);
    }
    let seats = db::list_seats(&state.pool, license_row.id).await?;
    Ok(Json(seats))
}

// ---------------------------------------------------------------------------
// Customer team management (Phase 12b.4)
// ---------------------------------------------------------------------------

/// Request body for `POST /api/customer/team/invite`.
#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub email: String,
    pub role: String,
}

/// Invite a team member (stored as a pending invitation).
pub async fn invite_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<InviteRequest>,
) -> Result<(StatusCode, Json<db::TeamMemberRow>), ApiError> {
    let claims = require_customer_token(&headers)?;
    let account_id = parse_uuid(&claims.sub)?;
    let customer = db::get_customer_by_account(&state.pool, account_id).await?;
    if req.email.is_empty() || !req.email.contains('@') {
        return Err(ApiError::BadRequest("valid email is required".to_string()));
    }
    let member_id = Uuid::new_v4();
    db::insert_team_member(&state.pool, member_id, customer.id, &req.email, &req.role).await?;
    let row = db::TeamMemberRow {
        id: member_id,
        customer_id: customer.id,
        email: req.email,
        role: req.role,
        status: "invited".to_string(),
        created_at: Utc::now(),
    };
    Ok((StatusCode::CREATED, Json(row)))
}

/// List team members for the current customer.
pub async fn list_team(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<db::TeamMemberRow>>, ApiError> {
    let claims = require_customer_token(&headers)?;
    let account_id = parse_uuid(&claims.sub)?;
    let customer = db::get_customer_by_account(&state.pool, account_id).await?;
    let members = db::list_team_members(&state.pool, customer.id).await?;
    Ok(Json(members))
}

/// Remove a team member.
pub async fn remove_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = require_customer_token(&headers)?;
    let account_id = parse_uuid(&claims.sub)?;
    let customer = db::get_customer_by_account(&state.pool, account_id).await?;
    db::delete_team_member(&state.pool, customer.id, id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// ---------------------------------------------------------------------------
// Stripe billing (Phase 12c)
// ---------------------------------------------------------------------------

/// Request body for `POST /api/billing/checkout`.
#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub plan: String,
    pub success_url: String,
    pub cancel_url: String,
}

/// Response body for checkout.
#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
}

/// Map a plan name to a Stripe price ID from environment variables.
fn plan_to_price_id(plan: &str) -> Option<String> {
    let env_var = match plan {
        "starter" => "STRIPE_PRICE_STARTER",
        "pro" => "STRIPE_PRICE_PRO",
        "enterprise" => "STRIPE_PRICE_ENTERPRISE",
        _ => return None,
    };
    std::env::var(env_var).ok().filter(|s| !s.is_empty())
}

/// Create a Stripe Checkout session. Returns 503 if Stripe is not configured.
pub async fn stripe_checkout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<CheckoutResponse>, ApiError> {
    let _claims = require_customer_token(&headers)?;
    let api_key = state
        .stripe_api_key
        .as_deref()
        .ok_or_else(|| ApiError::ServiceUnavailable("Stripe is not configured".to_string()))?;

    let price_id = plan_to_price_id(&req.plan).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "unknown plan: {} (no Stripe price ID mapped)",
            req.plan
        ))
    })?;

    let client = reqwest::Client::new();
    let mut form = vec![
        ("mode".to_string(), "subscription".to_string()),
        ("line_items[0][price]".to_string(), price_id),
        ("line_items[0][quantity]".to_string(), "1".to_string()),
        ("success_url".to_string(), req.success_url),
        ("cancel_url".to_string(), req.cancel_url),
    ];
    if let Ok(stripe_customer_id) = std::env::var("STRIPE_CUSTOMER_ID") {
        form.push(("customer".to_string(), stripe_customer_id));
    }

    let resp = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(api_key, Some(""))
        .form(&form)
        .send()
        .await
        .map_err(|e| ApiError::ServiceUnavailable(format!("Stripe request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %body, "Stripe checkout session creation failed");
        return Err(ApiError::ServiceUnavailable(format!(
            "Stripe returned {status}"
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ApiError::ServiceUnavailable(format!("invalid Stripe response: {e}")))?;
    let url = json
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::ServiceUnavailable("Stripe response missing url".to_string()))?
        .to_string();

    Ok(Json(CheckoutResponse { checkout_url: url }))
}

/// Request body for `POST /api/billing/portal`.
#[derive(Debug, Deserialize)]
pub struct PortalRequest {
    pub return_url: String,
}

/// Response body for the billing portal.
#[derive(Debug, Serialize)]
pub struct PortalResponse {
    pub portal_url: String,
}

/// Create a Stripe Customer Portal session. Returns 503 if Stripe is not configured.
pub async fn stripe_portal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PortalRequest>,
) -> Result<Json<PortalResponse>, ApiError> {
    let _claims = require_customer_token(&headers)?;
    let api_key = state
        .stripe_api_key
        .as_deref()
        .ok_or_else(|| ApiError::ServiceUnavailable("Stripe is not configured".to_string()))?;

    let customer_id = std::env::var("STRIPE_CUSTOMER_ID").map_err(|_| {
        ApiError::BadRequest(
            "STRIPE_CUSTOMER_ID env var is required for billing portal".to_string(),
        )
    })?;

    let client = reqwest::Client::new();
    let form = vec![
        ("customer".to_string(), customer_id),
        ("return_url".to_string(), req.return_url),
    ];

    let resp = client
        .post("https://api.stripe.com/v1/billing_portal/sessions")
        .basic_auth(api_key, Some(""))
        .form(&form)
        .send()
        .await
        .map_err(|e| ApiError::ServiceUnavailable(format!("Stripe request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %body, "Stripe portal session creation failed");
        return Err(ApiError::ServiceUnavailable(format!(
            "Stripe returned {status}"
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ApiError::ServiceUnavailable(format!("invalid Stripe response: {e}")))?;
    let url = json
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::ServiceUnavailable("Stripe response missing url".to_string()))?
        .to_string();

    Ok(Json(PortalResponse { portal_url: url }))
}

/// Verify a Stripe webhook signature using the Stripe-Signature header.
///
/// Stripe's signature format: `t=<timestamp>,v1=<signature>`. The signature
/// is an HMAC-SHA256 of `<timestamp>.<payload>` using the webhook signing
/// secret.
fn verify_stripe_signature(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
) -> Result<(), ApiError> {
    let mut timestamp: Option<&str> = None;
    let mut signatures: Vec<&str> = Vec::new();
    for part in signature_header.split(',') {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "t" => timestamp = Some(value),
                "v1" => signatures.push(value),
                _ => {}
            }
        }
    }

    let ts = timestamp
        .ok_or_else(|| ApiError::BadRequest("Stripe-Signature missing timestamp".to_string()))?;

    if signatures.is_empty() {
        return Err(ApiError::BadRequest(
            "Stripe-Signature missing v1 signature".to_string(),
        ));
    }

    let signed_payload = format!("{ts}.{}", String::from_utf8_lossy(payload));
    let type_hmac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::BadRequest("invalid webhook secret".to_string()))?;
    use hmac::Mac;
    let mut mac = type_hmac;
    mac.update(signed_payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if signatures.iter().any(|s| *s == expected) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

/// Stripe webhook handler. Verifies the signature, processes events, and
/// logs all events to the audit log.
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let secret = state.stripe_webhook_secret.as_deref().ok_or_else(|| {
        ApiError::ServiceUnavailable("Stripe webhook secret is not configured".to_string())
    })?;

    let sig_header = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("missing Stripe-Signature header".to_string()))?;

    verify_stripe_signature(&body, sig_header, secret)?;

    let event: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON body: {e}")))?;

    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::info!(event_type = %event_type, "received Stripe webhook");

    // Log all webhook events to the audit log.
    db::insert_audit(
        &state.pool,
        "stripe.webhook",
        None,
        &format!("Stripe webhook: {event_type}"),
        event.clone(),
    )
    .await
    .ok();

    match event_type {
        "checkout.session.completed" => {
            handle_checkout_completed(&state, &event).await?;
        }
        "customer.subscription.updated" => {
            handle_subscription_updated(&state, &event).await?;
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(&state, &event).await?;
        }
        "invoice.payment_failed" => {
            handle_payment_failed(&state, &event).await?;
        }
        _ => {
            tracing::debug!(event_type = %event_type, "unhandled Stripe event type");
        }
    }

    Ok(Json(serde_json::json!({ "received": true })))
}

/// On checkout.session.completed: create a license for the customer.
async fn handle_checkout_completed(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), ApiError> {
    let data = event
        .get("data")
        .and_then(|d| d.get("object"))
        .ok_or_else(|| ApiError::BadRequest("malformed checkout event".to_string()))?;

    let customer_stripe_id = data
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let plan = data
        .get("metadata")
        .and_then(|m| m.get("plan"))
        .and_then(|v| v.as_str())
        .unwrap_or("starter");

    let (seats, features) = plan_defaults(plan);
    let now = Utc::now();
    let expires_at = now + chrono::Duration::days(365);

    let license_id = format!("lic_{}", random_id(12));
    let claims = LicenseClaims {
        license_id: license_id.clone(),
        customer: customer_stripe_id.to_string(),
        plan: plan.to_string(),
        seats,
        instance_id: String::new(),
        issued_at: now,
        expires_at,
        features: features.clone(),
    };

    let file = state.signer.sign_license(&claims)?;

    let row = LicenseRow {
        id: Uuid::new_v4(),
        customer_id: customer_stripe_id.to_string(),
        license_id: license_id.clone(),
        plan: plan.to_string(),
        seats: seats as i32,
        instance_id: None,
        issued_at: now,
        expires_at,
        features: serde_json::to_value(&features).unwrap_or(serde_json::json!([])),
        status: "active".to_string(),
        signature: file.signature.clone(),
    };

    db::insert_license(&state.pool, &row).await?;

    db::insert_audit(
        &state.pool,
        "license.issued",
        None,
        &format!("license {license_id} created from Stripe checkout for {customer_stripe_id}"),
        serde_json::json!({ "license_id": license_id, "plan": plan, "seats": seats }),
    )
    .await
    .ok();

    Ok(())
}

/// On subscription.updated: update the license (seats, plan).
async fn handle_subscription_updated(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), ApiError> {
    let data = event
        .get("data")
        .and_then(|d| d.get("object"))
        .ok_or_else(|| ApiError::BadRequest("malformed subscription event".to_string()))?;

    let customer_stripe_id = data
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Find the customer's active license and update it.
    let licenses = db::list_licenses(&state.pool, Some(customer_stripe_id)).await?;
    if let Some(lic) = licenses.into_iter().find(|l| l.status == "active") {
        let plan = data
            .get("metadata")
            .and_then(|m| m.get("plan"))
            .and_then(|v| v.as_str())
            .unwrap_or(&lic.plan);
        let (new_seats, _) = plan_defaults(plan);
        // Update seats via a direct SQL update (re-signing is out of scope here).
        sqlx::query("UPDATE licenses SET seats = $2 WHERE license_id = $1;")
            .bind(&lic.license_id)
            .bind(new_seats as i32)
            .execute(&state.pool)
            .await
            .map_err(DbError::from)?;
    }

    db::insert_audit(
        &state.pool,
        "license.updated",
        None,
        &format!("subscription updated for customer {customer_stripe_id}"),
        event.clone(),
    )
    .await
    .ok();

    Ok(())
}

/// On subscription.deleted: revoke the license.
async fn handle_subscription_deleted(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), ApiError> {
    let data = event
        .get("data")
        .and_then(|d| d.get("object"))
        .ok_or_else(|| ApiError::BadRequest("malformed subscription event".to_string()))?;

    let customer_stripe_id = data
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let licenses = db::list_licenses(&state.pool, Some(customer_stripe_id)).await?;
    for lic in licenses.into_iter().filter(|l| l.status == "active") {
        db::set_license_status(&state.pool, &lic.license_id, "revoked")
            .await
            .ok();
    }

    db::insert_audit(
        &state.pool,
        "license.revoked",
        None,
        &format!("subscription deleted for customer {customer_stripe_id}"),
        event.clone(),
    )
    .await
    .ok();

    Ok(())
}

/// On invoice.payment_failed: suspend the license.
async fn handle_payment_failed(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), ApiError> {
    let data = event
        .get("data")
        .and_then(|d| d.get("object"))
        .ok_or_else(|| ApiError::BadRequest("malformed invoice event".to_string()))?;

    let customer_stripe_id = data
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let licenses = db::list_licenses(&state.pool, Some(customer_stripe_id)).await?;
    for lic in licenses.into_iter().filter(|l| l.status == "active") {
        db::set_license_status(&state.pool, &lic.license_id, "suspended")
            .await
            .ok();
    }

    db::insert_audit(
        &state.pool,
        "license.suspended",
        None,
        &format!("payment failed for customer {customer_stripe_id}"),
        event.clone(),
    )
    .await
    .ok();

    Ok(())
}

/// Customer billing summary (from local data when Stripe is unavailable).
#[derive(Debug, Serialize)]
pub struct BillingSummary {
    pub stripe_configured: bool,
    pub invoices: Vec<serde_json::Value>,
}

/// `GET /api/customer/billing` — list billing info for the customer.
pub async fn customer_billing(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BillingSummary>, ApiError> {
    let _claims = require_customer_token(&headers)?;
    let stripe_configured = state.stripe_api_key.is_some();
    // Without Stripe, we return an empty invoice list.
    Ok(Json(BillingSummary {
        stripe_configured,
        invoices: Vec::new(),
    }))
}

/// Return default seat count and features for a plan name.
fn plan_defaults(plan: &str) -> (u32, Vec<String>) {
    match plan {
        "starter" => (
            10,
            vec!["auth".to_string(), "rbac".to_string(), "audit".to_string()],
        ),
        "pro" => (
            50,
            vec![
                "auth".to_string(),
                "rbac".to_string(),
                "audit".to_string(),
                "oidc".to_string(),
                "mfa".to_string(),
            ],
        ),
        "enterprise" => (
            1000,
            vec![
                "auth".to_string(),
                "rbac".to_string(),
                "audit".to_string(),
                "oidc".to_string(),
                "mfa".to_string(),
                "ldap".to_string(),
                "multi_instance".to_string(),
            ],
        ),
        _ => (
            5,
            vec!["auth".to_string(), "rbac".to_string(), "audit".to_string()],
        ),
    }
}

/// Monthly price (in cents) for MRR calculation when Stripe is unavailable.
fn plan_monthly_price_cents(plan: &str) -> i64 {
    match plan {
        "starter" => 4900,
        "pro" => 19900,
        "enterprise" => 49900,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Admin auth (Phase 12d.1)
// ---------------------------------------------------------------------------

/// Request body for `POST /api/admin/login`.
#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub email: String,
    pub password: String,
}

/// Admin auth response.
#[derive(Debug, Serialize)]
pub struct AdminAuthResponse {
    pub token: String,
    pub admin_id: Uuid,
    pub role: String,
}

/// Admin login — verify credentials and issue an admin JWT.
pub async fn admin_login(
    State(state): State<AppState>,
    Json(req): Json<AdminLoginRequest>,
) -> Result<Json<AdminAuthResponse>, ApiError> {
    let admin = db::get_admin_by_email(&state.pool, &req.email).await?;
    auth::verify_password(&req.password, &admin.password_hash)
        .map_err(|_| ApiError::Unauthorized)?;
    let token = auth::issue_admin_token(admin.id, &admin.role)?;
    Ok(Json(AdminAuthResponse {
        token,
        admin_id: admin.id,
        role: admin.role,
    }))
}

// ---------------------------------------------------------------------------
// Admin customer management (Phase 12d.2)
// ---------------------------------------------------------------------------

/// Query params for listing customers.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List all customers (with pagination).
pub async fn admin_list_customers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PaginationQuery>,
) -> Result<Json<Vec<db::CustomerRow>>, ApiError> {
    require_admin_token(&headers)?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let customers = db::list_all_customers(&state.pool, limit, offset).await?;
    Ok(Json(customers))
}

/// Customer detail response (combines account + customer info).
#[derive(Debug, Serialize)]
pub struct CustomerDetail {
    pub customer: db::CustomerRow,
    pub account: db::AccountRow,
}

/// Get a customer detail by customer UUID.
pub async fn admin_get_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CustomerDetail>, ApiError> {
    require_admin_token(&headers)?;
    // We need to fetch the customer row by ID — use a direct query.
    let customer: db::CustomerRow = sqlx::query_as::<_, db::CustomerRow>(
        r#"
        SELECT id, account_id, company_name, contact_email, created_at
        FROM customers WHERE id = $1;
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(DbError::from)?
    .ok_or_else(|| ApiError::NotFound(format!("customer {id}")))?;

    let account = db::get_account_by_id(&state.pool, customer.account_id).await?;
    Ok(Json(CustomerDetail { customer, account }))
}

/// Suspend a customer (set account status to 'suspended').
pub async fn admin_suspend_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin_token(&headers)?;
    // `id` is the customer UUID; look up the account.
    let customer: db::CustomerRow = sqlx::query_as::<_, db::CustomerRow>(
        "SELECT id, account_id, company_name, contact_email, created_at FROM customers WHERE id = $1;",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(DbError::from)?
    .ok_or_else(|| ApiError::NotFound(format!("customer {id}")))?;

    db::set_account_status(&state.pool, customer.account_id, "suspended").await?;
    db::insert_audit(
        &state.pool,
        "customer.suspended",
        Some(customer.account_id),
        &format!("customer {id} suspended"),
        serde_json::json!({ "customer_id": id }),
    )
    .await
    .ok();
    Ok(Json(serde_json::json!({ "success": true })))
}

/// Activate a customer (set account status to 'active').
pub async fn admin_activate_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin_token(&headers)?;
    let customer: db::CustomerRow = sqlx::query_as::<_, db::CustomerRow>(
        "SELECT id, account_id, company_name, contact_email, created_at FROM customers WHERE id = $1;",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(DbError::from)?
    .ok_or_else(|| ApiError::NotFound(format!("customer {id}")))?;

    db::set_account_status(&state.pool, customer.account_id, "active").await?;
    db::insert_audit(
        &state.pool,
        "customer.activated",
        Some(customer.account_id),
        &format!("customer {id} activated"),
        serde_json::json!({ "customer_id": id }),
    )
    .await
    .ok();
    Ok(Json(serde_json::json!({ "success": true })))
}

// ---------------------------------------------------------------------------
// Admin license management (Phase 12d.3)
// ---------------------------------------------------------------------------

/// Request body for `POST /api/admin/licenses`.
#[derive(Debug, Deserialize)]
pub struct AdminCreateLicenseRequest {
    pub customer_id: String,
    pub customer_name: Option<String>,
    pub plan: String,
    pub seats: u32,
    pub expires_at: DateTime<Utc>,
    pub features: Vec<String>,
    pub instance_id: Option<String>,
}

/// Create a license for a customer (admin JWT auth).
pub async fn admin_create_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminCreateLicenseRequest>,
) -> Result<Json<LicenseFile>, ApiError> {
    require_admin_token(&headers)?;
    if req.seats == 0 {
        return Err(ApiError::BadRequest("seats must be > 0".to_string()));
    }
    if req.customer_id.is_empty() {
        return Err(ApiError::BadRequest("customer_id is required".to_string()));
    }

    let license_id = format!("lic_{}", random_id(12));
    let customer = req.customer_name.unwrap_or_else(|| req.customer_id.clone());
    let now = Utc::now();

    let claims = LicenseClaims {
        license_id: license_id.clone(),
        customer,
        plan: req.plan.clone(),
        seats: req.seats,
        instance_id: req.instance_id.clone().unwrap_or_default(),
        issued_at: now,
        expires_at: req.expires_at,
        features: req.features.clone(),
    };

    let file = state.signer.sign_license(&claims)?;

    let row = LicenseRow {
        id: Uuid::new_v4(),
        customer_id: req.customer_id.clone(),
        license_id: license_id.clone(),
        plan: req.plan,
        seats: req.seats as i32,
        instance_id: req.instance_id,
        issued_at: now,
        expires_at: req.expires_at,
        features: serde_json::to_value(&req.features).unwrap_or(serde_json::json!([])),
        status: "active".to_string(),
        signature: file.signature.clone(),
    };

    db::insert_license(&state.pool, &row).await?;

    db::insert_audit(
        &state.pool,
        "license.issued",
        None,
        &format!(
            "admin issued license {license_id} for customer {}",
            req.customer_id
        ),
        serde_json::json!({ "license_id": license_id, "plan": row.plan, "seats": row.seats }),
    )
    .await
    .ok();

    Ok(Json(file))
}

/// Revoke a license (admin JWT auth).
pub async fn admin_revoke_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<LicenseRow>, ApiError> {
    require_admin_token(&headers)?;
    let row = db::revoke_license(&state.pool, &id).await?;
    db::insert_audit(
        &state.pool,
        "license.revoked",
        None,
        &format!("admin revoked license {id}"),
        serde_json::json!({ "license_id": id }),
    )
    .await
    .ok();
    Ok(Json(row))
}

/// Request body for `POST /api/admin/licenses/:id/extend`.
#[derive(Debug, Deserialize)]
pub struct ExtendLicenseRequest {
    pub expires_at: DateTime<Utc>,
}

/// Extend a license's expiry (admin JWT auth).
pub async fn admin_extend_license(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ExtendLicenseRequest>,
) -> Result<Json<LicenseRow>, ApiError> {
    require_admin_token(&headers)?;
    let row = db::extend_license(&state.pool, &id, req.expires_at).await?;
    db::insert_audit(
        &state.pool,
        "license.extended",
        None,
        &format!("admin extended license {id} to {}", req.expires_at),
        serde_json::json!({ "license_id": id, "expires_at": req.expires_at }),
    )
    .await
    .ok();
    Ok(Json(row))
}

// ---------------------------------------------------------------------------
// Admin dashboard (Phase 12d.4)
// ---------------------------------------------------------------------------

/// Dashboard metrics response.
#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub total_customers: i64,
    pub active_licenses: i64,
    pub total_seats: i64,
    /// Monthly recurring revenue in cents (USD). Calculated from the licenses
    /// table when Stripe is not configured.
    pub mrr_cents: i64,
    /// Churn rate as a percentage (0-100). Simplified: 0 when no data.
    pub churn_rate: f64,
    pub stripe_configured: bool,
}

/// Revenue dashboard — returns aggregate metrics.
pub async fn admin_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DashboardResponse>, ApiError> {
    require_admin_token(&headers)?;

    let total_customers = db::count_customers(&state.pool).await?;
    let active_licenses = db::count_active_licenses(&state.pool).await?;
    let total_seats = db::sum_active_seats(&state.pool).await?;

    // Calculate MRR from active licenses (sum of plan prices).
    let licenses = db::list_licenses(&state.pool, None).await?;
    let mrr_cents: i64 = licenses
        .iter()
        .filter(|l| l.status == "active")
        .map(|l| plan_monthly_price_cents(&l.plan))
        .sum();

    // Simplified churn rate: not enough data for a real calculation.
    let churn_rate = 0.0;

    Ok(Json(DashboardResponse {
        total_customers,
        active_licenses,
        total_seats,
        mrr_cents,
        churn_rate,
        stripe_configured: state.stripe_api_key.is_some(),
    }))
}

// ---------------------------------------------------------------------------
// Auth helpers
// ---------------------------------------------------------------------------

/// Require a valid customer JWT. Returns the claims.
fn require_customer_token(headers: &HeaderMap) -> Result<auth::Claims, ApiError> {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok());
    let claims = auth::extract_bearer(auth_header)?;
    if claims.kind != TokenKind::Customer {
        return Err(ApiError::Forbidden);
    }
    Ok(claims)
}

/// Require a valid admin JWT. Returns the claims.
fn require_admin_token(headers: &HeaderMap) -> Result<auth::Claims, ApiError> {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok());
    let claims = auth::extract_bearer(auth_header)?;
    if claims.kind != TokenKind::Admin {
        return Err(ApiError::Forbidden);
    }
    Ok(claims)
}

/// Parse a UUID from a string, returning a BadRequest error on failure.
fn parse_uuid(s: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(s).map_err(|e| ApiError::BadRequest(format!("invalid UUID: {e}")))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a random alphanumeric ID (lowercase a-z, 0-9).
fn random_id(len: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Helper to convert a HashMap of query params (used in tests).
#[allow(dead_code)]
pub fn query_to_map(q: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in q.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

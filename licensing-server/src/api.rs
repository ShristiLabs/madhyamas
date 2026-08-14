//! Axum API handlers for the licensing server.
//!
//! Exposes license issuance, retrieval, revocation, verification, and seat
//! tracking endpoints under `/api`. All mutating endpoints (and license
//! retrieval) require an `X-Admin-Key` header matching the configured admin
//! key. The verification and seat endpoints are authenticated the same way for
//! now; the customer portal auth (Phase 12b) and admin portal auth (Phase 12d)
//! will replace this with proper JWT/session auth.

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
}

/// API error type — maps domain errors to HTTP status codes.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("database error: {0}")]
    Db(#[from] DbError),
    #[error("signing error: {0}")]
    Sign(#[from] SignError),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    #[allow(dead_code)]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("unauthorized")]
    Unauthorized,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Db(DbError::LicenseNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("license not found: {id}"))
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
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
        };
        let body = Json(serde_json::json!({ "error": message }));
        (status, body).into_response()
    }
}

/// Build the API router with all endpoints.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/licenses", post(create_license).get(list_licenses))
        .route("/api/licenses/verify", post(verify_license))
        .route("/api/licenses/{id}", get(get_license))
        .route("/api/licenses/{id}/revoke", post(revoke_license))
        .route("/api/seats/register", post(register_seat))
        .route("/api/seats/heartbeat", post(heartbeat_seat))
        .route("/api/seats/deregister", post(deregister_seat))
        .route("/api/seats/{license_id}", get(list_seats))
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

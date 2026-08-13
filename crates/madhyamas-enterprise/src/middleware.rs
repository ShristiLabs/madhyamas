//! Axum middleware for enterprise authentication and authorization enforcement.
//!
//! This module provides:
//! - [`auth_middleware`]: a tower/Axum middleware that validates the
//!   `Authorization: Bearer <token>` header using [`AuthManager::validate_jwt`]
//!   and injects the resulting [`JwtClaims`] into request extensions. Requests
//!   to public paths bypass authentication.
//! - [`AuthUser`]: an extractor that pulls the authenticated [`JwtClaims`] out
//!   of request extensions (set by [`auth_middleware`]).
//! - [`PermissionState`] / [`require_permission_middleware`]: a middleware
//!   pair that checks the authenticated user's role has a required
//!   [`Permission`] via [`RbacManager`]. Apply with
//!   `axum::middleware::from_fn_with_state`.
//!
//! Auth is only enforced when enterprise features are enabled **and** an
//! [`AuthManager`] is provided.
//!
//! # Applying the middleware
//!
//! Because `auth_middleware` and `require_permission_middleware` are `async fn`
//! items (whose coroutine return types are not nameable), the idiomatic Axum
//! pattern is to apply them inline with `from_fn_with_state` rather than via a
//! wrapper function returning `impl Layer`:
//!
//! ```ignore
//! use axum::middleware::from_fn_with_state;
//! use madhyamas_enterprise::middleware::{auth_middleware, require_permission_middleware, PermissionState};
//! use madhyamas_enterprise::{AuthManager, Permission, ResourceType, RbacManager};
//! use std::sync::Arc;
//!
//! let auth: Arc<AuthManager> = /* ... */;
//! router.layer(from_fn_with_state(auth, auth_middleware));
//!
//! let perm_state = PermissionState {
//!     rbac: Arc::new(RbacManager::new()),
//!     resource_type: ResourceType::Config,
//!     permission: Permission::Read,
//! };
//! router.route_layer(from_fn_with_state(perm_state, require_permission_middleware));
//! ```

use axum::{
    extract::{Request, State},
    http::{header, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use madhyamas_api::AppState;
use std::sync::Arc;

use crate::{AuthManager, JwtClaims, Permission, RbacManager, ResourceType, UserRole};

/// Paths that never require authentication. These are matched against the
/// full request path (including the `/api` prefix used by nested routes).
const PUBLIC_PATHS: &[&str] = &[
    "/health",
    "/api/health",
    "/api/health/detailed",
    "/api/auth/login",
];

/// Returns true if the request path is exempt from authentication.
///
/// Static web assets (served via the router fallback) are also considered
/// public — they have no `/api` prefix and are not API endpoints.
fn is_public_path(uri: &Uri) -> bool {
    let path = uri.path();
    if PUBLIC_PATHS.contains(&path) {
        return true;
    }
    // Anything outside the `/api` namespace is a static asset / web UI route
    // and does not require a JWT.
    !path.starts_with("/api/")
}

/// Build a `401 Unauthorized` JSON response.
fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "unauthorized",
            "message": message,
        })),
    )
        .into_response()
}

/// Build a `403 Forbidden` JSON response.
fn forbidden(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "error": "forbidden",
            "message": message,
        })),
    )
        .into_response()
}

/// Axum middleware that enforces JWT authentication.
///
/// Extracts the `Authorization: Bearer <token>` header, validates it via
/// [`AuthManager::validate_jwt`], and on success inserts the [`JwtClaims`]
/// into the request extensions so downstream handlers and extractors (e.g.
/// [`AuthUser`], [`require_permission_middleware`]) can access the
/// authenticated user.
///
/// Public paths (see [`PUBLIC_PATHS`]) and non-`/api` static assets bypass
/// this check entirely.
///
/// Apply with `axum::middleware::from_fn_with_state(auth_manager, auth_middleware)`.
pub async fn auth_middleware(
    State(state): State<Arc<AuthManager>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Public routes and static assets skip authentication.
    if is_public_path(request.uri()) {
        return next.run(request).await;
    }

    // Extract the bearer token, producing an owned value so the immutable
    // borrow of `request.headers()` is released before we mutate extensions.
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|t| t.to_string());

    let token = match token {
        Some(t) => t,
        None => return unauthorized("Missing or invalid Authorization header"),
    };

    match state.validate_jwt(&token) {
        Ok(claims) => {
            // Inject the validated claims for downstream handlers/extractors.
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(err) => unauthorized(&err.to_string()),
    }
}

/// Extractor that returns the [`JwtClaims`] injected by [`auth_middleware`].
///
/// Returns a `401` response if no authenticated claims are present (e.g. the
/// auth middleware did not run or rejected the request).
#[derive(Debug, Clone)]
pub struct AuthUser(pub JwtClaims);

impl axum::extract::FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<JwtClaims>()
            .cloned()
            .map(AuthUser)
            .ok_or_else(|| unauthorized("Authentication required"))
    }
}

/// Parse a [`UserRole`] from the `role` string embedded in JWT claims.
///
/// Unknown roles fall back to [`UserRole::ReadOnly`] (least privilege).
fn role_from_claims(claims: &JwtClaims) -> UserRole {
    UserRole::from_label(&claims.role)
}

/// Middleware state for permission checks via [`require_permission_middleware`].
#[derive(Clone)]
pub struct PermissionState {
    /// RBAC manager used to evaluate the user's permissions.
    pub rbac: Arc<RbacManager>,
    /// Resource type the permission applies to.
    pub resource_type: ResourceType,
    /// Required permission.
    pub permission: Permission,
}

/// Middleware that checks the authenticated user's role has the required
/// permission; otherwise returns `403 Forbidden`.
///
/// Expects [`auth_middleware`] to have run first and injected [`JwtClaims`]
/// into the request extensions. If no claims are present the request is
/// rejected with `401`.
///
/// Apply with `axum::middleware::from_fn_with_state(state, require_permission_middleware)`.
pub async fn require_permission_middleware(
    State(state): State<PermissionState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(claims) = request.extensions().get::<JwtClaims>() else {
        return unauthorized("Authentication required");
    };

    let role = role_from_claims(claims);
    if state
        .rbac
        .has_permission(&role, state.resource_type, state.permission)
    {
        next.run(request).await
    } else {
        forbidden("Insufficient permissions")
    }
}

/// Build a [`PermissionState`] suitable for use with
/// [`require_permission_middleware`] via `from_fn_with_state`.
///
/// This is the ergonomic equivalent of `require_permission(permission)`: it
/// constructs a fresh [`RbacManager`] with the default role/permission matrix.
/// To reuse an existing RBAC manager, construct [`PermissionState`] directly.
///
/// # Example
/// ```ignore
/// use axum::middleware::from_fn_with_state;
/// use madhyamas_enterprise::middleware::{require_permission, require_permission_middleware};
/// use madhyamas_enterprise::{Permission, ResourceType};
///
/// router.route_layer(from_fn_with_state(
///     require_permission(ResourceType::Config, Permission::Read),
///     require_permission_middleware,
/// ));
/// ```
pub fn require_permission(resource_type: ResourceType, permission: Permission) -> PermissionState {
    PermissionState {
        rbac: Arc::new(RbacManager::new()),
        resource_type,
        permission,
    }
}

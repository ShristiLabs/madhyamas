//! Axum middleware for enterprise authentication and authorization enforcement.
//!
//! This module provides:
//! - [`auth_middleware`]: a tower/Axum middleware that validates the
//!   `Authorization: Bearer <token>` header OR the `X-API-Key` header OR the
//!   `?api_key=` query parameter, and injects an [`AuthUser`] into request
//!   extensions. Requests to public paths bypass authentication.
//! - [`AuthUser`]: an extractor that pulls the authenticated identity (JWT
//!   claims or API key scopes) out of request extensions (set by
//!   [`auth_middleware`]).
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
    extract::{Query, Request, State},
    http::{header, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use madhyamas_api::AppState;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    AuthManager, EnterpriseStore, JwtClaims, Permission, RbacManager, ResourceType, Scope, UserRole,
};

/// Paths that never require authentication. These are matched against the
/// full request path (including the `/api` prefix used by nested routes).
const PUBLIC_PATHS: &[&str] = &[
    "/health",
    "/api/health",
    "/api/health/detailed",
    "/api/auth/login",
    "/api/auth/refresh",
    "/api/license",
];

/// Returns true if the request path is exempt from authentication.
///
/// This function handles both the full path (e.g. `/api/auth/login`) and
/// the nested path (e.g. `/auth/login`) because axum's `.nest("/api", ...)`
/// strips the `/api` prefix before the nested router processes the request.
fn is_public_path(uri: &Uri) -> bool {
    let path = uri.path();
    // Check exact matches for both with and without /api prefix.
    if PUBLIC_PATHS.contains(&path) {
        return true;
    }
    // Also check without the /api prefix (for nested router context).
    let stripped = path.strip_prefix("/api/").unwrap_or(path);
    if PUBLIC_PATHS.contains(&stripped) {
        return true;
    }
    // In the nested router context, all paths start with `/` (the /api
    // prefix has been stripped). Auth routes like `/auth/login` are public.
    // Non-API paths (static assets) don't start with `/` in the nested
    // context, but in the top-level router they don't start with `/api/`.
    // Since the enterprise router is nested under /api, all its paths
    // start with `/` after stripping. We consider a path "public" if it
    // matches a known public path pattern.
    matches!(
        stripped,
        "/health" | "/health/detailed" | "/auth/login" | "/auth/refresh" | "/license"
    )
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

/// Query params extracted for `?api_key=` support.
#[derive(Debug, Deserialize)]
struct ApiKeyQuery {
    api_key: Option<String>,
}

/// Determine the required scope for a given HTTP method + path.
///
/// Returns `None` for routes that don't map to a scope (e.g. auth routes,
/// onboarding). The scope format is `<resource>:<permission>` where
/// `permission` is derived from the HTTP method: GET → `read`,
/// POST/PUT/PATCH → `write`, DELETE → `delete`.
pub fn required_scope(method: &axum::http::Method, path: &str) -> Option<Scope> {
    // Strip /api prefix if present (handles both nested and non-nested paths).
    let path = path.strip_prefix("/api/").unwrap_or(path);
    // Auth/onboarding/license routes are not scope-gated.
    if path.starts_with("/auth/")
        || path.starts_with("/onboarding")
        || path == "/license"
        || path == "/health"
        || path == "/health/detailed"
        || path == "/metrics"
        || path == "/performance"
    {
        return None;
    }
    let permission = match *method {
        axum::http::Method::GET => "read",
        axum::http::Method::POST | axum::http::Method::PUT | axum::http::Method::PATCH => "write",
        axum::http::Method::DELETE => "delete",
        _ => "read",
    };
    let resource = if path.starts_with("/traffic") || path.starts_with("/sessions") {
        "traffic"
    } else if path.starts_with("/mocks") {
        "mocks"
    } else if path.starts_with("/rewrites") {
        "rewrites"
    } else if path.starts_with("/breakpoints") {
        "breakpoints"
    } else if path.starts_with("/throttle") {
        "throttle"
    } else if path.starts_with("/blocklist") {
        "blocklist"
    } else if path.starts_with("/focus") {
        "focus"
    } else if path.starts_with("/scripts") {
        "scripts"
    } else if path.starts_with("/plugins") {
        "plugins"
    } else if path.starts_with("/config") {
        "config"
    } else if path.starts_with("/users") {
        "users"
    } else if path.starts_with("/audit") {
        "audit"
    } else if path.starts_with("/rbac") {
        "rbac"
    } else {
        return None;
    };
    Some(Scope::parse(&format!("{resource}:{permission}")))
}

/// Check whether any of the granted scopes satisfies the required scope.
fn scope_authorized(required: &Scope, granted: &[String]) -> bool {
    granted.iter().any(|g| {
        let parsed = Scope::parse(g);
        Scope::matches(required, &parsed)
    })
}

/// Axum middleware that enforces authentication via JWT or API key.
///
/// Authentication is attempted in this order:
/// 1. `X-API-Key` header → [`AuthManager::validate_api_key`]
/// 2. `?api_key=` query parameter → [`AuthManager::validate_api_key`]
/// 3. `Authorization: Bearer <token>` header → [`AuthManager::validate_jwt`]
///
/// On success, an [`AuthUser`] is inserted into request extensions. For API
/// key auth, the granted scopes are checked against the route's required
/// scope (see [`required_scope`]); a mismatch yields `403 Forbidden`.
///
/// Public paths (see [`PUBLIC_PATHS`]) bypass this check entirely.
///
/// Apply with `axum::middleware::from_fn(auth_middleware)` and ensure
/// `Extension<Arc<AuthManager>>`, `Extension<Arc<dyn EnterpriseStore>>`,
/// and `Extension<Arc<AuditLogger>>` are added as outer extension layers.
pub async fn auth_middleware(
    Extension(state): Extension<Arc<AuthManager>>,
    Extension(store): Extension<Arc<dyn EnterpriseStore>>,
    Extension(audit): Extension<Arc<crate::AuditLogger>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Public routes skip authentication.
    if is_public_path(request.uri()) {
        return next.run(request).await;
    }

    // When strict auth is not required, let requests through. This allows
    // bootstrap (e.g. creating the first admin user) before any credentials
    // exist.
    if !state.require_auth() {
        return next.run(request).await;
    }

    tracing::debug!(
        "auth_middleware: path={}, require_auth=true",
        request.uri().path()
    );

    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 1. Try X-API-Key header.
    let api_key_header = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 2. Try ?api_key= query param.
    let api_key_query = if api_key_header.is_none() {
        Query::<ApiKeyQuery>::try_from_uri(request.uri())
            .ok()
            .and_then(|q| q.api_key.clone())
    } else {
        None
    };

    if let Some(key) = api_key_header.or(api_key_query) {
        match state.validate_api_key(&key).await {
            Ok(api_key_auth) => {
                // Scope enforcement for API key auth.
                if let Some(ref required) = required_scope(&method, &path) {
                    if !scope_authorized(required, &api_key_auth.scopes) {
                        return forbidden("Insufficient API key scope");
                    }
                }
                let auth_user = AuthUser {
                    claims: None,
                    scopes: Some(api_key_auth.scopes.clone()),
                    user_id: api_key_auth.user_id.clone(),
                    role: "user".to_string(),
                    key_id: Some(api_key_auth.key_id.clone()),
                    session_id: None,
                };
                audit.log(
                    crate::AuditEvent::new(crate::AuditEventType::Login, "API key authenticated")
                        .with_user(api_key_auth.user_id.clone())
                        .with_api_key(api_key_auth.key_id.clone()),
                );
                request.extensions_mut().insert(auth_user);
                return next.run(request).await;
            }
            Err(err) => return unauthorized(&err.to_string()),
        }
    }

    // 3. Try Authorization: Bearer <token>.
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
            // Session idle timeout: if the JWT carries a session ID, check
            // the session's last_activity in the store.
            if let Some(ref sid) = claims.sid {
                match store.get_session(sid).await {
                    Ok(Some(session)) => {
                        if session.revoked {
                            return unauthorized("Session revoked");
                        }
                        if let Ok(parsed) =
                            chrono::DateTime::parse_from_rfc3339(&session.last_activity)
                        {
                            let last = parsed.with_timezone(&chrono::Utc);
                            let idle_secs =
                                chrono::Utc::now().signed_duration_since(last).num_seconds();
                            if idle_secs > state.session_idle_timeout_secs() as i64 {
                                let _ = store.revoke_session(sid).await;
                                return unauthorized("Session idle timeout exceeded");
                            }
                        }
                        let _ = store.update_session_activity(sid).await;
                    }
                    Ok(None) => {
                        return unauthorized("Session not found");
                    }
                    Err(_) => return unauthorized("Session lookup failed"),
                }
            }
            let auth_user = AuthUser {
                user_id: claims.sub.clone(),
                role: claims.role.clone(),
                session_id: claims.sid.clone(),
                claims: Some(claims),
                scopes: None,
                key_id: None,
            };
            request.extensions_mut().insert(auth_user);
            next.run(request).await
        }
        Err(err) => unauthorized(&err.to_string()),
    }
}

/// Authenticated user identity injected by [`auth_middleware`].
///
/// When authentication was via JWT, `claims` is `Some` and `scopes` is
/// `None`. When authentication was via API key, `claims` is `None` and
/// `scopes` is `Some`. The `user_id` and `role` fields are always set.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// JWT claims, when authenticated via bearer token.
    pub claims: Option<JwtClaims>,
    /// API key scopes, when authenticated via API key.
    pub scopes: Option<Vec<String>>,
    /// User ID (from JWT `sub` or API key owner).
    pub user_id: String,
    /// Role label (from JWT `role` claim, or `"user"` for API keys).
    pub role: String,
    /// API key record ID, when authenticated via API key.
    pub key_id: Option<String>,
    /// Session ID, when authenticated via JWT with a session claim.
    pub session_id: Option<String>,
}

impl axum::extract::FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

/// Parse a [`UserRole`] from the `role` string in [`AuthUser`].
///
/// Unknown roles fall back to [`UserRole::ReadOnly`] (least privilege).
fn role_from_auth_user(auth_user: &AuthUser) -> UserRole {
    UserRole::from_label(&auth_user.role)
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
/// Expects [`auth_middleware`] to have run first and injected an [`AuthUser`]
/// into the request extensions. If no [`AuthUser`] is present the request is
/// rejected with `401`.
///
/// For JWT-authenticated users, the role from the JWT claims is checked
/// against the RBAC matrix. For API-key-authenticated users, scope
/// enforcement has already been applied in [`auth_middleware`], so this
/// middleware allows the request through (the scopes were already validated
/// against the route's required scope).
///
/// Apply with `axum::middleware::from_fn_with_state(state, require_permission_middleware)`.
pub async fn require_permission_middleware(
    State(state): State<PermissionState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(auth_user) = request.extensions().get::<AuthUser>() else {
        return unauthorized("Authentication required");
    };

    // API key auth: scope already enforced in auth_middleware.
    if auth_user.scopes.is_some() {
        return next.run(request).await;
    }

    let role = role_from_auth_user(auth_user);
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

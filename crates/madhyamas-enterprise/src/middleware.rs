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
//! # CSRF (Phase 9.12)
//!
//! Authentication currently uses JWT bearer tokens in the `Authorization`
//! header (or API keys in `X-API-Key`), **not** cookies. Bearer-token auth
//! is inherently immune to CSRF because browsers do not automatically
//! attach the `Authorization` header to cross-origin requests the way they
//! do with cookies. **If cookie-based authentication is added in the
//! future, CSRF tokens MUST be implemented** — the auth middleware below
//! would need to validate a double-submit cookie or a custom `X-CSRF-Token`
//! header on all state-changing requests (POST/PUT/PATCH/DELETE).
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
    // Device enrollment redemption (issue #106): the short-lived
    // enrollment token itself is the credential — the device scanning
    // the QR has no web session to authenticate with.
    "/api/devices/enroll",
    // CA certificate distribution (issue #107): the CA certificate is
    // public by definition — it is the artifact clients (browsers,
    // companion apps following the `madhyamas://connect` QR payload's
    // `ca=` URL) must fetch BEFORE they can trust the proxy. Only the
    // CA *key* is a secret and it never leaves the server.
    "/api/cert/ca",
    // WebSocket traffic stream (issue #107): `/api/ws` authenticates
    // in-handler via `?token=` / `Sec-WebSocket-Protocol` because the
    // upgrade extractor must consume the connection before middleware
    // could reject it (Phase 9 design). Skipping here preserves that
    // flow while the rest of `/api` gains middleware coverage.
    "/api/ws",
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
        "/health"
            | "/health/detailed"
            | "/auth/login"
            | "/auth/refresh"
            | "/license"
            | "/devices/enroll"
            | "/cert/ca"
            | "/ws"
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

/// How a route classifies principals, per the issue #107 feature-scope
/// taxonomy (`docs/CREDENTIAL_ONBOARDING.md` — *Feature scopes*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteAccess {
    /// Any authenticated principal may proceed (JWT or API key); no
    /// feature scope is required. Reserved for the self-identity surface
    /// (`/api/auth/me`, `/api/auth/logout`, `/api/auth/validate`).
    Authenticated,
    /// API keys must hold this scope; JWT principals pass through (their
    /// authorization is decided by RBAC layers / handlers as before).
    Scope(Scope),
    /// JWT web-session principals only. API keys are rejected with `403`
    /// regardless of their scopes — key/device management, user/admin
    /// endpoints, scripts/plugins, traffic deletion, and session switching
    /// stay with the owner's web session (issue #107 decision: exclusions
    /// are JWT-only). This is also the deny-by-default outcome for any
    /// route not present in the map.
    JwtOnly,
}

/// Classify a request against the issue #107 route-to-scope map.
///
/// The map covers every `/api` route in both the OSS surface
/// (`madhyamas-api/src/routes.rs`) and the enterprise router
/// (`middleware`-independent routes included). Anything not matched falls
/// through to [`RouteAccess::JwtOnly`] — deny-by-default for key
/// principals, unchanged pass-through for JWT (whose authorization is
/// enforced by RBAC layers downstream).
///
/// Method handling: reads (`GET`/`HEAD`) map to the feature's `:read`
/// scope; every mutation (`POST`/`PUT`/`PATCH`/`DELETE`) maps to `:write`
/// — rule CRUD *deletion* is part of the write line (only deletion of
/// captured traffic/data is excluded, which is handled by explicit
/// JwtOnly entries below). Asymmetric scopes (`traffic:export` on the HAR
/// round-trip, `replay:execute` on everything replay, `sessions:read` on
/// reads only) are explicit.
pub fn route_access(method: &axum::http::Method, path: &str) -> RouteAccess {
    use axum::http::Method;
    use RouteAccess::{Authenticated, JwtOnly};

    // Strip the /api prefix if present (handles both nested and non-nested
    // paths; base-path deployments strip their prefix at the outer nest).
    // Stripping "/api" — not "/api/" — keeps the leading slash so the
    // "/"-anchored matches below see the same shape for both forms:
    // stripping "/api/" from "/api/traffic" would yield "traffic" and
    // fall through to deny-by-default.
    let path = path.strip_prefix("/api").unwrap_or(path);
    let is_read = matches!(*method, Method::GET | Method::HEAD);

    // Self-identity surface: keys keep exactly these (no escalation
    // possible from them).
    if matches!(path, "/auth/me" | "/auth/logout" | "/auth/validate") {
        return Authenticated;
    }

    // JWT-only surface (issue #107 decision 1). `/devices/enroll` and the
    // auth/login|refresh|license/health|cert/ca/ws paths are public and
    // are skipped by `is_public_path` before this function runs.
    if path.starts_with("/auth/api-keys") // key management
        || path.starts_with("/devices")   // device management
        || path.starts_with("/users")     // user/admin endpoints
        || path.starts_with("/rbac")      // user/admin endpoints
        || path.starts_with("/audit")     // user/admin endpoints
        || path.starts_with("/onboarding")
        || path.starts_with("/scripts")   // code-execution adjacent
        || path.starts_with("/plugins")   // code-execution adjacent
        || path.starts_with("/secrets")
        || path == "/traffic/clear"       // traffic deletion
        || path == "/ws-traffic/clear"    // traffic deletion
        || path == "/grpc/clear"          // traffic deletion
        || (path.starts_with("/sessions") && !is_read)
    // switching/creation/import/deletion
    {
        return JwtOnly;
    }

    // Feature-scope surface (taxonomy order follows
    // docs/CREDENTIAL_ONBOARDING.md). The only non-read mutation on the
    // traffic surface left here is POST /traffic/import/har — the HAR
    // round-trip pairs with export under `traffic:export`.
    let required = if path.starts_with("/traffic")
        || path.starts_with("/ws-traffic")
        || path.starts_with("/grpc")
    {
        if is_read {
            "traffic:read"
        } else {
            "traffic:export"
        }
    } else if path.starts_with("/sessions") {
        // Reads only (list/get/export); mutations returned JwtOnly above.
        "sessions:read"
    } else if path.starts_with("/export/har") || path.starts_with("/export/curl") {
        "traffic:export"
    } else if path.starts_with("/replay") {
        // The whole replay feature — including saved-request management
        // and history — hangs off the single opt-in `replay:execute`
        // scope: replay sends requests upstream (side effects leave the
        // proxy), so the taxonomy deliberately grants nothing weaker.
        "replay:execute"
    } else if path.starts_with("/mocks") {
        if is_read {
            "mocks:read"
        } else {
            "mocks:write"
        }
    } else if path.starts_with("/rewrites") {
        if is_read {
            "rewrites:read"
        } else {
            "rewrites:write"
        }
    } else if path.starts_with("/breakpoints") {
        if is_read {
            "breakpoints:read"
        } else {
            "breakpoints:write"
        }
    } else if path.starts_with("/blocklist") {
        if is_read {
            "blocklist:read"
        } else {
            "blocklist:write"
        }
    } else if path.starts_with("/throttle") {
        if is_read {
            "throttle:read"
        } else {
            "throttle:write"
        }
    } else if path.starts_with("/config")
        || path.starts_with("/autosave")
        || path.starts_with("/capture")
        || path.starts_with("/focus")
        || path.starts_with("/mirror")
        || path.starts_with("/logs")
        || path.starts_with("/persistence")
        || path.starts_with("/metrics")
        || path.starts_with("/performance")
        || path.starts_with("/instances")
    {
        // The config family also carries capture stats/toggle, focus
        // hosts, mirror, log rotation, persistence round-trip and the
        // monitoring surface (metrics/performance/instances) — all
        // "reason about / tune the setup" state. `/config/export` and
        // `/config/import` (enterprise router) land here too, preserving
        // the pre-#107 config:read / config:write mapping they enforced.
        if is_read {
            "config:read"
        } else {
            "config:write"
        }
    } else {
        // Deny-by-default for key principals; JWT pass-through keeps
        // today's behavior (RBAC layers decide downstream).
        return JwtOnly;
    };
    RouteAccess::Scope(Scope::parse(required))
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
/// scope (see [`route_access`]); a mismatch — or a route on the JWT-only
/// exclusion list — yields `403 Forbidden`.
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
                // Feature-scope enforcement for API key auth (issue #107).
                // Legacy grants are expanded to their taxonomy equivalents
                // first (see `effective_scopes`).
                let effective = crate::auth::effective_scopes(&api_key_auth.scopes);
                match route_access(&method, &path) {
                    RouteAccess::Scope(ref required) => {
                        if !scope_authorized(required, &effective) {
                            return forbidden("Insufficient API key scope");
                        }
                    }
                    RouteAccess::Authenticated => {}
                    RouteAccess::JwtOnly => {
                        return forbidden(
                            "API keys are not permitted on this endpoint; \
                             a web-session (JWT) login is required",
                        );
                    }
                }
                let auth_user = AuthUser {
                    claims: None,
                    scopes: Some(effective),
                    user_id: api_key_auth.user_id.clone(),
                    role: "user".to_string(),
                    key_id: Some(api_key_auth.key_id.clone()),
                    session_id: None,
                    device_id: api_key_auth.device_id.clone(),
                };
                // Data-axis enforcement for device-derived agent keys
                // (issue #108): publish the parent-device binding as an
                // api-crate extension so the traffic read handlers can
                // force `device_id = parent` server-side (caller-supplied
                // device params are intersected, never widened). The
                // capability axis is already enforced by `route_access`
                // above for any key principal.
                if let Some(ref device_id) = api_key_auth.device_id {
                    request
                        .extensions_mut()
                        .insert(madhyamas_api::auth::DeviceScope {
                            device_id: device_id.clone(),
                        });
                }
                // Principal snapshot for intercept-rule mutations
                // (issue #109): every key principal carries user + key id
                // (+ parent device for agent keys) so rule CRUD handlers
                // can enforce the device axis and attribute audit events.
                request
                    .extensions_mut()
                    .insert(madhyamas_api::auth::RuleActor {
                        user_id: Some(api_key_auth.user_id.clone()),
                        key_id: Some(api_key_auth.key_id.clone()),
                        device_id: api_key_auth.device_id.clone(),
                    });
                // Audit key logins only on the /auth surface: since issue
                // #107 the middleware covers the whole /api nest, and
                // polling routes (traffic, mocks, ...) would otherwise
                // flood the audit log with per-request Login events.
                // Strip "/api" (not "/api/") so both the nest-stripped
                // ("/auth/me") and full ("/api/auth/me") forms match.
                if path
                    .strip_prefix("/api")
                    .unwrap_or(&path)
                    .starts_with("/auth/")
                {
                    audit.log(
                        crate::AuditEvent::new(
                            crate::AuditEventType::Login,
                            "API key authenticated",
                        )
                        .with_user(api_key_auth.user_id.clone())
                        .with_api_key(api_key_auth.key_id.clone()),
                    );
                } else {
                    tracing::debug!(
                        key_id = %api_key_auth.key_id,
                        "API key authenticated"
                    );
                }
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
                device_id: None,
            };
            // Principal snapshot for intercept-rule mutations
            // (issue #109): JWT principals manage every rule scope.
            request
                .extensions_mut()
                .insert(madhyamas_api::auth::RuleActor {
                    user_id: Some(auth_user.user_id.clone()),
                    key_id: None,
                    device_id: None,
                });
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
/// When the key was a device-derived agent key (`mdy_agent_...`,
/// issue #108), `device_id` carries the parent-device binding (the
/// principal resolves to `(user, device, scopes)` — the two-axis model
/// from docs/CREDENTIAL_ONBOARDING.md).
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
    /// Parent device ID, when the key is a device-derived agent key
    /// (issue #108); `None` for JWT and plain user-key principals.
    pub device_id: Option<String>,
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
/// against the RBAC matrix. API-key-authenticated requests are rejected:
/// every route guarded by this middleware is on the JWT-only exclusion
/// list (user/admin endpoints, audit clear), and the route-scope map in
/// [`auth_middleware`] already denies keys there — this check ensures the
/// middleware can never wave a key principal through on its own
/// (issue #107 closes that pre-existing bypass).
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

    // API key auth: permission-gated routes require a JWT web-session
    // principal (issue #107).
    if auth_user.scopes.is_some() {
        return forbidden("API key authentication is not permitted on this endpoint");
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Uri};

    fn scope(resource: &str, permission: &str) -> RouteAccess {
        RouteAccess::Scope(Scope {
            resource: resource.to_string(),
            permission: permission.to_string(),
        })
    }

    fn access(method: &str, path: &str) -> RouteAccess {
        route_access(&Method::from_bytes(method.as_bytes()).unwrap(), path)
    }

    #[test]
    fn route_access_traffic_family_reads_and_har_roundtrip() {
        // Reads (with and without the /api prefix) map to traffic:read.
        assert_eq!(access("GET", "/api/traffic"), scope("traffic", "read"));
        assert_eq!(access("GET", "/traffic"), scope("traffic", "read"));
        assert_eq!(access("HEAD", "/api/traffic"), scope("traffic", "read"));
        assert_eq!(access("GET", "/api/ws-traffic"), scope("traffic", "read"));
        assert_eq!(access("GET", "/api/grpc"), scope("traffic", "read"));
        // The only non-read traffic mutation left is the HAR round-trip,
        // which pairs with export under traffic:export.
        assert_eq!(
            access("POST", "/api/traffic/import/har"),
            scope("traffic", "export")
        );
        // Traffic deletion is JWT-only.
        assert_eq!(access("DELETE", "/api/traffic/clear"), RouteAccess::JwtOnly);
        assert_eq!(
            access("DELETE", "/api/ws-traffic/clear"),
            RouteAccess::JwtOnly
        );
        assert_eq!(access("DELETE", "/api/grpc/clear"), RouteAccess::JwtOnly);
    }

    #[test]
    fn route_access_sessions_read_only_for_keys() {
        assert_eq!(access("GET", "/api/sessions"), scope("sessions", "read"));
        assert_eq!(
            access("GET", "/api/sessions/some-id/export"),
            scope("sessions", "read")
        );
        // Switching/creating/importing/deleting sessions is JWT-only.
        assert_eq!(access("POST", "/api/sessions"), RouteAccess::JwtOnly);
        assert_eq!(
            access("DELETE", "/api/sessions/some-id"),
            RouteAccess::JwtOnly
        );
    }

    #[test]
    fn route_access_exports_and_replay() {
        assert_eq!(access("GET", "/api/export/har"), scope("traffic", "export"));
        assert_eq!(
            access("GET", "/api/export/curl"),
            scope("traffic", "export")
        );
        // The whole replay feature hangs off the opt-in replay:execute.
        assert_eq!(access("GET", "/api/replay"), scope("replay", "execute"));
        assert_eq!(
            access("POST", "/api/replay/requests"),
            scope("replay", "execute")
        );
        assert_eq!(
            access("DELETE", "/api/replay/history"),
            scope("replay", "execute")
        );
    }

    #[test]
    fn route_access_rule_families_split_read_write() {
        for resource in ["mocks", "rewrites", "breakpoints", "blocklist", "throttle"] {
            assert_eq!(
                access("GET", &format!("/api/{resource}")),
                scope(resource, "read"),
                "GET /{resource}"
            );
            assert_eq!(
                access("POST", &format!("/api/{resource}")),
                scope(resource, "write"),
                "POST /{resource}"
            );
            assert_eq!(
                access("PUT", &format!("/api/{resource}/rule-1")),
                scope(resource, "write"),
                "PUT /{resource}/rule-1"
            );
            assert_eq!(
                access("DELETE", &format!("/api/{resource}/rule-1")),
                scope(resource, "write"),
                "DELETE /{resource}/rule-1 (rule deletion is write)"
            );
        }
    }

    #[test]
    fn route_access_config_family_covers_monitoring_and_tuning() {
        for path in [
            "/api/config",
            "/api/autosave",
            "/api/capture",
            "/api/focus",
            "/api/mirror",
            "/api/logs",
            "/api/persistence",
            "/api/metrics",
            "/api/performance",
            "/api/instances",
        ] {
            assert_eq!(
                access("GET", path),
                scope("config", "read"),
                "GET {path} should be config:read"
            );
        }
        for (method, path) in [
            ("PATCH", "/api/config"),
            ("PUT", "/api/config"),
            ("POST", "/api/capture/toggle"),
            ("POST", "/api/logs/rotation"),
            ("POST", "/api/persistence/export"),
        ] {
            assert_eq!(
                access(method, path),
                scope("config", "write"),
                "{method} {path} should be config:write"
            );
        }
    }

    #[test]
    fn route_access_jwt_only_exclusions_reject_keys_on_reads_too() {
        for (method, path) in [
            ("GET", "/api/auth/api-keys"),
            ("POST", "/api/auth/api-keys"),
            ("DELETE", "/api/auth/api-keys/key-1"),
            ("GET", "/api/devices"),
            ("POST", "/api/devices"),
            ("POST", "/api/devices/dev-1/rotate"),
            ("GET", "/api/users"),
            ("POST", "/api/users"),
            ("DELETE", "/api/users/user-1"),
            ("GET", "/api/rbac/roles"),
            ("GET", "/api/audit"),
            ("DELETE", "/api/audit/clear"),
            ("GET", "/api/onboarding"),
            ("POST", "/api/onboarding"),
            ("GET", "/api/scripts"),
            ("POST", "/api/scripts"),
            ("GET", "/api/plugins"),
            ("POST", "/api/plugins"),
            ("GET", "/api/secrets"),
            ("PUT", "/api/secrets/my-secret"),
        ] {
            assert_eq!(
                access(method, path),
                RouteAccess::JwtOnly,
                "{method} {path} must be JWT-only"
            );
        }
    }

    #[test]
    fn route_access_self_identity_surface_accepts_keys() {
        for path in ["/auth/me", "/auth/logout", "/auth/validate"] {
            assert_eq!(
                access("GET", &format!("/api{path}")),
                RouteAccess::Authenticated,
                "GET {path}"
            );
            assert_eq!(
                access("POST", &format!("/api{path}")),
                RouteAccess::Authenticated,
                "POST {path}"
            );
        }
    }

    #[test]
    fn route_access_unmapped_routes_deny_keys_by_default() {
        assert_eq!(access("GET", "/api/mystery"), RouteAccess::JwtOnly);
        assert_eq!(access("POST", "/api/mystery/deeper"), RouteAccess::JwtOnly);
        assert_eq!(access("GET", "/api"), RouteAccess::JwtOnly);
    }

    #[test]
    fn is_public_path_covers_ca_distribution_and_ws() {
        for path in [
            "/api/auth/login",
            "/auth/login",
            "/api/auth/refresh",
            "/api/health",
            "/api/health/detailed",
            "/health/detailed",
            "/api/license",
            "/api/devices/enroll",
            "/devices/enroll",
            // #107 additions: CA certificate distribution (QR flow) and
            // the in-handler-authenticated WebSocket stream.
            "/api/cert/ca",
            "/cert/ca",
            "/api/ws",
            "/ws",
        ] {
            assert!(
                is_public_path(&path.parse::<Uri>().unwrap()),
                "{path} should be public"
            );
        }
        // Near-misses and protected routes stay non-public.
        for path in [
            "/api/traffic",
            "/api/auth/me",
            "/api/cert",
            "/api/cert/ca/anything",
            "/api/devices",
            "/api/ws-traffic",
            "/api/wsfoo",
        ] {
            assert!(
                !is_public_path(&path.parse::<Uri>().unwrap()),
                "{path} should NOT be public"
            );
        }
    }

    #[test]
    fn scope_authorized_exact_and_wildcard_matching() {
        let required = Scope::parse("traffic:read");
        assert!(scope_authorized(&required, &["traffic:read".to_string()]));
        assert!(scope_authorized(&required, &["*".to_string()]));
        assert!(scope_authorized(&required, &["traffic:*".to_string()]));
        assert!(scope_authorized(&required, &["*:read".to_string()]));
        assert!(scope_authorized(
            &required,
            &["mocks:write".to_string(), "traffic:read".to_string()]
        ));
        assert!(!scope_authorized(&required, &["traffic:write".to_string()]));
        assert!(!scope_authorized(&required, &["mocks:read".to_string()]));
        assert!(!scope_authorized(&required, &[]));
    }
}

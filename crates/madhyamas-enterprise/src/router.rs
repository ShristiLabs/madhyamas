//! Enterprise route definitions.
//!
//! These routes are extracted from `madhyamas-api/src/routes.rs` (the
//! enterprise block). They return a [`Router`] keyed on
//! `Arc<madhyamas_api::AppState>` so the main binary can merge them with the
//! core API router when the enterprise tier is enabled. The persistent
//! [`EnterpriseStore`] is injected via an [`axum::Extension`] layer so the
//! handlers can access it without `madhyamas-api` depending on this crate.

use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, post, put},
    Extension, Router,
};
use madhyamas_api::AppState;
use std::sync::Arc;

use crate::{
    handlers, middleware, AuditLogger, AuthManager, EnterpriseStore, License, Permission,
    RbacManager, RedisState, ResourceType,
};

/// Create the enterprise router (all enterprise endpoints under `/api`).
///
/// `store` is injected into request extensions so enterprise handlers can
/// persist/restore users, API keys, sessions, and audit events. `auth` is
/// injected so login/refresh/token-validation handlers can mint and verify
/// JWTs. The verified `license` (if any) is injected so the
/// [`handlers::get_license_info`] and [`handlers::get_health_check`]
/// handlers can report license status. The `audit` logger is injected so
/// login/logout handlers can record audit events. `redis` is injected so
/// the detailed health check can probe Redis connectivity (Phase 6d).
///
/// Since issue #107 the authentication middleware itself is NOT applied
/// here: the main binary wraps the whole `/api` nest (OSS routes merged
/// with this router) in [`middleware::auth_middleware`], so every `/api`
/// route is guarded uniformly when `--enable-auth` is on. Public routes
/// (login, refresh, detailed health, license info, device enrollment,
/// CA certificate, `/ws`) bypass the check inside the middleware (see
/// `is_public_path`).
pub fn create_enterprise_router(
    store: Arc<dyn EnterpriseStore>,
    auth: Arc<AuthManager>,
    audit: Arc<AuditLogger>,
    license: Option<License>,
    redis: Option<Arc<RedisState>>,
) -> Router<Arc<AppState>> {
    let rbac = Arc::new(RbacManager::new());

    // Routes that require admin-level RBAC permission (user management).
    let user_routes = Router::new()
        .route("/users", get(handlers::get_users))
        .route("/users", post(handlers::create_user))
        .route("/users/{id}", get(handlers::get_user))
        .route("/users/{id}", put(handlers::update_user))
        .route("/users/{id}", delete(handlers::delete_user))
        .layer(from_fn_with_state(
            middleware::PermissionState {
                rbac: rbac.clone(),
                resource_type: ResourceType::Config,
                permission: Permission::Write,
            },
            middleware::require_permission_middleware,
        ));

    // Audit routes: read for everyone authenticated, clear requires admin.
    let audit_clear_routes = Router::new().route(
        "/audit/clear",
        delete(handlers::clear_audit_events).layer(from_fn_with_state(
            middleware::PermissionState {
                rbac: rbac.clone(),
                resource_type: ResourceType::Config,
                permission: Permission::Delete,
            },
            middleware::require_permission_middleware,
        )),
    );

    Router::new()
        // Performance & Monitoring
        .route("/metrics", get(handlers::get_metrics))
        .route("/metrics/cluster", get(handlers::get_cluster_metrics))
        .route("/instances", get(handlers::get_instances))
        .route("/health/detailed", get(handlers::get_health_check))
        .route("/performance", get(handlers::get_performance_stats))
        // License (public — informational, no auth required)
        .route("/license", get(handlers::get_license_info))
        // Authentication
        .route("/auth/login", post(handlers::login))
        .route("/auth/refresh", post(handlers::refresh_token))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/me", get(handlers::get_current_user))
        .route("/auth/validate", post(handlers::validate_token))
        .route("/auth/api-keys", get(handlers::get_api_keys))
        .route("/auth/api-keys", post(handlers::create_api_key))
        .route("/auth/api-keys/{id}", delete(handlers::revoke_api_key))
        // Device management (issue #104): per-device credentials are
        // connect-only; these routes manage the device registry. A
        // `mdy_dev_` key is rejected by the auth middleware.
        .route("/devices", get(handlers::get_devices))
        .route("/devices", post(handlers::create_device))
        .route("/devices/{id}", delete(handlers::delete_device))
        .route("/devices/{id}/rotate", post(handlers::rotate_device_key))
        .route("/devices/{id}/revoke", post(handlers::revoke_device))
        // QR enrollment (issue #106): token issuance is user-authenticated;
        // redemption is PUBLIC — the enrollment token itself is the
        // credential (the device has no web session when it scans the QR).
        .route(
            "/devices/{id}/enrollment-token",
            post(handlers::issue_device_enrollment_token),
        )
        .route("/devices/enroll", post(handlers::enroll_device))
        // Device-derived agent keys (issue #108): minted in the device's
        // context with a user-picked feature-scope subset, referentially
        // bound to the device (rotation-immune, cascade on revoke/delete).
        // The whole `/devices` surface is JWT-only (route_access), so a
        // device key or agent key can never mint keys.
        .route(
            "/devices/{id}/agent-keys",
            get(handlers::list_agent_keys).post(handlers::create_agent_key),
        )
        .route(
            "/devices/{id}/agent-keys/{key_id}",
            delete(handlers::revoke_agent_key),
        )
        // User Management (admin-only via RBAC)
        .merge(user_routes)
        // RBAC
        .route("/rbac/roles", get(handlers::get_roles))
        .route("/rbac/permissions", get(handlers::get_permissions))
        .route("/rbac/check", post(handlers::check_permission))
        // Audit Logs
        .route("/audit", get(handlers::get_audit_events))
        .route("/audit/stats", get(handlers::get_audit_stats))
        .route("/audit/export", get(handlers::export_audit_events))
        .merge(audit_clear_routes)
        // Onboarding
        .route("/onboarding", get(handlers::get_onboarding_status))
        .route(
            "/onboarding/complete",
            post(handlers::complete_onboarding_step),
        )
        .route("/onboarding/skip", post(handlers::skip_onboarding))
        // Configuration
        .route("/config/export", get(handlers::export_config))
        .route("/config/import", post(handlers::import_config))
        // Inject the persistent store, auth manager, audit logger, and
        // verified license into request extensions so enterprise handlers
        // can access them without madhyamas-api depending on this crate.
        // These are inner layers — they insert values before the route
        // handler runs. The auth middleware no longer lives here (issue
        // #107: it wraps the whole /api nest in the main binary), so no
        // outer re-injection is needed.
        .layer(Extension(store))
        .layer(Extension(auth))
        .layer(Extension(audit))
        .layer(Extension(license))
        .layer(Extension(redis))
}

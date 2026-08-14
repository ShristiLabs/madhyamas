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
/// persist/restore users, API keys, sessions, and audit events. When `auth`
/// is `Some`, JWT authentication is enforced on the enterprise routes via
/// [`middleware::auth_middleware`] (gated by `require_auth`). Public routes
/// (login, refresh, detailed health, license info) bypass the check inside
/// the middleware (see `is_public_path`). The verified `license` (if any) is
/// injected so the [`handlers::get_license_info`] and
/// [`handlers::get_health_check`] handlers can report license status. The
/// `audit` logger is injected so login/logout handlers can record audit
/// events. `redis` is injected so the detailed health check can probe Redis
/// connectivity (Phase 6d).
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

    let router = Router::new()
        // Performance & Monitoring
        .route("/metrics", get(handlers::get_metrics))
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
        // handler runs.
        .layer(Extension(store.clone()))
        .layer(Extension(auth.clone()))
        .layer(Extension(audit.clone()))
        .layer(Extension(license))
        .layer(Extension(redis));

    // Enforce JWT/API-key authentication on enterprise routes. The middleware
    // honors `AuthManager::require_auth()`, so it only rejects requests
    // when strict auth is enabled. Public routes (login, refresh,
    // detailed health, license info) and static assets bypass the check
    // inside the middleware (see `is_public_path`).
    //
    // The store and audit logger are re-injected as outer extension layers
    // (applied before the middleware) so the auth middleware can access them
    // for session idle timeout checks and audit logging. In axum, the last
    // `.layer()` is outermost and runs first; inner `Extension` layers
    // insert their values only after outer middleware has already executed,
    // so the store must be provided outside the middleware as well.
    router
        .layer(axum::middleware::from_fn(middleware::auth_middleware))
        .layer(Extension(auth))
        .layer(Extension(audit))
        .layer(Extension(store))
}

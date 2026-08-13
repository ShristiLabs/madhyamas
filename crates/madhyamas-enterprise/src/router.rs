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

use crate::{handlers, middleware, AuthManager, EnterpriseStore};

/// Create the enterprise router (all enterprise endpoints under `/api`).
///
/// `store` is injected into request extensions so enterprise handlers can
/// persist/restore users, API keys, sessions, and audit events. When `auth`
/// is `Some`, JWT authentication is enforced on the enterprise routes via
/// [`middleware::auth_middleware`] (gated by `require_auth`). Public routes
/// (login, detailed health) bypass the check inside the middleware (see
/// `is_public_path`).
pub fn create_enterprise_router(
    store: Arc<dyn EnterpriseStore>,
    auth: Arc<AuthManager>,
) -> Router<Arc<AppState>> {
    let router = Router::new()
        // Performance & Monitoring
        .route("/metrics", get(handlers::get_metrics))
        .route("/health/detailed", get(handlers::get_health_check))
        .route("/performance", get(handlers::get_performance_stats))
        // Authentication
        .route("/auth/login", post(handlers::login))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/me", get(handlers::get_current_user))
        .route("/auth/validate", post(handlers::validate_token))
        .route("/auth/api-keys", get(handlers::get_api_keys))
        .route("/auth/api-keys", post(handlers::create_api_key))
        .route("/auth/api-keys/{id}", delete(handlers::revoke_api_key))
        // User Management
        .route("/users", get(handlers::get_users))
        .route("/users", post(handlers::create_user))
        .route("/users/{id}", get(handlers::get_user))
        .route("/users/{id}", put(handlers::update_user))
        .route("/users/{id}", delete(handlers::delete_user))
        // RBAC
        .route("/rbac/roles", get(handlers::get_roles))
        .route("/rbac/permissions", get(handlers::get_permissions))
        .route("/rbac/check", post(handlers::check_permission))
        // Audit Logs
        .route("/audit", get(handlers::get_audit_events))
        .route("/audit/stats", get(handlers::get_audit_stats))
        .route("/audit/export", get(handlers::export_audit_events))
        .route("/audit/clear", delete(handlers::clear_audit_events))
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
        // Inject the persistent store and auth manager into request
        // extensions so enterprise handlers can access them without
        // madhyamas-api depending on this crate.
        .layer(Extension(store))
        .layer(Extension(auth.clone()));

    // Enforce JWT authentication on enterprise routes. The middleware itself
    // honors `AuthManager::require_auth()`, so it only rejects requests when
    // strict auth is enabled. Public routes (login, detailed health) and
    // static assets bypass the check inside the middleware (see
    // `is_public_path`).
    router.layer(from_fn_with_state(auth, middleware::auth_middleware))
}

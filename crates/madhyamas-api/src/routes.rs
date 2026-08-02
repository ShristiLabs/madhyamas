//! API routes

#[cfg(feature = "enterprise")]
use axum::middleware::from_fn_with_state;
use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
#[cfg(feature = "enterprise")]
use madhyamas_core::enterprise::AuthManager;
use std::sync::Arc;

use super::handlers;
use super::intercept_handlers;
#[cfg(feature = "enterprise")]
use super::middleware;
#[cfg(any(feature = "grpc", feature = "scripting", feature = "plugins"))]
use super::phase3_handlers;
#[cfg(feature = "enterprise")]
use super::phase4_handlers;
use super::AppState;

/// Create API routes
pub fn create_routes() -> Router<Arc<AppState>> {
    #[cfg(feature = "enterprise")]
    {
        create_routes_inner(false, None)
    }
    #[cfg(not(feature = "enterprise"))]
    {
        create_routes_inner()
    }
}

/// Create API routes with optional Phase 4 (enterprise) endpoints enabled.
///
/// When `enabled` is `true` **and** `auth_service` is provided, JWT
/// authentication is enforced on the Phase 4 routes via
/// [`middleware::auth_middleware`]. Public Phase 4 routes (login, detailed
/// health) bypass the check. When `enabled` is `true` but no auth service is
/// supplied, Phase 4 routes are mounted without authentication (useful for
/// development). When `enabled` is `false`, Phase 4 routes are not mounted at
/// all and `auth_service` is ignored.
#[cfg(feature = "enterprise")]
pub fn create_routes_with_phase4(
    enabled: bool,
    auth_service: Option<Arc<AuthManager>>,
) -> Router<Arc<AppState>> {
    create_routes_inner(enabled, auth_service)
}

fn create_routes_inner(
    #[cfg(feature = "enterprise")] phase4_enabled: bool,
    #[cfg(feature = "enterprise")] auth_service: Option<Arc<AuthManager>>,
) -> Router<Arc<AppState>> {
    let router = Router::new()
        // Traffic endpoints
        .route("/traffic", get(handlers::get_traffic))
        .route("/traffic/{id}", get(handlers::get_traffic_entry))
        .route(
            "/traffic/{id}/script-traces",
            get(phase3_handlers::get_traffic_script_traces),
        )
        .route("/traffic/clear", post(handlers::clear_traffic))
        .route("/traffic/count", get(handlers::get_traffic_count))
        .route("/traffic/import/har", post(handlers::import_traffic_har))
        // Session endpoints
        .route("/sessions", get(handlers::get_sessions))
        .route("/sessions", post(handlers::create_session))
        .route("/sessions/{id}", get(handlers::get_session))
        .route("/sessions/{id}", delete(handlers::delete_session))
        .route("/sessions/{id}/export", get(handlers::export_session))
        .route("/sessions/{id}/switch", post(handlers::switch_session))
        .route("/sessions/import", post(handlers::import_session))
        // Export endpoints
        .route("/export/har", get(handlers::export_har))
        .route("/export/curl/{id}", get(handlers::export_curl))
        // Certificate endpoints
        .route("/cert/ca", get(handlers::get_ca_certificate))
        // WebSocket for real-time updates
        .route("/ws", get(handlers::ws_handler))
        // Config endpoints
        .route("/config", get(handlers::get_config))
        .route("/config", patch(handlers::patch_config))
        // Auto Save endpoints
        .route("/autosave", get(handlers::get_autosave_config))
        .route("/autosave", patch(handlers::update_autosave_config))
        .route(
            "/autosave/snapshot",
            post(handlers::trigger_autosave_snapshot),
        )
        // Capture / passthrough mode
        .route("/capture", get(handlers::get_capture_status))
        .route("/capture/toggle", post(handlers::toggle_capture))
        .route("/capture/stats", get(handlers::get_capture_stats))
        // Health check
        .route("/health", get(|| async { "OK" }))
        // === WebSocket Traffic ===
        .route("/ws-traffic/connections", get(handlers::get_ws_connections))
        .route(
            "/ws-traffic/connections/{id}",
            get(handlers::get_ws_connection),
        )
        .route("/ws-traffic/messages", get(handlers::get_ws_messages))
        .route("/ws-traffic/clear", post(handlers::clear_ws_traffic))
        // === Phase 2: Interception Features ===
        // Breakpoint endpoints
        .route(
            "/breakpoints",
            get(intercept_handlers::get_breakpoint_rules),
        )
        .route(
            "/breakpoints",
            post(intercept_handlers::create_breakpoint_rule),
        )
        .route(
            "/breakpoints/{id}",
            get(intercept_handlers::get_breakpoint_rule),
        )
        .route(
            "/breakpoints/{id}",
            delete(intercept_handlers::delete_breakpoint_rule),
        )
        .route(
            "/breakpoints/paused",
            get(intercept_handlers::get_paused_traffic),
        )
        .route(
            "/breakpoints/paused/{id}",
            get(intercept_handlers::get_paused_item),
        )
        .route(
            "/breakpoints/paused/{id}/resume",
            post(intercept_handlers::resume_paused_item),
        )
        // Mock endpoints
        .route("/mocks", get(intercept_handlers::get_mock_rules))
        .route("/mocks", post(intercept_handlers::create_mock_rule))
        .route(
            "/mocks/templates",
            get(intercept_handlers::get_mock_templates),
        )
        .route("/mocks/{id}", get(intercept_handlers::get_mock_rule))
        .route("/mocks/{id}", put(intercept_handlers::update_mock_rule))
        .route("/mocks/{id}", delete(intercept_handlers::delete_mock_rule))
        .route(
            "/mocks/{id}/toggle",
            post(intercept_handlers::toggle_mock_rule),
        )
        .route(
            "/mocks/batch-toggle",
            post(intercept_handlers::batch_toggle_mocks),
        )
        // Mock Collections
        .route(
            "/mocks/collections",
            get(intercept_handlers::get_mock_collections),
        )
        .route(
            "/mocks/collections",
            post(intercept_handlers::create_mock_collection),
        )
        .route(
            "/mocks/collections/{id}",
            get(intercept_handlers::get_mock_collection),
        )
        .route(
            "/mocks/collections/{id}",
            delete(intercept_handlers::delete_mock_collection),
        )
        .route(
            "/mocks/collections/{id}/toggle",
            post(intercept_handlers::toggle_mock_collection),
        )
        // Mock Recording
        .route(
            "/mocks/recording",
            post(intercept_handlers::set_mock_recording),
        )
        .route(
            "/mocks/recording/status",
            get(intercept_handlers::get_mock_recording_status),
        )
        .route(
            "/mocks/recording/recorded",
            get(intercept_handlers::get_recorded_mocks),
        )
        .route(
            "/mocks/recording/promote",
            post(intercept_handlers::promote_recorded_mocks),
        )
        .route(
            "/mocks/recording/clear",
            post(intercept_handlers::clear_recorded_mocks),
        )
        // Mock Analytics & Hit History
        .route(
            "/mocks/analytics",
            get(intercept_handlers::get_mock_analytics),
        )
        .route(
            "/mocks/{id}/analytics",
            get(intercept_handlers::get_mock_rule_analytics),
        )
        .route(
            "/mocks/{id}/history",
            get(intercept_handlers::get_mock_hit_history),
        )
        .route(
            "/mocks/history/clear",
            post(intercept_handlers::clear_mock_hit_history),
        )
        // Mock Testing & Preview
        .route("/mocks/{id}/test", post(intercept_handlers::test_mock_rule))
        .route(
            "/mocks/preview",
            post(intercept_handlers::preview_mock_match),
        )
        // Mock Import/Export
        .route("/mocks/export", get(intercept_handlers::export_mocks))
        .route("/mocks/import", post(intercept_handlers::import_mocks))
        // Mock Versioning
        .route(
            "/mocks/{id}/duplicate",
            post(intercept_handlers::duplicate_mock_rule),
        )
        .route(
            "/mocks/{id}/rollback",
            post(intercept_handlers::rollback_mock_rule),
        )
        .route(
            "/mocks/{id}/versions",
            get(intercept_handlers::get_mock_version_history),
        )
        // Advanced Mock Creation
        .route(
            "/mocks/advanced",
            post(intercept_handlers::create_advanced_mock_rule),
        )
        // Rewrite endpoints
        .route("/rewrites", get(intercept_handlers::get_rewrite_rules))
        .route("/rewrites", post(intercept_handlers::create_rewrite_rule))
        .route(
            "/rewrites/templates",
            get(intercept_handlers::get_rewrite_templates),
        )
        .route("/rewrites/{id}", get(intercept_handlers::get_rewrite_rule))
        .route(
            "/rewrites/{id}",
            put(intercept_handlers::update_rewrite_rule),
        )
        .route(
            "/rewrites/{id}",
            delete(intercept_handlers::delete_rewrite_rule),
        )
        .route(
            "/rewrites/{id}/toggle",
            post(intercept_handlers::toggle_rewrite_rule),
        )
        .route(
            "/rewrites/batch-toggle",
            post(intercept_handlers::batch_toggle_rewrites),
        )
        // Throttle endpoints
        .route("/throttle", get(intercept_handlers::get_throttle_profile))
        .route("/throttle", post(intercept_handlers::set_throttle_profile))
        .route(
            "/throttle/enabled",
            post(intercept_handlers::set_throttle_enabled),
        )
        .route(
            "/throttle/presets",
            get(intercept_handlers::get_throttle_presets),
        )
        // Replay endpoints
        .route("/replay/saved", get(intercept_handlers::get_saved_requests))
        .route("/replay/saved", post(intercept_handlers::save_request))
        .route(
            "/replay/saved/{id}",
            get(intercept_handlers::get_saved_request),
        )
        .route(
            "/replay/saved/{id}",
            delete(intercept_handlers::delete_saved_request),
        )
        .route(
            "/replay/execute/{id}",
            post(intercept_handlers::replay_request),
        )
        .route(
            "/replay/execute/{id}/batch",
            post(intercept_handlers::replay_request_batch),
        )
        .route(
            "/replay/history",
            get(intercept_handlers::get_replay_history),
        )
        .route(
            "/replay/history",
            delete(intercept_handlers::clear_replay_history),
        )
        // Block List endpoints
        .route("/blocklist", get(intercept_handlers::get_block_list))
        .route(
            "/blocklist",
            post(intercept_handlers::create_block_list_entry),
        )
        .route(
            "/blocklist/stats",
            get(intercept_handlers::get_block_list_stats),
        )
        .route(
            "/blocklist/{id}",
            get(intercept_handlers::get_block_list_entry),
        )
        .route(
            "/blocklist/{id}",
            put(intercept_handlers::update_block_list_entry),
        )
        .route(
            "/blocklist/{id}",
            delete(intercept_handlers::delete_block_list_entry),
        )
        .route(
            "/blocklist/{id}/toggle",
            post(intercept_handlers::toggle_block_list_entry),
        )
        // Focus host endpoints
        .route("/focus", get(handlers::get_focus_hosts))
        .route("/focus", post(handlers::add_focus_host))
        .route("/focus/{id}", delete(handlers::remove_focus_host))
        .route("/focus", delete(handlers::clear_focus_hosts))
        // Mirror tool endpoints
        .route("/mirror", get(handlers::get_mirror_status))
        .route("/mirror/toggle", post(handlers::toggle_mirror))
        .route("/mirror/config", patch(handlers::update_mirror_config))
        // === Persistence endpoints ===
        .route("/persistence/export", get(handlers::export_all_rules))
        .route("/persistence/import", post(handlers::import_all_rules))
        .route("/persistence/save", post(handlers::save_all_rules))
        .route("/persistence/load", post(handlers::load_all_rules));

    // === Phase 3: gRPC Support ===
    #[cfg(feature = "grpc")]
    let router = router
        .route(
            "/grpc/connections",
            get(phase3_handlers::get_grpc_connections),
        )
        .route("/grpc/streams", get(phase3_handlers::get_grpc_streams))
        .route("/grpc/frames", get(phase3_handlers::get_grpc_frames))
        .route("/grpc/stats", get(phase3_handlers::get_grpc_stats))
        .route("/grpc/clear", post(phase3_handlers::clear_grpc_frames));

    // === Phase 3: Scripting System ===
    #[cfg(feature = "scripting")]
    let router = router
        .route("/scripts", get(phase3_handlers::get_scripts))
        .route("/scripts", post(phase3_handlers::create_script))
        .route(
            "/scripts/templates",
            get(phase3_handlers::get_script_templates),
        )
        .route("/scripts/config", get(phase3_handlers::get_script_config))
        .route(
            "/scripts/config",
            put(phase3_handlers::update_script_config),
        )
        .route(
            "/scripts/history",
            get(phase3_handlers::get_scripts_history),
        )
        .route("/scripts/test", post(phase3_handlers::test_script))
        .route("/scripts/validate", post(phase3_handlers::validate_script))
        .route(
            "/scripts/match-preview",
            post(phase3_handlers::match_preview_scripts),
        )
        .route("/scripts/{id}", get(phase3_handlers::get_script))
        .route("/scripts/{id}", put(phase3_handlers::update_script))
        .route("/scripts/{id}", delete(phase3_handlers::delete_script))
        .route("/scripts/{id}/toggle", post(phase3_handlers::toggle_script))
        .route(
            "/scripts/{id}/reorder",
            post(phase3_handlers::reorder_script),
        )
        .route(
            "/scripts/{id}/history",
            get(phase3_handlers::get_script_history),
        )
        .route(
            "/scripts/{id}/history",
            delete(phase3_handlers::clear_script_history),
        );

    // === Phase 3: Plugin System ===
    #[cfg(feature = "plugins")]
    let router = router
        .route("/plugins", get(phase3_handlers::get_plugins))
        .route("/plugins/{id}", get(phase3_handlers::get_plugin))
        .route("/plugins/{id}/enable", post(phase3_handlers::enable_plugin))
        .route(
            "/plugins/{id}/disable",
            post(phase3_handlers::disable_plugin),
        )
        .route(
            "/plugins/{id}/stats",
            get(phase3_handlers::get_plugin_stats),
        )
        .route("/plugins/reload", post(phase3_handlers::reload_plugins));

    // === Phase 4: Enterprise features (conditionally enabled) ===
    #[cfg(feature = "enterprise")]
    {
        if phase4_enabled {
            let phase4_router = router
                // Performance & Monitoring
                .route("/metrics", get(phase4_handlers::get_metrics))
                .route("/health/detailed", get(phase4_handlers::get_health_check))
                .route("/performance", get(phase4_handlers::get_performance_stats))
                // Authentication
                .route("/auth/login", post(phase4_handlers::login))
                .route("/auth/logout", post(phase4_handlers::logout))
                .route("/auth/me", get(phase4_handlers::get_current_user))
                .route("/auth/validate", post(phase4_handlers::validate_token))
                .route("/auth/api-keys", get(phase4_handlers::get_api_keys))
                .route("/auth/api-keys", post(phase4_handlers::create_api_key))
                .route(
                    "/auth/api-keys/{id}",
                    delete(phase4_handlers::revoke_api_key),
                )
                // User Management
                .route("/users", get(phase4_handlers::get_users))
                .route("/users", post(phase4_handlers::create_user))
                .route("/users/{id}", get(phase4_handlers::get_user))
                .route("/users/{id}", put(phase4_handlers::update_user))
                .route("/users/{id}", delete(phase4_handlers::delete_user))
                // RBAC
                .route("/rbac/roles", get(phase4_handlers::get_roles))
                .route("/rbac/permissions", get(phase4_handlers::get_permissions))
                .route("/rbac/check", post(phase4_handlers::check_permission))
                // Audit Logs
                .route("/audit", get(phase4_handlers::get_audit_events))
                .route("/audit/stats", get(phase4_handlers::get_audit_stats))
                .route("/audit/export", get(phase4_handlers::export_audit_events))
                .route("/audit/clear", delete(phase4_handlers::clear_audit_events))
                // Onboarding
                .route("/onboarding", get(phase4_handlers::get_onboarding_status))
                .route(
                    "/onboarding/complete",
                    post(phase4_handlers::complete_onboarding_step),
                )
                .route("/onboarding/skip", post(phase4_handlers::skip_onboarding))
                // Configuration
                .route("/config/export", get(phase4_handlers::export_config))
                .route("/config/import", post(phase4_handlers::import_config));

            // Enforce JWT authentication on Phase 4 routes when an auth service is
            // provided. Public routes (login, detailed health) and static assets
            // bypass the check inside the middleware (see `is_public_path`).
            if let Some(auth) = auth_service {
                return phase4_router.layer(from_fn_with_state(auth, middleware::auth_middleware));
            } else {
                return phase4_router;
            }
        }
    }

    router
}

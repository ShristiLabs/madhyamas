//! API routes

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;

use super::handlers;
use super::intercept_handlers;
use super::phase3_handlers;
use super::phase4_handlers;
use super::AppState;

/// Create API routes
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Traffic endpoints
        .route("/traffic", get(handlers::get_traffic))
        .route("/traffic/{id}", get(handlers::get_traffic_entry))
        .route("/traffic/clear", post(handlers::clear_traffic))
        .route("/traffic/count", get(handlers::get_traffic_count))
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
            delete(intercept_handlers::delete_rewrite_rule),
        )
        .route(
            "/rewrites/{id}/toggle",
            post(intercept_handlers::toggle_rewrite_rule),
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
            "/replay/history",
            get(intercept_handlers::get_replay_history),
        )
        .route(
            "/replay/history",
            delete(intercept_handlers::clear_replay_history),
        )
        // === Persistence endpoints ===
        .route("/persistence/export", get(handlers::export_all_rules))
        .route("/persistence/import", post(handlers::import_all_rules))
        .route("/persistence/save", post(handlers::save_all_rules))
        .route("/persistence/load", post(handlers::load_all_rules))
        // === Phase 3: gRPC Support ===
        .route(
            "/grpc/connections",
            get(phase3_handlers::get_grpc_connections),
        )
        .route("/grpc/streams", get(phase3_handlers::get_grpc_streams))
        .route("/grpc/frames", get(phase3_handlers::get_grpc_frames))
        .route("/grpc/stats", get(phase3_handlers::get_grpc_stats))
        .route("/grpc/clear", post(phase3_handlers::clear_grpc_frames))
        // === Phase 3: Scripting System ===
        .route("/scripts", get(phase3_handlers::get_scripts))
        .route("/scripts", post(phase3_handlers::create_script))
        .route(
            "/scripts/templates",
            get(phase3_handlers::get_script_templates),
        )
        .route("/scripts/config", get(phase3_handlers::get_script_config))
        .route("/scripts/{id}", get(phase3_handlers::get_script))
        .route("/scripts/{id}", put(phase3_handlers::update_script))
        .route("/scripts/{id}", delete(phase3_handlers::delete_script))
        .route("/scripts/{id}/toggle", post(phase3_handlers::toggle_script))
        // === Phase 3: Plugin System ===
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
        .route("/plugins/reload", post(phase3_handlers::reload_plugins))
        // === Phase 4: Performance & Monitoring ===
        .route("/metrics", get(phase4_handlers::get_metrics))
        .route("/health/detailed", get(phase4_handlers::get_health_check))
        .route("/performance", get(phase4_handlers::get_performance_stats))
        // === Phase 4: Authentication ===
        .route("/auth/login", post(phase4_handlers::login))
        .route("/auth/logout", post(phase4_handlers::logout))
        .route("/auth/me", get(phase4_handlers::get_current_user))
        .route("/auth/validate", post(phase4_handlers::validate_token))
        .route("/auth/api-keys", get(phase4_handlers::get_api_keys))
        .route("/auth/api-keys", post(phase4_handlers::create_api_key))
        .route("/auth/api-keys/{id}", delete(phase4_handlers::revoke_api_key))
        // === Phase 4: User Management ===
        .route("/users", get(phase4_handlers::get_users))
        .route("/users", post(phase4_handlers::create_user))
        .route("/users/{id}", get(phase4_handlers::get_user))
        .route("/users/{id}", put(phase4_handlers::update_user))
        .route("/users/{id}", delete(phase4_handlers::delete_user))
        // === Phase 4: RBAC ===
        .route("/rbac/roles", get(phase4_handlers::get_roles))
        .route("/rbac/permissions", get(phase4_handlers::get_permissions))
        .route("/rbac/check", post(phase4_handlers::check_permission))
        // === Phase 4: Audit Logs ===
        .route("/audit", get(phase4_handlers::get_audit_events))
        .route("/audit/stats", get(phase4_handlers::get_audit_stats))
        .route("/audit/export", get(phase4_handlers::export_audit_events))
        .route("/audit/clear", delete(phase4_handlers::clear_audit_events))
        // === Phase 4: Onboarding ===
        .route("/onboarding", get(phase4_handlers::get_onboarding_status))
        .route("/onboarding/complete", post(phase4_handlers::complete_onboarding_step))
        .route("/onboarding/skip", post(phase4_handlers::skip_onboarding))
        // === Phase 4: Configuration ===
        .route("/config/export", get(phase4_handlers::export_config))
        .route("/config/import", post(phase4_handlers::import_config))
}

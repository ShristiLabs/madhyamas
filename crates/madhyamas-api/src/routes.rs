//! API routes

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;

use super::handlers;
use super::intercept_handlers;
#[cfg(any(feature = "grpc", feature = "scripting", feature = "plugins"))]
use super::tools_handlers;
use super::AppState;

/// Create API routes
pub fn create_routes() -> Router<Arc<AppState>> {
    create_routes_inner()
}

fn create_routes_inner() -> Router<Arc<AppState>> {
    let router = Router::new()
        // Traffic endpoints
        .route("/traffic", get(handlers::get_traffic))
        .route("/traffic/{id}", get(handlers::get_traffic_entry))
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
        // Health check — verifies database connectivity (unauthenticated,
        // for Docker/nginx health probes)
        .route("/health", get(handlers::health_check))
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
        .route(
            "/mocks/from-traffic",
            post(intercept_handlers::create_mock_from_traffic),
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
            put(intercept_handlers::update_mock_collection),
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
            "/replay/saved/from-traffic",
            post(intercept_handlers::batch_save_requests_from_traffic),
        )
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
        // Log rotation endpoints
        .route("/logs", get(handlers::get_log_status))
        .route("/logs", patch(handlers::update_log_config))
        .route("/logs/rotate", post(handlers::rotate_logs))
        // === Persistence endpoints ===
        .route("/persistence/export", get(handlers::export_all_rules))
        .route("/persistence/import", post(handlers::import_all_rules))
        .route("/persistence/save", post(handlers::save_all_rules))
        .route("/persistence/load", post(handlers::load_all_rules));

    // === gRPC Support ===
    #[cfg(feature = "grpc")]
    let router = router
        .route(
            "/grpc/connections",
            get(tools_handlers::get_grpc_connections),
        )
        .route("/grpc/streams", get(tools_handlers::get_grpc_streams))
        .route("/grpc/frames", get(tools_handlers::get_grpc_frames))
        .route("/grpc/stats", get(tools_handlers::get_grpc_stats))
        .route("/grpc/clear", post(tools_handlers::clear_grpc_frames));

    // === Scripting System ===
    #[cfg(feature = "scripting")]
    let router = router
        .route(
            "/traffic/{id}/script-traces",
            get(tools_handlers::get_traffic_script_traces),
        )
        .route("/scripts", get(tools_handlers::get_scripts))
        .route("/scripts", post(tools_handlers::create_script))
        .route(
            "/scripts/templates",
            get(tools_handlers::get_script_templates),
        )
        .route("/scripts/config", get(tools_handlers::get_script_config))
        .route("/scripts/config", put(tools_handlers::update_script_config))
        .route("/scripts/history", get(tools_handlers::get_scripts_history))
        .route("/scripts/test", post(tools_handlers::test_script))
        .route("/scripts/validate", post(tools_handlers::validate_script))
        .route(
            "/scripts/match-preview",
            post(tools_handlers::match_preview_scripts),
        )
        .route("/scripts/{id}", get(tools_handlers::get_script))
        .route("/scripts/{id}", put(tools_handlers::update_script))
        .route("/scripts/{id}", delete(tools_handlers::delete_script))
        .route("/scripts/{id}/toggle", post(tools_handlers::toggle_script))
        .route(
            "/scripts/{id}/reorder",
            post(tools_handlers::reorder_script),
        )
        .route(
            "/scripts/{id}/history",
            get(tools_handlers::get_script_history),
        )
        .route(
            "/scripts/{id}/history",
            delete(tools_handlers::clear_script_history),
        );

    // === Plugin System ===
    #[cfg(feature = "plugins")]
    let router = router
        .route("/plugins", get(tools_handlers::get_plugins))
        .route("/plugins/{id}", get(tools_handlers::get_plugin))
        .route("/plugins/{id}/enable", post(tools_handlers::enable_plugin))
        .route(
            "/plugins/{id}/disable",
            post(tools_handlers::disable_plugin),
        )
        .route("/plugins/{id}/stats", get(tools_handlers::get_plugin_stats))
        .route("/plugins/reload", post(tools_handlers::reload_plugins))
        .route("/plugins/install", post(tools_handlers::install_plugin))
        .route(
            "/plugins/{id}/uninstall",
            delete(tools_handlers::uninstall_plugin),
        )
        .route(
            "/plugins/{id}/settings",
            get(tools_handlers::get_plugin_settings),
        )
        .route(
            "/plugins/{id}/settings",
            put(tools_handlers::update_plugin_settings),
        )
        .route(
            "/plugins/{id}/schema",
            get(tools_handlers::get_plugin_settings_schema),
        )
        .route(
            "/plugins/{id}/panels",
            get(tools_handlers::get_plugin_panels),
        )
        .route("/plugins/{id}/logs", get(tools_handlers::get_plugin_logs))
        .route("/plugins/registry", get(tools_handlers::list_registry))
        .route(
            "/plugins/registry/search",
            get(tools_handlers::search_registry),
        )
        .route(
            "/plugins/registry/{id}",
            get(tools_handlers::get_registry_entry),
        )
        .route(
            "/plugins/registry/config",
            get(tools_handlers::get_registry_config),
        )
        .route(
            "/plugins/registry/config",
            put(tools_handlers::set_registry_config),
        )
        .route(
            "/plugins/registry/refresh",
            post(tools_handlers::refresh_registry),
        )
        .route(
            "/plugins/templates",
            get(tools_handlers::list_plugin_templates),
        )
        .route("/plugins/scaffold", post(tools_handlers::scaffold_plugin));

    router
}

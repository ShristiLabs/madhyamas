//! Tool registry and executor for MCP server.
//!
//! All MCP tools implement the [`tool_trait::McpTool`] trait and are
//! registered in [`default_registry`].  Adding a new tool is as simple as
//! defining a struct in the appropriate domain module and pushing it into
//! the registry builder — no edits to any other file are required.
//!
//! ## Module layout
//!
//! | Module        | Tools                                          |
//! |---------------|------------------------------------------------|
//! | `sessions`    | Session list/create/switch/export/import       |
//! | `config`      | Proxy config + capture mode                    |
//! | `certificate` | CA certificate info                            |
//! | `traffic`     | Traffic list/get/search/count/clear/import-har |
//! | `mocks`       | Mock CRUD + collections + analytics + recording|
//! | `breakpoints` | Breakpoint CRUD + paused traffic               |
//! | `replay`      | Replay/save/list/export-curl + clear history   |
//! | `rewrites`    | Rewrite CRUD + templates                      |
//! | `throttle`    | Throttle get/set/toggle/presets               |
//! | `grpc`        | gRPC connections/streams/frames/stats/clear    |
//! | `scripts`     | Script CRUD + templates/test/validate/history |
//! | `plugins`     | Plugin CRUD + registry + panels/templates     |
//! | `focus`       | Focus host CRUD                                |
//! | `mirror`      | Mirror status/toggle/config                    |
//! | `logs`        | Log rotation status/rotate/config              |
//! | `blocklist`   | Block list CRUD + stats + toggle               |
//! | `ws_traffic`  | WebSocket connections/messages/clear           |
//! | `autosave`    | Auto Save config/update/snapshot               |
//! | `helpers`     | Shared `get_id`/`api_result` utilities         |
//! | `tool_trait`  | `McpTool` trait + `DynToolRegistry`            |

mod autosave;
mod blocklist;
mod breakpoints;
mod certificate;
mod config;
mod focus;
mod grpc;
mod helpers;
mod logs;
mod mirror;
mod mocks;
mod plugins;
mod replay;
mod rewrites;
mod scripts;
mod sessions;
mod throttle;
mod tool_trait;
mod traffic;
mod ws_traffic;

pub use tool_trait::{tool_definition, DynToolRegistry, McpTool};

// Re-export all tool structs so they can be registered in `default_registry`.
pub use autosave::{GetAutoSaveConfigTool, TriggerAutoSaveSnapshotTool, UpdateAutoSaveConfigTool};
pub use blocklist::{
    CreateBlockListEntryTool, DeleteBlockListEntryTool, GetBlockListEntryTool,
    GetBlockListStatsTool, ListBlockListTool, ToggleBlockListEntryTool, UpdateBlockListEntryTool,
};
pub use breakpoints::{
    CreateBreakpointTool, DeleteBreakpointTool, GetBreakpointTool, GetPausedItemTool,
    ListBreakpointsTool, ListPausedTrafficTool, ResumePausedItemTool,
};
pub use certificate::GetCertInfoTool;
pub use config::{GetCaptureStatusTool, GetConfigTool, ToggleCaptureTool, UpdateConfigTool};
pub use focus::{AddFocusHostTool, ClearFocusHostsTool, ListFocusHostsTool, RemoveFocusHostTool};
pub use grpc::{
    ClearGrpcTool, GetGrpcConnectionsTool, GetGrpcFramesTool, GetGrpcStatsTool, GetGrpcStreamsTool,
};
pub use logs::{GetLogStatusTool, RotateLogsTool, UpdateLogConfigTool};
pub use mirror::{GetMirrorStatusTool, ToggleMirrorTool, UpdateMirrorConfigTool};
pub use mocks::{
    BatchToggleMocksTool, ClearMockHistoryTool, ClearMockRecordingTool, CreateAdvancedMockTool,
    CreateMockCollectionTool, CreateMockTool, DeleteMockCollectionTool, DeleteMockTool,
    DuplicateMockTool, ExportMocksTool, GetMockAnalyticsTool, GetMockCollectionTool,
    GetMockHitHistoryTool, GetMockRecordingStatusTool, GetMockTemplatesTool, GetMockTool,
    GetMockVersionsTool, GetRecordedMocksTool, ImportMocksTool, ListMockCollectionsTool,
    ListMocksTool, PreviewMockMatchTool, PromoteRecordedMocksTool, RollbackMockTool,
    SetMockRecordingTool, TestMockTool, ToggleMockCollectionTool, ToggleMockTool,
    UpdateMockCollectionTool, UpdateMockTool,
};
pub use plugins::{
    DisablePluginTool, EnablePluginTool, GetPluginLogsTool, GetPluginPanelsTool,
    GetPluginSchemaTool, GetPluginSettingsTool, GetPluginStatsTool, GetPluginTemplatesTool,
    GetPluginTool, GetRegistryConfigTool, InstallPluginTool, ListPluginsTool, ListRegistryTool,
    RefreshRegistryTool, ReloadPluginsTool, SearchRegistryTool, UninstallPluginTool,
    UpdatePluginSettingsTool, UpdateRegistryConfigTool,
};
pub use replay::{
    ClearReplayHistoryTool, ExportCurlTool, ListSavedRequestsTool, ReplayAdvancedTool,
    ReplayRequestTool, SaveRequestTool,
};
pub use rewrites::{
    BatchToggleRewritesTool, CreateRewriteTool, DeleteRewriteTool, GetRewriteTemplatesTool,
    ListRewritesTool, ToggleRewriteTool, UpdateRewriteTool,
};
pub use scripts::{
    ClearScriptHistoryTool, CreateScriptTool, DeleteScriptTool, GetScriptConfigTool,
    GetScriptHistoryAllTool, GetScriptHistoryTool, GetScriptTemplatesTool, GetScriptTool,
    ListScriptsTool, ReorderScriptTool, ScriptMatchPreviewTool, TestScriptTool, ToggleScriptTool,
    UpdateScriptConfigTool, UpdateScriptTool, ValidateScriptTool,
};
pub use sessions::{
    CreateSessionTool, ExportSessionTool, ImportSessionTool, ListSessionsTool, SwitchSessionTool,
};
pub use throttle::{GetThrottlePresetsTool, GetThrottleTool, SetThrottleTool, ToggleThrottleTool};
pub use traffic::{
    ClearTrafficTool, GetTrafficCountTool, GetTrafficEntryTool, GetTrafficScriptTracesTool,
    GetTrafficTool, ImportHarTool, SearchTrafficTool,
};
pub use ws_traffic::{
    ClearWsTrafficTool, GetWsConnectionTool, GetWsMessagesTool, ListWsConnectionsTool,
};

/// Sanitize an ID for safe inclusion in a URL path segment.
///
/// Rejects path traversal attempts (`..`, `/`, `\`, control characters)
/// and ensures the ID only contains URL-safe characters.
pub fn sanitize_id(id: &str) -> Result<String, crate::types::McpError> {
    if id.is_empty() {
        return Err(crate::types::McpError::InvalidParams(
            "ID cannot be empty".to_string(),
        ));
    }
    // Reject anything with path separators or traversal sequences
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(crate::types::McpError::InvalidParams(
            "Invalid ID: path separators not allowed".to_string(),
        ));
    }
    // Only allow alphanumeric, dash, underscore, and dot (for UUIDs/versions)
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(crate::types::McpError::InvalidParams(
            "Invalid ID: only alphanumeric, dash, underscore, and dot allowed".to_string(),
        ));
    }
    Ok(id.to_string())
}

// ---------------------------------------------------------------------------
// Registry builder
// ---------------------------------------------------------------------------

/// Build a [`DynToolRegistry`] pre-populated with every MCP tool.
///
/// Call this once at server startup.
pub fn default_registry() -> DynToolRegistry {
    let mut reg = DynToolRegistry::new();

    // Sessions
    reg.register(Box::new(ListSessionsTool));
    reg.register(Box::new(CreateSessionTool));
    reg.register(Box::new(SwitchSessionTool));
    reg.register(Box::new(ExportSessionTool));
    reg.register(Box::new(ImportSessionTool));

    // Config & capture
    reg.register(Box::new(GetConfigTool));
    reg.register(Box::new(UpdateConfigTool));
    reg.register(Box::new(GetCaptureStatusTool));
    reg.register(Box::new(ToggleCaptureTool));

    // Certificate
    reg.register(Box::new(GetCertInfoTool));

    // Traffic
    reg.register(Box::new(GetTrafficTool));
    reg.register(Box::new(GetTrafficEntryTool));
    reg.register(Box::new(SearchTrafficTool));
    reg.register(Box::new(GetTrafficCountTool));
    reg.register(Box::new(ClearTrafficTool));
    reg.register(Box::new(ImportHarTool));
    reg.register(Box::new(GetTrafficScriptTracesTool));

    // Mocks
    reg.register(Box::new(ListMocksTool));
    reg.register(Box::new(CreateMockTool));
    reg.register(Box::new(DeleteMockTool));
    reg.register(Box::new(ToggleMockTool));
    reg.register(Box::new(CreateAdvancedMockTool));
    reg.register(Box::new(UpdateMockTool));
    reg.register(Box::new(GetMockTool));
    reg.register(Box::new(DuplicateMockTool));
    reg.register(Box::new(RollbackMockTool));
    reg.register(Box::new(GetMockVersionsTool));
    reg.register(Box::new(ListMockCollectionsTool));
    reg.register(Box::new(CreateMockCollectionTool));
    reg.register(Box::new(DeleteMockCollectionTool));
    reg.register(Box::new(ToggleMockCollectionTool));
    reg.register(Box::new(GetMockCollectionTool));
    reg.register(Box::new(UpdateMockCollectionTool));
    reg.register(Box::new(GetMockAnalyticsTool));
    reg.register(Box::new(GetMockHitHistoryTool));
    reg.register(Box::new(TestMockTool));
    reg.register(Box::new(PreviewMockMatchTool));
    reg.register(Box::new(ExportMocksTool));
    reg.register(Box::new(ImportMocksTool));
    reg.register(Box::new(SetMockRecordingTool));
    reg.register(Box::new(GetMockRecordingStatusTool));
    reg.register(Box::new(GetRecordedMocksTool));
    reg.register(Box::new(PromoteRecordedMocksTool));
    reg.register(Box::new(GetMockTemplatesTool));
    reg.register(Box::new(BatchToggleMocksTool));
    reg.register(Box::new(ClearMockRecordingTool));
    reg.register(Box::new(ClearMockHistoryTool));

    // Breakpoints
    reg.register(Box::new(ListBreakpointsTool));
    reg.register(Box::new(CreateBreakpointTool));
    reg.register(Box::new(DeleteBreakpointTool));
    reg.register(Box::new(GetBreakpointTool));
    reg.register(Box::new(ListPausedTrafficTool));
    reg.register(Box::new(GetPausedItemTool));
    reg.register(Box::new(ResumePausedItemTool));

    // Replay
    reg.register(Box::new(ReplayRequestTool));
    reg.register(Box::new(ReplayAdvancedTool));
    reg.register(Box::new(SaveRequestTool));
    reg.register(Box::new(ListSavedRequestsTool));
    reg.register(Box::new(ExportCurlTool));
    reg.register(Box::new(ClearReplayHistoryTool));

    // Rewrites
    reg.register(Box::new(ListRewritesTool));
    reg.register(Box::new(CreateRewriteTool));
    reg.register(Box::new(DeleteRewriteTool));
    reg.register(Box::new(ToggleRewriteTool));
    reg.register(Box::new(GetRewriteTemplatesTool));
    reg.register(Box::new(UpdateRewriteTool));
    reg.register(Box::new(BatchToggleRewritesTool));

    // Throttle
    reg.register(Box::new(GetThrottleTool));
    reg.register(Box::new(SetThrottleTool));
    reg.register(Box::new(ToggleThrottleTool));
    reg.register(Box::new(GetThrottlePresetsTool));

    // gRPC
    reg.register(Box::new(GetGrpcConnectionsTool));
    reg.register(Box::new(GetGrpcStreamsTool));
    reg.register(Box::new(GetGrpcFramesTool));
    reg.register(Box::new(GetGrpcStatsTool));
    reg.register(Box::new(ClearGrpcTool));

    // Scripts
    reg.register(Box::new(ListScriptsTool));
    reg.register(Box::new(CreateScriptTool));
    reg.register(Box::new(GetScriptTool));
    reg.register(Box::new(UpdateScriptTool));
    reg.register(Box::new(DeleteScriptTool));
    reg.register(Box::new(ToggleScriptTool));
    reg.register(Box::new(GetScriptTemplatesTool));
    reg.register(Box::new(TestScriptTool));
    reg.register(Box::new(ValidateScriptTool));
    reg.register(Box::new(GetScriptHistoryTool));
    reg.register(Box::new(ReorderScriptTool));
    reg.register(Box::new(ScriptMatchPreviewTool));
    reg.register(Box::new(GetScriptHistoryAllTool));
    reg.register(Box::new(ClearScriptHistoryTool));
    reg.register(Box::new(GetScriptConfigTool));
    reg.register(Box::new(UpdateScriptConfigTool));

    // Plugins
    reg.register(Box::new(ListPluginsTool));
    reg.register(Box::new(GetPluginTool));
    reg.register(Box::new(EnablePluginTool));
    reg.register(Box::new(DisablePluginTool));
    reg.register(Box::new(GetPluginStatsTool));
    reg.register(Box::new(ReloadPluginsTool));
    reg.register(Box::new(InstallPluginTool));
    reg.register(Box::new(UninstallPluginTool));
    reg.register(Box::new(SearchRegistryTool));
    reg.register(Box::new(ListRegistryTool));
    reg.register(Box::new(GetPluginSchemaTool));
    reg.register(Box::new(GetPluginSettingsTool));
    reg.register(Box::new(UpdatePluginSettingsTool));
    reg.register(Box::new(GetPluginLogsTool));
    reg.register(Box::new(GetPluginPanelsTool));
    reg.register(Box::new(GetPluginTemplatesTool));
    reg.register(Box::new(GetRegistryConfigTool));
    reg.register(Box::new(UpdateRegistryConfigTool));
    reg.register(Box::new(RefreshRegistryTool));

    // Focus hosts
    reg.register(Box::new(ListFocusHostsTool));
    reg.register(Box::new(AddFocusHostTool));
    reg.register(Box::new(RemoveFocusHostTool));
    reg.register(Box::new(ClearFocusHostsTool));

    // Mirror
    reg.register(Box::new(GetMirrorStatusTool));
    reg.register(Box::new(ToggleMirrorTool));
    reg.register(Box::new(UpdateMirrorConfigTool));

    // Log rotation
    reg.register(Box::new(GetLogStatusTool));
    reg.register(Box::new(RotateLogsTool));
    reg.register(Box::new(UpdateLogConfigTool));

    // Block list
    reg.register(Box::new(ListBlockListTool));
    reg.register(Box::new(GetBlockListStatsTool));
    reg.register(Box::new(CreateBlockListEntryTool));
    reg.register(Box::new(GetBlockListEntryTool));
    reg.register(Box::new(UpdateBlockListEntryTool));
    reg.register(Box::new(DeleteBlockListEntryTool));
    reg.register(Box::new(ToggleBlockListEntryTool));

    // WebSocket traffic
    reg.register(Box::new(ListWsConnectionsTool));
    reg.register(Box::new(GetWsConnectionTool));
    reg.register(Box::new(GetWsMessagesTool));
    reg.register(Box::new(ClearWsTrafficTool));

    // Auto Save
    reg.register(Box::new(GetAutoSaveConfigTool));
    reg.register(Box::new(UpdateAutoSaveConfigTool));
    reg.register(Box::new(TriggerAutoSaveSnapshotTool));

    reg
}

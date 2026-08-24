//! Public-API integration tests for enterprise MCP tools, migrated from
//! the inline module in src/tools/enterprise.rs.

use madhyamas_mcp::tools::McpTool;
use madhyamas_mcp::tools::{
    CreateUserTool, DeleteUserTool, ExportAuditTool, ExportConfigTool, GetAuditEventsTool,
    GetHealthTool, GetLicenseInfoTool, GetMetricsTool, ImportConfigTool, ListUsersTool,
    UpdateUserRoleTool,
};

#[test]
fn test_enterprise_tool_names() {
    assert_eq!(ListUsersTool.name(), "madhyamas_list_users");
    assert_eq!(CreateUserTool.name(), "madhyamas_create_user");
    assert_eq!(DeleteUserTool.name(), "madhyamas_delete_user");
    assert_eq!(UpdateUserRoleTool.name(), "madhyamas_update_user_role");
    assert_eq!(GetAuditEventsTool.name(), "madhyamas_get_audit_events");
    assert_eq!(ExportAuditTool.name(), "madhyamas_export_audit");
    assert_eq!(GetLicenseInfoTool.name(), "madhyamas_get_license_info");
    assert_eq!(GetMetricsTool.name(), "madhyamas_get_metrics");
    assert_eq!(GetHealthTool.name(), "madhyamas_get_health");
    assert_eq!(ExportConfigTool.name(), "madhyamas_export_config");
    assert_eq!(ImportConfigTool.name(), "madhyamas_import_config");
}

#[test]
fn test_enterprise_tool_annotations() {
    let ann = ListUsersTool.annotations().expect("annotations");
    assert_eq!(ann.read_only, Some(true));
    assert_eq!(ann.destructive, Some(false));
    assert_eq!(ann.idempotent, Some(true));
    assert!(ann.required_permission.is_some());

    let ann = DeleteUserTool.annotations().expect("annotations");
    assert_eq!(ann.destructive, Some(true));

    let ann = CreateUserTool.annotations().expect("annotations");
    assert_eq!(ann.idempotent, Some(false));
}

#[test]
fn test_enterprise_registry_registers_all() {
    let reg = madhyamas_mcp::tools::enterprise_registry();
    assert_eq!(reg.len(), 11);
}

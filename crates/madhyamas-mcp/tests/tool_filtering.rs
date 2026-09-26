//! Public-API integration tests for the issue #107 MCP tool-annotation
//! contract: every registered tool carries annotations (deny-by-default
//! filtering hides un-annotated tools from API-key principals), the
//! `jwt:only` sentinel marks the JWT-only surface, and the taxonomy spot
//! checks pin representative tools to their feature scopes.

use madhyamas_mcp::tools::{default_registry, enterprise_registry};
use madhyamas_mcp::types::{ToolAnnotations, JWT_ONLY_PERMISSION};

fn permission_of(tool: &madhyamas_mcp::types::Tool) -> Option<&str> {
    tool.annotations
        .as_ref()
        .and_then(|a| a.required_permission.as_deref())
}

#[test]
fn every_default_registry_tool_is_annotated() {
    let tools = default_registry().list_tools();
    assert!(!tools.is_empty(), "default registry should not be empty");
    let unannotated: Vec<&str> = tools
        .iter()
        .filter(|t| t.annotations.is_none())
        .map(|t| t.name.as_str())
        .collect();
    assert!(
        unannotated.is_empty(),
        "tools without annotations are hidden for API-key principals \
         (deny-by-default); annotate or mark public(): {unannotated:?}"
    );
}

#[test]
fn every_enterprise_registry_tool_is_annotated() {
    let tools = enterprise_registry().list_tools();
    assert!(!tools.is_empty(), "enterprise registry should not be empty");
    let unannotated: Vec<&str> = tools
        .iter()
        .filter(|t| t.annotations.is_none())
        .map(|t| t.name.as_str())
        .collect();
    assert!(
        unannotated.is_empty(),
        "enterprise tools without annotations would vanish for keys: {unannotated:?}"
    );
}

#[test]
fn taxonomy_spot_checks_pin_representative_tools() {
    let tools = default_registry().list_tools();
    let by_name = |name: &str| {
        tools
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("tool {name} not in default registry"))
    };
    assert_eq!(
        permission_of(by_name("madhyamas_get_traffic")),
        Some("traffic:read")
    );
    assert_eq!(
        permission_of(by_name("madhyamas_clear_traffic")),
        Some(JWT_ONLY_PERMISSION)
    );
    assert_eq!(
        permission_of(by_name("madhyamas_import_har")),
        Some("traffic:export")
    );
    // The CA distribution route is public; its tool must be visible to
    // every principal class (public(), not an un-annotated tool).
    let cert = by_name("madhyamas_get_cert_info");
    assert!(
        cert.annotations.is_some(),
        "cert tool must carry annotations"
    );
}

#[test]
fn enterprise_user_management_tools_are_jwt_only() {
    let tools = enterprise_registry().list_tools();
    let by_name = |name: &str| {
        tools
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("tool {name} not in enterprise registry"))
    };
    for name in [
        "madhyamas_list_users",
        "madhyamas_create_user",
        "madhyamas_delete_user",
        "madhyamas_update_user_role",
    ] {
        assert_eq!(
            permission_of(by_name(name)),
            Some(JWT_ONLY_PERMISSION),
            "{name} backs a JWT-only REST route"
        );
    }
}

#[test]
fn tool_annotations_permission_sets_required_scope() {
    let ann = ToolAnnotations::permission("mocks:write");
    assert_eq!(ann.required_permission.as_deref(), Some("mocks:write"));
    assert!(ann.read_only.is_none());
    assert!(ann.destructive.is_none());
    assert!(ann.idempotent.is_none());
}

#[test]
fn tool_annotations_public_has_no_required_permission() {
    let ann = ToolAnnotations::public();
    assert_eq!(ann.required_permission, None);
    assert_eq!(ann.read_only, Some(true));
}

#[test]
fn tool_annotations_serde_roundtrip_keeps_permission_field() {
    let ann = ToolAnnotations::permission("traffic:export");
    let json = serde_json::to_value(&ann).unwrap();
    assert_eq!(json["required_permission"], "traffic:export");
    // MCP-spec hint names are camelCase on the wire.
    let public = serde_json::to_value(ToolAnnotations::public()).unwrap();
    assert_eq!(public["readOnlyHint"], true);
    assert!(public.get("required_permission").is_none());
}

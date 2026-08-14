//! Enterprise MCP tools.
//!
//! These tools expose enterprise-tier API endpoints (user management,
//! audit logging, licensing, metrics, health, and config import/export)
//! as MCP tools for AI agents. They are registered conditionally by
//! [`super::enterprise_registry`] when the connected API server reports
//! an enterprise tier. Against an OSS server the endpoints return 404
//! and the tools surface the error gracefully.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError, ToolAnnotations};

// ============ User Management ============

/// List all users (enterprise tier).
pub struct ListUsersTool;

#[async_trait::async_trait]
impl McpTool for ListUsersTool {
    fn name(&self) -> &str {
        "madhyamas_list_users"
    }

    fn description(&self) -> &str {
        "List all registered users (enterprise tier). Requires admin permission."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(true),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: Some("users:read".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/users", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Create a new user (enterprise tier).
pub struct CreateUserTool;

#[async_trait::async_trait]
impl McpTool for CreateUserTool {
    fn name(&self) -> &str {
        "madhyamas_create_user"
    }

    fn description(&self) -> &str {
        "Create a new user account (enterprise tier). Requires admin permission."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "username": { "type": "string", "description": "Username for the new user" },
                "email": { "type": "string", "description": "Email address" },
                "password": { "type": "string", "description": "Initial password" },
                "role": {
                    "type": "string",
                    "enum": ["admin", "user", "viewer"],
                    "description": "User role"
                }
            },
            "required": ["username", "email", "password", "role"]
        })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(false),
            destructive: Some(false),
            idempotent: Some(false),
            required_permission: Some("users:write".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let username = arguments
            .get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("username is required".to_string()))?;
        let email = arguments
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("email is required".to_string()))?;
        let password = arguments
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("password is required".to_string()))?;
        let role = arguments
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("role is required".to_string()))?;
        let body = json!({
            "username": username,
            "email": email,
            "password": password,
            "role": role
        });
        let resp = client
            .post(format!("{}/api/users", api_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Delete a user (enterprise tier).
pub struct DeleteUserTool;

#[async_trait::async_trait]
impl McpTool for DeleteUserTool {
    fn name(&self) -> &str {
        "madhyamas_delete_user"
    }

    fn description(&self) -> &str {
        "Delete a user account by ID (enterprise tier). Requires admin permission."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "The ID of the user to delete" }
            },
            "required": ["id"]
        })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(false),
            destructive: Some(true),
            idempotent: Some(true),
            required_permission: Some("users:delete".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let resp = client
            .delete(format!("{}/api/users/{}", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Update a user's role (enterprise tier).
pub struct UpdateUserRoleTool;

#[async_trait::async_trait]
impl McpTool for UpdateUserRoleTool {
    fn name(&self) -> &str {
        "madhyamas_update_user_role"
    }

    fn description(&self) -> &str {
        "Update a user's role by ID (enterprise tier). Requires admin permission."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "The ID of the user to update" },
                "role": {
                    "type": "string",
                    "enum": ["admin", "user", "viewer"],
                    "description": "New role for the user"
                }
            },
            "required": ["id", "role"]
        })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(false),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: Some("users:write".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let role = arguments
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("role is required".to_string()))?;
        let body = json!({ "role": role });
        let resp = client
            .put(format!("{}/api/users/{}", api_url, id))
            .json(&body)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

// ============ Audit Logging ============

/// Get audit events (enterprise tier).
pub struct GetAuditEventsTool;

#[async_trait::async_trait]
impl McpTool for GetAuditEventsTool {
    fn name(&self) -> &str {
        "madhyamas_get_audit_events"
    }

    fn description(&self) -> &str {
        "Query audit events with optional filters (enterprise tier)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_id": { "type": "string", "description": "Filter by user ID" },
                "event_type": { "type": "string", "description": "Filter by event type" },
                "limit": { "type": "integer", "description": "Maximum results (default: 100)" },
                "offset": { "type": "integer", "description": "Pagination offset" }
            }
        })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(true),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: Some("audit:read".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let mut parts: Vec<String> = Vec::new();
        if let Some(uid) = arguments.get("user_id").and_then(|v| v.as_str()) {
            parts.push(format!("user_id={}", uid));
        }
        if let Some(et) = arguments.get("event_type").and_then(|v| v.as_str()) {
            parts.push(format!("event_types={}", et));
        }
        if let Some(limit) = arguments.get("limit").and_then(|v| v.as_u64()) {
            parts.push(format!("limit={}", limit));
        }
        if let Some(offset) = arguments.get("offset").and_then(|v| v.as_u64()) {
            parts.push(format!("offset={}", offset));
        }
        let path = if parts.is_empty() {
            "audit".to_string()
        } else {
            format!("audit?{}", parts.join("&"))
        };
        let resp = client
            .get(format!("{}/api/{}", api_url, path))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Export audit events (enterprise tier).
pub struct ExportAuditTool;

#[async_trait::async_trait]
impl McpTool for ExportAuditTool {
    fn name(&self) -> &str {
        "madhyamas_export_audit"
    }

    fn description(&self) -> &str {
        "Export all audit events (enterprise tier). Returns a JSON document."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(true),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: Some("audit:read".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/audit/export", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

// ============ License, Metrics, Health ============

/// Get license info (enterprise tier).
pub struct GetLicenseInfoTool;

#[async_trait::async_trait]
impl McpTool for GetLicenseInfoTool {
    fn name(&self) -> &str {
        "madhyamas_get_license_info"
    }

    fn description(&self) -> &str {
        "Get the current license status and details (enterprise tier)."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(true),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: None,
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/license", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get metrics (enterprise tier).
pub struct GetMetricsTool;

#[async_trait::async_trait]
impl McpTool for GetMetricsTool {
    fn name(&self) -> &str {
        "madhyamas_get_metrics"
    }

    fn description(&self) -> &str {
        "Get current performance and operational metrics (enterprise tier)."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(true),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: Some("metrics:read".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/metrics", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Get detailed health (enterprise tier).
pub struct GetHealthTool;

#[async_trait::async_trait]
impl McpTool for GetHealthTool {
    fn name(&self) -> &str {
        "madhyamas_get_health"
    }

    fn description(&self) -> &str {
        "Get detailed health status including tier, license, and dependency checks."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(true),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: None,
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/health/detailed", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

// ============ Config Import/Export ============

/// Export configuration (enterprise tier).
pub struct ExportConfigTool;

#[async_trait::async_trait]
impl McpTool for ExportConfigTool {
    fn name(&self) -> &str {
        "madhyamas_export_config"
    }

    fn description(&self) -> &str {
        "Export the full Madhyamas configuration as JSON (enterprise tier)."
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(true),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: Some("config:read".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/config/export", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

/// Import configuration (enterprise tier).
pub struct ImportConfigTool;

#[async_trait::async_trait]
impl McpTool for ImportConfigTool {
    fn name(&self) -> &str {
        "madhyamas_import_config"
    }

    fn description(&self) -> &str {
        "Import a configuration JSON document (enterprise tier)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "config_json": {
                    "type": "object",
                    "description": "The configuration JSON to import"
                }
            },
            "required": ["config_json"]
        })
    }

    fn annotations(&self) -> Option<ToolAnnotations> {
        Some(ToolAnnotations {
            read_only: Some(false),
            destructive: Some(false),
            idempotent: Some(true),
            required_permission: Some("config:write".to_string()),
        })
    }

    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let config = arguments
            .get("config_json")
            .ok_or_else(|| McpError::InvalidParams("config_json is required".to_string()))?;
        let body = json!({ "config": config });
        let resp = client
            .post(format!("{}/api/config/import", api_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
        }
        Ok(json_text(
            &json!({ "success": true, "message": "Configuration imported" }),
        ))
    }
}

/// Register all enterprise tools into the given registry.
pub fn register_enterprise_tools(reg: &mut super::DynToolRegistry) {
    reg.register(Box::new(ListUsersTool));
    reg.register(Box::new(CreateUserTool));
    reg.register(Box::new(DeleteUserTool));
    reg.register(Box::new(UpdateUserRoleTool));
    reg.register(Box::new(GetAuditEventsTool));
    reg.register(Box::new(ExportAuditTool));
    reg.register(Box::new(GetLicenseInfoTool));
    reg.register(Box::new(GetMetricsTool));
    reg.register(Box::new(GetHealthTool));
    reg.register(Box::new(ExportConfigTool));
    reg.register(Box::new(ImportConfigTool));
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut reg = super::super::DynToolRegistry::new();
        register_enterprise_tools(&mut reg);
        assert_eq!(reg.len(), 11);
    }
}

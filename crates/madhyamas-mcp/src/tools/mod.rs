//! Tool registry and executor for MCP server

mod breakpoints;
mod executor;
mod grpc;
mod mocks;
mod plugins;
mod registry;
mod replay;
mod rewrites;
mod scripts;
mod sessions;
mod throttle;
mod tool_trait;
mod traffic;
mod modern_tools;

pub use executor::ToolExecutor;
pub use modern_tools::default_registry as default_dyn_registry;
pub use registry::ToolRegistry;
pub use tool_trait::{DynToolRegistry, McpTool, tool_definition};

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

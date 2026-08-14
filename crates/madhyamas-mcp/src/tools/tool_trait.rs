//! Self-registering MCP tool trait.
//!
//! Each tool implements [`McpTool`] and is collected into a
//! [`DynToolRegistry`] via [`DynToolRegistry::register`].  The tool
//! carries both its schema and its handler in a single struct, so adding
//! a new tool requires no edits to any other file.

use reqwest::Client;
use serde_json::Value;

use crate::types::{ContentBlock, McpError, Tool, ToolAnnotations};

/// A self-describing MCP tool.
///
/// Implementors provide their name, description, JSON-schema, and an
/// async `execute` handler.  Tools are registered in
/// [`DynToolRegistry::register`] and invoked automatically by the MCP server.
#[async_trait::async_trait]
pub trait McpTool: Send + Sync {
    /// Unique tool name (e.g. `"madhyamas_get_traffic"`).
    fn name(&self) -> &str;

    /// Human-readable description shown to the AI agent.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn input_schema(&self) -> Value;

    /// Tool annotations (MCP spec hints + enterprise permission).
    ///
    /// Defaults to `None`. Override to declare `readOnlyHint`,
    /// `destructiveHint`, `idempotentHint`, and `required_permission`.
    fn annotations(&self) -> Option<ToolAnnotations> {
        None
    }

    /// Execute the tool, returning content blocks for the MCP response.
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError>;
}

/// Convert a trait object into the wire-format [`Tool`] struct.
pub fn tool_definition<T: McpTool + ?Sized>(t: &T) -> Tool {
    Tool {
        name: t.name().to_string(),
        description: t.description().to_string(),
        input_schema: t.input_schema(),
        annotations: t.annotations(),
    }
}

/// Registry of trait-based tools.
///
/// Holds `Box<dyn McpTool>` instances and provides lookup by name.
/// The MCP server queries this registry for `tools/list` and delegates
/// `tools/call` to the matching implementation.
pub struct DynToolRegistry {
    tools: Vec<Box<dyn McpTool>>,
}

impl DynToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool implementation.
    pub fn register(&mut self, tool: Box<dyn McpTool>) {
        self.tools.push(tool);
    }

    /// List all registered tools as wire-format definitions.
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .map(|t| tool_definition(t.as_ref()))
            .collect()
    }

    /// Find a tool by name and execute it.
    pub async fn execute(
        &self,
        name: &str,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Option<Result<Vec<ContentBlock>, McpError>> {
        let tool = self.tools.iter().find(|t| t.name() == name)?;
        Some(tool.execute(client, api_url, arguments).await)
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Merge all tools from `other` into this registry, consuming `other`.
    pub fn merge(&mut self, mut other: DynToolRegistry) {
        self.tools.append(&mut other.tools);
    }

    /// Check whether a tool with the given name is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name() == name)
    }
}

impl Default for DynToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

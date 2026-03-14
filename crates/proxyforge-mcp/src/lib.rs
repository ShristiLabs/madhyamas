//! ProxyForge MCP Server
//!
//! This crate implements the Model Context Protocol (MCP) server
//! that exposes ProxyForge capabilities as tools for AI agents.

pub mod server;
pub mod tools;
pub mod types;

pub use server::McpServer;
pub use types::{McpConfig, McpError};

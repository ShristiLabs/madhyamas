//! Madhyamas MCP Server
//!
//! This crate implements the Model Context Protocol (MCP) server
//! that exposes Madhyamas capabilities as tools for AI agents.

pub mod server;
pub mod tools;
pub mod types;

pub use server::McpServer;
pub use types::{McpAuth, McpConfig, McpError};

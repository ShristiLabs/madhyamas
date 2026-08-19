//! Madhyamas MCP Server
//!
//! This crate implements the Model Context Protocol (MCP) server
//! that exposes Madhyamas capabilities as tools for AI agents.

// Beta clippy (2026-08 rollout) fires `double_must_use` on #[async_trait]
// trait signatures (macro-generated boxed future is `must_use`, and so is
// the returned `Result`). Silenced until the lint behavior stabilizes.
#![allow(clippy::double_must_use, clippy::manual_clamp)]
pub mod server;
pub mod tools;
pub mod types;

pub use server::McpServer;
pub use types::{McpAuth, McpConfig, McpError, McpTransport};

//! ProxyForge MCP Server entry point
//!
//! This binary runs as an MCP server using stdio transport.

use std::env;
use std::process;

use proxyforge_mcp::{McpConfig, McpServer};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    // Initialize logging
    let level = if env::var("RUST_LOG").is_ok() {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)  // Don't include module path
        .with_thread_ids(false)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Get configuration from environment
    let api_url = env::var("PROXYFORGE_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());

    let timeout_secs: u64 = env::var("PROXYFORGE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    info!("Starting ProxyForge MCP Server");
    info!("API URL: {}", api_url);

    let config = McpConfig {
        api_url,
        timeout_secs,
    };

    let server = McpServer::new(config)
        .expect("Failed to create MCP server");

    // Run the server using stdio transport
    if let Err(e) = server.run() {
        eprintln!("MCP server error: {}", e);
        process::exit(1);
    }
}

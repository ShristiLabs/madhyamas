//! Madhyamas MCP Server entry point
//!
//! This binary runs as an MCP server using stdio transport.

use std::env;
use std::process;

use madhyamas_mcp::{McpAuth, McpConfig, McpServer, McpTransport};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Resolve MCP auth from env vars. API key takes precedence over JWT.
fn resolve_auth(api_key: &Option<String>, token: &Option<String>) -> McpAuth {
    if let Some(key) = api_key {
        return McpAuth::ApiKey(key.clone());
    }
    if let Some(t) = token {
        return McpAuth::Jwt(t.clone());
    }
    McpAuth::None
}

fn main() {
    // Initialize logging
    let level = if env::var("RUST_LOG").is_ok() {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false) // Don't include module path
        .with_thread_ids(false)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr())) // No escapes when redirected
        .with_writer(std::io::stderr) // Write to stderr to avoid corrupting stdio JSON-RPC
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Get configuration from environment
    let api_url =
        env::var("MADHYAMAS_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());

    let timeout_secs: u64 = env::var("MADHYAMAS_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    // Resolve auth from env vars (API key takes precedence over JWT).
    let api_key = env::var("MADHYAMAS_API_KEY").ok();
    let token = env::var("MADHYAMAS_TOKEN").ok();
    let auth = resolve_auth(&api_key, &token);

    info!("Starting Madhyamas MCP Server");
    info!("API URL: {}", api_url);

    let config = McpConfig {
        api_url,
        timeout_secs,
        auth,
        transport: McpTransport::Stdio,
    };

    let server = McpServer::new(config).expect("Failed to create MCP server");

    // Run the server using stdio transport
    if let Err(e) = server.run() {
        eprintln!("MCP server error: {}", e);
        process::exit(1);
    }
}

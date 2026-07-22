//! Madhyamas — Unified HTTP/HTTPS Debugging Proxy
//!
//! Single binary that combines:
//! - **Proxy server + Web UI** (`madhyamas` or `madhyamas serve`)
//! - **MCP server** (`madhyamas mcp`)
//! - **CLI commands** (`madhyamas traffic list`, `madhyamas mocks create`, etc.)
//!
//! Web UI assets are embedded at compile time — no external files needed.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;

use madhyamas_api::create_router;
use madhyamas_core::{
    BreakpointManager, CertificateManager, GrpcManager, MockManager, PluginManager, ProxyConfig,
    ProxyEngine, RewriteManager, ScriptRuntime, ThrottleManager, TrafficStore,
};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Re-export CLI commands from the madhyamas-cli library
use madhyamas_cli::Commands as CliCommands;

// Re-export MCP server from the madhyamas-mcp library
use madhyamas_mcp::{McpConfig, McpServer};

#[derive(Parser, Debug)]
#[command(name = "madhyamas")]
#[command(author = "Madhyamas Team", version, about, long_about = None)]
#[command(about = "HTTP/HTTPS Debugging Proxy — unified binary (proxy + web UI + MCP + CLI)")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    // ── Proxy server options (used when no subcommand or `serve`) ──
    /// Port for the proxy server
    #[arg(short, long, default_value = "8888", global = true)]
    proxy_port: u16,

    /// Port for the web UI API
    #[arg(short, long, default_value = "3001", global = true)]
    api_port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1", global = true)]
    host: String,

    /// Public IP address to show to users (optional)
    #[arg(long, env = "MADHYAMAS_PUBLIC_IP", global = true)]
    public_ip: Option<String>,

    /// Certificate storage path (defaults to ~/.madhyamas/certs)
    #[arg(long, global = true)]
    cert_path: Option<String>,

    /// Database path for traffic storage (defaults to ~/.madhyamas/traffic.db)
    #[arg(long, global = true)]
    db_path: Option<String>,

    /// Log file path (defaults to ~/.madhyamas/logs)
    #[arg(long, global = true)]
    log_path: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Maximum requests to keep in memory
    #[arg(long, default_value = "10000", global = true)]
    max_requests: usize,

    /// Disable HTTPS interception
    #[arg(long, global = true)]
    no_https: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the proxy server with web UI (default action)
    Serve,

    /// Run as an MCP (Model Context Protocol) server via stdio
    #[command(name = "mcp")]
    Mcp {
        /// API server URL to connect to
        #[arg(
            long,
            env = "MADHYAMAS_API_URL",
            default_value = "http://127.0.0.1:3001"
        )]
        api_url: String,

        /// Request timeout in seconds
        #[arg(long, env = "MADHYAMAS_TIMEOUT", default_value_t = 30)]
        timeout_secs: u64,
    },

    /// CLI commands for interacting with a running Madhyamas server
    #[command(flatten)]
    Cli(CliCommands),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install rustls crypto provider (required by rustls 0.23+)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls CryptoProvider");

    let args = Args::parse();

    // Initialize logging
    let level = if args.verbose || cfg!(debug_assertions) {
        Level::DEBUG
    } else {
        Level::INFO
    };

    match args.command {
        Some(Command::Mcp {
            api_url,
            timeout_secs,
        }) => {
            // MCP mode: log to stderr to avoid corrupting stdio JSON-RPC
            let subscriber = FmtSubscriber::builder()
                .with_max_level(level)
                .with_target(false)
                .with_thread_ids(false)
                .with_writer(std::io::stderr)
                .finish();
            let _ = tracing::subscriber::set_global_default(subscriber);

            info!("Starting Madhyamas MCP Server");
            info!("API URL: {}", api_url);

            let config = McpConfig {
                api_url,
                timeout_secs,
            };
            let server = McpServer::new(config).expect("Failed to create MCP server");
            if let Err(e) = server.run() {
                eprintln!("MCP server error: {}", e);
                std::process::exit(1);
            }
            Ok(())
        }

        Some(Command::Cli(cli_cmd)) => {
            // CLI mode: standard logging
            let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
            let _ = tracing::subscriber::set_global_default(subscriber);

            let api_url = std::env::var("MADHYAMAS_API_URL")
                .unwrap_or_else(|_| format!("http://{}:{}", args.host, args.api_port));
            cli_cmd.execute(api_url).await
        }

        Some(Command::Serve) | None => {
            // Proxy server mode (default)
            let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
            let _ = tracing::subscriber::set_global_default(subscriber);

            run_proxy_server(args).await
        }
    }
}

async fn run_proxy_server(args: Args) -> Result<()> {
    info!("Starting Madhyamas...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Create configuration with defaults, then override with CLI args
    let defaults = ProxyConfig::default();
    let config = ProxyConfig {
        proxy_port: args.proxy_port,
        api_port: args.api_port,
        host: args.host,
        public_ip: args.public_ip,
        cert_path: args.cert_path.unwrap_or(defaults.cert_path),
        db_path: args.db_path.unwrap_or(defaults.db_path),
        log_path: args.log_path.unwrap_or(defaults.log_path),
        verbose: args.verbose,
        max_requests: args.max_requests,
        intercept_https: !args.no_https,
        max_body_size: defaults.max_body_size,
    };

    // Ensure data directories exist
    config.ensure_directories()?;
    info!("Data directory: ~/.madhyamas/");

    // Initialize certificate manager
    let cert_manager = CertificateManager::new(&config.cert_path).await?;

    // Initialize traffic store
    let traffic_store = TrafficStore::new(config.db_path.clone())?;
    traffic_store.set_max_body_size(config.max_body_size);

    info!("Starting proxy engine...");
    let cert_manager_for_api = cert_manager.clone();

    // Create shared intercept managers
    let mock_manager = Arc::new(MockManager::new());
    let rewrite_manager = Arc::new(RewriteManager::new());
    let breakpoint_manager = Arc::new(BreakpointManager::new(100));
    let throttle_manager = Arc::new(ThrottleManager::new());
    let grpc_manager = Arc::new(GrpcManager::default());
    let script_runtime = Arc::new(ScriptRuntime::default());
    let plugin_manager = Arc::new(PluginManager::default());

    let proxy_engine = ProxyEngine::new(config.clone(), cert_manager, traffic_store.clone())
        .await?
        .with_mock_manager(mock_manager.clone())
        .with_rewrite_manager(rewrite_manager.clone())
        .with_breakpoint_manager(breakpoint_manager.clone())
        .with_throttle_manager(throttle_manager.clone())
        .with_grpc_manager(grpc_manager.clone())
        .with_script_runtime(script_runtime.clone())
        .with_plugin_manager(plugin_manager.clone());

    let proxy_engine_clone = proxy_engine.clone();
    let proxy_task = tokio::spawn(async move {
        if let Err(e) = proxy_engine_clone.start().await {
            tracing::error!("Proxy engine error: {}", e);
        }
    });

    let api_state = madhyamas_api::AppState::new(traffic_store.clone())
        .with_cert_manager(cert_manager_for_api)
        .with_proxy_config(Arc::new(config.clone()))
        .with_mock_manager(mock_manager)
        .with_rewrite_manager(rewrite_manager)
        .with_breakpoint_manager(breakpoint_manager)
        .with_throttle_manager(throttle_manager)
        .with_grpc_manager(grpc_manager)
        .with_script_runtime(script_runtime)
        .with_plugin_manager(plugin_manager);

    let app = create_router(api_state);

    let api_addr = config.api_addr();
    info!("Starting API server on {}", api_addr);

    let listener = tokio::net::TcpListener::bind(&api_addr).await?;
    info!("Madhyamas is ready!");
    info!("Proxy: http://{}", config.proxy_addr());
    info!("Web UI: http://{}", api_addr);
    info!("");
    info!("Configure your browser/app to use the proxy:");
    info!("  HTTP Proxy: {}:{}", config.host, config.proxy_port);
    info!("  HTTPS Proxy: {}:{}", config.host, config.proxy_port);
    info!("");
    info!("To intercept HTTPS, install the CA certificate:");
    info!(
        "  Certificate location: {}",
        config.ca_cert_path().display()
    );
    info!("");
    info!("Other modes:");
    info!("  MCP server:  madhyamas mcp");
    info!("  CLI:         madhyamas traffic list  (or mocks/breakpoints/sessions/...)");

    drop(proxy_task);

    axum::serve(listener, app).await?;
    Ok(())
}

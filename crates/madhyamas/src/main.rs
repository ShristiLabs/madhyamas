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

use madhyamas_api::{create_router, RateLimitConfig};
use madhyamas_core::{
    BreakpointManager, CertificateManager, ExtensionManager, InterceptStore, MemoryManager,
    MetricsCollector, MockManager, Persistable, PerformanceMonitor, ProxyConfig, ProxyEngine,
    RewriteManager, ThrottleManager, TrafficStore,
};
#[cfg(feature = "grpc")]
use madhyamas_core::GrpcManager;
#[cfg(feature = "plugins")]
use madhyamas_core::{PluginExtension, PluginManager};
#[cfg(feature = "scripting")]
use madhyamas_core::{ScriptExtension, ScriptRuntime};

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
    #[arg(
        short,
        long,
        env = "MADHYAMAS_PROXY_PORT",
        default_value = "8888",
        global = true
    )]
    proxy_port: u16,

    /// Port for the web UI API
    #[arg(
        short,
        long,
        env = "MADHYAMAS_API_PORT",
        default_value = "3001",
        global = true
    )]
    api_port: u16,

    /// Host to bind to
    #[arg(
        long,
        env = "MADHYAMAS_HOST",
        default_value = "127.0.0.1",
        global = true
    )]
    host: String,

    /// Public IP address to show to users (optional)
    #[arg(long, env = "MADHYAMAS_PUBLIC_IP", global = true)]
    public_ip: Option<String>,

    /// Certificate storage path (defaults to ~/.madhyamas/certs)
    #[arg(long, env = "MADHYAMAS_CERT_PATH", global = true)]
    cert_path: Option<String>,

    /// Database path for traffic storage (defaults to ~/.madhyamas/traffic.db)
    #[arg(long, env = "MADHYAMAS_DB_PATH", global = true)]
    db_path: Option<String>,

    /// Log file path (defaults to ~/.madhyamas/logs)
    #[arg(long, env = "MADHYAMAS_LOG_PATH", global = true)]
    log_path: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, env = "MADHYAMAS_VERBOSE", global = true)]
    verbose: bool,

    /// Maximum requests to keep in memory
    #[arg(
        long,
        env = "MADHYAMAS_MAX_REQUESTS",
        default_value = "10000",
        global = true
    )]
    max_requests: usize,

    /// Disable HTTPS interception
    #[arg(long, env = "MADHYAMAS_NO_HTTPS", global = true)]
    no_https: bool,

    /// Enable API rate limiting (disabled by default).
    ///
    /// When enabled, the API server limits requests per peer IP to
    /// --rate-limit-rps per second with a burst of --rate-limit-burst.
    /// Useful when exposing the API to a less-trusted network.
    #[arg(long, env = "MADHYAMAS_RATE_LIMIT", global = true)]
    rate_limit: bool,

    /// Rate limit: max requests per second per peer IP (only with --rate-limit).
    #[arg(
        long,
        env = "MADHYAMAS_RATE_LIMIT_RPS",
        default_value = "600",
        global = true
    )]
    rate_limit_rps: u32,

    /// Rate limit: burst size (only with --rate-limit).
    #[arg(
        long,
        env = "MADHYAMAS_RATE_LIMIT_BURST",
        default_value = "1000",
        global = true
    )]
    rate_limit_burst: u32,
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

    // Initialize intercept rule persistence store (SQLite). Rules are
    // loaded on startup and saved whenever they change via the API.
    let intercept_db_path = std::path::Path::new(&config.db_path)
        .with_file_name("intercept.db");
    let intercept_store = InterceptStore::new(&intercept_db_path)?;

    info!("Starting proxy engine...");
    let cert_manager_for_api = cert_manager.clone();

    // Create shared intercept managers, wired to the persistence store.
    // Each manager loads its persisted rules on construction.
    let mock_manager = Arc::new({
        let m = MockManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load() {
            tracing::warn!("Failed to load mock rules from store: {}", e);
        }
        m
    });
    let rewrite_manager = Arc::new({
        let m = RewriteManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load() {
            tracing::warn!("Failed to load rewrite rules from store: {}", e);
        }
        m
    });
    let breakpoint_manager = Arc::new({
        let m = BreakpointManager::new(100).with_store(intercept_store.clone());
        if let Err(e) = m.load() {
            tracing::warn!("Failed to load breakpoint rules from store: {}", e);
        }
        m
    });
    let throttle_manager = Arc::new({
        let m = ThrottleManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load() {
            tracing::warn!("Failed to load throttle profile from store: {}", e);
        }
        m
    });
    #[cfg(feature = "grpc")]
    let grpc_manager = Arc::new(GrpcManager::default());
    #[cfg(feature = "scripting")]
    let script_runtime = Arc::new(ScriptRuntime::default());
    #[cfg(feature = "plugins")]
    let plugin_manager = Arc::new(PluginManager::default());

    // Build the unified extension manager. Script and plugin runtimes are
    // registered as adapters so the pipeline can invoke them through a
    // single trait-object dispatch with priority ordering.
    let extension_manager = {
        let mgr = ExtensionManager::new();
        #[cfg(feature = "scripting")]
        mgr.register(Arc::new(ScriptExtension::new(script_runtime.clone())));
        #[cfg(feature = "plugins")]
        mgr.register(Arc::new(PluginExtension::new(plugin_manager.clone())));
        Arc::new(mgr)
    };

    // Create performance modules: metrics collection, memory management,
    // and a background performance monitor that emits alerts when thresholds
    // are exceeded.
    let metrics_collector = Arc::new(MetricsCollector::new());
    let memory_manager = Arc::new(MemoryManager::new());
    let performance_monitor = Arc::new(PerformanceMonitor::new());

    let proxy_engine = ProxyEngine::new(config.clone(), cert_manager, traffic_store.clone())
        .await?
        .with_mock_manager(mock_manager.clone())
        .with_rewrite_manager(rewrite_manager.clone())
        .with_breakpoint_manager(breakpoint_manager.clone())
        .with_throttle_manager(throttle_manager.clone())
        .with_extension_manager(extension_manager)
        .with_metrics_collector(metrics_collector)
        .with_memory_manager(memory_manager)
        .with_performance_monitor(performance_monitor);
    #[cfg(feature = "grpc")]
    let proxy_engine = proxy_engine.with_grpc_manager(grpc_manager.clone());
    #[cfg(feature = "scripting")]
    let proxy_engine = proxy_engine.with_script_runtime(script_runtime.clone());
    #[cfg(feature = "plugins")]
    let proxy_engine = proxy_engine.with_plugin_manager(plugin_manager.clone());

    let proxy_engine_clone = proxy_engine.clone();
    let proxy_task = tokio::spawn(async move {
        if let Err(e) = proxy_engine_clone.start().await {
            tracing::error!("Proxy engine error: {}", e);
        }
    });

    let api_state = madhyamas_api::AppState::new(traffic_store.clone())
        .with_cert_manager(cert_manager_for_api)
        .with_proxy_config(Arc::new(parking_lot::RwLock::new(config.clone())))
        .with_mock_manager(mock_manager)
        .with_rewrite_manager(rewrite_manager)
        .with_breakpoint_manager(breakpoint_manager)
        .with_throttle_manager(throttle_manager);
    #[cfg(feature = "grpc")]
    let api_state = api_state.with_grpc_manager(grpc_manager);
    #[cfg(feature = "scripting")]
    let api_state = api_state.with_script_runtime(script_runtime);
    #[cfg(feature = "plugins")]
    let api_state = api_state.with_plugin_manager(plugin_manager);

    let rate_limit_config = if args.rate_limit {
        RateLimitConfig::enabled(args.rate_limit_rps, args.rate_limit_burst)
    } else {
        RateLimitConfig::disabled()
    };

    let app = create_router(api_state, rate_limit_config);

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
    info!("");
    info!("Press Ctrl+C to shut down gracefully.");

    // Graceful shutdown: wait for SIGINT/SIGTERM, then drain the API server
    // and abort the proxy task so in-flight work is not abandoned abruptly.
    let shutdown = async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::warn!("signal handler error: {e}");
            return;
        }
        info!("Shutdown signal received, draining connections...");
    };

    let api_handle = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown);

    // Run both tasks concurrently. When the API server completes its graceful
    // shutdown, abort the proxy task.
    tokio::select! {
        res = api_handle => {
            if let Err(e) = res {
                tracing::error!("API server error: {e}");
            }
        }
        _ = proxy_task => {
            tracing::warn!("Proxy task exited unexpectedly");
        }
    }

    info!("Shutting down proxy engine...");
    // Best-effort: give the proxy a moment to close active connections.
    // ProxyEngine::start() owns the TCP listener; dropping the engine handle
    // closes the listener. The Arc is released when this function returns.
    drop(proxy_engine);

    info!("Madhyamas stopped. Goodbye!");
    Ok(())
}

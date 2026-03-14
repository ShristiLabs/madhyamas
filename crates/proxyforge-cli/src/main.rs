//! ProxyForge CLI entry point

use anyhow::Result;
use clap::Parser;

use proxyforge_api::create_router;
use proxyforge_core::{CertificateManager, ProxyConfig, ProxyEngine, TrafficStore};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "proxyforge")]
#[command(author, version, about)]
#[command(about = "HTTP/HTTPS Debugging Proxy")]
struct Args {
    /// Port for the proxy server
    #[arg(short, long, default_value = "8888")]
    proxy_port: u16,

    /// Port for the web UI API
    #[arg(short, long, default_value = "3001")]
    api_port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Certificate storage path (defaults to ~/.proxyforge/certs)
    #[arg(long)]
    cert_path: Option<String>,

    /// Database path for traffic storage (defaults to ~/.proxyforge/traffic.db)
    #[arg(long)]
    db_path: Option<String>,

    /// Log file path (defaults to ~/.proxyforge/logs)
    #[arg(long)]
    log_path: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Maximum requests to keep in memory
    #[arg(long, default_value = "10000")]
    max_requests: usize,

    /// Disable HTTPS interception
    #[arg(long)]
    no_https: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let level = if cfg!(debug_assertions) {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber);

    info!("Starting ProxyForge...");

    // Parse command line arguments
    let args = Args::parse();

    // Create configuration with defaults, then override with CLI args
    let defaults = ProxyConfig::default();
    let config = ProxyConfig {
        proxy_port: args.proxy_port,
        api_port: args.api_port,
        host: args.host,
        cert_path: args.cert_path.unwrap_or(defaults.cert_path),
        db_path: args.db_path.unwrap_or(defaults.db_path),
        log_path: args.log_path.unwrap_or(defaults.log_path),
        verbose: args.verbose,
        max_requests: args.max_requests,
        intercept_https: !args.no_https,
    };

    // Ensure data directories exist
    config.ensure_directories()?;
    info!("Data directory: ~/.proxyforge/");

    // Initialize certificate manager
    let cert_manager = CertificateManager::new(&config.cert_path).await?;

    // Initialize traffic store
    let traffic_store = TrafficStore::in_memory()?;

    info!("Starting proxy engine...");
    let cert_manager_for_api = cert_manager.clone();
    let proxy_engine =
        ProxyEngine::new(config.clone(), cert_manager, traffic_store.clone()).await?;

    // Clone for the proxy task
    let proxy_engine_clone = proxy_engine.clone();
    let proxy_task = tokio::spawn(async move {
        if let Err(e) = proxy_engine_clone.start().await {
            tracing::error!("Proxy engine error: {}", e);
        }
    });

    // Create API state with certificate manager for cert download
    let api_state = proxyforge_api::AppState::new(traffic_store.clone())
        .with_cert_manager(cert_manager_for_api);

    // Create the API router
    let app = create_router(api_state);

    // Start the API server
    let api_addr = config.api_addr();
    info!("Starting API server on {}", api_addr);

    let listener = tokio::net::TcpListener::bind(&api_addr).await?;
    info!("ProxyForge is ready!");
    info!("Proxy: http://{}", config.proxy_addr());
    info!("Web UI: http://{}", api_addr);
    info!("");
    info!("Configure your browser/app to use the proxy:");
    info!("  HTTP Proxy: {}:{}", config.host, config.proxy_port);
    info!("  HTTPS Proxy: {}:{}", config.host, config.proxy_port);
    info!("");
    info!("To intercept HTTPS, install the CA certificate:");
    info!("  Certificate location: {}", config.ca_cert_path().display());

    // Drop the proxy_task handle since we don't need to join it
    // (it runs until the program exits)
    drop(proxy_task);

    axum::serve(listener, app).await?;
    Ok(())
}

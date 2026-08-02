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
#[cfg(feature = "grpc")]
use madhyamas_core::GrpcManager;
use madhyamas_core::{
    AutoSaveManager, BlockListManager, BreakpointManager, CertificateManager, ExtensionManager,
    InterceptStore, MemoryManager, MetricsCollector, MirrorWriter, MockManager, PerformanceMonitor,
    Persistable, ProxyConfig, ProxyEngine, RewriteManager, SessionManager, ThrottleManager,
    TrafficStore, UpstreamProxyConfig,
};
#[cfg(feature = "plugins")]
use madhyamas_core::{PluginExtension, PluginManager};
#[cfg(feature = "scripting")]
use madhyamas_core::{ScriptExtension, ScriptRuntime};

use tracing::{debug, info, Level};
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

    /// Enable the SOCKS5 proxy listener (RFC 1928). When enabled, Madhyamas
    /// also listens on a SOCKS5 port (see --socks-port) in addition to the
    /// HTTP/HTTPS proxy port. SOCKS5 is a blind TCP tunnel — HTTPS traffic
    /// sent over SOCKS cannot be MITM-intercepted; use the HTTP proxy port
    /// with CONNECT for HTTPS interception.
    #[arg(long, env = "MADHYAMAS_ENABLE_SOCKS", global = true)]
    enable_socks: bool,

    /// Port for the SOCKS5 proxy listener (only used with --enable-socks).
    /// Defaults to 1080 (the conventional SOCKS port).
    #[arg(long, env = "MADHYAMAS_SOCKS_PORT", global = true)]
    socks_port: Option<u16>,

    /// Username for SOCKS5 username/password authentication (RFC 1929).
    /// When set together with --socks-password, the SOCKS listener requires
    /// clients to authenticate. Omit for no-auth (open) SOCKS.
    #[arg(long, env = "MADHYAMAS_SOCKS_USERNAME", global = true)]
    socks_username: Option<String>,

    /// Password for SOCKS5 username/password authentication. Ignored unless
    /// --socks-username is also set.
    #[arg(long, env = "MADHYAMAS_SOCKS_PASSWORD", global = true)]
    socks_password: Option<String>,

    /// Enable upstream (external) proxy chaining. When set, all outbound
    /// traffic is forwarded through the upstream proxy specified by
    /// --upstream-proxy. This is essential for corporate networks with a
    /// mandatory egress proxy.
    #[arg(long, env = "MADHYAMAS_UPSTREAM_PROXY_ENABLED", global = true)]
    upstream_proxy_enabled: bool,

    /// Upstream proxy host:port (e.g. `corp-proxy.example.com:8080`).
    /// When --upstream-proxy-enabled is set, this must be provided.
    /// The protocol is determined by --upstream-protocol (default: http).
    #[arg(long, env = "MADHYAMAS_UPSTREAM_PROXY", global = true)]
    upstream_proxy: Option<String>,

    /// Upstream proxy protocol: `http`, `https`, or `socks5`.
    /// Defaults to `http`. `https` uses a TLS-wrapped HTTP proxy (works
    /// for HTTP forwarding via reqwest; not supported for raw TCP
    /// tunneling). `socks5` uses SOCKS5 (RFC 1928/1929).
    #[arg(long, env = "MADHYAMAS_UPSTREAM_PROTOCOL", global = true)]
    upstream_protocol: Option<String>,

    /// Upstream proxy Basic-auth credentials in `username:password` format.
    /// For SOCKS5, this uses RFC 1929 username/password authentication.
    #[arg(long, env = "MADHYAMAS_UPSTREAM_AUTH", global = true)]
    upstream_auth: Option<String>,

    /// Comma-separated list of hosts/CIDRs to bypass the upstream proxy
    /// (e.g. `localhost,127.0.0.0/8,*.internal.corp`). Matching is
    /// case-insensitive; supports suffix matching and CIDR notation.
    #[arg(long, env = "MADHYAMAS_UPSTREAM_NO_PROXY", global = true)]
    upstream_no_proxy: Option<String>,

    /// IP address or CIDR range to allow connections from (repeatable).
    ///
    /// When one or more `--allowed-ip` flags are provided, only connections
    /// from the listed IPs/ranges are accepted by the proxy (and SOCKS5
    /// listener). Loopback addresses (`127.0.0.1`, `::1`) are always
    /// allowed regardless of this list, so a locally-started proxy can
    /// never be locked out.
    ///
    /// Examples:
    /// - `--allowed-ip 192.168.1.0/24` (allow a subnet)
    /// - `--allowed-ip 10.0.0.5 --allowed-ip 10.0.0.6` (allow specific IPs)
    /// - `--allowed-ip fd00::/8` (allow an IPv6 range)
    ///
    /// When omitted, connections from any address are allowed (default).
    /// Can also be set via the `MADHYAMAS_ALLOWED_IPS` environment variable
    /// as a comma-separated list. Runtime updates via the API
    /// (`PATCH /api/config` with `allowed_ips`) take effect immediately
    /// for new connections and persist across restarts.
    #[arg(
        long,
        env = "MADHYAMAS_ALLOWED_IPS",
        global = true,
        value_delimiter = ','
    )]
    allowed_ip: Vec<String>,

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

/// Build the [`UpstreamProxyConfig`] from CLI args, falling back to the
/// saved config when CLI flags are not provided.
///
/// Precedence (per field):
/// 1. CLI flag (if set)
/// 2. Saved config (if present)
/// 3. Default
///
/// The `--upstream-proxy` flag accepts `host:port` and is split into the
/// `host` and `port` fields. The `--upstream-auth` flag accepts
/// `username:password` and is split into `auth_username`/`auth_password`.
/// The `--upstream-no-proxy` flag is a comma-separated list.
fn build_upstream_proxy_config(args: &Args, saved: &Option<ProxyConfig>) -> UpstreamProxyConfig {
    let default = UpstreamProxyConfig::default();
    let saved_upstream = saved.as_ref().map(|s| &s.upstream_proxy);

    // Determine enabled state: CLI flag OR saved config (so runtime API
    // changes persist). If neither is set, defaults to false.
    let enabled = if args.upstream_proxy_enabled {
        true
    } else {
        saved_upstream.map(|u| u.enabled).unwrap_or(default.enabled)
    };

    // Parse host:port from --upstream-proxy, or fall back to saved/default.
    let (host, port) = if let Some(ref proxy_str) = args.upstream_proxy {
        parse_host_port(proxy_str).unwrap_or((String::new(), 0))
    } else {
        (
            saved_upstream
                .map(|u| u.host.clone())
                .unwrap_or(default.host.clone()),
            saved_upstream.map(|u| u.port).unwrap_or(default.port),
        )
    };

    // Protocol: CLI flag > saved > default
    let protocol = args
        .upstream_protocol
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .or_else(|| saved_upstream.map(|u| u.protocol.clone()))
        .unwrap_or(default.protocol);

    // Auth: CLI flag > saved > default
    let (auth_username, auth_password) = if let Some(ref auth_str) = args.upstream_auth {
        parse_auth_credentials(auth_str)
    } else {
        (
            saved_upstream.and_then(|u| u.auth_username.clone()),
            saved_upstream.and_then(|u| u.auth_password.clone()),
        )
    };

    // No-proxy hosts: CLI flag > saved > default
    let no_proxy_hosts = if let Some(ref no_proxy_str) = args.upstream_no_proxy {
        no_proxy_str
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        saved_upstream
            .map(|u| u.no_proxy_hosts.clone())
            .unwrap_or(default.no_proxy_hosts)
    };

    UpstreamProxyConfig {
        enabled,
        protocol,
        host,
        port,
        auth_username,
        auth_password,
        no_proxy_hosts,
    }
}

/// Parse a `host:port` string into its components.
fn parse_host_port(s: &str) -> Option<(String, u16)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Handle IPv6 addresses in brackets: [::1]:8080
    if let Some(rest) = s.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let host = &rest[..end];
            let after = &rest[end + 1..];
            if let Some(port_str) = after.strip_prefix(':') {
                let port = port_str.parse::<u16>().ok()?;
                return Some((host.to_string(), port));
            }
        }
    }
    // Regular host:port
    if let Some((host, port_str)) = s.rsplit_once(':') {
        let port = port_str.parse::<u16>().ok()?;
        return Some((host.to_string(), port));
    }
    None
}

/// Parse `username:password` into its components.
fn parse_auth_credentials(s: &str) -> (Option<String>, Option<String>) {
    if let Some((user, pass)) = s.split_once(':') {
        (Some(user.to_string()), Some(pass.to_string()))
    } else {
        (Some(s.to_string()), None)
    }
}

async fn run_proxy_server(args: Args) -> Result<()> {
    info!("Starting Madhyamas...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Load the persisted config (if it exists) as the base, then override
    // with CLI args. This ensures that runtime config changes made via the
    // API (e.g. passthrough domains, max_body_size, intercept_https) survive
    // restarts. CLI args always take precedence over saved values so users
    // can temporarily override settings without editing the config file.
    let saved = ProxyConfig::load_saved();
    let defaults = ProxyConfig::default();
    // Compute the upstream proxy config before the struct construction
    // because several `args.*` fields are moved by `unwrap_or_else` below.
    let upstream_proxy = build_upstream_proxy_config(&args, &saved);
    let config = ProxyConfig {
        proxy_port: args.proxy_port,
        api_port: args.api_port,
        host: args.host,
        public_ip: args.public_ip,
        cert_path: args.cert_path.unwrap_or_else(|| {
            saved
                .as_ref()
                .map(|s| s.cert_path.clone())
                .unwrap_or(defaults.cert_path)
        }),
        db_path: args.db_path.unwrap_or_else(|| {
            saved
                .as_ref()
                .map(|s| s.db_path.clone())
                .unwrap_or(defaults.db_path)
        }),
        log_path: args.log_path.unwrap_or_else(|| {
            saved
                .as_ref()
                .map(|s| s.log_path.clone())
                .unwrap_or(defaults.log_path)
        }),
        verbose: args.verbose,
        max_requests: args.max_requests,
        intercept_https: !args.no_https,
        // These fields have no CLI arg — preserve the saved value if present,
        // otherwise use the default. This is what makes runtime API changes
        // (passthrough_domains, max_body_size) persist across restarts.
        max_body_size: saved
            .as_ref()
            .map(|s| s.max_body_size)
            .unwrap_or(defaults.max_body_size),
        max_total_size_mb: saved
            .as_ref()
            .and_then(|s| s.max_total_size_mb)
            .or(defaults.max_total_size_mb),
        capture_request_bodies: saved
            .as_ref()
            .map(|s| s.capture_request_bodies)
            .unwrap_or(defaults.capture_request_bodies),
        capture_response_bodies: saved
            .as_ref()
            .map(|s| s.capture_response_bodies)
            .unwrap_or(defaults.capture_response_bodies),
        ignored_domains: saved
            .as_ref()
            .map(|s| s.ignored_domains.clone())
            .unwrap_or(defaults.ignored_domains),
        passthrough_domains: saved
            .as_ref()
            .map(|s| s.passthrough_domains.clone())
            .unwrap_or(defaults.passthrough_domains),
        enable_h2_downstream: saved
            .as_ref()
            .map(|s| s.enable_h2_downstream)
            .unwrap_or(defaults.enable_h2_downstream),
        // SOCKS5 listener: --enable-socks turns it on; the port/auth fields
        // fall back to the saved config (if any) so runtime API changes to
        // SOCKS auth persist across restarts, then to CLI args, then defaults.
        enable_socks: args.enable_socks
            || saved
                .as_ref()
                .map(|s| s.enable_socks)
                .unwrap_or(defaults.enable_socks),
        socks_port: Some(args.socks_port.unwrap_or_else(|| {
            saved
                .as_ref()
                .and_then(|s| s.socks_port)
                .unwrap_or(defaults.socks_port.unwrap_or(1080))
        })),
        socks_auth_username: args
            .socks_username
            .clone()
            .or_else(|| saved.as_ref().and_then(|s| s.socks_auth_username.clone())),
        socks_auth_password: args
            .socks_password
            .clone()
            .or_else(|| saved.as_ref().and_then(|s| s.socks_auth_password.clone())),
        // Upstream proxy chaining: CLI flags take precedence over saved
        // config. When --upstream-proxy-enabled is not set, fall back to
        // the saved config (if any) so runtime API changes persist.
        upstream_proxy,
        // IP access control (allowlist): CLI --allowed-ip flags take
        // precedence over the saved config. When no CLI flags are provided,
        // fall back to the saved config so runtime API changes to
        // `allowed_ips` persist across restarts. An empty CLI list with no
        // saved config means "allow all" (the default).
        allowed_ips: if !args.allowed_ip.is_empty() {
            args.allowed_ip.clone()
        } else {
            saved
                .as_ref()
                .map(|s| s.allowed_ips.clone())
                .unwrap_or_default()
        },
        // Auto Save: preserve the saved config (if any) so runtime API
        // changes persist across restarts. Defaults to disabled when no
        // saved config exists.
        auto_save: saved
            .as_ref()
            .map(|s| s.auto_save.clone())
            .unwrap_or_default(),
        // Mirror: preserve the saved config (if any) so runtime API changes
        // persist across restarts. Defaults to disabled when no saved config
        // exists.
        mirror: saved.as_ref().map(|s| s.mirror.clone()).unwrap_or_default(),
    };

    if saved.is_some() {
        info!(
            "Loaded saved config from {}",
            ProxyConfig::config_file_path().display()
        );
    } else {
        debug!("No saved config found, using defaults");
    }

    // Ensure data directories exist
    config.ensure_directories()?;
    info!("Data directory: ~/.madhyamas/");

    // Validate the IP allowlist early so a bad entry fails fast at startup
    // rather than silently rejecting (or accepting) connections.
    if config.access_control_enabled() {
        match config.access_control_list() {
            Ok(acl) => {
                info!(
                    "IP access control enabled: {} entr{} (loopback always allowed)",
                    acl.len(),
                    if acl.len() == 1 { "y" } else { "ies" }
                );
            }
            Err(e) => {
                anyhow::bail!("Invalid allowed_ips configuration: {}", e);
            }
        }
    } else {
        debug!("IP access control disabled (allow all connections)");
    }

    // Initialize certificate manager
    let cert_manager = CertificateManager::new(&config.cert_path).await?;

    // Initialize traffic store
    let traffic_store = TrafficStore::new(config.db_path.clone())?;
    traffic_store.set_max_body_size(config.max_body_size);
    traffic_store.set_max_entries(config.max_requests);
    if let Some(mb) = config.max_total_size_mb {
        traffic_store.set_max_total_size_bytes(mb * 1024 * 1024);
    }
    traffic_store.set_capture_request_bodies(config.capture_request_bodies);
    traffic_store.set_capture_response_bodies(config.capture_response_bodies);
    traffic_store.set_ignored_domains(config.ignored_domains.clone());

    // Initialize intercept rule persistence store (SQLite). Rules are
    // loaded on startup and saved whenever they change via the API.
    let intercept_db_path = std::path::Path::new(&config.db_path).with_file_name("intercept.db");
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
    let block_list_manager = Arc::new({
        let m = BlockListManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load() {
            tracing::warn!("Failed to load block list entries from store: {}", e);
        }
        m
    });
    #[cfg(feature = "grpc")]
    let grpc_manager = Arc::new(GrpcManager::default());
    #[cfg(feature = "scripting")]
    let script_runtime = {
        let runtime = ScriptRuntime::default();
        // Attach SQLite persistence so scripts and execution history
        // survive restarts. We open a second connection to the same
        // database file used by TrafficStore (WAL mode is already
        // enabled there, so concurrent access is safe). Loaded scripts
        // are registered with the runtime before it is shared with the
        // proxy pipeline and API layer.
        match rusqlite::Connection::open(&config.db_path) {
            Ok(conn) => {
                // Match TrafficStore pragmas for safe concurrent access.
                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                let _ = conn.execute_batch("PRAGMA synchronous=NORMAL;");
                let conn = Arc::new(parking_lot::Mutex::new(conn));
                if let Err(e) = runtime.with_persistence(conn) {
                    tracing::warn!("Failed to attach script persistence: {}", e);
                } else {
                    let count = runtime.get_scripts().len();
                    info!("Loaded {} persisted script(s) from database", count);
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to open script persistence database at {}: {}",
                    config.db_path,
                    e
                );
            }
        }
        Arc::new(runtime)
    };
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

    // Create a single shared config that both the proxy engine and the API
    // layer reference. This allows runtime config changes made via the API
    // (e.g. adding SSL passthrough domains in the web UI) to take effect
    // immediately in the proxy engine without a restart.
    let shared_config = Arc::new(parking_lot::RwLock::new(config.clone()));

    // Create the Auto Save manager. The manager holds its own shared config
    // so the API layer can update it live (PATCH /api/autosave). The manager
    // is only started (background task) when auto_save.enabled is true.
    let session_manager = Arc::new(SessionManager::new(traffic_store.clone()));
    let autosave_manager = AutoSaveManager::new(
        config.auto_save.clone(),
        traffic_store.clone(),
        session_manager.clone(),
    );

    // Create the mirror writer. The writer holds its own shared config so the
    // API layer can update it live (PATCH /api/mirror/config). The writer is
    // registered with the traffic store so captured responses are written to
    // disk asynchronously after being stored in the database.
    let mirror_writer = MirrorWriter::new(config.mirror.clone());

    let proxy_engine = ProxyEngine::new(shared_config.clone(), cert_manager, traffic_store.clone())
        .await?
        .with_mock_manager(mock_manager.clone())
        .with_rewrite_manager(rewrite_manager.clone())
        .with_breakpoint_manager(breakpoint_manager.clone())
        .with_throttle_manager(throttle_manager.clone())
        .with_block_list_manager(block_list_manager.clone())
        .with_extension_manager(extension_manager)
        .with_metrics_collector(metrics_collector)
        .with_memory_manager(memory_manager)
        .with_performance_monitor(performance_monitor)
        .with_auto_save_manager(autosave_manager.clone())
        .with_mirror_writer(mirror_writer.clone());
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
        .with_proxy_config(shared_config)
        .with_session_manager(session_manager)
        .with_autosave_manager(autosave_manager)
        .with_mirror_writer(mirror_writer)
        .with_mock_manager(mock_manager)
        .with_rewrite_manager(rewrite_manager)
        .with_breakpoint_manager(breakpoint_manager)
        .with_throttle_manager(throttle_manager)
        .with_block_list_manager(block_list_manager);
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
    if config.enable_socks {
        info!(
            "  SOCKS5 Proxy: {}:{}{}",
            config.host,
            config.socks_port(),
            if config.socks_auth_enabled() {
                " (auth required)"
            } else {
                ""
            }
        );
    }
    if config.upstream_proxy_active() {
        info!(
            "  Upstream proxy: {}://{}:{}{}",
            config.upstream_proxy.protocol,
            config.upstream_proxy.host,
            config.upstream_proxy.port,
            if config.upstream_proxy.auth_enabled() {
                " (auth required)"
            } else {
                ""
            }
        );
        if !config.upstream_proxy.no_proxy_hosts.is_empty() {
            info!(
                "  Upstream bypass: {}",
                config.upstream_proxy.no_proxy_hosts.join(", ")
            );
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_host_port_regular() {
        let (host, port) = parse_host_port("proxy.example.com:8080").unwrap();
        assert_eq!(host, "proxy.example.com");
        assert_eq!(port, 8080);
    }

    #[test]
    fn parse_host_port_ip() {
        let (host, port) = parse_host_port("127.0.0.1:1080").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 1080);
    }

    #[test]
    fn parse_host_port_ipv6_bracketed() {
        let (host, port) = parse_host_port("[::1]:1080").unwrap();
        assert_eq!(host, "::1");
        assert_eq!(port, 1080);
    }

    #[test]
    fn parse_host_port_trims_whitespace() {
        let (host, port) = parse_host_port("  proxy.example.com:8080  ").unwrap();
        assert_eq!(host, "proxy.example.com");
        assert_eq!(port, 8080);
    }

    #[test]
    fn parse_host_port_missing_port_returns_none() {
        assert!(parse_host_port("proxy.example.com").is_none());
    }

    #[test]
    fn parse_host_port_invalid_port_returns_none() {
        assert!(parse_host_port("proxy.example.com:abc").is_none());
    }

    #[test]
    fn parse_host_port_empty_returns_none() {
        assert!(parse_host_port("").is_none());
    }

    #[test]
    fn parse_auth_credentials_with_password() {
        let (user, pass) = parse_auth_credentials("alice:secret");
        assert_eq!(user.as_deref(), Some("alice"));
        assert_eq!(pass.as_deref(), Some("secret"));
    }

    #[test]
    fn parse_auth_credentials_without_password() {
        let (user, pass) = parse_auth_credentials("alice");
        assert_eq!(user.as_deref(), Some("alice"));
        assert!(pass.is_none());
    }

    #[test]
    fn parse_auth_credentials_empty_password() {
        let (user, pass) = parse_auth_credentials("alice:");
        assert_eq!(user.as_deref(), Some("alice"));
        assert_eq!(pass.as_deref(), Some(""));
    }
}

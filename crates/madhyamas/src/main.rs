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
use std::str::FromStr;
use std::sync::Arc;

use madhyamas_api::{create_router, RateLimitConfig};
#[cfg(feature = "grpc")]
use madhyamas_core::GrpcManager;
#[cfg(all(feature = "plugins", feature = "wasm-runtime"))]
use madhyamas_core::WasmRuntime;
use madhyamas_core::{
    AutoSaveManager, BlockListManager, BreakpointManager, CertificateManager, ExtensionManager,
    InterceptStoreBackend, LogHandle, MemoryManager, MetricsCollector, MirrorWriter, MockManager,
    PerformanceMonitor, Persistable, PostgresInterceptStore, PostgresTrafficStore, ProxyConfig,
    ProxyEngine, RewriteManager, RotatingFileWriter, SessionManager, SqliteInterceptStore,
    ThrottleManager, TrafficStore, TrafficStoreBackend, UpstreamProxyConfig,
};
#[cfg(feature = "plugins")]
use madhyamas_core::{
    PluginExtension, PluginInstaller, PluginManager, PluginStoreBackend, PostgresPluginStore,
    SqlitePluginStore,
};
#[cfg(feature = "scripting")]
use madhyamas_core::{
    PostgresScriptStore, ScriptExtension, ScriptRuntime, ScriptStoreBackend, SqliteScriptStore,
};

use tracing::{debug, info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

// Re-export CLI commands from the madhyamas-cli library
use madhyamas_cli::{CliAuth, Commands as CliCommands};

// Re-export MCP server from the madhyamas-mcp library
use madhyamas_mcp::{McpAuth, McpConfig, McpServer, McpTransport};

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

    /// Enable enterprise authentication (JWT + API keys). Only effective
    /// when the binary is built with the `enterprise` feature (the default).
    /// In the OSS build this flag is accepted but has no effect.
    #[arg(long, env = "MADHYAMAS_ENABLE_AUTH", global = true)]
    enable_auth: bool,

    /// JWT secret used for signing/validating tokens. If not provided, a
    /// default development secret is used. **Change this in production.**
    /// The secret is never logged.
    #[arg(long, env = "MADHYAMAS_JWT_SECRET", global = true)]
    jwt_secret: Option<String>,

    /// Path to an Ed25519-signed license file (enterprise tier). When
    /// provided, the license is verified at startup and the binary fails
    /// fast if the signature is invalid or the license has expired. When
    /// omitted, the binary runs in unlicensed enterprise mode (auth/RBAC/
    /// audit still functional). The verifying public key is read from
    /// `MADHYAMAS_LICENSE_PUBLIC_KEY` (base64-encoded 32 bytes).
    #[arg(long, env = "MADHYAMAS_LICENSE_FILE", global = true)]
    license_file: Option<String>,

    /// Bootstrap admin username (enterprise tier). On first run — when the
    /// users table is empty — an admin user is created with this username.
    /// Defaults to `admin` if neither the flag nor the env var is set.
    #[arg(long, env = "MADHYAMAS_ADMIN_USERNAME", global = true)]
    admin_username: Option<String>,

    /// Bootstrap admin password (enterprise tier). On first run, the admin
    /// user is created with this password. If neither this flag nor the env
    /// var is set, a random password is generated and logged once with a
    /// warning to change it immediately. The password is never logged when
    /// provided via this flag.
    #[arg(long, env = "MADHYAMAS_ADMIN_PASSWORD", global = true)]
    admin_password: Option<String>,

    /// Database URL for storage backends. When set to a PostgreSQL URL
    /// (e.g. `postgres://user:pass@host:5432/db`), all storage backends
    /// (traffic, intercept, config, plugins, scripts, enterprise) use
    /// PostgreSQL instead of SQLite. When omitted or set to a `sqlite://`
    /// URL, the default SQLite backends are used (existing behavior).
    #[arg(long, env = "MADHYAMAS_DATABASE_URL", global = true)]
    database_url: Option<String>,

    /// Redis URL for cross-instance state synchronization (enterprise tier,
    /// multi-instance mode). When provided, the binary connects to Redis and
    /// enables pub/sub event broadcasting (WebSocket traffic events, config
    /// changes, intercept rule changes) and license seat coordination across
    /// instances. When omitted, the binary runs in single-instance mode (all
    /// multi-instance features disabled — current behavior).
    ///
    /// Accepted URL schemes:
    /// - `redis://host:port` — plain TCP
    /// - `redis://:password@host:port` — auth
    /// - `rediss://host:port` — TLS
    /// - `rediss://:password@host:port` — TLS + auth
    #[arg(long, env = "MADHYAMAS_REDIS_URL", global = true)]
    redis_url: Option<String>,

    /// Path to a PEM-encoded CA certificate file for HTTPS interception.
    /// When set together with `--ca-key-file`, the CA is loaded from these
    /// files instead of being generated fresh. If the files do not exist
    /// yet, a new CA is generated and written to them so other instances
    /// can load the same CA. For multi-instance deployments, share the same
    /// CA across instances via a Kubernetes Secret or shared volume.
    #[arg(long, env = "MADHYAMAS_CA_CERT_FILE", global = true)]
    ca_cert_file: Option<String>,

    /// Path to a PEM-encoded CA private key file for HTTPS interception.
    /// Used together with `--ca-cert-file` to load or store the shared CA.
    /// For multi-instance deployments, share the same CA key across
    /// instances via a Kubernetes Secret or shared volume.
    #[arg(long, env = "MADHYAMAS_CA_KEY_FILE", global = true)]
    ca_key_file: Option<String>,

    /// Base path for serving the API and web UI (load-balancer context-path
    /// routing). When set to e.g. `/madhyamas`, all routes are served under
    /// `/madhyamas/api/...`, `/madhyamas/health`, `/madhyamas/ws`, and the
    /// web UI at `/madhyamas/`. Defaults to `/` (root path, unchanged
    /// behavior). The path is normalized to not have a trailing slash.
    #[arg(long, env = "MADHYAMAS_BASE_PATH", global = true)]
    base_path: Option<String>,

    /// API key for authenticating MCP/CLI requests against an enterprise
    /// API server with `--enable-auth`. Sent as the `X-API-Key` header.
    /// Overrides the `MADHYAMAS_API_KEY` environment variable. When both
    /// `--api-key` and `--token` are provided, the API key takes
    /// precedence. In OSS mode (or when auth is disabled) this is accepted
    /// but ignored by the API server.
    #[arg(long, env = "MADHYAMAS_API_KEY", global = true)]
    api_key: Option<String>,

    /// JWT token for authenticating MCP/CLI requests against an enterprise
    /// API server with `--enable-auth`. Sent as the
    /// `Authorization: Bearer <token>` header. Overrides the
    /// `MADHYAMAS_TOKEN` environment variable. When both `--api-key` and
    /// `--token` are provided, the API key takes precedence. In OSS mode
    /// (or when auth is disabled) this is accepted but ignored by the API
    /// server.
    #[arg(long, env = "MADHYAMAS_TOKEN", global = true)]
    token: Option<String>,

    /// Require authentication for proxy CONNECT/HTTP requests (Phase 9.6).
    /// When enabled (enterprise tier with `--enable-auth`), proxy clients
    /// must supply credentials via `Proxy-Authorization: Basic` or
    /// `X-API-Key` header. Unauthenticated proxy requests receive
    /// `407 Proxy Authentication Required`. Default: off (proxy is open).
    /// This is for deployments where the proxy itself needs protection,
    /// not just the API.
    #[arg(long, env = "MADHYAMAS_PROXY_AUTH", global = true)]
    proxy_auth: bool,

    /// Expected instance ID for license replay prevention (Phase 9.14).
    /// When provided, the license file's `instance_id` must match this
    /// value or the license is rejected at startup. When omitted, any
    /// `instance_id` in the license is accepted (single-instance mode).
    /// In multi-instance deployments, set a unique instance ID per
    /// instance to prevent a license issued for one instance from being
    /// reused on another.
    #[arg(long, env = "MADHYAMAS_INSTANCE_ID", global = true)]
    instance_id: Option<String>,

    /// Path to a file containing the database URL (Phase 9.15). Reads the
    /// first non-empty line as the connection string. Useful for secret
    /// managers (Vault, AWS Secrets Manager) that write credentials to
    /// files. When both `--database-url` and `--database-url-file` are
    /// provided, `--database-url` takes precedence. The connection string
    /// is never logged.
    #[arg(long, env = "MADHYAMAS_DATABASE_URL_FILE", global = true)]
    database_url_file: Option<String>,

    /// Path to a PEM-encoded CA certificate for Redis TLS verification
    /// (Phase 9.2). When `--redis-url` uses the `rediss://` scheme and
    /// this flag is set, the custom CA is used to verify the Redis TLS
    /// certificate (for self-signed Redis TLS). When omitted, the system
    /// CA store is used for `rediss://` verification.
    #[arg(long, env = "MADHYAMAS_REDIS_CA_CERT", global = true)]
    redis_ca_cert: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the proxy server with web UI (default action)
    Serve,

    /// Run as an MCP (Model Context Protocol) server via stdio or HTTP
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

        /// Transport mode: stdio (default) or http
        #[arg(long, env = "MADHYAMAS_MCP_TRANSPORT", default_value = "stdio")]
        transport: String,

        /// Port for HTTP transport (only used with --transport http)
        #[arg(long, env = "MADHYAMAS_MCP_PORT", default_value_t = 3002)]
        mcp_port: u16,
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
            transport,
            mcp_port,
        }) => {
            // MCP mode: log to stderr to avoid corrupting stdio JSON-RPC
            let subscriber = fmt::Subscriber::builder()
                .with_max_level(level)
                .with_target(false)
                .with_thread_ids(false)
                .with_writer(std::io::stderr)
                .finish();
            let _ = tracing::subscriber::set_global_default(subscriber);

            info!("Starting Madhyamas MCP Server");
            info!("API URL: {}", api_url);

            let auth = resolve_mcp_auth(&args.api_key, &args.token);
            let mcp_transport = match transport.as_str() {
                "http" => McpTransport::Http { port: mcp_port },
                _ => McpTransport::Stdio,
            };
            let config = McpConfig {
                api_url,
                timeout_secs,
                auth,
                transport: mcp_transport,
            };
            let server = McpServer::new(config).expect("Failed to create MCP server");
            match server.transport() {
                McpTransport::Stdio => {
                    if let Err(e) = server.run() {
                        eprintln!("MCP server error: {}", e);
                        std::process::exit(1);
                    }
                }
                McpTransport::Http { port } => {
                    info!("MCP HTTP transport on port {}", port);
                    if let Err(e) = server.run_http(port) {
                        eprintln!("MCP server error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Ok(())
        }

        Some(Command::Cli(cli_cmd)) => {
            // CLI mode: standard logging
            let subscriber = fmt::Subscriber::builder().with_max_level(level).finish();
            let _ = tracing::subscriber::set_global_default(subscriber);

            let api_url = std::env::var("MADHYAMAS_API_URL")
                .unwrap_or_else(|_| format!("http://{}:{}", args.host, args.api_port));
            let auth = resolve_cli_auth(&args.api_key, &args.token);
            cli_cmd.execute(api_url, auth).await
        }

        Some(Command::Serve) | None => {
            // Proxy server mode (default).
            //
            // Initialize logging early so the proxy server startup messages
            // are captured. The log directory and rotation config are
            // resolved from CLI args / saved config / defaults (same
            // precedence as the full ProxyConfig built inside
            // run_proxy_server).
            let saved = ProxyConfig::load_saved();
            let defaults = ProxyConfig::default();
            let log_path = args.log_path.clone().unwrap_or_else(|| {
                saved
                    .as_ref()
                    .map(|s| s.log_path.clone())
                    .unwrap_or(defaults.log_path.clone())
            });
            let log_config = saved
                .as_ref()
                .map(|s| s.log_config.clone())
                .unwrap_or_default();
            let log_handle = init_logging(log_path, log_config, args.verbose);

            run_proxy_server(args, log_handle).await
        }
    }
}

/// Resolve MCP authentication credentials from CLI flags / env vars.
///
/// Precedence (matches the enterprise auth middleware's acceptance order):
/// 1. `--api-key` (or `MADHYAMAS_API_KEY` env var) → [`McpAuth::ApiKey`]
/// 2. `--token` (or `MADHYAMAS_TOKEN` env var) → [`McpAuth::Jwt`]
/// 3. Neither set → [`McpAuth::None`] (OSS mode or auth disabled)
///
/// When both are provided, the API key takes precedence. clap already
/// populates the `Option`s from their env vars, so the caller passes the
/// resolved values directly.
fn resolve_mcp_auth(api_key: &Option<String>, token: &Option<String>) -> McpAuth {
    if let Some(key) = api_key {
        return McpAuth::ApiKey(key.clone());
    }
    if let Some(t) = token {
        return McpAuth::Jwt(t.clone());
    }
    McpAuth::None
}

/// Resolve CLI authentication credentials from CLI flags / env vars.
///
/// Same precedence as [`resolve_mcp_auth`]: API key > JWT > none.
fn resolve_cli_auth(api_key: &Option<String>, token: &Option<String>) -> CliAuth {
    if let Some(key) = api_key {
        return CliAuth::ApiKey(key.clone());
    }
    if let Some(t) = token {
        return CliAuth::Jwt(t.clone());
    }
    CliAuth::None
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

/// Resolve the database URL from CLI args (Phase 9.15).
///
/// Precedence:
/// 1. `--database-url` (or `MADHYAMAS_DATABASE_URL` env var)
/// 2. `--database-url-file` (or `MADHYAMAS_DATABASE_URL_FILE` env var) —
///    reads the first non-empty line from the file
/// 3. `None` (use default SQLite)
///
/// The connection string is never logged by this function.
fn resolve_database_url(
    database_url: &Option<String>,
    database_url_file: &Option<String>,
) -> Result<Option<String>> {
    if let Some(url) = database_url {
        return Ok(Some(url.clone()));
    }
    if let Some(ref path) = database_url_file {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read --database-url-file {path}: {e}"))?;
        let url = contents
            .lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .ok_or_else(|| anyhow::anyhow!("--database-url-file {path} is empty"))?;
        return Ok(Some(url.to_string()));
    }
    Ok(None)
}

/// Redact the password from a database connection string for safe logging
/// (Phase 9.15). Handles `postgres://user:pass@host/db`,
/// `postgresql://user:pass@host/db`, and `sqlite://path` (no password to
/// redact). Returns a string with the password replaced by `***`, or the
/// original string if no password is present.
fn redact_db_url(url: &str) -> String {
    // Try to parse as a URL. If parsing fails, return as-is (don't risk
    // leaking — but also don't crash).
    match url::Url::parse(url) {
        Ok(mut parsed) => {
            if parsed.password().is_some() {
                let _ = parsed.set_password(Some("***"));
            }
            parsed.to_string()
        }
        Err(_) => {
            // Not a valid URL (e.g. sqlite://path). Return as-is — SQLite
            // URLs don't contain passwords.
            url.to_string()
        }
    }
}

#[cfg(test)]
mod tests_url_redact {
    use super::*;

    #[test]
    fn test_redact_db_url_postgres_with_password() {
        let redacted = redact_db_url("postgres://user:secretpass@localhost:5432/db");
        assert!(redacted.contains("***"));
        assert!(!redacted.contains("secretpass"));
    }

    #[test]
    fn test_redact_db_url_postgresql_with_password() {
        let redacted = redact_db_url("postgresql://admin:p@ssw0rd@db.example.com:5432/prod");
        assert!(redacted.contains("***"));
        assert!(!redacted.contains("p@ssw0rd"));
    }

    #[test]
    fn test_redact_db_url_no_password() {
        let redacted = redact_db_url("postgres://user@localhost:5432/db");
        assert!(!redacted.contains("***"));
        assert!(redacted.contains("localhost"));
    }

    #[test]
    fn test_redact_db_url_sqlite() {
        let url = "sqlite:///home/user/.madhyamas/traffic.db";
        let redacted = redact_db_url(url);
        assert_eq!(redacted, url);
    }

    #[test]
    fn test_resolve_database_url_prefers_direct() {
        let result =
            resolve_database_url(&Some("postgres://localhost/db".to_string()), &None).unwrap();
        assert_eq!(result.as_deref(), Some("postgres://localhost/db"));
    }

    #[test]
    fn test_resolve_database_url_from_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_db_url_file.txt");
        std::fs::write(&path, "postgres://user:pass@localhost/db\n").unwrap();
        let result = resolve_database_url(&None, &Some(path.display().to_string())).unwrap();
        assert_eq!(result.as_deref(), Some("postgres://user:pass@localhost/db"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_resolve_database_url_none() {
        let result = resolve_database_url(&None, &None).unwrap();
        assert!(result.is_none());
    }
}

async fn run_proxy_server(args: Args, log_handle: LogHandle) -> Result<()> {
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
        // Log rotation: preserve the saved config (if any) so runtime API
        // changes persist across restarts. Defaults to enabled with daily
        // rotation when no saved config exists.
        log_config: saved
            .as_ref()
            .map(|s| s.log_config.clone())
            .unwrap_or_default(),
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

    // Phase 9.15: resolve the database URL from --database-url or
    // --database-url-file. The resolved URL is used for all storage
    // backends. The connection string is never logged in plaintext —
    // use `redact_db_url` when logging is needed.
    let database_url = resolve_database_url(&args.database_url, &args.database_url_file)?;
    if let Some(ref db_url) = database_url {
        tracing::info!("Database URL resolved: {}", redact_db_url(db_url));
    }

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

    // Initialize certificate manager. When --ca-cert-file and --ca-key-file
    // are both provided, the CA is loaded from (or generated and written to)
    // those files so multiple instances can share the same CA (Phase 6b).
    let cert_manager = CertificateManager::new_with_ca_files(
        &config.cert_path,
        args.ca_cert_file.as_deref(),
        args.ca_key_file.as_deref(),
    )
    .await?;

    // Initialize traffic store. When --database-url points to a PostgreSQL
    // instance, use PostgresTrafficStore; otherwise fall back to the default
    // SQLite TrafficStore.
    let traffic_store: Arc<dyn TrafficStoreBackend + Send + Sync> =
        if let Some(ref db_url) = database_url {
            if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
                info!("Using PostgreSQL traffic store: {}", redact_db_url(db_url));
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(10)
                    .connect(db_url)
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to connect to PostgreSQL: {e}"))?;
                // Run schema migrations under a PostgreSQL advisory lock to
                // prevent concurrent migration races in multi-instance setups.
                run_pg_migrations(&pool).await?;
                let store = PostgresTrafficStore::new(pool).await?;
                store.set_max_body_size(config.max_body_size);
                store.set_max_entries(config.max_requests);
                if let Some(mb) = config.max_total_size_mb {
                    store.set_max_total_size_bytes(mb * 1024 * 1024);
                }
                store.set_capture_request_bodies(config.capture_request_bodies);
                store.set_capture_response_bodies(config.capture_response_bodies);
                store.set_ignored_domains(config.ignored_domains.clone());
                store
            } else {
                info!(
                    "Using SQLite traffic store (explicit URL): {}",
                    redact_db_url(db_url)
                );
                let store = TrafficStore::new(config.db_path.clone()).await?;
                configure_traffic_store(&store, &config);
                store
            }
        } else {
            let store = TrafficStore::new(config.db_path.clone()).await?;
            configure_traffic_store(&store, &config);
            store
        };

    // Initialize intercept rule persistence store. When using PostgreSQL,
    // the intercept store shares the same database; otherwise it uses a
    // separate SQLite file (intercept.db).
    let intercept_store: Arc<dyn InterceptStoreBackend + Send + Sync> =
        if let Some(ref db_url) = database_url {
            if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(5)
                    .connect(db_url)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("failed to connect to PostgreSQL for intercept: {e}")
                    })?;
                Arc::new(PostgresInterceptStore::new(pool).await?)
            } else {
                let intercept_db_path =
                    std::path::Path::new(&config.db_path).with_file_name("intercept.db");
                let intercept_db_url = format!("sqlite://{}", intercept_db_path.display());
                let intercept_connect_options =
                    sqlx::sqlite::SqliteConnectOptions::from_str(&intercept_db_url)
                        .map_err(|e| anyhow::anyhow!("failed to parse intercept db url: {e}"))?
                        .create_if_missing(true);
                let intercept_pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(intercept_connect_options)
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to open intercept db: {e}"))?;
                Arc::new(SqliteInterceptStore::new(intercept_pool).await?)
            }
        } else {
            let intercept_db_path =
                std::path::Path::new(&config.db_path).with_file_name("intercept.db");
            let intercept_db_url = format!("sqlite://{}", intercept_db_path.display());
            let intercept_connect_options =
                sqlx::sqlite::SqliteConnectOptions::from_str(&intercept_db_url)
                    .map_err(|e| anyhow::anyhow!("failed to parse intercept db url: {e}"))?
                    .create_if_missing(true);
            let intercept_pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(intercept_connect_options)
                .await
                .map_err(|e| anyhow::anyhow!("failed to open intercept db: {e}"))?;
            Arc::new(SqliteInterceptStore::new(intercept_pool).await?)
        };

    info!("Starting proxy engine...");
    let cert_manager_for_api = cert_manager.clone();

    // Create shared intercept managers, wired to the persistence store.
    // Each manager loads its persisted rules on construction.
    let mock_manager = Arc::new({
        let m = MockManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load().await {
            tracing::warn!("Failed to load mock rules from store: {}", e);
        }
        m
    });
    let rewrite_manager = Arc::new({
        let m = RewriteManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load().await {
            tracing::warn!("Failed to load rewrite rules from store: {}", e);
        }
        m
    });
    let breakpoint_manager = Arc::new({
        let m = BreakpointManager::new(100).with_store(intercept_store.clone());
        if let Err(e) = m.load().await {
            tracing::warn!("Failed to load breakpoint rules from store: {}", e);
        }
        m
    });
    let throttle_manager = Arc::new({
        let m = ThrottleManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load().await {
            tracing::warn!("Failed to load throttle profile from store: {}", e);
        }
        m
    });
    let block_list_manager = Arc::new({
        let m = BlockListManager::new().with_store(intercept_store.clone());
        if let Err(e) = m.load().await {
            tracing::warn!("Failed to load block list entries from store: {}", e);
        }
        m
    });
    #[cfg(feature = "grpc")]
    let grpc_manager = Arc::new(GrpcManager::default());
    #[cfg(feature = "scripting")]
    let script_runtime = {
        let runtime = ScriptRuntime::default();
        // Attach async script store so scripts and execution history
        // survive restarts. When using PostgreSQL, the script store shares
        // the same database; otherwise it uses a SQLite pool to the same
        // database file used by TrafficStore.
        let script_store_result: Result<Arc<dyn ScriptStoreBackend + Send + Sync>, anyhow::Error> =
            if let Some(ref db_url) = database_url {
                if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
                    let pool = sqlx::postgres::PgPoolOptions::new()
                        .max_connections(5)
                        .connect(db_url)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("failed to connect to PostgreSQL for scripts: {e}")
                        })?;
                    PostgresScriptStore::new(pool)
                        .await
                        .map(|s| {
                            let store: Arc<dyn ScriptStoreBackend + Send + Sync> = Arc::new(s);
                            store
                        })
                        .map_err(|e| anyhow::anyhow!("failed to init script store: {e}"))
                } else {
                    let script_db_url = format!("sqlite://{}", config.db_path);
                    let script_opts = sqlx::sqlite::SqliteConnectOptions::from_str(&script_db_url)
                        .map(|opts| opts.create_if_missing(true))
                        .map_err(|e| anyhow::anyhow!("failed to parse script db url: {e}"))?;
                    let pool = sqlx::sqlite::SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect_with(script_opts)
                        .await
                        .map_err(|e| anyhow::anyhow!("failed to open script db: {e}"))?;
                    SqliteScriptStore::new(pool)
                        .await
                        .map(|s| {
                            let store: Arc<dyn ScriptStoreBackend + Send + Sync> = Arc::new(s);
                            store
                        })
                        .map_err(|e| anyhow::anyhow!("failed to init script store: {e}"))
                }
            } else {
                let script_db_url = format!("sqlite://{}", config.db_path);
                let script_opts = sqlx::sqlite::SqliteConnectOptions::from_str(&script_db_url)
                    .map(|opts| opts.create_if_missing(true))
                    .map_err(|e| anyhow::anyhow!("failed to parse script db url: {e}"))?;
                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(script_opts)
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to open script db: {e}"))?;
                SqliteScriptStore::new(pool)
                    .await
                    .map(|s| {
                        let store: Arc<dyn ScriptStoreBackend + Send + Sync> = Arc::new(s);
                        store
                    })
                    .map_err(|e| anyhow::anyhow!("failed to init script store: {e}"))
            };
        match script_store_result {
            Ok(store) => {
                if let Err(e) = runtime.with_persistence(store).await {
                    tracing::warn!("Failed to attach script persistence: {}", e);
                } else {
                    let count = runtime.get_scripts().len();
                    info!("Loaded {} persisted script(s) from database", count);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to initialize script store: {}", e);
            }
        }
        Arc::new(runtime)
    };
    #[cfg(feature = "plugins")]
    let plugin_manager = {
        // Derive the plugin install + persistence paths from the data
        // directory (same folder as traffic.db).
        let data_dir = std::path::Path::new(&config.db_path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        let plugins_dir = data_dir.join("plugins");
        std::fs::create_dir_all(&plugins_dir).ok();
        let plugins_db = data_dir.join("plugins.db");

        let mut mgr = PluginManager::new();

        // Attach async plugin store (state, settings, invocation logs).
        // When using PostgreSQL, the plugin store shares the same database;
        // otherwise it uses a separate SQLite file (plugins.db).
        let plugin_store_result: Result<Arc<dyn PluginStoreBackend + Send + Sync>, anyhow::Error> =
            if let Some(ref db_url) = database_url {
                if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
                    let pool = sqlx::postgres::PgPoolOptions::new()
                        .max_connections(5)
                        .connect(db_url)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("failed to connect to PostgreSQL for plugins: {e}")
                        })?;
                    PostgresPluginStore::new(pool)
                        .await
                        .map(|s| {
                            let store: Arc<dyn PluginStoreBackend + Send + Sync> = Arc::new(s);
                            store
                        })
                        .map_err(|e| anyhow::anyhow!("failed to init plugin store: {e}"))
                } else {
                    let plugins_db_url = format!("sqlite://{}", plugins_db.display());
                    let plugin_opts = sqlx::sqlite::SqliteConnectOptions::from_str(&plugins_db_url)
                        .map(|opts| opts.create_if_missing(true))
                        .map_err(|e| anyhow::anyhow!("failed to parse plugins db url: {e}"))?;
                    let pool = sqlx::sqlite::SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect_with(plugin_opts)
                        .await
                        .map_err(|e| anyhow::anyhow!("failed to open plugins db: {e}"))?;
                    SqlitePluginStore::new(pool)
                        .await
                        .map(|s| {
                            let store: Arc<dyn PluginStoreBackend + Send + Sync> = Arc::new(s);
                            store
                        })
                        .map_err(|e| anyhow::anyhow!("failed to init plugin store: {e}"))
                }
            } else {
                let plugins_db_url = format!("sqlite://{}", plugins_db.display());
                let plugin_opts = sqlx::sqlite::SqliteConnectOptions::from_str(&plugins_db_url)
                    .map(|opts| opts.create_if_missing(true))
                    .map_err(|e| anyhow::anyhow!("failed to parse plugins db url: {e}"))?;
                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(plugin_opts)
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to open plugins db: {e}"))?;
                SqlitePluginStore::new(pool)
                    .await
                    .map(|s| {
                        let store: Arc<dyn PluginStoreBackend + Send + Sync> = Arc::new(s);
                        store
                    })
                    .map_err(|e| anyhow::anyhow!("failed to init plugin store: {e}"))
            };
        match plugin_store_result {
            Ok(store) => {
                info!("Plugin persistence opened");
                mgr = mgr.with_persistence(store);
            }
            Err(e) => {
                tracing::warn!("Failed to initialize plugin store: {}", e);
            }
        }

        // Attach the installer (download, checksum verify, zip extract).
        let installer = Arc::new(PluginInstaller::new(plugins_dir.clone()));
        mgr = mgr.with_installer(installer);

        // Attach the WASM runtime (enables plugin code execution).
        #[cfg(feature = "wasm-runtime")]
        {
            match WasmRuntime::new() {
                Ok(rt) => {
                    info!("WASM runtime initialized for plugins");
                    mgr = mgr.with_wasm_runtime(Arc::new(rt));
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize WASM runtime: {}", e);
                }
            }
        }

        // Discover and load any plugins already on disk.
        if let Err(e) = mgr.refresh().await {
            tracing::warn!("Initial plugin discovery failed: {}", e);
        }

        Arc::new(mgr)
    };

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

    // Clone the metrics collector for the Phase 6e background cluster metrics
    // task (runs inside the enterprise block below). The original is moved
    // into the proxy engine; this clone is captured for periodic Redis updates.
    #[cfg(feature = "enterprise")]
    let metrics_collector_for_redis = metrics_collector.clone();

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
        .with_log_handle(log_handle)
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

    // Enterprise tier: construct EnterpriseState, inject the three trait
    // impls (AuthProvider, Authorizer, AuditSink) into AppState, and build
    // the enterprise router to merge under /api. This entire block is
    // compiled out in the OSS build (--no-default-features), so no
    // enterprise code is linked.
    #[cfg(feature = "enterprise")]
    let (api_state, enterprise_router, redis_state_for_shutdown) = {
        let jwt_secret = args.jwt_secret.clone().unwrap_or_else(|| {
            tracing::warn!(
                "No --jwt-secret provided; using default development secret. \
                 Set --jwt-secret or MADHYAMAS_JWT_SECRET in production."
            );
            madhyamas_enterprise::AuthConfig::default().jwt_secret
        });
        let auth_config = madhyamas_enterprise::AuthConfig {
            enabled: args.enable_auth,
            jwt_secret: jwt_secret.clone(),
            ..madhyamas_enterprise::AuthConfig::default()
        };
        // Phase 3: Ed25519 license verification. If --license-file is
        // provided, verify it at startup (fail-fast on invalid/expired/
        // tampered licenses). If no license file is provided, the binary
        // starts in unlicensed enterprise mode — auth/RBAC/audit still
        // function; seat-count enforcement and feature gating arrive in
        // later phases.
        let license: Option<madhyamas_enterprise::License> =
            if let Some(ref license_path) = args.license_file {
                let mut verifier = madhyamas_enterprise::LicenseVerifier::from_env()
                    .map_err(|e| anyhow::anyhow!("license verifier init failed: {e}"))?;
                // Phase 9.14: instance ID replay prevention. When
                // --instance-id is provided, the license's instance_id must
                // match.
                if let Some(ref expected_id) = args.instance_id {
                    tracing::info!(
                        "License instance ID enforcement enabled (expected: {})",
                        expected_id
                    );
                    verifier = verifier.with_expected_instance_id(expected_id.clone());
                }
                let license = verifier
                    .verify(std::path::Path::new(license_path))
                    .map_err(|e| {
                        tracing::error!("License verification failed: {e}");
                        anyhow::anyhow!("license verification failed: {e}")
                    })?;
                tracing::info!(
                    "License verified: {} (plan={}, seats={}, expires={})",
                    license.claims.license_id,
                    license.claims.plan,
                    license.claims.seats,
                    license.claims.expires_at
                );
                Some(license)
            } else {
                tracing::info!(
                    "No license file provided; running in unlicensed enterprise mode \
                     (auth/RBAC/audit still functional)"
                );
                None
            };
        // Construct the persistent enterprise store. When using PostgreSQL,
        // the enterprise store shares the same database; otherwise it uses
        // a separate SQLite file (enterprise.db) alongside traffic.db.
        let store: std::sync::Arc<dyn madhyamas_enterprise::EnterpriseStore> =
            if let Some(ref db_url) = database_url {
                if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
                    tracing::info!(
                        "Enterprise store using PostgreSQL: {}",
                        redact_db_url(db_url)
                    );
                    let pool = sqlx::postgres::PgPoolOptions::new()
                        .max_connections(5)
                        .connect(db_url)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("failed to connect to PostgreSQL for enterprise: {e}")
                        })?;
                    let store = madhyamas_enterprise::PostgresEnterpriseStore::new(pool)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("failed to initialize enterprise store: {e}")
                        })?;
                    std::sync::Arc::new(store)
                } else {
                    let enterprise_db_path =
                        std::path::Path::new(&config.db_path).with_file_name("enterprise.db");
                    let enterprise_db_dir = enterprise_db_path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    std::fs::create_dir_all(&enterprise_db_dir).ok();
                    let db_url = format!("sqlite://{}", enterprise_db_path.display());
                    let connect_options = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)
                        .map_err(|e| anyhow::anyhow!("failed to parse enterprise db url: {e}"))?
                        .create_if_missing(true);
                    let pool = sqlx::sqlite::SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect_with(connect_options)
                        .await
                        .map_err(|e| anyhow::anyhow!("failed to open enterprise db: {e}"))?;
                    tracing::info!(
                        "Enterprise store opened at {}",
                        enterprise_db_path.display()
                    );
                    let store = madhyamas_enterprise::SqliteEnterpriseStore::new(pool)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("failed to initialize enterprise store: {e}")
                        })?;
                    std::sync::Arc::new(store)
                }
            } else {
                let enterprise_db_path =
                    std::path::Path::new(&config.db_path).with_file_name("enterprise.db");
                let enterprise_db_dir = enterprise_db_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                std::fs::create_dir_all(&enterprise_db_dir).ok();
                let db_url = format!("sqlite://{}", enterprise_db_path.display());
                let connect_options = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)
                    .map_err(|e| anyhow::anyhow!("failed to parse enterprise db url: {e}"))?
                    .create_if_missing(true);
                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(connect_options)
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to open enterprise db: {e}"))?;
                tracing::info!(
                    "Enterprise store opened at {}",
                    enterprise_db_path.display()
                );
                let store = madhyamas_enterprise::SqliteEnterpriseStore::new(pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to initialize enterprise store: {e}"))?;
                std::sync::Arc::new(store)
            };
        // Phase 4a.5: bootstrap admin user on first run (empty users table).
        bootstrap_admin_user(
            &store,
            args.admin_username.clone(),
            args.admin_password.clone(),
        )
        .await?;
        // Phase 6a/6c: Redis cross-instance state coordination. When
        // --redis-url is provided, connect to Redis and enable pub/sub event
        // broadcasting + license seat tracking. When omitted, multi-instance
        // features are disabled (single-instance mode).
        let instance_id = uuid::Uuid::new_v4().to_string();
        let redis_state: Option<std::sync::Arc<madhyamas_enterprise::RedisState>> =
            if let Some(ref redis_url) = args.redis_url {
                tracing::info!(
                    "Connecting to Redis for multi-instance state: {}",
                    redis_url
                );
                let state = madhyamas_enterprise::RedisState::new(redis_url, instance_id.clone())
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to connect to Redis: {e}"))?;
                tracing::info!("Redis connected (instance_id={})", instance_id);
                Some(std::sync::Arc::new(state))
            } else {
                tracing::info!(
                    "No --redis-url provided; running in single-instance mode \
                     (multi-instance features disabled)"
                );
                None
            };
        // Phase 6c: license seat coordination. When both --license-file and
        // --redis-url are provided, register this instance in Redis and check
        // the active instance count against the license seat limit.
        if let (Some(ref rs), Some(ref lic)) = (&redis_state, &license) {
            let addr = format!("{}:{}", config.host, config.api_port);
            let license_id = lic.claims.license_id.clone();
            rs.register_instance(&instance_id, &license_id, &addr)
                .await
                .map_err(|e| anyhow::anyhow!("failed to register instance in Redis: {e}"))?;
            let active = rs
                .active_instance_count()
                .await
                .map_err(|e| anyhow::anyhow!("failed to query active instance count: {e}"))?;
            let seats = lic.claims.seats as usize;
            tracing::info!(
                "License seat check: {active} active instances, license allows {seats} seats"
            );
            if active > seats {
                if let Err(e) = rs.deregister_instance(&instance_id).await {
                    tracing::warn!("failed to deregister instance: {e}");
                }
                return Err(anyhow::anyhow!(
                    "license seat limit exceeded: {active} active instances, license allows {seats} seats"
                ));
            }
            // Start heartbeat task (every 60s, refresh instance score + TTL).
            let rs_heartbeat = rs.clone();
            let hb_id = instance_id.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.tick().await; // skip immediate tick
                loop {
                    interval.tick().await;
                    if let Err(e) = rs_heartbeat.heartbeat(&hb_id).await {
                        tracing::warn!("Redis heartbeat failed: {e}");
                    }
                }
            });
            // Phase 6e: start background cluster metrics task (every 30s).
            // Collects local metrics from the MetricsCollector and updates
            // the instance's metrics snapshot in Redis so /api/metrics/cluster
            // can aggregate across all instances.
            let rs_metrics = rs.clone();
            let metrics_id = instance_id.clone();
            let metrics_mc = metrics_collector_for_redis.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                interval.tick().await; // skip immediate tick
                loop {
                    interval.tick().await;
                    let snap = metrics_mc.snapshot();
                    let instance_metrics = madhyamas_enterprise::InstanceMetrics {
                        cpu_usage: 0.0, // CPU usage not tracked locally; future work
                        memory_usage_mb: 0,
                        active_connections: snap.active_connections,
                        request_count: snap.request_count,
                        uptime_secs: snap.uptime_secs,
                    };
                    if let Err(e) = rs_metrics
                        .update_instance_metrics(&metrics_id, &instance_metrics)
                        .await
                    {
                        tracing::warn!("Redis metrics update failed: {e}");
                    }
                }
            });
        }
        let enterprise = madhyamas_enterprise::EnterpriseState::new(auth_config)
            .with_store(store.clone())
            .with_license(license)
            .with_redis(redis_state.clone());
        // Wire the store into AuthManager (for API key validation) and
        // AuditLogger (for persistent audit events + hash chain).
        let auth = std::sync::Arc::new(
            madhyamas_enterprise::AuthManager::new(madhyamas_enterprise::AuthConfig {
                enabled: args.enable_auth,
                require_auth: args.enable_auth,
                jwt_secret: jwt_secret.clone(),
                ..madhyamas_enterprise::AuthConfig::default()
            })
            .with_store(store.clone()),
        );
        let audit = std::sync::Arc::new(
            madhyamas_enterprise::AuditLogger::default().with_store(store.clone()),
        );
        // Phase 9.6: attach proxy auth validator when --proxy-auth is
        // enabled. The proxy engine is already running (started earlier),
        // but the validator is stored in a OnceLock that hasn't been set
        // yet — so this takes effect immediately for all subsequent
        // connections.
        if args.proxy_auth {
            tracing::info!("Proxy auth enabled: CONNECT/HTTP requests require credentials");
            // with_proxy_auth_validator sets a OnceLock on the underlying
            // ProxyEngine (shared via Arc). The returned Arc is dropped.
            let _ = proxy_engine.clone().with_proxy_auth_validator(auth.clone());
        }
        let api_state = api_state
            .with_auth_provider(auth.clone())
            .with_authorizer(enterprise.rbac.clone())
            .with_audit_sink(audit.clone());
        // Wire the Redis event publisher into AppState so API handlers can
        // publish config/intercept change notifications cross-instance.
        let api_state = if let Some(ref rs) = redis_state {
            api_state.with_event_publisher(rs.clone())
        } else {
            api_state
        };
        // Phase 6a: start Redis pub/sub bridge tasks for cross-instance
        // WebSocket event broadcasting, config propagation, and intercept
        // rule sync. These are no-ops when redis_state is None.
        if let Some(ref rs) = redis_state {
            // WS event bridge: local broadcast → Redis publish.
            let rs_pub = rs.clone();
            let local_sender = api_state.traffic_store.event_sender();
            let mut local_rx = api_state.traffic_store.subscribe();
            let pub_instance_id = instance_id.clone();
            tokio::spawn(async move {
                loop {
                    match local_rx.recv().await {
                        Ok(event) => {
                            let wrapper = madhyamas_enterprise::RedisTrafficEvent {
                                instance_id: pub_instance_id.clone(),
                                event,
                            };
                            if let Ok(json) = serde_json::to_string(&wrapper) {
                                if let Err(e) =
                                    rs_pub.publish(madhyamas_core::CHANNEL_EVENTS, &json).await
                                {
                                    tracing::warn!("Redis event publish failed: {e}");
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Redis bridge lagged behind by {n} local events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::debug!("Local event channel closed, stopping Redis bridge");
                            break;
                        }
                    }
                }
            });
            // WS event bridge: Redis subscribe → local broadcast.
            let rs_sub = rs.clone();
            let local_sender_clone = local_sender;
            let sub_instance_id = instance_id.clone();
            tokio::spawn(async move {
                use futures::StreamExt;
                loop {
                    match rs_sub.subscribe(madhyamas_core::CHANNEL_EVENTS).await {
                        Ok(mut stream) => {
                            while let Some(msg) = stream.next().await {
                                if let Ok(payload) = msg.get_payload::<String>() {
                                    if let Ok(wrapper) =
                                        serde_json::from_str::<
                                            madhyamas_enterprise::RedisTrafficEvent,
                                        >(&payload)
                                    {
                                        if wrapper.instance_id == sub_instance_id {
                                            continue;
                                        }
                                        let _ = local_sender_clone.send(wrapper.event);
                                    }
                                }
                            }
                            tracing::warn!("Redis events stream ended, reconnecting...");
                        }
                        Err(e) => {
                            tracing::warn!("Redis events subscribe failed: {e}");
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });
            // Config change subscriber: on notification, log (reload from
            // shared store is future work — config is currently local file +
            // in-memory).
            let rs_cfg = rs.clone();
            tokio::spawn(async move {
                use futures::StreamExt;
                loop {
                    match rs_cfg.subscribe(madhyamas_core::CHANNEL_CONFIG_EVENT).await {
                        Ok(mut stream) => {
                            while let Some(msg) = stream.next().await {
                                if let Ok(payload) = msg.get_payload::<String>() {
                                    tracing::info!(
                                        "Config change notification from Redis: {payload}"
                                    );
                                }
                            }
                            tracing::debug!("Redis config stream ended, reconnecting...");
                        }
                        Err(e) => {
                            tracing::warn!("Redis config subscribe failed: {e}");
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });
            // Intercept rule change subscriber: on notification, reload all
            // intercept rules from the shared store so this instance picks up
            // changes made on other instances.
            let rs_int = rs.clone();
            let int_mock = api_state.mock_manager.clone();
            let int_rewrite = api_state.rewrite_manager.clone();
            let int_breakpoint = api_state.breakpoint_manager.clone();
            let int_throttle = api_state.throttle_manager.clone();
            let int_block_list = api_state.block_list_manager.clone();
            tokio::spawn(async move {
                use futures::StreamExt;
                use madhyamas_core::Persistable;
                loop {
                    match rs_int
                        .subscribe(madhyamas_core::CHANNEL_INTERCEPT_EVENT)
                        .await
                    {
                        Ok(mut stream) => {
                            while let Some(msg) = stream.next().await {
                                if let Ok(payload) = msg.get_payload::<String>() {
                                    tracing::info!(
                                        "Intercept rule change notification from Redis: {payload}; reloading rules"
                                    );
                                    if let Err(e) = int_mock.load().await {
                                        tracing::warn!("Failed to reload mock rules: {e}");
                                    }
                                    if let Err(e) = int_rewrite.load().await {
                                        tracing::warn!("Failed to reload rewrite rules: {e}");
                                    }
                                    if let Err(e) = int_breakpoint.load().await {
                                        tracing::warn!("Failed to reload breakpoint rules: {e}");
                                    }
                                    if let Err(e) = int_throttle.load().await {
                                        tracing::warn!("Failed to reload throttle profile: {e}");
                                    }
                                    if let Err(e) = int_block_list.load().await {
                                        tracing::warn!("Failed to reload block list: {e}");
                                    }
                                }
                            }
                            tracing::debug!("Redis intercept stream ended, reconnecting...");
                        }
                        Err(e) => {
                            tracing::warn!("Redis intercept subscribe failed: {e}");
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });
        }
        let router = madhyamas_enterprise::create_enterprise_router(
            store,
            auth.clone(),
            audit.clone(),
            enterprise.license.clone(),
            redis_state.clone(),
        );
        (api_state, Some(router), redis_state)
    };
    #[cfg(not(feature = "enterprise"))]
    let enterprise_router: Option<axum::Router<std::sync::Arc<madhyamas_api::AppState>>> = None;
    #[cfg(not(feature = "enterprise"))]
    let _redis_state_for_shutdown: Option<()> = None;

    let rate_limit_config = if args.rate_limit {
        RateLimitConfig::enabled(args.rate_limit_rps, args.rate_limit_burst)
    } else {
        RateLimitConfig::disabled()
    };

    // Capture the WS manager before api_state is moved into create_router
    // (needed for graceful shutdown — closing all WS connections).
    let shutdown_ws = api_state.ws_manager.clone();

    let app = create_router(
        api_state,
        rate_limit_config,
        enterprise_router,
        args.base_path.as_deref().unwrap_or("/"),
    );

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
    // In multi-instance mode (Redis enabled), the instance is deregistered
    // from the Redis seat tracker so the seat is released promptly.
    // WebSocket connections are closed and audit log is flushed (Phase 6d).
    #[cfg(feature = "enterprise")]
    let shutdown_redis = redis_state_for_shutdown.clone();
    let shutdown = async move {
        // Wait for either SIGINT (ctrl_c) or SIGTERM (Unix signal).
        let sigint = tokio::signal::ctrl_c();
        #[cfg(unix)]
        let sigterm = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut s) => {
                    s.recv().await;
                }
                Err(e) => {
                    tracing::warn!("SIGTERM handler setup error: {e}");
                }
            }
        };
        #[cfg(not(unix))]
        let sigterm = std::future::pending::<()>();
        tokio::select! {
            res = sigint => {
                if let Err(e) = res {
                    tracing::warn!("signal handler error: {e}");
                    return;
                }
            }
            _ = sigterm => {}
        }
        info!("Shutdown signal received, draining connections...");
        // Close all WebSocket connections so in-flight proxy WS tunnels are
        // torn down promptly (Phase 6d).
        shutdown_ws.close_all_connections();
        // Phase 6c: deregister this instance from the Redis seat tracker so
        // the seat is released promptly (no waiting for the 120s TTL).
        #[cfg(feature = "enterprise")]
        if let Some(rs) = shutdown_redis {
            if let Err(e) = rs.deregister_instance(rs.instance_id()).await {
                tracing::warn!("Failed to deregister instance from Redis: {e}");
            } else {
                tracing::info!("Instance deregistered from Redis seat tracker");
            }
        }
        // Flush pending audit log entries (Phase 6d). The audit logger writes
        // synchronously to the store, so there is no batch to drain — log
        // confirmation for operational visibility.
        info!("Audit log flushed");
        info!("Graceful shutdown complete");
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

/// Initialize the global tracing subscriber with stdout + rotating file
/// layers.
///
/// - **Stdout layer**: always on, ANSI-formatted (or JSON when
///   `log_config.json_format` is true), so `tail`/`docker logs` work.
/// - **File layer**: on when `log_config.enabled`, writing to
///   `<log_dir>/madhyamas.log` via [`RotatingFileWriter`] (time/size-based
///   rotation + on-demand rotation).
///
/// A background `tokio` task wakes every 60 seconds to perform time-based
/// rotation (hourly/daily) and prune archived files to `max_files`.
///
/// Returns a [`LogHandle`] that must be held for the program lifetime (the
/// file appender is dropped if the handle is dropped) and is also stored in
/// the API `AppState` for on-demand rotation.
fn init_logging(
    log_dir: String,
    log_config: madhyamas_core::LogConfig,
    verbose: bool,
) -> LogHandle {
    // Build the EnvFilter: honor RUST_LOG if set, otherwise use a global
    // level (matching the previous `with_max_level` behavior — all targets
    // at the configured level, not just the `madhyamas` target).
    let level = if verbose || cfg!(debug_assertions) {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level.as_str().to_lowercase()))
        .unwrap_or_else(|_| EnvFilter::new(level.as_str().to_lowercase()));

    // Stdout fmt layer (always on).
    let stdout_layer = fmt::layer().with_target(false).with_filter(filter.clone());

    let mut layers = vec![stdout_layer.boxed()];

    // Optional file layer.
    let mut handle: Option<LogHandle> = None;
    if log_config.enabled {
        match RotatingFileWriter::new(&log_dir, log_config.clone()) {
            Ok(writer) => {
                // Print the file path to stderr directly — the global
                // subscriber is not installed yet, so tracing macros would
                // be dropped.
                let current = writer.current_path();
                eprintln!(
                    "Log file: {} (rotation: {}, max_files: {}, max_size: {} MB)",
                    current.display(),
                    log_config.rotation.label(),
                    log_config.max_files,
                    log_config
                        .rotation
                        .effective_size_cap_mb(log_config.max_file_size_mb),
                );

                // Spawn the background rotation/prune task. Holds a clone of
                // the writer (Arc-backed) so it keeps running for the
                // program lifetime.
                let bg_writer = writer.clone();
                tokio::spawn(async move {
                    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
                    // The first tick fires immediately; skip it so we don't
                    // rotate right after startup.
                    ticker.tick().await;
                    loop {
                        ticker.tick().await;
                        if let Err(e) = bg_writer.check_time_rotation() {
                            eprintln!("log time-rotation check failed: {}", e);
                        }
                        if let Err(e) = bg_writer.prune() {
                            eprintln!("log prune failed: {}", e);
                        }
                    }
                });

                let file_layer = if log_config.json_format {
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_writer(writer.clone())
                        .with_filter(filter.clone())
                        .boxed()
                } else {
                    fmt::layer()
                        .with_target(false)
                        .with_writer(writer.clone())
                        .with_filter(filter.clone())
                        .boxed()
                };
                layers.push(file_layer);
                handle = Some(LogHandle::new(writer));
            }
            Err(e) => {
                eprintln!(
                    "Warning: failed to open log file in {}: {} — logging to stdout only",
                    log_dir, e
                );
            }
        }
    } else {
        eprintln!("File logging disabled (log_config.enabled = false) — stdout only");
    }

    // Install the global subscriber.
    tracing_subscriber::registry().with(layers).init();

    // Return the handle, or a no-op handle backed by a fresh writer pointing
    // at a throwaway temp dir if file logging was disabled. The no-op handle
    // still satisfies the AppState contract (handlers can call rotate_now()
    // without crashing; it just rotates a file nobody writes to).
    handle.unwrap_or_else(|| {
        // Create a handle backed by a disabled writer so the API surface
        // remains functional even when file logging is off.
        let disabled_cfg = madhyamas_core::LogConfig {
            enabled: false,
            ..log_config
        };
        let dir = std::env::temp_dir().join("madhyamas-logs-disabled");
        let writer = RotatingFileWriter::new(&dir, disabled_cfg)
            .expect("failed to create disabled log writer in temp dir");
        LogHandle::new(writer)
    })
}

/// Apply runtime configuration (size limits, capture flags, ignored
/// domains) to a [`TrafficStore`]. This is a helper used by the SQLite
/// code path in [`run_proxy_server`].
fn configure_traffic_store(store: &TrafficStore, config: &ProxyConfig) {
    store.set_max_body_size(config.max_body_size);
    store.set_max_entries(config.max_requests);
    if let Some(mb) = config.max_total_size_mb {
        store.set_max_total_size_bytes(mb * 1024 * 1024);
    }
    store.set_capture_request_bodies(config.capture_request_bodies);
    store.set_capture_response_bodies(config.capture_response_bodies);
    store.set_ignored_domains(config.ignored_domains.clone());
}

/// Run PostgreSQL schema migrations under a transactional advisory lock.
///
/// The advisory lock (`pg_advisory_xact_lock`) prevents concurrent
/// migration attempts when multiple proxy instances start simultaneously
/// against the same database. Each store's `new()` constructor also runs
/// idempotent `CREATE TABLE IF NOT EXISTS` DDL, so this function is a
/// no-op once tables exist.
async fn run_pg_migrations(pool: &sqlx::PgPool) -> Result<()> {
    // Advisory lock key — arbitrary fixed value chosen for Madhyamas.
    // Using a transactional lock ensures it's released on commit/rollback.
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(0x4D414448)")
        .execute(&mut *tx)
        .await?;
    // The individual store constructors run their own DDL, so there's
    // nothing to do here beyond holding the lock. In the future, sqlx
    // migrations can be added here.
    tx.commit().await?;
    Ok(())
}

/// Bootstrap a default admin user on first run (empty users table).
///
/// Username comes from `--admin-username` / `MADHYAMAS_ADMIN_USERNAME`
/// (default `admin`). Password comes from `--admin-password` /
/// `MADHYAMAS_ADMIN_PASSWORD`; if neither is set a random password is
/// generated and logged once with a warning to change it. The password is
/// never logged when provided via flag/env.
#[cfg(feature = "enterprise")]
async fn bootstrap_admin_user(
    store: &std::sync::Arc<dyn madhyamas_enterprise::EnterpriseStore>,
    admin_username: Option<String>,
    admin_password: Option<String>,
) -> Result<()> {
    use madhyamas_enterprise::{hash_password, User, UserRole, UserStatus};

    let existing = store
        .list_users()
        .await
        .map_err(|e| anyhow::anyhow!("bootstrap: failed to list users: {e}"))?;
    if !existing.is_empty() {
        return Ok(());
    }
    let username = admin_username.unwrap_or_else(|| "admin".to_string());
    let (password, auto_generated) = match admin_password {
        Some(p) if !p.is_empty() => (p, false),
        _ => {
            // Generate a random 24-character password.
            use rand::Rng;
            const CHARSET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let mut rng = rand::rng();
            let password: String = (0..24)
                .map(|_| {
                    let idx = rng.random_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect();
            (password, true)
        }
    };
    let password_hash = hash_password(&password)
        .map_err(|e| anyhow::anyhow!("bootstrap: password hashing failed: {e}"))?;
    let user = User::new(
        uuid::Uuid::new_v4().to_string(),
        username.clone(),
        Some(format!("{username}@local")),
        UserRole::Admin,
        username.clone(),
        UserStatus::Active,
    );
    store
        .create_user(&user, &password_hash)
        .await
        .map_err(|e| anyhow::anyhow!("bootstrap: failed to create admin user: {e}"))?;
    if auto_generated {
        tracing::warn!(
            "Bootstrap: created admin user '{}'. \
             Auto-generated password (CHANGE IMMEDIATELY): {}",
            username,
            password
        );
    } else {
        tracing::info!("Bootstrap: created admin user '{}'", username);
    }
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

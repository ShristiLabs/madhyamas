//! Configuration for Madhyamas

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Get the default data directory for Madhyamas
/// Uses ~/.madhyamas on all platforms
fn get_data_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".madhyamas")
    } else {
        PathBuf::from(".")
    }
}

/// Main proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Port to listen on for proxy connections
    pub proxy_port: u16,

    /// Port for the web UI and API
    pub api_port: u16,

    /// Host to bind to
    pub host: String,

    /// Public IP address for external access (optional)
    /// If set, this will be shown to users instead of auto-detected IP
    /// Useful when hosting proxy on a remote server or specific network interface
    pub public_ip: Option<String>,

    /// Certificate storage path
    pub cert_path: String,

    /// Database path for traffic storage
    pub db_path: String,

    /// Log file path
    pub log_path: String,

    /// Enable verbose logging
    pub verbose: bool,

    /// Maximum requests to keep in memory
    pub max_requests: usize,

    /// Enable HTTPS interception
    pub intercept_https: bool,

    /// Maximum body size to capture in bytes (default 20 MB).
    /// Bodies larger than this are truncated when stored.
    pub max_body_size: usize,

    /// Maximum total recording size in megabytes. When the sum of all stored
    /// request/response bodies exceeds this limit, the oldest entries are
    /// pruned (FIFO). When `None`, no total-size limit is enforced.
    /// Default: `None` (unlimited).
    #[serde(default)]
    pub max_total_size_mb: Option<usize>,

    /// Whether to capture request bodies. When `false`, request bodies are
    /// not stored (headers and metadata are still recorded). Default: `true`.
    #[serde(default = "default_true")]
    pub capture_request_bodies: bool,

    /// Whether to capture response bodies. When `false`, response bodies are
    /// not stored (headers and metadata are still recorded). Default: `true`.
    #[serde(default = "default_true")]
    pub capture_response_bodies: bool,

    /// Domains whose traffic should not be recorded (capture ignore list).
    /// Supports suffix and wildcard matching (e.g. `*.example.com` matches
    /// `api.example.com`). Default: empty (record all traffic).
    #[serde(default)]
    pub ignored_domains: Vec<String>,

    /// Domains to exclude from TLS interception (SSL passthrough).
    /// Connections to these hosts are tunneled directly without decryption.
    /// The traffic is still listed but flagged as passthrough.
    /// Supports suffix matching (e.g. "example.com" matches "api.example.com").
    #[serde(default)]
    pub passthrough_domains: Vec<String>,

    /// Enable HTTP/2 downstream (client-facing) support.
    ///
    /// When enabled, the proxy advertises both `h2` and `http/1.1` via ALPN
    /// during the TLS handshake with the client. If the client negotiates
    /// `h2`, the proxy parses HTTP/2 frames (using the `h2` crate) and
    /// multiplexes streams through the existing interception pipeline.
    /// HTTP/1.1 clients continue to work via ALPN fallback.
    ///
    /// This is required for intercepting gRPC traffic (which mandates HTTP/2).
    /// Defaults to `false` for safety; flip to `true` once validated.
    #[serde(default)]
    pub enable_h2_downstream: bool,

    /// Enable a SOCKS5 proxy listener (RFC 1928) in addition to the HTTP/HTTPS
    /// proxy listener. SOCKS5 is a blind TCP tunnel: the client asks the proxy
    /// to connect to an arbitrary `host:port` and then relays raw bytes in both
    /// directions. This is convenient for clients (browsers, CLI tools, mobile
    /// devices) that prefer SOCKS over HTTP CONNECT.
    ///
    /// Because SOCKS tunnels the client's TLS session end-to-end, HTTPS
    /// traffic cannot be MITM-intercepted via the SOCKS port — the connection
    /// is forwarded directly to the target. To intercept HTTPS, configure the
    /// client to use the HTTP proxy port with CONNECT instead. HTTP traffic
    /// sent over SOCKS is also tunneled blindly (a connection entry is still
    /// recorded so the activity is visible in the web UI).
    ///
    /// Defaults to `false`.
    #[serde(default)]
    pub enable_socks: bool,

    /// Port for the SOCKS5 proxy listener. Only used when `enable_socks` is
    /// `true`. Defaults to `1080` (the conventional SOCKS port). The SOCKS
    /// listener binds to the same `host` as the HTTP proxy.
    #[serde(default)]
    pub socks_port: Option<u16>,

    /// Optional username for SOCKS5 username/password authentication
    /// (RFC 1929). When `None`, the SOCKS listener advertises only the
    /// "no authentication required" method. When set together with
    /// `socks_auth_password`, the listener requires credentials from
    /// clients. Defaults to `None` (no auth).
    #[serde(default)]
    pub socks_auth_username: Option<String>,

    /// Optional password for SOCKS5 username/password authentication.
    /// Ignored unless `socks_auth_username` is also set.
    #[serde(default)]
    pub socks_auth_password: Option<String>,

    /// Upstream (external) proxy chaining configuration.
    ///
    /// When enabled, all outbound traffic — both the `reqwest`-based HTTP
    /// forwarding path and the raw TCP `CONNECT`/passthrough tunnels — is
    /// routed through the configured upstream proxy. This is essential for
    /// corporate environments where direct internet access is blocked and a
    /// mandatory egress proxy must be used.
    ///
    /// Supported upstream protocols: `http`, `https`, and `socks5`.
    /// Basic authentication (username/password) is supported for all three.
    /// The `no_proxy_hosts` bypass list allows specific hosts/CIDRs to skip
    /// the upstream proxy and connect directly.
    ///
    /// See [`UpstreamProxyConfig`] for field details.
    #[serde(default)]
    pub upstream_proxy: UpstreamProxyConfig,

    /// IP allowlist for the proxy and API listeners.
    ///
    /// When non-empty, only connections from the listed IP addresses or
    /// CIDR ranges (e.g. `192.168.1.0/24`, `10.0.0.5`, `fd00::/8`) are
    /// accepted. Loopback addresses (`127.0.0.1`, `::1`) are always
    /// allowed regardless of this list, so a locally-started proxy can
    /// never be locked out.
    ///
    /// An empty list (the default) allows connections from any address —
    /// this preserves backward compatibility for existing deployments.
    /// The list is applied live: updating it via the API
    /// (`PATCH /api/config`) takes effect for new connections immediately
    /// without a restart.
    ///
    /// See [`crate::AccessControlList`] for matching semantics and
    /// [`docs/ACCESS_CONTROL.md`](../../docs/ACCESS_CONTROL.md) for the
    /// end-user guide.
    #[serde(default)]
    pub allowed_ips: Vec<String>,

    /// Auto Save configuration for periodic session backup and rotation.
    ///
    /// When enabled, the proxy periodically exports the current session to a
    /// backup directory (as HAR or Madhyamas-native Session format) for
    /// disaster recovery. Old backups are pruned automatically (keep last N).
    /// Optional session rotation starts a new session after a configurable
    /// number of requests or elapsed minutes.
    ///
    /// See [`AutoSaveConfig`] for field details and
    /// [`docs/AUTO_SAVE.md`](../../docs/AUTO_SAVE.md) for the end-user guide.
    #[serde(default)]
    pub auto_save: AutoSaveConfig,

    /// Mirror tool configuration for saving response bodies to disk.
    ///
    /// When enabled, the proxy writes each captured response body to disk
    /// following the URL path structure (`output_dir/host/path/content`),
    /// along with a `.meta.json` sidecar containing request/response
    /// metadata. This is useful for offline browsing, debugging, and
    /// archiving.
    ///
    /// See [`MirrorConfig`] for field details and
    /// [`docs/MIRROR.md`](../../docs/MIRROR.md) for the end-user guide.
    #[serde(default)]
    pub mirror: MirrorConfig,

    /// Log file rotation configuration.
    ///
    /// Controls how the proxy's log file (`<log_path>/madhyamas.log`) is
    /// rotated to prevent unbounded growth. Supports time-based rotation
    /// (hourly/daily), size-based rotation, automatic pruning of old files,
    /// and on-demand rotation via the API/CLI/MCP.
    ///
    /// See [`LogConfig`] for field details and
    /// [`docs/LOGGING.md`](../../docs/LOGGING.md) for the end-user guide.
    #[serde(default)]
    pub log_config: LogConfig,
}

/// Upstream (external) proxy chaining configuration.
///
/// Madhyamas forwards all outbound traffic through this proxy when `enabled`
/// is `true`. This is the equivalent of Charles Proxy's "External Proxy"
/// feature and is critical for:
///
/// - Corporate networks with a mandatory egress proxy
/// - Chaining multiple debugging proxies (e.g. Madhyamas → mitmproxy → internet)
/// - Routing traffic through a SOCKS5 gateway (e.g. SSH dynamic forwarding)
///
/// # Protocols
///
/// | Protocol | Use case |
/// |----------|----------|
/// | `http`   | Plain HTTP CONNECT proxy (most common corporate proxy) |
/// | `https`  | TLS-wrapped HTTP proxy (proxy URL is `https://...`) |
/// | `socks5` | SOCKS5 proxy (e.g. `ssh -D 1080` dynamic forwarding) |
///
/// # Bypass list
///
/// `no_proxy_hosts` is a list of hostnames/CIDRs that should bypass the
/// upstream proxy and connect directly. Matching is case-insensitive and
/// supports suffix matching (e.g. `example.com` matches `api.example.com`)
/// and exact CIDR matching (e.g. `192.168.0.0/16`). The special token `*`
/// disables bypass (everything goes through the proxy); an empty list also
/// means "no bypass".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpstreamProxyConfig {
    /// Master switch. When `false`, all upstream proxy fields are ignored
    /// and Madhyamas connects directly to target servers (the default).
    #[serde(default)]
    pub enabled: bool,

    /// Upstream proxy protocol: `"http"`, `"https"`, or `"socks5"`.
    /// Defaults to `"http"` when unset.
    #[serde(default = "default_upstream_protocol")]
    pub protocol: String,

    /// Upstream proxy hostname or IP address (e.g. `corp-proxy.example.com`).
    #[serde(default)]
    pub host: String,

    /// Upstream proxy port (e.g. `8080`).
    #[serde(default)]
    pub port: u16,

    /// Optional Basic-auth username. When set, `auth_password` must also be
    /// set. For SOCKS5 this uses RFC 1929 username/password authentication.
    #[serde(default)]
    pub auth_username: Option<String>,

    /// Optional Basic-auth password. Ignored unless `auth_username` is set.
    #[serde(default)]
    pub auth_password: Option<String>,

    /// Bypass list: hosts/CIDRs that connect directly, bypassing the upstream
    /// proxy. See the type-level docs for matching semantics.
    #[serde(default)]
    pub no_proxy_hosts: Vec<String>,
}

impl Default for UpstreamProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protocol: default_upstream_protocol(),
            host: String::new(),
            port: 0,
            auth_username: None,
            auth_password: None,
            no_proxy_hosts: Vec::new(),
        }
    }
}

/// Default value provider for [`UpstreamProxyConfig::protocol`].
fn default_upstream_protocol() -> String {
    "http".to_string()
}

/// Default value provider for boolean config fields that default to `true`.
fn default_true() -> bool {
    true
}

/// Default value provider for [`AutoSaveConfig::interval_seconds`].
fn default_autosave_interval() -> u64 {
    300
}

/// Default value provider for [`AutoSaveConfig::export_format`].
fn default_autosave_format() -> String {
    "har".to_string()
}

/// Default value provider for [`AutoSaveConfig::max_backups`].
fn default_autosave_max_backups() -> usize {
    10
}

/// Auto Save configuration for periodic session backup and rotation.
///
/// Traffic is stored in SQLite in real time (every request/response is
/// persisted immediately), so Auto Save is not the primary persistence
/// mechanism. Instead, it provides:
///
/// - **Periodic HAR/Session export** to a backup directory for disaster
///   recovery (e.g. if the SQLite database is corrupted or accidentally
///   deleted).
/// - **Automatic session rotation** — start a new session after N requests
///   or M minutes, archiving the old one.
/// - **Backup pruning** — keep only the last `max_backups` files, deleting
///   the oldest first.
///
/// See [`docs/AUTO_SAVE.md`](../../docs/AUTO_SAVE.md) for the end-user guide.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoSaveConfig {
    /// Master switch. When `false` (the default), no periodic export or
    /// rotation is performed.
    #[serde(default)]
    pub enabled: bool,

    /// Interval between snapshots in seconds. Default: 300 (5 minutes).
    #[serde(default = "default_autosave_interval")]
    pub interval_seconds: u64,

    /// Export format: `"har"` (HAR 1.2, interoperable) or `"session"`
    /// (Madhyamas-native `SessionExport` JSON, restorable via import).
    /// Default: `"har"`.
    #[serde(default = "default_autosave_format")]
    pub export_format: String,

    /// Directory where backup files are written. The directory is created
    /// if it does not exist. Default: `~/.madhyamas/backups`.
    #[serde(default = "default_autosave_output_dir")]
    pub output_dir: String,

    /// Maximum number of backup files to keep. Older files are deleted
    /// (pruned) after each snapshot when this limit is exceeded.
    /// Default: 10.
    #[serde(default = "default_autosave_max_backups")]
    pub max_backups: usize,

    /// When set, rotate (start a new session) after this many requests have
    /// been recorded in the current session. `None` disables request-based
    /// rotation.
    #[serde(default)]
    pub rotate_after_requests: Option<usize>,

    /// When set, rotate (start a new session) after this many minutes have
    /// elapsed since the current session started. `None` disables
    /// time-based rotation.
    #[serde(default)]
    pub rotate_after_minutes: Option<u64>,
}

/// Default value provider for [`AutoSaveConfig::output_dir`].
fn default_autosave_output_dir() -> String {
    get_data_dir().join("backups").to_string_lossy().to_string()
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: default_autosave_interval(),
            export_format: default_autosave_format(),
            output_dir: default_autosave_output_dir(),
            max_backups: default_autosave_max_backups(),
            rotate_after_requests: None,
            rotate_after_minutes: None,
        }
    }
}

/// Default value provider for [`MirrorConfig::output_dir`].
fn default_mirror_output_dir() -> String {
    get_data_dir().join("mirror").to_string_lossy().to_string()
}

/// Mirror tool configuration for saving response bodies to disk.
///
/// When `enabled` is `true`, the proxy writes each captured response body
/// to disk following the URL path structure (`output_dir/host/path/content`),
/// along with a `.meta.json` sidecar containing request/response metadata.
/// This is the equivalent of Charles Proxy's "Mirror" / "Save Responses"
/// feature and is useful for offline browsing, debugging, and archiving.
///
/// # Path mapping
///
/// | URL | Filesystem path |
/// |-----|-----------------|
/// | `https://api.example.com/v1/users/123` | `output_dir/api.example.com/v1/users/123/index.json` |
/// | `https://cdn.example.com/assets/img/logo.png` | `output_dir/cdn.example.com/assets/img/logo.png` |
///
/// Paths ending with `/` or having no file extension are saved as
/// `index.html` (or `index.json` based on content-type). Query strings are
/// stored in the metadata sidecar to keep filenames clean.
///
/// See [`docs/MIRROR.md`](../../docs/MIRROR.md) for the end-user guide.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MirrorConfig {
    /// Master switch. When `false` (the default), no response bodies are
    /// written to disk.
    #[serde(default)]
    pub enabled: bool,

    /// Directory where mirrored response bodies are written. The directory
    /// is created if it does not exist. Default: `~/.madhyamas/mirror`.
    #[serde(default = "default_mirror_output_dir")]
    pub output_dir: String,

    /// Optional list of host patterns to mirror. When set, only responses
    /// from matching hosts are written. Patterns support exact hostnames,
    /// wildcard subdomains (`*.example.com`), and globs (`*api*`).
    /// When `None` or empty, all hosts are mirrored.
    #[serde(default)]
    pub host_filter: Option<Vec<String>>,

    /// Whether to also save request bodies to disk (alongside response
    /// bodies). Request bodies are written as `<file>.request`. Default:
    /// `false`.
    #[serde(default)]
    pub save_request_bodies: bool,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            output_dir: default_mirror_output_dir(),
            host_filter: None,
            save_request_bodies: false,
        }
    }
}

/// Default value provider for [`LogConfig::max_files`].
fn default_log_max_files() -> usize {
    7
}

/// Default value provider for [`LogConfig::max_file_size_mb`].
fn default_log_max_file_size_mb() -> u64 {
    100
}

/// Default value provider for [`LogConfig::rotation`].
fn default_log_rotation() -> LogRotation {
    LogRotation::Daily
}

/// Log file rotation strategy.
///
/// Controls when the current log file is rotated (renamed with a timestamp
/// suffix and a fresh file opened). Archived files are pruned to
/// [`LogConfig::max_files`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode")]
pub enum LogRotation {
    /// Never rotate based on time or size. The log file grows without bound
    /// (not recommended for long-running deployments).
    #[serde(rename = "never")]
    Never,
    /// Rotate once per hour. A new file is opened at the top of each hour.
    #[serde(rename = "hourly")]
    Hourly,
    /// Rotate once per day (at midnight local time). This is the default.
    #[serde(rename = "daily")]
    Daily,
    /// Rotate when the current file exceeds `size_mb` megabytes. No
    /// time-based rotation is performed.
    #[serde(rename = "size")]
    SizeMB {
        /// Maximum size in megabytes before the file is rotated.
        size_mb: u64,
    },
}

impl Default for LogRotation {
    fn default() -> Self {
        default_log_rotation()
    }
}

impl LogRotation {
    /// Returns a human-readable label for the rotation mode.
    pub fn label(&self) -> String {
        match self {
            LogRotation::Never => "never".to_string(),
            LogRotation::Hourly => "hourly".to_string(),
            LogRotation::Daily => "daily".to_string(),
            LogRotation::SizeMB { size_mb } => format!("size ({} MB)", size_mb),
        }
    }

    /// Effective per-file size cap in megabytes. Time-based rotation modes
    /// use [`LogConfig::max_file_size_mb`] as a safety cap so a single file
    /// can never grow unbounded between scheduled rotations. `Never` uses
    /// the same cap. `SizeMB` uses its own `size_mb`.
    pub fn effective_size_cap_mb(&self, fallback_mb: u64) -> u64 {
        match self {
            LogRotation::Never | LogRotation::Hourly | LogRotation::Daily => fallback_mb,
            LogRotation::SizeMB { size_mb } => *size_mb,
        }
    }
}

/// Log file rotation configuration.
///
/// When `enabled` is `true` (the default), the proxy writes log events to
/// `<log_path>/madhyamas.log` and rotates the file according to
/// [`LogConfig::rotation`]. Archived files are named
/// `madhyamas.log.<timestamp>` and pruned to [`LogConfig::max_files`].
///
/// On-demand rotation is always available via `POST /api/logs/rotate`
/// (and the `madhyamas logs rotate` CLI / `madhyamas_logs_rotate` MCP tool)
/// regardless of the configured rotation mode.
///
/// See [`docs/LOGGING.md`](../../docs/LOGGING.md) for the end-user guide.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogConfig {
    /// Master switch. When `false`, logs are written to stdout only and no
    /// log files are created. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Rotation strategy. Default: [`LogRotation::Daily`].
    #[serde(default = "default_log_rotation")]
    pub rotation: LogRotation,

    /// Maximum number of archived log files to keep. When the count is
    /// exceeded, the oldest archived file is deleted. Default: 7.
    #[serde(default = "default_log_max_files")]
    pub max_files: usize,

    /// Hard per-file size cap in megabytes. Even with time-based rotation
    /// (hourly/daily), a single file that exceeds this size is rotated
    /// immediately. This is a safety net to prevent unbounded growth.
    /// Default: 100 MB. Ignored when `rotation` is [`LogRotation::SizeMB`]
    /// (which has its own `size_mb`).
    #[serde(default = "default_log_max_file_size_mb")]
    pub max_file_size_mb: u64,

    /// Write log events as structured JSON instead of the default
    /// human-readable text format. Default: `false`.
    #[serde(default)]
    pub json_format: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rotation: default_log_rotation(),
            max_files: default_log_max_files(),
            max_file_size_mb: default_log_max_file_size_mb(),
            json_format: false,
        }
    }
}

impl UpstreamProxyConfig {
    /// Build the reqwest-compatible proxy URL string.
    ///
    /// Returns `None` when the config is disabled or the host is empty.
    /// The returned URL has the form `<protocol>://<host>:<port>` (no
    /// credentials — those are attached separately via `reqwest::Proxy::
    /// basic_auth` to avoid leaking them in logs/URLs).
    ///
    /// # Errors
    ///
    /// Returns an error if `protocol` is not one of `http`, `https`, or
    /// `socks5`.
    pub fn proxy_url(&self) -> crate::Result<Option<String>> {
        if !self.enabled || self.host.trim().is_empty() {
            return Ok(None);
        }
        let scheme = match self.protocol.to_lowercase().as_str() {
            "http" => "http",
            "https" => "https",
            "socks5" => "socks5",
            other => {
                return Err(crate::Error::Config(format!(
                    "Invalid upstream proxy protocol: `{other}` (expected http, https, or socks5)"
                )));
            }
        };
        Ok(Some(format!("{}://{}:{}", scheme, self.host, self.port)))
    }

    /// Whether Basic/SOCKS5 authentication credentials are configured.
    pub fn auth_enabled(&self) -> bool {
        self.auth_username.is_some() && self.auth_password.is_some()
    }

    /// Check whether a target host should bypass the upstream proxy.
    ///
    /// Matching is case-insensitive. Supports:
    /// - Suffix matching: `example.com` matches `api.example.com`
    /// - Exact hostname: `localhost`
    /// - CIDR notation: `192.168.0.0/16`, `10.0.0.0/8`
    /// - Bare IP: `127.0.0.1`
    ///
    /// An empty bypass list never bypasses (returns `false`).
    pub fn should_bypass(&self, target_host: &str) -> bool {
        if self.no_proxy_hosts.is_empty() {
            return false;
        }
        let target = target_host.trim().trim_end_matches('.').to_lowercase();
        if target.is_empty() {
            return false;
        }

        for entry in &self.no_proxy_hosts {
            let entry = entry.trim().trim_end_matches('.').to_lowercase();
            if entry.is_empty() {
                continue;
            }

            // CIDR notation (contains '/'): parse and check IP containment.
            if let Some((ip_str, cidr_str)) = entry.split_once('/') {
                if let (Ok(ip), Ok(cidr)) =
                    (ip_str.parse::<std::net::IpAddr>(), cidr_str.parse::<u8>())
                {
                    if let Ok(net) = ipnet_like(ip, cidr) {
                        if let Ok(target_ip) = target.parse::<std::net::IpAddr>() {
                            if net.contains(&target_ip) {
                                return true;
                            }
                        }
                    }
                }
                continue;
            }

            // Bare IP address: exact match.
            if entry.parse::<std::net::IpAddr>().is_ok() {
                if entry == target {
                    return true;
                }
                continue;
            }

            // Wildcard suffix: "*.example.com" matches "api.example.com".
            let entry_pattern = entry.strip_prefix("*.").unwrap_or(&entry);
            if target == entry_pattern || target.ends_with(&format!(".{entry_pattern}")) {
                return true;
            }
        }

        false
    }
}

/// Minimal CIDR containment helper that avoids pulling in a new dependency.
///
/// Constructs an `IpNet`-like range from an IP address and a prefix length
/// and provides `contains()` for membership tests. Returns `Err` if the
/// prefix length is out of range for the address family.
fn ipnet_like(ip: std::net::IpAddr, prefix: u8) -> Result<CidrRange, String> {
    match ip {
        std::net::IpAddr::V4(v4) => {
            if prefix > 32 {
                return Err(format!("IPv4 prefix too large: {prefix}"));
            }
            let mask: u32 = if prefix == 0 {
                0
            } else {
                (!0u32) << (32 - prefix)
            };
            let network = u32::from_be_bytes(v4.octets()) & mask;
            Ok(CidrRange::V4 { network, mask })
        }
        std::net::IpAddr::V6(v6) => {
            if prefix > 128 {
                return Err(format!("IPv6 prefix too large: {prefix}"));
            }
            let octets = v6.octets();
            let mut network = [0u8; 16];
            let mut mask = [0u8; 16];
            // Build the mask one bit at a time.
            for i in 0..128 {
                if i < prefix as usize {
                    mask[i / 8] |= 0x80 >> (i % 8);
                }
            }
            for i in 0..16 {
                network[i] = octets[i] & mask[i];
            }
            Ok(CidrRange::V6 { network, mask })
        }
    }
}

/// A minimal CIDR range supporting both IPv4 and IPv6 containment tests.
enum CidrRange {
    V4 { network: u32, mask: u32 },
    V6 { network: [u8; 16], mask: [u8; 16] },
}

impl CidrRange {
    fn contains(&self, addr: &std::net::IpAddr) -> bool {
        match (self, addr) {
            (CidrRange::V4 { network, mask }, std::net::IpAddr::V4(v4)) => {
                (u32::from_be_bytes(v4.octets()) & mask) == *network
            }
            (CidrRange::V6 { network, mask }, std::net::IpAddr::V6(v6)) => {
                let octets = v6.octets();
                for i in 0..16 {
                    if (octets[i] & mask[i]) != network[i] {
                        return false;
                    }
                }
                true
            }
            _ => false, // address family mismatch
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        let data_dir = get_data_dir();
        Self {
            proxy_port: 8888,
            api_port: 3001,
            host: "127.0.0.1".to_string(),
            public_ip: None,
            cert_path: data_dir.join("certs").to_string_lossy().to_string(),
            db_path: data_dir.join("traffic.db").to_string_lossy().to_string(),
            log_path: data_dir.join("logs").to_string_lossy().to_string(),
            verbose: false,
            max_requests: 10000,
            intercept_https: true,
            max_body_size: 20 * 1024 * 1024, // 20 MB
            max_total_size_mb: None,
            capture_request_bodies: true,
            capture_response_bodies: true,
            ignored_domains: Vec::new(),
            passthrough_domains: Vec::new(),
            enable_h2_downstream: false,
            enable_socks: false,
            socks_port: None,
            socks_auth_username: None,
            socks_auth_password: None,
            upstream_proxy: UpstreamProxyConfig::default(),
            allowed_ips: Vec::new(),
            auto_save: AutoSaveConfig::default(),
            mirror: MirrorConfig::default(),
            log_config: LogConfig::default(),
        }
    }
}

impl ProxyConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect the local private IP address
    /// Priority:
    /// 1. MADHYAMAS_PUBLIC_IP environment variable (for Docker/container environments)
    /// 2. MADHYAMAS_HOST_IP environment variable (alternative for Docker)
    /// 3. Auto-detect from network interfaces (prefers 192.168.x.x over Docker bridge IPs)
    pub fn detect_private_ip() -> Option<String> {
        // First check environment variables (essential for Docker)
        if let Ok(ip) = std::env::var("MADHYAMAS_PUBLIC_IP") {
            if !ip.is_empty() {
                return Some(ip);
            }
        }
        if let Ok(ip) = std::env::var("MADHYAMAS_HOST_IP") {
            if !ip.is_empty() {
                return Some(ip);
            }
        }

        use local_ip_address::list_afinet_netifas;

        if let Ok(network_interfaces) = list_afinet_netifas() {
            // Collect all private IPs and prioritize them
            let mut private_ips: Vec<(u8, String)> = Vec::new();

            for (name, ip) in network_interfaces.iter() {
                let ip_str = ip.to_string();
                let iface_name = name.to_lowercase();

                // Skip loopback
                if ip_str.starts_with("127.") {
                    continue;
                }

                // Skip Docker bridge interfaces (docker0, br-*, veth*)
                if iface_name.starts_with("docker")
                    || iface_name.starts_with("br-")
                    || iface_name.starts_with("veth")
                {
                    continue;
                }

                // Check if it's a private IP
                if let Ok(std::net::IpAddr::V4(ipv4)) = ip_str.parse::<std::net::IpAddr>() {
                    let octets = ipv4.octets();

                    // 192.168.0.0/16 - highest priority (typical home/office LAN)
                    if octets[0] == 192 && octets[1] == 168 {
                        private_ips.push((1, ip_str.clone()));
                        continue;
                    }

                    // 10.0.0.0/8 - medium priority
                    if octets[0] == 10 {
                        // Skip typical Docker network ranges (172.17-31.x.x)
                        private_ips.push((2, ip_str.clone()));
                        continue;
                    }

                    // 172.16.0.0/12 - lower priority (often Docker networks)
                    if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
                        // Docker typically uses 172.17.x.x - 172.31.x.x
                        // 172.16.x.x is less common for Docker
                        let priority = if octets[1] == 16 { 3 } else { 4 };
                        private_ips.push((priority, ip_str.clone()));
                    }
                }
            }

            // Sort by priority and return the best match
            private_ips.sort_by_key(|(priority, _)| *priority);
            if let Some((_, ip)) = private_ips.first() {
                return Some(ip.clone());
            }
        }

        None
    }

    /// Check if running inside a Docker container
    pub fn is_docker() -> bool {
        // Check for /.dockerenv file
        if std::path::Path::new("/.dockerenv").exists() {
            return true;
        }
        // Check cgroup for docker
        if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
            if cgroup.contains("docker") || cgroup.contains("kubepods") {
                return true;
            }
        }
        false
    }

    /// Load configuration from a file
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::Error::Config(format!("Failed to read config file: {}", e)))?;

        let config: Self = serde_json::from_str(&content)
            .map_err(|e| crate::Error::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    /// Get the default config file path (`~/.madhyamas/config.json`).
    pub fn config_file_path() -> PathBuf {
        get_data_dir().join("config.json")
    }

    /// Save the configuration to a file as JSON.
    pub fn save_to_file(&self, path: &std::path::Path) -> crate::Result<()> {
        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::Error::Config(format!("Failed to create config directory: {}", e))
            })?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::Config(format!("Failed to serialize config: {}", e)))?;

        // Write atomically: write to a temp file then rename, so a crash
        // during write doesn't corrupt the existing config.
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json)
            .map_err(|e| crate::Error::Config(format!("Failed to write config file: {}", e)))?;
        std::fs::rename(&tmp_path, path)
            .map_err(|e| crate::Error::Config(format!("Failed to rename config file: {}", e)))?;

        Ok(())
    }

    /// Save to the default config file path (`~/.madhyamas/config.json`).
    pub fn save(&self) -> crate::Result<()> {
        self.save_to_file(&Self::config_file_path())
    }

    /// Load the persisted config from the default path, if it exists.
    /// Returns `None` if the file doesn't exist (first run or never saved).
    pub fn load_saved() -> Option<Self> {
        let path = Self::config_file_path();
        if !path.exists() {
            return None;
        }
        match Self::from_file(path.to_str()?) {
            Ok(config) => Some(config),
            Err(e) => {
                tracing::warn!("Failed to load saved config, using defaults: {}", e);
                None
            }
        }
    }

    /// Get the proxy address
    pub fn proxy_addr(&self) -> String {
        format!("{}:{}", self.host, self.proxy_port)
    }

    /// Get the API address
    pub fn api_addr(&self) -> String {
        format!("{}:{}", self.host, self.api_port)
    }

    /// Get the effective SOCKS5 port. Returns the configured `socks_port`
    /// if set, otherwise the conventional default of `1080`.
    pub fn socks_port(&self) -> u16 {
        self.socks_port.unwrap_or(1080)
    }

    /// Get the SOCKS5 listener address (`host:socks_port`).
    pub fn socks_addr(&self) -> String {
        format!("{}:{}", self.host, self.socks_port())
    }

    /// Whether SOCKS5 username/password authentication is configured.
    pub fn socks_auth_enabled(&self) -> bool {
        self.socks_auth_username.is_some()
    }

    /// Ensure all required data directories exist
    pub fn ensure_directories(&self) -> crate::Result<()> {
        std::fs::create_dir_all(&self.cert_path)
            .map_err(|e| crate::Error::Config(format!("Failed to create cert directory: {}", e)))?;
        std::fs::create_dir_all(&self.log_path)
            .map_err(|e| crate::Error::Config(format!("Failed to create log directory: {}", e)))?;
        Ok(())
    }

    /// Get the path to the CA certificate
    pub fn ca_cert_path(&self) -> PathBuf {
        PathBuf::from(&self.cert_path).join("madhyamas-ca.pem")
    }

    /// Get the path to the CA private key
    pub fn ca_key_path(&self) -> PathBuf {
        PathBuf::from(&self.cert_path).join("madhyamas-ca-key.pem")
    }

    /// Check if a host should be SSL-passed-through (not intercepted).
    /// Uses suffix matching so "example.com" matches "api.example.com".
    pub fn should_passthrough(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.');
        self.passthrough_domains.iter().any(|d| {
            let d = d.trim().trim_end_matches('.');
            !d.is_empty() && (host == d || host.ends_with(&format!(".{}", d)))
        })
    }

    /// Check if a target host should bypass the upstream proxy.
    ///
    /// Delegates to [`UpstreamProxyConfig::should_bypass`]. When the upstream
    /// proxy is disabled, this always returns `false` (no bypass needed —
    /// there is nothing to bypass).
    pub fn should_bypass_upstream(&self, host: &str) -> bool {
        self.upstream_proxy.enabled && self.upstream_proxy.should_bypass(host)
    }

    /// Whether upstream proxy chaining is active (enabled with a non-empty host).
    pub fn upstream_proxy_active(&self) -> bool {
        self.upstream_proxy.enabled && !self.upstream_proxy.host.trim().is_empty()
    }

    /// Build an [`AccessControlList`] from the configured `allowed_ips`.
    ///
    /// Returns an "allow all" list when `allowed_ips` is empty (the
    /// default). Invalid entries produce an error so callers can surface
    /// bad configuration early rather than silently falling back to
    /// "allow all".
    ///
    /// Call this on the live config snapshot (under the read lock) each
    /// time a connection is accepted so that API updates to
    /// `allowed_ips` take effect immediately for new connections.
    pub fn access_control_list(&self) -> crate::Result<crate::AccessControlList> {
        crate::AccessControlList::new(&self.allowed_ips)
    }

    /// Whether IP access control is currently active (non-empty allowlist).
    pub fn access_control_enabled(&self) -> bool {
        !self.allowed_ips.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(enabled: bool, protocol: &str, host: &str, port: u16) -> UpstreamProxyConfig {
        UpstreamProxyConfig {
            enabled,
            protocol: protocol.to_string(),
            host: host.to_string(),
            port,
            auth_username: None,
            auth_password: None,
            no_proxy_hosts: Vec::new(),
        }
    }

    #[test]
    fn proxy_url_returns_none_when_disabled() {
        let c = cfg(false, "http", "proxy.example.com", 8080);
        assert_eq!(c.proxy_url().unwrap(), None);
    }

    #[test]
    fn proxy_url_returns_none_when_host_empty() {
        let c = cfg(true, "http", "", 8080);
        assert_eq!(c.proxy_url().unwrap(), None);
    }

    #[test]
    fn proxy_url_builds_http_url() {
        let c = cfg(true, "http", "corp-proxy.example.com", 8080);
        assert_eq!(
            c.proxy_url().unwrap().as_deref(),
            Some("http://corp-proxy.example.com:8080")
        );
    }

    #[test]
    fn proxy_url_builds_https_url() {
        let c = cfg(true, "https", "secure-proxy.example.com", 443);
        assert_eq!(
            c.proxy_url().unwrap().as_deref(),
            Some("https://secure-proxy.example.com:443")
        );
    }

    #[test]
    fn proxy_url_builds_socks5_url() {
        let c = cfg(true, "socks5", "127.0.0.1", 1080);
        assert_eq!(
            c.proxy_url().unwrap().as_deref(),
            Some("socks5://127.0.0.1:1080")
        );
    }

    #[test]
    fn proxy_url_rejects_invalid_protocol() {
        let c = cfg(true, "ftp", "proxy.example.com", 8080);
        assert!(c.proxy_url().is_err());
    }

    #[test]
    fn proxy_url_normalizes_protocol_case() {
        let c = cfg(true, "SOCKS5", "127.0.0.1", 1080);
        assert_eq!(
            c.proxy_url().unwrap().as_deref(),
            Some("socks5://127.0.0.1:1080")
        );
    }

    #[test]
    fn auth_enabled_requires_both_username_and_password() {
        let mut c = cfg(true, "http", "proxy", 8080);
        assert!(!c.auth_enabled());
        c.auth_username = Some("user".to_string());
        assert!(!c.auth_enabled());
        c.auth_password = Some("pass".to_string());
        assert!(c.auth_enabled());
    }

    #[test]
    fn should_bypass_empty_list_never_bypasses() {
        let c = cfg(true, "http", "proxy", 8080);
        assert!(!c.should_bypass("localhost"));
        assert!(!c.should_bypass("example.com"));
    }

    #[test]
    fn should_bypass_exact_hostname_match() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["localhost".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("localhost"));
        // Suffix matching: "localhost" matches "api.localhost" (consistent
        // with the existing should_passthrough logic).
        assert!(c.should_bypass("api.localhost"));
        // But not a completely different host.
        assert!(!c.should_bypass("example.com"));
    }

    #[test]
    fn should_bypass_suffix_match() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["example.com".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("example.com"));
        assert!(c.should_bypass("api.example.com"));
        assert!(c.should_bypass("www.example.com"));
        assert!(!c.should_bypass("notexample.com"));
    }

    #[test]
    fn should_bypass_wildcard_suffix() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["*.internal.corp".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("api.internal.corp"));
        assert!(c.should_bypass("internal.corp"));
        assert!(!c.should_bypass("external.corp"));
    }

    #[test]
    fn should_bypass_bare_ipv4() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["127.0.0.1".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("127.0.0.1"));
        assert!(!c.should_bypass("127.0.0.2"));
    }

    #[test]
    fn should_bypass_ipv4_cidr() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["192.168.0.0/16".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("192.168.1.100"));
        assert!(c.should_bypass("192.168.0.0"));
        assert!(c.should_bypass("192.168.255.255"));
        assert!(!c.should_bypass("192.169.0.1"));
        assert!(!c.should_bypass("10.0.0.1"));
    }

    #[test]
    fn should_bypass_ipv4_cidr_24() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["10.0.0.0/24".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("10.0.0.0"));
        assert!(c.should_bypass("10.0.0.255"));
        assert!(!c.should_bypass("10.0.1.0"));
    }

    #[test]
    fn should_bypass_ipv6_cidr() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["fd00::/8".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("fd00::1"));
        assert!(c.should_bypass("fd12:3456::abcd"));
        assert!(!c.should_bypass("fe00::1"));
    }

    #[test]
    fn should_bypass_case_insensitive() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["Example.COM".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("API.example.com"));
        assert!(c.should_bypass("EXAMPLE.com"));
    }

    #[test]
    fn should_bypass_multiple_entries() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec![
                "localhost".to_string(),
                "127.0.0.0/8".to_string(),
                "*.internal.corp".to_string(),
            ],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("localhost"));
        assert!(c.should_bypass("127.0.0.1"));
        assert!(c.should_bypass("127.255.255.255"));
        assert!(c.should_bypass("api.internal.corp"));
        assert!(!c.should_bypass("example.com"));
    }

    #[test]
    fn should_bypass_trims_entries() {
        let c = UpstreamProxyConfig {
            no_proxy_hosts: vec!["  localhost  ".to_string()],
            ..cfg(true, "http", "proxy", 8080)
        };
        assert!(c.should_bypass("localhost"));
    }

    #[test]
    fn default_upstream_proxy_is_disabled() {
        let c = UpstreamProxyConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.protocol, "http");
        assert!(c.host.is_empty());
        assert_eq!(c.port, 0);
        assert!(!c.auth_enabled());
        assert!(c.no_proxy_hosts.is_empty());
    }

    #[test]
    fn proxy_config_default_has_disabled_upstream() {
        let c = ProxyConfig::default();
        assert!(!c.upstream_proxy_active());
        assert!(!c.should_bypass_upstream("anything.com"));
    }

    #[test]
    fn proxy_config_upstream_proxy_active_when_enabled_with_host() {
        let c = ProxyConfig {
            upstream_proxy: cfg(true, "http", "corp-proxy", 8080),
            ..Default::default()
        };
        assert!(c.upstream_proxy_active());
    }

    #[test]
    fn proxy_config_upstream_proxy_inactive_when_enabled_but_no_host() {
        let c = ProxyConfig {
            upstream_proxy: cfg(true, "http", "", 8080),
            ..Default::default()
        };
        assert!(!c.upstream_proxy_active());
    }

    #[test]
    fn proxy_config_should_bypass_upstream_respects_disabled_state() {
        let c = ProxyConfig {
            upstream_proxy: UpstreamProxyConfig {
                enabled: false,
                no_proxy_hosts: vec!["localhost".to_string()],
                ..cfg(false, "http", "proxy", 8080)
            },
            ..Default::default()
        };
        // Even though "localhost" is in the bypass list, the proxy is
        // disabled so should_bypass_upstream must return false.
        assert!(!c.should_bypass_upstream("localhost"));
    }

    #[test]
    fn upstream_config_serializes_and_deserializes() {
        let c = UpstreamProxyConfig {
            enabled: true,
            protocol: "socks5".to_string(),
            host: "proxy.example.com".to_string(),
            port: 1080,
            auth_username: Some("user".to_string()),
            auth_password: Some("pass".to_string()),
            no_proxy_hosts: vec!["localhost".to_string(), "10.0.0.0/8".to_string()],
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: UpstreamProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn upstream_config_deserializes_with_defaults_for_missing_fields() {
        // Simulates an old config file that predates the upstream_proxy field.
        let json = r#"{"enabled": true, "host": "proxy", "port": 8080}"#;
        let c: UpstreamProxyConfig = serde_json::from_str(json).unwrap();
        assert!(c.enabled);
        assert_eq!(c.protocol, "http"); // default applied
        assert_eq!(c.host, "proxy");
        assert_eq!(c.port, 8080);
        assert!(c.auth_username.is_none());
        assert!(c.no_proxy_hosts.is_empty());
    }

    #[test]
    fn proxy_config_with_upstream_serializes_roundtrip() {
        let c = ProxyConfig {
            upstream_proxy: UpstreamProxyConfig {
                enabled: true,
                protocol: "https".to_string(),
                host: "secure-proxy.example.com".to_string(),
                port: 443,
                auth_username: Some("alice".to_string()),
                auth_password: Some("secret".to_string()),
                no_proxy_hosts: vec!["localhost".to_string()],
            },
            ..Default::default()
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: ProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c.upstream_proxy, back.upstream_proxy);
    }

    #[test]
    fn proxy_config_without_upstream_field_deserializes_with_default() {
        // A config JSON that omits the upstream_proxy field entirely (as
        // written by older versions of Madhyamas) must deserialize with the
        // default disabled upstream proxy.
        let json = r#"{
            "proxy_port": 8888,
            "api_port": 3001,
            "host": "127.0.0.1",
            "public_ip": null,
            "cert_path": "/tmp/certs",
            "db_path": "/tmp/db",
            "log_path": "/tmp/logs",
            "verbose": false,
            "max_requests": 10000,
            "intercept_https": true,
            "max_body_size": 20971520,
            "passthrough_domains": []
        }"#;
        let c: ProxyConfig = serde_json::from_str(json).unwrap();
        assert!(!c.upstream_proxy_active());
        assert_eq!(c.upstream_proxy, UpstreamProxyConfig::default());
    }
}

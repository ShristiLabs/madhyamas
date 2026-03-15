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
}

impl Default for ProxyConfig {
    fn default() -> Self {
        let data_dir = get_data_dir();
        Self {
            proxy_port: 8888,
            api_port: 3000,
            host: "127.0.0.1".to_string(),
            public_ip: None,
            cert_path: data_dir.join("certs").to_string_lossy().to_string(),
            db_path: data_dir.join("traffic.db").to_string_lossy().to_string(),
            log_path: data_dir.join("logs").to_string_lossy().to_string(),
            verbose: false,
            max_requests: 10000,
            intercept_https: true,
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

    /// Get the proxy address
    pub fn proxy_addr(&self) -> String {
        format!("{}:{}", self.host, self.proxy_port)
    }

    /// Get the API address
    pub fn api_addr(&self) -> String {
        format!("{}:{}", self.host, self.api_port)
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
}

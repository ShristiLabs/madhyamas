//! Configuration for ProxyForge

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Get the default data directory for ProxyForge
/// Uses ~/.proxyforge on all platforms
fn get_data_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".proxyforge")
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
        PathBuf::from(&self.cert_path).join("proxyforge-ca.pem")
    }

    /// Get the path to the CA private key
    pub fn ca_key_path(&self) -> PathBuf {
        PathBuf::from(&self.cert_path).join("proxyforge-ca-key.pem")
    }
}

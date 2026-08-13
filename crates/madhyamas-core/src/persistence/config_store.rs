//! Configuration persistence

use serde::{Deserialize, Serialize};

/// Persisted application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedConfig {
    /// Proxy listen address
    pub proxy_addr: String,
    /// API listen address
    pub api_addr: String,
    /// Certificate directory
    pub cert_dir: String,
    /// Data directory
    pub data_dir: String,
    /// Log level
    pub log_level: String,
    /// Theme preference
    pub theme: String,
    /// Window size (width, height)
    pub window_size: Option<(u32, u32)>,
    /// Column widths for traffic table
    pub column_widths: Option<Vec<u32>>,
    /// Custom settings
    pub custom: serde_json::Value,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            proxy_addr: "127.0.0.1:8888".to_string(),
            api_addr: "127.0.0.1:3000".to_string(),
            cert_dir: "./certs".to_string(),
            data_dir: "./data".to_string(),
            log_level: "info".to_string(),
            theme: "system".to_string(),
            window_size: None,
            column_widths: None,
            custom: serde_json::json!({}),
        }
    }
}

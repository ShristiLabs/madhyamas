//! Configuration persistence

use crate::Error;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

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

/// Store for application configuration
pub struct ConfigStore {
    conn: Mutex<Connection>,
}

impl ConfigStore {
    /// Create a new config store
    pub fn new<P: AsRef<Path>>(path: P) -> crate::Result<Arc<Self>> {
        let conn = Connection::open(path).map_err(Error::Database)?;

        let store = Arc::new(Self {
            conn: Mutex::new(conn),
        });

        store.create_tables()?;
        Ok(store)
    }

    /// Create an in-memory store
    pub fn in_memory() -> crate::Result<Arc<Self>> {
        let conn = Connection::open_in_memory().map_err(Error::Database)?;

        let store = Arc::new(Self {
            conn: Mutex::new(conn),
        });

        store.create_tables()?;
        Ok(store)
    }

    /// Create database tables
    fn create_tables(&self) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Get a config value
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> crate::Result<Option<T>> {
        let conn = self.conn.lock();

        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM config WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(Error::Database)?;

        match value {
            Some(v) => {
                let parsed: T = serde_json::from_str(&v)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Set a config value
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> crate::Result<()> {
        let conn = self.conn.lock();
        let json = serde_json::to_string(value)?;

        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            params![key, json],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Delete a config value
    pub fn delete(&self, key: &str) -> crate::Result<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM config WHERE key = ?1", params![key])
            .map_err(Error::Database)?;
        Ok(rows > 0)
    }

    /// Load full configuration
    pub fn load_config(&self) -> crate::Result<PersistedConfig> {
        let mut config = PersistedConfig::default();

        if let Some(v) = self.get::<String>("proxy_addr")? {
            config.proxy_addr = v;
        }
        if let Some(v) = self.get::<String>("api_addr")? {
            config.api_addr = v;
        }
        if let Some(v) = self.get::<String>("cert_dir")? {
            config.cert_dir = v;
        }
        if let Some(v) = self.get::<String>("data_dir")? {
            config.data_dir = v;
        }
        if let Some(v) = self.get::<String>("log_level")? {
            config.log_level = v;
        }
        if let Some(v) = self.get::<String>("theme")? {
            config.theme = v;
        }
        if let Some(v) = self.get::<(u32, u32)>("window_size")? {
            config.window_size = Some(v);
        }
        if let Some(v) = self.get::<Vec<u32>>("column_widths")? {
            config.column_widths = Some(v);
        }
        if let Some(v) = self.get::<serde_json::Value>("custom")? {
            config.custom = v;
        }

        Ok(config)
    }

    /// Save full configuration
    pub fn save_config(&self, config: &PersistedConfig) -> crate::Result<()> {
        self.set("proxy_addr", &config.proxy_addr)?;
        self.set("api_addr", &config.api_addr)?;
        self.set("cert_dir", &config.cert_dir)?;
        self.set("data_dir", &config.data_dir)?;
        self.set("log_level", &config.log_level)?;
        self.set("theme", &config.theme)?;
        if let Some(ref v) = config.window_size {
            self.set("window_size", v)?;
        }
        if let Some(ref v) = config.column_widths {
            self.set("column_widths", v)?;
        }
        self.set("custom", &config.custom)?;
        Ok(())
    }

    /// Export config to JSON
    pub fn export(&self) -> crate::Result<String> {
        let config = self.load_config()?;
        Ok(serde_json::to_string_pretty(&config)?)
    }

    /// Import config from JSON
    pub fn import(&self, json: &str) -> crate::Result<()> {
        let config: PersistedConfig = serde_json::from_str(json)?;
        self.save_config(&config)?;
        Ok(())
    }
}

//! SQLite-backed [`ConfigStoreBackend`] implementation.
//!
//! [`SqliteConfigStore`] wraps a [`sqlx::SqlitePool`] and persists
//! configuration key/value pairs as JSON text in a single `config` table,
//! mirroring the schema and serialization used by the former `rusqlite`
//! `ConfigStore`. All queries use runtime SQL strings with `?` placeholders.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use sqlx::{FromRow, SqlitePool};

use crate::persistence::PersistedConfig;
use crate::storage::ConfigStoreBackend;
use crate::Result;

/// Schema for the `config` table (key/value, JSON text values).
const SCHEMA_CONFIG: &str = "CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
)";

/// Row shape for the `config` table.
#[derive(Debug, FromRow)]
struct ConfigRow {
    value: String,
}

/// SQLite-backed configuration store.
pub struct SqliteConfigStore {
    pool: SqlitePool,
}

impl SqliteConfigStore {
    /// Create a new store over the given pool, ensuring the `config` table
    /// exists.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        sqlx::query(SCHEMA_CONFIG).execute(&pool).await?;
        Ok(Self { pool })
    }

    /// Borrow the underlying pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Typed get helper: reads the JSON text for `key` and deserializes it
    /// into `T`.
    async fn get_typed<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        match self.get_value(key).await? {
            Some(value) => {
                let typed = serde_json::from_value(value)?;
                Ok(Some(typed))
            }
            None => Ok(None),
        }
    }

    /// Typed set helper: serializes `value` to JSON and stores it under
    /// `key`.
    async fn set_typed<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<()> {
        let json = serde_json::to_value(value)?;
        self.set_value(key, &json).await
    }
}

#[async_trait]
impl ConfigStoreBackend for SqliteConfigStore {
    async fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let row: Option<ConfigRow> =
            sqlx::query_as::<_, ConfigRow>("SELECT value FROM config WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            Some(r) => {
                let value = serde_json::from_str(&r.value)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn set_value(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let json = serde_json::to_string(value)?;
        sqlx::query("INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(&json)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM config WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn load_config(&self) -> Result<PersistedConfig> {
        let mut config = PersistedConfig::default();

        if let Some(v) = self.get_typed::<String>("proxy_addr").await? {
            config.proxy_addr = v;
        }
        if let Some(v) = self.get_typed::<String>("api_addr").await? {
            config.api_addr = v;
        }
        if let Some(v) = self.get_typed::<String>("cert_dir").await? {
            config.cert_dir = v;
        }
        if let Some(v) = self.get_typed::<String>("data_dir").await? {
            config.data_dir = v;
        }
        if let Some(v) = self.get_typed::<String>("log_level").await? {
            config.log_level = v;
        }
        if let Some(v) = self.get_typed::<String>("theme").await? {
            config.theme = v;
        }
        if let Some(v) = self.get_typed::<(u32, u32)>("window_size").await? {
            config.window_size = Some(v);
        }
        if let Some(v) = self.get_typed::<Vec<u32>>("column_widths").await? {
            config.column_widths = Some(v);
        }
        if let Some(v) = self.get_typed::<serde_json::Value>("custom").await? {
            config.custom = v;
        }

        Ok(config)
    }

    async fn save_config(&self, config: &PersistedConfig) -> Result<()> {
        self.set_typed("proxy_addr", &config.proxy_addr).await?;
        self.set_typed("api_addr", &config.api_addr).await?;
        self.set_typed("cert_dir", &config.cert_dir).await?;
        self.set_typed("data_dir", &config.data_dir).await?;
        self.set_typed("log_level", &config.log_level).await?;
        self.set_typed("theme", &config.theme).await?;
        if let Some(ref v) = config.window_size {
            self.set_typed("window_size", v).await?;
        }
        if let Some(ref v) = config.column_widths {
            self.set_typed("column_widths", v).await?;
        }
        self.set_typed("custom", &config.custom).await?;
        Ok(())
    }

    async fn export(&self) -> Result<String> {
        let config = self.load_config().await?;
        Ok(serde_json::to_string_pretty(&config)?)
    }

    async fn import(&self, json: &str) -> Result<()> {
        let config: PersistedConfig = serde_json::from_str(json)?;
        self.save_config(&config).await?;
        Ok(())
    }
}

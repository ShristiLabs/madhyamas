//! Persistence layer for intercept rules and configuration

mod config_store;

pub use config_store::PersistedConfig;

// The former `InterceptStore` (rusqlite) has been migrated to
// `SqliteInterceptStore` (sqlx) in `crate::storage::sqlite::intercept`.

/// Common persistence interface for in-memory managers.
///
/// `ReplayManager`, `WsManager`, and `GrpcManager` each hold
/// `RwLock<Vec<_>>` / `RwLock<HashMap<_, _>>` collections independently. This
/// trait gives them a uniform `save` / `load` / `clear` / `size` surface so a
/// future shared storage backend can drive them through a single API.
///
/// # Current status
///
/// There is no shared storage backend wired up yet, so `save` and `load` are
/// in-memory no-ops (`Ok(())`). `clear` and `size` operate on the live
/// in-memory state.
#[async_trait::async_trait]
pub trait Persistable {
    /// Persist the current in-memory state to the backing store.
    async fn save(&self) -> crate::Result<()>;

    /// Load state from the backing store, replacing current in-memory data.
    async fn load(&self) -> crate::Result<()>;

    /// Remove all persisted data and clear in-memory state.
    async fn clear(&self) -> crate::Result<()>;

    /// Number of items currently tracked in memory.
    fn size(&self) -> usize;
}

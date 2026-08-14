//! PostgreSQL-backed storage implementations using [`sqlx::PgPool`].
//!
//! Each module provides a concrete `PostgresXStore` that implements the
//! corresponding async backend trait from [`crate::storage`]. All SQL uses
//! runtime `sqlx::query` / `sqlx::query_as::<_, T>` strings with `$N`
//! placeholders (PostgreSQL parameter style) so the crate builds without a
//! database at build time. The traffic store schema includes optimized
//! indexes (GIN on JSONB headers, trigram on URL, BRIN on timestamp) and a
//! tiered body storage table per `docs/ENTERPRISE_PERF_SECURITY.md` §6.

pub mod config;
pub mod intercept;
#[cfg(feature = "plugins")]
pub mod plugin;
#[cfg(feature = "scripting")]
pub mod script;
pub mod traffic;

pub use config::PostgresConfigStore;
pub use intercept::PostgresInterceptStore;
#[cfg(feature = "plugins")]
pub use plugin::PostgresPluginStore;
#[cfg(feature = "scripting")]
pub use script::PostgresScriptStore;
pub use traffic::PostgresTrafficStore;

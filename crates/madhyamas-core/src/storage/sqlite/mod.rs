//! SQLite-backed storage implementations using [`sqlx::SqlitePool`].
//!
//! Each module provides a concrete `SqliteXStore` that implements the
//! corresponding async backend trait from [`crate::storage`]. All SQL uses
//! runtime `sqlx::query` / `sqlx::query_as::<_, T>` strings (not the
//! compile-time `query!` macro) so the crate builds without a database at
//! build time. This module is the Phase 2c migration target; stores are
//! migrated here one at a time, smallest first.

pub mod config;
pub mod intercept;
#[cfg(feature = "plugins")]
pub mod plugin;
#[cfg(feature = "scripting")]
pub mod script;

pub use config::SqliteConfigStore;
pub use intercept::SqliteInterceptStore;
#[cfg(feature = "plugins")]
pub use plugin::SqlitePluginStore;
#[cfg(feature = "scripting")]
pub use script::SqliteScriptStore;

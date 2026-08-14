//! Script persistence — migrated to sqlx.
//!
//! The former sync `ScriptPersistence` struct has been replaced by the
//! async [`crate::storage::SqliteScriptStore`] (implementing
//! [`crate::storage::ScriptStoreBackend`]) in Phase 2c. Script and
//! execution row types ([`Script`], [`ScriptExecution`],
//! [`ScriptErrorPolicy`]) live in [`super::runtime`].

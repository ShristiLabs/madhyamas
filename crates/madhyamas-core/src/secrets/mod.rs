//! Secrets and environment-variable exposure for plugins and scripts (#87).
//!
//! # Design (v1)
//!
//! - **Substitution only**: `${ENV:VAR}` and `${SECRET:name}` placeholders are
//!   expanded at load/enable time in plugin settings, script source, and
//!   plugin manifest config. There is intentionally **no** runtime `env_get`
//!   host function and no `madhyamas.env.get` script API — secrets stay out
//!   of serialized contexts, script traces, and guest memory beyond the
//!   substituted values themselves.
//! - **Deny-by-default per-name grants**: plugin manifests
//!   (`madhyamas-plugin.toml`) and script records declare which env var /
//!   secret names they may receive. Only granted names are substituted; a
//!   plugin or script without grants sees its placeholders untouched. The raw
//!   process environment is never exposed wholesale.
//! - **Storage**: the OSS tier uses an encrypted-at-rest file keystore
//!   ([`FileKeystore`]); the enterprise tier stores secrets in the
//!   enterprise store (PostgreSQL/SQLite) behind its RBAC/auth layer and
//!   audit trail. Both implement [`SecretStore`].
//! - **Write-only values**: secret values are never returned in plaintext by
//!   any management API endpoint (same semantics as `auth_password`).
//!   Listing secret *names* is allowed.
//! - **Redaction**: known secret values plus configurable header patterns are
//!   redacted from traffic capture, HAR export, plugin logs, and script
//!   traces via the shared [`Redactor`].
//! - **No rotation/TTL in v1**: secrets are updated manually via the API/UI.

pub mod keystore;
pub mod redaction;
pub mod service;
pub mod substitution;

pub use keystore::FileKeystore;
pub use redaction::Redactor;
pub use service::{SecretAuditEvent, SecretAuditSink, SecretService, SecretStore};
pub use substitution::{expand_grants_str, expand_json, expand_str, Grants};

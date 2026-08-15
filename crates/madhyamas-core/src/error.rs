//! Unified application error trait.
//!
//! This module defines the [`AppError`] trait, which provides a common
//! interface across the different error types used throughout the Madhyamas
//! workspace:
//!
//! - [`crate::Error`] — the core proxy engine error enum.
//! - [`madhyamas_mcp::types::McpError`] — MCP protocol errors.
//!
//! The trait bridges these otherwise-unrelated error types so that the API
//! layer (and any other consumer) can treat them uniformly when producing
//! HTTP responses, deciding whether to retry, or logging stable error codes.
//!
//! The trait lives in `madhyamas-core` so that both `madhyamas-api` and
//! `madhyamas-mcp` can depend on it without introducing circular
//! dependencies.

use serde_json::{json, Value};

/// A unified error trait bridging the core and MCP error types.
///
/// All implementors must also implement [`std::error::Error`] and be
/// `Send + Sync + 'static` so they can be used across threads and stored in
/// trait objects (e.g. `Box<dyn AppError>`).
pub trait AppError: std::error::Error + Send + Sync + 'static {
    /// A stable, machine-readable string code identifying the error category.
    ///
    /// This is intended for logging, metrics, and API response `code` fields.
    /// It should remain stable across releases for a given error category.
    fn error_code(&self) -> &str;

    /// Whether the operation that produced this error can reasonably be
    /// retried by the caller.
    fn is_retryable(&self) -> bool;

    /// A JSON representation suitable for inclusion in API error responses.
    ///
    /// The default implementation produces an object with `code`, `message`,
    /// and `retryable` fields derived from [`error_code`](AppError::error_code),
    /// the [`Display`](std::fmt::Display) representation, and
    /// [`is_retryable`](AppError::is_retryable) respectively. Implementors may
    /// override this to add extra structured context.
    fn as_response_json(&self) -> Value {
        json!({
            "code": self.error_code(),
            "message": self.to_string(),
            "retryable": self.is_retryable(),
        })
    }
}

// ---------------------------------------------------------------------------
// Implementation for the core `Error` enum
// ---------------------------------------------------------------------------

impl AppError for crate::Error {
    fn error_code(&self) -> &str {
        match self {
            crate::Error::Io(_) => "CORE_IO",
            crate::Error::Tls(_) => "CORE_TLS",
            crate::Error::Certificate(_) => "CORE_CERTIFICATE",
            crate::Error::Sqlx(_) => "CORE_DATABASE",
            crate::Error::Serialization(_) => "CORE_SERIALIZATION",
            crate::Error::Proxy(_) => "CORE_PROXY",
            crate::Error::Config(_) => "CORE_CONFIG",
            crate::Error::Channel(_) => "CORE_CHANNEL",
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            // Transient infrastructure failures are generally retryable.
            crate::Error::Io(_) | crate::Error::Channel(_) => true,
            // Configuration, serialization, certificate, and database schema
            // issues are unlikely to resolve themselves on retry.
            crate::Error::Tls(_)
            | crate::Error::Certificate(_)
            | crate::Error::Sqlx(_)
            | crate::Error::Serialization(_)
            | crate::Error::Proxy(_)
            | crate::Error::Config(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_codes_are_stable() {
        assert_eq!(crate::Error::Config("x".into()).error_code(), "CORE_CONFIG");
        assert_eq!(
            crate::Error::Io(std::io::Error::other("boom")).error_code(),
            "CORE_IO"
        );
    }

    #[test]
    fn core_io_is_retryable() {
        assert!(crate::Error::Io(std::io::Error::other("boom")).is_retryable());
        assert!(!crate::Error::Config("x".into()).is_retryable());
    }

    #[test]
    fn response_json_contains_expected_fields() {
        let err = crate::Error::Config("bad value".into());
        let json = err.as_response_json();
        assert_eq!(json["code"], "CORE_CONFIG");
        assert_eq!(json["retryable"], false);
        assert!(json["message"].as_str().unwrap().contains("bad value"));
    }
}

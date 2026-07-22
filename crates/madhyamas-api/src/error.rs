//! Unified API error type.
//!
//! All API handlers should return `Result<T, ApiError>` (or `ApiError` directly)
//! so that error responses have a consistent JSON shape:
//!
//! ```json
//! { "error": "human-readable message" }
//! ```

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use madhyamas_core::error::AppError;
use serde::Serialize;

/// Standard error response body.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

/// Unified API error type. Converts into a `(StatusCode, Json<ErrorBody>)` response.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, msg)
    }

    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, msg)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::bad_request(format!("JSON error: {}", e))
    }
}

/// Convert any [`AppError`] into an [`ApiError`].
///
/// The HTTP status code is chosen based on the stable [`AppError::error_code`]
/// prefix so that, for example, enterprise auth/permission failures map to
/// `401`/`403` while core infrastructure failures map to `500`/`503`.
impl From<Box<dyn AppError>> for ApiError {
    fn from(err: Box<dyn AppError>) -> Self {
        let code = err.error_code();
        let status = status_for_code(code);
        let message = err.to_string();
        Self::new(status, message)
    }
}

/// Convert any [`AppError`] into an [`ApiError`] by boxing it first.
///
/// This is the ergonomic entry point for handlers that have a concrete
/// error type implementing [`AppError`]:
///
/// ```ignore
/// fn handler() -> ApiResult<Json<Value>> {
///     do_something().map_err(ApiError::from_app_error)?;
///     Ok(Json(json!({})))
/// }
/// ```
impl ApiError {
    /// Convert any [`AppError`] into an [`ApiError`].
    pub fn from_app_error<E: AppError>(err: E) -> Self {
        let code = err.error_code();
        let status = status_for_code(code);
        let message = err.to_string();
        Self::new(status, message)
    }
}

/// Map a stable error code to an appropriate HTTP status code.
fn status_for_code(code: &str) -> StatusCode {
    if let Some(rest) = code.strip_prefix("ENTERPRISE_") {
        match rest {
            "AUTH_FAILED" | "TOKEN_EXPIRED" | "JWT_ERROR" => StatusCode::UNAUTHORIZED,
            "PERMISSION_DENIED" => StatusCode::FORBIDDEN,
            "USER_NOT_FOUND" | "ROLE_NOT_FOUND" => StatusCode::NOT_FOUND,
            "AUDIT_ERROR" => StatusCode::INTERNAL_SERVER_ERROR,
            "INVALID_CONFIG" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    } else if let Some(rest) = code.strip_prefix("MCP_") {
        match rest {
            "INVALID_PARAMS" | "PARSE" => StatusCode::BAD_REQUEST,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "HTTP" | "JSON_RPC" => StatusCode::SERVICE_UNAVAILABLE,
            "TOOL_EXECUTION" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    } else {
        // CORE_* errors
        match code {
            "CORE_CONFIG" | "CORE_SERIALIZATION" => StatusCode::BAD_REQUEST,
            "CORE_DATABASE" => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Convenience type alias for handler return types.
pub type ApiResult<T> = Result<T, ApiError>;

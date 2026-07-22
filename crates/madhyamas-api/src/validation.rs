//! Request body validation helpers.
//!
//! This module bridges the [`validator`] crate with the API crate's
//! [`ApiError`] type so that handlers can validate deserialized request
//! bodies in a single call and return a consistent `400 Bad Request`
//! response when validation fails.
//!
//! Custom validation functions live here for core types (e.g.
//! [`MatchCondition`], [`MockResponse`], [`ThrottleProfile`]) that are
//! defined in `madhyamas-core` and therefore cannot derive `Validate`
//! themselves without adding the `validator` dependency to the core crate.
//! Using `#[validate(custom(function = "..."))]` on the API-side request
//! wrappers keeps the dependency boundary clean.

use crate::error::ApiError;
use madhyamas_core::{
    ConditionalResponse, MatchCondition, MockResponse, ProbabilisticResponse, ResponseConfig,
    ThrottleProfile,
};
use validator::{Validate, ValidationError};

/// Validate a request body and convert any validation errors into an
/// [`ApiError::bad_request`].
///
/// ```ignore
/// pub async fn create_thing(Json(req): Json<CreateThingRequest>) -> impl IntoResponse {
///     if let Err(e) = validation::validate(&req) {
///         return e.into_response();
///     }
///     // ...
/// }
/// ```
pub fn validate<T: Validate>(t: &T) -> Result<(), ApiError> {
    t.validate()
        .map_err(|errors| ApiError::bad_request(errors.to_string()))
}

// ---------------------------------------------------------------------------
// Custom validation functions for core types
// ---------------------------------------------------------------------------

/// Ensure that any regex/URL pattern carried by a [`MatchCondition`] is
/// non-empty. This covers `UrlPattern`, `PathPattern`, `Host`, `BodyPattern`,
/// `ContentType`, `HeaderPattern` and `QueryParamPattern` variants.
pub fn validate_match_condition(condition: &MatchCondition) -> Result<(), ValidationError> {
    match condition {
        MatchCondition::UrlPattern { pattern }
        | MatchCondition::PathPattern { pattern }
        | MatchCondition::Host { pattern }
        | MatchCondition::BodyPattern { pattern }
        | MatchCondition::ContentType { pattern } => {
            if pattern.trim().is_empty() {
                return Err(ValidationError::new("url_pattern cannot be empty"));
            }
        }
        MatchCondition::HeaderPattern { name, pattern } => {
            if name.trim().is_empty() {
                return Err(ValidationError::new("header name cannot be empty"));
            }
            if pattern.trim().is_empty() {
                return Err(ValidationError::new("header pattern cannot be empty"));
            }
        }
        MatchCondition::QueryParamPattern { name, pattern } => {
            if name.trim().is_empty() {
                return Err(ValidationError::new("query param name cannot be empty"));
            }
            if pattern.trim().is_empty() {
                return Err(ValidationError::new("query param pattern cannot be empty"));
            }
        }
        MatchCondition::Header { name, .. } | MatchCondition::QueryParam { name, .. } => {
            if name.trim().is_empty() {
                return Err(ValidationError::new("name cannot be empty"));
            }
        }
        MatchCondition::And { conditions } | MatchCondition::Or { conditions } => {
            for c in conditions {
                validate_match_condition(c)?;
            }
        }
        MatchCondition::Not { condition } => validate_match_condition(condition)?,
        _ => {}
    }
    Ok(())
}

/// Validate that a [`MockResponse`] has a sensible HTTP status code
/// (100-599 inclusive).
pub fn validate_mock_response(response: &MockResponse) -> Result<(), ValidationError> {
    validate_status_code(response.status_code)
}

/// Validate the status code of every response contained in a
/// [`ResponseConfig`].
pub fn validate_response_config(config: &ResponseConfig) -> Result<(), ValidationError> {
    match config {
        ResponseConfig::Single { response } => validate_mock_response(response),
        ResponseConfig::Sequence { responses, .. } => {
            for r in responses {
                validate_mock_response(r)?;
            }
            Ok(())
        }
        ResponseConfig::Conditional {
            conditions,
            default_response,
        } => {
            validate_mock_response(default_response)?;
            for ConditionalResponse { response, .. } in conditions {
                validate_mock_response(response)?;
            }
            Ok(())
        }
        ResponseConfig::Probabilistic { responses } => {
            for ProbabilisticResponse { response, .. } in responses {
                validate_mock_response(response)?;
            }
            Ok(())
        }
    }
}

/// Validate a single HTTP status code is within the valid range.
pub fn validate_status_code(code: u16) -> Result<(), ValidationError> {
    if !(100..=599).contains(&code) {
        return Err(ValidationError::new(
            "status_code must be between 100 and 599",
        ));
    }
    Ok(())
}

/// Validate a [`ThrottleProfile`] has reasonable values:
/// - `latency_ms` between 0 and 60000 (1 minute)
/// - `jitter_ms` between 0 and 60000
/// - `packet_loss_percent` between 0 and 100
/// - bandwidth values (`download_bps`/`upload_bps`) not absurdly large
///   (capped at 1 Tbps; 0 means unlimited)
pub fn validate_throttle_profile(profile: &ThrottleProfile) -> Result<(), ValidationError> {
    const MAX_LATENCY_MS: u64 = 60_000;
    const MAX_BANDWIDTH_BPS: u64 = 1_000_000_000_000; // 1 Tbps

    if profile.latency_ms > MAX_LATENCY_MS {
        return Err(ValidationError::new(
            "latency_ms must be between 0 and 60000",
        ));
    }
    if profile.jitter_ms > MAX_LATENCY_MS {
        return Err(ValidationError::new(
            "jitter_ms must be between 0 and 60000",
        ));
    }
    if profile.packet_loss_percent > 100 {
        return Err(ValidationError::new(
            "packet_loss_percent must be between 0 and 100",
        ));
    }
    if profile.download_bps > MAX_BANDWIDTH_BPS {
        return Err(ValidationError::new(
            "download_bps exceeds the maximum reasonable value (1 Tbps)",
        ));
    }
    if profile.upload_bps > MAX_BANDWIDTH_BPS {
        return Err(ValidationError::new(
            "upload_bps exceeds the maximum reasonable value (1 Tbps)",
        ));
    }
    Ok(())
}

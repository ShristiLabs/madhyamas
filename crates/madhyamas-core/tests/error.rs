//! Integration tests for the public error API: stable codes, retryability,
//! and the JSON response shape produced via the `AppError` trait.

use madhyamas_core::error::AppError;
use madhyamas_core::Error;

#[test]
fn core_error_codes_are_stable() {
    assert_eq!(Error::Config("x".into()).error_code(), "CORE_CONFIG");
    assert_eq!(
        Error::Io(std::io::Error::other("boom")).error_code(),
        "CORE_IO"
    );
}

#[test]
fn core_io_is_retryable() {
    assert!(Error::Io(std::io::Error::other("boom")).is_retryable());
    assert!(!Error::Config("x".into()).is_retryable());
}

#[test]
fn response_json_contains_expected_fields() {
    let err = Error::Config("bad value".into());
    let json = err.as_response_json();
    assert_eq!(json["code"], "CORE_CONFIG");
    assert_eq!(json["retryable"], false);
    assert!(json["message"].as_str().unwrap().contains("bad value"));
}

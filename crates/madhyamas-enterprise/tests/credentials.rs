//! Integration tests for the public credentials API: Argon2id hash/verify
//! and password complexity policy.

use madhyamas_enterprise::{hash_password, validate_password_complexity, verify_password};

#[test]
fn test_hash_and_verify() {
    let hash = hash_password("correct-horse-battery-staple").expect("hash ok");
    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password("correct-horse-battery-staple", &hash).expect("verify ok"));
}

#[test]
fn test_wrong_password() {
    let hash = hash_password("correct-horse-battery-staple").expect("hash ok");
    assert!(!verify_password("definitely-not-it", &hash).expect("verify ok"));
}

#[test]
fn test_malformed_hash() {
    let result = verify_password("anything", "not-a-valid-phc-string");
    assert!(result.is_err(), "malformed hash should return Err");
}

#[test]
fn test_password_complexity_valid() {
    assert!(validate_password_complexity("Abcdefg123!@#").is_ok());
    assert!(validate_password_complexity("Str0ng!Pass123").is_ok());
    assert!(validate_password_complexity("Aa1!aaaaaaaaaa").is_ok());
}

#[test]
fn test_password_complexity_too_short() {
    let err = validate_password_complexity("Ab1!short").expect_err("too short");
    assert!(
        err.to_string().contains("at least 12"),
        "expected length message, got: {err}"
    );
}

#[test]
fn test_password_complexity_no_uppercase() {
    let err = validate_password_complexity("abcdefg123!@#x").expect_err("no uppercase");
    assert!(
        err.to_string().contains("uppercase"),
        "expected uppercase message, got: {err}"
    );
}

#[test]
fn test_password_complexity_no_lowercase() {
    let err = validate_password_complexity("ABCDEFG123!@#X").expect_err("no lowercase");
    assert!(
        err.to_string().contains("lowercase"),
        "expected lowercase message, got: {err}"
    );
}

#[test]
fn test_password_complexity_no_digit() {
    let err = validate_password_complexity("Abcdefgh!@#xyz").expect_err("no digit");
    assert!(
        err.to_string().contains("digit"),
        "expected digit message, got: {err}"
    );
}

#[test]
fn test_password_complexity_no_special() {
    let err = validate_password_complexity("Abcdefg123xyz").expect_err("no special");
    assert!(
        err.to_string().contains("special"),
        "expected special message, got: {err}"
    );
}

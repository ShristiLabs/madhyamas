//! Password hashing and verification using Argon2id (Phase 4a).
//!
//! Passwords are hashed with Argon2id (the recommended variant of the Argon2
//! family) using a fresh random salt per hash. The resulting PHC string
//! (`$argon2id$v=19$m=...,t=...,p=...$<salt>$<hash>`) is stored in the
//! `users.password_hash` column. Verification parses the PHC string and
//! re-runs Argon2id with the embedded parameters and salt.
//!
//! The public [`hash_password`] / [`verify_password`] functions are the only
//! credential primitives the rest of the crate uses; plaintext passwords are
//! never logged or persisted.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

use super::enterprise_error::EnterpriseError;

/// Special characters allowed in passwords (for the complexity check).
const SPECIAL_CHARS: &[char] = &[
    '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '-', '=', '[', ']', '{', '}', '|',
    ';', '\'', ':', '"', ',', '.', '/', '<', '>', '?',
];

/// Minimum password length enforced by [`validate_password_complexity`].
pub const MIN_PASSWORD_LENGTH: usize = 12;

/// Validate that a password meets the complexity policy (Phase 9.9):
/// - At least 12 characters
/// - At least 1 uppercase letter (A–Z)
/// - At least 1 lowercase letter (a–z)
/// - At least 1 digit (0–9)
/// - At least 1 special character (!@#$%^&*()_+-=[]{}|;':",./<>?)
///
/// Returns `Ok(())` when all checks pass, or an [`EnterpriseError`]
/// with a descriptive message identifying the first unmet requirement.
/// Call this before [`hash_password`] in `create_user` / `update_user`
/// handlers so weak passwords are rejected with a `400` before hashing.
pub fn validate_password_complexity(password: &str) -> Result<(), EnterpriseError> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(EnterpriseError::InvalidConfig {
            message: format!("Password must be at least {MIN_PASSWORD_LENGTH} characters long"),
        });
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(EnterpriseError::InvalidConfig {
            message: "Password must contain at least 1 uppercase letter".to_string(),
        });
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(EnterpriseError::InvalidConfig {
            message: "Password must contain at least 1 lowercase letter".to_string(),
        });
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(EnterpriseError::InvalidConfig {
            message: "Password must contain at least 1 digit".to_string(),
        });
    }
    if !password.chars().any(|c| SPECIAL_CHARS.contains(&c)) {
        return Err(EnterpriseError::InvalidConfig {
            message: "Password must contain at least 1 special character".to_string(),
        });
    }
    Ok(())
}

/// Hash a plaintext password with Argon2id and a fresh random salt.
///
/// Returns the PHC string (`$argon2id$...`), which encodes the algorithm,
/// version, memory/time/parallelism parameters, salt, and derived hash. Store
/// the full string — verification reads the parameters back out of it.
pub fn hash_password(password: &str) -> Result<String, EnterpriseError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| EnterpriseError::InvalidConfig {
            message: format!("password hashing failed: {e}"),
        })?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored PHC string.
///
/// Returns `Ok(true)` on a match, `Ok(false)` on a mismatch (the caller
/// decides how to handle a failed verification — typically a 401), and
/// `Err` only when the stored hash is malformed and cannot be parsed.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, EnterpriseError> {
    let parsed = PasswordHash::new(hash).map_err(|e| EnterpriseError::InvalidConfig {
        message: format!("malformed password hash: {e}"),
    })?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

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
}

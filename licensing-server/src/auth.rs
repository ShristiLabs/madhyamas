//! Authentication primitives for the licensing server.
//!
//! Provides JWT issuance/verification (for both customer and admin sessions)
//! and Argon2id password hashing. JWTs carry the subject (account or admin
//! UUID), a `kind` claim distinguishing customer vs admin tokens, and an
//! optional role claim for admins.
//!
//! # Configuration
//!
//! The signing secret is read from the `JWT_SECRET` environment variable,
//! falling back to a development-only secret when unset. **Set
//! `JWT_SECRET` to a strong random value in production.**

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Default development JWT secret. **Never use in production.**
const DEV_JWT_SECRET: &str = "madhyamas-licensing-dev-secret-change-me";

/// Token lifetime: 7 days for customer sessions.
const CUSTOMER_TOKEN_TTL_DAYS: i64 = 7;
/// Token lifetime: 1 day for admin sessions.
const ADMIN_TOKEN_TTL_DAYS: i64 = 1;

/// Distinguishes customer vs admin JWTs in the `kind` claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    Customer,
    Admin,
}

/// Claims embedded in every JWT issued by the licensing server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the account UUID (customer) or admin UUID (admin).
    pub sub: String,
    /// Token kind: `customer` or `admin`.
    pub kind: TokenKind,
    /// Admin role (only present for admin tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Expiry (UNIX timestamp).
    pub exp: usize,
    /// Issued-at (UNIX timestamp).
    pub iat: usize,
}

/// Authentication error.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("password hash error: {0}")]
    Hash(String),
    #[error("password verify error: {0}")]
    Verify(String),
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("invalid token: {0}")]
    InvalidToken(String),
}

/// Hash a plaintext password using Argon2id. Returns a PHC-string suitable
/// for storage in a `password_hash` column.
pub fn hash_password(plaintext: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(plaintext.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Hash(e.to_string()))
}

/// Verify a plaintext password against a stored PHC-string hash.
pub fn verify_password(plaintext: &str, phc_hash: &str) -> Result<(), AuthError> {
    let parsed = PasswordHash::new(phc_hash).map_err(|e| AuthError::Verify(e.to_string()))?;
    Argon2::default()
        .verify_password(plaintext.as_bytes(), &parsed)
        .map_err(|e| AuthError::Verify(e.to_string()))
}

/// Read the JWT signing secret from the environment, falling back to the
/// development secret.
pub fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| DEV_JWT_SECRET.to_string())
}

/// Issue a customer JWT for the given account UUID.
pub fn issue_customer_token(account_id: Uuid) -> Result<String, AuthError> {
    let now = Utc::now();
    let exp = now + Duration::days(CUSTOMER_TOKEN_TTL_DAYS);
    let claims = Claims {
        sub: account_id.to_string(),
        kind: TokenKind::Customer,
        role: None,
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map_err(Into::into)
}

/// Issue an admin JWT for the given admin UUID and role.
pub fn issue_admin_token(admin_id: Uuid, role: &str) -> Result<String, AuthError> {
    let now = Utc::now();
    let exp = now + Duration::days(ADMIN_TOKEN_TTL_DAYS);
    let claims = Claims {
        sub: admin_id.to_string(),
        kind: TokenKind::Admin,
        role: Some(role.to_string()),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map_err(Into::into)
}

/// Verify a JWT and return the decoded claims.
pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

/// Extract and verify a bearer token from an Authorization header value
/// string (e.g. "Bearer eyJ..."). Returns the claims on success.
pub fn extract_bearer(auth_header: Option<&str>) -> Result<Claims, AuthError> {
    let raw = auth_header.ok_or_else(|| AuthError::InvalidToken("missing header".into()))?;
    let token = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .ok_or_else(|| AuthError::InvalidToken("not a bearer token".into()))?;
    verify_token(token)
}

#[cfg(test)]
mod tests {
    //! Auth primitive tests — password hashing and JWT round-trip.

    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let hash = hash_password("hunter2").expect("hash");
        assert_ne!(hash, "hunter2");
        verify_password("hunter2", &hash).expect("verify should succeed");
        assert!(verify_password("wrong", &hash).is_err());
    }

    #[test]
    fn test_customer_token_roundtrip() {
        let id = Uuid::new_v4();
        let token = issue_customer_token(id).expect("issue");
        let claims = verify_token(&token).expect("verify");
        assert_eq!(claims.sub, id.to_string());
        assert_eq!(claims.kind, TokenKind::Customer);
        assert!(claims.role.is_none());
    }

    #[test]
    fn test_admin_token_roundtrip() {
        let id = Uuid::new_v4();
        let token = issue_admin_token(id, "super_admin").expect("issue");
        let claims = verify_token(&token).expect("verify");
        assert_eq!(claims.sub, id.to_string());
        assert_eq!(claims.kind, TokenKind::Admin);
        assert_eq!(claims.role.as_deref(), Some("super_admin"));
    }

    #[test]
    fn test_extract_bearer() {
        let id = Uuid::new_v4();
        let token = issue_customer_token(id).expect("issue");
        let claims = extract_bearer(Some(&format!("Bearer {token}"))).expect("extract");
        assert_eq!(claims.sub, id.to_string());
        assert!(extract_bearer(Some("Token abc")).is_err());
        assert!(extract_bearer(None).is_err());
    }
}

//! Ed25519 license signing for the licensing server.
//!
//! This module produces Ed25519-signed license files in the **exact same
//! format** the Madhyamas proxy binary verifies (see
//! `crates/madhyamas-enterprise/src/license.rs`). The licensing server holds
//! the Ed25519 **private** signing key and never exposes it; proxy instances
//! hold only the corresponding **public** key (distributed via the
//! `MADHYAMAS_LICENSE_PUBLIC_KEY` environment variable).
//!
//! # License file format
//!
//! ```json
//! {
//!   "license_id": "lic_abc123",
//!   "customer": "Acme Corp",
//!   "plan": "enterprise",
//!   "seats": 50,
//!   "instance_id": "inst_xyz789",
//!   "issued_at": "2026-01-01T00:00:00Z",
//!   "expires_at": "2027-01-01T00:00:00Z",
//!   "features": ["auth", "rbac", "audit", "multi_instance", "oidc"],
//!   "signature": "base64_ed25519_signature_of_canonical_json"
//! }
//! ```
//!
//! # Canonical JSON contract
//!
//! The signature is computed over the claims **only** (the `signature` field
//! is excluded). The canonical form is: [`serde_json::Value`] with all object
//! keys sorted recursively (lexicographically by UTF-8 byte order) and
//! serialized with [`serde_json::to_string`] (compact, no whitespace). Array
//! element order is preserved. This is identical to the proxy's canonical JSON
//! routine — the two implementations MUST produce the same bytes for a given
//! set of claims.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The claims embedded in a license file (the JSON payload that gets signed).
///
/// This is a byte-for-byte compatible copy of
/// `madhyamas_enterprise::LicenseClaims` — same fields, same serde derive
/// order, same types — so that the canonical JSON produced here is identical
/// to what the proxy's verifier expects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    pub license_id: String,
    pub customer: String,
    pub plan: String,
    pub seats: u32,
    pub instance_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub features: Vec<String>,
}

/// A complete license file: claims + Ed25519 signature (base64).
///
/// The `signature` field is flattened alongside the claims in the JSON, so the
/// on-disk format is a single flat object (see the module docs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    #[serde(flatten)]
    pub claims: LicenseClaims,
    /// base64-encoded detached Ed25519 signature over the canonical JSON of
    /// [`LicenseClaims`].
    pub signature: String,
}

/// License signing error.
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("license signing IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid private key: {0}")]
    InvalidKey(String),
    #[error("canonical JSON serialization error: {0}")]
    Canonical(#[from] serde_json::Error),
    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("license signature invalid")]
    InvalidSignature,
    #[error("license expired at {expires_at}")]
    Expired { expires_at: DateTime<Utc> },
}

/// Holds the Ed25519 private signing key and signs license claims.
///
/// Construct with [`LicenseSigner::new`] (raw 32-byte key),
/// [`LicenseSigner::from_file`] (load from disk), or
/// [`LicenseSigner::generate`] (create a fresh keypair for first-run setup).
pub struct LicenseSigner {
    signing_key: SigningKey,
}

impl LicenseSigner {
    /// Construct a signer from a raw 32-byte Ed25519 private key.
    pub fn new(private_key_bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(private_key_bytes);
        Self { signing_key }
    }

    /// Load a private key from a file. The file may contain either 32 raw
    /// bytes or a base64-encoded 32-byte string (whitespace is trimmed).
    pub fn from_file(path: &Path) -> Result<Self, SignError> {
        let contents = std::fs::read(path)?;
        let bytes = if contents.len() == 32 {
            // Raw 32 bytes.
            contents
        } else {
            // Try base64 — trim whitespace first.
            let trimmed = std::str::from_utf8(&contents)
                .map_err(|e| {
                    SignError::InvalidKey(format!("file is not valid UTF-8 or raw bytes: {e}"))
                })?
                .trim();
            BASE64.decode(trimmed.as_bytes())?
        };
        if bytes.len() != 32 {
            return Err(SignError::InvalidKey(format!(
                "expected 32-byte private key, got {} bytes",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self::new(&arr))
    }

    /// Generate a fresh Ed25519 keypair. Returns the signer (private key) and
    /// the corresponding verifying (public) key. Used for first-run setup and
    /// key rotation.
    pub fn generate() -> (Self, VerifyingKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        (Self { signing_key }, verifying_key)
    }

    /// Return the public (verifying) key corresponding to this signing key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Return the raw 32-byte private key.
    pub fn raw_private_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign a set of license claims: serialize to canonical JSON, sign with
    /// Ed25519, and return a complete [`LicenseFile`] with the base64
    /// signature.
    pub fn sign_license(&self, claims: &LicenseClaims) -> Result<LicenseFile, SignError> {
        let canonical = canonical_json(claims)?;
        let signature = self.signing_key.sign(canonical.as_bytes());
        Ok(LicenseFile {
            claims: claims.clone(),
            signature: BASE64.encode(signature.to_bytes()),
        })
    }
}

/// Verify a license file's signature and expiry using a public key.
///
/// This mirrors the proxy's `LicenseVerifier::verify_claims` so the licensing
/// server can self-verify (e.g. for the `/api/licenses/verify` endpoint).
pub fn verify_license(file: &LicenseFile, public_key: &VerifyingKey) -> Result<(), SignError> {
    let canonical = canonical_json(&file.claims)?;
    let sig_bytes = BASE64.decode(file.signature.as_bytes())?;
    if sig_bytes.len() != 64 {
        return Err(SignError::InvalidSignature);
    }
    let signature = Signature::from_bytes(
        sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| SignError::InvalidSignature)?,
    );
    public_key
        .verify(canonical.as_bytes(), &signature)
        .map_err(|_| SignError::InvalidSignature)?;
    if Utc::now() > file.claims.expires_at {
        return Err(SignError::Expired {
            expires_at: file.claims.expires_at,
        });
    }
    Ok(())
}

/// Serialize [`LicenseClaims`] to canonical JSON: a [`serde_json::Value`] with
/// all object keys sorted recursively (lexicographic UTF-8 byte order),
/// serialized compactly via [`serde_json::to_string`].
///
/// This is the byte sequence the proxy's verifier re-computes to check the
/// signature. It MUST be identical to the proxy's `canonical_json`.
fn canonical_json(claims: &LicenseClaims) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(claims)?;
    sort_json_keys(&mut value);
    serde_json::to_string(&value)
}

/// Recursively sort the keys of every JSON object in `value` (in place).
///
/// Arrays are traversed but their element order is preserved. Non-object
/// values are left untouched.
fn sort_json_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: std::collections::BTreeMap<String, serde_json::Value> = std::mem::take(map)
                .into_iter()
                .map(|(k, mut v)| {
                    sort_json_keys(&mut v);
                    (k, v)
                })
                .collect();
            let rebuilt: serde_json::Map<String, serde_json::Value> = sorted.into_iter().collect();
            *map = rebuilt;
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sort_json_keys(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    //! License signing and verification tests.
    //!
    //! These exercise the signer, the verifier, and canonical JSON
    //! compatibility without requiring a database connection.

    use super::*;
    use chrono::Duration;

    fn sample_claims() -> LicenseClaims {
        LicenseClaims {
            license_id: "lic_test_001".to_string(),
            customer: "Acme Corp".to_string(),
            plan: "enterprise".to_string(),
            seats: 50,
            instance_id: "inst_xyz789".to_string(),
            issued_at: Utc::now() - Duration::days(1),
            expires_at: Utc::now() + Duration::days(365),
            features: vec!["auth".to_string(), "rbac".to_string(), "audit".to_string()],
        }
    }

    #[test]
    fn test_sign_and_verify() {
        let (signer, public_key) = LicenseSigner::generate();
        let claims = sample_claims();
        let file = signer.sign_license(&claims).expect("sign");
        verify_license(&file, &public_key).expect("verify should succeed");
    }

    #[test]
    fn test_verify_expired() {
        let (signer, public_key) = LicenseSigner::generate();
        let mut claims = sample_claims();
        claims.expires_at = Utc::now() - Duration::days(1);
        let file = signer.sign_license(&claims).expect("sign");
        let err = verify_license(&file, &public_key).expect_err("should be expired");
        assert!(matches!(err, SignError::Expired { .. }));
    }

    #[test]
    fn test_verify_tampered() {
        let (signer, public_key) = LicenseSigner::generate();
        let claims = sample_claims();
        let mut file = signer.sign_license(&claims).expect("sign");
        let mut sig_bytes = BASE64.decode(file.signature.as_bytes()).expect("decode");
        let last = sig_bytes.len() - 1;
        sig_bytes[last] ^= 0xff;
        file.signature = BASE64.encode(&sig_bytes);
        let err = verify_license(&file, &public_key).expect_err("tampered should fail");
        assert!(matches!(err, SignError::InvalidSignature));
    }

    #[test]
    fn test_verify_wrong_key() {
        let (signer, _public_key) = LicenseSigner::generate();
        let claims = sample_claims();
        let file = signer.sign_license(&claims).expect("sign");
        let (other, other_public) = LicenseSigner::generate();
        let _ = other;
        let err = verify_license(&file, &other_public).expect_err("wrong key should fail");
        assert!(matches!(err, SignError::InvalidSignature));
    }

    #[test]
    fn test_canonical_json_stable() {
        let claims = sample_claims();
        let canonical_a = canonical_json(&claims).expect("canonical a");
        let mut unordered = serde_json::json!({
            "features": claims.features,
            "expires_at": claims.expires_at,
            "issued_at": claims.issued_at,
            "instance_id": claims.instance_id,
            "seats": claims.seats,
            "plan": claims.plan,
            "customer": claims.customer,
            "license_id": claims.license_id,
        });
        sort_json_keys(&mut unordered);
        let canonical_b = serde_json::to_string(&unordered).expect("canonical b");
        assert_eq!(canonical_a, canonical_b);
    }

    #[test]
    fn test_from_file_raw_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("private.key");
        let (signer, public_key) = LicenseSigner::generate();
        std::fs::write(&path, signer.raw_private_key()).expect("write");
        let loaded = LicenseSigner::from_file(&path).expect("load");
        let claims = sample_claims();
        let file = loaded.sign_license(&claims).expect("sign");
        verify_license(&file, &public_key).expect("verify");
    }

    #[test]
    fn test_from_file_base64() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("private.b64");
        let (signer, public_key) = LicenseSigner::generate();
        let encoded = BASE64.encode(signer.raw_private_key());
        std::fs::write(&path, &encoded).expect("write");
        let loaded = LicenseSigner::from_file(&path).expect("load");
        let claims = sample_claims();
        let file = loaded.sign_license(&claims).expect("sign");
        verify_license(&file, &public_key).expect("verify");
    }

    #[test]
    fn test_license_file_roundtrip_json() {
        let (signer, _public_key) = LicenseSigner::generate();
        let claims = sample_claims();
        let file = signer.sign_license(&claims).expect("sign");
        let json = serde_json::to_string(&file).expect("serialize");
        let parsed: LicenseFile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.claims.license_id, claims.license_id);
        assert_eq!(parsed.signature, file.signature);
    }
}

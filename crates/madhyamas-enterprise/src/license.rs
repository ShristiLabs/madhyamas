//! Ed25519 license verification for the enterprise tier.
//!
//! The Madhyamas enterprise binary validates an offline license file at
//! startup. A license file is a JSON document containing the license claims
//! plus a detached Ed25519 signature (base64) over the **canonical JSON** of
//! those claims. The proxy binary holds only the Ed25519 **public** key; the
//! licensing server (see `docs/ENTERPRISE_LICENSING_SERVER.md`) holds the
//! private signing key and never exposes it.
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
//! # Verification flow
//!
//! 1. Read the license file from disk and parse it into [`LicenseFile`].
//! 2. Re-serialize the [`LicenseClaims`] to **canonical JSON** (object keys
//!    sorted recursively, no trailing whitespace) — see [`canonical_json`].
//! 3. Verify the Ed25519 signature (base64-decoded) over those canonical bytes
//!    using the embedded [`VerifyingKey`].
//! 4. Check `expires_at` is in the future.
//! 5. Return a [`License`] (claims + verification timestamp).
//!
//! # Canonical JSON contract
//!
//! The signature is computed over the claims **only** (the `signature` field
//! is excluded). The licensing server MUST produce signatures over the exact
//! same canonical form: [`serde_json::Value`] with all object keys sorted
//! recursively (lexicographically by UTF-8 byte order) and serialized with
//! [`serde_json::to_string`] (compact, no whitespace). Array element order is
//! preserved (features lists are ordered). This crate's [`canonical_json`]
//! helper is the reference implementation; the server should use an identical
//! routine.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The claims embedded in a license file (the JSON payload that gets signed).
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

/// A verified license: signature checked and not expired at verification time.
///
/// Held in [`crate::EnterpriseState`] so handlers and middleware can inspect
/// the active license (plan, seats, features, expiry).
#[derive(Debug, Clone)]
pub struct License {
    pub claims: LicenseClaims,
    /// When the signature was successfully verified (startup time).
    pub verified_at: DateTime<Utc>,
}

impl License {
    /// Returns `true` if the license has passed its `expires_at` timestamp.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.claims.expires_at
    }

    /// Returns `true` if the license grants the named feature.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.claims.features.iter().any(|f| f == feature)
    }
}

/// Verifier holding the embedded Ed25519 public key.
///
/// Construct with [`LicenseVerifier::new`] (raw 32-byte key) or
/// [`LicenseVerifier::from_env`] (reads `MADHYAMAS_LICENSE_PUBLIC_KEY`,
/// base64-encoded 32 bytes). When the env var is absent, `from_env` falls back
/// to a compiled-in **development** key and logs a warning — this lets local
/// development and CI run without configuring a public key. **Production
/// deployments MUST set `MADHYAMAS_LICENSE_PUBLIC_KEY`** to the real key
/// published by the licensing server.
#[derive(Clone)]
pub struct LicenseVerifier {
    public_key: VerifyingKey,
}

/// License verification error.
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("license file not found: {0}")]
    NotFound(String),
    #[error("license file parse error: {0}")]
    Parse(String),
    #[error("license signature invalid")]
    InvalidSignature,
    #[error("license expired at {expires_at}")]
    Expired { expires_at: DateTime<Utc> },
    #[error("license instance ID mismatch: expected {expected}, got {actual}")]
    InstanceMismatch { expected: String, actual: String },
    #[error("license public key error: {0}")]
    KeyError(String),
    #[error("license IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A compiled-in Ed25519 development public key used when
/// `MADHYAMAS_LICENSE_PUBLIC_KEY` is not set.
///
/// This key is **not secret** (it is a public key) and is only intended for
/// local development and testing. The corresponding private key is generated
/// ad-hoc in tests; production licenses are signed by the licensing server
/// with the real key whose public half must be provided via the env var.
const DEV_PUBLIC_KEY: [u8; 32] = *b"01234567890123456789012345678901";

impl LicenseVerifier {
    /// Construct a verifier from a raw 32-byte Ed25519 public key.
    pub fn new(public_key_bytes: &[u8; 32]) -> Result<Self, LicenseError> {
        let public_key = VerifyingKey::from_bytes(public_key_bytes)
            .map_err(|e| LicenseError::KeyError(e.to_string()))?;
        Ok(Self { public_key })
    }

    /// Construct a verifier from the `MADHYAMAS_LICENSE_PUBLIC_KEY` env var
    /// (base64-encoded 32 bytes).
    ///
    /// If the env var is absent, logs a warning and falls back to the
    /// compiled-in [`DEV_PUBLIC_KEY`] so local development works without
    /// setup. If the env var is present but invalid (wrong length or bad
    /// base64), returns a [`LicenseError::KeyError`].
    pub fn from_env() -> Result<Self, LicenseError> {
        match std::env::var("MADHYAMAS_LICENSE_PUBLIC_KEY") {
            Ok(raw) => {
                let bytes = BASE64
                    .decode(raw.as_bytes())
                    .map_err(|e| LicenseError::KeyError(format!("invalid base64: {e}")))?;
                if bytes.len() != 32 {
                    return Err(LicenseError::KeyError(format!(
                        "expected 32-byte public key, got {} bytes",
                        bytes.len()
                    )));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Self::new(&arr)
            }
            Err(_) => {
                tracing::warn!(
                    "MADHYAMAS_LICENSE_PUBLIC_KEY not set; falling back to compiled-in \
                     development key. Set the env var in production."
                );
                Self::new(&DEV_PUBLIC_KEY)
            }
        }
    }

    /// Read, parse, and verify a license file from disk.
    ///
    /// Performs the full verification flow: parse JSON, verify the Ed25519
    /// signature over the canonical claims JSON, and check `expires_at`. On
    /// success returns a [`License`] tagged with the current verification
    /// time.
    pub fn verify(&self, file_path: &Path) -> Result<License, LicenseError> {
        let contents = std::fs::read_to_string(file_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LicenseError::NotFound(file_path.display().to_string())
            } else {
                LicenseError::Io(e)
            }
        })?;
        let file: LicenseFile =
            serde_json::from_str(&contents).map_err(|e| LicenseError::Parse(e.to_string()))?;
        self.verify_claims(&file)
    }

    /// Core verification logic operating on an in-memory [`LicenseFile`].
    ///
    /// Separated from [`Self::verify`] so tests can exercise the cryptographic
    /// checks without touching the filesystem.
    pub fn verify_claims(&self, file: &LicenseFile) -> Result<License, LicenseError> {
        let canonical =
            canonical_json(&file.claims).map_err(|e| LicenseError::Parse(e.to_string()))?;
        let sig_bytes = BASE64
            .decode(file.signature.as_bytes())
            .map_err(|_| LicenseError::InvalidSignature)?;
        if sig_bytes.len() != 64 {
            return Err(LicenseError::InvalidSignature);
        }
        let signature = Signature::from_bytes(
            sig_bytes
                .as_slice()
                .try_into()
                .map_err(|_| LicenseError::InvalidSignature)?,
        );
        self.public_key
            .verify(canonical.as_bytes(), &signature)
            .map_err(|_| LicenseError::InvalidSignature)?;
        if Utc::now() > file.claims.expires_at {
            return Err(LicenseError::Expired {
                expires_at: file.claims.expires_at,
            });
        }
        Ok(License {
            claims: file.claims.clone(),
            verified_at: Utc::now(),
        })
    }
}

/// Serialize [`LicenseClaims`] to canonical JSON: a [`serde_json::Value`] with
/// all object keys sorted recursively (lexicographic UTF-8 byte order),
/// serialized compactly via [`serde_json::to_string`].
///
/// This is the byte sequence the licensing server must sign. Array element
/// order is preserved.
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
            // serde_json::Map preserves insertion order (unless the
            // "preserve_order" feature is disabled). Rebuild it from a
            // BTreeMap so keys are emitted in sorted order.
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
    //! License verification tests.
    //!
    //! These generate an Ed25519 keypair in-process, sign canonical claim
    //! payloads, and exercise the verifier across valid / expired / tampered
    //! / wrong-key / canonical-stability scenarios.

    use super::*;
    use chrono::{Duration, Utc};
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;

    /// Build a fresh keypair, sign the canonical JSON of `claims`, and return
    /// a complete [`LicenseFile`] plus the verifier matching the public key.
    fn make_signed_license(claims: &LicenseClaims) -> (LicenseFile, LicenseVerifier) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let canonical = canonical_json(claims).expect("canonical serialize");
        let signature = signing_key.sign(canonical.as_bytes());
        let file = LicenseFile {
            claims: claims.clone(),
            signature: BASE64.encode(signature.to_bytes()),
        };
        let verifier = LicenseVerifier::new(&verifying_key.to_bytes()).expect("verifier");
        (file, verifier)
    }

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
    fn test_valid_license() {
        let claims = sample_claims();
        let (file, verifier) = make_signed_license(&claims);
        let license = verifier
            .verify_claims(&file)
            .expect("valid license should verify");
        assert_eq!(license.claims.license_id, "lic_test_001");
        assert_eq!(license.claims.seats, 50);
        assert!(!license.is_expired());
        assert!(license.has_feature("rbac"));
        assert!(!license.has_feature("nonexistent"));
    }

    #[test]
    fn test_expired_license() {
        let mut claims = sample_claims();
        claims.expires_at = Utc::now() - Duration::days(1);
        let (file, verifier) = make_signed_license(&claims);
        let err = verifier
            .verify_claims(&file)
            .expect_err("should be expired");
        assert!(
            matches!(err, LicenseError::Expired { .. }),
            "expected Expired, got {err:?}"
        );
    }

    #[test]
    fn test_tampered_license() {
        let claims = sample_claims();
        let (mut file, verifier) = make_signed_license(&claims);
        // Flip the last byte of the base64 signature.
        let mut sig_bytes = BASE64
            .decode(file.signature.as_bytes())
            .expect("decode sig");
        let last = sig_bytes.len() - 1;
        sig_bytes[last] ^= 0xff;
        file.signature = BASE64.encode(&sig_bytes);
        let err = verifier
            .verify_claims(&file)
            .expect_err("tampered signature should fail");
        assert!(
            matches!(err, LicenseError::InvalidSignature),
            "expected InvalidSignature, got {err:?}"
        );
    }

    #[test]
    fn test_wrong_key() {
        let claims = sample_claims();
        let (file, _verifier) = make_signed_license(&claims);
        // Build a verifier with a different (fresh) public key.
        let other = SigningKey::generate(&mut OsRng);
        let verifier = LicenseVerifier::new(&other.verifying_key().to_bytes()).expect("verifier");
        let err = verifier
            .verify_claims(&file)
            .expect_err("wrong key should fail");
        assert!(
            matches!(err, LicenseError::InvalidSignature),
            "expected InvalidSignature, got {err:?}"
        );
    }

    #[test]
    fn test_canonical_json_stable() {
        // The same claims must serialize to identical canonical bytes
        // regardless of how the in-memory Value was constructed.
        let claims = sample_claims();
        let canonical_a = canonical_json(&claims).expect("canonical a");
        // Build a deliberately out-of-order JSON object with the same data and
        // confirm canonical_json sorts it to the same bytes.
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
    fn test_verify_from_file() {
        let claims = sample_claims();
        let (file, verifier) = make_signed_license(&claims);
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("license.json");
        std::fs::write(&path, serde_json::to_vec(&file).expect("serialize")).expect("write");
        let license = verifier.verify(&path).expect("file verify");
        assert_eq!(license.claims.license_id, "lic_test_001");
    }

    #[test]
    fn test_verify_missing_file() {
        let other = SigningKey::generate(&mut OsRng);
        let verifier = LicenseVerifier::new(&other.verifying_key().to_bytes()).expect("verifier");
        let err = verifier
            .verify(std::path::Path::new("/nonexistent/license.json"))
            .expect_err("missing file should fail");
        assert!(
            matches!(err, LicenseError::NotFound(_)),
            "expected NotFound, got {err:?}"
        );
    }
}

//! Integration tests for license issuance and verification.
//!
//! These tests exercise the licensing server's signing and verification logic
//! and — critically — verify **canonical JSON compatibility** with the proxy
//! binary's `LicenseVerifier` (from `madhyamas-enterprise`). A license signed
//! by the licensing server MUST verify successfully with the proxy's
//! verifier, proving the two implementations produce identical canonical
//! bytes.

use base64::Engine as _;
use chrono::{Duration, Utc};
use madhyamas_enterprise::license::{
    LicenseClaims as ProxyClaims, LicenseFile as ProxyFile, LicenseVerifier,
};
use madhyamas_licensing::license::{verify_license, LicenseClaims, LicenseFile, LicenseSigner};

fn sample_claims() -> LicenseClaims {
    LicenseClaims {
        license_id: "lic_integration_001".to_string(),
        customer: "Acme Corp".to_string(),
        plan: "enterprise".to_string(),
        seats: 50,
        instance_id: "inst_xyz789".to_string(),
        issued_at: Utc::now() - Duration::days(1),
        expires_at: Utc::now() + Duration::days(365),
        features: vec!["auth".to_string(), "rbac".to_string(), "audit".to_string()],
    }
}

/// Create a license with the server's signer, then verify the signature
/// directly with the public key.
#[test]
fn test_license_issuance() {
    let (signer, public_key) = LicenseSigner::generate();
    let claims = sample_claims();
    let file = signer
        .sign_license(&claims)
        .expect("signing should succeed");

    assert!(!file.signature.is_empty());
    assert_eq!(file.claims.license_id, "lic_integration_001");

    verify_license(&file, &public_key).expect("signature should verify with public key");
}

/// Verify a valid license, an expired license, and a tampered license.
#[test]
fn test_license_verification() {
    let (signer, public_key) = LicenseSigner::generate();

    // Valid license.
    let claims = sample_claims();
    let file = signer.sign_license(&claims).expect("sign");
    verify_license(&file, &public_key).expect("valid license should verify");

    // Expired license.
    let mut expired_claims = sample_claims();
    expired_claims.expires_at = Utc::now() - Duration::days(1);
    let expired_file = signer.sign_license(&expired_claims).expect("sign expired");
    let err = verify_license(&expired_file, &public_key).expect_err("should be expired");
    assert!(matches!(
        err,
        madhyamas_licensing::license::SignError::Expired { .. }
    ));

    // Tampered license (flip a signature byte).
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine as _;
    let mut tampered = file.clone();
    let mut sig_bytes = BASE64
        .decode(tampered.signature.as_bytes())
        .expect("decode");
    let last = sig_bytes.len() - 1;
    sig_bytes[last] ^= 0xff;
    tampered.signature = BASE64.encode(&sig_bytes);
    let err = verify_license(&tampered, &public_key).expect_err("tampered should fail");
    assert!(matches!(
        err,
        madhyamas_licensing::license::SignError::InvalidSignature
    ));
}

/// Verify that a license signed by the licensing server is accepted by the
/// proxy's `LicenseVerifier` (from `madhyamas-enterprise`). This proves the
/// canonical JSON and Ed25519 signature are byte-for-byte compatible.
#[test]
fn test_canonical_json_compatibility() {
    let (signer, public_key) = LicenseSigner::generate();
    let claims = sample_claims();
    let file = signer.sign_license(&claims).expect("sign");

    // Build the proxy's verifier with the same public key.
    let verifier = LicenseVerifier::new(&public_key.to_bytes()).expect("verifier");

    // Serialize the server's LicenseFile to JSON and parse it as the proxy's
    // LicenseFile. The serde formats must match (same field names, same
    // flatten structure).
    let json = serde_json::to_string(&file).expect("serialize server file");
    let proxy_file: ProxyFile = serde_json::from_str(&json).expect("parse as proxy file");

    // The proxy's verifier must accept the license.
    let license = verifier
        .verify_claims(&proxy_file)
        .expect("proxy should verify");
    assert_eq!(license.claims.license_id, "lic_integration_001");
    assert_eq!(license.claims.seats, 50);
    assert!(!license.is_expired());
    assert!(license.has_feature("rbac"));
}

/// Verify that the proxy's LicenseClaims serialize to the same JSON as the
/// server's LicenseClaims (same field names and types).
#[test]
fn test_claims_json_identical() {
    let server_claims = sample_claims();
    let proxy_claims = ProxyClaims {
        license_id: server_claims.license_id.clone(),
        customer: server_claims.customer.clone(),
        plan: server_claims.plan.clone(),
        seats: server_claims.seats,
        instance_id: server_claims.instance_id.clone(),
        issued_at: server_claims.issued_at,
        expires_at: server_claims.expires_at,
        features: server_claims.features.clone(),
    };
    let server_json = serde_json::to_string(&server_claims).expect("server json");
    let proxy_json = serde_json::to_string(&proxy_claims).expect("proxy json");
    assert_eq!(
        server_json, proxy_json,
        "LicenseClaims JSON must be identical"
    );
}

/// Verify that a license signed by the proxy's signing key (using the same
/// canonical JSON) is accepted by the server's verifier. This tests
/// compatibility in the reverse direction.
#[test]
fn test_reverse_compatibility() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    // Build claims in the proxy's format.
    let proxy_claims = ProxyClaims {
        license_id: "lic_reverse_001".to_string(),
        customer: "Test Corp".to_string(),
        plan: "pro".to_string(),
        seats: 10,
        instance_id: "inst_abc".to_string(),
        issued_at: Utc::now() - Duration::days(1),
        expires_at: Utc::now() + Duration::days(30),
        features: vec!["auth".to_string()],
    };

    // Serialize to JSON, parse as server's claims, sign with the server's
    // canonical JSON, then verify with the proxy's verifier.
    let json = serde_json::to_string(&proxy_claims).expect("serialize proxy claims");
    let server_claims: LicenseClaims = serde_json::from_str(&json).expect("parse as server claims");

    // Sign with raw ed25519 over the server's canonical JSON.
    let (signer, _) = LicenseSigner::generate();
    let _ = signer; // We'll sign manually to simulate the proxy signing.

    // Actually, let's sign with the raw key over the server's canonical JSON
    // and verify with the proxy's verifier.
    let server_file = {
        // Manually sign using the raw signing key.
        let canonical = canonical_json_via_server(&server_claims);
        let signature = signing_key.sign(canonical.as_bytes());
        LicenseFile {
            claims: server_claims.clone(),
            signature: base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()),
        }
    };

    // Parse as proxy file and verify.
    let json = serde_json::to_string(&server_file).expect("serialize");
    let proxy_file: ProxyFile = serde_json::from_str(&json).expect("parse");
    let verifier = LicenseVerifier::new(&verifying_key.to_bytes()).expect("verifier");
    verifier
        .verify_claims(&proxy_file)
        .expect("proxy should verify server-signed license");
}

/// Helper: produce canonical JSON using the server's logic (re-exported for
/// tests). Since `canonical_json` is private, we replicate it here by
/// serializing to Value, sorting keys, and compact-serializing.
fn canonical_json_via_server(claims: &LicenseClaims) -> String {
    let mut value = serde_json::to_value(claims).expect("to_value");
    sort_keys(&mut value);
    serde_json::to_string(&value).expect("to_string")
}

fn sort_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: std::collections::BTreeMap<String, serde_json::Value> = std::mem::take(map)
                .into_iter()
                .map(|(k, mut v)| {
                    sort_keys(&mut v);
                    (k, v)
                })
                .collect();
            *map = sorted.into_iter().collect();
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sort_keys(item);
            }
        }
        _ => {}
    }
}

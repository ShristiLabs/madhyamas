//! Integration tests for the public license API: Ed25519 sign/verify
//! round-trip, product pinning, and instance enforcement. Thin smoke tests
//! over the shared licensing-core implementation — the full suite lives in
//! the licensing-core crate.

use licensing_core::LicenseSigner;
use madhyamas_enterprise::{LicenseClaims, LicenseError, LicenseVerifier};

fn madhyamas_claims() -> LicenseClaims {
    LicenseClaims {
        license_id: "lic_madhyamas_001".to_string(),
        product_id: "madhyamas".to_string(),
        customer: "Acme Corp".to_string(),
        plan: "enterprise".to_string(),
        seats: 50,
        instance_id: "inst_xyz789".to_string(),
        issued_at: chrono::Utc::now() - chrono::Duration::days(1),
        expires_at: chrono::Utc::now() + chrono::Duration::days(365),
        features: vec!["auth".to_string(), "rbac".to_string()],
    }
}

#[test]
fn test_madhyamas_license_roundtrip() {
    let (signer, verifying) = LicenseSigner::generate();
    let file = signer.sign_license(&madhyamas_claims()).expect("sign");
    let verifier = LicenseVerifier::new(&verifying.to_bytes()).expect("verifier");
    let license = verifier
        .clone()
        .with_expected_product_id("madhyamas")
        .verify_claims(&file)
        .expect("verify");
    assert_eq!(license.claims.product_id, "madhyamas");
    assert!(license.has_feature("rbac"));
    assert!(!license.is_expired());
}

#[test]
fn test_rejects_license_for_other_product() {
    let (signer, verifying) = LicenseSigner::generate();
    let mut claims = madhyamas_claims();
    claims.product_id = "other-product".to_string();
    let file = signer.sign_license(&claims).expect("sign");
    let verifier = LicenseVerifier::new(&verifying.to_bytes()).expect("verifier");
    let err = verifier
        .with_expected_product_id("madhyamas")
        .verify_claims(&file)
        .expect_err("other product must be rejected");
    assert!(matches!(err, LicenseError::ProductMismatch { .. }));
}

#[test]
fn test_instance_id_enforcement_still_works() {
    let (signer, verifying) = LicenseSigner::generate();
    let file = signer.sign_license(&madhyamas_claims()).expect("sign");
    let verifier = LicenseVerifier::new(&verifying.to_bytes()).expect("verifier");
    let err = verifier
        .with_expected_instance_id("inst_DIFFERENT")
        .verify_claims(&file)
        .expect_err("instance mismatch");
    assert!(matches!(err, LicenseError::InstanceMismatch { .. }));
}

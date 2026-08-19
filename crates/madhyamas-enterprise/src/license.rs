//! Ed25519 license verification for the enterprise tier.
//!
//! The implementation now lives in the shared [`licensing_core`] crate —
//! the same crate the ShristiLabs licensing server signs with
//! (https://github.com/ShristiLabs/licensing) — so both sides verify
//! byte-identical claims by construction. This module re-exports it to
//! keep the `madhyamas_enterprise` public API stable.
//!
//! The proxy binary validates an offline license file at startup. A license
//! file is a JSON document containing the license claims plus a detached
//! Ed25519 signature (base64) over the **canonical JSON** of those claims.
//! The proxy holds only the Ed25519 **public** key for the `madhyamas`
//! product (`MADHYAMAS_LICENSE_PUBLIC_KEY`); the licensing server holds the
//! private signing key and never exposes it.
//!
//! # License file format (claims format v2)
//!
//! ```json
//! {
//!   "license_id": "lic_abc123",
//!   "product_id": "madhyamas",
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
//! The required `product_id` claim (v2) scopes the license to a ShristiLabs
//! product; the startup verifier pins it to `"madhyamas"` so licenses
//! issued for other products are rejected.
//!
//! # Verification flow
//!
//! 1. Read the license file from disk and parse it into [`LicenseFile`].
//! 2. Re-serialize the [`LicenseClaims`] to canonical JSON (object keys
//!    sorted recursively) — see [`licensing_core::canonical_json`].
//! 3. Verify the Ed25519 signature over those canonical bytes using the
//!    embedded public key.
//! 4. Check `expires_at` is in the future, and any pinned instance/product
//!    expectations hold.
//! 5. Return a [`License`] (claims + verification timestamp).

pub use licensing_core::{
    canonical_json, sort_json_keys, License, LicenseClaims, LicenseError, LicenseFile,
    LicenseVerifier, CLAIMS_FORMAT_VERSION,
};

#[cfg(test)]
mod tests {
    //! Thin smoke tests over the shared licensing-core implementation —
    //! the full suite lives in the licensing-core crate (including the
    //! golden-fixture test that freezes the claims format).

    use super::*;
    use licensing_core::LicenseSigner;

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
}

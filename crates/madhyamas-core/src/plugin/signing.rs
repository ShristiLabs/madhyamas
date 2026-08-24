//! Plugin signing utilities (Ed25519).
//!
//! Provides:
//! - [`sign_package`] — sign a plugin zip with a publisher's secret key.
//! - [`generate_keypair`] — generate a new Ed25519 keypair for a publisher.
//!
//! The signature file (`signature.sig`) is a detached Ed25519 signature over
//! the **raw zip bytes**. The publisher's public key (hex-encoded) is placed
//! in the manifest's `publisher_public_key` field.
//!
//! ## Signing a plugin package
//!
//! ```no_run
//! use madhyamas_core::{generate_keypair, sign_package, bytes_to_hex};
//!
//! // 1. Generate a keypair (do this once and store the secret key securely).
//! let kp = generate_keypair();
//! println!("public key (hex): {}", bytes_to_hex(&kp.public_key));
//! println!("secret key (hex): {}", bytes_to_hex(&kp.secret_key));
//!
//! // 2. Sign the zip bytes.
//! let zip_bytes = std::fs::read("my-plugin.zip")?;
//! let sig = sign_package(&zip_bytes, &kp.secret_key)?;
//! std::fs::write("signature.sig", sig)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Verifying at install time
//!
//! Verification happens automatically in [`super::installer::PluginInstaller`]
//! when the manifest declares a `publisher_public_key`.

use crate::Error;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;

/// An Ed25519 keypair for plugin signing.
#[derive(Debug)]
pub struct PluginKeypair {
    /// 32-byte secret key (hex-encode for storage).
    pub secret_key: [u8; 32],
    /// 32-byte public key (hex-encode for the manifest).
    pub public_key: [u8; 32],
}

/// Generate a new Ed25519 keypair using the OS RNG.
pub fn generate_keypair() -> PluginKeypair {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    PluginKeypair {
        secret_key: signing_key.to_bytes(),
        public_key: verifying_key.to_bytes(),
    }
}

/// Sign a plugin package (raw zip bytes) with a publisher's secret key.
///
/// Returns the 64-byte detached Ed25519 signature.
pub fn sign_package(zip_bytes: &[u8], secret_key: &[u8; 32]) -> crate::Result<[u8; 64]> {
    let signing_key = SigningKey::from_bytes(secret_key);
    let signature = signing_key.sign(zip_bytes);
    Ok(signature.to_bytes())
}

/// Verify a plugin package signature against a publisher's public key.
///
/// This is the same verification performed by the installer, exposed as a
/// standalone function for testing and CLI tooling.
pub fn verify_package(
    zip_bytes: &[u8],
    signature: &[u8; 64],
    public_key: &[u8; 32],
) -> crate::Result<()> {
    use ed25519_dalek::{Signature, Verifier};
    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|e| Error::Config(format!("invalid Ed25519 public key: {}", e)))?;
    let signature = Signature::from_bytes(signature);
    verifying_key
        .verify(zip_bytes, &signature)
        .map_err(|e| Error::Config(format!("signature verification failed: {}", e)))?;
    Ok(())
}

/// Decode a hex string into a fixed-size byte array.
pub fn hex_to_bytes<const N: usize>(hex: &str) -> crate::Result<[u8; N]> {
    let bytes = hex_decode(hex)?;
    if bytes.len() != N {
        return Err(Error::Config(format!(
            "expected {} bytes from hex, got {}",
            N,
            bytes.len()
        )));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Encode bytes as a lowercase hex string.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> crate::Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err(Error::Config("odd-length hex string".into()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let (chunks, _rem) = bytes.as_chunks::<2>();
    for chunk in chunks {
        let hi = hex_val(chunk[0])?;
        let lo = hex_val(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_val(c: u8) -> crate::Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(Error::Config(format!(
            "invalid hex character: {:?}",
            c as char
        ))),
    }
}

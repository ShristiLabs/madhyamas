//! Encrypted-at-rest file keystore for the OSS tier (#87).
//!
//! # Format
//!
//! Secrets live in `<data_dir>/secrets.enc.json`:
//!
//! ```json
//! { "version": 1, "entries": { "<name>": { "nonce": "<hex>", "ciphertext": "<hex>" } } }
//! ```
//!
//! Each value is sealed independently with AES-256-GCM (authenticated
//! encryption; any tampering fails decryption rather than yielding
//! plaintext).
//!
//! # Key management
//!
//! The 32-byte master key is resolved in this order:
//!
//! 1. `MADHYAMAS_SECRETS_KEY` — either 64 hex characters or exactly 32 raw
//!    bytes (e.g. from a Docker secret / Kubernetes secret env var).
//! 2. `MADHYAMAS_SECRETS_KEY_FILE` — path to a file containing 64 hex chars
//!    or 32 raw bytes (recommended: keep the key file on a different volume
//!    than the keystore, or inject it via a secret manager).
//! 3. Auto-generated random key at `<data_dir>/secrets.key` (0600). This is
//!    a convenience fallback for local development; rotating or losing that
//!    file makes existing secrets unrecoverable. Production deployments
//!    should always use option 1 or 2.
//!
//! The key is held only in memory after resolution and is never written to
//! the log.

use crate::Error;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Env var holding the master key (64 hex chars or 32 raw bytes).
pub const KEY_ENV_VAR: &str = "MADHYAMAS_SECRETS_KEY";
/// Env var holding a path to a key file.
pub const KEY_FILE_ENV_VAR: &str = "MADHYAMAS_SECRETS_KEY_FILE";

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Serialize, Deserialize)]
struct SealedEntry {
    nonce: String,
    ciphertext: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct KeystoreFile {
    version: u32,
    entries: HashMap<String, SealedEntry>,
}

/// Resolve the 32-byte master key per the documented precedence.
pub fn resolve_key(data_dir: &Path) -> crate::Result<Vec<u8>> {
    if let Ok(material) = std::env::var(KEY_ENV_VAR) {
        return parse_key_material(&material, KEY_ENV_VAR);
    }
    if let Ok(path) = std::env::var(KEY_FILE_ENV_VAR) {
        let material = std::fs::read_to_string(&path)
            .map_err(|e| Error::Config(format!("failed to read {}: {}", path, e)))?;
        return parse_key_material(material.trim(), &path);
    }
    // Fallback: auto-generated key file next to the keystore.
    let key_path = data_dir.join("secrets.key");
    if key_path.exists() {
        let material = std::fs::read_to_string(&key_path)
            .map_err(|e| Error::Config(format!("failed to read {}: {}", key_path.display(), e)))?;
        return parse_key_material(material.trim(), &key_path.display().to_string());
    }
    let mut key = vec![0u8; KEY_LEN];
    rand::rng().fill_bytes(&mut key);
    std::fs::create_dir_all(data_dir)?;
    let tmp = data_dir.join("secrets.key.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        f.write_all(hex_encode(&key).as_bytes())?;
    }
    std::fs::rename(&tmp, &key_path)?;
    tracing::warn!(
        "generated a new secrets keystore key at {} — use {} or {} in production; \
         losing this file makes stored secrets unrecoverable",
        key_path.display(),
        KEY_ENV_VAR,
        KEY_FILE_ENV_VAR
    );
    Ok(key)
}

/// Parse key material: 64 hex chars or exactly 32 raw bytes.
fn parse_key_material(material: &str, source: &str) -> crate::Result<Vec<u8>> {
    if material.len() == KEY_LEN * 2 && material.chars().all(|c| c.is_ascii_hexdigit()) {
        return hex_decode(material)
            .ok_or_else(|| Error::Config(format!("invalid hex in key source {}", source)));
    }
    let bytes = material.as_bytes();
    if bytes.len() == KEY_LEN {
        return Ok(bytes.to_vec());
    }
    Err(Error::Config(format!(
        "secrets key from {} must be 64 hex characters or exactly 32 raw bytes (got {} bytes)",
        source,
        material.len()
    )))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

/// Encrypt `plaintext` with `key`; returns `(nonce, ciphertext)` hex pair.
pub fn seal(key: &[u8], plaintext: &str) -> crate::Result<(String, String)> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad: b"madhyamas-secret",
            },
        )
        .map_err(|_| Error::Config("secret encryption failed".into()))?;
    Ok((hex_encode(&nonce_bytes), hex_encode(&ct)))
}

/// Decrypt a `(nonce, ciphertext)` hex pair produced by [`seal`].
pub fn unseal(key: &[u8], nonce: &str, ciphertext: &str) -> crate::Result<String> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes =
        hex_decode(nonce).ok_or_else(|| Error::Config("invalid nonce encoding".into()))?;
    let ct = hex_decode(ciphertext)
        .ok_or_else(|| Error::Config("invalid ciphertext encoding".into()))?;
    if nonce_bytes.len() != NONCE_LEN {
        return Err(Error::Config("invalid nonce length".into()));
    }
    let pt = cipher
        .decrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: &ct,
                aad: b"madhyamas-secret",
            },
        )
        .map_err(|_| {
            Error::Config("secret decryption failed (wrong key or tampered store)".into())
        })?;
    String::from_utf8(pt).map_err(|_| Error::Config("secret is not valid UTF-8".into()))
}

/// File-backed, encrypted-at-rest keystore (OSS tier).
///
/// All operations are synchronous file I/O guarded by an in-process mutex;
/// the keystore is small (tens of entries), so this is not a hot path.
pub struct FileKeystore {
    path: PathBuf,
    key: Vec<u8>,
    lock: std::sync::Mutex<()>,
}

impl FileKeystore {
    /// Open (or create) the keystore at `<data_dir>/secrets.enc.json`,
    /// resolving the master key via [`resolve_key`].
    pub fn open(data_dir: &Path) -> crate::Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let key = resolve_key(data_dir)?;
        Ok(Self {
            path: data_dir.join("secrets.enc.json"),
            key,
            lock: std::sync::Mutex::new(()),
        })
    }

    /// Keystore with an explicit key (used by tests and by the enterprise
    /// tier, which reuses the sealing primitives against its own storage).
    pub fn with_key(path: PathBuf, key: Vec<u8>) -> Self {
        Self {
            path,
            key,
            lock: std::sync::Mutex::new(()),
        }
    }

    fn load(&self) -> crate::Result<KeystoreFile> {
        if !self.path.exists() {
            return Ok(KeystoreFile {
                version: 1,
                entries: HashMap::new(),
            });
        }
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| Error::Config(format!("failed to read keystore: {}", e)))?;
        let file: KeystoreFile = serde_json::from_str(&text)
            .map_err(|e| Error::Config(format!("corrupt keystore: {}", e)))?;
        if file.version != 1 {
            return Err(Error::Config(format!(
                "unsupported keystore version {}",
                file.version
            )));
        }
        Ok(file)
    }

    fn save(&self, file: &KeystoreFile) -> crate::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(file)
            .map_err(|e| Error::Config(format!("keystore serialize: {}", e)))?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Set (or overwrite) a secret. The value is sealed before it touches
    /// disk.
    pub fn set(&self, name: &str, value: &str) -> crate::Result<()> {
        let _g = self.lock.lock();
        let mut file = self.load()?;
        let (nonce, ct) = seal(&self.key, value)?;
        file.entries.insert(
            name.to_string(),
            SealedEntry {
                nonce,
                ciphertext: ct,
            },
        );
        self.save(&file)
    }

    /// Delete a secret. Returns whether the name existed.
    pub fn delete(&self, name: &str) -> crate::Result<bool> {
        let _g = self.lock.lock();
        let mut file = self.load()?;
        let removed = file.entries.remove(name).is_some();
        if removed {
            self.save(&file)?;
        }
        Ok(removed)
    }

    /// Get and decrypt a single secret (internal use: substitution only).
    pub fn get(&self, name: &str) -> crate::Result<Option<String>> {
        let _g = self.lock.lock();
        let file = self.load()?;
        match file.entries.get(name) {
            None => Ok(None),
            Some(e) => Ok(Some(unseal(&self.key, &e.nonce, &e.ciphertext)?)),
        }
    }

    /// List secret names (sorted). Values are never included.
    pub fn names(&self) -> crate::Result<Vec<String>> {
        let _g = self.lock.lock();
        let file = self.load()?;
        let mut names: Vec<String> = file.entries.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    /// Decrypt all entries (startup load into the in-memory service cache).
    pub fn load_all(&self) -> crate::Result<HashMap<String, String>> {
        let _g = self.lock.lock();
        let file = self.load()?;
        let mut out = HashMap::new();
        for (name, e) in &file.entries {
            out.insert(name.clone(), unseal(&self.key, &e.nonce, &e.ciphertext)?);
        }
        Ok(out)
    }
}

impl super::service::SecretStore for FileKeystore {
    fn load_all(&self) -> crate::Result<HashMap<String, String>> {
        FileKeystore::load_all(self)
    }

    fn set(&self, name: &str, value: &str) -> crate::Result<()> {
        FileKeystore::set(self, name, value)
    }

    fn delete(&self, name: &str) -> crate::Result<bool> {
        FileKeystore::delete(self, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> Vec<u8> {
        // Deterministic 32-byte key for tests.
        (0u8..32).collect()
    }

    #[test]
    fn seal_unseal_round_trip() {
        let key = test_key();
        let (n, c) = seal(&key, "hunter2").unwrap();
        assert_eq!(unseal(&key, &n, &c).unwrap(), "hunter2");
    }

    #[test]
    fn seal_is_non_deterministic() {
        let key = test_key();
        let (n1, c1) = seal(&key, "same").unwrap();
        let (n2, c2) = seal(&key, "same").unwrap();
        assert_ne!(c1, c2, "ciphertexts must not repeat");
        assert_ne!(n1, n2, "nonces must not repeat");
    }

    #[test]
    fn unseal_wrong_key_fails() {
        let (n, c) = seal(&test_key(), "value").unwrap();
        let other: Vec<u8> = (100u8..132).collect();
        assert!(unseal(&other, &n, &c).is_err());
    }

    #[test]
    fn unseal_tampered_ciphertext_fails() {
        let key = test_key();
        let (n, c) = seal(&key, "value").unwrap();
        // Flip a hex digit in the ciphertext.
        let first = if c.starts_with('0') { '1' } else { '0' };
        let tampered = format!("{}{}", first, &c[1..]);
        assert!(unseal(&key, &n, &tampered).is_err());
    }

    #[test]
    fn parse_key_material_variants() {
        let hex64 = "00".repeat(32);
        assert_eq!(parse_key_material(&hex64, "t").unwrap().len(), 32);
        let raw32 = "a".repeat(32);
        assert_eq!(parse_key_material(&raw32, "t").unwrap().len(), 32);
        assert!(parse_key_material("short", "t").is_err());
        let bad_hex = "zz".repeat(32);
        assert!(parse_key_material(&bad_hex, "t").is_err());
    }
}

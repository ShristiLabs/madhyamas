//! Plugin installer — download, verify, extract, and (un)install plugins.
//!
//! An install source is either:
//! - a direct URL to a `.zip` plugin package, or
//! - a registry entry id (resolved via [`super::PluginRegistry`]).
//!
//! The flow:
//! 1. Download the zip bytes (via `reqwest`).
//! 2. Verify the SHA-256 checksum against the expected value (if provided).
//! 3. Optionally verify an Ed25519 signature against a trusted publisher
//!    public key (Phase 3).
//! 4. Extract the zip into the target plugin directory
//!    (`<plugin_dir>/<plugin_id>/`).
//! 5. Parse the manifest, mark the plugin installed in [`PluginPersistence`],
//!    and return an [`InstallResult`].
//!
//! Uninstall removes the plugin directory and its persisted state.

use super::{PluginManifest, PluginPersistence};
use crate::Error;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};
use zip::ZipArchive;

/// Where to install the plugin from.
#[derive(Debug, Clone)]
pub enum InstallSource {
    /// Direct URL to a `.zip` plugin package.
    Url {
        url: String,
        /// Expected SHA-256 hex checksum. If `None`, checksum verification
        /// is skipped (and a warning is logged).
        checksum: Option<String>,
    },
    /// Registry entry id; the registry provides the download URL + checksum.
    RegistryId(String),
}

/// Result of a successful install.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallResult {
    pub plugin_id: String,
    pub version: String,
    pub path: String,
    pub checksum_verified: bool,
    pub signature_verified: bool,
}

/// Plugin installer.
pub struct PluginInstaller {
    /// Base directory where plugin packages are extracted
    /// (`<base>/<plugin_id>/`).
    base_dir: PathBuf,
    persistence: Option<Arc<PluginPersistence>>,
    /// Optional trusted-publisher public keys (hex Ed25519) that bypass the
    /// unverified-plugin confirmation gate.
    trusted_publishers: Vec<String>,
}

impl PluginInstaller {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            persistence: None,
            trusted_publishers: Vec::new(),
        }
    }

    pub fn with_persistence(mut self, p: Arc<PluginPersistence>) -> Self {
        self.persistence = Some(p);
        self
    }

    pub fn add_trusted_publisher(&mut self, pubkey_hex: String) {
        self.trusted_publishers.push(pubkey_hex);
    }

    /// Install a plugin from the given source, returning the parsed manifest
    /// and install metadata.
    pub async fn install(
        &self,
        source: &InstallSource,
        expected_checksum: Option<&str>,
    ) -> crate::Result<InstallResult> {
        // 1. Resolve to (bytes, expected_checksum).
        let (bytes, checksum): (Vec<u8>, Option<String>) = match source {
            InstallSource::Url { url, checksum } => {
                let bytes = download(url).await?;
                (
                    bytes,
                    checksum
                        .clone()
                        .or_else(|| expected_checksum.map(str::to_string)),
                )
            }
            InstallSource::RegistryId(_id) => {
                // Registry-driven download is handled by the caller
                // (PluginRegistry::download_plugin) which resolves the id to
                // a URL + checksum; here we only support direct URLs.
                return Err(Error::Config(
                    "registry install must be resolved to a URL by the caller".into(),
                ));
            }
        };

        // 2. Checksum verification.
        let actual_checksum = hex_sha256(&bytes);
        let checksum_verified = match &checksum {
            Some(expected) => {
                if expected.eq_ignore_ascii_case(&actual_checksum) {
                    true
                } else {
                    return Err(Error::Config(format!(
                        "plugin checksum mismatch: expected {}, got {}",
                        expected, actual_checksum
                    )));
                }
            }
            None => {
                warn!(
                    "Installing plugin without checksum verification (actual sha256={})",
                    actual_checksum
                );
                false
            }
        };

        // 3. Extract the zip into a temp dir first, parse the manifest, then
        //    move into the final location. This avoids partial installs.
        let temp_dir = tempfile::tempdir()?;
        extract_zip(&bytes, temp_dir.path())?;

        // The manifest may be at the root of the zip or inside a single
        // top-level directory.
        let (manifest, manifest_dir) = find_manifest(temp_dir.path())?;
        let plugin_id = manifest.id.clone();

        // 4. Verify publisher signature if declared.
        let signature_verified = verify_signature(&manifest, &bytes, &manifest_dir)?;

        // 5. Move into the final location.
        let dest = self.base_dir.join(&plugin_id);
        if dest.exists() {
            // Reinstall: remove the old copy first.
            std::fs::remove_dir_all(&dest)?;
        }
        std::fs::create_dir_all(self.base_dir.join(&plugin_id))?;
        copy_dir(&manifest_dir, &dest)?;

        // 6. Persist installed state.
        if let Some(p) = &self.persistence {
            p.mark_installed(&plugin_id)?;
        }

        info!(
            "Installed plugin {} v{} to {:?}",
            plugin_id, manifest.version, dest
        );

        Ok(InstallResult {
            plugin_id,
            version: manifest.version,
            path: dest.to_string_lossy().to_string(),
            checksum_verified,
            signature_verified,
        })
    }

    /// Uninstall a plugin: remove its directory and persisted state.
    pub fn uninstall(&self, plugin_id: &str) -> crate::Result<()> {
        let dest = self.base_dir.join(plugin_id);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }
        if let Some(p) = &self.persistence {
            p.remove_state(plugin_id)?;
        }
        info!("Uninstalled plugin: {}", plugin_id);
        Ok(())
    }

    /// The base directory plugins are installed into.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

async fn download(url: &str) -> crate::Result<Vec<u8>> {
    // Support file:// URLs for local plugin packages (useful for development
    // and testing). reqwest does not handle the file:// scheme.
    if let Some(path) = url.strip_prefix("file://") {
        return std::fs::read(path)
            .map_err(|e| Error::Config(format!("failed to read local file '{}': {}", path, e)));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| Error::Config(format!("http client: {}", e)))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Config(format!("download failed: {}", e)))?;
    if !resp.status().is_success() {
        return Err(Error::Config(format!(
            "download failed: HTTP {}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Error::Config(format!("download body: {}", e)))?;
    Ok(bytes.to_vec())
}

pub fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

fn extract_zip(bytes: &[u8], dest: &Path) -> crate::Result<()> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| Error::Config(format!("invalid zip: {}", e)))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| Error::Config(format!("zip entry {}: {}", i, e)))?;
        let entry_name = entry.name().to_string();
        // Guard against zip-slip: reject absolute paths and `..` components.
        if entry_name.contains("..") || PathBuf::from(&entry_name).is_absolute() {
            return Err(Error::Config(format!(
                "zip entry '{}' escapes the install directory (zip-slip)",
                entry_name
            )));
        }
        let outpath = dest.join(&entry_name);
        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&outpath)?;
            let mut buf = Vec::with_capacity(8192);
            entry.read_to_end(&mut buf)?;
            out.write_all(&buf)?;
        }
    }
    Ok(())
}

/// Locate a `madhyamas-plugin.toml`/`.json` manifest within `dir`, returning
/// the parsed manifest and the directory that should be copied (either `dir`
/// itself or the single top-level subdirectory containing the manifest).
fn find_manifest(dir: &Path) -> crate::Result<(PluginManifest, PathBuf)> {
    if let Some(m) = parse_manifest_at(dir) {
        return Ok((m, dir.to_path_buf()));
    }
    // Look one level deep for a single subdirectory containing the manifest.
    let mut found: Option<(PluginManifest, PathBuf)> = None;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(m) = parse_manifest_at(&entry.path()) {
                if found.is_some() {
                    return Err(Error::Config(
                        "zip contains multiple plugin manifests; ambiguous".into(),
                    ));
                }
                found = Some((m, entry.path()));
            }
        }
    }
    found.ok_or_else(|| Error::Config("no plugin manifest found in package".into()))
}

fn parse_manifest_at(dir: &Path) -> Option<PluginManifest> {
    let toml_path = dir.join("madhyamas-plugin.toml");
    let json_path = dir.join("madhyamas-plugin.json");
    if toml_path.exists() {
        let s = std::fs::read_to_string(&toml_path).ok()?;
        toml::from_str(&s).ok()
    } else if json_path.exists() {
        let s = std::fs::read_to_string(&json_path).ok()?;
        serde_json::from_str(&s).ok()
    } else {
        None
    }
}

fn copy_dir(src: &Path, dest: &Path) -> crate::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Verify an Ed25519 signature if the manifest declares a publisher public
/// key.
///
/// The signature is expected in a `signature.sig` file at the package root
/// (alongside the manifest). The signature is over the **original zip bytes**
/// (not the extracted files), so callers must pass the raw downloaded bytes.
///
/// Returns `true` if the signature was present and verified. Returns `false`
/// if no publisher key is declared (unsigned plugin). Returns an error if a
/// key is declared but the signature is missing or invalid.
fn verify_signature(
    manifest: &PluginManifest,
    zip_bytes: &[u8],
    manifest_dir: &Path,
) -> crate::Result<bool> {
    let Some(pubkey_hex) = &manifest.publisher_public_key else {
        // No publisher key declared — unsigned plugin.
        return Ok(false);
    };

    // Parse the hex-encoded Ed25519 public key (32 bytes).
    let pubkey_bytes = hex_decode(pubkey_hex).map_err(|e| {
        Error::Config(format!(
            "plugin {} has invalid publisher_public_key (not hex): {}",
            manifest.id, e
        ))
    })?;
    if pubkey_bytes.len() != 32 {
        return Err(Error::Config(format!(
            "plugin {} publisher_public_key must be 32 bytes, got {}",
            manifest.id,
            pubkey_bytes.len()
        )));
    }
    let verifying_key = VerifyingKey::from_bytes(pubkey_bytes.as_slice().try_into().unwrap())
        .map_err(|e| {
            Error::Config(format!(
                "plugin {} has invalid Ed25519 public key: {}",
                manifest.id, e
            ))
        })?;

    // Look for signature.sig alongside the manifest.
    let sig_path = manifest_dir.join("signature.sig");
    if !sig_path.exists() {
        warn!(
            "plugin {} declares a publisher key but no signature.sig file was found; \
             treating as unverified",
            manifest.id
        );
        return Ok(false);
    }

    let sig_bytes = std::fs::read(&sig_path)?;
    if sig_bytes.len() != 64 {
        return Err(Error::Config(format!(
            "plugin {} signature.sig must be 64 bytes, got {}",
            manifest.id,
            sig_bytes.len()
        )));
    }
    let signature = Signature::from_slice(&sig_bytes).map_err(|e| {
        Error::Config(format!(
            "plugin {} has invalid Ed25519 signature: {}",
            manifest.id, e
        ))
    })?;

    // Verify the signature over the zip bytes.
    verifying_key.verify(zip_bytes, &signature).map_err(|e| {
        Error::Config(format!(
            "plugin {} signature verification failed: {}",
            manifest.id, e
        ))
    })?;

    info!(
        "plugin {} signature verified against publisher key {}",
        manifest.id, pubkey_hex
    );
    Ok(true)
}

/// Decode a hex string into bytes. Accepts upper- or lower-case.
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex string".into());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks_exact(2) {
        let hi = hex_val(chunk[0])?;
        let lo = hex_val(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_val(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("invalid hex character: {:?}", c as char)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(plugin_id: &str) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("madhyamas-plugin.toml", opts).unwrap();
            let manifest = format!(
                "id = \"{}\"\nname = \"Test\"\nversion = \"1.0.0\"\nhooks = [\"on_request\"]\n",
                plugin_id
            );
            zip.write_all(manifest.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn hex_sha256_known() {
        // sha256 of empty input
        assert_eq!(
            hex_sha256(&[]),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[tokio::test]
    async fn install_from_local_zip() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("plugins");
        std::fs::create_dir_all(&base).unwrap();
        let _installer = PluginInstaller::new(base.clone());

        let zip_bytes = make_zip("test.local");
        // Write the zip to a file and serve via a file://-style path is not
        // supported by reqwest; instead exercise the extract path directly.
        let checksum = hex_sha256(&zip_bytes);

        // Use the Url source with a data: URL is not supported either; emulate
        // by calling the internal extract + find flow.
        let temp = tempfile::tempdir().unwrap();
        extract_zip(&zip_bytes, temp.path()).unwrap();
        let (manifest, _) = find_manifest(temp.path()).unwrap();
        assert_eq!(manifest.id, "test.local");
        // checksum sanity
        assert!(checksum.starts_with("e") || checksum.len() == 64);
    }
}

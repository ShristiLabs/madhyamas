//! Certificate generation and management

use super::GeneratedCert;
use crate::Error;
use parking_lot::RwLock;
use rcgen::{
    CertificateParams, DistinguishedName, DnType, Issuer, KeyPair, PKCS_ECDSA_P256_SHA256,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

/// Manages CA certificate and generates leaf certificates for HTTPS interception
#[allow(dead_code)]
pub struct CertificateManager {
    /// CA certificate in PEM format
    ca_cert_pem: Vec<u8>,
    /// CA private key in PEM format
    ca_key_pem: Vec<u8>,
    /// CA key pair for signing
    ca_key_pair: KeyPair,
    /// CA certificate params for creating issuer
    ca_params: CertificateParams,
    /// Cached leaf certificates by hostname (with creation timestamp)
    cache: RwLock<HashMap<String, CachedCert>>,
    /// Path to store certificates
    cert_path: String,
    /// Maximum cache entries (LRU eviction)
    max_cache_size: usize,
    /// Cache TTL in seconds
    cache_ttl_secs: u64,
}

/// A cached certificate with its creation timestamp
#[derive(Clone)]
struct CachedCert {
    cert: GeneratedCert,
    created_at: std::time::Instant,
}

impl CertificateManager {
    /// Organization name for certificates
    const ORG_NAME: &'static str = "Madhyamas";

    /// Create a new certificate manager, generating or loading the CA
    pub async fn new(cert_path: &str) -> crate::Result<Arc<Self>> {
        // Ensure cert directory exists
        fs::create_dir_all(cert_path)
            .await
            .map_err(|e| Error::Certificate(format!("Failed to create cert directory: {}", e)))?;

        let ca_cert_path = Path::new(cert_path).join("madhyamas-ca.pem");
        let ca_key_path = Path::new(cert_path).join("madhyamas-ca-key.pem");

        let (ca_cert_pem, ca_key_pem, ca_key_pair, ca_params) =
            if ca_cert_path.exists() && ca_key_path.exists() {
                // Load existing CA
                info!("Loading existing CA certificate");
                Self::load_ca(&ca_cert_path, &ca_key_path).await?
            } else {
                // Generate new CA
                info!("Generating new CA certificate");
                let (cert_pem, key_pem, key_pair, params) = Self::generate_ca()?;

                // Save CA to disk
                fs::write(&ca_cert_path, &cert_pem)
                    .await
                    .map_err(|e| Error::Certificate(format!("Failed to write CA cert: {}", e)))?;
                fs::write(&ca_key_path, &key_pem)
                    .await
                    .map_err(|e| Error::Certificate(format!("Failed to write CA key: {}", e)))?;

                // Set CA key permissions to 0600 (owner read/write only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o600);
                    tokio::fs::set_permissions(&ca_key_path, perms)
                        .await
                        .map_err(|e| {
                            Error::Certificate(format!("Failed to set CA key permissions: {}", e))
                        })?;
                }

                (cert_pem, key_pem, key_pair, params)
            };

        Ok(Arc::new(Self {
            ca_cert_pem,
            ca_key_pem,
            ca_key_pair,
            ca_params,
            cache: RwLock::new(HashMap::new()),
            cert_path: cert_path.to_string(),
            max_cache_size: 10_000,
            cache_ttl_secs: 24 * 60 * 60, // 24 hours
        }))
    }

    /// Load existing CA certificate
    async fn load_ca(
        cert_path: &Path,
        key_path: &Path,
    ) -> crate::Result<(Vec<u8>, Vec<u8>, KeyPair, CertificateParams)> {
        let cert_pem = fs::read(cert_path)
            .await
            .map_err(|e| Error::Certificate(format!("Failed to read CA cert: {}", e)))?;
        let key_pem = fs::read(key_path)
            .await
            .map_err(|e| Error::Certificate(format!("Failed to read CA key: {}", e)))?;

        let key_pair = KeyPair::from_pem(&String::from_utf8_lossy(&key_pem))
            .map_err(|e| Error::Certificate(format!("Failed to parse CA key: {}", e)))?;

        // Regenerate CA certificate params
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::OrganizationName, Self::ORG_NAME);
        dn.push(DnType::CommonName, "Madhyamas Root CA");
        params.distinguished_name = dn;
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
        ];

        Ok((cert_pem, key_pem, key_pair, params))
    }

    /// Generate a new CA certificate
    fn generate_ca() -> crate::Result<(Vec<u8>, Vec<u8>, KeyPair, CertificateParams)> {
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::OrganizationName, Self::ORG_NAME);
        dn.push(DnType::CommonName, "Madhyamas Root CA");
        params.distinguished_name = dn;

        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
        ];

        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| Error::Certificate(format!("Failed to generate key pair: {}", e)))?;

        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| Error::Certificate(format!("Failed to create CA cert: {}", e)))?;

        Ok((
            cert.pem().into_bytes(),
            key_pair.serialize_pem().into_bytes(),
            key_pair,
            params,
        ))
    }

    /// Get the CA certificate in PEM format
    pub fn ca_certificate_pem(&self) -> &[u8] {
        &self.ca_cert_pem
    }

    /// Generate a leaf certificate for a specific hostname.
    ///
    /// Uses an LRU-style cache with a configurable max size (default 10K entries)
    /// and TTL (default 24h). Expired entries are evicted on access.
    pub fn generate_cert_for_host(&self, hostname: &str) -> crate::Result<GeneratedCert> {
        // Check cache first (with TTL check)
        {
            let cache = self.cache.read();
            if let Some(entry) = cache.get(hostname) {
                let age = entry.created_at.elapsed().as_secs();
                if age < self.cache_ttl_secs {
                    return Ok(entry.cert.clone());
                }
            }
        }

        info!("Generating certificate for: {}", hostname);

        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::OrganizationName, Self::ORG_NAME);
        dn.push(DnType::CommonName, hostname);
        params.distinguished_name = dn;

        params.is_ca = rcgen::IsCa::NoCa;
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::DigitalSignature,
            rcgen::KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![
            rcgen::ExtendedKeyUsagePurpose::ServerAuth,
            rcgen::ExtendedKeyUsagePurpose::ClientAuth,
        ];

        // Add subject alternative names
        params.subject_alt_names =
            vec![rcgen::SanType::DnsName(hostname.try_into().map_err(
                |e| Error::Certificate(format!("Invalid hostname: {:?}", e)),
            )?)];

        // Generate key pair for this certificate
        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| Error::Certificate(format!("Failed to generate key pair: {}", e)))?;

        // Sign with CA
        let issuer = Issuer::from_params(&self.ca_params, &self.ca_key_pair);
        let cert = params
            .signed_by(&key_pair, &issuer)
            .map_err(|e| Error::Certificate(format!("Failed to sign cert: {}", e)))?;

        let generated = GeneratedCert {
            certificate: cert.pem().into_bytes(),
            private_key: key_pair.serialize_pem().into_bytes(),
        };

        // Cache the certificate with eviction
        {
            let mut cache = self.cache.write();

            // Evict expired entries
            let ttl = self.cache_ttl_secs;
            cache.retain(|_, v| v.created_at.elapsed().as_secs() < ttl);

            // If still over max size, evict oldest entries
            if cache.len() >= self.max_cache_size {
                let to_remove: Vec<String> = cache
                    .iter()
                    .min_by_key(|(_, v)| v.created_at)
                    .map(|(k, _)| k.clone())
                    .into_iter()
                    .collect();
                for key in to_remove {
                    cache.remove(&key);
                }
            }

            cache.insert(
                hostname.to_string(),
                CachedCert {
                    cert: generated.clone(),
                    created_at: std::time::Instant::now(),
                },
            );
        }

        Ok(generated)
    }

    /// Clear the certificate cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        info!("Certificate cache cleared");
    }

    /// Get the path where certificates are stored
    pub fn cert_path(&self) -> &str {
        &self.cert_path
    }
}

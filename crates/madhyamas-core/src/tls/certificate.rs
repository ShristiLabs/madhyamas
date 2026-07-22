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
    /// Cached leaf certificates by hostname
    cache: RwLock<HashMap<String, GeneratedCert>>,
    /// Path to store certificates
    cert_path: String,
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

    /// Generate a leaf certificate for a specific hostname
    pub fn generate_cert_for_host(&self, hostname: &str) -> crate::Result<GeneratedCert> {
        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(cert) = cache.get(hostname) {
                return Ok(cert.clone());
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

        // Cache the certificate
        {
            let mut cache = self.cache.write();
            cache.insert(hostname.to_string(), generated.clone());
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

//! TLS certificate management for HTTPS interception

mod certificate;

pub use certificate::CertificateManager;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// Generated certificate pair
#[derive(Clone)]
pub struct GeneratedCert {
    pub certificate: Vec<u8>,
    pub private_key: Vec<u8>,
}

impl GeneratedCert {
    /// Get certificate as DER
    pub fn cert_der(&self) -> CertificateDer<'_> {
        CertificateDer::from(self.certificate.clone())
    }

    /// Get private key as DER
    pub fn key_der(&self) -> PrivateKeyDer<'_> {
        PrivateKeyDer::try_from(self.private_key.clone())
            .unwrap_or_else(|_| PrivateKeyDer::Pkcs1(self.private_key.clone().into()))
    }
}

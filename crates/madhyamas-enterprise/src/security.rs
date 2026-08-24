//! Security helpers for enterprise features (Phase 9.7).
//!
//! This module provides [`validate_callback_url`], a helper that guards
//! against Server-Side Request Forgery (SSRF) in OIDC / SSO callback URLs.
//! It is intended for use when OIDC / SSO integration is added in a future
//! phase — the function is ready now so the validation contract is fixed
//! and tested.
//!
//! # Rules
//!
//! 1. The URL **must** use the `https` scheme (plain `http` is rejected).
//! 2. The host **must not** resolve to a private / loopback / link-local /
//!    unspecified IP address. The following ranges are blocked:
//!    - `10.0.0.0/8` (private)
//!    - `172.16.0.0/12` (private)
//!    - `192.168.0.0/16` (private)
//!    - `127.0.0.0/8` (loopback)
//!    - `169.254.0.0/16` (link-local)
//!    - `0.0.0.0/8` (unspecified)
//!    - `::1/128` (IPv6 loopback)
//!    - `fc00::/7` (IPv6 unique-local)
//!    - `fe80::/10` (IPv6 link-local)
//! 3. The host must not be a bare IP address that falls in any of the
//!    above ranges. Domain names are allowed (the caller is expected to
//!    resolve them at use time and re-check the resolved IP).

use std::net::IpAddr;

use super::enterprise_error::EnterpriseError;

/// Validate an OIDC / SSO callback URL against SSRF rules (Phase 9.7).
///
/// Returns `Ok(())` when the URL is safe (HTTPS scheme, non-private host),
/// or an [`EnterpriseError::InvalidConfig`] with a descriptive message when
/// the URL is rejected.
///
/// **Note:** This function performs a syntactic check only — it does not
/// resolve DNS. When OIDC is implemented, the caller should resolve the
/// hostname and call [`is_private_ip`] on the resolved address before
/// making an outbound request.
pub fn validate_callback_url(url_str: &str) -> Result<(), EnterpriseError> {
    let parsed = url::Url::parse(url_str).map_err(|e| EnterpriseError::InvalidConfig {
        message: format!("invalid callback URL: {e}"),
    })?;

    if parsed.scheme() != "https" {
        return Err(EnterpriseError::InvalidConfig {
            message: "callback URL must use HTTPS scheme".to_string(),
        });
    }

    match parsed.host() {
        Some(url::Host::Ipv4(ip)) => {
            if is_private_ip(&IpAddr::V4(ip)) {
                return Err(EnterpriseError::InvalidConfig {
                    message: format!("callback URL host {ip} is a private/loopback address"),
                });
            }
        }
        Some(url::Host::Ipv6(ip)) => {
            if is_private_ip(&IpAddr::V6(ip)) {
                return Err(EnterpriseError::InvalidConfig {
                    message: format!("callback URL host {ip} is a private/loopback address"),
                });
            }
        }
        Some(url::Host::Domain(_)) => {
            // Domain names are allowed at the syntactic level. The caller
            // must resolve and re-check the IP at connection time.
        }
        None => {
            return Err(EnterpriseError::InvalidConfig {
                message: "callback URL has no host".to_string(),
            });
        }
    }

    Ok(())
}

/// Returns `true` if the IP address is private, loopback, link-local, or
/// unspecified — i.e. should never be the target of an outbound SSRF
/// request.
pub fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_documentation()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unicast_link_local()
                || is_ipv6_unique_local(ip)
        }
    }
}

/// Check if an IPv6 address is in the `fc00::/7` unique-local range.
/// (`std::net::Ipv6Addr` does not expose this directly.)
fn is_ipv6_unique_local(ip: &std::net::Ipv6Addr) -> bool {
    let octets = ip.octets();
    // fc00::/7 means the first 7 bits are 1111110, i.e. the first byte
    // is 0xfc or 0xfd.
    octets[0] & 0xfe == 0xfc
}

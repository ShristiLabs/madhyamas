//! IP-based access control (allowlist) for the proxy and API servers.
//!
//! [`AccessControlList`] parses a list of IP/CIDR entries (e.g.
//! `192.168.1.5`, `10.0.0.0/8`, `fd00::/16`) and provides fast membership
//! tests via [`AccessControlList::is_allowed`].
//!
//! # Semantics
//!
//! - **Empty list → allow all.** This is the default and preserves backward
//!   compatibility: a proxy started without `--allowed-ip` accepts every
//!   connection.
//! - **Non-empty list → allow only listed IPs/CIDRs.** Any connection from
//!   an address outside the configured ranges is rejected.
//! - **Localhost is always allowed.** `127.0.0.1` (IPv4 loopback) and `::1`
//!   (IPv6 loopback) are permitted regardless of the allowlist contents.
//!   This prevents administrators from accidentally locking themselves out
//!   of a locally-running proxy.
//!
//! # CIDR support
//!
//! Both IPv4 (`/0`–`/32`) and IPv6 (`/0`–`/128`) CIDR notation are supported.
//! A bare IP address (e.g. `192.168.1.5`) is treated as a `/32` (IPv4) or
//! `/128` (IPv6) host route.
//!
//! This module deliberately avoids pulling in the `ipnet` crate — the
//! containment math is trivial and self-contained (mirroring the approach
//! already used in [`crate::config`] for the upstream-proxy bypass list).

use std::net::IpAddr;

/// An IP/CIDR entry in an [`AccessControlList`].
///
/// Each entry is either a parsed CIDR range or a bare IP address (which is
/// normalized to a `/32` or `/128` host route at parse time).
#[derive(Debug, Clone)]
enum AclEntry {
    V4 { network: u32, mask: u32 },
    V6 { network: [u8; 16], mask: [u8; 16] },
}

impl AclEntry {
    /// Returns `true` if `addr` falls inside this entry's network range.
    fn contains(&self, addr: &IpAddr) -> bool {
        match (self, addr) {
            (AclEntry::V4 { network, mask }, IpAddr::V4(v4)) => {
                (u32::from_be_bytes(v4.octets()) & mask) == *network
            }
            (AclEntry::V6 { network, mask }, IpAddr::V6(v6)) => {
                let octets = v6.octets();
                for i in 0..16 {
                    if (octets[i] & mask[i]) != network[i] {
                        return false;
                    }
                }
                true
            }
            _ => false, // address-family mismatch
        }
    }
}

/// An IP access control list (allowlist).
///
/// Construct with [`AccessControlList::new`] from a slice of string rules.
/// An empty slice produces an "allow all" list.
///
/// # Examples
///
/// ```
/// use madhyamas_core::AccessControlList;
/// use std::net::IpAddr;
///
/// // Empty list: everything is allowed (default behavior).
/// let acl = AccessControlList::new(&[]).unwrap();
/// assert!(acl.is_allowed("10.1.2.3".parse::<IpAddr>().unwrap()));
///
/// // Restrict to a /24 subnet.
/// let acl = AccessControlList::new(&["192.168.1.0/24".to_string()]).unwrap();
/// assert!(acl.is_allowed("192.168.1.50".parse::<IpAddr>().unwrap()));
/// assert!(!acl.is_allowed("192.168.2.50".parse::<IpAddr>().unwrap()));
///
/// // Localhost is always allowed, even when not in the list.
/// assert!(acl.is_allowed("127.0.0.1".parse::<IpAddr>().unwrap()));
/// ```
pub struct AccessControlList {
    entries: Vec<AclEntry>,
    /// When `true`, every address is allowed (empty rule list).
    allow_all: bool,
}

impl AccessControlList {
    /// Build an allowlist from a list of string rules.
    ///
    /// Each rule may be:
    /// - A bare IP address: `"192.168.1.5"`, `"::1"`
    /// - A CIDR range: `"192.168.0.0/16"`, `"fd00::/8"`
    ///
    /// Whitespace is trimmed and entries are case-insensitive. An empty
    /// slice (or a slice of only blank entries) produces an "allow all"
    /// list.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Config`] if any entry cannot be parsed as
    /// an IP address or CIDR range.
    pub fn new(rules: &[String]) -> crate::Result<Self> {
        let mut entries = Vec::with_capacity(rules.len());

        for raw in rules {
            let rule = raw.trim();
            if rule.is_empty() {
                continue;
            }
            entries.push(parse_entry(rule)?);
        }

        Ok(Self {
            allow_all: entries.is_empty(),
            entries,
        })
    }

    /// Create an "allow all" list (no restrictions).
    pub fn allow_all() -> Self {
        Self {
            entries: Vec::new(),
            allow_all: true,
        }
    }

    /// Returns `true` if the list is empty (no restrictions, allow all).
    pub fn is_allow_all(&self) -> bool {
        self.allow_all
    }

    /// Number of configured entries (excluding the implicit localhost
    /// allowance). Returns `0` for an allow-all list.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when there are no configured entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check whether `addr` is permitted by this access control list.
    ///
    /// - An allow-all list (empty rules) always returns `true`.
    /// - Loopback addresses (`127.0.0.1`, `::1`) always return `true`
    ///   regardless of the configured entries, so a locally-started proxy
    ///   can never be locked out.
    /// - Otherwise the address must match at least one configured CIDR
    ///   range or bare IP entry.
    pub fn is_allowed(&self, addr: IpAddr) -> bool {
        if self.allow_all {
            return true;
        }
        // Localhost is always allowed — prevents accidental lockout.
        if is_loopback(&addr) {
            return true;
        }
        self.entries.iter().any(|entry| entry.contains(&addr))
    }
}

impl std::fmt::Debug for AccessControlList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessControlList")
            .field("entry_count", &self.entries.len())
            .field("allow_all", &self.allow_all)
            .finish()
    }
}

/// Returns `true` for IPv4/IPv6 loopback addresses.
fn is_loopback(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

/// Parse a single rule string into an [`AclEntry`].
///
/// Accepts `ip`, `ip/prefix`, or `ip/cidr`. A bare IP is normalized to a
/// full-host mask (`/32` for IPv4, `/128` for IPv6).
fn parse_entry(rule: &str) -> crate::Result<AclEntry> {
    let rule = rule.trim();
    if rule.is_empty() {
        return Err(crate::Error::Config(
            "Empty access control entry".to_string(),
        ));
    }

    // Split into IP and optional prefix length.
    let (ip_part, prefix_opt) = match rule.split_once('/') {
        Some((ip, prefix)) => (ip, Some(prefix)),
        None => (rule, None),
    };

    let ip: IpAddr = ip_part
        .trim()
        .parse()
        .map_err(|e| crate::Error::Config(format!("Invalid IP address `{ip_part}`: {e}")))?;

    let prefix = match prefix_opt {
        Some(p) => p
            .trim()
            .parse::<u8>()
            .map_err(|e| crate::Error::Config(format!("Invalid CIDR prefix `{p}`: {e}")))?,
        None => match ip {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        },
    };

    build_entry(ip, prefix)
}

/// Construct an [`AclEntry`] from an IP address and prefix length.
fn build_entry(ip: IpAddr, prefix: u8) -> crate::Result<AclEntry> {
    match ip {
        IpAddr::V4(v4) => {
            if prefix > 32 {
                return Err(crate::Error::Config(format!(
                    "IPv4 CIDR prefix too large: {prefix}"
                )));
            }
            let mask: u32 = if prefix == 0 {
                0
            } else {
                (!0u32) << (32 - prefix)
            };
            let network = u32::from_be_bytes(v4.octets()) & mask;
            Ok(AclEntry::V4 { network, mask })
        }
        IpAddr::V6(v6) => {
            if prefix > 128 {
                return Err(crate::Error::Config(format!(
                    "IPv6 CIDR prefix too large: {prefix}"
                )));
            }
            let octets = v6.octets();
            let mut network = [0u8; 16];
            let mut mask = [0u8; 16];
            for i in 0..128 {
                if i < prefix as usize {
                    mask[i / 8] |= 0x80 >> (i % 8);
                }
            }
            for i in 0..16 {
                network[i] = octets[i] & mask[i];
            }
            Ok(AclEntry::V6 { network, mask })
        }
    }
}

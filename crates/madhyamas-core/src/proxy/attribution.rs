//! Attribution context for proxied connections (issue #103).
//!
//! Every accepted connection carries a small [`AttributionContext`] from
//! the accept loop through connection handling into the traffic-entry
//! construction points (the intercept pipeline, the CONNECT/TLS-failure
//! and passthrough entries, and the SOCKS5 tunnel entry). Today
//! [`AttributionContext::client_addr`] and [`AttributionContext::device_id`]
//! are stamped onto captured entries (issues #103 and #105); the `device_id`
//! slot is populated by the engine for device-
//! authenticated connections (issue #104) and is what per-device traffic
//! filters and device-scoped intercept rules will consume in later
//! credential-onboarding issues. `device_name` (issue #105) rides along as
//! display metadata for naming the device's auto-created capture session.
//!
//! Tier placement: the struct lives in core and is inert in the OSS
//! tier — the proxy listener performs no authentication there, so the
//! context only ever carries the client address and listener kind. The
//! enterprise tier additionally resolves a [`super::ProxyPrincipal`] from
//! proxy credentials at CONNECT (see `ProxyAuthValidator`) and copies a
//! resolved device identity into [`AttributionContext::device_id`].

use std::net::SocketAddr;

/// Which listener accepted the connection that produced a traffic entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerKind {
    /// The plain-HTTP / CONNECT proxy listener.
    Http,
    /// The SOCKS5 blind-tunnel listener.
    Socks,
}

/// Attribution metadata for a proxied connection (issue #103).
///
/// Built once per accepted connection and threaded through connection
/// handling so every [`crate::traffic::TrafficEntry`] constructed for the
/// connection can be attributed to its origin.
#[derive(Debug, Clone)]
pub struct AttributionContext {
    /// Device principal for the connection. Populated by the engine after
    /// proxy-auth validation when the credential resolves to a device
    /// (issue #104); `None` for unauthenticated and user-authenticated
    /// connections. Later issues in the credential-onboarding milestone
    /// (per-device traffic filters, device-scoped intercept rules) consume
    /// it without re-threading this context.
    pub device_id: Option<String>,
    /// Display name of the device record (issue #105). Purely metadata for
    /// naming the device's auto-created capture session ("Device: Hari's
    /// Pixel"); `None` whenever `device_id` is `None`. Not an identity —
    /// filters key on `device_id`, never on the name.
    pub device_name: Option<String>,
    /// Address of the directly-connected client. This is the address the
    /// proxy accepted, which may differ from the device identity when
    /// traffic arrives through NAT or a local VPN.
    pub client_addr: Option<SocketAddr>,
    /// Listener that accepted the connection.
    pub listener: ListenerKind,
}

impl AttributionContext {
    /// Create a context for a connection accepted on `listener` from
    /// `client_addr`.
    pub fn new(listener: ListenerKind, client_addr: Option<SocketAddr>) -> Self {
        Self {
            device_id: None,
            device_name: None,
            client_addr,
            listener,
        }
    }

    /// The client address formatted as `ip:port`, ready to persist on a
    /// traffic entry. `None` when the address is unknown.
    pub fn client_addr_string(&self) -> Option<String> {
        self.client_addr.map(|a| a.to_string())
    }
}

impl Default for AttributionContext {
    /// The default context describes an unknown-origin connection on the
    /// primary (HTTP) listener: no device, no client address. Used by the
    /// [`super::pipeline::Pipeline`] when no attribution is available
    /// (e.g. entries constructed outside a proxied connection).
    fn default() -> Self {
        Self {
            device_id: None,
            device_name: None,
            client_addr: None,
            listener: ListenerKind::Http,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_addr_string_formats_ipv4_and_ipv6() {
        let v4: SocketAddr = "192.168.1.24:51424".parse().unwrap();
        let v6: SocketAddr = "[2001:db8::1]:443".parse().unwrap();

        let ctx = AttributionContext::new(ListenerKind::Http, Some(v4));
        assert_eq!(
            ctx.client_addr_string().as_deref(),
            Some("192.168.1.24:51424")
        );

        let ctx = AttributionContext::new(ListenerKind::Socks, Some(v6));
        assert_eq!(
            ctx.client_addr_string().as_deref(),
            Some("[2001:db8::1]:443")
        );
    }

    #[test]
    fn client_addr_string_none_when_address_unknown() {
        let ctx = AttributionContext::new(ListenerKind::Socks, None);
        assert_eq!(ctx.client_addr_string(), None);
    }

    #[test]
    fn default_context_is_unknown_origin_on_http_listener() {
        let ctx = AttributionContext::default();
        assert_eq!(ctx.listener, ListenerKind::Http);
        assert!(ctx.client_addr.is_none());
        assert!(ctx.device_id.is_none());
    }

    #[test]
    fn new_sets_listener_and_address_and_keeps_device_slot_empty() {
        let addr: SocketAddr = "10.0.0.5:8888".parse().unwrap();
        let ctx = AttributionContext::new(ListenerKind::Socks, Some(addr));
        assert_eq!(ctx.listener, ListenerKind::Socks);
        assert_eq!(ctx.client_addr, Some(addr));
        // The constructor never infers a device: the engine fills the
        // slot from the resolved proxy principal (issue #104).
        assert!(ctx.device_id.is_none());
        assert!(ctx.device_name.is_none());
    }
}

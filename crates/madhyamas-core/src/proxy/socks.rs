//! SOCKS5 proxy support (RFC 1928 / RFC 1929).
//!
//! This module implements a minimal SOCKS5 server: it accepts the SOCKS5
//! greeting, negotiates authentication (none or username/password), parses a
//! `CONNECT` request, dials the requested target, and then relays raw bytes
//! bidirectionally between the client and the target.
//!
//! SOCKS5 is a *blind* TCP tunnel — the proxy never interprets the application
//! protocol spoken through it. As a result:
//!
//! - HTTPS traffic cannot be MITM-intercepted via the SOCKS port (the client's
//!   TLS session is forwarded end-to-end to the target). Use the HTTP proxy
//!   port with `CONNECT` for HTTPS interception.
//! - HTTP traffic is also tunneled blindly. A traffic entry is still recorded
//!   (flagged `is_passthrough`) so the connection is visible in the web UI.
//!
//! The protocol parsing is split into pure, allocation-free functions that
//! operate on byte slices so they can be unit-tested without any I/O. The
//! async [`handle_socks5_connection`] function drives the handshake over a
//! real [`tokio::net::TcpStream`] and then hands off to the shared relay loop.
//!
//! ## Protocol summary
//!
//! ```text
//! Client                              Server
//!  │  VER=5 NMETHODS METHODS           │
//!  │ ────────────────────────────────► │  (greeting)
//!  │                                  │
//!  │ ◄──────────────────────────────── │  VER=5 METHOD
//!  │                                  │
//!  │  (if METHOD=2) VER=1 ULEN UNAME  │
//!  │  PLEN PASSWD                     │
//!  │ ────────────────────────────────► │  (auth)
//!  │ ◄──────────────────────────────── │  VER=1 STATUS
//!  │                                  │
//!  │  VER=5 CMD RSV ATYP DST.ADDR PORT │
//!  │ ────────────────────────────────► │  (request)
//!  │ ◄──────────────────────────────── │  VER=5 REP RSV ATYP BND.ADDR PORT
//!  │                                  │
//!  │  ◄─────── raw TCP relay ───────► │
//! ```

use crate::config::ProxyConfig;
use crate::traffic::{RequestData, TrafficEntry, TrafficStore};
use crate::Error;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// SOCKS protocol version.
pub const SOCKS_VERSION: u8 = 0x05;

/// SOCKS5 authentication method identifiers (RFC 1928 §3).
pub const METHOD_NO_AUTH: u8 = 0x00;
pub const METHOD_USER_PASS: u8 = 0x02;
pub const METHOD_NO_ACCEPTABLE: u8 = 0xFF;

/// SOCKS5 command codes (RFC 1928 §4).
pub const CMD_CONNECT: u8 = 0x01;
pub const CMD_BIND: u8 = 0x02;
pub const CMD_UDP_ASSOCIATE: u8 = 0x03;

/// SOCKS5 address types (RFC 1928 §5).
pub const ATYP_IPV4: u8 = 0x01;
pub const ATYP_DOMAIN: u8 = 0x03;
pub const ATYP_IPV6: u8 = 0x04;

/// Username/password auth sub-protocol version (RFC 1929).
pub const USER_PASS_VERSION: u8 = 0x01;

/// SOCKS5 reply codes (RFC 1928 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SocksReply {
    Succeeded = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TtlExpired = 0x06,
    CommandNotSupported = 0x07,
    AddressTypeNotSupported = 0x08,
}

impl SocksReply {
    /// Human-readable description of a reply code.
    pub fn as_str(self) -> &'static str {
        match self {
            SocksReply::Succeeded => "succeeded",
            SocksReply::GeneralFailure => "general SOCKS server failure",
            SocksReply::ConnectionNotAllowed => "connection not allowed by ruleset",
            SocksReply::NetworkUnreachable => "network unreachable",
            SocksReply::HostUnreachable => "host unreachable",
            SocksReply::ConnectionRefused => "connection refused",
            SocksReply::TtlExpired => "TTL expired",
            SocksReply::CommandNotSupported => "command not supported",
            SocksReply::AddressTypeNotSupported => "address type not supported",
        }
    }
}

/// A parsed SOCKS5 client greeting (method negotiation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Greeting {
    /// Auth methods offered by the client.
    pub methods: Vec<u8>,
}

/// A parsed SOCKS5 request (after auth).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocksRequest {
    /// Command byte (CONNECT / BIND / UDP ASSOCIATE).
    pub command: u8,
    /// Resolved target host: either a parsed IP address or a domain name.
    pub host: SocksHost,
    /// Target port (big-endian on the wire).
    pub port: u16,
}

/// The host portion of a SOCKS5 request, preserving the original address type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocksHost {
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    /// A domain name as sent by the client (e.g. with `--socks5-hostname`).
    Domain(String),
}

impl SocksHost {
    /// Render the host as a string suitable for logging and `TcpStream::connect`.
    pub fn as_str(&self) -> String {
        match self {
            SocksHost::Ipv4(ip) => ip.to_string(),
            SocksHost::Ipv6(ip) => ip.to_string(),
            SocksHost::Domain(d) => d.clone(),
        }
    }
}

/// A parsed username/password authentication exchange (RFC 1929).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
}

// ─── Pure parsing functions (no I/O, unit-testable) ──────────────────────

/// Parse a SOCKS5 client greeting.
///
/// Wire format: `VER(1) | NMETHODS(1) | METHODS(NMETHODS)`.
/// Returns the offered methods, or an error if the bytes are not a valid
/// SOCKS5 greeting. The caller is responsible for ensuring `buf` contains
/// the *complete* greeting (the methods list is variable-length).
pub fn parse_greeting(buf: &[u8]) -> Result<Greeting, Error> {
    if buf.len() < 2 {
        return Err(Error::Proxy(format!(
            "SOCKS5 greeting too short: {} bytes",
            buf.len()
        )));
    }
    if buf[0] != SOCKS_VERSION {
        return Err(Error::Proxy(format!(
            "Unsupported SOCKS version: 0x{:02x} (expected 0x05)",
            buf[0]
        )));
    }
    let nmethods = buf[1] as usize;
    if buf.len() < 2 + nmethods {
        return Err(Error::Proxy(format!(
            "SOCKS5 greeting truncated: declared {} methods but only {} bytes follow",
            nmethods,
            buf.len() - 2
        )));
    }
    let methods = buf[2..2 + nmethods].to_vec();
    Ok(Greeting { methods })
}

/// Select the authentication method to advertise back to the client.
///
/// If the server is configured with credentials, it prefers username/password
/// (`0x02`) when the client offers it; otherwise it falls back to no-auth
/// (`0x00`). If neither is acceptable, it returns `METHOD_NO_ACCEPTABLE`.
pub fn select_method(greeting: &Greeting, require_auth: bool) -> u8 {
    let offers_no_auth = greeting.methods.contains(&METHOD_NO_AUTH);
    let offers_user_pass = greeting.methods.contains(&METHOD_USER_PASS);

    if require_auth {
        if offers_user_pass {
            METHOD_USER_PASS
        } else {
            METHOD_NO_ACCEPTABLE
        }
    } else if offers_no_auth {
        METHOD_NO_AUTH
    } else if offers_user_pass {
        // Server doesn't require auth but client only offered user/pass —
        // accept it anyway so the client can proceed.
        METHOD_USER_PASS
    } else {
        METHOD_NO_ACCEPTABLE
    }
}

/// Parse a SOCKS5 username/password authentication message (RFC 1929).
///
/// Wire format: `VER(1, =0x01) | ULEN(1) | UNAME(ULEN) | PLEN(1) | PASSWD(PLEN)`.
pub fn parse_auth_credentials(buf: &[u8]) -> Result<AuthCredentials, Error> {
    if buf.len() < 2 {
        return Err(Error::Proxy("SOCKS5 auth message too short".into()));
    }
    if buf[0] != USER_PASS_VERSION {
        return Err(Error::Proxy(format!(
            "Unsupported SOCKS5 auth version: 0x{:02x} (expected 0x01)",
            buf[0]
        )));
    }
    let ulen = buf[1] as usize;
    if buf.len() < 2 + ulen + 1 {
        return Err(Error::Proxy(
            "SOCKS5 auth message truncated (username)".into(),
        ));
    }
    let username = String::from_utf8(buf[2..2 + ulen].to_vec())
        .map_err(|e| Error::Proxy(format!("Invalid SOCKS5 username (non-UTF-8): {}", e)))?;
    let plen_off = 2 + ulen;
    let plen = buf[plen_off] as usize;
    if buf.len() < plen_off + 1 + plen {
        return Err(Error::Proxy(
            "SOCKS5 auth message truncated (password)".into(),
        ));
    }
    let password = String::from_utf8(buf[plen_off + 1..plen_off + 1 + plen].to_vec())
        .map_err(|e| Error::Proxy(format!("Invalid SOCKS5 password (non-UTF-8): {}", e)))?;
    Ok(AuthCredentials { username, password })
}

/// Parse a SOCKS5 request.
///
/// Wire format:
/// `VER(1) | CMD(1) | RSV(1) | ATYP(1) | DST.ADDR(variable) | DST.PORT(2)`.
pub fn parse_request(buf: &[u8]) -> Result<SocksRequest, Error> {
    if buf.len() < 4 {
        return Err(Error::Proxy(format!(
            "SOCKS5 request too short: {} bytes",
            buf.len()
        )));
    }
    if buf[0] != SOCKS_VERSION {
        return Err(Error::Proxy(format!(
            "Unsupported SOCKS version in request: 0x{:02x}",
            buf[0]
        )));
    }
    let command = buf[1];
    // buf[2] is reserved (RSV) — ignore.
    let atyp = buf[3];

    let (host, port_off) = match atyp {
        ATYP_IPV4 => {
            if buf.len() < 4 + 4 + 2 {
                return Err(Error::Proxy("SOCKS5 IPv4 request truncated".into()));
            }
            let octets = [buf[4], buf[5], buf[6], buf[7]];
            let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
            (SocksHost::Ipv4(ip), 4 + 4)
        }
        ATYP_IPV6 => {
            if buf.len() < 4 + 16 + 2 {
                return Err(Error::Proxy("SOCKS5 IPv6 request truncated".into()));
            }
            let mut seg = [0u8; 16];
            seg.copy_from_slice(&buf[4..4 + 16]);
            let ip = Ipv6Addr::from(seg);
            (SocksHost::Ipv6(ip), 4 + 16)
        }
        ATYP_DOMAIN => {
            if buf.len() < 4 + 1 {
                return Err(Error::Proxy(
                    "SOCKS5 domain request truncated (length)".into(),
                ));
            }
            let dlen = buf[4] as usize;
            if buf.len() < 4 + 1 + dlen + 2 {
                return Err(Error::Proxy(
                    "SOCKS5 domain request truncated (name)".into(),
                ));
            }
            let domain = String::from_utf8(buf[5..5 + dlen].to_vec())
                .map_err(|e| Error::Proxy(format!("Invalid SOCKS5 domain (non-UTF-8): {}", e)))?;
            (SocksHost::Domain(domain), 4 + 1 + dlen)
        }
        other => {
            return Err(Error::Proxy(format!(
                "Unsupported SOCKS5 address type: 0x{:02x}",
                other
            )))
        }
    };

    if buf.len() < port_off + 2 {
        return Err(Error::Proxy("SOCKS5 request truncated (port)".into()));
    }
    let port = u16::from_be_bytes([buf[port_off], buf[port_off + 1]]);
    Ok(SocksRequest {
        command,
        host,
        port,
    })
}

/// Build the server's method-selection reply: `VER(1) | METHOD(1)`.
pub fn build_method_reply(method: u8) -> [u8; 2] {
    [SOCKS_VERSION, method]
}

/// Build the username/password auth status reply: `VER(1, =0x01) | STATUS(1)`.
/// `status == 0` means success.
pub fn build_auth_status_reply(success: bool) -> [u8; 2] {
    [USER_PASS_VERSION, if success { 0x00 } else { 0x01 }]
}

/// Build a SOCKS5 command reply for a successful CONNECT.
///
/// `bind_addr` is the local address the server bound to for the upstream
/// connection. We echo it back per RFC 1928 §6; many clients ignore it.
pub fn build_connect_reply(reply: SocksReply, bind_addr: Option<SocketAddr>) -> Vec<u8> {
    let mut out = Vec::with_capacity(22);
    out.push(SOCKS_VERSION);
    out.push(reply as u8);
    out.push(0x00); // reserved

    match bind_addr {
        Some(SocketAddr::V4(addr)) => {
            out.push(ATYP_IPV4);
            out.extend_from_slice(&addr.ip().octets());
            out.extend_from_slice(&addr.port().to_be_bytes());
        }
        Some(SocketAddr::V6(addr)) => {
            out.push(ATYP_IPV6);
            out.extend_from_slice(&addr.ip().octets());
            out.extend_from_slice(&addr.port().to_be_bytes());
        }
        // No bind address available — return a zeroed IPv4 BND.ADDR/PORT.
        // Clients generally ignore BND.ADDR for CONNECT.
        None => {
            out.push(ATYP_IPV4);
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&[0, 0]);
        }
    }
    out
}

// ─── Async connection handling ───────────────────────────────────────────

/// Shared state needed by the SOCKS handler. Holds owned, cheaply-clonable
/// handles (`Arc`s and a `broadcast::Sender`) so it can be moved into spawned
/// tasks without lifetime concerns.
pub struct SocksContext {
    pub config: Arc<ProxyConfig>,
    pub traffic_store: Arc<TrafficStore>,
    pub traffic_tx: broadcast::Sender<TrafficEntry>,
}

/// Bind and run the SOCKS5 listener. Returns when the listener is closed
/// (i.e. the engine is being dropped). Each accepted connection is handled
/// in its own task.
pub async fn serve_socks5(ctx: SocksContext) -> crate::Result<()> {
    let addr: SocketAddr = ctx
        .config
        .socks_addr()
        .parse()
        .map_err(|e| Error::Proxy(format!("Invalid SOCKS bind address: {}", e)))?;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| Error::Proxy(format!("Failed to bind SOCKS port: {}", e)))?;
    info!("SOCKS5 proxy listening on {}", addr);

    let require_auth = ctx.config.socks_auth_enabled();
    let auth_user = ctx.config.socks_auth_username.clone();
    let auth_pass = ctx.config.socks_auth_password.clone();

    loop {
        let (client_socket, client_addr) = listener
            .accept()
            .await
            .map_err(|e| Error::Proxy(format!("Failed to accept SOCKS connection: {}", e)))?;

        let traffic_store = ctx.traffic_store.clone();
        let traffic_tx = ctx.traffic_tx.clone();
        let auth_user = auth_user.clone();
        let auth_pass = auth_pass.clone();

        tokio::spawn(async move {
            debug!("SOCKS5 connection from {}", client_addr);
            if let Err(e) = handle_socks5_connection(
                client_socket,
                &traffic_store,
                &traffic_tx,
                require_auth,
                auth_user.as_deref(),
                auth_pass.as_deref(),
            )
            .await
            {
                debug!("SOCKS5 connection from {} ended: {}", client_addr, e);
            }
        });
    }
}

/// Drive a single SOCKS5 connection to completion: handshake, dial target,
/// record a traffic entry, and relay bytes.
///
/// This is the top-level per-connection handler. It is `pub` so it can be
/// invoked directly by tests with an already-connected stream.
pub async fn handle_socks5_connection(
    mut client_socket: TcpStream,
    traffic_store: &crate::traffic::TrafficStore,
    traffic_tx: &tokio::sync::broadcast::Sender<TrafficEntry>,
    require_auth: bool,
    auth_username: Option<&str>,
    auth_password: Option<&str>,
) -> crate::Result<()> {
    // ── 1. Method negotiation (greeting) ───────────────────────────────
    // The greeting is short (≤ 257 bytes); read up to that much. We read
    // exactly the greeting first, then the request, because the request may
    // follow immediately in the same TCP segment.
    let mut greeting_buf = [0u8; 2];
    client_socket
        .read_exact(&mut greeting_buf)
        .await
        .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 greeting: {}", e)))?;

    let nmethods = greeting_buf[1] as usize;
    let mut methods = vec![0u8; nmethods];
    client_socket
        .read_exact(&mut methods)
        .await
        .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 methods: {}", e)))?;

    let mut full_greeting = Vec::with_capacity(2 + nmethods);
    full_greeting.extend_from_slice(&greeting_buf);
    full_greeting.extend_from_slice(&methods);
    let greeting = parse_greeting(&full_greeting)?;
    let method = select_method(&greeting, require_auth);

    client_socket
        .write_all(&build_method_reply(method))
        .await
        .map_err(|e| Error::Proxy(format!("Failed to write SOCKS5 method reply: {}", e)))?;

    if method == METHOD_NO_ACCEPTABLE {
        warn!(
            "SOCKS5: no acceptable auth method (client offered {:?}, require_auth={})",
            greeting.methods, require_auth
        );
        return Err(Error::Proxy(
            "SOCKS5: no acceptable authentication method".into(),
        ));
    }

    // ── 2. Username/password authentication (RFC 1929) ─────────────────
    if method == METHOD_USER_PASS {
        // First two bytes: version + username length.
        let mut auth_hdr = [0u8; 2];
        client_socket
            .read_exact(&mut auth_hdr)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 auth header: {}", e)))?;
        let ulen = auth_hdr[1] as usize;
        let mut username_buf = vec![0u8; ulen];
        client_socket
            .read_exact(&mut username_buf)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 username: {}", e)))?;
        let mut plen_buf = [0u8; 1];
        client_socket
            .read_exact(&mut plen_buf)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 password length: {}", e)))?;
        let plen = plen_buf[0] as usize;
        let mut password_buf = vec![0u8; plen];
        client_socket
            .read_exact(&mut password_buf)
            .await
            .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 password: {}", e)))?;

        let mut full_auth = Vec::with_capacity(2 + ulen + 1 + plen);
        full_auth.extend_from_slice(&auth_hdr);
        full_auth.extend_from_slice(&username_buf);
        full_auth.push(plen_buf[0]);
        full_auth.extend_from_slice(&password_buf);
        let creds = parse_auth_credentials(&full_auth)?;

        let ok = match (auth_username, auth_password) {
            (Some(u), Some(p)) => creds.username == u && creds.password == p,
            _ => false,
        };
        client_socket
            .write_all(&build_auth_status_reply(ok))
            .await
            .map_err(|e| Error::Proxy(format!("Failed to write SOCKS5 auth status: {}", e)))?;
        if !ok {
            warn!(
                "SOCKS5: authentication failed for user {:?}",
                creds.username
            );
            return Err(Error::Proxy("SOCKS5: authentication failed".into()));
        }
        debug!("SOCKS5: authenticated user {:?}", creds.username);
    }

    // ── 3. Request ────────────────────────────────────────────────────
    // Fixed 4-byte header: VER CMD RSV ATYP.
    let mut req_hdr = [0u8; 4];
    client_socket
        .read_exact(&mut req_hdr)
        .await
        .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 request header: {}", e)))?;

    let atyp = req_hdr[3];
    // Read the address portion based on ATYP, then the 2-byte port.
    let addr_bytes: Vec<u8> = match atyp {
        ATYP_IPV4 => {
            let mut b = vec![0u8; 4];
            client_socket.read_exact(&mut b).await?;
            b
        }
        ATYP_IPV6 => {
            let mut b = vec![0u8; 16];
            client_socket.read_exact(&mut b).await?;
            b
        }
        ATYP_DOMAIN => {
            let mut len = [0u8; 1];
            client_socket.read_exact(&mut len).await?;
            let mut b = vec![0u8; len[0] as usize];
            // Re-read with the length prefix included so parse_request sees
            // the full domain block (length byte + name).
            let mut full = Vec::with_capacity(1 + b.len());
            full.push(len[0]);
            client_socket.read_exact(&mut b).await?;
            full.extend_from_slice(&b);
            full
        }
        other => {
            // Tell the client we don't support this address type.
            let _ = client_socket
                .write_all(&build_connect_reply(
                    SocksReply::AddressTypeNotSupported,
                    None,
                ))
                .await;
            return Err(Error::Proxy(format!(
                "Unsupported SOCKS5 address type: 0x{:02x}",
                other
            )));
        }
    };
    let mut port_buf = [0u8; 2];
    client_socket
        .read_exact(&mut port_buf)
        .await
        .map_err(|e| Error::Proxy(format!("Failed to read SOCKS5 port: {}", e)))?;

    let mut full_request = Vec::with_capacity(4 + addr_bytes.len() + 2);
    full_request.extend_from_slice(&req_hdr);
    full_request.extend_from_slice(&addr_bytes);
    full_request.extend_from_slice(&port_buf);
    let request = parse_request(&full_request)?;

    // Only CONNECT is supported. BIND and UDP ASSOCIATE are not.
    if request.command != CMD_CONNECT {
        let _ = client_socket
            .write_all(&build_connect_reply(SocksReply::CommandNotSupported, None))
            .await;
        return Err(Error::Proxy(format!(
            "Unsupported SOCKS5 command: 0x{:02x}",
            request.command
        )));
    }

    let host_str = request.host.as_str();
    let port = request.port;
    info!("SOCKS5 CONNECT: {}:{}", host_str, port);

    // ── 4. Record a traffic entry ──────────────────────────────────────
    let session_id = traffic_store.current_session_id();
    let scheme = if port == 443 { "https" } else { "tcp" };
    let mut entry = TrafficEntry::new(
        &session_id,
        RequestData {
            method: crate::traffic::HttpMethod::Connect,
            url: format!("{}://{}:{}/", scheme, host_str, port),
            host: host_str.clone(),
            path: format!(":{}", port),
            headers: std::collections::HashMap::new(),
            body: None,
            content_type: None,
            http_version: Some("SOCKS5".to_string()),
        },
    );
    entry.is_passthrough = true;
    let _ = traffic_store.store_request(&entry);
    let _ = traffic_tx.send(entry.clone());

    // ── 5. Dial the target ─────────────────────────────────────────────
    let target_addr = format!("{}:{}", host_str, port);
    let upstream_socket =
        match tokio::time::timeout(Duration::from_secs(30), TcpStream::connect(&target_addr)).await
        {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                warn!("SOCKS5: failed to connect to {}: {}", target_addr, e);
                let reply = map_io_error_to_reply(&e);
                let _ = client_socket
                    .write_all(&build_connect_reply(reply, None))
                    .await;
                let _ = traffic_store.store_response(
                    &entry.id,
                    &crate::traffic::ResponseData {
                        status_code: 502,
                        status_message: Some(format!(
                            "Bad Gateway (SOCKS5 connect failed: {})",
                            reply.as_str()
                        )),
                        headers: std::collections::HashMap::new(),
                        body: Some(
                            format!(
                                "SOCKS5 CONNECT to {} failed.\n\nError: {}\n\n\
                                 The target was unreachable. SOCKS5 tunnels the \
                                 connection directly; the request/response contents \
                                 are not inspected.",
                                target_addr, e
                            )
                            .into_bytes(),
                        ),
                        content_type: Some("text/plain".to_string()),
                        duration_ms: 0,
                        http_version: Some("SOCKS5".to_string()),
                    },
                );
                let _ = traffic_tx.send(entry);
                return Err(Error::Proxy(format!("SOCKS5 connect failed: {}", e)));
            }
            Err(_) => {
                warn!("SOCKS5: timeout connecting to {}", target_addr);
                let _ = client_socket
                    .write_all(&build_connect_reply(SocksReply::TtlExpired, None))
                    .await;
                let _ = traffic_store.store_response(
                    &entry.id,
                    &crate::traffic::ResponseData {
                        status_code: 504,
                        status_message: Some("Gateway Timeout (SOCKS5)".to_string()),
                        headers: std::collections::HashMap::new(),
                        body: Some(
                            format!("SOCKS5 CONNECT to {} timed out (30s).", target_addr)
                                .into_bytes(),
                        ),
                        content_type: Some("text/plain".to_string()),
                        duration_ms: 30000,
                        http_version: Some("SOCKS5".to_string()),
                    },
                );
                let _ = traffic_tx.send(entry);
                return Err(Error::Proxy("SOCKS5 connect timeout".into()));
            }
        };

    // ── 6. Send success reply and relay ────────────────────────────────
    let bind_addr = upstream_socket.local_addr().ok();
    client_socket
        .write_all(&build_connect_reply(SocksReply::Succeeded, bind_addr))
        .await
        .map_err(|e| Error::Proxy(format!("Failed to write SOCKS5 success reply: {}", e)))?;

    let _ = traffic_store.store_response(
        &entry.id,
        &crate::traffic::ResponseData {
            status_code: 200,
            status_message: Some("Connection Established (SOCKS5)".to_string()),
            headers: std::collections::HashMap::new(),
            body: Some(
                format!(
                    "SOCKS5 tunnel established to {}.\n\n\
                     The connection is relayed as a blind TCP tunnel. Request and \
                     response contents are not inspected. To intercept HTTPS, use \
                     the HTTP proxy port with CONNECT instead.",
                    target_addr
                )
                .into_bytes(),
            ),
            content_type: Some("text/plain".to_string()),
            duration_ms: 0,
            http_version: Some("SOCKS5".to_string()),
        },
    );
    let _ = traffic_tx.send(entry);

    relay(client_socket, upstream_socket).await;
    Ok(())
}

/// Bidirectional byte relay between client and upstream, mirroring the
/// passthrough tunnel in the proxy engine. Both directions run concurrently
/// with a generous timeout; the relay ends when either side closes or errors.
async fn relay(mut client: TcpStream, mut upstream: TcpStream) {
    let (mut client_rx, mut client_tx) = client.split();
    let (mut upstream_rx, mut upstream_tx) = upstream.split();

    let client_to_upstream = async {
        let mut buf = vec![0u8; 8192];
        loop {
            match client_rx.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if upstream_tx.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = upstream_tx.shutdown().await;
    };

    let upstream_to_client = async {
        let mut buf = vec![0u8; 8192];
        loop {
            match upstream_rx.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if client_tx.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = client_tx.shutdown().await;
    };

    tokio::try_join!(
        tokio::time::timeout(Duration::from_secs(300), client_to_upstream),
        tokio::time::timeout(Duration::from_secs(300), upstream_to_client),
    )
    .ok();
}

/// Map a `std::io::Error` from `TcpStream::connect` to a SOCKS5 reply code
/// so the client gets a meaningful failure reason.
fn map_io_error_to_reply(e: &std::io::Error) -> SocksReply {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::ConnectionRefused => SocksReply::ConnectionRefused,
        ErrorKind::NetworkUnreachable => SocksReply::NetworkUnreachable,
        ErrorKind::HostUnreachable => SocksReply::HostUnreachable,
        _ => SocksReply::GeneralFailure,
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    /// Build a SOCKS5 CONNECT request for a `SocketAddr`. Only IPv4 is
    /// supported here (the tests bind to 127.0.0.1).
    fn ipv4_connect_request(target: std::net::SocketAddr) -> Vec<u8> {
        let ip = match target {
            std::net::SocketAddr::V4(a) => *a.ip(),
            _ => panic!("test target must be IPv4"),
        };
        let octets = ip.octets();
        let port_bytes = target.port().to_be_bytes();
        let mut req = vec![0x05, CMD_CONNECT, 0x00, ATYP_IPV4];
        req.extend_from_slice(&octets);
        req.extend_from_slice(&port_bytes);
        req
    }

    #[test]
    fn parse_greeting_no_auth() {
        // VER=5 NMETHODS=1 METHODS=[0x00]
        let buf = [0x05, 0x01, 0x00];
        let g = parse_greeting(&buf).expect("valid greeting");
        assert_eq!(g.methods, vec![METHOD_NO_AUTH]);
    }

    #[test]
    fn parse_greeting_multiple_methods() {
        // VER=5 NMETHODS=2 METHODS=[0x00, 0x02]
        let buf = [0x05, 0x02, 0x00, 0x02];
        let g = parse_greeting(&buf).expect("valid greeting");
        assert_eq!(g.methods, vec![METHOD_NO_AUTH, METHOD_USER_PASS]);
    }

    #[test]
    fn parse_greeting_wrong_version_rejected() {
        let buf = [0x04, 0x01, 0x00]; // SOCKS4
        let err = parse_greeting(&buf).unwrap_err();
        assert!(err.to_string().contains("Unsupported SOCKS version"));
    }

    #[test]
    fn parse_greeting_truncated_rejected() {
        // Declares 2 methods but provides 0
        let buf = [0x05, 0x02];
        let err = parse_greeting(&buf).unwrap_err();
        assert!(err.to_string().contains("truncated"));
    }

    #[test]
    fn parse_greeting_empty_rejected() {
        let err = parse_greeting(&[]).unwrap_err();
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn select_method_prefers_no_auth_when_not_required() {
        let g = Greeting {
            methods: vec![METHOD_NO_AUTH, METHOD_USER_PASS],
        };
        assert_eq!(select_method(&g, false), METHOD_NO_AUTH);
    }

    #[test]
    fn select_method_requires_auth_when_configured() {
        let g = Greeting {
            methods: vec![METHOD_NO_AUTH, METHOD_USER_PASS],
        };
        assert_eq!(select_method(&g, true), METHOD_USER_PASS);
    }

    #[test]
    fn select_method_no_acceptable_when_client_lacks_required_auth() {
        let g = Greeting {
            methods: vec![METHOD_NO_AUTH],
        };
        assert_eq!(select_method(&g, true), METHOD_NO_ACCEPTABLE);
    }

    #[test]
    fn select_method_no_acceptable_when_no_methods_offered() {
        let g = Greeting { methods: vec![] };
        assert_eq!(select_method(&g, false), METHOD_NO_ACCEPTABLE);
    }

    #[test]
    fn select_method_falls_back_to_user_pass_if_only_option() {
        let g = Greeting {
            methods: vec![METHOD_USER_PASS],
        };
        assert_eq!(select_method(&g, false), METHOD_USER_PASS);
    }

    #[test]
    fn parse_auth_credentials_valid() {
        // VER=1 ULEN=4 "user" PLEN=4 "pass"
        let buf = [
            0x01, 0x04, b'u', b's', b'e', b'r', 0x04, b'p', b'a', b's', b's',
        ];
        let creds = parse_auth_credentials(&buf).expect("valid auth");
        assert_eq!(creds.username, "user");
        assert_eq!(creds.password, "pass");
    }

    #[test]
    fn parse_auth_credentials_wrong_version_rejected() {
        let buf = [0x05, 0x01, b'a'];
        let err = parse_auth_credentials(&buf).unwrap_err();
        assert!(err.to_string().contains("Unsupported SOCKS5 auth version"));
    }

    #[test]
    fn parse_auth_credentials_truncated_username_rejected() {
        // ULEN=4 but only 2 username bytes provided
        let buf = [0x01, 0x04, b'a', b'b'];
        let err = parse_auth_credentials(&buf).unwrap_err();
        assert!(err.to_string().contains("truncated"));
    }

    #[test]
    fn parse_auth_credentials_empty_strings_allowed() {
        // VER=1 ULEN=0 PLEN=0
        let buf = [0x01, 0x00, 0x00];
        let creds = parse_auth_credentials(&buf).expect("valid empty auth");
        assert_eq!(creds.username, "");
        assert_eq!(creds.password, "");
    }

    #[test]
    fn parse_request_ipv4_connect() {
        // VER=5 CMD=1(CONNECT) RSV=0 ATYP=1(IPv4) 127.0.0.1 port=8080
        let port_bytes = 8080u16.to_be_bytes();
        let mut buf = vec![0x05, CMD_CONNECT, 0x00, ATYP_IPV4, 127, 0, 0, 1];
        buf.extend_from_slice(&port_bytes);
        let req = parse_request(&buf).expect("valid request");
        assert_eq!(req.command, CMD_CONNECT);
        assert_eq!(req.host, SocksHost::Ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(req.port, 8080);
    }

    #[test]
    fn parse_request_domain_connect() {
        // VER=5 CMD=1 RSV=0 ATYP=3(domain) len=11 "example.com" port=443
        let domain = b"example.com";
        let port_bytes = 443u16.to_be_bytes();
        let mut buf = vec![0x05, CMD_CONNECT, 0x00, ATYP_DOMAIN, domain.len() as u8];
        buf.extend_from_slice(domain);
        buf.extend_from_slice(&port_bytes);
        let req = parse_request(&buf).expect("valid request");
        assert_eq!(req.command, CMD_CONNECT);
        assert_eq!(req.host, SocksHost::Domain("example.com".to_string()));
        assert_eq!(req.port, 443);
    }

    #[test]
    fn parse_request_ipv6_connect() {
        // ::1 port=443
        let ip = Ipv6Addr::LOCALHOST;
        let port_bytes = 443u16.to_be_bytes();
        let mut buf = vec![0x05, CMD_CONNECT, 0x00, ATYP_IPV6];
        buf.extend_from_slice(&ip.octets());
        buf.extend_from_slice(&port_bytes);
        let req = parse_request(&buf).expect("valid request");
        assert_eq!(req.host, SocksHost::Ipv6(ip));
        assert_eq!(req.port, 443);
    }

    #[test]
    fn parse_request_unsupported_command_parsed_but_flagged() {
        // BIND command — parsing succeeds; the caller checks CMD_CONNECT.
        let port_bytes = 80u16.to_be_bytes();
        let mut buf = vec![0x05, CMD_BIND, 0x00, ATYP_IPV4, 1, 1, 1, 1];
        buf.extend_from_slice(&port_bytes);
        let req = parse_request(&buf).expect("parses fine");
        assert_eq!(req.command, CMD_BIND);
    }

    #[test]
    fn parse_request_unsupported_address_type_rejected() {
        let port_bytes = 80u16.to_be_bytes();
        let mut buf = vec![0x05, CMD_CONNECT, 0x00, 0x09]; // ATYP=9 invalid
        buf.extend_from_slice(&port_bytes);
        let err = parse_request(&buf).unwrap_err();
        assert!(err.to_string().contains("Unsupported SOCKS5 address type"));
    }

    #[test]
    fn parse_request_wrong_version_rejected() {
        let buf = [0x04, CMD_CONNECT, 0x00, ATYP_IPV4, 1, 1, 1, 1, 0, 80];
        let err = parse_request(&buf).unwrap_err();
        assert!(err
            .to_string()
            .contains("Unsupported SOCKS version in request"));
    }

    #[test]
    fn parse_request_truncated_ipv4_rejected() {
        // IPv4 address present but no port bytes — the IPv4 branch detects
        // the truncation (needs 4 octets + 2 port bytes after the header).
        let buf = [0x05, CMD_CONNECT, 0x00, ATYP_IPV4, 1, 1, 1, 1];
        let err = parse_request(&buf).unwrap_err();
        assert!(err.to_string().contains("IPv4 request truncated"));
    }

    #[test]
    fn parse_request_domain_truncated_name_rejected() {
        // Domain length says 5 but only 2 name bytes (and no port) follow.
        let full = vec![0x05, CMD_CONNECT, 0x00, ATYP_DOMAIN, 5, b'a', b'b'];
        let err = parse_request(&full).unwrap_err();
        assert!(err.to_string().contains("domain request truncated (name)"));
    }

    #[test]
    fn build_method_reply_format() {
        assert_eq!(build_method_reply(METHOD_NO_AUTH), [0x05, 0x00]);
        assert_eq!(build_method_reply(METHOD_USER_PASS), [0x05, 0x02]);
        assert_eq!(build_method_reply(METHOD_NO_ACCEPTABLE), [0x05, 0xFF]);
    }

    #[test]
    fn build_auth_status_reply_format() {
        assert_eq!(build_auth_status_reply(true), [0x01, 0x00]);
        assert_eq!(build_auth_status_reply(false), [0x01, 0x01]);
    }

    #[test]
    fn build_connect_reply_success_ipv4_bind() {
        let bind = "127.0.0.1:8888".parse::<SocketAddr>().unwrap();
        let reply = build_connect_reply(SocksReply::Succeeded, Some(bind));
        // VER REP RSV ATYP=1 4 octets 2 port bytes
        assert_eq!(reply[0], SOCKS_VERSION);
        assert_eq!(reply[1], SocksReply::Succeeded as u8);
        assert_eq!(reply[2], 0x00);
        assert_eq!(reply[3], ATYP_IPV4);
        assert_eq!(&reply[4..8], &[127, 0, 0, 1]);
        assert_eq!(&reply[8..10], &8888u16.to_be_bytes());
    }

    #[test]
    fn build_connect_reply_failure_no_bind() {
        let reply = build_connect_reply(SocksReply::ConnectionRefused, None);
        assert_eq!(reply[0], SOCKS_VERSION);
        assert_eq!(reply[1], SocksReply::ConnectionRefused as u8);
        assert_eq!(reply[3], ATYP_IPV4);
        assert_eq!(&reply[4..8], &[0, 0, 0, 0]);
    }

    #[test]
    fn socks_reply_descriptions() {
        assert_eq!(SocksReply::Succeeded.as_str(), "succeeded");
        assert_eq!(SocksReply::ConnectionRefused.as_str(), "connection refused");
        assert_eq!(
            SocksReply::CommandNotSupported.as_str(),
            "command not supported"
        );
    }

    #[test]
    fn map_io_error_to_reply_classification() {
        assert_eq!(
            map_io_error_to_reply(&std::io::Error::from(std::io::ErrorKind::ConnectionRefused)),
            SocksReply::ConnectionRefused
        );
        assert_eq!(
            map_io_error_to_reply(&std::io::Error::from(
                std::io::ErrorKind::NetworkUnreachable
            )),
            SocksReply::NetworkUnreachable
        );
        assert_eq!(
            map_io_error_to_reply(&std::io::Error::from(std::io::ErrorKind::HostUnreachable)),
            SocksReply::HostUnreachable
        );
        // Unknown kind → general failure
        assert_eq!(
            map_io_error_to_reply(&std::io::Error::from(std::io::ErrorKind::Other)),
            SocksReply::GeneralFailure
        );
    }

    #[test]
    fn socks_host_as_str() {
        assert_eq!(
            SocksHost::Ipv4(Ipv4Addr::new(1, 2, 3, 4)).as_str(),
            "1.2.3.4"
        );
        assert_eq!(
            SocksHost::Domain("example.com".into()).as_str(),
            "example.com"
        );
    }

    /// End-to-end handshake over a loopback TCP pair. This exercises the
    /// async handler with a real socket pair (no external server needed).
    #[tokio::test]
    async fn handshake_no_auth_then_connect_to_local_listener() {
        use tokio::net::TcpListener;

        // A dummy "target" the SOCKS proxy will dial.
        let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_addr = target.local_addr().unwrap();
        let (target_tx, _target_rx) = tokio::sync::oneshot::channel::<()>();
        let target_task = tokio::spawn(async move {
            let (mut s, _) = target.accept().await.unwrap();
            let mut buf = [0u8; 5];
            s.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"hello");
            s.write_all(b"world").await.unwrap();
            let _ = target_tx.send(());
        });

        // The SOCKS server side: we drive handle_socks5_connection directly
        // with a connected client socket. We use a TCP listener as a
        // pipe pair since tokio doesn't expose a raw socketpair.
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();

        let (traffic_tx, _) = tokio::sync::broadcast::channel(16);
        let db =
            std::env::temp_dir().join(format!("madhyamas-socks-test-{}.db", uuid::Uuid::new_v4()));
        let store = crate::traffic::TrafficStore::new(db.to_str().unwrap()).unwrap();

        let server_task = tokio::spawn(async move {
            let (sock, _) = proxy_listener.accept().await.unwrap();
            handle_socks5_connection(sock, &store, &traffic_tx, false, None, None)
                .await
                .unwrap();
        });

        // Client side: connect to the SOCKS listener and perform the handshake.
        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        // Greeting: no-auth only
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        let mut method_reply = [0u8; 2];
        client.read_exact(&mut method_reply).await.unwrap();
        assert_eq!(method_reply, [0x05, 0x00]);

        // Request: CONNECT to target_addr (IPv4)
        let req = ipv4_connect_request(target_addr);
        client.write_all(&req).await.unwrap();

        // Read the SOCKS5 success reply (variable length: 10 for IPv4).
        let mut reply_hdr = [0u8; 4];
        client.read_exact(&mut reply_hdr).await.unwrap();
        assert_eq!(reply_hdr[0], SOCKS_VERSION);
        assert_eq!(reply_hdr[1], SocksReply::Succeeded as u8);
        // Consume the BND.ADDR + BND.PORT (IPv4 → 4 + 2 bytes).
        let mut rest = vec![0u8; 6];
        client.read_exact(&mut rest).await.unwrap();

        // Now the tunnel is established: send bytes through the SOCKS proxy
        // and verify they reach the target, and the response comes back.
        client.write_all(b"hello").await.unwrap();
        let mut resp = [0u8; 5];
        client.read_exact(&mut resp).await.unwrap();
        assert_eq!(&resp, b"world");

        // Close the client so the relay loop sees EOF and exits promptly
        // (otherwise it would block until the 300s relay timeout).
        drop(client);

        target_task.await.unwrap();
        // The server task runs the relay; give it a moment to drain.
        let _ = tokio::time::timeout(Duration::from_secs(5), server_task).await;
        let _ = std::fs::remove_file(&db);
    }

    /// Verifies that when auth is required and the client offers only no-auth,
    /// the server replies with NO-ACCEPTABLE-METHODS and the handshake fails.
    #[tokio::test]
    async fn handshake_rejects_when_auth_required_but_not_offered() {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();

        let (traffic_tx, _) = tokio::sync::broadcast::channel(16);
        let db = std::env::temp_dir().join(format!(
            "madhyamas-socks-auth-test-{}.db",
            uuid::Uuid::new_v4()
        ));
        let store = crate::traffic::TrafficStore::new(db.to_str().unwrap()).unwrap();

        let server_task = tokio::spawn(async move {
            let (sock, _) = proxy_listener.accept().await.unwrap();
            // Expect this to error out (no acceptable method).
            let _ = handle_socks5_connection(
                sock,
                &store,
                &traffic_tx,
                true, // require auth
                Some("user"),
                Some("pass"),
            )
            .await;
        });

        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        // Greeting: offer only no-auth (0x00)
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        let mut method_reply = [0u8; 2];
        client.read_exact(&mut method_reply).await.unwrap();
        assert_eq!(method_reply, [0x05, METHOD_NO_ACCEPTABLE]);

        server_task.await.unwrap();
        let _ = std::fs::remove_file(&db);
    }

    /// Verifies username/password authentication succeeds with correct
    /// credentials and the tunnel is established.
    #[tokio::test]
    async fn handshake_user_pass_auth_success() {
        let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_addr = target.local_addr().unwrap();
        let target_task = tokio::spawn(async move {
            let (mut s, _) = target.accept().await.unwrap();
            let mut buf = [0u8; 3];
            s.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"hi!");
            s.write_all(b"yo!").await.unwrap();
        });

        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();

        let (traffic_tx, _) = tokio::sync::broadcast::channel(16);
        let db = std::env::temp_dir().join(format!(
            "madhyamas-socks-userpass-test-{}.db",
            uuid::Uuid::new_v4()
        ));
        let store = crate::traffic::TrafficStore::new(db.to_str().unwrap()).unwrap();

        let server_task = tokio::spawn(async move {
            let (sock, _) = proxy_listener.accept().await.unwrap();
            handle_socks5_connection(
                sock,
                &store,
                &traffic_tx,
                true,
                Some("alice"),
                Some("secret"),
            )
            .await
            .unwrap();
        });

        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        // Greeting: offer both no-auth and user/pass
        client.write_all(&[0x05, 0x02, 0x00, 0x02]).await.unwrap();
        let mut method_reply = [0u8; 2];
        client.read_exact(&mut method_reply).await.unwrap();
        assert_eq!(method_reply, [0x05, METHOD_USER_PASS]);

        // Auth: username "alice", password "secret"
        let mut auth = vec![0x01, 0x05, b'a', b'l', b'i', b'c', b'e', 0x06];
        auth.extend_from_slice(b"secret");
        client.write_all(&auth).await.unwrap();
        let mut auth_reply = [0u8; 2];
        client.read_exact(&mut auth_reply).await.unwrap();
        assert_eq!(auth_reply, [0x01, 0x00]);

        // CONNECT request to target
        let req = ipv4_connect_request(target_addr);
        client.write_all(&req).await.unwrap();
        let mut reply_hdr = [0u8; 4];
        client.read_exact(&mut reply_hdr).await.unwrap();
        assert_eq!(reply_hdr[1], SocksReply::Succeeded as u8);
        let mut rest = vec![0u8; 6];
        client.read_exact(&mut rest).await.unwrap();

        client.write_all(b"hi!").await.unwrap();
        let mut resp = [0u8; 3];
        client.read_exact(&mut resp).await.unwrap();
        assert_eq!(&resp, b"yo!");

        // Close the client so the relay drains and exits promptly.
        drop(client);

        target_task.await.unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(5), server_task).await;
        let _ = std::fs::remove_file(&db);
    }

    /// Verifies that incorrect credentials are rejected.
    #[tokio::test]
    async fn handshake_user_pass_auth_wrong_password_rejected() {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();

        let (traffic_tx, _) = tokio::sync::broadcast::channel(16);
        let db = std::env::temp_dir().join(format!(
            "madhyamas-socks-badauth-test-{}.db",
            uuid::Uuid::new_v4()
        ));
        let store = crate::traffic::TrafficStore::new(db.to_str().unwrap()).unwrap();

        let server_task = tokio::spawn(async move {
            let (sock, _) = proxy_listener.accept().await.unwrap();
            let _ = handle_socks5_connection(
                sock,
                &store,
                &traffic_tx,
                true,
                Some("alice"),
                Some("secret"),
            )
            .await;
        });

        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        client.write_all(&[0x05, 0x02, 0x00, 0x02]).await.unwrap();
        let mut method_reply = [0u8; 2];
        client.read_exact(&mut method_reply).await.unwrap();
        assert_eq!(method_reply, [0x05, METHOD_USER_PASS]);

        // Wrong password
        let mut auth = vec![0x01, 0x05, b'a', b'l', b'i', b'c', b'e', 0x04];
        auth.extend_from_slice(b"nope");
        client.write_all(&auth).await.unwrap();
        let mut auth_reply = [0u8; 2];
        client.read_exact(&mut auth_reply).await.unwrap();
        assert_eq!(auth_reply, [0x01, 0x01]); // failure

        server_task.await.unwrap();
        let _ = std::fs::remove_file(&db);
    }
}

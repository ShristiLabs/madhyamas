//! Client-side upstream proxy chaining.
//!
//! This module provides the logic for establishing a tunneled TCP connection
//! to an arbitrary `host:port` *through* an upstream (external) proxy. It is
//! used by the proxy engine's `CONNECT` and SSL-passthrough code paths when
//! [`crate::config::UpstreamProxyConfig`] is enabled.
//!
//! Two upstream proxy protocols are supported for raw tunneling:
//!
//! - **HTTP CONNECT** (RFC 7231 §4.3.6): the proxy opens a TCP connection to
//!   the upstream proxy and sends `CONNECT <target>:<port> HTTP/1.1`.
//!   Optional `Proxy-Authorization` headers carry Basic-auth credentials.
//! - **SOCKS5** (RFC 1928/1929): the proxy performs the SOCKS5 client
//!   handshake (greeting → optional username/password auth → CONNECT
//!   request) and then relays raw bytes.
//!
//! HTTPS upstream proxies (`protocol = "https"`) are supported for the
//! reqwest-based HTTP forwarding path (reqwest handles TLS internally) but
//! **not** for raw TCP tunneling (CONNECT/passthrough), because the TLS
//! layer cannot be returned as a plain `TcpStream`. An error is returned if
//! an HTTPS upstream proxy is configured for a tunnel path.
//!
//! The bypass list ([`UpstreamProxyConfig::should_bypass`]) is evaluated by
//! the caller *before* invoking [`connect_through_upstream`]; this function
//! assumes the target should go through the proxy.
//!
//! All of the pure protocol-encoding helpers (`build_http_connect_request`,
//! `build_socks5_greeting`, `parse_http_connect_response`,
//! `parse_socks5_method_reply`, etc.) operate on byte slices and are unit
//! tested without any I/O. The async [`connect_through_upstream`] function
//! drives the handshake over a real [`tokio::net::TcpStream`].

use crate::config::UpstreamProxyConfig;
use crate::proxy::socks::{
    ATYP_DOMAIN, ATYP_IPV4, ATYP_IPV6, METHOD_NO_ACCEPTABLE, METHOD_NO_AUTH, METHOD_USER_PASS,
    SOCKS_VERSION, USER_PASS_VERSION,
};
use crate::Error;
use base64::Engine;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, warn};

/// Maximum number of bytes to read when parsing the upstream proxy's
/// HTTP CONNECT response. Responses are small (status line + headers);
/// 8 KiB is more than enough and prevents unbounded reads.
const HTTP_CONNECT_RESPONSE_MAX: usize = 8192;

/// Establish a tunneled TCP connection to `target_host:target_port`
/// through the configured upstream proxy.
///
/// Returns a `TcpStream` that is ready for raw byte relay — the upstream
/// proxy has already completed its CONNECT/SOCKS5 handshake and is
/// forwarding data to the target.
///
/// The caller is responsible for checking the bypass list *before* calling
/// this function. If the target should connect directly, the caller should
/// use `TcpStream::connect` instead.
///
/// # Supported protocols for tunneling
///
/// - `http`: HTTP CONNECT proxy (most common). The TCP connection to the
///   proxy is used directly after the CONNECT handshake.
/// - `socks5`: SOCKS5 proxy. The TCP connection is used directly after the
///   SOCKS5 handshake.
/// - `https`: **Not supported for raw tunneling** (CONNECT/passthrough paths)
///   because the TLS layer cannot be returned as a plain `TcpStream`. HTTPS
///   upstream proxies still work for the reqwest-based HTTP forwarding path
///   (which uses reqwest's built-in TLS). An error is returned if an `https`
///   upstream proxy is configured for a tunnel path.
///
/// # Errors
///
/// Returns an error if:
/// - The upstream proxy is disabled or misconfigured
/// - The TCP connection to the upstream proxy fails or times out
/// - The HTTP CONNECT response is not `2xx`
/// - The SOCKS5 handshake fails (auth rejected, connect refused, etc.)
/// - An `https` upstream proxy is used for a tunnel path (not supported)
/// - The `protocol` field has an unsupported value
pub async fn connect_through_upstream(
    upstream: &UpstreamProxyConfig,
    target_host: &str,
    target_port: u16,
) -> crate::Result<TcpStream> {
    let proxy_addr = format!("{}:{}", upstream.host, upstream.port);

    debug!(
        "Connecting to upstream proxy {} ({}) for target {}:{}",
        proxy_addr, upstream.protocol, target_host, target_port
    );

    // Connect to the upstream proxy with a generous timeout (corporate
    // proxies can be slow, especially over WAN links).
    let tcp = tokio::time::timeout(Duration::from_secs(30), TcpStream::connect(&proxy_addr))
        .await
        .map_err(|_| {
            Error::Proxy(format!(
                "Timeout connecting to upstream proxy {}",
                proxy_addr
            ))
        })?
        .map_err(|e| {
            Error::Proxy(format!(
                "Failed to connect to upstream proxy {}: {}",
                proxy_addr, e
            ))
        })?;

    // Disable Nagle's algorithm for low-latency relay.
    let _ = tcp.set_nodelay(true);

    match upstream.protocol.to_lowercase().as_str() {
        "http" => http_connect_handshake(tcp, upstream, target_host, target_port).await,
        "socks5" => socks5_connect_handshake(tcp, upstream, target_host, target_port).await,
        "https" => Err(Error::Proxy(
            "HTTPS upstream proxy is not supported for raw TCP tunneling \
             (CONNECT/passthrough). It works for HTTP forwarding via reqwest. \
             Use an HTTP or SOCKS5 upstream proxy for tunnel paths."
                .into(),
        )),
        other => Err(Error::Config(format!(
            "Unsupported upstream proxy protocol: `{other}`"
        ))),
    }
}

/// Drive the HTTP CONNECT handshake over an already-connected `TcpStream`.
///
/// Sends `CONNECT <host>:<port> HTTP/1.1` with optional
/// `Proxy-Authorization` and waits for a `2xx` response. On success the
/// stream is returned as a raw tunnel ready for byte relay.
async fn http_connect_handshake(
    mut stream: TcpStream,
    upstream: &UpstreamProxyConfig,
    target_host: &str,
    target_port: u16,
) -> crate::Result<TcpStream> {
    let request = build_http_connect_request(upstream, target_host, target_port);
    stream
        .write_all(&request)
        .await
        .map_err(|e| Error::Proxy(format!("Failed to send CONNECT to upstream proxy: {}", e)))?;

    // Read the response. We read in a loop until we find the end of the
    // headers (\r\n\r\n) or hit the buffer cap.
    let mut buf = vec![0u8; HTTP_CONNECT_RESPONSE_MAX];
    let mut total = 0;
    loop {
        if total >= buf.len() {
            return Err(Error::Proxy(format!(
                "Upstream proxy CONNECT response too large (>{})",
                buf.len()
            )));
        }
        let n = stream.read(&mut buf[total..]).await.map_err(|e| {
            Error::Proxy(format!(
                "Failed to read CONNECT response from upstream proxy: {}",
                e
            ))
        })?;
        if n == 0 {
            return Err(Error::Proxy(
                "Upstream proxy closed connection during CONNECT response".into(),
            ));
        }
        total += n;
        if let Some(status) = parse_http_connect_response(&buf[..total])? {
            if !status.is_success() {
                warn!(
                    "Upstream proxy rejected CONNECT to {}:{}: {} {}",
                    target_host,
                    target_port,
                    status.code,
                    status.reason.trim()
                );
                return Err(Error::Proxy(format!(
                    "Upstream proxy rejected CONNECT ({} {}): {}",
                    status.code,
                    status.reason.trim(),
                    status.reason.trim()
                )));
            }
            debug!(
                "Upstream proxy CONNECT established for {}:{} (HTTP {})",
                target_host, target_port, status.code
            );
            break;
        }
        // Not enough data yet — keep reading.
    }

    Ok(stream)
}

/// Build the HTTP CONNECT request bytes for an upstream proxy.
///
/// The request includes:
/// - `CONNECT <host>:<port> HTTP/1.1` request line
/// - `Host: <host>:<port>` header
/// - `Proxy-Authorization: Basic <base64>` header when auth is configured
/// - `Proxy-Connection: Keep-Alive` header
/// - Terminating `\r\n`
///
/// This is a pure function — no I/O — so it can be unit tested directly.
pub fn build_http_connect_request(
    upstream: &UpstreamProxyConfig,
    target_host: &str,
    target_port: u16,
) -> Vec<u8> {
    let mut request = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nProxy-Connection: Keep-Alive\r\n",
        host = target_host,
        port = target_port
    );

    if upstream.auth_enabled() {
        let user = upstream.auth_username.as_deref().unwrap_or("");
        let pass = upstream.auth_password.as_deref().unwrap_or("");
        let credentials = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", user, pass).as_bytes());
        request.push_str(&format!("Proxy-Authorization: Basic {}\r\n", credentials));
    }

    request.push_str("\r\n");
    request.into_bytes()
}

/// Parsed HTTP CONNECT response status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpConnectStatus {
    /// HTTP status code (e.g. 200, 407).
    pub code: u16,
    /// Reason phrase (e.g. "Connection Established").
    pub reason: String,
}

impl HttpConnectStatus {
    /// Whether the status code indicates a successful tunnel (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.code)
    }
}

/// Parse an HTTP CONNECT response from a byte buffer.
///
/// Returns:
/// - `Ok(None)` if the buffer doesn't yet contain a complete response
///   (the caller should read more data and retry).
/// - `Ok(Some(status))` if the response was parsed successfully.
/// - `Err` if the buffer contains malformed data.
///
/// A complete response is defined by the presence of `\r\n\r\n`
/// (end-of-headers). Only the status line is parsed; headers are
/// skipped because the tunnel is now opaque.
pub fn parse_http_connect_response(buf: &[u8]) -> crate::Result<Option<HttpConnectStatus>> {
    // Find the end-of-headers marker.
    let header_end = find_subsequence(buf, b"\r\n\r\n");
    let Some(header_end) = header_end else {
        return Ok(None); // need more data
    };

    let header_block = &buf[..header_end];
    let text = std::str::from_utf8(header_block).map_err(|e| {
        Error::Proxy(format!(
            "Upstream CONNECT response is not valid UTF-8: {}",
            e
        ))
    })?;

    let status_line = text
        .lines()
        .next()
        .ok_or_else(|| Error::Proxy("Upstream CONNECT response has no status line".into()))?;

    // Parse: "HTTP/1.1 200 Connection Established"
    let mut parts = status_line.split_whitespace();
    let _version = parts
        .next()
        .ok_or_else(|| Error::Proxy("Upstream CONNECT status line missing HTTP version".into()))?;
    let code_str = parts
        .next()
        .ok_or_else(|| Error::Proxy("Upstream CONNECT status line missing status code".into()))?;
    let code: u16 = code_str.parse().map_err(|_| {
        Error::Proxy(format!(
            "Upstream CONNECT status code is not a number: `{}`",
            code_str
        ))
    })?;
    let reason = parts.collect::<Vec<_>>().join(" ");

    Ok(Some(HttpConnectStatus { code, reason }))
}

/// Find the first occurrence of `needle` in `haystack`. Returns the byte
/// index of the start of the match, or `None`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Build the SOCKS5 client greeting (method selection) message.
///
/// Wire format: `VER(1) | NMETHODS(1) | METHODS(...)`.
/// We offer both no-auth (0x00) and username/password (0x02) so the
/// server can pick whichever it supports.
pub fn build_socks5_greeting() -> Vec<u8> {
    vec![SOCKS_VERSION, 2, METHOD_NO_AUTH, METHOD_USER_PASS]
}

/// Build the SOCKS5 username/password auth sub-protocol request
/// (RFC 1929).
///
/// Wire format:
/// `VER(1,=0x01) | ULEN(1) | UNAME(ULEN) | PLEN(1) | PASSWD(PLEN)`.
pub fn build_socks5_auth_request(username: &str, password: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 + username.len() + 1 + password.len());
    out.push(USER_PASS_VERSION);
    out.push(username.len() as u8);
    out.extend_from_slice(username.as_bytes());
    out.push(password.len() as u8);
    out.extend_from_slice(password.as_bytes());
    out
}

/// Build the SOCKS5 CONNECT request for a target `host:port`.
///
/// Uses the domain address type (`ATYP_DOMAIN`) so the upstream SOCKS
/// proxy performs DNS resolution — this matches how most SOCKS clients
/// (curl `--socks5-hostname`, Firefox) behave and avoids leaking the
/// target's IP through a DNS lookup on the proxy host.
pub fn build_socks5_connect_request(host: &str, port: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 1 + host.len() + 2);
    out.push(SOCKS_VERSION);
    out.push(0x01); // CMD_CONNECT
    out.push(0x00); // reserved
    out.push(ATYP_DOMAIN);
    out.push(host.len() as u8);
    out.extend_from_slice(host.as_bytes());
    out.extend_from_slice(&port.to_be_bytes());
    out
}

/// Parse the SOCKS5 method-selection reply from the server.
///
/// Wire format: `VER(1) | METHOD(1)`.
/// Returns the selected method, or an error if the reply is malformed
/// or the server rejected all methods (`0xFF`).
pub fn parse_socks5_method_reply(buf: &[u8]) -> crate::Result<u8> {
    if buf.len() < 2 {
        return Err(Error::Proxy("SOCKS5 method reply too short".into()));
    }
    if buf[0] != SOCKS_VERSION {
        return Err(Error::Proxy(format!(
            "SOCKS5 method reply has wrong version: 0x{:02x}",
            buf[0]
        )));
    }
    let method = buf[1];
    if method == METHOD_NO_ACCEPTABLE {
        return Err(Error::Proxy(
            "SOCKS5 upstream proxy rejected all authentication methods".into(),
        ));
    }
    Ok(method)
}

/// Parse the SOCKS5 username/password auth status reply (RFC 1929).
///
/// Wire format: `VER(1,=0x01) | STATUS(1)`. `STATUS == 0` means success.
pub fn parse_socks5_auth_reply(buf: &[u8]) -> crate::Result<bool> {
    if buf.len() < 2 {
        return Err(Error::Proxy("SOCKS5 auth reply too short".into()));
    }
    if buf[0] != USER_PASS_VERSION {
        return Err(Error::Proxy(format!(
            "SOCKS5 auth reply has wrong version: 0x{:02x}",
            buf[0]
        )));
    }
    Ok(buf[1] == 0x00)
}

/// Parse the SOCKS5 CONNECT reply from the server.
///
/// Wire format:
/// `VER(1) | REP(1) | RSV(1) | ATYP(1) | BND.ADDR(variable) | BND.PORT(2)`.
/// Returns `Ok(())` if `REP == 0` (succeeded), or an error describing the
/// failure code.
pub fn parse_socks5_connect_reply(buf: &[u8]) -> crate::Result<()> {
    if buf.len() < 4 {
        return Err(Error::Proxy("SOCKS5 connect reply too short".into()));
    }
    if buf[0] != SOCKS_VERSION {
        return Err(Error::Proxy(format!(
            "SOCKS5 connect reply has wrong version: 0x{:02x}",
            buf[0]
        )));
    }
    let rep = buf[1];
    if rep == 0x00 {
        return Ok(()); // succeeded
    }
    let reason = match rep {
        0x01 => "general SOCKS server failure",
        0x02 => "connection not allowed by ruleset",
        0x03 => "network unreachable",
        0x04 => "host unreachable",
        0x05 => "connection refused",
        0x06 => "TTL expired",
        0x07 => "command not supported",
        0x08 => "address type not supported",
        _ => "unknown SOCKS5 error",
    };
    Err(Error::Proxy(format!(
        "SOCKS5 upstream proxy CONNECT failed (code 0x{:02x}): {}",
        rep, reason
    )))
}

/// Drive the SOCKS5 client handshake over an already-connected TCP stream.
///
/// Steps:
/// 1. Send greeting (offer no-auth + username/password methods)
/// 2. Read method reply; if username/password selected, send auth request
/// 3. Send CONNECT request for `target_host:target_port`
/// 4. Read connect reply; on success the stream is a raw tunnel
async fn socks5_connect_handshake(
    mut stream: TcpStream,
    upstream: &UpstreamProxyConfig,
    target_host: &str,
    target_port: u16,
) -> crate::Result<TcpStream> {
    // 1. Greeting
    stream
        .write_all(&build_socks5_greeting())
        .await
        .map_err(|e| Error::Proxy(format!("SOCKS5 greeting write failed: {}", e)))?;

    let mut buf = [0u8; 2];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| Error::Proxy(format!("SOCKS5 method reply read failed: {}", e)))?;
    let method = parse_socks5_method_reply(&buf)?;

    // 2. Auth (if required by the server)
    if method == METHOD_USER_PASS {
        let username = upstream.auth_username.as_deref().unwrap_or("");
        let password = upstream.auth_password.as_deref().unwrap_or("");
        stream
            .write_all(&build_socks5_auth_request(username, password))
            .await
            .map_err(|e| Error::Proxy(format!("SOCKS5 auth write failed: {}", e)))?;

        let mut auth_buf = [0u8; 2];
        stream
            .read_exact(&mut auth_buf)
            .await
            .map_err(|e| Error::Proxy(format!("SOCKS5 auth reply read failed: {}", e)))?;
        let success = parse_socks5_auth_reply(&auth_buf)?;
        if !success {
            return Err(Error::Proxy(
                "SOCKS5 upstream proxy rejected username/password credentials".into(),
            ));
        }
        debug!("SOCKS5 upstream proxy auth succeeded");
    }

    // 3. CONNECT request
    stream
        .write_all(&build_socks5_connect_request(target_host, target_port))
        .await
        .map_err(|e| Error::Proxy(format!("SOCKS5 CONNECT write failed: {}", e)))?;

    // 4. Read connect reply. The reply is variable-length (depends on
    // BND.ADDR address type), so we read the first 4 bytes to learn the
    // ATYP, then read the remaining address+port bytes.
    let mut header = [0u8; 4];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|e| Error::Proxy(format!("SOCKS5 connect reply read failed: {}", e)))?;

    // Determine how many more bytes to read based on ATYP.
    let addr_len = match header[3] {
        ATYP_IPV4 => 4,
        ATYP_IPV6 => 16,
        ATYP_DOMAIN => {
            // Need 1 length byte + N domain bytes.
            let mut len_buf = [0u8; 1];
            stream
                .read_exact(&mut len_buf)
                .await
                .map_err(|e| Error::Proxy(format!("SOCKS5 domain length read failed: {}", e)))?;
            len_buf[0] as usize
        }
        other => {
            return Err(Error::Proxy(format!(
                "SOCKS5 connect reply has unsupported ATYP: 0x{:02x}",
                other
            )));
        }
    };

    // Read the address bytes + 2-byte port.
    let mut rest = vec![0u8; addr_len + 2];
    stream
        .read_exact(&mut rest)
        .await
        .map_err(|e| Error::Proxy(format!("SOCKS5 connect reply body read failed: {}", e)))?;

    // Combine header + rest for parsing.
    let mut full_reply = header.to_vec();
    full_reply.extend_from_slice(&rest);
    parse_socks5_connect_reply(&full_reply)?;

    debug!(
        "SOCKS5 upstream proxy CONNECT established for {}:{}",
        target_host, target_port
    );
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UpstreamProxyConfig;

    fn upstream(protocol: &str, auth: bool) -> UpstreamProxyConfig {
        UpstreamProxyConfig {
            enabled: true,
            protocol: protocol.to_string(),
            host: "proxy.example.com".to_string(),
            port: 8080,
            auth_username: if auth { Some("user".to_string()) } else { None },
            auth_password: if auth { Some("pass".to_string()) } else { None },
            no_proxy_hosts: Vec::new(),
        }
    }

    // ── HTTP CONNECT request building ───────────────────────────────────

    #[test]
    fn build_http_connect_request_no_auth() {
        let u = upstream("http", false);
        let req = build_http_connect_request(&u, "api.example.com", 443);
        let text = std::str::from_utf8(&req).unwrap();
        assert!(text.starts_with("CONNECT api.example.com:443 HTTP/1.1\r\n"));
        assert!(text.contains("Host: api.example.com:443\r\n"));
        assert!(text.contains("Proxy-Connection: Keep-Alive\r\n"));
        assert!(!text.contains("Proxy-Authorization"));
        assert!(text.ends_with("\r\n\r\n"));
    }

    #[test]
    fn build_http_connect_request_with_auth() {
        let u = upstream("http", true);
        let req = build_http_connect_request(&u, "api.example.com", 443);
        let text = std::str::from_utf8(&req).unwrap();
        // base64("user:pass") == "dXNlcjpwYXNz"
        assert!(text.contains("Proxy-Authorization: Basic dXNlcjpwYXNz\r\n"));
    }

    #[test]
    fn build_http_connect_request_includes_port() {
        let u = upstream("http", false);
        let req = build_http_connect_request(&u, "example.com", 8443);
        let text = std::str::from_utf8(&req).unwrap();
        assert!(text.contains("CONNECT example.com:8443 HTTP/1.1\r\n"));
        assert!(text.contains("Host: example.com:8443\r\n"));
    }

    // ── HTTP CONNECT response parsing ───────────────────────────────────

    #[test]
    fn parse_http_connect_response_success() {
        let resp = b"HTTP/1.1 200 Connection Established\r\n\r\n";
        let status = parse_http_connect_response(resp).unwrap().unwrap();
        assert_eq!(status.code, 200);
        assert_eq!(status.reason, "Connection Established");
        assert!(status.is_success());
    }

    #[test]
    fn parse_http_connect_response_with_headers() {
        let resp = b"HTTP/1.1 200 OK\r\nProxy-Agent: squid\r\n\r\n";
        let status = parse_http_connect_response(resp).unwrap().unwrap();
        assert_eq!(status.code, 200);
        assert!(status.is_success());
    }

    #[test]
    fn parse_http_connect_response_auth_required() {
        let resp =
            b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic\r\n\r\n";
        let status = parse_http_connect_response(resp).unwrap().unwrap();
        assert_eq!(status.code, 407);
        assert!(!status.is_success());
    }

    #[test]
    fn parse_http_connect_response_forbidden() {
        let resp = b"HTTP/1.1 403 Forbidden\r\n\r\n";
        let status = parse_http_connect_response(resp).unwrap().unwrap();
        assert_eq!(status.code, 403);
        assert!(!status.is_success());
    }

    #[test]
    fn parse_http_connect_response_incomplete_returns_none() {
        let resp = b"HTTP/1.1 200 Connection Established\r\n";
        assert_eq!(parse_http_connect_response(resp).unwrap(), None);
    }

    #[test]
    fn parse_http_connect_response_empty_returns_none() {
        assert_eq!(parse_http_connect_response(b"").unwrap(), None);
    }

    #[test]
    fn parse_http_connect_response_missing_code() {
        let resp = b"HTTP/1.1\r\n\r\n";
        assert!(parse_http_connect_response(resp).is_err());
    }

    #[test]
    fn parse_http_connect_response_non_numeric_code() {
        let resp = b"HTTP/1.1 XYZ Bad\r\n\r\n";
        assert!(parse_http_connect_response(resp).is_err());
    }

    // ── SOCKS5 message building ─────────────────────────────────────────

    #[test]
    fn build_socks5_greeting_offers_both_methods() {
        let g = build_socks5_greeting();
        assert_eq!(g, vec![0x05, 0x02, 0x00, 0x02]);
    }

    #[test]
    fn build_socks5_auth_request_format() {
        let req = build_socks5_auth_request("user", "pass");
        assert_eq!(
            req,
            vec![0x01, 0x04, b'u', b's', b'e', b'r', 0x04, b'p', b'a', b's', b's']
        );
    }

    #[test]
    fn build_socks5_connect_request_domain() {
        let req = build_socks5_connect_request("example.com", 443);
        assert_eq!(req[0], 0x05); // version
        assert_eq!(req[1], 0x01); // CONNECT
        assert_eq!(req[2], 0x00); // reserved
        assert_eq!(req[3], 0x03); // ATYP_DOMAIN
        assert_eq!(req[4], 11); // domain length
        assert_eq!(&req[5..16], b"example.com");
        assert_eq!(&req[16..18], &[0x01, 0xBB]); // port 443
    }

    // ── SOCKS5 reply parsing ────────────────────────────────────────────

    #[test]
    fn parse_socks5_method_reply_no_auth() {
        assert_eq!(parse_socks5_method_reply(&[0x05, 0x00]).unwrap(), 0x00);
    }

    #[test]
    fn parse_socks5_method_reply_user_pass() {
        assert_eq!(parse_socks5_method_reply(&[0x05, 0x02]).unwrap(), 0x02);
    }

    #[test]
    fn parse_socks5_method_reply_no_acceptable_methods() {
        assert!(parse_socks5_method_reply(&[0x05, 0xFF]).is_err());
    }

    #[test]
    fn parse_socks5_method_reply_wrong_version() {
        assert!(parse_socks5_method_reply(&[0x04, 0x00]).is_err());
    }

    #[test]
    fn parse_socks5_method_reply_too_short() {
        assert!(parse_socks5_method_reply(&[0x05]).is_err());
    }

    #[test]
    fn parse_socks5_auth_reply_success() {
        assert!(parse_socks5_auth_reply(&[0x01, 0x00]).unwrap());
    }

    #[test]
    fn parse_socks5_auth_reply_failure() {
        assert!(!parse_socks5_auth_reply(&[0x01, 0x01]).unwrap());
    }

    #[test]
    fn parse_socks5_auth_reply_wrong_version() {
        assert!(parse_socks5_auth_reply(&[0x02, 0x00]).is_err());
    }

    #[test]
    fn parse_socks5_connect_reply_success() {
        let reply = vec![0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        assert!(parse_socks5_connect_reply(&reply).is_ok());
    }

    #[test]
    fn parse_socks5_connect_reply_connection_refused() {
        let reply = vec![0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        let err = parse_socks5_connect_reply(&reply).unwrap_err();
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn parse_socks5_connect_reply_host_unreachable() {
        let reply = vec![0x05, 0x04, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        let err = parse_socks5_connect_reply(&reply).unwrap_err();
        assert!(err.to_string().contains("host unreachable"));
    }

    #[test]
    fn parse_socks5_connect_reply_too_short() {
        assert!(parse_socks5_connect_reply(&[0x05, 0x00]).is_err());
    }

    #[test]
    fn parse_socks5_connect_reply_wrong_version() {
        let reply = vec![0x04, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        assert!(parse_socks5_connect_reply(&reply).is_err());
    }

    // ── find_subsequence helper ─────────────────────────────────────────

    #[test]
    fn find_subsequence_finds_match() {
        assert_eq!(find_subsequence(b"abc\r\n\r\ndef", b"\r\n\r\n"), Some(3));
    }

    #[test]
    fn find_subsequence_no_match() {
        assert_eq!(find_subsequence(b"abcdef", b"xyz"), None);
    }

    #[test]
    fn find_subsequence_empty_haystack() {
        assert_eq!(find_subsequence(b"", b"abc"), None);
    }
}

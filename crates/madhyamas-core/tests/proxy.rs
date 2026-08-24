//! Integration tests for the public proxy API: SOCKS5 method negotiation
//! and end-to-end handshakes, pipeline body decompression, and proxy
//! config defaults.

use std::time::Duration;

use madhyamas_core::config::ProxyConfig;
use madhyamas_core::proxy::pipeline::Pipeline;
use madhyamas_core::proxy::socks::{
    handle_socks5_connection, select_method, Greeting, SocksHost, SocksReply, ATYP_IPV4,
    CMD_CONNECT, METHOD_NO_ACCEPTABLE, METHOD_NO_AUTH, METHOD_USER_PASS, SOCKS_VERSION,
};
use madhyamas_core::traffic::TrafficStore;

// ============================================================================
// SOCKS5 — method negotiation
// ============================================================================

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

// ============================================================================
// SOCKS5 — reply / host display
// ============================================================================

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
fn socks_host_as_str() {
    use std::net::Ipv4Addr;
    assert_eq!(
        SocksHost::Ipv4(Ipv4Addr::new(1, 2, 3, 4)).as_str(),
        "1.2.3.4"
    );
    assert_eq!(
        SocksHost::Domain("example.com".into()).as_str(),
        "example.com"
    );
}

// ============================================================================
// SOCKS5 — end-to-end handshakes over loopback TCP pairs
// ============================================================================

/// Build a SOCKS5 CONNECT request for a `SocketAddr`. Only IPv4 is
/// supported here (the tests bind to 127.0.0.1).
fn ipv4_connect_request(target: std::net::SocketAddr) -> Vec<u8> {
    let ip = match target {
        std::net::SocketAddr::V4(a) => *a.ip(),
        _ => panic!("test target must be IPv4"),
    };
    let octets = ip.octets();
    let port_bytes = target.port().to_be_bytes();
    let mut req = vec![SOCKS_VERSION, CMD_CONNECT, 0x00, ATYP_IPV4];
    req.extend_from_slice(&octets);
    req.extend_from_slice(&port_bytes);
    req
}

/// End-to-end handshake over a loopback TCP pair. This exercises the
/// async handler with a real socket pair (no external server needed).
#[tokio::test]
async fn handshake_no_auth_then_connect_to_local_listener() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

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
    let db = std::env::temp_dir().join(format!("madhyamas-socks-test-{}.db", uuid::Uuid::new_v4()));
    let store = TrafficStore::new(db.to_str().unwrap()).await.unwrap();

    let server_task = tokio::spawn(async move {
        let (sock, _) = proxy_listener.accept().await.unwrap();
        handle_socks5_connection(sock, &*store, &traffic_tx, false, None, None)
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();

    let (traffic_tx, _) = tokio::sync::broadcast::channel(16);
    let db = std::env::temp_dir().join(format!(
        "madhyamas-socks-auth-test-{}.db",
        uuid::Uuid::new_v4()
    ));
    let store = TrafficStore::new(db.to_str().unwrap()).await.unwrap();

    let server_task = tokio::spawn(async move {
        let (sock, _) = proxy_listener.accept().await.unwrap();
        // Expect this to error out (no acceptable method).
        let _ = handle_socks5_connection(
            sock,
            &*store,
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

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
    let store = TrafficStore::new(db.to_str().unwrap()).await.unwrap();

    let server_task = tokio::spawn(async move {
        let (sock, _) = proxy_listener.accept().await.unwrap();
        handle_socks5_connection(
            sock,
            &*store,
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();

    let (traffic_tx, _) = tokio::sync::broadcast::channel(16);
    let db = std::env::temp_dir().join(format!(
        "madhyamas-socks-badauth-test-{}.db",
        uuid::Uuid::new_v4()
    ));
    let store = TrafficStore::new(db.to_str().unwrap()).await.unwrap();

    let server_task = tokio::spawn(async move {
        let (sock, _) = proxy_listener.accept().await.unwrap();
        let _ = handle_socks5_connection(
            sock,
            &*store,
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

// ============================================================================
// Pipeline — body decompression
// ============================================================================

#[test]
fn test_decompress_body_zstd() {
    use std::collections::HashMap;

    let original = b"Hello, zstd! The quick brown fox jumps over the lazy dog.".to_vec();
    let compressed = zstd::encode_all(&original[..], 3).expect("zstd encode");

    let mut headers = HashMap::new();
    headers.insert("Content-Encoding".to_string(), "zstd".to_string());
    headers.insert("Content-Length".to_string(), compressed.len().to_string());

    let result = Pipeline::decompress_body(Some("zstd"), compressed, &mut headers);

    assert_eq!(result, Some(original.clone()));
    // Content-Encoding header should be removed after successful decompression
    assert!(!headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("content-encoding")));
    // Content-Length should be updated to the decompressed size
    let cl = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .map(|(_, v)| v.as_str())
        .expect("content-length present");
    assert_eq!(cl, original.len().to_string());
}

#[test]
fn test_decompress_body_zstd_corrupt_falls_back_to_original() {
    use std::collections::HashMap;

    let corrupt = vec![0x28, 0xb5, 0x2f, 0xfd, 0xff, 0x00, 0x01, 0x02];
    let mut headers = HashMap::new();
    headers.insert("Content-Encoding".to_string(), "zstd".to_string());

    let result = Pipeline::decompress_body(Some("zstd"), corrupt.clone(), &mut headers);

    // On decompression failure, the original (corrupt) body is returned.
    assert_eq!(result, Some(corrupt));
}

#[test]
fn test_decompress_body_gzip_no_regression() {
    use std::collections::HashMap;
    use std::io::Read;

    let original = b"Hello, gzip! Decompression still works.".to_vec();
    let mut encoder = flate2::read::GzEncoder::new(&original[..], flate2::Compression::default());
    let mut compressed = Vec::new();
    encoder.read_to_end(&mut compressed).expect("gzip encode");

    let mut headers = HashMap::new();
    headers.insert("Content-Encoding".to_string(), "gzip".to_string());

    let result = Pipeline::decompress_body(Some("gzip"), compressed, &mut headers);

    assert_eq!(result, Some(original));
}

#[test]
fn test_decompress_body_no_encoding_returns_as_is() {
    use std::collections::HashMap;

    let body = b"plain body".to_vec();
    let mut headers = HashMap::new();

    let result = Pipeline::decompress_body(None, body.clone(), &mut headers);

    assert_eq!(result, Some(body));
}

// ============================================================================
// Proxy config defaults
// ============================================================================

#[test]
fn test_config_enable_h2_downstream_default_false() {
    let config = ProxyConfig::default();
    assert!(!config.enable_h2_downstream);
}

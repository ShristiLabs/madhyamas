//! Integration tests for the public proxy API: SOCKS5 method negotiation
//! and end-to-end handshakes, pipeline body decompression, attribution
//! (`client_addr` stamping on captured entries), and proxy config defaults.

use std::time::Duration;

use madhyamas_core::config::ProxyConfig;
use madhyamas_core::proxy::attribution::{AttributionContext, ListenerKind};
use madhyamas_core::proxy::pipeline::Pipeline;
use madhyamas_core::proxy::socks::{
    handle_socks5_connection, select_method, Greeting, SocksHost, SocksReply, ATYP_IPV4,
    CMD_CONNECT, METHOD_NO_ACCEPTABLE, METHOD_NO_AUTH, METHOD_USER_PASS, SOCKS_VERSION,
};
use madhyamas_core::traffic::{HttpMethod, RequestData, TrafficStore};
use madhyamas_core::ProxyPrincipal;

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
    // Keep a handle to the store so the captured entry can be inspected
    // after the handshake (issue #103: client_addr attribution).
    let assert_store = store.clone();

    let server_task = tokio::spawn(async move {
        let (sock, peer) = proxy_listener.accept().await.unwrap();
        let attribution = AttributionContext::new(ListenerKind::Socks, Some(peer));
        handle_socks5_connection(sock, &*store, &traffic_tx, false, None, None, attribution)
            .await
            .unwrap();
    });

    // Client side: connect to the SOCKS listener and perform the handshake.
    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    let client_local = client.local_addr().unwrap();
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

    // Issue #103: the tunnel's traffic entry must carry the client's
    // address, taken from the connection's attribution context.
    let session_id = assert_store.current_session_id();
    let entries = assert_store
        .get_traffic_by_session(&session_id)
        .await
        .expect("list captured entries");
    assert_eq!(entries.len(), 1, "one SOCKS tunnel entry expected");
    assert!(entries[0].is_passthrough);
    assert_eq!(
        entries[0].client_addr,
        Some(client_local.to_string()),
        "captured entry must be attributed to the connecting client"
    );

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
        let (sock, peer) = proxy_listener.accept().await.unwrap();
        // Expect this to error out (no acceptable method).
        let _ = handle_socks5_connection(
            sock,
            &*store,
            &traffic_tx,
            true, // require auth
            Some("user"),
            Some("pass"),
            AttributionContext::new(ListenerKind::Socks, Some(peer)),
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
        let (sock, peer) = proxy_listener.accept().await.unwrap();
        handle_socks5_connection(
            sock,
            &*store,
            &traffic_tx,
            true,
            Some("alice"),
            Some("secret"),
            AttributionContext::new(ListenerKind::Socks, Some(peer)),
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
        let (sock, peer) = proxy_listener.accept().await.unwrap();
        let _ = handle_socks5_connection(
            sock,
            &*store,
            &traffic_tx,
            true,
            Some("alice"),
            Some("secret"),
            AttributionContext::new(ListenerKind::Socks, Some(peer)),
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
// Attribution (issue #103)
// ============================================================================

/// The OSS default: with no proxy auth validator configured, connections
/// resolve to the unauthenticated principal (no user, no API key).
#[test]
fn proxy_principal_defaults_to_unauthenticated() {
    let principal = ProxyPrincipal::unauthenticated();
    assert_eq!(principal, ProxyPrincipal::default());
    assert!(principal.user_id.is_none());
    assert!(principal.api_key_id.is_none());
    assert!(!principal.is_authenticated());
}

/// A request processed through the pipeline with an attribution context
/// must produce a stored entry carrying the context's `client_addr`
/// (issue #103: captured entries are attributed to their origin).
#[tokio::test]
async fn pipeline_stamps_client_addr_on_captured_entry() {
    use std::collections::HashMap;

    use madhyamas_test_utils::spawn_mock_server;

    // A local "upstream" so the pipeline completes a full forward cycle.
    let (upstream_url, _upstream_rx) = spawn_mock_server().await;
    let upstream_authority = upstream_url
        .strip_prefix("http://")
        .unwrap_or(&upstream_url)
        .to_string();

    let store = TrafficStore::in_memory().await.expect("in-memory store");
    let (traffic_tx, _) = tokio::sync::broadcast::channel(16);
    let client_addr: std::net::SocketAddr = "127.0.0.1:51531".parse().unwrap();

    // Build the pipeline the way the engine does for an HTTP connection,
    // attaching the attribution context built at accept time.
    let no_proxy_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("test http client");
    let pipeline = Pipeline::new(
        ProxyConfig::default(),
        no_proxy_client,
        &*store,
        &traffic_tx,
        None,
        None,
        None,
        None,
        None,
        #[cfg(feature = "grpc")]
        None,
        #[cfg(feature = "scripting")]
        None,
        #[cfg(feature = "plugins")]
        None,
        None,
        None,
        None,
    )
    .with_attribution(AttributionContext::new(
        ListenerKind::Http,
        Some(client_addr),
    ));

    let mut request = RequestData {
        method: HttpMethod::Get,
        url: format!("{upstream_url}/attribution"),
        host: upstream_authority,
        path: "/attribution".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        http_version: Some("HTTP/1.1".to_string()),
    };

    // The pipeline writes the serialized response to the client stream;
    // a duplex pipe stands in for the client socket.
    let (mut client_stream, _client_read_side) = tokio::io::duplex(64 * 1024);
    pipeline
        .process_request(&mut request, &mut client_stream)
        .await
        .expect("process request through pipeline");

    let session_id = store.current_session_id();
    let entries = store
        .get_traffic_by_session(&session_id)
        .await
        .expect("list captured entries");
    assert_eq!(entries.len(), 1, "exactly one entry should be captured");
    assert_eq!(
        entries[0].client_addr,
        Some("127.0.0.1:51531".to_string()),
        "captured entry must carry the attribution context's client_addr"
    );
    assert!(
        entries[0].response.is_some(),
        "mock upstream should have produced a response"
    );
}

/// Without an attribution context the pipeline behaves exactly as before
/// issue #103: entries are stored with no client address.
#[tokio::test]
async fn pipeline_without_attribution_stores_no_client_addr() {
    use std::collections::HashMap;

    use madhyamas_test_utils::spawn_mock_server;

    let (upstream_url, _upstream_rx) = spawn_mock_server().await;
    let upstream_authority = upstream_url
        .strip_prefix("http://")
        .unwrap_or(&upstream_url)
        .to_string();

    let store = TrafficStore::in_memory().await.expect("in-memory store");
    let (traffic_tx, _) = tokio::sync::broadcast::channel(16);

    let no_proxy_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("test http client");
    // No with_attribution call: the pipeline keeps its default
    // (unknown-origin) context.
    let pipeline = Pipeline::new(
        ProxyConfig::default(),
        no_proxy_client,
        &*store,
        &traffic_tx,
        None,
        None,
        None,
        None,
        None,
        #[cfg(feature = "grpc")]
        None,
        #[cfg(feature = "scripting")]
        None,
        #[cfg(feature = "plugins")]
        None,
        None,
        None,
        None,
    );

    let mut request = RequestData {
        method: HttpMethod::Get,
        url: format!("{upstream_url}/no-attribution"),
        host: upstream_authority,
        path: "/no-attribution".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        http_version: Some("HTTP/1.1".to_string()),
    };

    let (mut client_stream, _client_read_side) = tokio::io::duplex(64 * 1024);
    pipeline
        .process_request(&mut request, &mut client_stream)
        .await
        .expect("process request through pipeline");

    let session_id = store.current_session_id();
    let entries = store
        .get_traffic_by_session(&session_id)
        .await
        .expect("list captured entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].client_addr, None,
        "entries without attribution context store no client address"
    );
}

// ============================================================================
// Issue #104 — device principals and the proxy-auth policy flag
// ============================================================================

/// The OSS default and the derived default keep the device slot empty;
/// only device credentials (enterprise tier) populate it.
#[test]
fn proxy_principal_device_slot_defaults_to_none() {
    let principal = ProxyPrincipal::unauthenticated();
    assert_eq!(principal, ProxyPrincipal::default());
    assert!(principal.device_id.is_none());
    assert!(!principal.is_authenticated());
}

/// A principal carrying only a device identity counts as authenticated
/// (device keys are real principals, not half-authenticated ones).
#[test]
fn proxy_principal_device_only_is_authenticated() {
    let principal = ProxyPrincipal {
        user_id: None,
        api_key_id: Some("dk-1".to_string()),
        device_id: Some("dev-1".to_string()),
    };
    assert!(principal.is_authenticated());
    assert!(principal.user_id.is_none());
}

/// The engine distinguishes why proxy auth failed: `Missing` (no
/// credential headers — 407 only in strict mode) vs `Invalid` (rejected
/// credential — always 407).
#[test]
fn proxy_auth_error_distinguishes_missing_from_invalid() {
    use madhyamas_core::ProxyAuthError;

    assert_eq!(ProxyAuthError::Missing, ProxyAuthError::Missing);
    let invalid = ProxyAuthError::Invalid("Device key revoked".to_string());
    assert_ne!(ProxyAuthError::Missing, invalid);
    assert_ne!(invalid, ProxyAuthError::Invalid("other".to_string()));
    // Clone round-trips (the engine moves the message into the 407 body).
    let cloned = invalid.clone();
    assert_eq!(cloned, invalid);
}

/// The strict-mode flag defaults to `true` (the Phase 9.6 `--proxy-auth`
/// semantics of attaching a validator) and can be relaxed to `false`
/// (issue #104: unauthenticated traffic passes unattributed).
#[tokio::test]
async fn engine_proxy_auth_required_defaults_true_and_is_settable() {
    use madhyamas_core::{CertificateManager, ProxyEngine};

    let cert_dir = tempfile::TempDir::new().expect("temp dir for certs");
    let cert_path = cert_dir.path().join("ca.pem");

    let store = TrafficStore::in_memory().await.expect("in-memory store");
    let cert_manager = CertificateManager::new(cert_path.to_str().expect("utf8 path"))
        .await
        .expect("certificate manager");
    let engine = ProxyEngine::new(
        std::sync::Arc::new(parking_lot::RwLock::new(ProxyConfig::default())),
        cert_manager,
        store as std::sync::Arc<dyn madhyamas_core::TrafficStoreBackend + Send + Sync>,
    )
    .await
    .expect("engine");

    assert!(
        engine.proxy_auth_required(),
        "strict mode must default to true (Phase 9.6 semantics)"
    );
    engine.set_proxy_auth_required(false);
    assert!(
        !engine.proxy_auth_required(),
        "strict mode must be relaxable for the #104 default policy"
    );
    engine.set_proxy_auth_required(true);
    assert!(engine.proxy_auth_required());
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

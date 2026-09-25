//! Per-device traffic visibility through a REAL proxy engine (issue #105).
//!
//! These are the acceptance tests for the credential-onboarding milestone's
//! step 5: two clients authenticating with different device credentials
//! through one proxy instance produce two independent, correctly filtered
//! traffic views. The engine runs its real accept loop on an ephemeral
//! port; a test-local [`ProxyAuthValidator`] resolves the two device keys
//! to device principals exactly as the enterprise `AuthManager` does, so
//! the attribution → per-device-session → filter pipeline is exercised
//! end-to-end without BSL-licensed code.
//!
//! Covered entry-construction points: the plain-HTTP proxy path (intercept
//! pipeline), the CONNECT/TLS-failure path, and (via the store tests in
//! `traffic.rs`) persistence and filter scoping.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use madhyamas_core::{
    AttributionContext, CertificateManager, ListenerKind, ProxyAuthValidator, ProxyConfig,
    ProxyCredentials, ProxyEngine, ProxyPrincipal, TrafficEntry, TrafficFilter, TrafficStore,
    TrafficStoreBackend,
};
use madhyamas_test_utils::spawn_mock_server;
use parking_lot::RwLock;

/// Test stand-in for the enterprise `AuthManager`: maps two device keys
/// to device principals (id + record name), mirroring
/// `AuthManager::device_principal`.
struct TwoDeviceValidator {
    alpha_key: String,
    beta_key: String,
}

#[async_trait]
impl ProxyAuthValidator for TwoDeviceValidator {
    async fn validate(&self, credentials: &ProxyCredentials) -> Result<ProxyPrincipal, String> {
        let key = match credentials {
            ProxyCredentials::ProxyBasicAuth(creds) => {
                let (_user, password) = creds.split_once(':').unwrap_or((creds, ""));
                password
            }
            ProxyCredentials::ProxyBearer(token) => token,
            ProxyCredentials::ApiKey(key) => key,
        };
        match key {
            k if *k == self.alpha_key => Ok(ProxyPrincipal {
                user_id: None,
                api_key_id: Some("dk-alpha".to_string()),
                device_id: Some("dev-alpha".to_string()),
                device_name: Some("Alpha Phone".to_string()),
            }),
            k if *k == self.beta_key => Ok(ProxyPrincipal {
                user_id: None,
                api_key_id: Some("dk-beta".to_string()),
                device_id: Some("dev-beta".to_string()),
                device_name: Some("Beta Tablet".to_string()),
            }),
            _ => Err("unknown device key".to_string()),
        }
    }
}

/// Pick a free TCP port by binding to :0 and immediately releasing it.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind :0 for port discovery")
        .local_addr()
        .expect("local addr")
        .port()
}

/// Build a proxied reqwest client that authenticates to the proxy with a
/// device key in the Basic password field (the manual-apply flow from the
/// Devices panel).
fn device_client(proxy_url: &str, key: &str) -> reqwest::Client {
    let proxy = reqwest::Proxy::http(proxy_url)
        .expect("proxy url")
        .basic_auth("device", key);
    reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("proxied client")
}

/// A plain proxied client with no credentials (the unattributed scope).
fn unauthenticated_client(proxy_url: &str) -> reqwest::Client {
    let proxy = reqwest::Proxy::http(proxy_url).expect("proxy url");
    reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("plain proxied client")
}

/// Send a raw CONNECT with Basic proxy credentials and then garbage bytes
/// so the TLS handshake fails — exercising the engine's TLS-failure entry
/// path for a device-attributed connection. Returns the bytes the proxy
/// wrote back (the 200 response).
///
/// The garbage is a truncated ClientHello: the engine's acceptor keeps
/// waiting for the rest of the record, so the handshake only FAILS (and
/// records the 502 entry) once this side closes the socket. Reading to
/// EOF would therefore deadlock both sides — instead read the 200 with a
/// timeout, then shut the socket down to EOF the engine's read.
async fn raw_connect_with_device_key(
    proxy_addr: &str,
    host: &str,
    port: u16,
    key: &str,
) -> Vec<u8> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let basic = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(format!("device:{key}"))
    };
    let mut sock = tokio::net::TcpStream::connect(proxy_addr)
        .await
        .expect("connect to proxy");
    let request = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\n\
         Host: {host}:{port}\r\n\
         Proxy-Authorization: Basic {basic}\r\n\r\n"
    );
    sock.write_all(request.as_bytes())
        .await
        .expect("send CONNECT");
    // Truncated "TLS" bytes: incomplete, so the engine's handshake hangs
    // pending until the socket below closes — then it fails and the 502
    // entry is recorded, attributed to the device.
    sock.write_all(&[0xDE, 0xAD, 0xBE, 0xEF])
        .await
        .expect("junk");

    // Read the "200 Connection Established" response (bounded — the engine
    // never closes the socket itself while the handshake is pending).
    let mut out = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        let mut buf = [0u8; 128];
        loop {
            let n = sock.read(&mut buf).await.expect("read proxy response");
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
            if out.ends_with(b"\r\n\r\n") {
                break;
            }
        }
    })
    .await;
    // EOF the engine's handshake read: the acceptor fails NOW, the 502
    // entry is stored (device-attributed) while the test polls the store.
    let _ = sock.shutdown().await;
    out
}

/// Wait until the proxy listener accepts connections (engine.start() binds
/// asynchronously in its spawned task).
async fn wait_for_listener(addr: &str) {
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("proxy listener at {addr} never came up");
}

/// Poll until all three scopes have their expected entries (capture is
/// async with respect to the client's response).
async fn wait_for_entries(store: &TrafficStore) {
    for _ in 0..200 {
        let global = store
            .get_traffic(&TrafficFilter::default())
            .await
            .expect("poll global")
            .len();
        let alpha = store
            .get_traffic(&TrafficFilter {
                device_id: Some("dev-alpha".to_string()),
                ..Default::default()
            })
            .await
            .expect("poll alpha")
            .len();
        let beta = store
            .get_traffic(&TrafficFilter {
                device_id: Some("dev-beta".to_string()),
                ..Default::default()
            })
            .await
            .expect("poll beta")
            .len();
        if global >= 1 && alpha >= 1 && beta >= 1 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("expected captured entries (1 global + 1 alpha + 1 beta) never appeared");
}

/// The definition-of-done scenario: two devices capturing through one
/// instance produce two independent, correctly filtered traffic views.
#[tokio::test]
async fn two_devices_produce_independent_filtered_traffic_views() {
    let (upstream_url, _upstream_rx) = spawn_mock_server().await;
    let upstream_authority = upstream_url
        .strip_prefix("http://")
        .unwrap_or(&upstream_url)
        .to_string();

    let store = TrafficStore::in_memory().await.expect("in-memory store");

    // Subscribe BEFORE any traffic so broadcast events (WS snapshots) can
    // be asserted for device attribution.
    let mut events = store.subscribe();

    let cert_dir = tempfile::TempDir::new().expect("temp dir for certs");
    let cert_manager =
        CertificateManager::new(cert_dir.path().join("ca.pem").to_str().expect("utf8 path"))
            .await
            .expect("certificate manager");

    let port = free_port();
    let proxy_addr = format!("127.0.0.1:{port}");
    let config = ProxyConfig {
        host: "127.0.0.1".to_string(),
        proxy_port: port,
        ..Default::default()
    };

    let engine = ProxyEngine::new(
        Arc::new(RwLock::new(config)),
        cert_manager,
        store.clone() as Arc<dyn TrafficStoreBackend + Send + Sync>,
    )
    .await
    .expect("engine");
    // #104 default enterprise policy: supplied-but-invalid keys 407,
    // missing credentials pass unattributed (so the third client below
    // works without credentials).
    engine.set_proxy_auth_required(false);
    let engine = engine.with_proxy_auth_validator(Arc::new(TwoDeviceValidator {
        alpha_key: "mdy_dev_alpha".to_string(),
        beta_key: "mdy_dev_beta".to_string(),
    }));
    tokio::spawn(async move {
        let _ = engine.start().await;
    });
    wait_for_listener(&proxy_addr).await;

    // Two clients, different device credentials, different upstream paths.
    let alpha = device_client(&format!("http://{proxy_addr}"), "mdy_dev_alpha");
    let beta = device_client(&format!("http://{proxy_addr}"), "mdy_dev_beta");
    let plain = unauthenticated_client(&format!("http://{proxy_addr}"));

    alpha
        .get(format!("{upstream_url}/from-alpha"))
        .send()
        .await
        .expect("alpha request via proxy")
        .error_for_status()
        .expect("alpha 200");
    beta.get(format!("{upstream_url}/from-beta"))
        .send()
        .await
        .expect("beta request via proxy")
        .error_for_status()
        .expect("beta 200");
    plain
        .get(format!("{upstream_url}/unattributed"))
        .send()
        .await
        .expect("unauthenticated request via proxy")
        .error_for_status()
        .expect("unauthenticated 200");

    wait_for_entries(&store).await;

    // ── Filtered views: each device sees only its own entries ──────────
    let alpha_rows = store
        .get_traffic(&TrafficFilter {
            device_id: Some("dev-alpha".to_string()),
            ..Default::default()
        })
        .await
        .expect("alpha view");
    assert_eq!(alpha_rows.len(), 1, "alpha view: {alpha_rows:?}");
    assert_eq!(alpha_rows[0].device_id.as_deref(), Some("dev-alpha"));
    assert_eq!(alpha_rows[0].request.path, "/from-alpha");
    assert_eq!(
        alpha_rows[0].session_id, "device-dev-alpha",
        "device entries land in the per-device session"
    );

    let beta_rows = store
        .get_traffic(&TrafficFilter {
            device_id: Some("dev-beta".to_string()),
            ..Default::default()
        })
        .await
        .expect("beta view");
    assert_eq!(beta_rows.len(), 1);
    assert_eq!(beta_rows[0].device_id.as_deref(), Some("dev-beta"));
    assert_eq!(beta_rows[0].request.path, "/from-beta");
    assert_eq!(beta_rows[0].session_id, "device-dev-beta");

    // ── Unfiltered view: global session only (OSS scope unchanged) ─────
    let global_rows = store
        .get_traffic(&TrafficFilter::default())
        .await
        .expect("global view");
    assert_eq!(global_rows.len(), 1);
    assert_eq!(global_rows[0].device_id, None);
    assert_eq!(global_rows[0].request.path, "/unattributed");

    // ── Sessions: auto-created, named after the device records ─────────
    let sessions = store.list_sessions().await.expect("sessions");
    let alpha_session = sessions
        .iter()
        .find(|s| s.id == "device-dev-alpha")
        .expect("alpha session auto-created");
    assert_eq!(alpha_session.name.as_deref(), Some("Device: Alpha Phone"));
    let beta_session = sessions
        .iter()
        .find(|s| s.id == "device-dev-beta")
        .expect("beta session auto-created");
    assert_eq!(beta_session.name.as_deref(), Some("Device: Beta Tablet"));

    // ── WS snapshots carry device_id (drives live status + scoping) ─────
    let mut saw_alpha = false;
    let mut saw_beta = false;
    while let Ok(event) = events.try_recv() {
        if let madhyamas_core::TrafficEvent::Added(snapshot) = event {
            match snapshot.device_id.as_deref() {
                Some("dev-alpha") => saw_alpha = true,
                Some("dev-beta") => saw_beta = true,
                _ => {}
            }
        }
    }
    assert!(saw_alpha, "WS Added snapshot carried dev-alpha");
    assert!(saw_beta, "WS Added snapshot carried dev-beta");

    let _ = upstream_authority; // authority only needed for context above
}

/// A device-attributed CONNECT whose TLS handshake fails records its 502
/// entry in the device's session (the engine's TLS-failure construction
/// point), keeping failed attempts visible in the per-device view.
#[tokio::test]
async fn device_attributed_connect_tls_failure_lands_in_device_session() {
    // rustls 0.23+ needs a process-level CryptoProvider; the binary
    // installs one at startup, tests must do the same (idempotent if
    // another test already installed it).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let store = TrafficStore::in_memory().await.expect("in-memory store");
    let cert_dir = tempfile::TempDir::new().expect("temp dir for certs");
    let cert_manager =
        CertificateManager::new(cert_dir.path().join("ca.pem").to_str().expect("utf8 path"))
            .await
            .expect("certificate manager");

    let port = free_port();
    let proxy_addr = format!("127.0.0.1:{port}");
    let config = ProxyConfig {
        host: "127.0.0.1".to_string(),
        proxy_port: port,
        ..Default::default()
    };

    let engine = ProxyEngine::new(
        Arc::new(RwLock::new(config)),
        cert_manager,
        store.clone() as Arc<dyn TrafficStoreBackend + Send + Sync>,
    )
    .await
    .expect("engine");
    let engine = engine.with_proxy_auth_validator(Arc::new(TwoDeviceValidator {
        alpha_key: "mdy_dev_alpha".to_string(),
        beta_key: "mdy_dev_beta".to_string(),
    }));
    tokio::spawn(async move {
        let _ = engine.start().await;
    });
    wait_for_listener(&proxy_addr).await;

    let response =
        raw_connect_with_device_key(&proxy_addr, "pinned.example", 443, "mdy_dev_alpha").await;
    assert!(
        String::from_utf8_lossy(&response).starts_with("HTTP/1.1 200"),
        "CONNECT must be established before the handshake: {}",
        String::from_utf8_lossy(&response)
    );

    // The TLS-failure 502 entry is attributed to the device.
    let mut attributed = false;
    for _ in 0..100 {
        let rows = store
            .get_traffic(&TrafficFilter {
                device_id: Some("dev-alpha".to_string()),
                ..Default::default()
            })
            .await
            .expect("alpha view");
        if let Some(entry) = rows.first() {
            assert_eq!(entry.request.host, "pinned.example");
            assert_eq!(entry.session_id, "device-dev-alpha");
            assert_eq!(entry.device_id.as_deref(), Some("dev-alpha"));
            let resp = entry.response.as_ref().expect("502 response recorded");
            assert_eq!(resp.status_code, 502);
            attributed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(attributed, "TLS-failure entry never appeared in alpha view");

    // The global view does not contain it.
    let global = store
        .get_traffic(&TrafficFilter::default())
        .await
        .expect("global view");
    assert!(
        global.is_empty(),
        "device-attributed failure must not leak into the global session"
    );
}

/// Attribution context plumbing: the device name is display metadata for
/// session naming and never becomes an identity — filters key on the id.
#[test]
fn attribution_context_carries_device_name_alongside_id() {
    let mut ctx = AttributionContext::new(ListenerKind::Http, None);
    assert!(ctx.device_name.is_none());
    ctx.device_id = Some("dev-alpha".to_string());
    ctx.device_name = Some("Alpha Phone".to_string());
    assert_eq!(ctx.device_id.as_deref(), Some("dev-alpha"));
    assert_eq!(ctx.device_name.as_deref(), Some("Alpha Phone"));

    // The default (OSS) context keeps both device slots empty.
    let default_ctx = AttributionContext::default();
    assert!(default_ctx.device_id.is_none() && default_ctx.device_name.is_none());
}

/// Entries constructed without attribution are unattributed on the new
/// field, and pre-#105 JSON (no device_id key) still deserializes thanks
/// to `#[serde(default)]`.
#[test]
fn traffic_entry_device_id_defaults_and_back_compatibility() {
    let request = madhyamas_core::RequestData {
        method: madhyamas_core::HttpMethod::Get,
        url: "https://example.com/".to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        http_version: None,
    };
    let entry = TrafficEntry::new("default-session", request);
    assert_eq!(entry.device_id, None);

    // A serialized entry from before issue #105 parses with device_id=None.
    let legacy_json = serde_json::json!({
        "id": entry.id,
        "session_id": "default-session",
        "request": {
            "method": "GET",
            "url": "https://example.com/",
            "host": "example.com",
            "path": "/",
            "headers": {},
            "body": null,
        },
        "response": null,
        "timestamp": "2026-01-01T00:00:00Z",
        "modified": false,
        "notes": null,
        "request_size": 0,
        "response_size": null,
        "is_passthrough": false,
        "script_intercepted": false,
        "client_addr": null,
    });
    let parsed: TrafficEntry = serde_json::from_value(legacy_json).expect("legacy JSON parses");
    assert_eq!(parsed.device_id, None);
}

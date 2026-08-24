//! Public-API integration tests for MCP server tier detection, migrated
//! from the inline module in src/server.rs.
//!
//! The auth-header tests remain inline in src/server.rs because they reach
//! into `McpServer`'s private `http_client`/`api_url` fields.

use madhyamas_mcp::{McpAuth, McpConfig, McpServer, McpTransport};
use madhyamas_test_utils::spawn_mock_server;

/// Collect all request texts received by the mock server, waiting
/// briefly for requests to arrive.
async fn collect_requests(rx: &mut tokio::sync::mpsc::Receiver<String>) -> Vec<String> {
    let mut requests = Vec::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(500);
    loop {
        let Ok(msg) = tokio::time::timeout_at(deadline, rx.recv()).await else {
            break;
        };
        if let Some(req) = msg {
            requests.push(req);
        } else {
            break;
        }
    }
    requests
}

#[tokio::test]
async fn test_tier_detection_defaults_to_community() {
    let (url, mut rx) = spawn_mock_server().await;
    let config = McpConfig {
        api_url: url,
        timeout_secs: 5,
        auth: McpAuth::None,
        transport: McpTransport::Stdio,
    };
    let server = McpServer::new(config).unwrap();
    assert_eq!(server.tier(), "community");
    std::mem::forget(server);
    let _ = collect_requests(&mut rx).await;
}

#[tokio::test]
async fn test_tier_detection_unreachable_server() {
    // Bind to a port then immediately drop the listener so nothing is
    // listening — the tier detection should gracefully default to
    // "community".
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let url = format!("http://127.0.0.1:{}", port);
    let config = McpConfig {
        api_url: url,
        timeout_secs: 2,
        auth: McpAuth::None,
        transport: McpTransport::Stdio,
    };
    let server = McpServer::new(config).unwrap();
    assert_eq!(server.tier(), "community");
    std::mem::forget(server);
}

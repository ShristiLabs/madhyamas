//! Integration tests for the public CLI command surface: `CliAuth` header
//! construction and `ApiClient` default-header behavior against a mock API
//! server.

use madhyamas_cli::{ApiClient, CliAuth};
use madhyamas_test_utils::spawn_mock_server;

#[test]
fn test_cli_auth_none_headers() {
    assert!(CliAuth::None.auth_headers().is_empty());
    assert!(matches!(CliAuth::None, CliAuth::None));
}

#[test]
fn test_cli_auth_api_key_headers() {
    let auth = CliAuth::ApiKey("cli-key-xyz".to_string());
    let headers = auth.auth_headers();
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].0, "X-API-Key");
    assert_eq!(headers[0].1, "cli-key-xyz");
    assert!(!matches!(auth, CliAuth::None));
}

#[test]
fn test_cli_auth_jwt_headers() {
    let auth = CliAuth::Jwt("cli-jwt-abc".to_string());
    let headers = auth.auth_headers();
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].0, "Authorization");
    assert_eq!(headers[0].1, "Bearer cli-jwt-abc");
    assert!(!matches!(auth, CliAuth::None));
}

#[test]
fn test_cli_auth_default_none() {
    let auth = CliAuth::default();
    assert!(matches!(auth, CliAuth::None));
    assert!(auth.auth_headers().is_empty());
}

#[tokio::test]
async fn test_cli_client_sends_api_key_header() {
    let (url, mut rx) = spawn_mock_server().await;
    let client = ApiClient::new(url, CliAuth::ApiKey("cli-key-xyz".to_string()));
    let _ = client.get("traffic").await;
    let request = rx.recv().await.unwrap();
    let lower = request.to_ascii_lowercase();
    assert!(
        lower.contains("x-api-key: cli-key-xyz"),
        "request missing X-API-Key header: {}",
        request
    );
}

#[tokio::test]
async fn test_cli_client_sends_jwt_header() {
    let (url, mut rx) = spawn_mock_server().await;
    let client = ApiClient::new(url, CliAuth::Jwt("cli-jwt-abc".to_string()));
    let _ = client.get("traffic").await;
    let request = rx.recv().await.unwrap();
    let lower = request.to_ascii_lowercase();
    assert!(
        lower.contains("authorization: bearer cli-jwt-abc"),
        "request missing Authorization header: {}",
        request
    );
}

#[tokio::test]
async fn test_cli_client_without_auth_sends_no_auth_headers() {
    let (url, mut rx) = spawn_mock_server().await;
    let client = ApiClient::new(url, CliAuth::None);
    let _ = client.get("traffic").await;
    let request = rx.recv().await.unwrap();
    let lower = request.to_ascii_lowercase();
    assert!(
        !lower.contains("x-api-key"),
        "unexpected X-API-Key header: {}",
        request
    );
    assert!(
        !lower.contains("authorization:"),
        "unexpected Authorization header: {}",
        request
    );
}

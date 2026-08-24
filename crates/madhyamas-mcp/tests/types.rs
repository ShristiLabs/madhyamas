//! Public-API integration tests for MCP configuration types, migrated
//! from the inline module in src/types.rs.

use madhyamas_mcp::{McpAuth, McpConfig, McpTransport};

#[test]
fn test_mcp_auth_none_headers() {
    let config = McpConfig {
        api_url: "http://localhost".to_string(),
        timeout_secs: 5,
        auth: McpAuth::None,
        transport: McpTransport::Stdio,
    };
    assert!(config.auth_headers().is_empty());
    assert!(matches!(McpAuth::None, McpAuth::None));
}

#[test]
fn test_mcp_auth_api_key_headers() {
    let config = McpConfig {
        api_url: "http://localhost".to_string(),
        timeout_secs: 5,
        auth: McpAuth::ApiKey("secret-key-123".to_string()),
        transport: McpTransport::Stdio,
    };
    let headers = config.auth_headers();
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].0, "X-API-Key");
    assert_eq!(headers[0].1, "secret-key-123");
    assert!(!matches!(McpAuth::ApiKey("x".to_string()), McpAuth::None));
}

#[test]
fn test_mcp_auth_jwt_headers() {
    let config = McpConfig {
        api_url: "http://localhost".to_string(),
        timeout_secs: 5,
        auth: McpAuth::Jwt("jwt-token-456".to_string()),
        transport: McpTransport::Stdio,
    };
    let headers = config.auth_headers();
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].0, "Authorization");
    assert_eq!(headers[0].1, "Bearer jwt-token-456");
    assert!(!matches!(McpAuth::Jwt("x".to_string()), McpAuth::None));
}

#[test]
fn test_mcp_config_default_auth_none() {
    let config = McpConfig::default();
    assert!(config.auth_headers().is_empty());
    assert!(matches!(config.auth, McpAuth::None));
    assert!(matches!(config.transport, McpTransport::Stdio));
}

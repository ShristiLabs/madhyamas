//! Integration tests for the public WebSocket API: upgrade detection,
//! accept-key calculation, and the manager's connection/message recording.

use std::collections::HashMap;

use madhyamas_core::websocket::{
    calculate_accept_key, is_websocket_upgrade, WsDirection, WsManager, WsMessageType, WsPayload,
};

#[test]
fn test_websocket_detection() {
    let mut headers = HashMap::new();
    headers.insert("Upgrade".to_string(), "websocket".to_string());
    headers.insert("Connection".to_string(), "Upgrade".to_string());

    assert!(is_websocket_upgrade(&headers));
}

#[test]
fn test_accept_key() {
    // RFC 6455 example
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let accept = calculate_accept_key(key);
    assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
}

#[test]
fn test_ws_manager() {
    let manager = WsManager::new();
    manager.set_session("test-session");

    let conn_id =
        manager.create_connection("ws://example.com/ws", "example.com", "/ws", HashMap::new());
    assert!(!conn_id.is_empty());

    let msg_id = manager.record_message(
        &conn_id,
        WsDirection::Send,
        WsMessageType::Text,
        WsPayload::text("Hello".to_string()),
    );
    assert!(!msg_id.is_empty());

    assert_eq!(manager.message_count(), 1);
    assert_eq!(manager.connection_count(), 1);
}

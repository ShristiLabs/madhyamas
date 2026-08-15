//! WebSocket traffic capture and inspection
//!
//! This module handles WebSocket connection interception, message capture,
//! and real-time monitoring of WebSocket frames.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// WebSocket message direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WsDirection {
    /// Client to server
    Send,
    /// Server to client
    Receive,
}

/// WebSocket message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WsMessageType {
    /// Text message
    Text,
    /// Binary message
    Binary,
    /// Ping frame
    Ping,
    /// Pong frame
    Pong,
    /// Close frame
    Close,
    /// Continuation frame
    Continuation,
}

/// WebSocket message payload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsPayload {
    /// Raw bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<Vec<u8>>,
    /// Text content (if text message)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// JSON parsed content (if applicable)
    #[serde(skip_serializing)]
    pub json: Option<serde_json::Value>,
}

impl WsPayload {
    /// Create a text payload
    pub fn text(content: String) -> Self {
        Self {
            raw: Some(content.as_bytes().to_vec()),
            text: Some(content),
            json: None,
        }
    }

    /// Create a binary payload
    pub fn binary(data: Vec<u8>) -> Self {
        // Try to decode as UTF-8
        let text = String::from_utf8(data.clone()).ok();
        Self {
            raw: Some(data),
            text,
            json: None,
        }
    }

    /// Get the size in bytes
    pub fn size(&self) -> usize {
        self.raw.as_ref().map(|r| r.len()).unwrap_or(0)
    }

    /// Try to parse as JSON
    pub fn parse_json(&mut self) {
        if let Some(ref text) = self.text {
            if let Ok(json) = serde_json::from_str(text) {
                self.json = Some(json);
            }
        }
    }
}

/// A single WebSocket message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// Unique identifier
    pub id: String,
    /// Connection ID this message belongs to
    pub connection_id: String,
    /// Message direction
    pub direction: WsDirection,
    /// Message type
    pub message_type: WsMessageType,
    /// Message payload
    pub payload: WsPayload,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Opcode (WebSocket opcode)
    pub opcode: u8,
    /// Whether this is a final frame
    pub is_final: bool,
    /// Masking key (if client message)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<u32>,
}

impl WsMessage {
    /// Create a new WebSocket message
    pub fn new(
        connection_id: &str,
        direction: WsDirection,
        message_type: WsMessageType,
        payload: WsPayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            connection_id: connection_id.to_string(),
            direction,
            message_type,
            payload,
            timestamp: Utc::now(),
            opcode: match message_type {
                WsMessageType::Continuation => 0x0,
                WsMessageType::Text => 0x1,
                WsMessageType::Binary => 0x2,
                WsMessageType::Close => 0x8,
                WsMessageType::Ping => 0x9,
                WsMessageType::Pong => 0xA,
            },
            is_final: true,
            mask: None,
        }
    }

    /// Get display text for the message
    pub fn display_text(&self) -> String {
        if let Some(ref text) = self.payload.text {
            // Truncate long messages
            if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text.clone()
            }
        } else if let Some(ref raw) = self.payload.raw {
            format!("[{} bytes binary]", raw.len())
        } else {
            "[empty]".to_string()
        }
    }
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WsConnectionState {
    /// Connection is connecting
    Connecting,
    /// Connection is open
    Open,
    /// Connection is closing
    Closing,
    /// Connection is closed
    Closed,
}

/// WebSocket connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConnection {
    /// Unique identifier
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// WebSocket URL (ws:// or wss://)
    pub url: String,
    /// Host
    pub host: String,
    /// Path
    pub path: String,
    /// Connection state
    pub state: WsConnectionState,
    /// Request headers (from upgrade request)
    pub request_headers: HashMap<String, String>,
    /// Response headers (from upgrade response)
    pub response_headers: HashMap<String, String>,
    /// Subprotocol negotiated (if any)
    pub subprotocol: Option<String>,
    /// When the connection was established
    pub created_at: DateTime<Utc>,
    /// When the connection was closed (if closed)
    pub closed_at: Option<DateTime<Utc>>,
    /// Number of messages sent
    pub messages_sent: usize,
    /// Number of messages received
    pub messages_received: usize,
    /// Total bytes sent
    pub bytes_sent: usize,
    /// Total bytes received
    pub bytes_received: usize,
}

impl WsConnection {
    /// Create a new WebSocket connection
    pub fn new(session_id: &str, url: &str, host: &str, path: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            url: url.to_string(),
            host: host.to_string(),
            path: path.to_string(),
            state: WsConnectionState::Connecting,
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            subprotocol: None,
            created_at: Utc::now(),
            closed_at: None,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Mark connection as open
    pub fn set_open(
        &mut self,
        response_headers: HashMap<String, String>,
        subprotocol: Option<String>,
    ) {
        self.state = WsConnectionState::Open;
        self.response_headers = response_headers;
        self.subprotocol = subprotocol;
    }

    /// Mark connection as closed
    pub fn set_closed(&mut self) {
        self.state = WsConnectionState::Closed;
        self.closed_at = Some(Utc::now());
    }

    /// Record a message
    pub fn record_message(&mut self, direction: WsDirection, size: usize) {
        match direction {
            WsDirection::Send => {
                self.messages_sent += 1;
                self.bytes_sent += size;
            }
            WsDirection::Receive => {
                self.messages_received += 1;
                self.bytes_received += size;
            }
        }
    }
}

/// Filter for WebSocket messages
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WsFilter {
    /// Filter by connection ID
    pub connection_id: Option<String>,
    /// Filter by direction
    pub direction: Option<WsDirection>,
    /// Filter by message type
    pub message_type: Option<WsMessageType>,
    /// Search text in payload
    pub search: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// WebSocket upgrade detection helper
pub fn is_websocket_upgrade(headers: &HashMap<String, String>) -> bool {
    headers
        .iter()
        .any(|(k, v)| k.to_lowercase() == "upgrade" && v.to_lowercase() == "websocket")
        && headers
            .iter()
            .any(|(k, v)| k.to_lowercase() == "connection" && v.to_lowercase().contains("upgrade"))
}

/// WebSocket key from upgrade request
pub fn get_websocket_key(headers: &HashMap<String, String>) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "sec-websocket-key")
        .map(|(_, v)| v.clone())
}

/// Calculate Sec-WebSocket-Accept header value
pub fn calculate_accept_key(key: &str) -> String {
    use sha1::{Digest, Sha1};

    const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());

    let result = hasher.finalize();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result)
}

/// WebSocket Manager - tracks connections and messages
pub struct WsManager {
    connections: parking_lot::RwLock<HashMap<String, WsConnection>>,
    messages: parking_lot::RwLock<Vec<WsMessage>>,
    session_id: parking_lot::RwLock<String>,
    max_messages: usize,
}

impl WsManager {
    /// Create a new WebSocket manager
    pub fn new() -> Self {
        Self {
            connections: parking_lot::RwLock::new(HashMap::new()),
            messages: parking_lot::RwLock::new(Vec::new()),
            session_id: parking_lot::RwLock::new(String::new()),
            max_messages: 10000,
        }
    }

    /// Create with custom max messages
    pub fn with_max_messages(max_messages: usize) -> Self {
        Self {
            connections: parking_lot::RwLock::new(HashMap::new()),
            messages: parking_lot::RwLock::new(Vec::new()),
            session_id: parking_lot::RwLock::new(String::new()),
            max_messages,
        }
    }

    /// Set the current session ID
    pub fn set_session(&self, session_id: &str) {
        *self.session_id.write() = session_id.to_string();
    }

    /// Create a new connection
    pub fn create_connection(
        &self,
        url: &str,
        host: &str,
        path: &str,
        request_headers: HashMap<String, String>,
    ) -> String {
        let session_id = self.session_id.read().clone();
        let mut conn = WsConnection::new(&session_id, url, host, path);
        conn.request_headers = request_headers;

        let id = conn.id.clone();
        self.connections.write().insert(id.clone(), conn);
        id
    }

    /// Complete connection handshake
    pub fn complete_connection(
        &self,
        id: &str,
        response_headers: HashMap<String, String>,
        subprotocol: Option<String>,
    ) {
        if let Some(conn) = self.connections.write().get_mut(id) {
            conn.set_open(response_headers, subprotocol);
        }
    }

    /// Close a connection
    pub fn close_connection(&self, id: &str) {
        if let Some(conn) = self.connections.write().get_mut(id) {
            conn.set_closed();
        }
    }

    /// Close all tracked WebSocket connections (graceful shutdown). Marks
    /// every connection as closed so in-flight proxy WS tunnels are torn down
    /// promptly rather than waiting for TCP timeouts.
    pub fn close_all_connections(&self) {
        let mut conns = self.connections.write();
        let count = conns.len();
        for conn in conns.values_mut() {
            conn.set_closed();
        }
        if count > 0 {
            tracing::info!("Closed {count} WebSocket connection(s)");
        }
    }

    /// Record a message
    pub fn record_message(
        &self,
        connection_id: &str,
        direction: WsDirection,
        message_type: WsMessageType,
        payload: WsPayload,
    ) -> String {
        // Update connection stats
        let size = payload.size();
        if let Some(conn) = self.connections.write().get_mut(connection_id) {
            conn.record_message(direction, size);
        }

        // Create and store message
        let mut msg = WsMessage::new(connection_id, direction, message_type, payload);
        msg.payload.parse_json();

        let id = msg.id.clone();
        let mut messages = self.messages.write();

        // Enforce max messages limit
        if messages.len() >= self.max_messages {
            messages.remove(0);
        }

        messages.push(msg);
        id
    }

    /// Get all connections
    pub fn get_connections(&self) -> Vec<WsConnection> {
        self.connections.read().values().cloned().collect()
    }

    /// Get a specific connection
    pub fn get_connection(&self, id: &str) -> Option<WsConnection> {
        self.connections.read().get(id).cloned()
    }

    /// Get messages with optional filter
    pub fn get_messages(&self, filter: &WsFilter) -> Vec<WsMessage> {
        let messages = self.messages.read();

        let mut result: Vec<WsMessage> = messages
            .iter()
            .filter(|msg| {
                if let Some(ref conn_id) = filter.connection_id {
                    if &msg.connection_id != conn_id {
                        return false;
                    }
                }
                if let Some(ref dir) = filter.direction {
                    if &msg.direction != dir {
                        return false;
                    }
                }
                if let Some(ref msg_type) = filter.message_type {
                    if &msg.message_type != msg_type {
                        return false;
                    }
                }
                if let Some(ref search) = filter.search {
                    let search_lower = search.to_lowercase();
                    let matched = if let Some(ref text) = msg.payload.text {
                        text.to_lowercase().contains(&search_lower)
                    } else if let Some(ref raw) = msg.payload.raw {
                        // Search binary payloads: try as UTF-8 string first,
                        // then fall back to raw byte search
                        if let Ok(text) = std::str::from_utf8(raw) {
                            text.to_lowercase().contains(&search_lower)
                        } else {
                            // Raw byte search for non-UTF-8 binary data
                            raw.windows(search.len()).any(|w| w == search.as_bytes())
                        }
                    } else {
                        false
                    };
                    if !matched {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Apply pagination
        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit).collect();
        }

        result
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.read().len()
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.connections.read().len()
    }

    /// Clear all messages
    pub fn clear_messages(&self) {
        self.messages.write().clear();
    }

    /// Clear closed connections
    pub fn clear_closed_connections(&self) {
        self.connections
            .write()
            .retain(|_, conn| conn.state != WsConnectionState::Closed);
    }

    /// Export messages as JSON
    pub fn export_messages(&self) -> serde_json::Value {
        let messages = self.messages.read();
        serde_json::json!({
            "connections": self.connections.read().values().collect::<Vec<_>>(),
            "messages": messages.iter().collect::<Vec<_>>()
        })
    }
}

impl Default for WsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::persistence::Persistable for WsManager {
    async fn save(&self) -> crate::Result<()> {
        Ok(())
    }

    async fn load(&self) -> crate::Result<()> {
        Ok(())
    }

    async fn clear(&self) -> crate::Result<()> {
        self.clear_messages();
        Ok(())
    }

    fn size(&self) -> usize {
        self.message_count()
    }
}

/// WebSocket frame parser
pub struct WsFrameParser;

impl WsFrameParser {
    /// Parse a WebSocket frame header
    pub fn parse_header(data: &[u8]) -> Option<(bool, u8, u64, usize)> {
        if data.len() < 2 {
            return None;
        }

        let first_byte = data[0];
        let second_byte = data[1];

        let fin = (first_byte & 0x80) != 0;
        let opcode = first_byte & 0x0F;
        let masked = (second_byte & 0x80) != 0;

        let (payload_len, header_len) = match second_byte & 0x7F {
            126 => {
                if data.len() < 4 {
                    return None;
                }
                let len = (((data[2] as u16) << 8) | data[3] as u16) as u64;
                (len, 4)
            }
            127 => {
                if data.len() < 10 {
                    return None;
                }
                let len = u64::from_be_bytes([
                    data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
                ]);
                (len, 10)
            }
            len => (len as u64, 2),
        };

        let mask_len = if masked { 4 } else { 0 };
        let total_header_len = header_len + mask_len;

        Some((fin, opcode, payload_len, total_header_len))
    }

    /// Decode a masked payload
    pub fn decode_masked(data: &[u8], mask: [u8; 4]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ mask[i % 4])
            .collect()
    }

    /// Get message type from opcode
    pub fn message_type_from_opcode(opcode: u8) -> WsMessageType {
        match opcode {
            0x0 => WsMessageType::Continuation,
            0x1 => WsMessageType::Text,
            0x2 => WsMessageType::Binary,
            0x8 => WsMessageType::Close,
            0x9 => WsMessageType::Ping,
            0xA => WsMessageType::Pong,
            _ => WsMessageType::Binary,
        }
    }
}

/// WebSocket opcode constants.
const OPCODE_CONTINUATION: u8 = 0x0;
const OPCODE_TEXT: u8 = 0x1;
const OPCODE_BINARY: u8 = 0x2;
/// First control-frame opcode. Opcodes >= 0x8 are control frames and are
/// never fragmented (they may be interleaved with continuation frames).
const OPCODE_FIRST_CONTROL: u8 = 0x8;

/// WebSocket fragmentation reassembly state machine.
///
/// Per RFC 6455, a single WebSocket message may be split across multiple
/// frames:
///
/// - First frame: opcode = text (`0x1`) or binary (`0x2`), `FIN=0`
/// - Continuation frames: opcode = `0x0` (continuation), `FIN=0`
/// - Final frame: opcode = `0x0` (continuation), `FIN=1`
///
/// Control frames (close/ping/pong, opcodes `0x8`-`0xA`) may be interleaved
/// between continuation frames and must be delivered immediately without
/// disrupting the in-progress reassembly.
///
/// The reassembler is a standalone struct intended to be owned per
/// connection (one for each direction). It does not modify or depend on
/// [`WsFrameParser`]; callers feed it already-decoded frame payloads.
#[derive(Debug, Clone)]
pub struct WsFragmentReassembler {
    /// Opcode of the first frame in the in-progress fragmented message
    /// (text or binary). `None` when no message is being reassembled.
    opcode: Option<u8>,
    /// Accumulated payload bytes across all frames of the in-progress
    /// message so far.
    buffer: Vec<u8>,
}

impl WsFragmentReassembler {
    /// Create a new, empty reassembler.
    pub fn new() -> Self {
        Self {
            opcode: None,
            buffer: Vec::new(),
        }
    }

    /// Whether we are currently in the middle of a fragmented message
    /// (i.e. waiting for continuation frames).
    pub fn is_fragmenting(&self) -> bool {
        self.opcode.is_some()
    }

    /// Add a frame to the reassembler.
    ///
    /// Returns:
    /// - `Some(WsPayload)` when a complete message has been produced. This
    ///   happens either for a single unfragmented frame (`FIN=1` on a
    ///   text/binary opcode) or when the final continuation frame (`FIN=1`)
    ///   of a fragmented message arrives.
    /// - `None` when more continuation frames are needed.
    ///
    /// # Control frames
    ///
    /// Control frames (opcodes `0x8`-`0xF`: close/ping/pong) are never
    /// fragmented and may be interleaved with continuation frames. They are
    /// returned immediately and do **not** affect any in-progress
    /// reassembly.
    ///
    /// # Protocol errors
    ///
    /// If a non-continuation data opcode (text/binary) is received while
    /// already fragmenting, this is a protocol error: the in-progress
    /// message is discarded and a new message is started from the incoming
    /// frame. If a continuation frame arrives with no in-progress message,
    /// it is treated as a standalone (best-effort) message to avoid losing
    /// data.
    pub fn add_frame(&mut self, opcode: u8, fin: bool, payload: Vec<u8>) -> Option<WsPayload> {
        // Control frames (opcodes >= 0x8) are never fragmented and may be
        // interleaved with continuation frames. Return them immediately
        // without touching the reassembly state.
        if opcode >= OPCODE_FIRST_CONTROL {
            return Some(Self::payload_from_opcode(opcode, payload));
        }

        match opcode {
            OPCODE_CONTINUATION => {
                if self.opcode.is_none() {
                    // Protocol error: continuation without a start frame.
                    // Treat as a standalone message to avoid losing data.
                    return Some(Self::payload_from_opcode(OPCODE_CONTINUATION, payload));
                }
                self.buffer.extend(payload);
                if fin {
                    let complete = std::mem::take(&mut self.buffer);
                    let start_opcode = self.opcode.take().unwrap_or(OPCODE_BINARY);
                    Some(Self::payload_from_opcode(start_opcode, complete))
                } else {
                    None
                }
            }
            OPCODE_TEXT | OPCODE_BINARY => {
                // Start of a new data message.
                if self.opcode.is_some() {
                    // Protocol error: new data opcode while still fragmenting.
                    // Reset and start a new message from this frame.
                    self.buffer.clear();
                    self.opcode = None;
                }
                if fin {
                    // Unfragmented single-frame message.
                    Some(Self::payload_from_opcode(opcode, payload))
                } else {
                    self.opcode = Some(opcode);
                    self.buffer = payload;
                    None
                }
            }
            _ => {
                // Unknown/reserved opcode — return as-is without disturbing
                // any in-progress reassembly.
                Some(Self::payload_from_opcode(opcode, payload))
            }
        }
    }

    /// Reset the reassembler, discarding any in-progress message.
    pub fn reset(&mut self) {
        self.opcode = None;
        self.buffer.clear();
    }

    /// Build a [`WsPayload`] from a completed opcode + accumulated bytes.
    ///
    /// For text opcodes the bytes are decoded as UTF-8 (falling back to a
    /// raw binary payload if decoding fails). For all other opcodes the
    /// payload is treated as binary.
    fn payload_from_opcode(opcode: u8, data: Vec<u8>) -> WsPayload {
        match opcode {
            OPCODE_TEXT => {
                let text = String::from_utf8(data.clone()).ok();
                WsPayload {
                    raw: Some(data),
                    text,
                    json: None,
                }
            }
            _ => WsPayload::binary(data),
        }
    }
}

impl Default for WsFragmentReassembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_fragment_reassembler_single_frame() {
        // A single unfragmented text frame (FIN=1) should return immediately.
        let mut reassembler = WsFragmentReassembler::new();
        assert!(!reassembler.is_fragmenting());

        let result = reassembler.add_frame(OPCODE_TEXT, true, b"Hello".to_vec());
        let payload = result.expect("single FIN=1 frame should produce a payload");
        assert_eq!(payload.text.as_deref(), Some("Hello"));
        assert_eq!(payload.raw.as_deref(), Some(b"Hello".as_slice()));
        assert!(!reassembler.is_fragmenting());
    }

    #[test]
    fn test_fragment_reassembler_two_frame_text() {
        // First frame: text, FIN=0 -> None, now fragmenting.
        let mut reassembler = WsFragmentReassembler::new();
        assert_eq!(
            reassembler.add_frame(OPCODE_TEXT, false, b"Hel".to_vec()),
            None
        );
        assert!(reassembler.is_fragmenting());

        // Final continuation frame: FIN=1 -> complete payload.
        let payload = reassembler
            .add_frame(OPCODE_CONTINUATION, true, b"lo".to_vec())
            .expect("final continuation should produce a payload");
        assert_eq!(payload.text.as_deref(), Some("Hello"));
        assert_eq!(payload.raw.as_deref(), Some(b"Hello".as_slice()));
        assert!(!reassembler.is_fragmenting());
    }

    #[test]
    fn test_fragment_reassembler_three_frame_binary() {
        let mut reassembler = WsFragmentReassembler::new();

        // First frame: binary, FIN=0. 0xFF/0xFE are invalid UTF-8 lead bytes
        // so the reassembled payload should not be decoded as text.
        assert_eq!(
            reassembler.add_frame(OPCODE_BINARY, false, vec![0xFF, 0xFE]),
            None
        );
        assert!(reassembler.is_fragmenting());

        // Middle continuation frame: FIN=0.
        assert_eq!(
            reassembler.add_frame(OPCODE_CONTINUATION, false, vec![0xFD, 0x00]),
            None
        );
        assert!(reassembler.is_fragmenting());

        // Final continuation frame: FIN=1 -> complete binary payload.
        let payload = reassembler
            .add_frame(OPCODE_CONTINUATION, true, vec![0x01])
            .expect("final continuation should produce a payload");
        assert_eq!(
            payload.raw.as_deref(),
            Some(&[0xFF, 0xFE, 0xFD, 0x00, 0x01][..])
        );
        // The reassembled bytes are not valid UTF-8, so text should be None.
        assert!(payload.text.is_none());
        assert!(!reassembler.is_fragmenting());
    }

    #[test]
    fn test_fragment_reassembler_interleaved_control_frame() {
        let mut reassembler = WsFragmentReassembler::new();

        // Start a fragmented text message.
        assert_eq!(
            reassembler.add_frame(OPCODE_TEXT, false, b"Hel".to_vec()),
            None
        );
        assert!(reassembler.is_fragmenting());

        // A ping control frame arrives in the middle — it should be returned
        // immediately without disrupting the in-progress reassembly.
        let ping = reassembler
            .add_frame(0x9, true, b"ping-data".to_vec())
            .expect("control frame should be returned immediately");
        assert_eq!(ping.raw.as_deref(), Some(b"ping-data".as_slice()));
        // Reassembly should still be in progress.
        assert!(reassembler.is_fragmenting());

        // Now the final continuation frame arrives and completes the message.
        let payload = reassembler
            .add_frame(OPCODE_CONTINUATION, true, b"lo".to_vec())
            .expect("final continuation should produce a payload");
        assert_eq!(payload.text.as_deref(), Some("Hello"));
        assert!(!reassembler.is_fragmenting());
    }

    #[test]
    fn test_fragment_reassembler_empty_first_frame() {
        // An empty first frame is valid per RFC 6455.
        let mut reassembler = WsFragmentReassembler::new();
        assert_eq!(reassembler.add_frame(OPCODE_TEXT, false, Vec::new()), None);
        assert!(reassembler.is_fragmenting());

        let payload = reassembler
            .add_frame(OPCODE_CONTINUATION, true, b"Hi".to_vec())
            .expect("final continuation should produce a payload");
        assert_eq!(payload.text.as_deref(), Some("Hi"));
        assert_eq!(payload.raw.as_deref(), Some(b"Hi".as_slice()));
        assert!(!reassembler.is_fragmenting());
    }

    #[test]
    fn test_fragment_reassembler_protocol_error_new_data_opcode() {
        // Receiving a new data opcode while fragmenting is a protocol error:
        // the in-progress message is discarded and a new one starts.
        let mut reassembler = WsFragmentReassembler::new();
        assert_eq!(
            reassembler.add_frame(OPCODE_BINARY, false, vec![0x01, 0x02]),
            None
        );
        assert!(reassembler.is_fragmenting());

        // New text opcode while still fragmenting — resets and starts fresh.
        // Since FIN=0, no payload is produced yet.
        assert_eq!(
            reassembler.add_frame(OPCODE_TEXT, false, b"new".to_vec()),
            None
        );
        assert!(reassembler.is_fragmenting());

        // The previous binary bytes (0x01, 0x02) must have been discarded.
        let payload = reassembler
            .add_frame(OPCODE_CONTINUATION, true, b" msg".to_vec())
            .expect("final continuation should produce a payload");
        assert_eq!(payload.text.as_deref(), Some("new msg"));
    }

    #[test]
    fn test_fragment_reassembler_reset() {
        let mut reassembler = WsFragmentReassembler::new();
        assert_eq!(
            reassembler.add_frame(OPCODE_TEXT, false, b"partial".to_vec()),
            None
        );
        assert!(reassembler.is_fragmenting());

        reassembler.reset();
        assert!(!reassembler.is_fragmenting());

        // After reset, a continuation frame is treated as standalone.
        let payload = reassembler
            .add_frame(OPCODE_CONTINUATION, true, b"orphan".to_vec())
            .expect("orphan continuation after reset should produce a payload");
        assert_eq!(payload.raw.as_deref(), Some(b"orphan".as_slice()));
    }
}

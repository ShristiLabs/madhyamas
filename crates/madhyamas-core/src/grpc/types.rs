//! gRPC types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// gRPC message direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GrpcDirection {
    Request,
    Response,
}

/// gRPC message type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GrpcMessageType {
    /// Unary request/response
    Unary,
    /// Server streaming
    ServerStream,
    /// Client streaming
    ClientStream,
    /// Bidirectional streaming
    BidiStream,
}

/// gRPC compression algorithm
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GrpcCompression {
    #[default]
    None,
    Gzip,
    Deflate,
    Snappy,
}

/// gRPC message frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcFrame {
    /// Unique identifier
    pub id: String,
    /// Stream ID
    pub stream_id: String,
    /// Connection ID
    pub connection_id: String,
    /// Direction (request/response)
    pub direction: GrpcDirection,
    /// Whether this is compressed
    pub compressed: bool,
    /// Message data (raw bytes, base64 encoded for JSON)
    pub data: String,
    /// Decoded protobuf message (if available)
    pub decoded: Option<ProtoMessage>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Trailers (metadata at end of stream)
    pub is_trailer: bool,
}

impl GrpcFrame {
    pub fn new(
        stream_id: &str,
        connection_id: &str,
        direction: GrpcDirection,
        data: Vec<u8>,
        compressed: bool,
    ) -> Self {
        use base64::Engine;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            stream_id: stream_id.to_string(),
            connection_id: connection_id.to_string(),
            direction,
            compressed,
            data: base64::engine::general_purpose::STANDARD.encode(&data),
            decoded: None,
            timestamp: Utc::now(),
            is_trailer: false,
        }
    }

    /// Get raw data bytes
    pub fn data_bytes(&self) -> Option<Vec<u8>> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(&self.data)
            .ok()
    }
}

/// Decoded protobuf message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtoMessage {
    /// Message type name (e.g., "google.protobuf.StringValue")
    pub message_type: Option<String>,
    /// Decoded fields
    pub fields: Vec<ProtoField>,
    /// Raw JSON representation (if parsed successfully)
    pub json: Option<String>,
}

/// A single protobuf field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtoField {
    /// Field number
    pub number: u32,
    /// Wire type
    pub wire_type: u8,
    /// Field name (if known from descriptor)
    pub name: Option<String>,
    /// Field value
    pub value: ProtoValue,
}

/// Protobuf field value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProtoValue {
    Varint(u64),
    Fixed64(u64),
    Fixed32(u32),
    Bytes(String), // Base64
    String(String),
    LengthDelimited(String), // Base64
    Group(Vec<ProtoField>),
    Nested(Box<ProtoMessage>),
}

/// gRPC stream state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcStream {
    /// Stream ID
    pub id: String,
    /// Connection ID
    pub connection_id: String,
    /// Service name (e.g., "grpc.example.ExampleService")
    pub service: Option<String>,
    /// Method name (e.g., "GetUser")
    pub method: Option<String>,
    /// Full path (e.g., "/grpc.example.ExampleService/GetUser")
    pub path: Option<String>,
    /// Message type
    pub message_type: GrpcMessageType,
    /// Compression used
    pub compression: GrpcCompression,
    /// Request metadata (headers)
    pub request_metadata: HashMap<String, String>,
    /// Response metadata (headers)
    pub response_metadata: HashMap<String, String>,
    /// Response trailers
    pub response_trailers: HashMap<String, String>,
    /// Number of frames sent
    pub frames_sent: u64,
    /// Number of frames received
    pub frames_received: u64,
    /// Stream state
    pub state: GrpcStreamState,
    /// When the stream was created
    pub created_at: DateTime<Utc>,
    /// When the stream was closed
    pub closed_at: Option<DateTime<Utc>>,
    /// gRPC status code (if stream is closed)
    pub status_code: Option<i32>,
    /// Status message
    pub status_message: Option<String>,
}

impl GrpcStream {
    pub fn new(connection_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            connection_id: connection_id.to_string(),
            service: None,
            method: None,
            path: None,
            message_type: GrpcMessageType::Unary,
            compression: GrpcCompression::None,
            request_metadata: HashMap::new(),
            response_metadata: HashMap::new(),
            response_trailers: HashMap::new(),
            frames_sent: 0,
            frames_received: 0,
            state: GrpcStreamState::Open,
            created_at: Utc::now(),
            closed_at: None,
            status_code: None,
            status_message: None,
        }
    }

    /// Detect the gRPC message type from the final frame counts.
    ///
    /// This should be called once the stream is closed and all frame counts
    /// are final. The detection is based on the number of request frames
    /// (`frames_sent`) and response frames (`frames_received`):
    ///
    /// - **Unary**: exactly one request frame and at most one response frame.
    /// - **ServerStream**: one request frame and more than one response frame.
    /// - **ClientStream**: more than one request frame and at most one
    ///   response frame.
    /// - **BidiStream**: more than one request frame and more than one
    ///   response frame.
    ///
    /// HTTP/2 header hints (such as `content-type` starting with
    /// `application/grpc`) are used only to confirm that the stream is gRPC;
    /// frame counting is the primary detection mechanism.
    pub fn detect_message_type(&self) -> GrpcMessageType {
        // Header hint: if the request metadata does not look like gRPC, we
        // cannot reliably classify the stream and fall back to Unary.
        if let Some(content_type) = self.request_metadata.get("content-type") {
            if !content_type.starts_with("application/grpc") {
                return GrpcMessageType::Unary;
            }
        }

        let multi_request = self.frames_sent > 1;
        let multi_response = self.frames_received > 1;

        match (multi_request, multi_response) {
            (false, false) => GrpcMessageType::Unary,
            (false, true) => GrpcMessageType::ServerStream,
            (true, false) => GrpcMessageType::ClientStream,
            (true, true) => GrpcMessageType::BidiStream,
        }
    }
}

/// gRPC stream state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GrpcStreamState {
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

/// gRPC connection tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConnection {
    /// Connection ID
    pub id: String,
    /// Client address
    pub client_addr: String,
    /// Server address
    pub server_addr: String,
    /// Active streams
    pub active_streams: u64,
    /// Total streams
    pub total_streams: u64,
    /// When the connection was established
    pub created_at: DateTime<Utc>,
    /// Connection state
    pub state: GrpcConnectionState,
}

impl GrpcConnection {
    pub fn new(client_addr: &str, server_addr: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            client_addr: client_addr.to_string(),
            server_addr: server_addr.to_string(),
            active_streams: 0,
            total_streams: 0,
            created_at: Utc::now(),
            state: GrpcConnectionState::Active,
        }
    }
}

/// gRPC connection state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GrpcConnectionState {
    Active,
    Closing,
    Closed,
}

/// gRPC filter for searching
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrpcFilter {
    /// Filter by service name
    pub service: Option<String>,
    /// Filter by method name
    pub method: Option<String>,
    /// Filter by path
    pub path_pattern: Option<String>,
    /// Filter by status code
    pub status_code: Option<i32>,
    /// Filter by direction
    pub direction: Option<GrpcDirection>,
    /// Search in message content
    pub search: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

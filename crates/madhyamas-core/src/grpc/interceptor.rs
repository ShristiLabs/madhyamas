//! gRPC traffic interceptor and manager

use super::{GrpcConnection, GrpcDirection, GrpcFilter, GrpcFrame, GrpcStream};
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Manages gRPC traffic tracking and interception
pub struct GrpcManager {
    /// Active connections
    connections: RwLock<HashMap<String, GrpcConnection>>,
    /// Active streams
    streams: RwLock<HashMap<String, GrpcStream>>,
    /// Captured frames
    frames: RwLock<Vec<GrpcFrame>>,
    /// Index: stream_id → frame indices in `frames` for O(1) lookup
    frame_index: RwLock<HashMap<String, Vec<usize>>>,
    /// Maximum frames to keep
    max_frames: usize,
    /// Broadcast channel for real-time updates
    frame_tx: broadcast::Sender<GrpcFrame>,
}

impl GrpcManager {
    pub fn new(max_frames: usize) -> Self {
        let (frame_tx, _) = broadcast::channel(1024);
        Self {
            connections: RwLock::new(HashMap::new()),
            streams: RwLock::new(HashMap::new()),
            frames: RwLock::new(Vec::new()),
            frame_index: RwLock::new(HashMap::new()),
            max_frames,
            frame_tx,
        }
    }

    /// Register a new gRPC connection
    pub fn register_connection(&self, client_addr: &str, server_addr: &str) -> String {
        let conn = GrpcConnection::new(client_addr, server_addr);
        let id = conn.id.clone();
        self.connections.write().insert(id.clone(), conn);
        id
    }

    /// Close a connection
    pub fn close_connection(&self, id: &str) {
        if let Some(conn) = self.connections.write().get_mut(id) {
            conn.state = super::GrpcConnectionState::Closed;
        }
    }

    /// Register a new gRPC stream
    pub fn register_stream(&self, connection_id: &str, path: Option<&str>) -> String {
        let mut stream = GrpcStream::new(connection_id);

        // Parse service and method from path
        if let Some(p) = path {
            stream.path = Some(p.to_string());
            let parts: Vec<&str> = p.trim_start_matches('/').split('/').collect();
            if parts.len() >= 2 {
                stream.service = Some(parts[0].to_string());
                stream.method = Some(parts[1].to_string());
            }
        }

        let id = stream.id.clone();

        // Update connection
        if let Some(conn) = self.connections.write().get_mut(connection_id) {
            conn.active_streams += 1;
            conn.total_streams += 1;
        }

        self.streams.write().insert(id.clone(), stream);
        id
    }

    /// Update stream metadata from HTTP/2 headers
    pub fn update_stream_metadata(
        &self,
        stream_id: &str,
        direction: GrpcDirection,
        metadata: HashMap<String, String>,
    ) {
        if let Some(stream) = self.streams.write().get_mut(stream_id) {
            match direction {
                GrpcDirection::Request => {
                    stream.request_metadata = metadata.clone();

                    // Extract path if present
                    if let Some(path) = metadata.get(":path") {
                        stream.path = Some(path.clone());
                        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
                        if parts.len() >= 2 {
                            stream.service = Some(parts[0].to_string());
                            stream.method = Some(parts[1].to_string());
                        }
                    }

                    // Check for streaming
                    if let Some(te) = metadata.get("te") {
                        if te == "trailers" {
                            // This indicates streaming support
                        }
                    }
                }
                GrpcDirection::Response => {
                    stream.response_metadata = metadata.clone();

                    // Check for trailers
                    if let Some(trailer) = metadata.get("trailer") {
                        // Will receive trailers at end
                        let _ = trailer;
                    }
                }
            }
        }
    }

    /// Record a frame
    pub fn record_frame(&self, frame: GrpcFrame) {
        // Update stream counters
        if let Some(stream) = self.streams.write().get_mut(&frame.stream_id) {
            match frame.direction {
                GrpcDirection::Request => stream.frames_sent += 1,
                GrpcDirection::Response => stream.frames_received += 1,
            }
        }

        // Store frame and update index
        {
            let mut frames = self.frames.write();
            let idx = frames.len();
            frames.push(frame.clone());

            // Update stream_id → frame index mapping
            self.frame_index
                .write()
                .entry(frame.stream_id.clone())
                .or_default()
                .push(idx);

            // Trim if over limit
            if frames.len() > self.max_frames {
                let excess = frames.len() - self.max_frames;
                frames.drain(0..excess);
                // Shift all index values down by `excess` and remove invalid ones
                let mut index = self.frame_index.write();
                for indices in index.values_mut() {
                    for i in indices.iter_mut() {
                        *i = i.saturating_sub(excess);
                    }
                    indices.retain(|&i| i < frames.len());
                }
                // Remove empty entries
                index.retain(|_, v| !v.is_empty());
            }
        }

        // Broadcast
        let _ = self.frame_tx.send(frame);
    }

    /// Close a stream
    pub fn close_stream(
        &self,
        stream_id: &str,
        status_code: Option<i32>,
        status_message: Option<String>,
    ) {
        if let Some(stream) = self.streams.write().get_mut(stream_id) {
            stream.state = super::GrpcStreamState::Closed;
            stream.closed_at = Some(Utc::now());
            stream.status_code = status_code;
            stream.status_message = status_message;

            // Auto-detect the message type now that all frame counts are final.
            stream.message_type = stream.detect_message_type();

            // Update connection
            if let Some(conn) = self.connections.write().get_mut(&stream.connection_id) {
                conn.active_streams = conn.active_streams.saturating_sub(1);
            }
        }
    }

    /// Set stream trailers
    pub fn set_stream_trailers(&self, stream_id: &str, trailers: HashMap<String, String>) {
        if let Some(stream) = self.streams.write().get_mut(stream_id) {
            stream.response_trailers = trailers;

            // Extract grpc-status and grpc-message
            if let Some(status) = stream.response_trailers.get("grpc-status") {
                stream.status_code = status.parse().ok();
            }
            if let Some(msg) = stream.response_trailers.get("grpc-message") {
                stream.status_message = Some(msg.clone());
            }
        }
    }

    /// Get all connections
    pub fn get_connections(&self) -> Vec<GrpcConnection> {
        self.connections.read().values().cloned().collect()
    }

    /// Get all streams
    pub fn get_streams(&self) -> Vec<GrpcStream> {
        self.streams.read().values().cloned().collect()
    }

    /// Get a specific stream
    pub fn get_stream(&self, id: &str) -> Option<GrpcStream> {
        self.streams.read().get(id).cloned()
    }

    /// Get frames for a stream (uses index for O(1) lookup)
    pub fn get_stream_frames(&self, stream_id: &str) -> Vec<GrpcFrame> {
        let index = self.frame_index.read();
        let frames = self.frames.read();
        match index.get(stream_id) {
            Some(indices) => indices
                .iter()
                .filter_map(|&i| frames.get(i).cloned())
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get frames with filter
    pub fn get_frames(&self, filter: &GrpcFilter) -> Vec<GrpcFrame> {
        let frames = self.frames.read();

        let mut result: Vec<GrpcFrame> = frames
            .iter()
            .filter(|f| {
                // Filter by direction
                if let Some(ref dir) = filter.direction {
                    if f.direction != *dir {
                        return false;
                    }
                }

                // Search in content
                if let Some(ref search) = filter.search {
                    if !f.data.to_lowercase().contains(&search.to_lowercase()) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Filter by service/method via stream lookup
        if filter.service.is_some() || filter.method.is_some() || filter.path_pattern.is_some() {
            let streams = self.streams.read();
            result.retain(|f| {
                if let Some(stream) = streams.get(&f.stream_id) {
                    if let Some(ref svc) = filter.service {
                        if stream
                            .service
                            .as_ref()
                            .map(|s| !s.contains(svc))
                            .unwrap_or(true)
                        {
                            return false;
                        }
                    }
                    if let Some(ref method) = filter.method {
                        if stream
                            .method
                            .as_ref()
                            .map(|m| !m.contains(method))
                            .unwrap_or(true)
                        {
                            return false;
                        }
                    }
                    if let Some(ref pattern) = filter.path_pattern {
                        if stream
                            .path
                            .as_ref()
                            .map(|p| !p.contains(pattern))
                            .unwrap_or(true)
                        {
                            return false;
                        }
                    }
                }
                true
            });
        }

        // Apply status code filter
        if filter.status_code.is_some() {
            let streams = self.streams.read();
            result.retain(|f| {
                streams
                    .get(&f.stream_id)
                    .and_then(|s| s.status_code)
                    .map(|s| s == filter.status_code.unwrap())
                    .unwrap_or(false)
            });
        }

        // Apply offset and limit
        if let Some(offset) = filter.offset {
            result = result.into_iter().skip(offset).collect();
        }
        if let Some(limit) = filter.limit {
            result = result.into_iter().take(limit).collect();
        }

        result
    }

    /// Subscribe to frame updates
    pub fn subscribe(&self) -> broadcast::Receiver<GrpcFrame> {
        self.frame_tx.subscribe()
    }

    /// Clear all captured data
    pub fn clear(&self) {
        self.frames.write().clear();
        self.frame_index.write().clear();
    }

    /// Get statistics
    pub fn stats(&self) -> GrpcStats {
        let connections = self.connections.read();
        let streams = self.streams.read();
        let frames = self.frames.read();

        GrpcStats {
            total_connections: connections.len(),
            active_connections: connections
                .values()
                .filter(|c| c.state == super::GrpcConnectionState::Active)
                .count(),
            total_streams: streams.len(),
            active_streams: streams
                .values()
                .filter(|s| s.state == super::GrpcStreamState::Open)
                .count(),
            total_frames: frames.len(),
        }
    }
}

impl Default for GrpcManager {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl crate::persistence::Persistable for GrpcManager {
    fn save(&self) -> crate::Result<()> {
        Ok(())
    }

    fn load(&self) -> crate::Result<()> {
        Ok(())
    }

    fn clear(&self) -> crate::Result<()> {
        self.clear();
        Ok(())
    }

    fn size(&self) -> usize {
        self.frames.read().len()
    }
}

/// gRPC traffic statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub total_streams: usize,
    pub active_streams: usize,
    pub total_frames: usize,
}

/// gRPC service descriptor for decoding messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcServiceDescriptor {
    pub name: String,
    pub package: String,
    pub methods: Vec<GrpcMethodDescriptor>,
}

/// gRPC method descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcMethodDescriptor {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
}

/// Check if HTTP/2 path looks like gRPC.
///
/// gRPC paths are typically `/package.Service/Method`, but some services
/// use paths without dots (e.g. `/Service/Method`). We relax the check to
/// accept any two-segment path with a non-empty service and method, and
/// rely on the `content-type` header for definitive detection.
pub fn is_grpc_path(path: &str) -> bool {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
}

/// Check if content-type is gRPC
pub fn is_grpc_content_type(content_type: Option<&str>) -> bool {
    match content_type {
        Some(ct) => ct.starts_with("application/grpc"),
        None => false,
    }
}

/// Parse gRPC status from trailers
pub fn parse_grpc_status(trailers: &HashMap<String, String>) -> Option<(i32, Option<String>)> {
    let status = trailers.get("grpc-status")?.parse().ok()?;
    let message = trailers.get("grpc-message").cloned();
    Some((status, message))
}

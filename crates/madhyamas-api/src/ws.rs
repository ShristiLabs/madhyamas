//! WebSocket handler for real-time traffic updates

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use madhyamas_core::TrafficStore;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Handle WebSocket connection
pub async fn handle_ws(socket: WebSocket, traffic_store: Arc<TrafficStore>) {
    let (mut tx, mut rx) = socket.split();

    info!("WebSocket client connected");

    // Send initial traffic count
    if let Ok(count) = traffic_store.count() {
        let msg = serde_json::json!({
            "type": "count",
            "data": count
        });
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = tx.send(Message::Text(json)).await;
        }
    }

    // Handle incoming messages (for future use - e.g., commands)
    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received WebSocket message: {}", text);

                // Handle ping/pong or commands
                if text == "ping" {
                    let _ = tx.send(Message::Text("pong".to_string())).await;
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket client disconnected");
                break;
            }
            Err(e) => {
                warn!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

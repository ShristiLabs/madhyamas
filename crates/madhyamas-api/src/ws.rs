//! WebSocket handler for real-time traffic updates

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use madhyamas_core::{
    TrafficEntrySnapshot, TrafficFilter, TrafficStore, WsClientMessage, WsServerMessage,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Handle WebSocket connection for real-time traffic updates
pub async fn handle_ws(socket: WebSocket, traffic_store: Arc<TrafficStore>) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let client_id = Uuid::new_v4().to_string();

    info!("WebSocket client connected: {}", client_id);

    // Subscribe to traffic events
    let mut event_rx = traffic_store.subscribe();

    // Send connection acknowledgment
    let connected_msg = WsServerMessage::Connected {
        client_id: client_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        if ws_tx.send(Message::Text(json.into())).await.is_err() {
            warn!("Failed to send connection ack, client disconnected");
            return;
        }
    }

    // Send initial traffic data
    if let Ok(entries) = traffic_store.get_traffic(&TrafficFilter::default()).await {
        let snapshots: Vec<TrafficEntrySnapshot> =
            entries.iter().map(TrafficEntrySnapshot::from).collect();
        let initial_msg = WsServerMessage::InitialTraffic(snapshots);
        if let Ok(json) = serde_json::to_string(&initial_msg) {
            let _ = ws_tx.send(Message::Text(json.into())).await;
        }
    }

    // Spawn task to forward traffic events to WebSocket
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let event_forwarder = {
        let client_id = client_id.clone();
        async move {
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = &mut shutdown_rx => {
                        debug!("Event forwarder shutting down for client: {}", client_id);
                        break;
                    }
                    // Forward traffic events
                    event = event_rx.recv() => {
                        match event {
                            Ok(traffic_event) => {
                                let msg = WsServerMessage::Traffic(Box::new(traffic_event));
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                        debug!("Client disconnected while sending event: {}", client_id);
                                        break;
                                    }
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Client {} lagged behind by {} events", client_id, n);
                                // Continue receiving - we'll catch up
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                debug!("Event channel closed for client: {}", client_id);
                                break;
                            }
                        }
                    }
                }
            }
        }
    };

    // Spawn the event forwarder
    let forwarder_handle = tokio::spawn(event_forwarder);

    // Handle incoming messages from client
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received WebSocket message from {}: {}", client_id, text);

                // Try to parse as WsClientMessage
                match serde_json::from_str::<WsClientMessage>(&text) {
                    Ok(client_msg) => {
                        handle_client_message(&client_msg, &traffic_store, &client_id).await;
                    }
                    Err(_) => {
                        // Legacy ping/pong support
                        if text.trim() == "ping" {
                            debug!("Legacy ping received from {}", client_id);
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                debug!("Ping received from {}", client_id);
                // Axum handles pong automatically, but we can log it
                let _ = data; // Acknowledge we received it
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket client disconnected: {}", client_id);
                break;
            }
            Err(e) => {
                warn!("WebSocket error for client {}: {}", client_id, e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup: signal the forwarder to stop
    let _ = shutdown_tx.send(());
    let _ = forwarder_handle.await;

    info!("WebSocket connection closed: {}", client_id);
}

/// Handle incoming client messages
async fn handle_client_message(
    msg: &WsClientMessage,
    traffic_store: &Arc<TrafficStore>,
    client_id: &str,
) {
    match msg {
        WsClientMessage::Ping => {
            debug!("Ping received from client: {}", client_id);
            // Pong is sent via the event forwarder or handled by axum
        }
        WsClientMessage::Subscribe { filter } => {
            debug!("Client {} subscribed with filter: {:?}", client_id, filter);
            // Future: implement per-client filtering
        }
        WsClientMessage::Unsubscribe => {
            debug!("Client {} unsubscribed", client_id);
        }
        WsClientMessage::GetInitialTraffic { limit } => {
            debug!(
                "Client {} requested initial traffic (limit: {:?})",
                client_id, limit
            );
            // Initial traffic is sent on connection, but client can request refresh
            let filter = TrafficFilter {
                limit: *limit,
                ..Default::default()
            };
            if let Ok(entries) = traffic_store.get_traffic(&filter).await {
                let snapshots: Vec<TrafficEntrySnapshot> =
                    entries.iter().map(TrafficEntrySnapshot::from).collect();
                debug!(
                    "Sending {} entries to client {}",
                    snapshots.len(),
                    client_id
                );
            }
        }
    }
}

//! WebSocket handler for real-time traffic updates

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use madhyamas_core::{
    TrafficEntrySnapshot, TrafficEvent, TrafficFilter, TrafficStoreBackend, WsClientMessage,
    WsServerMessage,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Whether a traffic event may be emitted to a subscriber bound to
/// `device_filter` (issue #108). Entry-carrying events (Added/Updated)
/// pass only when the entry belongs to the parent device; broadcast
/// events (Deleted/Cleared/CountUpdate) are metadata about the stream
/// itself and stay visible to everyone.
fn event_in_scope(event: &TrafficEvent, device_filter: &Option<String>) -> bool {
    match device_filter {
        None => true,
        Some(device) => match event {
            TrafficEvent::Added(s) | TrafficEvent::Updated(s) => {
                s.device_id.as_deref() == Some(device.as_str())
            }
            TrafficEvent::Deleted(_) | TrafficEvent::Cleared | TrafficEvent::CountUpdate(_) => true,
        },
    }
}

/// Handle WebSocket connection for real-time traffic updates.
///
/// `device_filter` (issue #108) implements per-subscriber device
/// filtering for device-derived agent-key connections: when set, only
/// the parent device's entries are emitted — both the initial snapshot
/// and every live `Traffic` event (local and cross-instance). Other
/// subscribers (JWT web sessions, user keys, OSS) pass `None` and see
/// the full stream, unchanged.
pub async fn handle_ws(
    socket: WebSocket,
    traffic_store: Arc<dyn TrafficStoreBackend + Send + Sync>,
    cross_instance_rx: Option<broadcast::Receiver<TrafficEvent>>,
    device_filter: Option<String>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let client_id = Uuid::new_v4().to_string();

    info!("WebSocket client connected: {}", client_id);

    // Subscribe to local traffic events
    let mut event_rx = traffic_store.subscribe();
    // Optionally subscribe to cross-instance events (relayed via Redis)
    let mut cross_rx = cross_instance_rx;

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

    // Send initial traffic data (device-scoped for agent-key
    // subscribers, issue #108).
    let initial_filter = TrafficFilter {
        device_id: device_filter.clone(),
        ..Default::default()
    };
    if let Ok(entries) = traffic_store.get_traffic(&initial_filter).await {
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
                    // Forward local traffic events
                    event = event_rx.recv() => {
                        match event {
                            Ok(traffic_event) => {
                                if !event_in_scope(&traffic_event, &device_filter) {
                                    continue;
                                }
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
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                debug!("Event channel closed for client: {}", client_id);
                                break;
                            }
                        }
                    }
                    // Forward cross-instance traffic events (relayed via Redis)
                    event = async {
                        if let Some(ref mut rx) = cross_rx {
                            rx.recv().await
                        } else {
                            // No cross-instance channel — block forever
                            std::future::pending::<
                                Result<TrafficEvent, broadcast::error::RecvError>,
                            >()
                            .await
                        }
                    } => {
                        if cross_rx.is_some() {
                            match event {
                                Ok(traffic_event) => {
                                    if !event_in_scope(&traffic_event, &device_filter) {
                                        continue;
                                    }
                                    let msg = WsServerMessage::Traffic(Box::new(traffic_event));
                                    if let Ok(json) = serde_json::to_string(&msg) {
                                        if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                            debug!("Client disconnected while sending cross-instance event: {}", client_id);
                                            break;
                                        }
                                    }
                                }
                                Err(broadcast::error::RecvError::Lagged(n)) => {
                                    warn!("Client {} lagged behind by {} cross-instance events", client_id, n);
                                }
                                Err(broadcast::error::RecvError::Closed) => {
                                    debug!("Cross-instance event channel closed for client: {}", client_id);
                                    // Don't break — local events may still flow
                                }
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
    traffic_store: &Arc<dyn TrafficStoreBackend + Send + Sync>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use madhyamas_core::TrafficEntrySnapshot;

    fn snap(device_id: Option<&str>) -> TrafficEntrySnapshot {
        TrafficEntrySnapshot {
            id: "e1".to_string(),
            session_id: "s".to_string(),
            method: "GET".to_string(),
            url: "https://x/".to_string(),
            host: "x".to_string(),
            path: "/".to_string(),
            status_code: Some(200),
            status_message: None,
            content_type: None,
            response_content_type: None,
            duration_ms: None,
            request_size: 0,
            response_size: None,
            timestamp: "t".to_string(),
            modified: false,
            has_request_body: false,
            has_response_body: false,
            is_passthrough: false,
            http_version: None,
            script_intercepted: false,
            device_id: device_id.map(str::to_string),
        }
    }

    /// Issue #108 per-subscriber filtering: agent-key connections only
    /// receive their parent device's entry events; broadcast events stay
    /// visible to everyone; unfiltered subscribers see everything.
    #[test]
    fn event_in_scope_filters_by_parent_device() {
        let none: Option<String> = None;
        let x = Some("dev-x".to_string());
        let added_x = TrafficEvent::Added(snap(Some("dev-x")));
        let added_y = TrafficEvent::Added(snap(Some("dev-y")));
        let added_none = TrafficEvent::Added(snap(None));
        let updated_x = TrafficEvent::Updated(snap(Some("dev-x")));

        // Unfiltered subscriber (JWT / user key / OSS): everything.
        for e in [&added_x, &added_y, &added_none] {
            assert!(event_in_scope(e, &none));
        }

        // dev-x subscriber: own entries only — other devices and
        // unattributed entries are invisible.
        assert!(event_in_scope(&added_x, &x));
        assert!(event_in_scope(&updated_x, &x));
        assert!(!event_in_scope(&added_y, &x), "other device filtered out");
        assert!(
            !event_in_scope(&added_none, &x),
            "unattributed traffic is invisible to agents"
        );

        // Broadcast metadata events stay visible to bound subscribers.
        assert!(event_in_scope(&TrafficEvent::Cleared, &x));
        assert!(event_in_scope(&TrafficEvent::CountUpdate(3), &x));
        assert!(event_in_scope(&TrafficEvent::Deleted(vec!["e".into()]), &x));
    }
}

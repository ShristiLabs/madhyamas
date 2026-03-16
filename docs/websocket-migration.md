# WebSocket Migration for Real-time Traffic Updates

This document describes the migration from REST API polling to WebSocket-based real-time updates for the traffic view in the Madhyamas proxy tool.

## Overview

Previously, the web UI polled the REST API every 1-2 seconds to fetch traffic data. This has been replaced with a WebSocket-based approach that provides instant updates when traffic changes occur.

### Benefits

- **Instant updates**: Traffic appears immediately when captured, no polling delay
- **Reduced server load**: No repeated polling requests
- **Lower bandwidth**: Only changed data is transmitted
- **Better UX**: Live connection status indicator

### Hybrid Approach

The implementation uses a hybrid approach:
- **Primary**: WebSocket for real-time updates (default)
- **Fallback**: REST polling when WebSocket is unavailable or disabled

## Architecture

### Backend Components

#### 1. Traffic Events (`crates/madhyamas-core/src/traffic/events.rs`)

Defines the event types for WebSocket communication:

```rust
pub enum TrafficEvent {
    Added(TrafficEntrySnapshot),    // New request captured
    Updated(TrafficEntrySnapshot),  // Response received
    Deleted(Vec<String>),           // Entries deleted
    Cleared,                        // All traffic cleared
    CountUpdate(usize),             // Count changed
}
```

#### 2. TrafficStore Event Emitter

The `TrafficStore` now includes a `broadcast::Sender<TrafficEvent>` that emits events when:
- A new request is stored (`store_request`)
- A response is received (`store_response`)
- Traffic is cleared (`clear_traffic`)
- Specific entries are deleted (`delete_traffic`)

#### 3. WebSocket Handler (`crates/madhyamas-api/src/ws.rs`)

Enhanced handler that:
- Subscribes to traffic events from `TrafficStore`
- Sends initial traffic data on connection
- Forwards real-time events to connected clients
- Handles client messages (ping, subscribe, etc.)
- Supports automatic reconnection handling

### Frontend Components

#### 1. WebSocket Types (`web/src/types/websocket.ts`)

TypeScript types matching the backend Rust types:
- `TrafficEntrySnapshot`: Lightweight traffic entry for WebSocket
- `TrafficEvent`: Traffic change events
- `WsServerMessage`: Messages from server to client
- `WsClientMessage`: Messages from client to server
- `WsConnectionInfo`: Connection state information

#### 2. Generic WebSocket Hook (`web/src/hooks/useWebSocket.ts`)

Reusable WebSocket hook with:
- Automatic reconnection with exponential backoff
- Connection state management
- Message parsing and routing
- Manual connect/disconnect controls

#### 3. Traffic WebSocket Hook (`web/src/hooks/useTrafficWebSocket.ts`)

Traffic-specific hook that:
- Manages local traffic state
- Handles traffic events (add, update, delete, clear)
- Provides connection status
- Converts snapshots to full entries

#### 4. Updated useTraffic Hook (`web/src/hooks/useTraffic.ts`)

Enhanced hook with:
- WebSocket mode (default) with REST fallback
- Mode persistence to localStorage
- Client-side filtering for WebSocket data
- Seamless switching between modes

## Message Protocol

### Server → Client Messages

```typescript
type WsServerMessage =
  | { type: "Traffic"; data: TrafficEvent }
  | { type: "InitialTraffic"; data: TrafficEntrySnapshot[] }
  | { type: "Connected"; data: { client_id: string } }
  | { type: "Pong" }
  | { type: "Error"; data: { message: string } };
```

### Client → Server Messages

```typescript
type WsClientMessage =
  | { type: "Subscribe"; data?: { filter?: TrafficSubscriptionFilter } }
  | { type: "Unsubscribe" }
  | { type: "GetInitialTraffic"; data?: { limit?: number } }
  | { type: "Ping" };
```

## UI Changes

### Traffic View

- **Connection Status Indicator**: Shows "Live" (green), "Reconnecting" (yellow), or "Disconnected" (red)
- **Mode Toggle**: Button to switch between WebSocket and polling modes
- **Automatic Fallback**: Falls back to polling if WebSocket fails

### Config Dialog

- **Real-time Updates**: Toggle between WebSocket (Live) and Polling modes
- **Polling Interval**: Configure fallback polling interval (disabled when using WebSocket)

## Configuration

### Storage Keys

- `madhyamas-use-websocket`: WebSocket mode preference (default: "true")
- `madhyamas-appearance-config`: Includes `use_websocket` setting

### WebSocket URL

The WebSocket connects to `/api/ws` using the same host as the web UI:
- `ws://localhost:8080/api/ws` (HTTP)
- `wss://localhost:8080/api/ws` (HTTPS)

## Reconnection Strategy

The WebSocket hook implements exponential backoff:
- Initial delay: 1 second
- Maximum delay: 30 seconds
- Maximum attempts: 10
- Jitter: Random 0-1 second added to prevent thundering herd

## Performance Considerations

### TrafficEntrySnapshot

A lightweight version of `TrafficEntry` that excludes:
- Request/response bodies
- Full headers

This reduces bandwidth while providing enough information for the traffic list.

### Broadcast Channel

Uses `tokio::sync::broadcast` with capacity of 1024 events. If a client lags behind, it will skip missed events and continue receiving new ones.

## Testing

To test the WebSocket integration:

1. Start the proxy server
2. Open the web UI
3. Verify "Live" indicator appears (green)
4. Generate traffic through the proxy
5. Verify traffic appears instantly without page refresh
6. Test reconnection by restarting the server
7. Test fallback by disabling WebSocket in settings

## Future Enhancements

- Per-client filtering (subscribe with filter)
- Binary message format for better performance
- Compression for large traffic lists
- WebSocket authentication

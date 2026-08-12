# API — WebSocket & gRPC

Real-time traffic updates via WebSocket, WebSocket traffic inspection, and gRPC
stream inspection. Base path: `/api`.

## Real-time WebSocket (`/ws`)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/ws` | WebSocket upgrade for real-time traffic updates |

### Server-to-Client Messages (`WsServerMessage`)

| Message Type | Payload | Description |
|--------------|---------|-------------|
| `Traffic` | `Box<TrafficEvent>` | A traffic event notification (see below) |
| `InitialTraffic` | `Vec<TrafficEntrySnapshot>` | Initial traffic list sent on connection |
| `Connected` | `{ client_id: String }` | Connection established acknowledgment |
| `Pong` | - | Response to client ping (keep-alive) |
| `Error` | `{ message: String }` | Error message |

### Traffic Event Variants (`TrafficEvent`)

| Variant | Payload | Description |
|---------|---------|-------------|
| `Added` | `TrafficEntrySnapshot` | A new traffic entry was added (request captured) |
| `Updated` | `TrafficEntrySnapshot` | A traffic entry was updated (response received) |
| `Deleted` | `Vec<String>` | Specific traffic entries were deleted |
| `Cleared` | - | All traffic was cleared |
| `CountUpdate` | `usize` | Traffic count changed |

### Client-to-Server Messages (`WsClientMessage`)

| Message Type | Payload | Description |
|--------------|---------|-------------|
| `Subscribe` | `{ filter: Option<TrafficSubscriptionFilter> }` | Subscribe to traffic updates with optional filter |
| `Unsubscribe` | - | Unsubscribe from traffic updates |
| `GetInitialTraffic` | `{ limit: Option<usize> }` | Request initial traffic data |
| `Ping` | - | Keep-alive ping |

## WebSocket Traffic Inspection

Inspect WebSocket connections and messages passing through the proxy.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/ws-traffic/connections` | List WebSocket connections |
| GET | `/ws-traffic/connections/{id}` | Get a single WebSocket connection |
| GET | `/ws-traffic/messages` | List WebSocket messages |
| POST | `/ws-traffic/clear` | Clear captured WebSocket traffic |

## gRPC Inspection

gRPC traffic inspection (requires the `grpc` Cargo feature).

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/grpc/connections` | List gRPC connections |
| GET | `/grpc/streams` | List gRPC streams |
| GET | `/grpc/frames` | List gRPC frames |
| GET | `/grpc/stats` | Get gRPC statistics |
| POST | `/grpc/clear` | Clear captured gRPC frames |

## See Also

- [API.md](API.md) — API index
- [HTTP2_SUPPORT.md](HTTP2_SUPPORT.md) — HTTP/2 and gRPC support
- [WEB_FRONTEND.md](WEB_FRONTEND.md) — WebSocket client in the frontend

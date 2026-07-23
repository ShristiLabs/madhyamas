# Madhyamas API Reference

## Base URL

All endpoints are served from the Madhyamas API server (default: `http://127.0.0.1:3001/api`).

## Traffic

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/traffic` | List all traffic entries |
| GET | `/api/traffic/:id` | Get single traffic entry |
| POST | `/api/traffic/clear` | Clear all traffic |
| GET | `/api/traffic/count` | Get traffic count |

## Sessions

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/sessions` | List all sessions |
| POST | `/api/sessions` | Create new session |
| GET | `/api/sessions/:id` | Get session details |
| DELETE | `/api/sessions/:id` | Delete session |
| GET | `/api/sessions/:id/export` | Export session as HAR |
| POST | `/api/sessions/:id/switch` | Switch active session |
| POST | `/api/sessions/import` | Import session from HAR |

## Export

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/export/har` | Export traffic as HAR |
| GET | `/api/export/curl/:id` | Export request as cURL |

## Interception

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST/DELETE | `/api/breakpoints` | Manage breakpoint rules |
| GET/POST | `/api/breakpoints/paused` | List/resume paused requests |
| POST | `/api/breakpoints/paused/:id/resume` | Resume a paused request |
| GET/POST | `/api/mocks` | Manage mock rules |
| PUT/DELETE | `/api/mocks/:id` | Update/delete mock rule |
| POST | `/api/mocks/:id/toggle` | Enable/disable mock |
| GET/POST/DELETE | `/api/rewrites` | Manage rewrite rules |
| POST | `/api/rewrites/:id/toggle` | Enable/disable rewrite |
| GET/POST | `/api/throttle` | Manage throttling |
| POST | `/api/throttle/enabled` | Enable/disable throttling |
| GET | `/api/throttle/presets` | Get throttle presets |

## Replay

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/api/replay/saved` | Manage saved requests |
| POST | `/api/replay/execute/:id` | Replay a request |
| GET | `/api/replay/history` | View replay history |

## WebSocket & gRPC

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/ws` | WebSocket for real-time traffic updates |
| GET | `/api/ws-traffic/connections` | List WebSocket connections |
| GET | `/api/grpc/connections` | List gRPC connections |
| GET | `/api/grpc/streams` | List gRPC streams |
| GET | `/api/grpc/frames` | List gRPC frames |
| GET | `/api/grpc/stats` | Get gRPC statistics |

## Scripts & Plugins

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/api/scripts` | List/create scripts |
| PUT/DELETE | `/api/scripts/:id` | Update/delete script |
| POST | `/api/scripts/:id/toggle` | Enable/disable script |
| GET | `/api/plugins` | List plugins |
| POST | `/api/plugins/:id/enable` | Enable plugin |
| POST | `/api/plugins/:id/disable` | Disable plugin |
| POST | `/api/plugins/reload` | Reload plugins |

## Configuration & Capture

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/config` | Get proxy configuration |
| PATCH | `/api/config` | Update proxy configuration |
| GET | `/api/capture` | Get capture status |
| POST | `/api/capture/toggle` | Toggle traffic capture |
| GET | `/api/cert/ca` | Download CA certificate |

## Health

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check |

## Query Parameters

### Traffic Filter

```
GET /api/traffic?method=GET&url=*https://example.com*&status_code=200&content_type=application/json
```

| Parameter | Type | Description |
|-----------|------|-------------|
| method | string | HTTP method (GET, POST, etc.) |
| url | string | URL pattern (supports wildcards and regex) |
| status_code | number | HTTP status code |
| content_type | string | Response content type |

### Pagination

```
GET /api/traffic?limit=100&offset=0
```

| Parameter | Type | Description |
|-----------|------|-------------|
| limit | number | Max results to return |
| offset | number | Number of results to skip |

## WebSocket Events

The WebSocket endpoint (`/api/ws`) sends `WsServerMessage` messages to clients. Traffic updates are wrapped in `WsServerMessage::Traffic(Box<TrafficEvent>)`.

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

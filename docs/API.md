# Madhyamas API Documentation

## Base URL

| Endpoint | Method | Description |
|----------|------|-------------|
| `/api/traffic` | GET | List all captured traffic |
| `/api/traffic?filter=<filter>` | GET | Filter traffic by criteria |
| `/api/traffic/:id` | GET | Get specific traffic entry |
| `/api/traffic/clear` | POST | Clear all traffic |
| `/api/traffic/count` | GET | Get traffic count |
| `/api/sessions` | GET | List all sessions |
| `/api/sessions` | POST | Create new session |
| `/api/sessions/:id` | GET | Get session details |
| `/api/sessions/:id/export` | GET | Export session as HAR |
| `/api/sessions/:id/switch` | POST | Switch to session |
| `/api/sessions/import` | POST | Import session from HAR file |
| `/api/export/har` | GET | Export all traffic as HAR |
| `/api/export/curl/:id` | GET | Get cURL command for request |
| `/api/cert/ca` | GET | Get CA certificate PEM |
| `/api/breakpoints` | GET | List breakpoint rules |
| `/api/breakpoints` | POST | Create breakpoint rule |
| `/api/breakpoints/:id` | DELETE | Delete breakpoint rule |
| `/api/breakpoints/paused` | GET | List paused requests |
| `/api/breakpoints/paused/:id/resume` | POST | Resume paused request |
| `/api/mocks` | GET | List mock rules |
| `/api/mocks` | POST | Create mock rule |
| `/api/mocks/:id` | PUT | Update mock rule |
| `/api/mocks/:id` | DELETE | Delete mock rule |
| `/api/mocks/:id/toggle` | POST | Enable/disable mock |
| `/api/rewrites` | GET | List rewrite rules |
| `/api/rewrites` | POST | Create rewrite rule |
| `/api/rewrites/:id` | DELETE | Delete rewrite rule |
| `/api/rewrites/:id/toggle` | POST | Enable/disable rewrite |
| `/api/throttle` | GET | Get throttle profile |
| `/api/throttle` | POST | Set throttle profile |
| `/api/throttle/enabled` | POST | Enable/disable throttling |
| `/api/throttle/presets` | GET | Get throttle presets |
| `/api/replay/saved` | GET | List saved requests |
| `/api/replay/saved` | POST | Save request for replay |
| `/api/replay/execute/:id` | POST | Execute (replay) saved request |
| `/api/replay/history` | GET | Get replay history |
| `/api/grpc/connections` | GET | List gRPC connections |
| `/api/grpc/streams` | GET | List gRPC streams |
| `/api/grpc/frames` | GET | List gRPC frames |
| `/api/scripts` | GET | List scripts |
| `/api/scripts` | POST | Create script |
| `/api/scripts/:id` | PUT | Update script |
| `/api/scripts/:id` | DELETE | Delete script |
| `/api/scripts/:id/toggle` | POST | Enable/disable script |
| `/api/plugins` | GET | List plugins |
| `/api/plugins/:id/enable` | POST | Enable plugin |
| `/api/plugins/:id/disable` | POST | Disable plugin |
| `/api/ws` | GET | WebSocket connection for real-time updates |

| `/api/health` | GET | Health check |
| `/api/metrics` | GET | Performance metrics |
| `/api/onboarding` | GET | Get onboarding status |
| `/api/auth/login` | POST | User login |
| `/api/auth/logout` | POST | User logout |
| `/api/auth/me` | GET | Get current user |
| `/api/users` | GET | List users (admin) |
| `/api/users` | POST | Create user (admin) |
| `/api/audit` | GET | Get audit log entries |
| `/api/config` | GET | Get application config |

| `/api/config` | PUT | Update application config |

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


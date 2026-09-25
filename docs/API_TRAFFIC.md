# API — Traffic, Sessions, Export, Certificate

Endpoints for capturing, listing, filtering, and exporting HTTP traffic, plus
session management and the CA certificate. Base path: `/api`.

## Traffic

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/traffic` | List traffic entries (supports filtering and pagination, see [API.md](API.md)) |
| GET | `/traffic/{id}` | Get a single traffic entry |
| GET | `/traffic/{id}/script-traces` | Get script execution traces for a traffic entry |
| POST | `/traffic/clear` | Clear all captured traffic |
| GET | `/traffic/count` | Get the current traffic count |
| POST | `/traffic/import/har` | Import traffic from a HAR file |

### Traffic filtering and pagination

```
GET /api/traffic?method=GET&url=*example.com*&status_code=200&limit=100&offset=0
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `method` | string | HTTP method filter |
| `url` | string | URL pattern (wildcards and regex) |
| `status_code` | number | Status code filter |
| `content_type` | string | Response content type filter |
| `limit` | number | Max results |
| `offset` | number | Skip results |
| `device_id` | string | Filter by the device a connection was attributed to (issue #105; Enterprise device credentials). Scopes the query to that device's entries across sessions instead of the current session |

Entries carry an optional `device_id` field (null for unattributed entries
and rows captured before issue #105), and WebSocket `Added`/`Updated`
snapshots carry the same field so live views can be device-scoped.

## Sessions

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/sessions` | List all sessions (real persisted rows, most recently updated first; includes auto-created per-device sessions named after the device record) |
| POST | `/sessions` | Create a new session |
| GET | `/sessions/{id}` | Get session details |
| DELETE | `/sessions/{id}` | Delete a session |
| GET | `/sessions/{id}/export` | Export a session as HAR |
| POST | `/sessions/{id}/switch` | Switch the active session |
| POST | `/sessions/import` | Import a session from HAR |

## Export

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/export/har` | Export all traffic as a HAR file |
| GET | `/export/curl/{id}` | Export a request as a cURL command |

## Certificate

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/cert/ca` | Download the Madhyamas CA certificate (PEM) |

Install this CA into your system/browser trust store to enable HTTPS
interception. See [GETTING_STARTED.md](GETTING_STARTED.md).

## See Also

- [API.md](API.md) — API index
- [API_WEBSOCKET_GRPC.md](API_WEBSOCKET_GRPC.md) — WebSocket events for real-time traffic
- [HAR_IMPORT.md](HAR_IMPORT.md) — HAR import feature
- [RECORDING_LIMITS.md](RECORDING_LIMITS.md) — Recording size limits and FIFO pruning

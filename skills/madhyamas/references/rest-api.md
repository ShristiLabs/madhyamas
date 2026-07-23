# REST API Reference

All 130+ REST API endpoints. Base URL: `http://localhost:3001/api`. All endpoints return JSON unless otherwise noted.

## Phase 1 — Core (24 endpoints)

### Traffic

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/traffic` | List captured traffic with filters | `curl 'http://localhost:3001/api/traffic?method=POST&status=500&limit=50'` |
| GET | `/traffic/{id}` | Get single traffic entry | `curl http://localhost:3001/api/traffic/abc123` |
| POST | `/traffic/clear` | Clear all traffic | `curl -X POST http://localhost:3001/api/traffic/clear` |
| GET | `/traffic/count` | Get traffic count | `curl http://localhost:3001/api/traffic/count` |

**GET /traffic query parameters:** `method`, `url` (pattern), `status_code`, `content_type`, `limit`, `offset`

### Sessions

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/sessions` | List all sessions | `curl http://localhost:3001/api/sessions` |
| POST | `/sessions` | Create session | `curl -X POST -H 'Content-Type: application/json' -d '{"name":"debug-auth"}' http://localhost:3001/api/sessions` |
| GET | `/sessions/{id}` | Get session details | `curl http://localhost:3001/api/sessions/abc123` |
| DELETE | `/sessions/{id}` | Delete session | `curl -X DELETE http://localhost:3001/api/sessions/abc123` |
| GET | `/sessions/{id}/export` | Export session | `curl http://localhost:3001/api/sessions/abc123/export?format=har` |
| POST | `/sessions/{id}/switch` | Switch active session | `curl -X POST http://localhost:3001/api/sessions/abc123/switch` |
| POST | `/sessions/import` | Import session | `curl -X POST -H 'Content-Type: application/json' -d @session.json http://localhost:3001/api/sessions/import` |

### Export

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/export/har` | Export all traffic as HAR | `curl http://localhost:3001/api/export/har -o traffic.har` |
| GET | `/export/curl/{id}` | Export request as cURL | `curl http://localhost:3001/api/export/curl/abc123` |

### Certificate

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/cert/ca` | Download CA certificate (PEM) | `curl http://localhost:3001/api/cert/ca -o madhyamas-ca.pem` |

### Configuration

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/config` | Get proxy configuration | `curl http://localhost:3001/api/config` |
| PATCH | `/config` | Update configuration | `curl -X PATCH -H 'Content-Type: application/json' -d '{"intercept_https":false}' http://localhost:3001/api/config` |

**PATCH /config body fields:** `intercept_https` (boolean), `max_requests` (integer), `verbose` (boolean), `public_ip` (string|null), `max_body_size` (integer)

### Capture

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/capture` | Get capture status | `curl http://localhost:3001/api/capture` |
| POST | `/capture/toggle` | Toggle capture mode | `curl -X POST http://localhost:3001/api/capture/toggle` |

### WebSocket Traffic

| Method | Path | Description |
|--------|------|-------------|
| GET | `/ws-traffic/connections` | List WebSocket connections |
| GET | `/ws-traffic/connections/{id}` | Get specific WS connection |
| GET | `/ws-traffic/messages` | Get WS messages (filters: `connection_id`, `direction`, `message_type`, `search`) |
| POST | `/ws-traffic/clear` | Clear WebSocket traffic |

### Real-time Updates

| Method | Path | Description |
|--------|------|-------------|
| GET | `/ws` | WebSocket upgrade for real-time traffic updates |

**WebSocket server messages:** `Connected` (client_id), `InitialTraffic` (snapshot), `Traffic` (Added/Updated/Deleted/Cleared/CountUpdate events), `Pong`, `Error`

**WebSocket client messages:** `Ping`, `Subscribe` (with optional filter), `Unsubscribe`, `GetInitialTraffic`

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check (returns "OK") |

## Phase 2 — Interception (67 endpoints)

### Breakpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/breakpoints` | List all breakpoint rules |
| POST | `/breakpoints` | Create breakpoint rule |
| GET | `/breakpoints/{id}` | Get specific breakpoint rule |
| DELETE | `/breakpoints/{id}` | Delete breakpoint rule |
| GET | `/breakpoints/paused` | Get all paused traffic items |
| GET | `/breakpoints/paused/{id}` | Get specific paused item |
| POST | `/breakpoints/paused/{id}/resume` | Resume paused item (decision: Allow/Modify/Reject) |

**POST /breakpoints body:** `name` (string, 1-255 chars), `condition` (object), `direction` (request/response), `priority` (integer, optional)

**POST /breakpoints/paused/{id}/resume body:** `decision` — `"allow"`, `"modify"` (with modifications), or `"reject"`

### Mocks

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mocks` | List all mock rules |
| POST | `/mocks` | Create mock rule |
| GET | `/mocks/templates` | Get predefined mock templates |
| GET | `/mocks/{id}` | Get specific mock rule |
| PUT | `/mocks/{id}` | Update mock rule |
| DELETE | `/mocks/{id}` | Delete mock rule |
| POST | `/mocks/{id}/toggle` | Enable/disable mock |
| POST | `/mocks/batch-toggle` | Toggle multiple mocks |
| POST | `/mocks/advanced` | Create advanced mock (tags, collection, description) |
| POST | `/mocks/{id}/duplicate` | Duplicate a mock |
| POST | `/mocks/{id}/rollback` | Rollback mock to previous version |
| GET | `/mocks/{id}/versions` | Get mock version history |
| POST | `/mocks/{id}/test` | Test mock against sample request |
| POST | `/mocks/preview` | Preview which mock matches a request |
| GET | `/mocks/export` | Export all mocks as JSON |
| POST | `/mocks/import` | Import mocks (HAR/OpenAPI/Postman) |

**Mock Collections:**

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mocks/collections` | List all collections |
| POST | `/mocks/collections` | Create collection |
| GET | `/mocks/collections/{id}` | Get specific collection |
| DELETE | `/mocks/collections/{id}` | Delete collection (optional `delete_rules` flag) |
| POST | `/mocks/collections/{id}/toggle` | Toggle all mocks in collection |

**Mock Recording:**

| Method | Path | Description |
|--------|------|-------------|
| POST | `/mocks/recording` | Enable/disable recording mode |
| GET | `/mocks/recording/status` | Get recording status |
| GET | `/mocks/recording/recorded` | Get recorded mocks |
| POST | `/mocks/recording/promote` | Promote recorded mocks to active rules |
| POST | `/mocks/recording/clear` | Clear recorded mocks |

**Mock Analytics:**

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mocks/analytics` | Get hit analytics for all mocks |
| GET | `/mocks/{id}/analytics` | Get hit stats for specific mock |
| GET | `/mocks/{id}/history` | Get hit history for specific mock |
| POST | `/mocks/history/clear` | Clear all hit history |

### Rewrites

| Method | Path | Description |
|--------|------|-------------|
| GET | `/rewrites` | List all rewrite rules |
| POST | `/rewrites` | Create rewrite rule |
| GET | `/rewrites/templates` | Get predefined rewrite templates |
| GET | `/rewrites/{id}` | Get specific rewrite rule |
| DELETE | `/rewrites/{id}` | Delete rewrite rule |
| POST | `/rewrites/{id}/toggle` | Enable/disable rewrite |
| POST | `/rewrites/batch-toggle` | Toggle multiple rewrites |

**POST /rewrites body:** `name` (string), `condition` (object), `direction` (request/response/both), `rewrites` (array of action objects), `priority` (integer, optional)

### Throttle

| Method | Path | Description |
|--------|------|-------------|
| GET | `/throttle` | Get current throttle profile |
| POST | `/throttle` | Set throttle profile |
| POST | `/throttle/enabled` | Enable/disable throttling |
| GET | `/throttle/presets` | List throttle presets |

**POST /throttle body:** `download_bps`, `upload_bps`, `delay_ms`, `jitter_ms`, `packet_loss_percent`, `name`, `enabled`

### Replay

| Method | Path | Description |
|--------|------|-------------|
| GET | `/replay/saved` | List saved requests |
| POST | `/replay/saved` | Save a request |
| GET | `/replay/saved/{id}` | Get specific saved request |
| DELETE | `/replay/saved/{id}` | Delete saved request |
| POST | `/replay/execute/{id}` | Execute (replay) a saved request |
| GET | `/replay/history` | Get replay history |
| DELETE | `/replay/history` | Clear replay history |

### Persistence

| Method | Path | Description |
|--------|------|-------------|
| GET | `/persistence/export` | Export all rules (mocks, rewrites, breakpoints, throttle) as JSON |
| POST | `/persistence/import` | Import all rules from JSON |
| POST | `/persistence/save` | Save rules to persistent store (requires `X-Madhyamas-Confirm` header) |
| POST | `/persistence/load` | Load rules from persistent store |

Example: `curl -H 'X-Madhyamas-Confirm: true' -X POST http://localhost:3001/api/persistence/save`

## Phase 3 — Advanced (19 endpoints, feature-gated)

### gRPC (feature: `grpc`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/grpc/connections` | List gRPC connections |
| GET | `/grpc/streams` | List gRPC streams |
| GET | `/grpc/frames` | Get gRPC frames (filters: `service`, `method`, `path`, `direction`, `status_code`, `limit`) |
| GET | `/grpc/stats` | Get gRPC statistics |
| POST | `/grpc/clear` | Clear all gRPC frames |

### Scripts (feature: `scripting`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/scripts` | List all scripts |
| POST | `/scripts` | Create script |
| GET | `/scripts/templates` | Get script templates |
| GET | `/scripts/config` | Get script runtime configuration |
| GET | `/scripts/{id}` | Get specific script |
| PUT | `/scripts/{id}` | Update script |
| DELETE | `/scripts/{id}` | Delete script |
| POST | `/scripts/{id}/toggle` | Enable/disable script |

**POST /scripts body:** `name` (string, 1-255 chars), `source` (string, non-empty), `description` (optional), `hooks` (array, optional)

### Plugins (feature: `plugins`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/plugins` | List all plugins |
| GET | `/plugins/{id}` | Get specific plugin |
| POST | `/plugins/{id}/enable` | Enable plugin |
| POST | `/plugins/{id}/disable` | Disable plugin |
| GET | `/plugins/{id}/stats` | Get plugin statistics |
| POST | `/plugins/reload` | Reload all plugins from disk |

## Phase 4 — Enterprise (20+ endpoints, feature-gated, mostly stubs)

### Performance & Monitoring

| Method | Path | Description |
|--------|------|-------------|
| GET | `/metrics` | Performance metrics (request counts, latency, RPS) |
| GET | `/health/detailed` | Detailed health (version, uptime, memory, connections) |
| GET | `/performance` | Performance stats (metrics, memory, connection pool) |

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| POST | `/auth/login` | User login (returns JWT) |
| POST | `/auth/logout` | User logout |
| GET | `/auth/me` | Get current user |
| POST | `/auth/validate` | Validate JWT token |
| GET | `/auth/api-keys` | List API keys |
| POST | `/auth/api-keys` | Create API key |
| DELETE | `/auth/api-keys/{id}` | Revoke API key |

### User Management

| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List all users (admin) |
| POST | `/users` | Create user (admin) |
| GET | `/users/{id}` | Get user details |
| PUT | `/users/{id}` | Update user |
| DELETE | `/users/{id}` | Delete user |

### RBAC

| Method | Path | Description |
|--------|------|-------------|
| GET | `/rbac/roles` | List all roles |
| GET | `/rbac/permissions` | List all permissions |
| POST | `/rbac/check` | Check if user has permission |

### Audit Logging

| Method | Path | Description |
|--------|------|-------------|
| GET | `/audit` | Get audit log entries (filters: `event_types`, `user_id`, `resource`, `success`, time range) |
| GET | `/audit/stats` | Get audit statistics |
| GET | `/audit/export` | Export audit events |
| DELETE | `/audit/clear` | Clear audit events |

### Onboarding

| Method | Path | Description |
|--------|------|-------------|
| GET | `/onboarding` | Get onboarding status |
| POST | `/onboarding/complete` | Complete onboarding step |
| POST | `/onboarding/skip` | Skip onboarding |

### Configuration Import/Export

| Method | Path | Description |
|--------|------|-------------|
| GET | `/config/export` | Export all configuration |
| POST | `/config/import` | Import configuration |

> **Note:** Phase 4 endpoints are mostly stubs returning `NOT_IMPLEMENTED`. They are conditionally enabled and may require JWT authentication via middleware.

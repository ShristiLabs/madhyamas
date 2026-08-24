# REST API Reference

All 186 REST API endpoints (156 core + 31 enterprise). Base URL: `http://localhost:3001/api`. All endpoints return JSON unless otherwise noted.

## Phase 1 — Core (33 endpoints)

### Traffic

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/traffic` | List captured traffic with filters | `curl 'http://localhost:3001/api/traffic?method=POST&status=500&limit=50'` |
| GET | `/traffic/{id}` | Get single traffic entry | `curl http://localhost:3001/api/traffic/abc123` |
| GET | `/traffic/{id}/script-traces` | Get script execution traces for a traffic entry | `curl http://localhost:3001/api/traffic/abc123/script-traces` |
| POST | `/traffic/clear` | Clear all traffic | `curl -X POST http://localhost:3001/api/traffic/clear` |
| GET | `/traffic/count` | Get traffic count | `curl http://localhost:3001/api/traffic/count` |
| POST | `/traffic/import/har` | Import traffic from a HAR JSON document into a new session | `curl -X POST -H 'Content-Type: application/json' -d @capture.har http://localhost:3001/api/traffic/import/har` |

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

### Secrets

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/secrets` | List secret names (values are never returned) | `curl http://localhost:3001/api/secrets` |
| PUT | `/secrets/{name}` | Create or update a secret (write-only value) | `curl -X PUT -H 'Content-Type: application/json' -d '{"value":"hunter2"}' http://localhost:3001/api/secrets/api_token` |
| DELETE | `/secrets/{name}` | Delete a secret | `curl -X DELETE http://localhost:3001/api/secrets/api_token` |

**Security:** secret values are write-only. No endpoint ever returns a plaintext secret value — `GET /secrets` returns only `{"names": [...]}`, and `PUT`/`DELETE` responses contain only the name and status. If the secrets subsystem is not enabled, these endpoints return 404. Enterprise: these routes require the `config:*` (admin) scope; OSS has no auth.

### Auto Save

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/autosave` | Get Auto Save configuration | `curl http://localhost:3001/api/autosave` |
| PATCH | `/autosave` | Update Auto Save configuration | `curl -X PATCH -H 'Content-Type: application/json' -d '{"enabled":true,"interval_seconds":300}' http://localhost:3001/api/autosave` |
| POST | `/autosave/snapshot` | Trigger an immediate Auto Save snapshot | `curl -X POST http://localhost:3001/api/autosave/snapshot` |

**PATCH /autosave body fields:** `enabled` (boolean), `interval_seconds` (integer), `export_format` (`har`|`session`), `output_dir` (string), `max_backups` (integer), `rotate_after_requests` (integer), `rotate_after_minutes` (integer)

### Capture

| Method | Path | Description | Example |
|--------|------|-------------|---------|
| GET | `/capture` | Get capture status | `curl http://localhost:3001/api/capture` |
| POST | `/capture/toggle` | Toggle capture mode | `curl -X POST http://localhost:3001/api/capture/toggle` |
| GET | `/capture/stats` | Get capture statistics | `curl http://localhost:3001/api/capture/stats` |

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

## Phase 2 — Interception (80 endpoints)

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
| POST | `/mocks/from-traffic` | Create mock rule from captured traffic |
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
| PUT | `/mocks/collections/{id}` | Update collection metadata (name, description, enabled, tags) |
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
| POST | `/replay/saved/from-traffic` | Batch save requests from captured traffic |
| GET | `/replay/saved/{id}` | Get specific saved request |
| DELETE | `/replay/saved/{id}` | Delete saved request |
| POST | `/replay/execute/{id}` | Execute (replay) a saved request |
| POST | `/replay/execute/{id}/batch` | Batch replay (iterations, concurrency, delay) |
| GET | `/replay/history` | Get replay history |
| DELETE | `/replay/history` | Clear replay history |

**POST /replay/execute/{id}/batch body:** `iterations` (integer, max 10000), `concurrency` (integer, max 100), `delay_ms` (integer, optional), `modifications` (object, optional — same shape as replay)

### Block List

| Method | Path | Description |
|--------|------|-------------|
| GET | `/blocklist` | List all block list entries |
| POST | `/blocklist` | Create a block list entry |
| GET | `/blocklist/stats` | Get block list summary statistics |
| GET | `/blocklist/{id}` | Get a specific block list entry |
| PUT | `/blocklist/{id}` | Update a block list entry |
| DELETE | `/blocklist/{id}` | Delete a block list entry |
| POST | `/blocklist/{id}/toggle` | Enable/disable a block list entry |

**POST /blocklist body:** `pattern` (string, required), `note` (string, optional), `enabled` (boolean, default true), `status_code` (integer, default 403), `response_body` (string, optional), `content_type` (string, optional)

### Focus Hosts

| Method | Path | Description |
|--------|------|-------------|
| GET | `/focus` | List all focus host patterns |
| POST | `/focus` | Add a focus host pattern |
| DELETE | `/focus` | Clear all focus hosts |
| DELETE | `/focus/{id}` | Remove a specific focus host by ID |

**POST /focus body:** `pattern` (string, required) — exact hostname, wildcard subdomain (`*.example.com`), or glob (`*api*`)

### Mirror

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mirror` | Get mirror status and statistics |
| POST | `/mirror/toggle` | Toggle mirroring on/off |
| PATCH | `/mirror/config` | Update mirror configuration |

**POST /mirror/toggle body:** `enabled` (boolean, required)

**PATCH /mirror/config body:** `enabled` (boolean), `output_dir` (string), `host_filter` (array[string]), `save_request_bodies` (boolean)

### Logs

| Method | Path | Description |
|--------|------|-------------|
| GET | `/logs` | Get log rotation status (config, current file, archived files) |
| PATCH | `/logs` | Update log rotation configuration |
| POST | `/logs/rotate` | Rotate the current log file immediately (on-demand) |

**PATCH /logs body:** `enabled` (boolean), `rotation` (object: `{"mode":"never"|"hourly"|"daily"}` or `{"mode":"size","size_mb":<n>}`), `max_files` (integer), `max_file_size_mb` (integer), `json_format` (boolean), `async_mode` (string), `async_writing` (boolean), `async_buffer_size` (integer, restart to apply), `debug_logging` (object: `enabled` (boolean), `level` (`summary`|`headers`|`full`), `host_filter` (array of host patterns), `redact_headers` (array of header names), `redact_bodies` (boolean) — runtime-toggleable proxied-traffic debug logging)

### Persistence

| Method | Path | Description |
|--------|------|-------------|
| GET | `/persistence/export` | Export all rules (mocks, rewrites, breakpoints, throttle) as JSON |
| POST | `/persistence/import` | Import all rules from JSON |
| POST | `/persistence/save` | Save rules to persistent store (requires `X-Madhyamas-Confirm` header) |
| POST | `/persistence/load` | Load rules from persistent store |

Example: `curl -H 'X-Madhyamas-Confirm: true' -X POST http://localhost:3001/api/persistence/save`

## Phase 3 — Advanced (42 endpoints, feature-gated)

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
| PUT | `/scripts/config` | Update script runtime configuration |
| GET | `/scripts/history` | Get execution history across all scripts |
| POST | `/scripts/test` | Test (dry-run) a script against a sample context |
| POST | `/scripts/validate` | Validate a script's syntax without executing |
| POST | `/scripts/match-preview` | Preview which scripts would match a request |
| GET | `/scripts/{id}` | Get specific script |
| PUT | `/scripts/{id}` | Update script |
| DELETE | `/scripts/{id}` | Delete script |
| POST | `/scripts/{id}/toggle` | Enable/disable script |
| POST | `/scripts/{id}/reorder` | Reorder a script (change priority) |
| GET | `/scripts/{id}/history` | Get execution history for a specific script |
| DELETE | `/scripts/{id}/history` | Clear execution history for a specific script |

**POST /scripts body:** `name` (string, 1-255 chars), `source` (string, non-empty), `description` (optional), `hooks` (array, optional)

**PUT /scripts/config body:** `timeout_ms` (integer), `memory_limit_mb` (integer), `capture_console` (boolean)

### Plugins (feature: `plugins`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/plugins` | List all plugins |
| GET | `/plugins/{id}` | Get specific plugin |
| POST | `/plugins/{id}/enable` | Enable plugin |
| POST | `/plugins/{id}/disable` | Disable plugin |
| GET | `/plugins/{id}/stats` | Get plugin statistics |
| POST | `/plugins/reload` | Reload all plugins from disk |
| POST | `/plugins/install` | Install a plugin from a URL or registry id |
| DELETE | `/plugins/{id}/uninstall` | Uninstall a plugin |
| GET | `/plugins/{id}/settings` | Get a plugin's current settings |
| PUT | `/plugins/{id}/settings` | Update a plugin's settings |
| GET | `/plugins/{id}/schema` | Get a plugin's settings schema |
| GET | `/plugins/{id}/panels` | Get a plugin's declarative UI panels |
| GET | `/plugins/{id}/logs` | Get a plugin's recent invocation logs |
| GET | `/plugins/registry` | List all available plugins in the registry |
| GET | `/plugins/registry/search` | Search the plugin registry (query param `q`) |
| GET | `/plugins/registry/{id}` | Get a specific registry entry |
| GET | `/plugins/registry/config` | Get the registry configuration |
| PUT | `/plugins/registry/config` | Update the registry repository configuration |
| POST | `/plugins/registry/refresh` | Force-refresh the registry cache |
| GET | `/plugins/templates` | List available plugin scaffolding templates |
| POST | `/plugins/scaffold` | Scaffold a new plugin project from a template |

**POST /plugins/install body:** `source` (`url`|`registry`), `url` (string, when source=url), `id` (string, when source=registry), `checksum` (string, optional for URL source)

## Phase 4 — Enterprise (31 endpoints, feature-gated)

### Performance & Monitoring

| Method | Path | Description |
|--------|------|-------------|
| GET | `/metrics` | Performance metrics (request counts, latency, RPS) |
| GET | `/metrics/cluster` | Cluster-wide metrics across all instances |
| GET | `/instances` | List active instances in the cluster |
| GET | `/license` | Get license info (tier, seats, expiry) |
| GET | `/health/detailed` | Detailed health (version, uptime, memory, connections) |
| GET | `/performance` | Performance stats (metrics, memory, connection pool) |

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| POST | `/auth/login` | User login (returns JWT) |
| POST | `/auth/refresh` | Refresh JWT token |
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

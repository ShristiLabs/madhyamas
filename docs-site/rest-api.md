---
title: REST API Reference
description: The Madhyamas REST API — 177 endpoints under /api for traffic, sessions, mocks, breakpoints, rewrites, replay, scripts, plugins, gRPC, WebSocket traffic, config, and enterprise features.
---

# REST API Reference

Everything the Madhyamas web UI and CLI do is powered by a REST API listening on the same port as the web UI (default `3001`). All endpoints are under the `/api` prefix and return JSON unless otherwise noted. You can use this API directly to integrate Madhyamas into scripts, CI pipelines, dashboards, or custom tools.

## Base URL and Conventions

- **Base URL**: `http://localhost:3001/api`
- **Content type**: `application/json` for request bodies and responses
- **Authentication**: none by default. When the enterprise feature is enabled, supply a JWT via `Authorization: Bearer <token>` or an API key via `X-API-Key: <key>`.
- **Errors**: non-2xx responses carry a JSON body of the form `{"error": "...", "code": "..."}`.
- **Real-time updates**: connect to `GET /ws` (WebSocket) for live traffic events.

## Health and Real-time

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Liveness check — returns the text `OK` |
| GET | `/health/detailed` | Version, uptime, memory, connection stats (enterprise) |
| GET | `/ws` | WebSocket upgrade for real-time traffic updates |

WebSocket server events: `Connected`, `InitialTraffic`, `Traffic` (Added/Updated/Deleted/Cleared/CountUpdate), `Pong`, `Error`. Client messages: `Ping`, `Subscribe` (with optional filter), `Unsubscribe`, `GetInitialTraffic`.

## Traffic

| Method | Path | Description |
|--------|------|-------------|
| GET | `/traffic` | List captured traffic (query: `method`, `url`, `status_code`, `content_type`, `limit`, `offset`) |
| GET | `/traffic/{id}` | Get a single traffic entry with full headers and body |
| GET | `/traffic/{id}/script-traces` | Get script execution traces for a traffic entry |
| GET | `/traffic/count` | Get the total count of captured entries |
| POST | `/traffic/clear` | Clear all captured traffic |
| POST | `/traffic/import/har` | Import a HAR JSON document into a new session |

```bash
# List the 50 most recent 500s
curl 'http://localhost:3001/api/traffic?status_code=500&limit=50'

# Import a HAR file
curl -X POST -H 'Content-Type: application/json' -d @capture.har \
  http://localhost:3001/api/traffic/import/har
```

## Sessions

| Method | Path | Description |
|--------|------|-------------|
| GET | `/sessions` | List all sessions |
| POST | `/sessions` | Create a session (`{"name":"..."}`) |
| GET | `/sessions/{id}` | Get session details |
| DELETE | `/sessions/{id}` | Delete a session |
| GET | `/sessions/{id}/export` | Export a session (`?format=har`) |
| POST | `/sessions/{id}/switch` | Switch the active session |
| POST | `/sessions/import` | Import a session from JSON |

## Export

| Method | Path | Description |
|--------|------|-------------|
| GET | `/export/har` | Export all traffic as a HAR file |
| GET | `/export/curl/{id}` | Get a cURL command reproducing a request |

## Certificate

| Method | Path | Description |
|--------|------|-------------|
| GET | `/cert/ca` | Download the CA certificate (PEM) |

## Configuration

| Method | Path | Description |
|--------|------|-------------|
| GET | `/config` | Get the proxy configuration |
| PATCH | `/config` | Update configuration |
| GET | `/config/export` | Export all configuration (enterprise) |
| POST | `/config/import` | Import configuration (enterprise) |

`PATCH /config` accepts: `intercept_https` (boolean), `max_requests` (integer), `max_body_size` (integer), `verbose` (boolean), `public_ip` (string|null), `passthrough_domains` (array), `enable_h2_downstream` (boolean).

```bash
curl -X PATCH http://localhost:3001/api/config \
  -H 'Content-Type: application/json' \
  -d '{"intercept_https":false,"max_requests":50000}'
```

## Capture Mode

| Method | Path | Description |
|--------|------|-------------|
| GET | `/capture` | Get capture status (Recording or Passthrough) |
| POST | `/capture/toggle` | Toggle capture mode |
| GET | `/capture/stats` | Get capture statistics |

## Auto Save

| Method | Path | Description |
|--------|------|-------------|
| GET | `/autosave` | Get Auto Save configuration |
| PATCH | `/autosave` | Update Auto Save configuration |
| POST | `/autosave/snapshot` | Trigger an immediate backup snapshot |

`PATCH /autosave` accepts: `enabled`, `interval_seconds`, `export_format` (`har`|`session`), `output_dir`, `max_backups`, `rotate_after_requests`, `rotate_after_minutes`.

## Breakpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/breakpoints` | List all breakpoint rules |
| POST | `/breakpoints` | Create a breakpoint rule |
| GET | `/breakpoints/{id}` | Get a specific rule |
| DELETE | `/breakpoints/{id}` | Delete a rule |
| GET | `/breakpoints/paused` | List all paused traffic items |
| GET | `/breakpoints/paused/{id}` | Get a specific paused item |
| POST | `/breakpoints/paused/{id}/resume` | Resume with a decision: `allow`, `modify`, or `reject` |

## Mocks

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mocks` | List all mock rules |
| POST | `/mocks` | Create a mock rule |
| GET | `/mocks/templates` | Get predefined mock templates |
| GET | `/mocks/{id}` | Get a specific mock |
| PUT | `/mocks/{id}` | Update a mock |
| DELETE | `/mocks/{id}` | Delete a mock |
| POST | `/mocks/{id}/toggle` | Enable/disable a mock |
| POST | `/mocks/batch-toggle` | Toggle multiple mocks |
| POST | `/mocks/advanced` | Create an advanced mock (tags, collection, description) |
| POST | `/mocks/{id}/duplicate` | Duplicate a mock |
| POST | `/mocks/{id}/rollback` | Roll back a mock to a previous version |
| GET | `/mocks/{id}/versions` | Get mock version history |
| POST | `/mocks/{id}/test` | Test a mock against a sample request |
| POST | `/mocks/preview` | Preview which mock matches a request |
| GET | `/mocks/export` | Export all mocks as JSON |
| POST | `/mocks/import` | Import mocks (HAR/OpenAPI/Postman) |

### Mock Collections

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mocks/collections` | List all collections |
| POST | `/mocks/collections` | Create a collection |
| GET | `/mocks/collections/{id}` | Get a specific collection |
| PUT | `/mocks/collections/{id}` | Update collection metadata |
| DELETE | `/mocks/collections/{id}` | Delete a collection (optional `delete_rules`) |
| POST | `/mocks/collections/{id}/toggle` | Toggle all mocks in a collection |

### Mock Recording

| Method | Path | Description |
|--------|------|-------------|
| POST | `/mocks/recording` | Enable/disable recording mode |
| GET | `/mocks/recording/status` | Get recording status |
| GET | `/mocks/recording/recorded` | Get recorded mocks |
| POST | `/mocks/recording/promote` | Promote recorded mocks to active rules |
| POST | `/mocks/recording/clear` | Clear recorded mocks |

### Mock Analytics

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mocks/analytics` | Get hit analytics for all mocks |
| GET | `/mocks/{id}/analytics` | Get hit stats for a specific mock |
| GET | `/mocks/{id}/history` | Get hit history for a specific mock |
| POST | `/mocks/history/clear` | Clear all hit history |

## Rewrites

| Method | Path | Description |
|--------|------|-------------|
| GET | `/rewrites` | List all rewrite rules |
| POST | `/rewrites` | Create a rewrite rule |
| GET | `/rewrites/templates` | Get predefined rewrite templates |
| GET | `/rewrites/{id}` | Get a specific rewrite |
| DELETE | `/rewrites/{id}` | Delete a rewrite |
| POST | `/rewrites/{id}/toggle` | Enable/disable a rewrite |
| POST | `/rewrites/batch-toggle` | Toggle multiple rewrites |

`POST /rewrites` body: `name`, `condition` (object), `direction` (`request`|`response`|`both`), `rewrites` (array of action objects), `priority` (optional).

## Throttle

| Method | Path | Description |
|--------|------|-------------|
| GET | `/throttle` | Get the current throttle profile |
| POST | `/throttle` | Set the throttle profile |
| POST | `/throttle/enabled` | Enable/disable throttling |
| GET | `/throttle/presets` | List throttle presets |

`POST /throttle` body: `download_bps`, `upload_bps`, `delay_ms`, `jitter_ms`, `packet_loss_percent`, `name`, `enabled`.

## Replay

| Method | Path | Description |
|--------|------|-------------|
| GET | `/replay/saved` | List saved requests |
| POST | `/replay/saved` | Save a request |
| GET | `/replay/saved/{id}` | Get a specific saved request |
| DELETE | `/replay/saved/{id}` | Delete a saved request |
| POST | `/replay/execute/{id}` | Replay a saved request |
| POST | `/replay/execute/{id}/batch` | Batch replay (iterations, concurrency, delay) |
| GET | `/replay/history` | Get replay history |
| DELETE | `/replay/history` | Clear replay history |

`POST /replay/execute/{id}/batch` body: `iterations` (max 10,000), `concurrency` (max 100), `delay_ms`, `modifications` (object — same shape as single replay).

## Block List

| Method | Path | Description |
|--------|------|-------------|
| GET | `/blocklist` | List all block list entries |
| POST | `/blocklist` | Create an entry |
| GET | `/blocklist/stats` | Get summary statistics |
| GET | `/blocklist/{id}` | Get a specific entry |
| PUT | `/blocklist/{id}` | Update an entry |
| DELETE | `/blocklist/{id}` | Delete an entry |
| POST | `/blocklist/{id}/toggle` | Enable/disable an entry |

`POST /blocklist` body: `pattern` (required), `note`, `enabled` (default true), `status_code` (default 403), `response_body`, `content_type`.

## Focus Hosts

| Method | Path | Description |
|--------|------|-------------|
| GET | `/focus` | List all focus host patterns |
| POST | `/focus` | Add a focus host pattern |
| DELETE | `/focus` | Clear all focus hosts |
| DELETE | `/focus/{id}` | Remove a specific focus host |

`POST /focus` body: `pattern` (required) — exact hostname, wildcard subdomain (`*.example.com`), or glob (`*api*`).

## Mirror

| Method | Path | Description |
|--------|------|-------------|
| GET | `/mirror` | Get mirror status and statistics |
| POST | `/mirror/toggle` | Toggle mirroring on/off |
| PATCH | `/mirror/config` | Update mirror configuration |

`PATCH /mirror/config` body: `enabled`, `output_dir`, `host_filter` (array), `save_request_bodies`.

## Logs

| Method | Path | Description |
|--------|------|-------------|
| GET | `/logs` | Get log rotation status (config, current file, archived files) |
| PATCH | `/logs` | Update log rotation configuration |
| POST | `/logs/rotate` | Rotate the current log file immediately |

`PATCH /logs` body: `enabled`, `rotation` (`{"mode":"never"|"hourly"|"daily"}` or `{"mode":"size","size_mb":<n>}`), `max_files`, `max_file_size_mb`, `json_format`.

## Persistence

| Method | Path | Description |
|--------|------|-------------|
| GET | `/persistence/export` | Export all rules (mocks, rewrites, breakpoints, throttle) as JSON |
| POST | `/persistence/import` | Import all rules from JSON |
| POST | `/persistence/save` | Save rules to the persistent store (requires `X-Madhyamas-Confirm: true`) |
| POST | `/persistence/load` | Load rules from the persistent store |

## gRPC (feature: `grpc`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/grpc/connections` | List gRPC connections |
| GET | `/grpc/streams` | List gRPC streams |
| GET | `/grpc/frames` | Get gRPC frames (filters: `service`, `method`, `path`, `direction`, `status_code`, `limit`) |
| GET | `/grpc/stats` | Get gRPC statistics |
| POST | `/grpc/clear` | Clear all gRPC frames |

## WebSocket Traffic

| Method | Path | Description |
|--------|------|-------------|
| GET | `/ws-traffic/connections` | List WebSocket connections |
| GET | `/ws-traffic/connections/{id}` | Get a specific WebSocket connection |
| GET | `/ws-traffic/messages` | Get WebSocket messages (filters: `connection_id`, `direction`, `message_type`, `search`) |
| POST | `/ws-traffic/clear` | Clear WebSocket traffic |

## Scripts (feature: `scripting`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/scripts` | List all scripts |
| POST | `/scripts` | Create a script |
| GET | `/scripts/templates` | Get script templates |
| GET | `/scripts/config` | Get script runtime configuration |
| PUT | `/scripts/config` | Update script runtime configuration |
| GET | `/scripts/history` | Get execution history across all scripts |
| POST | `/scripts/test` | Dry-run a script against a sample context |
| POST | `/scripts/validate` | Validate a script's syntax without executing |
| POST | `/scripts/match-preview` | Preview which scripts would match a request |
| GET | `/scripts/{id}` | Get a specific script |
| PUT | `/scripts/{id}` | Update a script |
| DELETE | `/scripts/{id}` | Delete a script |
| POST | `/scripts/{id}/toggle` | Enable/disable a script |
| POST | `/scripts/{id}/reorder` | Reorder a script (change priority) |
| GET | `/scripts/{id}/history` | Get execution history for a specific script |
| DELETE | `/scripts/{id}/history` | Clear execution history for a specific script |

`POST /scripts` body: `name` (1-255 chars), `source` (non-empty), `description` (optional), `hooks` (array, optional). `PUT /scripts/config` body: `timeout_ms`, `memory_limit_mb`, `capture_console`.

## Plugins (feature: `plugins`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/plugins` | List all plugins |
| GET | `/plugins/{id}` | Get a specific plugin |
| POST | `/plugins/{id}/enable` | Enable a plugin |
| POST | `/plugins/{id}/disable` | Disable a plugin |
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
| GET | `/plugins/registry/search` | Search the registry (`?q=...`) |
| GET | `/plugins/registry/{id}` | Get a specific registry entry |
| GET | `/plugins/registry/config` | Get the registry configuration |
| PUT | `/plugins/registry/config` | Update the registry repository configuration |
| POST | `/plugins/registry/refresh` | Force-refresh the registry cache |
| GET | `/plugins/templates` | List available plugin scaffolding templates |
| POST | `/plugins/scaffold` | Scaffold a new plugin project from a template |

`POST /plugins/install` body: `source` (`url`|`registry`), `url` (when source=url), `id` (when source=registry), `checksum` (optional for URL source).

## Enterprise Endpoints (feature-gated)

These endpoints are conditionally enabled and may require JWT authentication. Several are stubs returning `NOT_IMPLEMENTED`; see [Enterprise](./enterprise) for the feature matrix.

### Performance and Monitoring

| Method | Path | Description |
|--------|------|-------------|
| GET | `/metrics` | Performance metrics (request counts, latency, RPS) |
| GET | `/performance` | Performance stats (metrics, memory, connection pool) |

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| POST | `/auth/login` | User login (returns JWT) |
| POST | `/auth/logout` | User logout |
| GET | `/auth/me` | Get current user |
| POST | `/auth/validate` | Validate a JWT |
| GET | `/auth/api-keys` | List API keys |
| POST | `/auth/api-keys` | Create an API key |
| DELETE | `/auth/api-keys/{id}` | Revoke an API key |

### User Management

| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List all users (admin) |
| POST | `/users` | Create a user (admin) |
| GET | `/users/{id}` | Get user details |
| PUT | `/users/{id}` | Update a user |
| DELETE | `/users/{id}` | Delete a user |

### RBAC

| Method | Path | Description |
|--------|------|-------------|
| GET | `/rbac/roles` | List all roles |
| GET | `/rbac/permissions` | List all permissions |
| POST | `/rbac/check` | Check if a user has a permission |

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
| POST | `/onboarding/complete` | Complete an onboarding step |
| POST | `/onboarding/skip` | Skip onboarding |

## See also

- [CLI reference](./cli) — a friendlier wrapper over the same API
- [MCP & AI Agents](./mcp) — LLM access to the API via MCP tools
- [Configuration](./configuration) — the runtime config fields accepted by `PATCH /api/config`
- [Enterprise](./enterprise) — auth, RBAC, audit, and user management

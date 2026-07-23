# MCP Tools Reference

All 67 MCP tools exposed by the Madhyamas MCP server. The MCP server uses stdio transport and connects to a running Madhyamas proxy instance via REST API.

## Traffic Inspection (5 tools)

### madhyamas_get_traffic

List captured HTTP traffic with advanced filtering.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `filter` | string | No | Filter expression to match URLs (supports wildcards) |
| `method` | string | No | Filter by HTTP method. Enum: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS |
| `status` | integer | No | Filter by HTTP status code (e.g., 200, 404, 500) |
| `file_type` | string | No | Filter by file type/extension (e.g., json, html, css, js, png) |
| `header` | string | No | Filter by header (format: `key:value` or just `key`) |
| `cookie` | string | No | Filter by cookie (format: `name=value` or just `name`) |
| `search` | string | No | Search in request/response bodies |
| `min_size` | integer | No | Minimum response size in bytes |
| `max_size` | integer | No | Maximum response size in bytes |
| `min_time` | integer | No | Minimum response time in milliseconds |
| `max_time` | integer | No | Maximum response time in milliseconds |
| `limit` | integer | No | Maximum results (default: 100) |
| `offset` | integer | No | Offset for pagination |

Example: Get all POST requests to API endpoints:
```
madhyamas_get_traffic(method="POST", filter="*/api/*")
```

### madhyamas_get_traffic_entry

Get detailed information about a specific traffic entry, including full request/response headers and bodies.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the traffic entry |

Example: `madhyamas_get_traffic_entry(id="abc123")`

### madhyamas_search_traffic

Search captured traffic by content (headers, bodies, URLs).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query string |

Example: `madhyamas_search_traffic(query="authorization")`

### madhyamas_get_traffic_count

Get the total count of captured traffic entries. No parameters.

Example: `madhyamas_get_traffic_count()`

### madhyamas_clear_traffic

Clear all captured traffic. Cannot be undone. No parameters.

Example: `madhyamas_clear_traffic()`

## Mock Rules (18 tools)

### madhyamas_list_mocks

List all mock rules currently configured. No parameters.

### madhyamas_create_mock

Create a mock rule to intercept and replace responses.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `url_pattern` | string | Yes | URL pattern to match (supports wildcards and regex) |
| `method` | string | No | HTTP method. Enum: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS |
| `status_code` | integer | No | HTTP status code (default: 200) |
| `headers` | object | No | Response headers to return |
| `body` | any | No | Response body to return |
| `delay_ms` | integer | No | Delay before responding (ms) |
| `enabled` | boolean | No | Enable immediately (default: true) |

Example: `madhyamas_create_mock(url_pattern="*/api/auth*", status_code=200, body='{"token":"fake"}')`

### madhyamas_delete_mock

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |

### madhyamas_toggle_mock

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |
| `enabled` | boolean | Yes | true to enable, false to disable |

### madhyamas_create_advanced_mock

Create an advanced mock with response sequences, conditional responses, or probabilistic responses.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Name for the mock rule |
| `condition` | object | Yes | Match condition, e.g., `{"type":"url_pattern","pattern":"https://api.example.com/.*"}` |
| `response_config` | object | Yes | Response config. Types: `single`, `sequence`, `conditional`, `probabilistic` |
| `description` | string | No | Optional description |
| `tags` | array[string] | No | Tags for organization |
| `collection_id` | string | No | Collection to add this mock to |
| `enabled` | boolean | No | Enable immediately (default: true) |
| `priority` | integer | No | Priority (lower = higher, default: 100) |

Example:
```
madhyamas_create_advanced_mock(
  name="Auth Mock Sequence",
  condition={"type":"url_pattern","pattern":"*/api/auth*"},
  response_config={"type":"sequence","responses":[
    {"status_code":200,"body":"{\"token\":\"first\"}"},
    {"status_code":401,"body":"{\"error\":\"expired\"}"}
  ]}
)
```

### madhyamas_update_mock

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |
| `mock` | object | Yes | Full mock rule object to update |

### madhyamas_get_mock

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |

### madhyamas_duplicate_mock

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID to duplicate |
| `new_name` | string | No | Optional new name for the duplicate |

### madhyamas_rollback_mock

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |
| `version` | integer | Yes | Version number to rollback to |

### madhyamas_get_mock_versions

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |

### madhyamas_test_mock

Test a mock rule against a sample request.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |
| `request` | object | Yes | Sample request data (url, method, headers, body) |

### madhyamas_preview_mock_match

Preview which mock rule would match a given request without intercepting.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `request` | object | Yes | Request data to test against all mocks |

### madhyamas_export_mocks

Export all mock rules as JSON. No parameters.

### madhyamas_import_mocks

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `format` | string | Yes | Import format. Enum: `har`, `openapi`, `postman` |
| `data` | string | Yes | Data to import (JSON string) |

### madhyamas_set_mock_recording

Enable/disable mock recording mode. When enabled, responses are captured as potential mock rules.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `enabled` | boolean | Yes | true to enable recording |

### madhyamas_get_mock_recording_status

Get current mock recording status. No parameters.

### madhyamas_get_recorded_mocks

Get all mock rules recorded from live traffic. No parameters.

### madhyamas_promote_recorded_mocks

Promote all recorded mocks to active mock rules. No parameters.

## Mock Collections (4 tools)

### madhyamas_list_mock_collections

List all mock collections. No parameters.

### madhyamas_create_mock_collection

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Collection name |
| `description` | string | No | Optional description |
| `tags` | array[string] | No | Tags for the collection |

### madhyamas_delete_mock_collection

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Collection ID |
| `delete_rules` | boolean | No | Also delete all rules in collection (default: false) |

### madhyamas_toggle_mock_collection

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Collection ID |
| `enabled` | boolean | Yes | true to enable all, false to disable all |

## Mock Analytics (2 tools)

### madhyamas_get_mock_analytics

Get hit analytics for all mock rules. No parameters.

### madhyamas_get_mock_hit_history

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Mock rule ID |

## Breakpoints (3 tools)

### madhyamas_list_breakpoints

List all breakpoint rules. No parameters.

### madhyamas_create_breakpoint

Create a breakpoint to pause traffic matching a pattern.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `url_pattern` | string | Yes | URL pattern to match |
| `method` | string | No | HTTP method. Enum: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS |
| `direction` | string | No | Intercept direction. Enum: `request`, `response`, `both` (default: request) |
| `enabled` | boolean | No | Enable immediately (default: true) |

Example: `madhyamas_create_breakpoint(url_pattern="*/api/auth*", direction="request")`

### madhyamas_delete_breakpoint

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Breakpoint rule ID |

## Replay (4 tools)

### madhyamas_replay_request

Replay a captured request with optional modifications.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Traffic entry ID to replay |
| `modifications` | object | No | Modifications with `headers` (object), `body` (any), `url` (string) |

Example: `madhyamas_replay_request(id="abc123", modifications={"headers":{"Authorization":"Bearer newtoken"}})`

### madhyamas_save_request

Save a request for later replay.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `traffic_id` | string | Yes | Traffic entry ID to save |
| `name` | string | No | Optional name for the saved request |

### madhyamas_list_saved_requests

List all saved requests. No parameters.

### madhyamas_export_curl

Export a request as a cURL command.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Traffic entry ID |

## Sessions (5 tools)

### madhyamas_list_sessions

List all debugging sessions. No parameters.

### madhyamas_create_session

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | No | Session name |
| `description` | string | No | Session description |

### madhyamas_switch_session

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Session ID to switch to |

### madhyamas_export_session

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Session ID |
| `format` | string | No | Export format. Enum: `har`, `curl` (default: har) |

### madhyamas_import_session

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `session_data` | object | Yes | Session data (HAR format or Madhyamas export) |

## Configuration (2 tools)

### madhyamas_get_config

Get current configuration (proxy port, API port, host, HTTPS interception, max requests). No parameters.

### madhyamas_update_config

Update runtime configuration. Only specified fields are updated.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `intercept_https` | boolean | No | Enable/disable HTTPS interception |
| `max_requests` | integer | No | Max requests in memory |
| `verbose` | boolean | No | Enable/disable verbose logging |
| `public_ip` | string/null | No | Public IP to display (null to auto-detect) |

## Capture Mode (2 tools)

### madhyamas_get_capture_status

Get capture mode status (recording vs passthrough). No parameters.

### madhyamas_toggle_capture

Toggle between recording and passthrough mode. No parameters.

## Throttle (4 tools)

### madhyamas_get_throttle

Get current throttle profile (bandwidth, latency, jitter, packet loss). No parameters.

### madhyamas_set_throttle

Set a custom throttle profile.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `download_bps` | integer | No | Download bandwidth (bytes/sec, 0 = unlimited) |
| `upload_bps` | integer | No | Upload bandwidth (bytes/sec, 0 = unlimited) |
| `delay_ms` | integer | No | Latency in milliseconds |
| `jitter_ms` | integer | No | Jitter in milliseconds |
| `packet_loss_percent` | integer | No | Packet loss percentage (0-100) |
| `name` | string | No | Profile name |
| `enabled` | boolean | No | Enable throttling immediately |

Example: `madhyamas_set_throttle(download_bps=50000, delay_ms=200, enabled=true)`

### madhyamas_toggle_throttle

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `enabled` | boolean | Yes | true to enable, false to disable |

### madhyamas_get_throttle_presets

List predefined throttle profiles (GPRS, EDGE, 3G, 4G LTE, etc.). No parameters.

## Rewrites (5 tools)

### madhyamas_list_rewrites

List all rewrite rules. No parameters.

### madhyamas_create_rewrite

Create a rewrite rule to modify URLs, headers, or bodies.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Rule name |
| `condition` | object | Yes | Match condition, e.g., `{"type":"url_pattern","pattern":"https://api.example.com/.*"}` |
| `direction` | string | Yes | Apply direction. Enum: `request`, `response`, `both` |
| `rewrites` | array[object] | Yes | Rewrite actions, e.g., `[{"type":"set_header","name":"X-Custom","value":"test"}]` |
| `enabled` | boolean | No | Enable immediately (default: true) |
| `priority` | integer | No | Priority (lower = higher, default: 100) |

### madhyamas_delete_rewrite

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Rewrite rule ID |

### madhyamas_toggle_rewrite

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Rewrite rule ID |
| `enabled` | boolean | Yes | true to enable, false to disable |

### madhyamas_get_rewrite_templates

Get predefined rewrite templates for common scenarios. No parameters.

## gRPC (5 tools)

### madhyamas_get_grpc_connections

List all captured gRPC connections. No parameters.

### madhyamas_get_grpc_streams

List all gRPC streams. No parameters.

### madhyamas_get_grpc_frames

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `filter` | string | No | Optional filter expression for frames |

### madhyamas_get_grpc_stats

Get aggregated gRPC statistics. No parameters.

### madhyamas_clear_grpc

Clear all gRPC frames and reset statistics. No parameters.

## Scripts (7 tools)

### madhyamas_list_scripts

List all registered scripts. No parameters.

### madhyamas_create_script

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Script name |
| `source` | string | Yes | Script source code |
| `hook` | string | No | Hook to attach (e.g., `on_request`, `on_response`) |
| `enabled` | boolean | No | Enable immediately (default: true) |

### madhyamas_get_script

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Script ID |

### madhyamas_update_script

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Script ID |
| `script` | object | Yes | Full script object to update |

### madhyamas_delete_script

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Script ID |

### madhyamas_toggle_script

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Script ID |
| `enabled` | boolean | Yes | true to enable, false to disable |

### madhyamas_get_script_templates

Get predefined script templates. No parameters.

## Plugins (6 tools)

### madhyamas_list_plugins

List all loaded plugins. No parameters.

### madhyamas_get_plugin

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Plugin ID |

### madhyamas_enable_plugin

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Plugin ID |

### madhyamas_disable_plugin

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Plugin ID |

### madhyamas_get_plugin_stats

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Plugin ID |

### madhyamas_reload_plugins

Reload all plugins from disk. No parameters.

## MCP Resources

The MCP server also exposes read-only resources:

| Resource URI | Description |
|--------------|-------------|
| `madhyamas://traffic` | All captured traffic |
| `madhyamas://sessions` | All debugging sessions |
| `madhyamas://config` | Proxy configuration |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MADHYAMAS_API_URL` | `http://127.0.0.1:3001` | API endpoint for the running proxy |
| `MADHYAMAS_TIMEOUT` | `30` | Request timeout in seconds |
| `RUST_LOG` | `info` | Logging level |

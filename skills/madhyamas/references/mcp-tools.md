# MCP Tools Reference

All 135 MCP tools exposed by the Madhyamas MCP server. The MCP server uses stdio transport and connects to a running Madhyamas proxy instance via REST API.

## Traffic Inspection (7 tools)

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

### madhyamas_import_har

Import traffic from a HAR (HTTP Archive) JSON document into a new session. Each log.entries[] entry is converted into a traffic entry. Invalid entries are skipped. Useful for loading traffic captured by other tools (Chrome DevTools, Charles, Fiddler).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `har` | object | Yes | The full HAR JSON document (must contain a `log` object with an `entries` array) |
| `session_name` | string | No | Optional name for the newly created session (default: `Imported HAR`) |
| `switch_session` | boolean | No | Switch the active session to the newly created one after import (default: false) |

Example: `madhyamas_import_har(har={"log":{"entries":[...]}}, session_name="Chrome capture")`

### madhyamas_get_traffic_script_traces

Get script execution traces for a specific traffic entry, showing which scripts ran on the request and their results.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Traffic entry ID |

Example: `madhyamas_get_traffic_script_traces(id="abc123")`

## Mock Rules (21 tools)

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

### madhyamas_get_mock_templates

List available predefined mock templates for quick creation. No parameters.

Example: `madhyamas_get_mock_templates()`

### madhyamas_batch_toggle_mocks

Enable or disable multiple mock rules in a single request.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `ids` | array[string] | Yes | List of mock rule IDs to toggle |
| `enabled` | boolean | Yes | true to enable, false to disable |

Example: `madhyamas_batch_toggle_mocks(ids=["abc","def"], enabled=false)`

### madhyamas_clear_mock_recording

Clear all mock candidates that have been recorded from live traffic. No parameters.

Example: `madhyamas_clear_mock_recording()`

### madhyamas_clear_mock_history

Clear all mock hit history and analytics data. No parameters.

Example: `madhyamas_clear_mock_history()`

## Mock Collections (6 tools)

### madhyamas_list_mock_collections

List all mock collections. No parameters.

### madhyamas_create_mock_collection

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Collection name |
| `description` | string | No | Optional description |
| `tags` | array[string] | No | Tags for the collection |

### madhyamas_get_mock_collection

Get details of a specific mock collection by ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Collection ID |

### madhyamas_update_mock_collection

Update a mock collection's metadata. Only provided fields are updated.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Collection ID |
| `name` | string | No | New name for the collection |
| `description` | string | No | New description |
| `enabled` | boolean | No | Whether the collection is enabled |
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

## Breakpoints (7 tools)

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

### madhyamas_get_breakpoint

Get details of a specific breakpoint rule by ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Breakpoint rule ID |

### madhyamas_delete_breakpoint

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Breakpoint rule ID |

### madhyamas_list_paused_traffic

List all traffic currently paused by breakpoints. Paused traffic can be inspected and then resumed (continued or aborted). No parameters.

Example: `madhyamas_list_paused_traffic()`

### madhyamas_get_paused_item

Get details of a specific paused traffic item by ID, including request headers and body.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Paused item ID |

Example: `madhyamas_get_paused_item(id="abc123")`

### madhyamas_resume_paused_item

Resume a paused traffic item. Use action='continue' to allow the request to proceed, or action='abort' to abort it.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Paused item ID |
| `action` | string | Yes | Action to take. Enum: `continue`, `abort` |

Example: `madhyamas_resume_paused_item(id="abc123", action="continue")`

## Replay (6 tools)

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

### madhyamas_replay_advanced

Replay a saved request multiple times with concurrency, iterations, and inter-request delay (batch/advanced replay). Returns aggregate statistics including success/failure counts and latency percentiles (min/avg/max/p95). Useful for basic load testing and performance benchmarking. Safety limits: iterations capped at 10,000 and concurrency at 100.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the saved request to replay |
| `iterations` | integer | Yes | Total number of requests to send (max 10,000, default: 1) |
| `concurrency` | integer | Yes | Number of simultaneous in-flight requests (max 100, default: 1) |
| `delay_ms` | integer | No | Optional delay between requests in milliseconds |
| `modifications` | object | No | Optional modifications to apply before replaying (applied to all iterations). Same shape as `madhyamas_replay_request`. |

Example: `madhyamas_replay_advanced(id="abc123", iterations=100, concurrency=10, delay_ms=50)`

### madhyamas_clear_replay_history

Clear all replay history entries. No parameters.

Example: `madhyamas_clear_replay_history()`

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

## Rewrites (7 tools)

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

### madhyamas_update_rewrite

Update an existing rewrite rule. The id, created_at, and hit_count fields are preserved.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Rewrite rule ID |
| `name` | string | Yes | Rule name |
| `condition` | object | Yes | Match condition |
| `direction` | string | Yes | Apply direction. Enum: `request`, `response`, `both` |
| `rewrites` | array[object] | Yes | Rewrite actions |
| `enabled` | boolean | No | Enable or disable |
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

### madhyamas_batch_toggle_rewrites

Enable or disable multiple rewrite rules in a single request.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `ids` | array[string] | Yes | List of rewrite rule IDs to toggle |
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

## Scripts (16 tools)

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

### madhyamas_test_script

Test (dry-run) a script against a sample request/response context without affecting live traffic or recording history.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | Yes | The script source code to test |
| `hook` | string | Yes | Hook to test against. Enum: `on_request`, `on_response`, `on_websocket_message`, `on_grpc_message`, `on_traffic_store`, `on_session_start`, `on_session_end` |

Example: `madhyamas_test_script(source="console.log(request.url)", hook="on_request")`

### madhyamas_validate_script

Validate a script's syntax without executing it. Returns whether the source is valid and any parse errors.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | Yes | The script source code to validate |

Example: `madhyamas_validate_script(source="console.log('hello')")`

### madhyamas_get_script_history

Get execution history for a specific script, showing recent runs with success/failure status, duration, console output, and errors.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the script to get history for |
| `limit` | integer | No | Maximum number of history entries to return (default: 50) |

Example: `madhyamas_get_script_history(id="abc123", limit=20)`

### madhyamas_get_script_history_all

Get execution history across all scripts, showing recent runs with success/failure status, duration, and errors. No parameters.

Example: `madhyamas_get_script_history_all()`

### madhyamas_clear_script_history

Clear execution history for a specific script.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Script ID |

Example: `madhyamas_clear_script_history(id="abc123")`

### madhyamas_reorder_script

Reorder a script by changing its priority. Lower priority values run earlier in the script chain.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Script ID |
| `priority` | integer | Yes | New priority position |

Example: `madhyamas_reorder_script(id="abc123", priority=10)`

### madhyamas_script_match_preview

Preview which scripts would match a given request without actually executing them.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `url` | string | Yes | URL to test |
| `method` | string | No | HTTP method (default: GET) |

Example: `madhyamas_script_match_preview(url="https://api.example.com/users")`

### madhyamas_get_script_config

Get the global script runtime configuration (timeout, memory limit, console capture settings). No parameters.

Example: `madhyamas_get_script_config()`

### madhyamas_update_script_config

Update the global script runtime configuration (timeout, memory limit, console capture). Only provided fields are updated.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `timeout_ms` | integer | No | Execution timeout in milliseconds |
| `memory_limit_mb` | integer | No | Memory limit in MB |
| `capture_console` | boolean | No | Enable console output capture |

Example: `madhyamas_update_script_config(timeout_ms=5000, capture_console=true)`

## Plugins (21 tools)

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

### madhyamas_install_plugin

Install a plugin from a URL or registry id.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | No | Install source: `url` or `registry` (default: `url`) |
| `target` | string | Yes | Plugin URL (source=url) or registry id (source=registry) |
| `checksum` | string | No | Expected SHA-256 checksum (optional for URL source) |

Example: `madhyamas_install_plugin(source="registry", target="cors-helper")`

### madhyamas_uninstall_plugin

Uninstall a plugin (removes from disk and persistence).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the plugin to uninstall |

Example: `madhyamas_uninstall_plugin(id="cors-helper")`

### madhyamas_search_registry

Search the plugin registry by name, description, or tags.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query |

Example: `madhyamas_search_registry(query="cors")`

### madhyamas_list_registry

List all available plugins in the registry. No parameters.

Example: `madhyamas_list_registry()`

### madhyamas_get_plugin_schema

Get a plugin's settings schema (for UI generation).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the plugin |

Example: `madhyamas_get_plugin_schema(id="cors-helper")`

### madhyamas_get_plugin_settings

Get a plugin's current settings.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the plugin |

Example: `madhyamas_get_plugin_settings(id="cors-helper")`

### madhyamas_update_plugin_settings

Update a plugin's settings.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the plugin |
| `settings` | object | Yes | Settings as a JSON object |

Example: `madhyamas_update_plugin_settings(id="cors-helper", settings={"allowed_origins":["*"]})`

### madhyamas_get_plugin_logs

Get a plugin's recent invocation logs.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the plugin |
| `limit` | integer | No | Maximum number of log entries (default 50) |

Example: `madhyamas_get_plugin_logs(id="cors-helper", limit=20)`

### madhyamas_get_plugin_panels

Get a plugin's declarative UI panels (custom UI components defined by the plugin manifest).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Plugin ID |

Example: `madhyamas_get_plugin_panels(id="cors-helper")`

### madhyamas_get_plugin_templates

List available plugin scaffolding templates (basic, cors, request-logger, domain-blocker, response-modifier). No parameters.

Example: `madhyamas_get_plugin_templates()`

### madhyamas_get_registry_config

Get the current plugin registry configuration (GitHub repo and cache settings). No parameters.

Example: `madhyamas_get_registry_config()`

### madhyamas_update_registry_config

Update the plugin registry repository configuration.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `repo` | string | Yes | GitHub repo (owner/repo format) |

Example: `madhyamas_update_registry_config(repo="ShristiLabs/madhyamas-plugins")`

### madhyamas_refresh_registry

Force-refresh the plugin registry cache from the configured GitHub repository. No parameters.

Example: `madhyamas_refresh_registry()`

## Auto Save (3 tools)

### madhyamas_get_autosave_config

Get the current Auto Save configuration (enabled, interval, export format, output directory, max backups, rotation settings). No parameters.

Example: `madhyamas_get_autosave_config()`

### madhyamas_update_autosave_config

Update the Auto Save configuration. Only provided fields are updated. Auto Save periodically exports the current session as HAR or Session format to a backup directory.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `enabled` | boolean | No | Enable or disable Auto Save |
| `interval_seconds` | integer | No | Seconds between snapshots |
| `export_format` | string | No | Export format. Enum: `har`, `session` |
| `output_dir` | string | No | Backup directory path |
| `max_backups` | integer | No | Number of backups to keep |
| `rotate_after_requests` | integer | No | Rotate session after N requests |
| `rotate_after_minutes` | integer | No | Rotate session after N minutes |

Example: `madhyamas_update_autosave_config(enabled=true, interval_seconds=300, export_format="har")`

### madhyamas_trigger_autosave_snapshot

Trigger an immediate Auto Save snapshot (save now) without waiting for the next interval. No parameters.

Example: `madhyamas_trigger_autosave_snapshot()`

## Block List (7 tools)

### madhyamas_list_blocklist

List all block list entries. Block list entries block requests matching a domain/pattern and return a configurable response instead of forwarding upstream. No parameters.

Example: `madhyamas_list_blocklist()`

### madhyamas_get_blocklist_stats

Get block list summary statistics (total entries, enabled count, total hits). No parameters.

Example: `madhyamas_get_blocklist_stats()`

### madhyamas_create_blocklist_entry

Create a block list entry to block requests matching a domain or pattern. Supports exact domains, wildcard subdomains (*.example.com), and globs (*ads*). Returns a configurable status code (default 403).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pattern` | string | Yes | Domain or wildcard pattern to block |
| `note` | string | No | Optional note describing why this entry exists |
| `enabled` | boolean | No | Whether the entry is enabled (default: true) |
| `status_code` | integer | No | HTTP status code to return (default: 403) |
| `response_body` | string | No | Response body to return when blocked |
| `content_type` | string | No | Content-Type header for the block response |

Example: `madhyamas_create_blocklist_entry(pattern="*.ads.example.com", status_code=403)`

### madhyamas_get_blocklist_entry

Get details of a specific block list entry by ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Block list entry ID |

Example: `madhyamas_get_blocklist_entry(id="abc123")`

### madhyamas_update_blocklist_entry

Update an existing block list entry. Provide the full entry object with modified fields.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Block list entry ID |
| `entry` | object | Yes | Full block list entry object with updates |

Example: `madhyamas_update_blocklist_entry(id="abc123", entry={"pattern":"*.ads.com","enabled":false})`

### madhyamas_delete_blocklist_entry

Delete a block list entry by ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Block list entry ID |

Example: `madhyamas_delete_blocklist_entry(id="abc123")`

### madhyamas_toggle_blocklist_entry

Enable or disable a block list entry.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Block list entry ID |
| `enabled` | boolean | Yes | true to enable, false to disable |

Example: `madhyamas_toggle_blocklist_entry(id="abc123", enabled=false)`

## Focus Hosts (4 tools)

### madhyamas_list_focus_hosts

List all focus host patterns. Focused hosts are highlighted in the traffic view. Patterns support exact hostnames, wildcard subdomains (*.example.com), and globs (*api*). No parameters.

Example: `madhyamas_list_focus_hosts()`

### madhyamas_add_focus_host

Add a focus host pattern to highlight matching traffic. Supports exact hostnames (api.example.com), wildcard subdomains (*.example.com), and globs (*api*).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pattern` | string | Yes | Host pattern to focus on |

Example: `madhyamas_add_focus_host(pattern="*.api.example.com")`

### madhyamas_remove_focus_host

Remove a focus host pattern by its ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Focus host ID to remove |

Example: `madhyamas_remove_focus_host(id="abc123")`

### madhyamas_clear_focus_hosts

Clear all focus host patterns. No parameters.

Example: `madhyamas_clear_focus_hosts()`

## Mirror (3 tools)

### madhyamas_get_mirror_status

Get the current mirror tool status, configuration, and statistics (files written, bytes written). The mirror tool saves response bodies to disk following the URL path structure. No parameters.

Example: `madhyamas_get_mirror_status()`

### madhyamas_toggle_mirror

Toggle the mirror tool on or off. When enabled, response bodies are saved to disk following the URL path structure.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `enabled` | boolean | Yes | Whether to enable (true) or disable (false) mirroring |

Example: `madhyamas_toggle_mirror(enabled=true)`

### madhyamas_update_mirror_config

Update the mirror tool configuration (output directory, host filter, save request bodies). Only provided fields are updated.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `enabled` | boolean | No | Enable or disable mirroring |
| `output_dir` | string | No | Directory where mirrored files are written |
| `host_filter` | array[string] | No | Host patterns to mirror (empty or null for all hosts) |
| `save_request_bodies` | boolean | No | Whether to also save request bodies |

Example: `madhyamas_update_mirror_config(output_dir="/tmp/mirror", host_filter=["api.example.com"])`

## Logs (3 tools)

### madhyamas_get_log_status

Get the current log rotation status: configuration, current log file path and size, and the list of archived (rotated) log files. No parameters.

Example: `madhyamas_get_log_status()`

### madhyamas_rotate_logs

Rotate the current log file immediately (on-demand). The current madhyamas.log is renamed with a timestamp suffix and a fresh file is opened. Archived files are pruned to max_files. No parameters.

Example: `madhyamas_rotate_logs()`

### madhyamas_update_log_config

Update the log rotation configuration (enabled, rotation mode, max_files, max_file_size_mb, json_format). Only provided fields are updated. Rotation mode changes take effect on next restart; size/max_files take effect immediately.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `enabled` | boolean | No | Enable or disable file logging |
| `rotation` | object | No | Rotation mode: `{"mode":"never"|"hourly"|"daily"}` or `{"mode":"size","size_mb":<n>}` |
| `max_files` | integer | No | Maximum number of archived log files to keep |
| `max_file_size_mb` | integer | No | Hard per-file size cap in MB (safety net for time-based rotation) |
| `json_format` | boolean | No | Use structured JSON log format (takes effect on restart) |

Example: `madhyamas_update_log_config(rotation={"mode":"daily"}, max_files=10)`

## WebSocket Traffic (4 tools)

### madhyamas_list_ws_connections

List all captured WebSocket connections observed by the proxy. No parameters.

Example: `madhyamas_list_ws_connections()`

### madhyamas_get_ws_connection

Get details of a specific WebSocket connection by ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | WebSocket connection ID |

Example: `madhyamas_get_ws_connection(id="abc123")`

### madhyamas_get_ws_messages

Get captured WebSocket messages with optional filtering by connection ID, direction, message type, and text search.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `connection_id` | string | No | Filter by connection ID |
| `direction` | string | No | Filter by direction. Enum: `send`, `receive` |
| `message_type` | string | No | Filter by message type. Enum: `text`, `binary`, `ping`, `pong`, `close` |
| `search` | string | No | Search in message payloads |
| `limit` | integer | No | Maximum number of results |
| `offset` | integer | No | Offset for pagination |

Example: `madhyamas_get_ws_messages(connection_id="abc123", direction="send", limit=50)`

### madhyamas_clear_ws_traffic

Clear all captured WebSocket messages and closed connections. This action cannot be undone. No parameters.

Example: `madhyamas_clear_ws_traffic()`

## Certificate (1 tool)

### madhyamas_get_cert_info

Get the proxy's CA certificate details (subject, issuer, validity, download URL) needed to configure browsers/clients for HTTPS interception. No parameters.

Example: `madhyamas_get_cert_info()`

## MCP Resources

The MCP server also exposes read-only resources:

| Resource URI | Description |
|--------------|-------------|
| `madhyamas://traffic` | All captured traffic |
| `madhyamas://sessions` | All debugging sessions |
| `madhyamas://config` | Proxy configuration |
| `madhyamas://session/{id}` | Details of a specific session |
| `madhyamas://traffic/{id}` | Details of a specific traffic entry |
| `madhyamas://mock/{id}` | Details of a specific mock rule |

## MCP Prompts

The MCP server exposes debugging prompts that inject API context:

| Prompt | Description | Arguments |
|--------|-------------|-----------|
| `debug-4xx` | Analyze recent 4xx responses and suggest fixes | None |
| `debug-5xx` | Analyze recent 5xx responses and identify root causes | None |
| `find-auth-issues` | Check for authentication-related issues in recent traffic | None |
| `mock-missing-endpoint` | Create a mock for a missing endpoint found in 404 responses | None |
| `compare-staging-prod` | Compare traffic between two sessions | `session1`, `session2` (required) |
| `audit-trail` | Show audit trail for a specific user or time period | `user_id` (optional) |

## Enterprise Tools (11 tools)

These tools are registered when the MCP server detects an enterprise-tier API server (via `/api/health/detailed`). Against an OSS server they are not registered.

### madhyamas_list_users

List all registered users (enterprise tier). Requires admin permission. No parameters.

Example: `madhyamas_list_users()`

### madhyamas_create_user

Create a new user account (enterprise tier). Requires admin permission.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `username` | string | Yes | Username for the new user |
| `email` | string | Yes | Email address |
| `password` | string | Yes | Initial password |
| `role` | string | Yes | User role. Enum: `admin`, `user`, `viewer` |

Example: `madhyamas_create_user(username="alice", email="alice@example.com", password="secret", role="user")`

### madhyamas_delete_user

Delete a user account by ID (enterprise tier). Requires admin permission.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the user to delete |

Example: `madhyamas_delete_user(id="abc123")`

### madhyamas_update_user_role

Update a user's role by ID (enterprise tier). Requires admin permission.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | The ID of the user to update |
| `role` | string | Yes | New role. Enum: `admin`, `user`, `viewer` |

Example: `madhyamas_update_user_role(id="abc123", role="admin")`

### madhyamas_get_audit_events

Query audit events with optional filters (enterprise tier).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `user_id` | string | No | Filter by user ID |
| `event_type` | string | No | Filter by event type |
| `limit` | integer | No | Maximum results (default: 100) |
| `offset` | integer | No | Pagination offset |

Example: `madhyamas_get_audit_events(user_id="abc123", limit=50)`

### madhyamas_export_audit

Export all audit events (enterprise tier). Returns a JSON document. No parameters.

Example: `madhyamas_export_audit()`

### madhyamas_get_license_info

Get the current license status and details (enterprise tier). No parameters.

Example: `madhyamas_get_license_info()`

### madhyamas_get_metrics

Get current performance and operational metrics (enterprise tier). No parameters.

Example: `madhyamas_get_metrics()`

### madhyamas_get_health

Get detailed health status including tier, license, and dependency checks. No parameters.

Example: `madhyamas_get_health()`

### madhyamas_export_config

Export the full Madhyamas configuration as JSON (enterprise tier). No parameters.

Example: `madhyamas_export_config()`

### madhyamas_import_config

Import a configuration JSON document (enterprise tier).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `config_json` | object | Yes | The configuration JSON to import |

Example: `madhyamas_import_config(config_json={"version":"0.1.6","settings":{}})`

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MADHYAMAS_API_URL` | `http://127.0.0.1:3001` | API endpoint for the running proxy |
| `MADHYAMAS_TIMEOUT` | `30` | Request timeout in seconds |
| `RUST_LOG` | `info` | Logging level |

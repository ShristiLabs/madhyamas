# Plugin API Reference

## Host ↔ Guest ABI

The Madhyamas WASM plugin ABI defines the contract between the host
(`WasmRuntime` in `madhyamas-core`) and the guest (a `.wasm` module built
with `madhyamas-plugin-sdk`).

### Guest Exports

The guest module must export:

#### `__madhyamas_alloc(size: i32) -> i32`

Bump-allocate `size` bytes in the guest's linear memory and return the
pointer (offset). The host calls this to allocate space for writing the
JSON-serialized `PluginContext` before calling `__madhyamas_hook`.

- **Input**: `size` — number of bytes to allocate
- **Output**: pointer (offset into linear memory), or `0` on failure
- **Called by**: host, before each hook invocation

#### `__madhyamas_hook(hook_id: i32, ctx_ptr: i32, ctx_len: i32) -> i64`

Dispatch the hook. The host has already written the JSON-serialized
`PluginContext` at `[ctx_ptr, ctx_ptr + ctx_len)` in the guest's linear
memory.

- **Input**:
  - `hook_id` — the hook index (see [Hook IDs](#hook-ids))
  - `ctx_ptr` — pointer to the context JSON bytes
  - `ctx_len` — length of the context JSON bytes
- **Output**: packed `(result_ptr << 32) | result_len` where the pointed-to
  bytes are a JSON-serialized `PluginResult`. A return of `0` means "no
  handler / continue".

### Host Imports (WASM module `env`)

The host provides:

#### `log(level: i32, ptr: i32, len: i32)`

Emit a log line. The guest passes a string from its linear memory.

- **Input**:
  - `level` — `0` = ERROR, `1` = WARN, `2` = INFO, `3` = DEBUG
  - `ptr` — pointer to the string bytes in linear memory
  - `len` — string length

Log lines are collected per-invocation and stored in the invocation audit
log (visible via `GET /api/plugins/{id}/logs`).

## Hook IDs

| ID | Hook | Constant |
|----|------|----------|
| 0 | `on_load` | `HOOK_ON_LOAD` |
| 1 | `on_enable` | `HOOK_ON_ENABLE` |
| 2 | `on_disable` | `HOOK_ON_DISABLE` |
| 3 | `on_unload` | `HOOK_ON_UNLOAD` |
| 4 | `on_request` | `HOOK_ON_REQUEST` |
| 5 | `on_response` | `HOOK_ON_RESPONSE` |
| 6 | `on_websocket` | `HOOK_ON_WEBSOCKET` |
| 7 | `on_grpc` | `HOOK_ON_GRPC` |
| 8 | `on_settings_change` | `HOOK_ON_SETTINGS_CHANGE` |
| 9 | `on_timer` | `HOOK_ON_TIMER` |

## Wire Format (JSON)

### PluginContext

```json
{
  "plugin_id": "com.example.my-plugin",
  "request_id": "abc-123",
  "session_id": "sess-456",
  "hook": "on_request",
  "request": {
    "method": "GET",
    "url": "http://example.com/path",
    "host": "example.com",
    "path": "/path",
    "headers": { "Content-Type": "application/json" },
    "body": null,
    "content_type": null
  },
  "response": null,
  "settings": { "key": "value" },
  "state": {},
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### PluginResult / Outcome

```json
{
  "handled": false,
  "continue_": true,
  "modified": true,
  "request": null,
  "response": {
    "status_code": 200,
    "status_message": "OK",
    "headers": { "X-Custom": "value" },
    "body": null,
    "content_type": null,
    "duration_ms": 0
  },
  "error": null,
  "logs": ["[INFO] my-plugin loaded"],
  "custom_response": null
}
```

Fields:
- `handled` — if `true`, the plugin handled the request; stop the chain.
- `continue_` — if `false`, stop the chain (even if not handled).
- `modified` — if `true`, the `request`/`response` fields contain
  modifications to apply.
- `request` — modified request (applied when `modified` is `true`).
- `response` — modified response (applied when `modified` is `true`).
- `error` — error message (stops the chain).
- `logs` — log lines emitted by the guest.
- `custom_response` — a short-circuit response (used when `handled` is
  `true` to return a custom response without forwarding upstream).

## Resource Limits

| Limit | Default | Configurable via |
|-------|---------|------------------|
| Fuel (CPU instructions) | 10,000,000 | `fuel_limit` in manifest |
| Linear memory | 256 MiB (host ceiling) | `max_memory_pages` in manifest |
| Network access | Not linked | `network` in manifest (declared-only in v1) |

When a plugin exhausts its fuel budget, the invocation traps and is reported
as an error in the invocation log.

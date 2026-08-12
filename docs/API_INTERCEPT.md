# API — Intercept Pipeline

Endpoints for the intercept pipeline: breakpoints, mocks, rewrites, throttle,
block list, focus, and replay. Base path: `/api`. See
[INTERCEPT_PIPELINE.md](INTERCEPT_PIPELINE.md) for the priority model and
[EXTENSION_SYSTEM.md](EXTENSION_SYSTEM.md) for the extension layer.

## Breakpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/breakpoints` | List breakpoint rules |
| POST | `/breakpoints` | Create a breakpoint rule |
| GET | `/breakpoints/{id}` | Get a breakpoint rule |
| DELETE | `/breakpoints/{id}` | Delete a breakpoint rule |
| GET | `/breakpoints/paused` | List paused traffic awaiting a breakpoint decision |
| GET | `/breakpoints/paused/{id}` | Get a paused item |
| POST | `/breakpoints/paused/{id}/resume` | Resume a paused request with a decision |

## Mocks

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/mocks` | List mock rules |
| POST | `/mocks` | Create a mock rule |
| GET | `/mocks/templates` | List built-in mock templates |
| GET | `/mocks/{id}` | Get a mock rule |
| PUT | `/mocks/{id}` | Update a mock rule |
| DELETE | `/mocks/{id}` | Delete a mock rule |
| POST | `/mocks/{id}/toggle` | Enable/disable a mock |
| POST | `/mocks/batch-toggle` | Toggle multiple mocks at once |
| POST | `/mocks/{id}/test` | Test a mock rule against a sample request |
| POST | `/mocks/preview` | Preview which mock would match a request |
| POST | `/mocks/{id}/duplicate` | Duplicate a mock rule |
| POST | `/mocks/{id}/rollback` | Roll back a mock rule to a prior version |
| GET | `/mocks/{id}/versions` | Get version history for a mock rule |
| POST | `/mocks/advanced` | Create an advanced mock (sequence/conditional/probabilistic) |
| GET | `/mocks/export` | Export all mock rules as JSON |
| POST | `/mocks/import` | Import mock rules from JSON |

### Mock Collections

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/mocks/collections` | List mock collections |
| POST | `/mocks/collections` | Create a collection |
| GET | `/mocks/collections/{id}` | Get a collection |
| PUT | `/mocks/collections/{id}` | Update a collection |
| DELETE | `/mocks/collections/{id}` | Delete a collection |
| POST | `/mocks/collections/{id}/toggle` | Enable/disable a collection |

### Mock Recording

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/mocks/recording` | Start/stop mock recording |
| GET | `/mocks/recording/status` | Get recording status |
| GET | `/mocks/recording/recorded` | List recorded mocks |
| POST | `/mocks/recording/promote` | Promote recorded mocks to permanent rules |
| POST | `/mocks/recording/clear` | Clear recorded mocks |

### Mock Analytics

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/mocks/analytics` | Aggregate mock analytics |
| GET | `/mocks/{id}/analytics` | Analytics for a single mock |
| GET | `/mocks/{id}/history` | Hit history for a mock |
| POST | `/mocks/history/clear` | Clear mock hit history |

## Rewrites

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/rewrites` | List rewrite rules |
| POST | `/rewrites` | Create a rewrite rule |
| GET | `/rewrites/templates` | List built-in rewrite templates (see [REWRITE_TEMPLATES.md](REWRITE_TEMPLATES.md)) |
| GET | `/rewrites/{id}` | Get a rewrite rule |
| PUT | `/rewrites/{id}` | Update a rewrite rule |
| DELETE | `/rewrites/{id}` | Delete a rewrite rule |
| POST | `/rewrites/{id}/toggle` | Enable/disable a rewrite |
| POST | `/rewrites/batch-toggle` | Toggle multiple rewrites at once |

## Throttle

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/throttle` | Get the current throttle profile |
| POST | `/throttle` | Set the throttle profile |
| POST | `/throttle/enabled` | Enable/disable throttling |
| GET | `/throttle/presets` | List throttle presets (e.g. 3G, 4G, DSL) |

## Block List

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/blocklist` | List block list entries |
| POST | `/blocklist` | Create a block list entry |
| GET | `/blocklist/stats` | Get block list statistics |
| GET | `/blocklist/{id}` | Get a block list entry |
| PUT | `/blocklist/{id}` | Update a block list entry |
| DELETE | `/blocklist/{id}` | Delete a block list entry |
| POST | `/blocklist/{id}/toggle` | Enable/disable an entry |

See [BLOCK_LIST.md](BLOCK_LIST.md) for the feature guide.

## Focus

Focus hosts are a visual emphasis feature (not a filter). See [FOCUS.md](FOCUS.md).

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/focus` | List focus hosts |
| POST | `/focus` | Add a focus host |
| DELETE | `/focus/{id}` | Remove a focus host |
| DELETE | `/focus` | Clear all focus hosts |

## Replay

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/replay/saved` | List saved requests |
| POST | `/replay/saved` | Save a request |
| GET | `/replay/saved/{id}` | Get a saved request |
| DELETE | `/replay/saved/{id}` | Delete a saved request |
| POST | `/replay/execute/{id}` | Replay a saved request |
| POST | `/replay/execute/{id}/batch` | Batch replay (iterations/concurrency/delay — see [REPEAT_ADVANCED.md](REPEAT_ADVANCED.md)) |
| GET | `/replay/history` | View replay history |
| DELETE | `/replay/history` | Clear replay history |

See also [EDIT_THEN_REPEAT.md](EDIT_THEN_REPEAT.md) for edit-then-repeat.

## See Also

- [API.md](API.md) — API index
- [INTERCEPT_PIPELINE.md](INTERCEPT_PIPELINE.md) — Intercept handler trait and priority model
- [BLOCK_LIST.md](BLOCK_LIST.md) — Block list feature
- [REWRITE_TEMPLATES.md](REWRITE_TEMPLATES.md) — Built-in rewrite templates
- [REPEAT_ADVANCED.md](REPEAT_ADVANCED.md) — Batch replay
- [EDIT_THEN_REPEAT.md](EDIT_THEN_REPEAT.md) — Edit-then-repeat

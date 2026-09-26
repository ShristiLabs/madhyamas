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

## Device-Scoped Rules (Enterprise)

Every rule type above — mock rules, rewrite rules, breakpoint rules, block
list entries, and the throttle profile — carries an optional
`device_id` scope (issue #109):

- `device_id: null` (or absent) — **user-global** rule: applies to every
  request, attributed or not. This is the only kind of rule the OSS tier
  ever creates, and pre-#109 rules keep exactly this behavior.
- `device_id: "<device-id>"` — **device-scoped** rule: matches only
  requests authenticated with that device's key (`mdy_dev_...`). Every
  other device — and unauthenticated traffic — flows through untouched.
  A device-scoped breakpoint pauses only the bound device's request.

Create (and rewrite update) bodies accept `device_id`. The rule list
responses include it. See
[CREDENTIAL_ONBOARDING.md](CREDENTIAL_ONBOARDING.md) for the
credential model.

### Default scoping for agent keys

Device-derived agent keys (`mdy_agent_...`, issue #108) are pinned to
their parent device on the data axis:

| Principal | Create | List / get | Update / delete / toggle |
|---|---|---|---|
| Agent key (device X) | omitted `device_id` defaults to **X**; explicit `null` → `403`; foreign device → `403` | X-scoped rules only | X-scoped rules only; global/other-device → `404` |
| Owner JWT / user API key | any scope (absent = global) | all rules | all rules |

Batch toggles apply to the agent's own rules only (foreign/global IDs
report as `not_found`). `GET /mocks/export` exports only the agent's
rules; `POST /mocks/import` and recorded-mock promotion land every rule
in the agent's device namespace. Mock hit analytics (`/mocks/analytics`,
per-rule stats/history) are filtered the same way, and clearing hit
history across all rules is owner/JWT-only. The capability axis from
issue #107 (which rule *types* a key may touch) applies unchanged on top.

**Update semantics:** full-replace updates (`PUT /mocks/{id}`,
`PUT /blocklist/{id}`) cannot *clear* a device scope — an absent or
`null` `device_id` in the body preserves the existing scope, and agents
are pinned to their parent device regardless. To globalize a scoped mock
or block-list entry the owner recreates it; rewrite rules (whose update
request carries the tri-state `device_id` field) can change scope freely.

Mock collections remain global grouping objects; an agent key cannot
toggle a collection or delete one with `delete_rules: true` (both would
flip/delete member rules across scopes) — it deletes rules individually.

### Throttle singleton semantics

The throttle profile is a single active row shared by all scopes.
Setting a profile **replaces** whatever profile was active — global
included; the owner can re-set a global profile at any time. An agent
key always writes its parent-device scope, sees the profile only when it
is scoped to its device (otherwise a `None` profile / disabled), and
`POST /throttle/enabled` is `403` for an agent while the active profile
is global (toggling it would mutate a rule the agent cannot see).

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

# API — Config, Capture & Operations

Runtime configuration, capture control, and operational endpoints (auto save,
mirror, logs, persistence, health). Base path: `/api`.

## Config

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/config` | Get the proxy configuration |
| PATCH | `/config` | Update the proxy configuration (live; no restart needed) |

The config includes proxy ports, host, upstream proxy settings, SOCKS settings,
access control, recording limits, and more. See [ARCHITECTURE.md](ARCHITECTURE.md)
for the full config surface.

## Capture

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/capture` | Get capture status (enabled/disabled, passthrough mode) |
| POST | `/capture/toggle` | Toggle traffic capture on/off |
| GET | `/capture/stats` | Get capture statistics (counts, sizes, quota) |

See [RECORDING_LIMITS.md](RECORDING_LIMITS.md) for recording size limits and
FIFO pruning.

## Auto Save

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/autosave` | Get auto-save configuration |
| PATCH | `/autosave` | Update auto-save configuration |
| POST | `/autosave/snapshot` | Trigger an immediate auto-save snapshot |

See [AUTO_SAVE.md](AUTO_SAVE.md) for the feature guide.

## Mirror

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/mirror` | Get mirror tool status |
| POST | `/mirror/toggle` | Enable/disable the mirror tool |
| PATCH | `/mirror/config` | Update mirror configuration |

See [MIRROR.md](MIRROR.md) for the feature guide.

## Logs

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/logs` | Get log status and configuration |
| PATCH | `/logs` | Update log configuration |
| POST | `/logs/rotate` | Trigger immediate log rotation |

See [LOGGING.md](LOGGING.md) for the log rotation feature.

## Persistence

Save and load all intercept rules (mocks, rewrites, breakpoints, throttle,
block list) to/from a single bundle.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/persistence/export` | Export all rules as JSON |
| POST | `/persistence/import` | Import rules from JSON |
| POST | `/persistence/save` | Save all rules to the intercept store |
| POST | `/persistence/load` | Load all rules from the intercept store |

See [PERSISTENCE.md](PERSISTENCE.md) for the persistence layer internals.

## Health

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Simple health check (returns `OK`) |

A detailed health check (`/health/detailed`) is available with the enterprise
feature — see [API_ENTERPRISE.md](API_ENTERPRISE.md).

## See Also

- [API.md](API.md) — API index
- [AUTO_SAVE.md](AUTO_SAVE.md) — Auto save feature
- [MIRROR.md](MIRROR.md) — Mirror tool
- [LOGGING.md](LOGGING.md) — Log rotation
- [RECORDING_LIMITS.md](RECORDING_LIMITS.md) — Recording limits
- [PERSISTENCE.md](PERSISTENCE.md) — Persistence layer

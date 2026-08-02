# Auto Save

Periodic session backup and rotation for disaster recovery.

## Overview

Madhyamas stores traffic in SQLite in real time — every request/response is
persisted immediately. Auto Save is **not** the primary persistence
mechanism. Instead, it provides an additional safety net:

- **Periodic HAR/Session export** to a backup directory for disaster recovery
  (e.g. if the SQLite database is corrupted or accidentally deleted).
- **Automatic session rotation** — start a new session after N requests or M
  minutes, archiving the old one.
- **Backup pruning** — keep only the last N backup files, deleting the oldest
  first.

A background task (`tokio::time::interval`) runs the export on a configurable
schedule. The task is started only when Auto Save is enabled and is stopped
gracefully (via a `oneshot` channel) on shutdown.

## Configuration

Auto Save is configured via the `auto_save` field of `ProxyConfig`. It is
**disabled by default**.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `false` | Master switch. When `false`, no periodic export or rotation runs. |
| `interval_seconds` | `u64` | `300` (5 min) | Seconds between snapshots. |
| `export_format` | `String` | `"har"` | Export format: `"har"` (HAR 1.2, interoperable) or `"session"` (Madhyamas-native `SessionExport` JSON, restorable via import). |
| `output_dir` | `String` | `~/.madhyamas/backups` | Directory where backup files are written. Created if it doesn't exist. |
| `max_backups` | `usize` | `10` | Maximum number of backup files to keep. Oldest are deleted after each snapshot. |
| `rotate_after_requests` | `Option<usize>` | `None` | When set, rotate (start a new session) after this many requests. `None` disables. |
| `rotate_after_minutes` | `Option<u64>` | `None` | When set, rotate after this many minutes since the session started. `None` disables. |

### Config File Example

```json
{
  "auto_save": {
    "enabled": true,
    "interval_seconds": 600,
    "export_format": "har",
    "output_dir": "/var/backups/madhyamas",
    "max_backups": 24,
    "rotate_after_requests": 5000,
    "rotate_after_minutes": 60
  }
}
```

## Backup File Naming

Files are named `session-<YYYYMMDD-HHMMSS>.<ext>`:

- HAR format → `session-20240115-143022.har`
- Session format → `session-20240115-143022.json`

The timestamp uses UTC.

## How Pruning Works

After each snapshot, the manager scans the output directory for files
matching the `session-*` prefix, sorts them by modification time (oldest
first), and deletes any files beyond `max_backups`. For example, with
`max_backups = 10` and 12 files present, the 2 oldest are deleted.

## How Session Rotation Works

If `rotate_after_requests` or `rotate_after_minutes` is set, the manager
checks the threshold at the start of each cycle (before exporting):

1. **Request-based**: if the current session's request count ≥
   `rotate_after_requests`, a new session is created and switched to.
2. **Time-based**: if the elapsed time since the session was created ≥
   `rotate_after_minutes`, a new session is created and switched to.

The old session remains in the SQLite database (it is not deleted) and is
exported as part of the snapshot. Subsequent traffic is recorded against the
new session.

## Configuring via Web UI

Open **Config → Auto Save** tab in the web UI:

1. Toggle **Enable Auto Save**.
2. Set the **Interval** (seconds between snapshots).
3. Choose the **Export Format** (HAR or Session).
4. Set the **Output Directory**.
5. Set **Max Backups** (how many files to keep).
6. Optionally set **Rotate After Requests** and/or **Rotate After Minutes**.
7. Click **Save Changes**.
8. Click **Save Now** to trigger an immediate snapshot.

## Configuring via CLI

```bash
# View current Auto Save config
madhyamas autosave get

# Enable Auto Save with 10-minute interval, HAR format
madhyamas autosave update --enabled true --interval-seconds 600 --export-format har

# Set output directory and max backups
madhyamas autosave update --output-dir /var/backups/madhyamas --max-backups 24

# Enable session rotation after 5000 requests
madhyamas autosave update --rotate-after-requests 5000

# Disable rotation (pass 0)
madhyamas autosave update --rotate-after-requests 0

# Trigger an immediate snapshot
madhyamas autosave snapshot
```

## Configuring via API

### Get Auto Save config

```bash
GET /api/autosave
```

Response:
```json
{
  "enabled": true,
  "interval_seconds": 300,
  "export_format": "har",
  "output_dir": "~/.madhyamas/backups",
  "max_backups": 10,
  "rotate_after_requests": null,
  "rotate_after_minutes": null
}
```

### Update Auto Save config

```bash
PATCH /api/autosave
Content-Type: application/json

{
  "enabled": true,
  "interval_seconds": 600,
  "export_format": "session",
  "output_dir": "/var/backups/madhyamas",
  "max_backups": 20,
  "rotate_after_requests": 1000,
  "rotate_after_minutes": null
}
```

All fields are optional — only provided fields are updated. Set
`rotate_after_requests` or `rotate_after_minutes` to `null` to disable
rotation.

Changes are applied to the live `AutoSaveManager` config and persisted to
the config file so they survive restarts.

> **Note:** Enabling/disabling or changing the interval requires a restart
> for the background task to pick up the new schedule (the task reads the
> config at start time). Other fields (format, output_dir, max_backups,
> rotation thresholds) take effect on the next cycle.

### Trigger an immediate snapshot

```bash
POST /api/autosave/snapshot
```

Response:
```json
{
  "success": true,
  "message": "Snapshot saved",
  "output_dir": "/var/backups/madhyamas"
}
```

## Restoring from a Backup

### HAR format

Import the HAR file via the web UI (Sessions → Import) or CLI:

```bash
madhyamas traffic import-har /path/to/session-20240115-143022.har --name "Restored" --switch
```

### Session format

Import the Session JSON via the API:

```bash
curl -X POST http://127.0.0.1:3001/api/sessions/import \
  -H "Content-Type: application/json" \
  -d @/path/to/session-20240115-143022.json
```

## Design Notes

- **Real-time SQLite is the primary store** — Auto Save is a backup
  mechanism, not the primary persistence.
- **Graceful shutdown** — the background task uses a `tokio::sync::oneshot`
  channel for clean shutdown. The task is also stopped automatically when
  the `AutoSaveManager` is dropped.
- **Idempotent** — running multiple snapshots produces separate timestamped
  files; pruning keeps only the last N.
- **Thread-safe** — the config is shared via `Arc<RwLock<AutoSaveConfig>>`
  so the API layer can update it live without a restart.

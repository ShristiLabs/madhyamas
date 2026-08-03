# Auto Save

Auto Save provides **periodic session backup and rotation** for disaster recovery. Madhyamas already stores traffic in SQLite in real time, so Auto Save is a backup mechanism — not the primary store. It periodically exports your session to a backup directory and prunes old backups automatically.

## Why Use Auto Save?

- **Disaster recovery** — if the SQLite database is corrupted or accidentally deleted, you still have HAR/Session exports on disk.
- **Automatic session rotation** — start a new session after N requests or M minutes, archiving the old one.
- **Backup pruning** — keep only the last N backup files, deleting the oldest first.

## Configuration

Auto Save is **disabled by default**. Configure it via the web UI, CLI, or REST API.

| Option | Default | Description |
|--------|---------|-------------|
| **Enabled** | `false` | Master switch |
| **Interval** | 300s (5 min) | Seconds between snapshots |
| **Export Format** | `har` | `har` (HAR 1.2, interoperable) or `session` (Madhyamas-native, restorable via import) |
| **Output Directory** | `~/.madhyamas/backups` | Where backup files are written |
| **Max Backups** | 10 | Maximum number of backup files to keep (oldest are deleted) |
| **Rotate After Requests** | (disabled) | Rotate to a new session after this many requests |
| **Rotate After Minutes** | (disabled) | Rotate to a new session after this many minutes |

### From the Web UI

Open **Config → Auto Save** tab:

1. Toggle **Enable Auto Save**
2. Set the **Interval** (seconds between snapshots)
3. Choose the **Export Format** (HAR or Session)
4. Set the **Output Directory**
5. Set **Max Backups**
6. Optionally set **Rotate After Requests** and/or **Rotate After Minutes**
7. Click **Save Changes**
8. Click **Save Now** to trigger an immediate snapshot

### From the CLI

```bash
madhyamas autosave get                                     # View current config
madhyamas autosave update --enabled true \
  --interval-seconds 600 --export-format har               # Enable with 10-min interval
madhyamas autosave update --output-dir /var/backups/madhyamas \
  --max-backups 24                                          # Set output dir and max backups
madhyamas autosave update --rotate-after-requests 5000     # Rotate after 5000 requests
madhyamas autosave update --rotate-after-requests 0       # Disable rotation
madhyamas autosave snapshot                               # Trigger an immediate snapshot
```

### From the REST API

```bash
# Get current config
curl http://127.0.0.1:3001/api/autosave

# Update config (all fields optional)
curl -X PATCH http://127.0.0.1:3001/api/autosave \
  -H "Content-Type: application/json" \
  -d '{"enabled": true, "interval_seconds": 600, "export_format": "har", "max_backups": 20}'

# Trigger an immediate snapshot
curl -X POST http://127.0.0.1:3001/api/autosave/snapshot
```

## Backup File Naming

Files are named `session-<YYYYMMDD-HHMMSS>.<ext>` using UTC timestamps:

- HAR format → `session-20260115-143022.har`
- Session format → `session-20260115-143022.json`

## How Pruning Works

After each snapshot, the manager scans the output directory for files matching the `session-*` prefix, sorts them by modification time (oldest first), and deletes any files beyond `max_backups`. With `max_backups = 10` and 12 files present, the 2 oldest are deleted.

## How Session Rotation Works

If `rotate_after_requests` or `rotate_after_minutes` is set, the manager checks the threshold at the start of each cycle:

1. **Request-based**: if the current session's request count reaches the threshold, a new session is created and switched to.
2. **Time-based**: if the elapsed time since the session was created reaches the threshold, a new session is created and switched to.

The old session remains in the SQLite database (it's not deleted) and is exported as part of the snapshot. Subsequent traffic is recorded against the new session.

::: warning
Enabling/disabling Auto Save or changing the interval requires a **restart** for the background task to pick up the new schedule. Other fields (format, output dir, max backups, rotation thresholds) take effect on the next cycle.
:::

## Restoring from a Backup

### HAR format

Import the HAR file via the web UI (Sessions → Import) or CLI:

```bash
madhyamas traffic import-har /path/to/session-20260115-143022.har --name "Restored" --switch
```

See [Importing HAR Files](./har-import) for details.

### Session format

Import the Session JSON via the API:

```bash
curl -X POST http://127.0.0.1:3001/api/sessions/import \
  -H "Content-Type: application/json" \
  -d @/path/to/session-20260115-143022.json
```

## Common Use Cases

### Long Capture Sessions

For long-running captures (e.g. overnight testing), enable Auto Save so you always have a recent backup even if the process crashes or the machine reboots.

### Automatic Session Rotation

Set `rotate_after_minutes` to 60 to start a fresh session every hour — useful for keeping individual sessions small and organized during extended debugging.

### Compliance and Audit

Export to HAR on a schedule and keep a bounded number of backups (`max_backups`) to satisfy audit requirements without unbounded disk growth.

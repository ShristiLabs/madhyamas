# Throttling

## Overview

Simulate slow or unreliable network conditions by limiting bandwidth, adding latency, jitter, and packet loss. Useful for testing how applications behave under poor network conditions.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_get_throttle` | Get current throttle profile |
| `madhyamas_set_throttle` | Set custom throttle profile |
| `madhyamas_toggle_throttle` | Enable/disable throttling |
| `madhyamas_get_throttle_presets` | List predefined profiles |

## CLI Commands

```bash
madhyamas throttle get
madhyamas throttle set [OPTIONS]
madhyamas throttle enable
madhyamas throttle disable
madhyamas throttle presets
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/throttle` | Get current profile |
| POST | `/api/throttle` | Set profile |
| POST | `/api/throttle/enabled` | Enable/disable |
| GET | `/api/throttle/presets` | List presets |

## Workflows

### Get Current Throttle Status

**MCP:** `madhyamas_get_throttle()`

**CLI:** `madhyamas throttle get`

**REST:** `curl http://localhost:3001/api/throttle`

### List Available Presets

**MCP:** `madhyamas_get_throttle_presets()`

**CLI:** `madhyamas throttle presets`

**REST:** `curl http://localhost:3001/api/throttle/presets`

Available presets:

| Preset | Download | Upload | Latency |
|--------|----------|--------|---------|
| GPRS (2G) | 50 KB/s | 20 KB/s | 500ms |
| EDGE (2G) | 200 KB/s | 100 KB/s | 300ms |
| 3G | 1 MB/s | 500 KB/s | 100ms |
| Slow 3G | 400 KB/s | 200 KB/s | 200ms |
| 4G LTE | 10 MB/s | 5 MB/s | 30ms |
| DSL | 2 MB/s | 500 KB/s | 20ms |
| Satellite | 5 MB/s | 2 MB/s | 600ms |

### Set a Custom Throttle Profile

**MCP:**
```
madhyamas_set_throttle(
  download_bps=50000,
  upload_bps=20000,
  delay_ms=200,
  jitter_ms=50,
  packet_loss_percent=2,
  enabled=true
)
```

**CLI:** `madhyamas throttle set --download-bps 50000 --upload-bps 20000 --delay-ms 200`

**REST:**
```bash
curl -X POST http://localhost:3001/api/throttle \
  -H 'Content-Type: application/json' \
  -d '{"download_bps":50000,"upload_bps":20000,"delay_ms":200,"jitter_ms":50,"packet_loss_percent":2,"enabled":true}'
```

### Enable Throttling

**MCP:** `madhyamas_toggle_throttle(enabled=true)`

**CLI:** `madhyamas throttle enable`

**REST:** `curl -X POST http://localhost:3001/api/throttle/enabled -d '{"enabled":true}'`

### Disable Throttling

**MCP:** `madhyamas_toggle_throttle(enabled=false)`

**CLI:** `madhyamas throttle disable`

**REST:** `curl -X POST http://localhost:3001/api/throttle/enabled -d '{"enabled":false}'`

### Simulate 3G Network

Set profile matching 3G speeds and enable:

**MCP:**
```
madhyamas_set_throttle(
  download_bps=1048576,
  upload_bps=512000,
  delay_ms=100,
  enabled=true
)
```

### Simulate Packet Loss

Test how your app handles unreliable connections:

**MCP:**
```
madhyamas_set_throttle(
  delay_ms=300,
  jitter_ms=100,
  packet_loss_percent=10,
  enabled=true
)
```

## Parameters Reference

| Parameter | Type | Description |
|-----------|------|-------------|
| `download_bps` | integer | Download bandwidth (bytes/sec, 0 = unlimited) |
| `upload_bps` | integer | Upload bandwidth (bytes/sec, 0 = unlimited) |
| `delay_ms` | integer | Base latency in milliseconds |
| `jitter_ms` | integer | Random latency variation in milliseconds |
| `packet_loss_percent` | integer | Packet loss percentage (0-100) |
| `name` | string | Optional profile name |
| `enabled` | boolean | Enable throttling immediately |

## Interception Pipeline Order

Throttling runs last (priority 40):
1. Rewrites (priority 10)
2. Mocks (priority 20)
3. Breakpoints (priority 30)
4. **Throttle (priority 40)**

Throttling applies after all other interceptions, just before forwarding to the upstream server.

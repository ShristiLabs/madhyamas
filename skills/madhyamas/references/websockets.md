# WebSocket Traffic Inspection

## Overview

Inspect WebSocket connections and messages flowing through the proxy. WebSocket traffic is captured alongside HTTP traffic, with full message history including direction, type, and payload.

## MCP Tools

WebSocket-specific MCP tools are not available. Use the REST API or CLI for WebSocket traffic inspection.

## CLI Commands

WebSocket-specific CLI commands are not available. Use the REST API.

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/ws-traffic/connections` | List all WebSocket connections |
| GET | `/api/ws-traffic/connections/{id}` | Get specific connection details |
| GET | `/api/ws-traffic/messages` | Get messages with filtering |
| POST | `/api/ws-traffic/clear` | Clear all WebSocket traffic |

## Workflows

### List WebSocket Connections

**REST:** `curl http://localhost:3001/api/ws-traffic/connections`

Returns all WebSocket connections with metadata:
- `id` — connection ID
- `url` — WebSocket URL
- `host`, `path` — connection target
- `state` — connection state (open/closed)
- `created_at`, `closed_at` — timestamps
- `messages_sent`, `messages_received` — message counts
- `bytes_sent`, `bytes_received` — byte counts

### Get Connection Details

**REST:** `curl http://localhost:3001/api/ws-traffic/connections/abc123`

### Get WebSocket Messages

Filter messages by connection, direction, type, or content:

**REST:**
```bash
# All messages
curl http://localhost:3001/api/ws-traffic/messages

# Filter by connection
curl 'http://localhost:3001/api/ws-traffic/messages?connection_id=abc123'

# Filter by direction (send/receive)
curl 'http://localhost:3001/api/ws-traffic/messages?direction=send'

# Filter by message type (text/binary/ping/pong/close)
curl 'http://localhost:3001/api/ws-traffic/messages?message_type=text'

# Search message content
curl 'http://localhost:3001/api/ws-traffic/messages?search=keyword'
```

### Clear WebSocket Traffic

**REST:** `curl -X POST http://localhost:3001/api/ws-traffic/clear`

## Message Types

| Type | Description |
|------|-------------|
| `text` | UTF-8 text message |
| `binary` | Binary data |
| `ping` | Keep-alive ping frame |
| `pong` | Keep-alive pong frame |
| `close` | Connection close frame |
| `continuation` | Fragmented message continuation |

## Message Data

Each message contains:
- `id` — message ID
- `connection_id` — parent connection
- `direction` — `send` or `receive`
- `message_type` — text, binary, ping, pong, close
- `payload` — raw bytes, text, and parsed JSON (if applicable)
- `opcode` — WebSocket opcode
- `is_final` — whether this is the final fragment
- `mask` — client-side masking key
- `timestamp` — when the message was captured

## Real-time Updates

WebSocket traffic updates are broadcast through the main real-time WebSocket endpoint (`/api/ws`). Subscribe to traffic events to get real-time notifications of new WebSocket connections and messages.

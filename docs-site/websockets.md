---
title: WebSocket Inspection
description: Inspect WebSocket connections and messages flowing through the Madhyamas proxy — list connections, filter messages by direction and type, search payloads, and clear captured traffic.
---

# WebSocket Inspection

Madhyamas captures WebSocket connections alongside HTTP traffic, so you can inspect bidirectional real-time traffic from chat apps, streaming APIs, and any client using `ws://` or `wss://`. Each connection records its full message history with direction, type, payload, and timestamps.

## How It Works

When a client opens a WebSocket connection through the proxy (via HTTP `CONNECT` or an HTTP/1.1 upgrade), Madhyamas upgrades both sides and forwards frames while recording them. Both text and binary frames are captured, along with control frames (ping/pong/close).

WebSocket upgrades use HTTP/1.1 semantics, so WebSocket inspection does not require [HTTP/2 downstream](./http2-grpc) to be enabled.

## Viewing WebSocket Traffic

WebSocket connections appear in the traffic list like any other entry, with the WebSocket URL as the path. Select a connection to see its metadata and the full message timeline in the detail panel.

Each connection record includes:

| Field | Description |
|-------|-------------|
| `id` | Connection ID |
| `url` | Full WebSocket URL |
| `host`, `path` | Connection target |
| `state` | `open` or `closed` |
| `created_at`, `closed_at` | Timestamps |
| `messages_sent`, `messages_received` | Message counts |
| `bytes_sent`, `bytes_received` | Byte counts |

## Message Types

| Type | Description |
|------|-------------|
| `text` | UTF-8 text message |
| `binary` | Binary data |
| `ping` | Keep-alive ping frame |
| `pong` | Keep-alive pong frame |
| `close` | Connection close frame |
| `continuation` | Fragmented message continuation |

Each message carries: `id`, `connection_id`, `direction` (`send` or `receive`), `message_type`, `payload` (raw bytes, text, and parsed JSON when applicable), `opcode`, `is_final`, `mask`, and `timestamp`.

## Filtering Messages

Use the REST API to filter WebSocket messages by connection, direction, type, or content:

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

## Clearing WebSocket Traffic

```bash
curl -X POST http://localhost:3001/api/ws-traffic/clear
```

## Real-time Updates

WebSocket traffic events are broadcast through the main real-time WebSocket endpoint at `GET /api/ws`. Subscribe to traffic events to receive live notifications of new WebSocket connections and messages — the web UI uses this to update the traffic list in real time.

## Scripting Hook

You can programmatically inspect or modify WebSocket messages with the `on_websocket_message` script hook. See [Scripting](./scripting) for the hook signature and examples.

## See also

- [Traffic Inspection](./traffic-inspection) — viewing and filtering all captured traffic
- [HTTP/2 & gRPC](./http2-grpc) — gRPC (HTTP/2) inspection
- [Scripting](./scripting) — the `on_websocket_message` hook
- [REST API reference](./rest-api) — `/api/ws-traffic/*` endpoints

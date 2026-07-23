# gRPC Inspection

## Overview

Inspect gRPC traffic flowing through the proxy. gRPC connections, streams, and frames are captured and can be analyzed. This is an experimental feature with partial implementation.

> **Note:** gRPC inspection requires HTTP/2 downstream support, which is not yet fully implemented. Detection is based on Content-Type (`application/grpc*`) and path patterns (`/package.Service/Method`).

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_get_grpc_connections` | List all gRPC connections |
| `madhyamas_get_grpc_streams` | List all gRPC streams |
| `madhyamas_get_grpc_frames` | Get captured frames (optional filter) |
| `madhyamas_get_grpc_stats` | Get aggregated statistics |
| `madhyamas_clear_grpc` | Clear all frames and reset stats |

## CLI Commands

```bash
madhyamas grpc connections
madhyamas grpc streams
madhyamas grpc frames [--connection-id <ID>] [--stream-id <ID>] [--limit <N>]
madhyamas grpc stats
madhyamas grpc clear
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/grpc/connections` | List gRPC connections |
| GET | `/api/grpc/streams` | List gRPC streams |
| GET | `/api/grpc/frames` | Get frames (filters: service, method, path, direction, status_code) |
| GET | `/api/grpc/stats` | Get statistics |
| POST | `/api/grpc/clear` | Clear all frames |

## Workflows

### List gRPC Connections

**MCP:** `madhyamas_get_grpc_connections()`

**CLI:** `madhyamas grpc connections`

**REST:** `curl http://localhost:3001/api/grpc/connections`

### List gRPC Streams

**MCP:** `madhyamas_get_grpc_streams()`

**CLI:** `madhyamas grpc streams`

**REST:** `curl http://localhost:3001/api/grpc/streams`

### Get gRPC Frames

**MCP:** `madhyamas_get_grpc_frames(filter="service:MyService")`

**CLI:** `madhyamas grpc frames --connection-id conn123 --limit 50`

**REST:** `curl 'http://localhost:3001/api/grpc/frames?connection_id=conn123&limit=50'`

### Get gRPC Statistics

**MCP:** `madhyamas_get_grpc_stats()`

**CLI:** `madhyamas grpc stats`

**REST:** `curl http://localhost:3001/api/grpc/stats`

### Clear gRPC Data

**MCP:** `madhyamas_clear_grpc()`

**CLI:** `madhyamas grpc clear`

**REST:** `curl -X POST http://localhost:3001/api/grpc/clear`

## gRPC Message Types

| Type | Description |
|------|-------------|
| Unary | Single request, single response |
| ServerStream | Single request, multiple responses |
| ClientStream | Multiple requests, single response |
| BidiStream | Bidirectional streaming |

## Limitations

- HTTP/2 downstream not fully implemented (only HTTP/1.1 on client-facing side)
- Protobuf descriptor support is limited
- Frame parsing is basic; full protobuf decoding may be incomplete

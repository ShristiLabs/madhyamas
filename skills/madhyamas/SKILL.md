---
name: madhyamas
description: >
  Procedural knowledge for using Madhyamas, an open-source HTTP/HTTPS debugging
  proxy built in Rust. Use this skill when: (1) debugging HTTP/HTTPS API traffic,
  (2) mocking API responses, (3) setting breakpoints on requests/responses,
  (4) rewriting URLs/headers/bodies, (5) throttling network conditions,
  (6) replaying captured requests, (7) inspecting WebSocket or gRPC traffic,
  (8) exporting traffic as HAR or cURL, (9) managing debugging sessions,
  (10) configuring MCP server for AI agent integration, (11) troubleshooting
  proxy/TLS/certificate issues, (12) using madhyamas CLI commands, or
  (13) calling the Madhyamas REST API. Covers MCP tools (146 tools), CLI
  commands (159 subcommands), and REST API (186 endpoints).
license: MIT OR Apache-2.0
metadata:
  author: madhyamas
  version: "0.1.0"
  project-url: https://github.com/ShristiLabs/madhyamas
---

# Madhyamas Proxy — AI Agent Guide

Madhyamas is an open-source HTTP/HTTPS debugging proxy built in Rust. It captures, inspects, and manipulates traffic between clients and servers. The unified `madhyamas` binary provides a proxy server, web UI, MCP server, and CLI.

## Quick Start

```bash
# 1. Install (choose one)
cargo install madhyamas                          # from crates.io
cargo build --release -p madhyamas               # from source
# Or download pre-built binary from GitHub Releases

# 2. Start the proxy server
madhyamas serve
# Proxy listens on :8888, API/Web UI on :3001

# 3. Verify
curl http://localhost:3001/api/health
# Returns: OK
```

Configure clients to use proxy `localhost:8888`. For HTTPS interception, install the CA certificate from `~/.madhyamas/certs/madhyamas-ca.pem` (or download via `GET /api/cert/ca`).

See [references/setup.md](references/setup.md) for detailed installation, configuration, and CA certificate setup.

## Choosing an Interface

Madhyamas exposes three interfaces. Use whichever is available in your context:

| Interface | When to Use | Setup | Reference |
|-----------|-------------|-------|-----------|
| **MCP tools** | Inside MCP-compatible agents (Claude Desktop, Windsurf, etc.) | `madhyamas mcp` (stdio transport) | [references/mcp-tools.md](references/mcp-tools.md) |
| **CLI commands** | Shell access, scripting, terminal-based agents | `madhyamas <command> <subcommand>` | [references/cli-commands.md](references/cli-commands.md) |
| **REST API** | Fine-grained control, custom scripts, direct HTTP | `curl http://localhost:3001/api/...` | [references/rest-api.md](references/rest-api.md) |

All three interfaces expose the same underlying functionality. MCP tools and CLI commands are wrappers around the REST API.

## Key Concepts

- **Proxy ports**: Proxy on `:8888` (HTTP/HTTPS traffic), API/UI on `:3001`
- **CA certificate**: Auto-generated at `~/.madhyamas/certs/`. Install in system trust store for HTTPS interception
- **Sessions**: Organize traffic into named sessions. Default session auto-created
- **Capture mode**: Recording (default) captures traffic; Passthrough forwards without recording
- **Interception pipeline order**: Rewrites (priority 10) → Mocks (20) → Breakpoints (30) → Throttle (40)
- **Data directory**: `~/.madhyamas/` contains `certs/`, `logs/`, `traffic.db` (SQLite)
- **MCP server**: Run `madhyamas mcp` for stdio-based MCP transport. Set `MADHYAMAS_API_URL` to connect to a running proxy instance

## Core Workflows

| Workflow | Description | Reference |
|----------|-------------|-----------|
| **Setup** | Install, configure, connect clients, install CA cert | [setup.md](references/setup.md) |
| **Inspect traffic** | List, filter, search, analyze captured HTTP/HTTPS traffic | [traffic-inspection.md](references/traffic-inspection.md) |
| **Mock responses** | Create mock rules to intercept and return fake responses | [mocking.md](references/mocking.md) |
| **Breakpoints** | Pause requests/responses for inspection and modification | [breakpoints.md](references/breakpoints.md) |
| **Rewrites** | Modify URLs, headers, bodies of matching traffic | [rewrites.md](references/rewrites.md) |
| **Throttling** | Simulate slow/unreliable network conditions | [throttling.md](references/throttling.md) |
| **Replay** | Re-execute captured requests with modifications | [replay.md](references/replay.md) |
| **Sessions** | Create, switch, export, import debugging sessions | [sessions.md](references/sessions.md) |
| **gRPC inspection** | Inspect gRPC connections, streams, frames (experimental) | [grpc.md](references/grpc.md) |
| **Scripting** | Automate traffic manipulation with JS/TS scripts (experimental) | [scripting.md](references/scripting.md) |
| **Plugins** | Manage Rust plugins for extended functionality (experimental) | [plugins.md](references/plugins.md) |
| **WebSockets** | Inspect WebSocket connection traffic and messages | [websockets.md](references/websockets.md) |
| **Export/Import** | HAR export, cURL export, rule persistence | [export-import.md](references/export-import.md) |
| **Troubleshooting** | Cert errors, port conflicts, DB locks, TLS failures | [troubleshooting.md](references/troubleshooting.md) |
| **Harness setup** | Configure MCP for Claude, Windsurf, Cursor, etc. | [harness-setup.md](references/harness-setup.md) |

## Reference Index

| File | When to Read |
|------|-------------|
| `references/setup.md` | First-time setup, installation, CA cert, client configuration |
| `references/mcp-tools.md` | Full reference for all 146 MCP tools with parameters |
| `references/cli-commands.md` | Full reference for all 159 CLI subcommands with flags |
| `references/rest-api.md` | Full reference for all 184 REST API endpoints |
| `references/traffic-inspection.md` | Filtering, searching, analyzing captured traffic |
| `references/mocking.md` | Creating and managing mock responses |
| `references/breakpoints.md` | Pausing and modifying traffic |
| `references/rewrites.md` | URL/header/body rewriting rules |
| `references/throttling.md` | Network condition simulation |
| `references/replay.md` | Replaying and saving requests |
| `references/sessions.md` | Session management |
| `references/grpc.md` | gRPC traffic inspection |
| `references/scripting.md` | JavaScript/TypeScript scripting |
| `references/plugins.md` | Plugin management |
| `references/websockets.md` | WebSocket traffic inspection |
| `references/export-import.md` | HAR/cURL export, persistence |
| `references/troubleshooting.md` | Common issues and solutions |
| `references/harness-setup.md` | Per-harness MCP configuration |

## MCP Server Setup

Run the MCP server for AI agent integration:

```bash
madhyamas mcp                    # stdio transport, connects to localhost:3001
MADHYAMAS_API_URL=http://host:3001 madhyamas mcp  # custom API URL
```

Add to your agent's MCP config:
```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/path/to/madhyamas",
      "args": ["mcp"],
      "env": { "MADHYAMAS_API_URL": "http://127.0.0.1:3001" }
    }
  }
}
```

See [references/harness-setup.md](references/harness-setup.md) for harness-specific configuration (Claude Desktop, Windsurf, Cursor, OpenCode, CommandCode, Devin).

## Common Patterns

**Debug failed API calls:**
1. Start proxy: `madhyamas serve`
2. Configure client to use proxy
3. Reproduce the issue
4. Inspect traffic: `madhyamas traffic list --status 500` or MCP `madhyamas_get_traffic` with `status=500`
5. Get details: `madhyamas traffic get <id>` or MCP `madhyamas_get_traffic_entry`

**Mock an API endpoint:**
1. Create mock: `madhyamas mocks create --url-pattern "*/api/auth*" --status-code 200 --body '{"token":"fake"}'`
2. Or MCP: `madhyamas_create_mock` with `url_pattern="*/api/auth*"`, `status_code=200`, `body='{"token":"fake"}'`
3. Test the mocked endpoint from your client

**Test under slow network:**
1. Enable throttle: `madhyamas throttle set --download-bps 50000 --delay-ms 200 && madhyamas throttle enable`
2. Or MCP: `madhyamas_set_throttle` with `download_bps=50000, delay_ms=200, enabled=true`
3. Reproduce your workflow to see behavior under slow conditions

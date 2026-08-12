---
title: MCP & AI Agents
description: Connect AI agents like Claude Desktop, Cursor, Windsurf, and Devin to Madhyamas via the Model Context Protocol (MCP) server — inspect traffic, create mocks, and drive the proxy from your LLM.
---

# MCP & AI Agents

Madhyamas ships with a built-in **Model Context Protocol (MCP)** server that lets AI agents inspect captured traffic, create mocks and rewrites, manage sessions, and control every feature of the proxy — without leaving your agent's chat or IDE.

MCP is an open standard that lets LLM-powered tools call external services through a uniform interface. The Madhyamas MCP server exposes **135 tools** covering traffic inspection, mocks, breakpoints, replay, sessions, scripting, plugins, and more.

## How It Works

```
┌────────────┐     stdio      ┌────────────────┐     REST      ┌──────────────┐
│  AI Agent  │ ─────────────▶ │ madhyamas mcp  │ ────────────▶ │ madhyamas    │
│  (Claude,  │   JSON-RPC     │  (MCP server)  │   /api/*      │  proxy + UI  │
│  Cursor…)  │ ◀───────────── │                │ ◀──────────── │              │
└────────────┘                └────────────────┘               └──────────────┘
```

1. You start the Madhyamas proxy (`madhyamas serve`).
2. Your AI agent spawns `madhyamas mcp` as a child process (stdio transport).
3. The MCP server forwards tool calls to the proxy's REST API at `http://127.0.0.1:3001`.
4. The agent sees the results and can act on them — list traffic, create a mock, replay a request, etc.

## Prerequisites

- Madhyamas installed and on your `PATH` (`madhyamas --version` works)
- The proxy running: `madhyamas serve`
- Health check passes: `curl http://localhost:3001/api/health` returns `OK`
- An MCP-compatible AI agent (Claude Desktop, Windsurf, Cursor with an MCP extension, OpenCode, CommandCode, Devin CLI, or any client that speaks MCP)

## Configuring Your Agent

The MCP server is invoked as `madhyamas mcp`. Add it to your agent's MCP server config under the key `madhyamas`.

### Claude Desktop

Config file location:

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/usr/local/bin/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://127.0.0.1:3001"
      }
    }
  }
}
```

Restart Claude Desktop after editing.

### Windsurf (Codeium)

Config file location:

- **macOS**: `~/.codeium/windsurf/mcp_config.json`
- **Linux**: `~/.config/windsurf/mcp_config.json`
- **Windows**: `%APPDATA%\windsurf\mcp_config.json`

Use the same JSON shape as Claude Desktop above. Restart Windsurf after editing.

### Cursor

Cursor does not natively host MCP servers. Two options:

1. **Use the CLI directly** in Cursor's integrated terminal — see the [CLI reference](./cli):
   ```bash
   madhyamas traffic list --json
   madhyamas mocks create --url-pattern "*/api/test*" --status-code 200
   ```
2. **Install an MCP-compatible Cursor extension** and add the same JSON config as Claude Desktop.

### OpenCode / CommandCode / Devin CLI

These harnesses follow the Agent Skills standard and read MCP config from a project or global directory (e.g. `.opencode/`, `.commandcode/`, `.devin/`). Add the `madhyamas` server using the same JSON shape as Claude Desktop. See `skills/madhyamas/references/harness-setup.md` in the repo for harness-specific paths.

### Generic MCP Client

For any MCP-compatible client, use this template:

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/path/to/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://127.0.0.1:3001",
        "MADHYAMAS_TIMEOUT": "30",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Docker

If the proxy runs in Docker with `-p 3001:3001`, the host-side MCP server connects to `http://localhost:3001` as shown in the generic template above. No container-internal networking is needed.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MADHYAMAS_API_URL` | `http://127.0.0.1:3001` | API endpoint of the running proxy |
| `MADHYAMAS_TIMEOUT` | `30` | Request timeout in seconds |
| `RUST_LOG` | `info` | Logging level (`trace`, `debug`, `info`, `warn`, `error`) |

## Verification

After configuring your agent, verify the MCP server works:

1. Confirm the proxy is healthy:
   ```bash
   curl http://localhost:3001/api/health
   ```
2. Test the MCP server manually:
   ```bash
   echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | madhyamas mcp
   ```
   You should see a JSON response listing all available tools.
3. In your agent, ask: "Show me the recent HTTP traffic captured by Madhyamas" or "List all Madhyamas MCP tools".

## Tool Categories

The MCP server exposes 135 tools grouped by feature. The table below summarizes the categories; see the [full tool reference](https://github.com/ShristiLabs/madhyamas/blob/main/skills/madhyamas/references/mcp-tools.md) for parameter-level detail.

| Category | Tools | What you can do |
|----------|-------|-----------------|
| Traffic Inspection | 7 | List, search, count, clear traffic; import HAR; get script traces |
| Mock Rules | 21 | Create, update, delete, toggle, test, and version mock rules |
| Mock Collections | 6 | Group mocks into collections and toggle them together |
| Mock Analytics | 2 | Inspect mock hit counts and analytics |
| Breakpoints | 7 | Create, list, delete, and pause/resume breakpoints |
| Replay | 6 | Replay saved requests, run advanced batches, view history |
| Sessions | 5 | Create, switch, delete, and export sessions |
| Configuration | 2 | Get and update proxy configuration |
| Capture Mode | 2 | Toggle between Recording and Passthrough modes |
| Throttle | 4 | Set latency/bandwidth/loss presets; enable/disable |
| Rewrites | 7 | Create, update, toggle, and apply rewrite templates |
| gRPC | 5 | Inspect gRPC connections, streams, frames, and stats |
| Scripts | 16 | Create, validate, test, toggle, and view history for JS scripts |
| Plugins | 21 | Install, enable, configure, sign, and scaffold WASM plugins |
| Auto Save | 3 | Configure and trigger session backups |
| Block List | 7 | Add, remove, and toggle blocked domains/patterns |
| Focus Hosts | 4 | Add and remove hosts highlighted in the traffic view |
| Mirror | 3 | Configure response mirroring to disk |
| Logs | 3 | View and rotate application logs |
| WebSocket Traffic | 4 | Inspect WebSocket connections and messages |
| Certificate | 1 | Download the CA certificate |

## Example Agent Prompts

Once connected, try these prompts in your AI agent:

- "List the last 10 requests to `api.example.com` captured by Madhyamas."
- "Create a mock that returns `200 OK` with a JSON body for any `GET /api/users/*` request."
- "Show me all 5xx responses from the last hour and summarize the error patterns."
- "Replay the saved login request 50 times with 5 concurrent connections and report the latency stats."
- "Add a rewrite that injects an `Authorization: Bearer test-token` header on all requests to `staging.example.com`."

## See also

- [CLI reference](./cli) — drive Madhyamas from the terminal (no MCP needed)
- [REST API reference](./rest-api) — the HTTP API the MCP server calls under the hood
- [Scripting](./scripting) — programmatic traffic handling inside the proxy
- [Plugins](./plugins) — extend the proxy with WebAssembly
- [Troubleshooting](./troubleshooting) — MCP connection and tool-discovery fixes

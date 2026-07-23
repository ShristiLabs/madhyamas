# Harness Setup

Configure the Madhyamas MCP server for each AI agent harness. The MCP server uses stdio transport and connects to a running Madhyamas proxy instance.

## Prerequisites

1. Install Madhyamas: `cargo install madhyamas` or download binary
2. Start the proxy: `madhyamas serve`
3. Verify: `curl http://localhost:3001/api/health` returns `OK`

## Claude Desktop

**Config file location:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

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

Restart Claude Desktop after editing. Verify by asking: "Show me the recent HTTP traffic captured by Madhyamas."

## Windsurf (Codeium)

**Config file location:**
- macOS: `~/.codeium/windsurf/mcp_config.json`
- Linux: `~/.config/windsurf/mcp_config.json`
- Windows: `%APPDATA%\windsurf\mcp_config.json`

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

Restart Windsurf after editing.

## Cursor

Cursor does not natively support MCP servers in the same way. Use the CLI interface instead:

1. Ensure `madhyamas` is in your PATH
2. Use CLI commands directly in Cursor's terminal:
   ```bash
   madhyamas traffic list --json
   madhyamas mocks create --url-pattern "*/api/test*" --status-code 200
   ```

Alternatively, if using Cursor with an MCP-compatible extension, add the same JSON config as Claude Desktop above.

## OpenCode

**Config file location:**
- Project: `.opencode/` directory
- Global: `~/.config/opencode/`

Add the MCP server configuration following the same JSON format as Claude Desktop. OpenCode also scans `.agents/skills/` and `.claude/skills/` for skill discovery.

## CommandCode

**Config file location:**
- Project: `.commandcode/` directory
- Global: `~/.commandcode/`

CommandCode follows the Agent Skills standard. Add MCP config using the same JSON format. CommandCode scans `.commandcode/skills/` and `.agents/skills/` for skill discovery.

## Devin CLI

**Config file location:**
- Project: `.devin/` directory
- Global: `~/.config/devin/`

Devin scans multiple locations for skills:
- `.agents/skills/`
- `.devin/skills/`
- `.github/skills/`
- `.claude/skills/`
- `.cursor/skills/`
- `.codex/skills/`
- `.cognition/skills/`
- `.windsurf/skills/`

Add the MCP server to Devin's MCP config using the same JSON format.

## Generic MCP Configuration

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

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MADHYAMAS_API_URL` | `http://127.0.0.1:3001` | API endpoint for the running proxy |
| `MADHYAMAS_TIMEOUT` | `30` | Request timeout in seconds |
| `RUST_LOG` | `info` | Logging level (trace/debug/info/warn/error) |

## Docker Setup

If running Madhyamas in Docker, the MCP server on the host connects to the container's exposed port:

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/usr/local/bin/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

Docker exposes port 3001 to localhost, so the MCP server can connect directly.

## Verification

After configuring, verify the MCP server works:

1. Check proxy is running: `curl http://localhost:3001/api/health`
2. Test MCP server manually:
   ```bash
   echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | madhyamas mcp
   ```
3. In your AI agent, ask: "List all Madhyamas MCP tools" or "Show me captured traffic"

## Troubleshooting MCP

See [troubleshooting.md](troubleshooting.md) for MCP-specific issues:
- MCP server not connecting
- Tools not appearing
- Permission denied errors
- Connection refused errors

# ProxyForge MCP Integration Guide

This guide explains how to integrate ProxyForge with AI assistants like Windsurf, Claude Desktop, and other MCP-compatible tools.

## What is MCP?

The Model Context Protocol (MCP) is a standard that allows AI assistants to interact with external tools and services. ProxyForge provides an MCP server that exposes proxy functionality to AI assistants.

## Available MCP Tools

The ProxyForge MCP server provides the following tools:

| Tool | Description |
|------|-------------|
| `proxyforge_get_traffic` | Get captured HTTP/HTTPS traffic with filtering |
| `proxyforge_get_traffic_details` | Get detailed information about a specific request |
| `proxyforge_clear_traffic` | Clear all captured traffic |
| `proxyforge_get_config` | Get current proxy configuration |
| `proxyforge_update_config` | Update runtime configuration |
| `proxyforge_get_capture_status` | Check if traffic capture is enabled |
| `proxyforge_toggle_capture` | Enable/disable traffic capture |
| `proxyforge_create_mock` | Create a mock response rule |
| `proxyforge_list_mocks` | List all mock rules |
| `proxyforge_delete_mock` | Delete a mock rule |
| `proxyforge_create_breakpoint` | Create a breakpoint rule |
| `proxyforge_list_breakpoints` | List all breakpoint rules |
| `proxyforge_delete_breakpoint` | Delete a breakpoint rule |
| `proxyforge_replay_request` | Replay a captured request |
| `proxyforge_list_sessions` | List all sessions |
| `proxyforge_create_session` | Create a new session |
| `proxyforge_switch_session` | Switch to a different session |

## Setup Options

### Option 1: Build from Source (Recommended for Development)

```bash
# Build the MCP binary
cargo build --release -p proxyforge-mcp

# The binary will be at: target/release/proxyforge-mcp
```

### Option 2: Extract from Docker

```bash
# Run the extraction script
./scripts/extract-mcp.sh

# The binary will be at: bin/proxyforge-mcp
```

### Option 3: Use Docker Directly

```bash
# Build the Docker image
docker compose build proxyforge

# Extract the binary
docker create --name temp proxyforge:latest
docker cp temp:/usr/local/bin/proxyforge-mcp ./proxyforge-mcp
docker rm temp
chmod +x ./proxyforge-mcp
```

## Windsurf Integration

### Step 1: Locate Your MCP Config File

Windsurf stores MCP configuration in:
- **macOS**: `~/.codeium/windsurf/mcp_config.json`
- **Linux**: `~/.config/windsurf/mcp_config.json`
- **Windows**: `%APPDATA%\windsurf\mcp_config.json`

### Step 2: Add ProxyForge MCP Server

Edit your `mcp_config.json` file and add the ProxyForge server:

```json
{
  "mcpServers": {
    "proxyforge": {
      "command": "/absolute/path/to/proxyforge-mcp",
      "env": {
        "PROXYFORGE_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

**Example for this project:**

```json
{
  "mcpServers": {
    "proxyforge": {
      "command": "/Users/harikiranbavineni/product-design-skill/proxyforge/target/release/proxyforge-mcp",
      "env": {
        "PROXYFORGE_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

### Step 3: Start ProxyForge

Make sure ProxyForge is running before using the MCP tools:

```bash
# Using Docker
./startup.sh

# Or run locally
cargo run --release
```

### Step 4: Restart Windsurf

Restart Windsurf to load the new MCP configuration.

### Step 5: Verify Integration

In Windsurf, you should now see ProxyForge tools available. Try asking:
- "Show me the recent HTTP traffic captured by ProxyForge"
- "What's the current ProxyForge configuration?"
- "Create a mock response for /api/test that returns 200 OK"

## Claude Desktop Integration

### Step 1: Locate Config File

Claude Desktop stores MCP configuration in:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

### Step 2: Add ProxyForge Server

```json
{
  "mcpServers": {
    "proxyforge": {
      "command": "/absolute/path/to/proxyforge-mcp",
      "env": {
        "PROXYFORGE_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

### Step 3: Restart Claude Desktop

Restart the application to load the new configuration.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PROXYFORGE_API_URL` | `http://127.0.0.1:3001` | ProxyForge API endpoint |
| `PROXYFORGE_TIMEOUT` | `30` | Request timeout in seconds |
| `RUST_LOG` | - | Set to `debug` for verbose logging |

## Docker with MCP

If running ProxyForge in Docker, the MCP server needs to connect to the Docker container's API:

```json
{
  "mcpServers": {
    "proxyforge": {
      "command": "/path/to/proxyforge-mcp",
      "env": {
        "PROXYFORGE_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

Since Docker exposes port 3001 to localhost, the MCP server running on your host can connect to `http://localhost:3001`.

## Troubleshooting

### MCP Server Not Connecting

1. Verify ProxyForge is running: `curl http://localhost:3001/api/health`
2. Check the MCP binary path is correct and executable
3. Check Windsurf/Claude logs for errors

### Tools Not Appearing

1. Restart your AI assistant after config changes
2. Verify JSON syntax in config file
3. Check that the binary has execute permissions: `chmod +x proxyforge-mcp`

### Permission Denied

```bash
chmod +x /path/to/proxyforge-mcp
```

### Connection Refused

Make sure ProxyForge is running and the API port (3001) is accessible:

```bash
# Check if ProxyForge is running
curl http://localhost:3001/api/health

# If using Docker, check container status
docker compose ps
```

## Example Usage in Windsurf

Once configured, you can use natural language to interact with ProxyForge:

```
User: Show me the last 10 HTTP requests captured by the proxy

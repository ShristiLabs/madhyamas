# Madhyamas MCP Integration Guide

This guide explains how to integrate Madhyamas with AI assistants like Windsurf, Claude Desktop, and other MCP-compatible tools.

## What is MCP?

The Model Context Protocol (MCP) is a standard that allows AI assistants to interact with external tools and services. Madhyamas provides an MCP server that exposes proxy functionality to AI assistants.

## Available MCP Tools

The Madhyamas MCP server provides the following tools:

| Tool | Description |
|------|-------------|
| `madhyamas_get_traffic` | Get captured HTTP/HTTPS traffic with filtering |
| `madhyamas_get_traffic_details` | Get detailed information about a specific request |
| `madhyamas_clear_traffic` | Clear all captured traffic |
| `madhyamas_get_config` | Get current proxy configuration |
| `madhyamas_update_config` | Update runtime configuration |
| `madhyamas_get_capture_status` | Check if traffic capture is enabled |
| `madhyamas_toggle_capture` | Enable/disable traffic capture |
| `madhyamas_create_mock` | Create a mock response rule |
| `madhyamas_list_mocks` | List all mock rules |
| `madhyamas_delete_mock` | Delete a mock rule |
| `madhyamas_create_breakpoint` | Create a breakpoint rule |
| `madhyamas_list_breakpoints` | List all breakpoint rules |
| `madhyamas_delete_breakpoint` | Delete a breakpoint rule |
| `madhyamas_replay_request` | Replay a captured request |
| `madhyamas_list_sessions` | List all sessions |
| `madhyamas_create_session` | Create a new session |
| `madhyamas_switch_session` | Switch to a different session |

## Setup Options

### Option 1: Build from Source (Recommended for Development)

```bash
# Build the unified binary
cargo build --release -p madhyamas

# Run as MCP server: target/release/madhyamas mcp
```

### Option 2: Use Docker

```bash
# Build the Docker image
docker compose build madhyamas

# The binary inside the container is at /usr/local/bin/madhyamas
# Run MCP mode: docker run --rm madhyamas:latest mcp
```

## Windsurf Integration

### Step 1: Locate Your MCP Config File

Windsurf stores MCP configuration in:
- **macOS**: `~/.codeium/windsurf/mcp_config.json`
- **Linux**: `~/.config/windsurf/mcp_config.json`
- **Windows**: `%APPDATA%\windsurf\mcp_config.json`

### Step 2: Add Madhyamas MCP Server

Edit your `mcp_config.json` file and add the Madhyamas server:

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/absolute/path/to/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

**Example for this project:**

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/path/to/madhyamas/target/release/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

### Step 3: Start Madhyamas

Make sure Madhyamas is running before using the MCP tools:

```bash
# Using Docker
./startup.sh

# Or run locally
cargo run --release
```

### Step 4: Restart Windsurf

Restart Windsurf to load the new MCP configuration.

### Step 5: Verify Integration

In Windsurf, you should now see Madhyamas tools available. Try asking:
- "Show me the recent HTTP traffic captured by Madhyamas"
- "What's the current Madhyamas configuration?"
- "Create a mock response for /api/test that returns 200 OK"

## Claude Desktop Integration

### Step 1: Locate Config File

Claude Desktop stores MCP configuration in:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

### Step 2: Add Madhyamas Server

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/absolute/path/to/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001"
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
| `MADHYAMAS_API_URL` | `http://127.0.0.1:3001` | Madhyamas API endpoint |
| `MADHYAMAS_TIMEOUT` | `30` | Request timeout in seconds |
| `RUST_LOG` | - | Set to `debug` for verbose logging |

## Docker with MCP

If running Madhyamas in Docker, the MCP server needs to connect to the Docker container's API:

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/path/to/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001"
      }
    }
  }
}
```

Since Docker exposes port 3001 to localhost, the MCP server running on your host can connect to `http://localhost:3001`.

## Troubleshooting

### MCP Server Not Connecting

1. Verify Madhyamas is running: `curl http://localhost:3001/api/health`
2. Check the MCP binary path is correct and executable
3. Check Windsurf/Claude logs for errors

### Tools Not Appearing

1. Restart your AI assistant after config changes
2. Verify JSON syntax in config file
3. Check that the binary has execute permissions: `chmod +x madhyamas`

### Permission Denied

```bash
chmod +x /path/to/madhyamas
```

### Connection Refused

Make sure Madhyamas is running and the API port (3001) is accessible:

```bash
# Check if Madhyamas is running
curl http://localhost:3001/api/health

# If using Docker, check container status
docker compose ps
```

## Example Usage in Windsurf

Once configured, you can use natural language to interact with Madhyamas:

```
User: Show me the last 10 HTTP requests captured by the proxy

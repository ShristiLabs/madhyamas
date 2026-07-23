# Plugins

## Overview

Manage Rust plugins that extend Madhyamas functionality. Plugins are discovered from local directories and can be enabled/disabled at runtime. This is an experimental feature.

> **Note:** Plugin code execution is not yet implemented. The plugin system supports manifest parsing, discovery, dependency validation, and state management, but actual plugin execution (WASM or dynamic loading) is a future enhancement.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_list_plugins` | List all loaded plugins |
| `madhyamas_get_plugin` | Get plugin details |
| `madhyamas_enable_plugin` | Enable a plugin |
| `madhyamas_disable_plugin` | Disable a plugin |
| `madhyamas_get_plugin_stats` | Get runtime statistics |
| `madhyamas_reload_plugins` | Reload all from disk |

## CLI Commands

```bash
madhyamas plugins list
madhyamas plugins get <ID>
madhyamas plugins enable <ID>
madhyamas plugins disable <ID>
madhyamas plugins stats <ID>
madhyamas plugins reload
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/plugins` | List all plugins |
| GET | `/api/plugins/{id}` | Get plugin details |
| POST | `/api/plugins/{id}/enable` | Enable plugin |
| POST | `/api/plugins/{id}/disable` | Disable plugin |
| GET | `/api/plugins/{id}/stats` | Get plugin statistics |
| POST | `/api/plugins/reload` | Reload all plugins |

## Workflows

### List All Plugins

**MCP:** `madhyamas_list_plugins()`

**CLI:** `madhyamas plugins list`

**REST:** `curl http://localhost:3001/api/plugins`

### Get Plugin Details

**MCP:** `madhyamas_get_plugin(id="my-plugin")`

**CLI:** `madhyamas plugins get my-plugin`

**REST:** `curl http://localhost:3001/api/plugins/my-plugin`

### Enable a Plugin

**MCP:** `madhyamas_enable_plugin(id="my-plugin")`

**CLI:** `madhyamas plugins enable my-plugin`

**REST:** `curl -X POST http://localhost:3001/api/plugins/my-plugin/enable`

### Disable a Plugin

**MCP:** `madhyamas_disable_plugin(id="my-plugin")`

**CLI:** `madhyamas plugins disable my-plugin`

**REST:** `curl -X POST http://localhost:3001/api/plugins/my-plugin/disable`

### Get Plugin Statistics

**MCP:** `madhyamas_get_plugin_stats(id="my-plugin")`

**CLI:** `madhyamas plugins stats my-plugin`

**REST:** `curl http://localhost:3001/api/plugins/my-plugin/stats`

### Reload All Plugins

**MCP:** `madhyamas_reload_plugins()`

**CLI:** `madhyamas plugins reload`

**REST:** `curl -X POST http://localhost:3001/api/plugins/reload`

## Plugin Discovery

Plugins are discovered from these directories:
- `./plugins/` (current directory)
- `~/.madhyamas/plugins/` (user home)
- Custom paths (configurable)

Each plugin directory must contain a manifest file:
- `madhyamas-plugin.toml` (TOML format)
- `madhyamas-plugin.json` (JSON format)

## Plugin Manifest Fields

| Field | Description |
|-------|-------------|
| `id` | Unique plugin identifier |
| `name` | Display name |
| `version` | Semver version |
| `description` | Plugin description |
| `author` | Author name |
| `capabilities` | List of capabilities (request/response hooks, traffic filtering, etc.) |
| `dependencies` | List of dependency constraints |
| `hooks` | List of hooks the plugin attaches to |

## Limitations

- No execution engine (WASM/dynamic loading not implemented)
- No remote plugin registry (local discovery only)
- Plugin code does not actually run yet

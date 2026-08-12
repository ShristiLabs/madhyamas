# API — Scripts & Plugins

Endpoints for the JavaScript scripting system and the WASM plugin system. Base
path: `/api`. See [SCRIPTING.md](SCRIPTING.md) and [PLUGINS.md](PLUGINS.md) for
the feature guides.

## Scripts

The scripting system is feature-gated behind the `scripting` Cargo feature.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/scripts` | List scripts |
| POST | `/scripts` | Create a script |
| GET | `/scripts/templates` | List built-in script templates |
| GET | `/scripts/config` | Get script runtime config |
| PUT | `/scripts/config` | Update script runtime config |
| GET | `/scripts/history` | Get script execution history (all scripts) |
| POST | `/scripts/test` | Test a script against a sample request/response |
| POST | `/scripts/validate` | Validate a script's syntax |
| POST | `/scripts/match-preview` | Preview which scripts would match a request |
| GET | `/scripts/{id}` | Get a script |
| PUT | `/scripts/{id}` | Update a script |
| DELETE | `/scripts/{id}` | Delete a script |
| POST | `/scripts/{id}/toggle` | Enable/disable a script |
| POST | `/scripts/{id}/reorder` | Reorder a script in the execution chain |
| GET | `/scripts/{id}/history` | Get execution history for a script |
| DELETE | `/scripts/{id}/history` | Clear execution history for a script |

See [SCRIPTING_API.md](SCRIPTING_API.md) for the JavaScript API available
inside scripts and [SCRIPTING_SECURITY.md](SCRIPTING_SECURITY.md) for the
sandbox model.

## Plugins

The plugin system is feature-gated behind the `plugins` Cargo feature.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/plugins` | List installed plugins |
| GET | `/plugins/{id}` | Get a plugin |
| POST | `/plugins/{id}/enable` | Enable a plugin |
| POST | `/plugins/{id}/disable` | Disable a plugin |
| GET | `/plugins/{id}/stats` | Get plugin execution stats |
| POST | `/plugins/reload` | Hot-reload all plugins |
| POST | `/plugins/install` | Install a plugin from a URL or file |
| DELETE | `/plugins/{id}/uninstall` | Uninstall a plugin |
| GET | `/plugins/{id}/settings` | Get plugin settings |
| PUT | `/plugins/{id}/settings` | Update plugin settings |
| GET | `/plugins/{id}/schema` | Get plugin settings JSON schema |
| GET | `/plugins/{id}/panels` | Get custom UI panels contributed by the plugin |
| GET | `/plugins/{id}/logs` | Get plugin execution logs |

### Plugin Registry

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/plugins/registry` | List the plugin registry |
| GET | `/plugins/registry/search` | Search the registry |
| GET | `/plugins/registry/{id}` | Get a registry entry |
| GET | `/plugins/registry/config` | Get registry config (remote URL, etc.) |
| PUT | `/plugins/registry/config` | Update registry config |
| POST | `/plugins/registry/refresh` | Refresh the local registry cache |
| GET | `/plugins/templates` | List plugin templates |
| POST | `/plugins/scaffold` | Scaffold a new plugin project |

See [PLUGIN_API.md](PLUGIN_API.md) for the guest SDK API and
[PLUGIN_DEVELOPMENT.md](PLUGIN_DEVELOPMENT.md) for the development guide.

## See Also

- [API.md](API.md) — API index
- [SCRIPTING.md](SCRIPTING.md) — Scripting system overview
- [SCRIPTING_API.md](SCRIPTING_API.md) — JavaScript API reference
- [PLUGINS.md](PLUGINS.md) — Plugin system overview
- [PLUGIN_DEVELOPMENT.md](PLUGIN_DEVELOPMENT.md) — Plugin development guide
- [EXTENSION_SYSTEM.md](EXTENSION_SYSTEM.md) — Unified extension model

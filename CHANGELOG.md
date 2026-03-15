# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project structure with Rust workspace (core, api, cli crates)
- React + TypeScript web frontend with Vite
- HTTP/HTTPS proxy functionality
- TLS certificate generation for HTTPS interception
- Traffic capture and storage in SQLite
- Web UI for traffic inspection
- Breakpoint functionality for request/response interception
- Mock server capabilities
- URL rewriting rules
- Network throttling simulation
- Request replay functionality
- Session save/load
- Docker support
- GitHub Actions CI/CD

### Added (Recent)

#### Capture Mode (Passthrough)

- **API**: `GET /api/capture` - Get current capture status (recording/passthrough)
- **API**: `POST /api/capture/toggle` - Toggle between recording and passthrough mode
- **Core**: `TrafficStore` now supports passthrough mode via `AtomicBool` flag
- **Frontend**: Recording/Passthrough toggle button in header with visual status indicator
- **CLI**: `madhyamas capture status|toggle|enable|disable` commands
- **MCP**: `madhyamas_get_capture_status` and `madhyamas_toggle_capture` tools

#### Runtime Configuration

- **API**: `PATCH /api/config` - Update runtime-changeable settings (intercept_https, max_requests, verbose, public_ip)
- **Frontend**: ConfigDialog component with 4 tabs (General, Upstream Proxy, Capture, Appearance)
- **CLI**: `madhyamas config get` and `madhyamas config update` commands with flags
- **MCP**: `madhyamas_update_config` tool for AI-driven configuration changes

#### Advanced Traffic Filtering

- **API**: Extended `GET /api/traffic` with `file_type`, `header`, `cookie` query parameters
- **Frontend**: FilterBuilder visual UI with 10+ filter types (method, status, content-type, size, timing, etc.)
- **Frontend**: TrafficView multi-select with checkboxes, bulk delete, keyboard shortcuts (Ctrl+A, Delete, Escape)
- **Frontend**: Export dropdown (HAR, cURL formats)

#### Advanced Traffic Filtering (CLI & MCP)

- **CLI**: `madhyamas traffic list` now supports `--status`, `--file-type`, `--header`, `--cookie`, `--search`, `--min-size`, `--max-size`, `--min-time`, `--max-time` flags
- **MCP**: `madhyamas_get_traffic` tool extended with all advanced filter parameters

#### Network IP Detection

- **Core**: `ProxyConfig::detect_private_ip()` for automatic LAN IP detection
- **API**: `GET /api/config` now returns detected network IP for mobile device setup
- **Docs**: IP_DETECTION.md, LOCAL_DEVELOPMENT.md, NETWORK_CONFIGURATION.md

#### Docker Network IP Detection

- **Core**: `detect_private_ip()` now checks `MADHYAMAS_PUBLIC_IP` and `MADHYAMAS_HOST_IP` environment variables first
- **Core**: Improved IP detection to skip Docker bridge interfaces (docker0, br-_, veth_)
- **Core**: Added `is_docker()` helper to detect container environment
- **Docker**: Added `madhyamas-host` service with `network_mode: host` for automatic IP detection
- **Docker**: Added `docker/get-host-ip.sh` helper script to detect host LAN IP
- **Docker**: Updated `docker-compose.yml` with `MADHYAMAS_PUBLIC_IP` environment variable support

#### Developer Experience

- Local development scripts: `startup-local.sh`, `stop-local.sh`
- Enhanced `startup.sh` with `--clean` flag for fresh builds
- Certificate helper UI improvements for easier mobile device onboarding

## [0.1.0] - 2024-XX-XX

### Added

- Initial release
- Core proxy engine
- Basic web UI
- Traffic interception and display

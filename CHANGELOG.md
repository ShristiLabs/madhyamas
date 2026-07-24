# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- AI agent skills package (67 MCP tools, 58 CLI subcommands, 130+ REST API endpoints)
- skills.sh and npm publishing support for the skills package

## [0.1.3] - 2025-07-24

### Fixed

- Chocolatey install script now finds madhyamas.exe recursively after zip
  extraction (the zip contains a subdirectory, so the previous script
  pointed the shim to a non-existent path, causing verification failure)

## [0.1.2] - 2025-07-24

### Fixed

- Homebrew tap instructions corrected from `madhyamas/tap` to `ShristiLabs/tap`
  (the tap repo is at github.com/ShristiLabs/homebrew-tap)
- Homebrew formula now includes SHA256 checksums for Linux builds
  (previously only macOS checksums were injected, Linux had placeholders)
- Removed unnecessary `openssl@3` dependency from Homebrew formula
  (madhyamas uses rustls — pure Rust TLS, no openssl needed)
- Release notes now include extracted changelog section instead of a static link
- RPM filename in release notes no longer hardcoded (uses wildcard pattern)
- Debug RPM packages (debuginfo, debugsource) excluded from GitHub Release assets

### Added

- New `Release Dispatch` workflow for on-demand releases with version bumping
  and release notes consolidation from CHANGELOG.md and git history

## [0.1.1] - 2025-07-24

### Fixed

- Chocolatey package now includes correct SHA256 checksum for Windows zip
  (previously shipped with literal `__CHECKSUM__` placeholder, causing
  automated verification failure)
- Release workflow downloads the actual checksum from the GitHub Release
  and injects it into the chocolateyinstall.ps1 script before packing

## [0.1.0] - 2025-07-22

### Added

- Initial project structure with Rust workspace (core, api, cli, mcp, main binary)
- React + TypeScript web frontend with Vite, Tailwind CSS, shadcn/ui
- HTTP/HTTPS proxy functionality with HTTP/2 upstream support
- TLS certificate generation for HTTPS interception (ECDSA P-256, per-host leaf certs)
- Traffic capture and storage in SQLite (WAL mode)
- Web UI for traffic inspection with real-time WebSocket updates
- Breakpoint functionality for request/response interception
- Mock server capabilities with collections, recording, import/export
- URL rewriting rules
- Network throttling simulation (3G, 4G, DSL presets)
- Request replay functionality
- Session save/load with HAR export/import
- Docker support
- GitHub Actions CI/CD
- MCP server for AI agent integration (67 tools)
- CLI with 58 subcommands
- gRPC traffic inspection (experimental)
- JavaScript/TypeScript scripting (experimental)
- Plugin system (experimental)
- Android VPN companion app for transparent traffic routing
- Syntax-highlightlined JSON viewer with JSONPath and JMESPath queries
- Image preview for image responses
- Compression toggle (gzip/deflate/brotli)
- Copy as cURL/HTTPie/fetch/wget

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
- Pre-commit hooks for cargo fmt, clippy, and npm lint
- CI path filters to skip irrelevant builds

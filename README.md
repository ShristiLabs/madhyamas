# Madhyamas

**Open Source HTTP/HTTPS Debugging Proxy**

A high-performance, cross-platform HTTP/HTTPS debugging proxy built in Rust with a modern web-based UI. Madhyamas is the free, open-source alternative to tools like Charles Proxy and Fiddler.

## Features

### Core Capabilities

- **HTTP/HTTPS Traffic Interception** — Capture and inspect all HTTP/HTTPS traffic in real-time
- **TLS/SSL Certificate Generation** — Automatic on-the-fly certificate generation for HTTPS interception
- **Traffic Inspection UI** — Modern React-based web UI for viewing and analyzing traffic
- **Request/Response Filtering** — Filter by URL, method, status code, host, content type, duration, headers, and cookies
- **Real-time WebSocket Updates** — Live traffic streaming via WebSocket (no polling required)
- **HTTP/2 Upstream Support** — Full HTTP/2 support for upstream connections with ALPN negotiation

### Traffic Inspection

- **Syntax-Highlighted JSON Viewer** — Prism.js-powered syntax highlighting for JSON bodies with Code and Tree views
- **JSON Prettify/Minify** — Toggle between prettified (2-space indent) and minified JSON output
- **JSONPath Queries** — Filter and extract JSON data using JSONPath expressions (e.g., `$.store.book[*].title`)
- **JMESPath Queries** — Query JSON data using JMESPath expressions (e.g., `store.book[*].title`)
- **Image Preview** — Automatic image rendering for image responses (PNG, JPEG, GIF, WebP, SVG, ICO, BMP, AVIF, TIFF) with download support
- **Compression Toggle** — Decompress gzip/deflate/brotli response bodies on demand with a toggle button
- **Base64 Body Decoding** — Automatic decoding of base64-prefixed binary bodies
- **Request/Response Size Tracking** — Accurate size computation (headers + body) displayed in traffic list and detail views
- **Body Search** — Full-text search within request and response bodies
- **Copy as cURL/HTTPie/fetch/wget** — Export any request as a command-line command

### Traffic Manipulation

- **Breakpoints** — Pause requests/responses for inspection and modification before forwarding
- **Response Mocking** — Serve custom responses instead of hitting real servers; supports collections, recording, import/export
- **URL/Header Rewriting** — Automatically modify traffic based on rules
- **Bandwidth Throttling** — Simulate slow network conditions (3G, 4G, DSL presets)
- **Request Replay** — Re-execute captured requests with modifications

### SSL/TLS Error Visibility

- **Failed TLS Handshake Recording** — CONNECT requests that fail TLS handshake (e.g., Android apps with certificate pinning) are recorded as 502 traffic entries so they're visible in the UI, with a clear error message explaining the cause

### Advanced Features

- **WebSocket Traffic Capture** — Inspect WebSocket messages in real-time
- **gRPC Support** _(Experimental)_ — Debug gRPC/Protocol Buffer traffic with frame parsing
- **JavaScript/TypeScript Scripting** _(Experimental)_ — Automate traffic manipulation with scripts
- **Plugin System** _(Experimental)_ — Extend functionality with custom Rust plugins
- **MCP Server** — AI agent integration via Model Context Protocol

> **Note:** Features marked Experimental have partial implementations and may be incomplete.

### Session Management

- **Session Save/Load** — Persist and restore debugging sessions
- **HAR Export** — Export traffic in HAR format for sharing
- **Session Import** — Import previously exported sessions
- **cURL Export** — Generate cURL commands for any request
- **Persistence Layer** — SQLite-based traffic storage with configurable retention

### Security & Configuration

- **Configurable Rate Limiting** — Opt-in API rate limiting (disabled by default) with configurable requests/sec and burst size
- **CORS Protection** — Safe-origin CORS policy for the web UI
- **Security Headers** — X-Frame-Options, X-Content-Type-Options, Referrer-Policy headers
- **Request Body Size Limit** — 10MB limit to prevent OOM from large payloads
- **Enterprise Features** _(Experimental)_ — Authentication, user management, RBAC, audit logging, onboarding wizard

## Comparison with Other Tools

| Feature            | Madhyamas | Charles Proxy | mitmproxy   | Fiddler   | Proxyman   |
| ------------------ | --------- | ------------- | ----------- | --------- | ---------- |
| **Open Source**    | ✅        | ❌            | ✅          | ❌        | ❌         |
| **Free**           | ✅        | ❌ ($50)      | ✅          | ✅        | Freemium   |
| **Cross-Platform** | ✅        | ✅            | ✅          | Windows   | macOS      |
| **Web UI**         | ✅        | ❌            | Limited     | ❌        | ❌         |
| **Rust-Powered**   | ✅        | ❌ (Java)     | ❌ (Python) | ❌ (.NET) | ❌ (Swift) |
| **gRPC Support**   | ✅ (Experimental) | ❌            | ✅          | ❌        | ❌         |
| **WebSocket**      | ✅        | Limited       | ✅          | ✅        | ✅         |
| **Scripting**      | ✅ JS/TS (Experimental) | ❌            | ✅ Python   | ❌        | ❌         |
| **Plugin System**  | ✅ (Experimental) | ❌            | ✅          | ✅        | ❌         |
| **JSON Query**     | ✅ JSONPath + JMESPath | ❌            | ❌          | ❌        | ❌         |
| **Image Preview**  | ✅        | ✅            | ❌          | ✅        | ✅         |
| **MCP/AI Agent**   | ✅        | ❌            | ❌          | ❌        | ❌         |

## Requirements

- **Rust** 1.88+ (for building from source)
- **Node.js** 18+ (for web UI development)
- **Cargo** (comes with Rust)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/madhyamas/madhyamas.git
cd madhyamas

# Build the backend (includes embedded web UI)
cargo build --release

# Or build web UI separately for development
cd web
npm install
npm run build
```

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/madhyamas/madhyamas/releases) page.

### Snapshot Builds

Snapshot builds are automatically generated from the latest `main` branch and are available as CI artifacts. A single unified binary is built for each supported platform — it includes the proxy server, web UI (embedded), MCP server, and CLI.

#### Available Binary

| Binary          | Description                   | Use Case                                          |
| --------------- | ----------------------------- | ------------------------------------------------- |
| `madhyamas`     | Unified binary (proxy + web UI + MCP + CLI) | Full debugging proxy with browser-based interface, CLI, and AI agent integration |

#### Subcommands

```bash
madhyamas              # Start proxy server with web UI (default)
madhyamas serve        # Same as above
madhyamas mcp          # Run as MCP server (stdio)
madhyamas traffic list # CLI command
madhyamas --help       # See all commands
```

#### Platform Support

| Platform                | Target                          | Architecture                              | Binary            | Install Instructions                                                       |
| ----------------------- | ------------------------------- | ----------------------------------------- | ----------------- | -------------------------------------------------------------------------- |
| **Linux x86_64**        | `x86_64-unknown-linux-gnu`      | Intel/AMD 64-bit                          | `madhyamas`       | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/`         |
| **Linux ARM64**         | `aarch64-unknown-linux-gnu`     | ARM 64-bit (Pi 4/5, AWS Graviton)         | `madhyamas`       | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/`         |
| **Linux ARMv7**         | `armv7-unknown-linux-gnueabihf` | ARM 32-bit (Pi 2/3/4 32-bit)              | `madhyamas`       | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/`         |
| **Linux ARMv6**         | `arm-unknown-linux-gnueabihf`   | ARM 32-bit (Pi Zero/Zero W)               | `madhyamas`       | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/`         |
| **Linux RISC-V 64**     | `riscv64gc-unknown-linux-gnu`   | RISC-V 64-bit (LicheeRV Nano, VisionFive) | `madhyamas`       | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/`         |
| **macOS Intel**         | `x86_64-apple-darwin`           | Intel Mac                                 | `madhyamas`       | `tar -xzf madhyamas-*.tar.gz && mv madhyamas /usr/local/bin/`              |
| **macOS Apple Silicon** | `aarch64-apple-darwin`          | M1/M2/M3 Mac                              | `madhyamas`       | `tar -xzf madhyamas-*.tar.gz && mv madhyamas /usr/local/bin/`              |
| **Windows x64**         | `x86_64-pc-windows-msvc`        | Intel/AMD 64-bit                          | `madhyamas.exe`   | Extract ZIP, add folder to `PATH` or move to `C:\Program Files\Madhyamas\` |

#### Downloading Snapshot Builds

1. Go to [GitHub Actions](https://github.com/ShristiLabs/madhyamas/actions/workflows/ci.yml)
2. Click on the latest successful workflow run
3. Download the artifact for your platform (e.g., `madhyamas-x86_64-unknown-linux-gnu`)
4. Extract and install using the instructions above

## Usage

### Quick Start

```bash
# Terminal 1: Start the proxy server
cargo run --release

# Terminal 2: Start the web UI (for development)
cd web
npm run dev
```

### CLI Options

```
madhyamas [OPTIONS]

Options:
  -p, --proxy-port <PORT>     Port for the proxy server [default: 8888]
  -a, --api-port <PORT>       Port for the web UI API [default: 3001]
  -H, --host <HOST>           Host to bind to [default: 127.0.0.1]
  -c, --cert-path <PATH>      Certificate storage path [default: ~/.madhyamas/certs]
  -d, --db-path <PATH>        Database path for traffic storage [default: ~/.madhyamas/traffic.db]
  -l, --log-path <PATH>       Log file path [default: ~/.madhyamas/logs]
  -m, --max-requests <NUM>    Maximum requests to keep in memory [default: 10000]
  -v, --verbose               Enable verbose logging
      --no-https              Disable HTTPS interception
      --rate-limit            Enable API rate limiting (disabled by default)
      --rate-limit-rps <NUM>  Rate limit: max requests per second per peer IP [default: 600]
      --rate-limit-burst <N>  Rate limit: burst size [default: 1000]
  -h, --help                  Print help
  -V, --version               Print version
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Logging level (trace/debug/info/warn/error) | `info` |
| `MADHYAMAS_HOST` | Bind host | `127.0.0.1` |
| `MADHYAMAS_API_PORT` | API port | `3001` |
| `MADHYAMAS_PROXY_PORT` | Proxy port | `8888` |
| `MADHYAMAS_PUBLIC_IP` | Public IP shown to users for remote access | — |
| `MADHYAMAS_API_URL` | API URL for CLI/MCP modes | `http://127.0.0.1:3001` |
| `MADHYAMAS_WEB_DIR` | Override web asset directory (dev only) | — |
| `MADHYAMAS_RATE_LIMIT` | Enable API rate limiting | `false` |
| `MADHYAMAS_RATE_LIMIT_RPS` | Rate limit requests per second | `600` |
| `MADHYAMAS_RATE_LIMIT_BURST` | Rate limit burst size | `1000` |

### Data Directory

Madhyamas stores all runtime data in `~/.madhyamas/` by default:

```
~/.madhyamas/
├── certs/              # TLS certificates (CA cert and keys)
│   ├── madhyamas-ca.pem
│   └── madhyamas-ca-key.pem
├── logs/               # Application logs
└── traffic.db          # SQLite database for traffic storage
```

### Configuring Your Client

1. Configure your browser or application to use the proxy:
   - **HTTP Proxy**: `localhost:8888`
   - **HTTPS Proxy**: `localhost:8888`

2. For HTTPS interception, install the CA certificate:
   - Certificate location: `~/.madhyamas/certs/madhyamas-ca.pem`
   - Download via API: `GET http://localhost:3001/api/cert/ca`

3. Open the web UI: `http://localhost:3001`

### Mobile Device Setup (Android/iOS)

1. Connect your mobile device to the same network as the machine running Madhyamas
2. Configure the device's Wi-Fi proxy to use the machine's IP and port 8888
3. Install the CA certificate on the device:
   - **Android**: Settings → Security → Install certificate → CA certificate → select `madhyamas-ca.pem`
   - **iOS**: Download the certificate via Safari, then Settings → Profile Downloaded → Install, then Settings → General → About → Certificate Trust Settings → enable trust
4. Note: Some Android apps use certificate pinning and will reject the proxy's CA. Failed TLS handshakes from these apps will appear in the traffic panel as 502 entries with an explanatory error message.

## API Endpoints

### Traffic

| Method | Endpoint             | Description              |
| ------ | -------------------- | ------------------------ |
| GET    | `/api/traffic`       | List all traffic entries |
| GET    | `/api/traffic/:id`   | Get single traffic entry |
| POST   | `/api/traffic/clear` | Clear all traffic        |
| GET    | `/api/traffic/count` | Get traffic count        |

### Sessions

| Method | Endpoint                   | Description           |
| ------ | -------------------------- | --------------------- |
| GET    | `/api/sessions`            | List all sessions     |
| POST   | `/api/sessions`            | Create new session    |
| GET    | `/api/sessions/:id`        | Get session details   |
| DELETE | `/api/sessions/:id`        | Delete session        |
| GET    | `/api/sessions/:id/export` | Export session        |
| POST   | `/api/sessions/:id/switch` | Switch active session |
| POST   | `/api/sessions/import`     | Import session        |

### Export

| Method | Endpoint               | Description            |
| ------ | ---------------------- | ---------------------- |
| GET    | `/api/export/har`      | Export traffic as HAR  |
| GET    | `/api/export/curl/:id` | Export request as cURL |

### Interception

| Method          | Endpoint           | Description             |
| --------------- | ------------------ | ----------------------- |
| GET/POST/DELETE | `/api/breakpoints` | Manage breakpoint rules |
| GET/POST        | `/api/mocks`       | Manage mock rules       |
| GET/POST/DELETE | `/api/rewrites`    | Manage rewrite rules    |
| GET/POST        | `/api/throttle`    | Manage throttling       |

### Replay

| Method   | Endpoint                  | Description           |
| -------- | ------------------------- | --------------------- |
| GET/POST | `/api/replay/saved`       | Manage saved requests |
| POST     | `/api/replay/execute/:id` | Replay a request      |
| GET      | `/api/replay/history`     | View replay history   |

### WebSocket & gRPC

| Method | Endpoint                      | Description                |
| ------ | ----------------------------- | -------------------------- |
| GET    | `/api/ws-traffic/connections` | List WebSocket connections |
| GET    | `/api/grpc/connections`       | List gRPC connections      |
| GET    | `/api/grpc/streams`           | List gRPC streams          |

### Configuration & Capture

| Method | Endpoint              | Description                  |
| ------ | --------------------- | ---------------------------- |
| GET    | `/api/config`         | Get proxy configuration      |
| PATCH  | `/api/config`         | Update proxy configuration   |
| GET    | `/api/capture`        | Get capture status           |
| POST   | `/api/capture/toggle` | Toggle traffic capture       |
| GET    | `/api/cert/ca`        | Download CA certificate      |

### Real-time Updates

| Method | Endpoint  | Description                             |
| ------ | --------- | --------------------------------------- |
| GET    | `/api/ws` | WebSocket for real-time traffic updates |

## MCP Server for AI Agents

Madhyamas includes a built-in MCP (Model Context Protocol) server that allows AI agents like Claude to interact with the proxy directly.

### Running the MCP Server

```bash
# Run the MCP server (stdio transport)
madhyamas mcp

# Or with custom API URL
MADHYAMAS_API_URL=http://localhost:3001 madhyamas mcp

# During development
cargo run --bin madhyamas -- mcp
```

### Configuring Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/path/to/madhyamas",
      "args": ["mcp"]
    }
  }
}
```

### Available MCP Tools

| Tool                            | Description                                       |
| ------------------------------- | ------------------------------------------------- |
| `madhyamas_get_traffic`         | List captured traffic with optional filtering     |
| `madhyamas_get_traffic_entry`   | Get full details of a specific request/response   |
| `madhyamas_search_traffic`      | Search traffic by content (headers, bodies, URLs) |
| `madhyamas_get_traffic_count`   | Get total count of captured requests              |
| `madhyamas_clear_traffic`       | Clear all captured traffic                        |
| `madhyamas_list_mocks`          | List all mock rules                               |
| `madhyamas_create_mock`         | Create a mock response rule                       |
| `madhyamas_delete_mock`         | Delete a mock rule                                |
| `madhyamas_toggle_mock`         | Enable/disable a mock rule                        |
| `madhyamas_list_breakpoints`    | List all breakpoint rules                         |
| `madhyamas_create_breakpoint`   | Create a breakpoint to pause traffic              |
| `madhyamas_delete_breakpoint`   | Delete a breakpoint rule                          |
| `madhyamas_replay_request`      | Replay a captured request                         |
| `madhyamas_save_request`        | Save a request for later replay                   |
| `madhyamas_list_saved_requests` | List all saved requests                           |
| `madhyamas_list_sessions`       | List all debugging sessions                       |
| `madhyamas_create_session`      | Create a new session                              |
| `madhyamas_export_session`      | Export a session as HAR                           |
| `madhyamas_import_session`      | Import a session from HAR                         |
| `madhyamas_switch_session`      | Switch the active session                         |
| `madhyamas_export_curl`         | Export a request as a cURL command                |
| `madhyamas_get_config`          | Get current proxy configuration                   |

### Example Usage with AI Agents

Once configured, AI agents can use Madhyamas to:

- **Debug API issues**: "Show me all failed requests to /api/users in the last 10 minutes"
- **Create mocks**: "Mock all requests to /api/auth to return a valid token"
- **Replay requests**: "Replay the login request with different credentials"
- **Analyze patterns**: "What are the most common API endpoints being called?"
- **Export for sharing**: "Export the last 50 requests as HAR format"

### CLI for AI Agents

Madhyamas also provides a comprehensive CLI for AI agents that prefer shell commands:

```bash
# View captured traffic
madhyamas traffic list
madhyamas traffic get <id>
madhyamas traffic search "api.example.com"
madhyamas traffic count
madhyamas traffic clear

# Manage mocks
madhyamas mock list
madhyamas mock create --url "*/api/*" --status 200 --body '{"ok":true}'
madhyamas mock delete <id>
madhyamas mock toggle <id> --enabled true

# Manage breakpoints
madhyamas breakpoint list
madhyamas breakpoint create --url "*/auth*" --direction request
madhyamas breakpoint delete <id>

# Manage sessions
madhyamas session list
madhyamas session create --name "debug-auth"
madhyamas session switch <id>
madhyamas session export <id> --format har
```

All commands support `--json` flag for machine-readable output.

## Project Structure

```
madhyamas/
├── Cargo.toml                 # Workspace configuration
├── crates/
│   ├── madhyamas/             # Unified binary (proxy + web UI + MCP + CLI)
│   ├── madhyamas-core/        # Core proxy engine (Rust)
│   ├── madhyamas-api/         # REST/WebSocket API + embedded web assets (Rust)
│   ├── madhyamas-cli/         # CLI library (re-exported by main binary)
│   └── madhyamas-mcp/         # MCP server library (re-exported by main binary)
├── web/                       # React frontend (embedded at compile time)
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
├── docs/                      # Documentation
├── docker/                    # Docker setup
└── README.md
```

## Technology Stack

### Backend (Rust)

- **axum** — Web framework
- **hyper** — HTTP server/client
- **tokio** — Async runtime
- **rustls** — TLS implementation
- **rcgen** — Certificate generation
- **rusqlite** — SQLite storage
- **clap** — CLI framework
- **reqwest** — HTTP client for upstream requests (gzip/deflate/brotli support)
- **tower-governor** — Rate limiting (opt-in)

### Frontend (React)

- **React 18** — UI framework
- **TypeScript** — Type safety
- **Vite** — Build tool
- **Tailwind CSS** — Styling
- **shadcn/ui** — UI components
- **TanStack Query** — Data fetching
- **Zustand** — State management
- **Prism.js** — Syntax highlighting for JSON viewer
- **react-json-view-lite** — Collapsible JSON tree view
- **jsonpath-plus** — JSONPath query engine
- **jmespath** — JMESPath query engine

## Development

```bash
# Run tests
cargo test

# Run with debug logging
cargo run -- --verbose

# Build web UI for production
cd web && npm run build

# Build the full binary (embeds web UI)
cargo build --release

# Lint
cargo clippy --all-targets --all-features
cargo fmt --all
```

### Development Workflow

The web UI is embedded into the Rust binary at compile time via `rust-embed`. For development:

1. Run the web UI dev server: `cd web && npm run dev`
2. Run the Rust backend: `cargo run -- --verbose`
3. The backend serves the web UI at `http://localhost:3001`
4. For production builds, always build the web UI first (`cd web && npm run build`), then rebuild the Rust binary

### Git Hooks (Pre-commit Checks)

A pre-commit hook is provided to catch formatting and clippy issues before they reach CI. To install:

```bash
./hooks/install.sh
```

This installs a `pre-commit` hook that runs:

- **`cargo fmt --all -- --check`** — fails if any Rust file is not formatted
- **`cargo clippy --all-targets --all-features -- -D warnings`** — fails on any clippy warning
- **`npm run lint`** — fails on frontend lint issues (when web files are changed)

The hook only runs when `.rs` files or frontend config files are staged. To bypass temporarily:

```bash
git commit --no-verify
```

## License

Dual-licensed under MIT OR Apache-2.0.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## Support

- **Issues**: [GitHub Issues](https://github.com/madhyamas/madhyamas/issues)
- **Discussions**: [GitHub Discussions](https://github.com/madhyamas/madhyamas/discussions)

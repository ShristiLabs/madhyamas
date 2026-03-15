# Madhyamas

**Open Source HTTP/HTTPS Debugging Proxy**

A high-performance, cross-platform HTTP/HTTPS debugging proxy built in Rust with a modern web-based UI. Madhyamas is the free, open-source alternative to tools like Charles Proxy and Fiddler.

## Features

### Core Capabilities
- **HTTP/HTTPS Traffic Interception** - Capture and inspect all HTTP/HTTPS traffic in real-time
- **TLS/SSL Certificate Generation** - Automatic on-the-fly certificate generation for HTTPS interception
- **Traffic Inspection UI** - Modern React-based web UI for viewing and analyzing traffic
- **Request/Response Filtering** - Filter by URL, method, status code, and more

### Traffic Manipulation
- **Breakpoints** - Pause requests/responses for inspection and modification
- **Response Mocking** - Serve custom responses instead of hitting real servers
- **URL/Header Rewriting** - Automatically modify traffic based on rules
- **Bandwidth Throttling** - Simulate slow network conditions (3G, 4G, DSL presets)
- **Request Replay** - Re-execute captured requests with modifications

### Advanced Features
- **WebSocket Traffic Capture** - Inspect WebSocket messages in real-time
- **gRPC Support** - Debug gRPC/Protocol Buffer traffic with frame parsing
- **JavaScript/TypeScript Scripting** - Automate traffic manipulation with scripts
- **Plugin System** - Extend functionality with custom Rust plugins
- **MCP Server** - AI agent integration via Model Context Protocol

### Session Management
- **Session Save/Load** - Persist and restore debugging sessions
- **HAR Export** - Export traffic in HAR format for sharing
- **cURL Export** - Generate cURL commands for any request
- **Persistence Layer** - SQLite-based traffic storage

## Comparison with Other Tools

| Feature | Madhyamas | Charles Proxy | mitmproxy | Fiddler | Proxyman |
|---------|------------|---------------|-----------|---------|----------|
| **Open Source** | ✅ | ❌ | ✅ | ❌ | ❌ |
| **Free** | ✅ | ❌ ($50) | ✅ | ✅ | Freemium |
| **Cross-Platform** | ✅ | ✅ | ✅ | Windows | macOS |
| **Web UI** | ✅ | ❌ | Limited | ❌ | ❌ |
| **Rust-Powered** | ✅ | ❌ (Java) | ❌ (Python) | ❌ (.NET) | ❌ (Swift) |
| **gRPC Support** | ✅ | ❌ | ✅ | ❌ | ❌ |
| **WebSocket** | ✅ | Limited | ✅ | ✅ | ✅ |
| **Scripting** | ✅ JS/TS | ❌ | ✅ Python | ❌ | ❌ |
| **Plugin System** | ✅ | ❌ | ✅ | ✅ | ❌ |

## Requirements

- **Rust** 1.75+ (for building from source)
- **Node.js** 18+ (for web UI development)
- **Cargo** (comes with Rust)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/madhyamas/madhyamas.git
cd madhyamas

# Build the backend
cargo build --release

# Install web UI dependencies
cd web
npm install
```

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/madhyamas/madhyamas/releases) page.

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
  -h, --help                  Print help
  -V, --version               Print version
```

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

## API Endpoints

### Traffic
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/traffic` | List all traffic entries |
| GET | `/api/traffic/:id` | Get single traffic entry |
| POST | `/api/traffic/clear` | Clear all traffic |
| GET | `/api/traffic/count` | Get traffic count |

### Sessions
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/sessions` | List all sessions |
| POST | `/api/sessions` | Create new session |
| GET | `/api/sessions/:id` | Get session details |
| DELETE | `/api/sessions/:id` | Delete session |
| GET | `/api/sessions/:id/export` | Export session |
| POST | `/api/sessions/:id/switch` | Switch active session |

### Export
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/export/har` | Export traffic as HAR |
| GET | `/api/export/curl/:id` | Export request as cURL |

### Interception
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST/DELETE | `/api/breakpoints` | Manage breakpoint rules |
| GET/POST | `/api/mocks` | Manage mock rules |
| GET/POST/DELETE | `/api/rewrites` | Manage rewrite rules |
| GET/POST | `/api/throttle` | Manage throttling |

### Replay
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET/POST | `/api/replay/saved` | Manage saved requests |
| POST | `/api/replay/execute/:id` | Replay a request |
| GET | `/api/replay/history` | View replay history |

### WebSocket & gRPC
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/ws-traffic/connections` | List WebSocket connections |
| GET | `/api/grpc/connections` | List gRPC connections |
| GET | `/api/grpc/streams` | List gRPC streams |

### Real-time Updates
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/ws` | WebSocket for real-time traffic updates |

## MCP Server for AI Agents

Madhyamas includes a built-in MCP (Model Context Protocol) server that allows AI agents like Claude to interact with the proxy directly.

### Running the MCP Server

```bash
# Build and run the MCP server
cargo run --bin madhyamas-mcp

# Or with custom API URL
MADHYAMAS_API_URL=http://localhost:3001 cargo run --bin madhyamas-mcp
```

### Configuring Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/path/to/madhyamas-mcp"
    }
  }
}
```

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `madhyamas_get_traffic` | List captured traffic with optional filtering |
| `madhyamas_get_traffic_entry` | Get full details of a specific request/response |
| `madhyamas_search_traffic` | Search traffic by content (headers, bodies, URLs) |
| `madhyamas_get_traffic_count` | Get total count of captured requests |
| `madhyamas_clear_traffic` | Clear all captured traffic |
| `madhyamas_list_mocks` | List all mock rules |
| `madhyamas_create_mock` | Create a mock response rule |
| `madhyamas_delete_mock` | Delete a mock rule |
| `madhyamas_toggle_mock` | Enable/disable a mock rule |
| `madhyamas_list_breakpoints` | List all breakpoint rules |
| `madhyamas_create_breakpoint` | Create a breakpoint to pause traffic |
| `madhyamas_delete_breakpoint` | Delete a breakpoint rule |
| `madhyamas_replay_request` | Replay a captured request |
| `madhyamas_save_request` | Save a request for later replay |
| `madhyamas_list_saved_requests` | List all saved requests |
| `madhyamas_list_sessions` | List all debugging sessions |
| `madhyamas_create_session` | Create a new session |
| `madhyamas_export_session` | Export a session as HAR |
| `madhyamas_import_session` | Import a session from HAR |
| `madhyamas_switch_session` | Switch the active session |
| `madhyamas_export_curl` | Export a request as a cURL command |
| `madhyamas_get_config` | Get current proxy configuration |

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
│   ├── madhyamas-core/       # Core proxy engine (Rust)
│   ├── madhyamas-api/        # REST/WebSocket API (Rust)
│   └── madhyamas-cli/        # CLI entry point (Rust)
├── web/                        # React frontend
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
└── README.md
```

## Technology Stack

### Backend (Rust)
- **axum** - Web framework
- **hyper** - HTTP server/client
- **tokio** - Async runtime
- **rustls** - TLS implementation
- **rcgen** - Certificate generation
- **rusqlite** - SQLite storage
- **clap** - CLI framework

### Frontend (React)
- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool
- **Tailwind CSS** - Styling
- **shadcn/ui** - UI components
- **TanStack Query** - Data fetching
- **Zustand** - State management

## Development

```bash
# Run tests
cargo test

# Run with debug logging
cargo run -- --verbose

# Build web UI for production
cd web && npm run build
```

## License

Dual-licensed under MIT OR Apache-2.0.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## Support

- **Issues**: [GitHub Issues](https://github.com/madhyamas/madhyamas/issues)
- **Discussions**: [GitHub Discussions](https://github.com/madhyamas/madhyamas/discussions)

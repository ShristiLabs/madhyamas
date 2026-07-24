# Getting Started with Madhyamas

## Requirements

- **Rust** 1.88+ (for building from source)
- **Node.js** 18+ (for web UI development)
- **Cargo** (comes with Rust)

## Installation

### From Source

```bash
git clone https://github.com/ShristiLabs/madhyamas.git
cd madhyamas
cargo build --release
# Binary: target/release/madhyamas
```

Build the web UI first if modifying frontend:
```bash
cd web && npm install && npm run build
cargo build --release
```

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/ShristiLabs/madhyamas/releases) page.

### Package Managers

```bash
# macOS (Homebrew)
brew tap ShristiLabs/tap
brew install madhyamas

# Windows (Chocolatey)
choco install madhyamas

# Linux (Snap)
sudo snap install madhyamas

# Arch Linux (AUR)
yay -S madhyamas
```

> **Note:** Package manager availability depends on the package being published. If not yet available, use [pre-built binaries](#pre-built-binaries) or [build from source](#from-source).

### Snapshot Builds

Snapshot builds are automatically generated from the latest `main` branch and are available as CI artifacts. A single unified binary is built for each supported platform — it includes the proxy server, web UI (embedded), MCP server, and CLI.

#### Available Binary

| Binary | Description | Use Case |
|--------|-------------|----------|
| `madhyamas` | Unified binary (proxy + web UI + MCP + CLI) | Full debugging proxy with browser-based interface, CLI, and AI agent integration |

#### Subcommands

```bash
madhyamas              # Start proxy server with web UI (default)
madhyamas serve        # Same as above
madhyamas mcp          # Run as MCP server (stdio)
madhyamas traffic list # CLI command
madhyamas --help       # See all commands
```

#### Platform Support

| Platform | Target | Architecture | Binary | Install Instructions |
|----------|--------|-------------|--------|---------------------|
| **Linux x86_64** | `x86_64-unknown-linux-gnu` | Intel/AMD 64-bit | `madhyamas` | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/` |
| **Linux ARM64** | `aarch64-unknown-linux-gnu` | ARM 64-bit (Pi 4/5, AWS Graviton) | `madhyamas` | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/` |
| **Linux ARMv7** | `armv7-unknown-linux-gnueabihf` | ARM 32-bit (Pi 2/3/4 32-bit) | `madhyamas` | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/` |
| **Linux ARMv6** | `arm-unknown-linux-gnueabihf` | ARM 32-bit (Pi Zero/Zero W) | `madhyamas` | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/` |
| **Linux RISC-V 64** | `riscv64gc-unknown-linux-gnu` | RISC-V 64-bit | `madhyamas` | `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/` |
| **macOS Intel** | `x86_64-apple-darwin` | Intel Mac | `madhyamas` | `tar -xzf madhyamas-*.tar.gz && mv madhyamas /usr/local/bin/` |
| **macOS Apple Silicon** | `aarch64-apple-darwin` | M1/M2/M3 Mac | `madhyamas` | `tar -xzf madhyamas-*.tar.gz && mv madhyamas /usr/local/bin/` |
| **Windows x64** | `x86_64-pc-windows-msvc` | Intel/AMD 64-bit | `madhyamas.exe` | Extract ZIP, add folder to `PATH` or move to `C:\Program Files\Madhyamas\` |

#### Downloading Snapshot Builds

1. Go to [GitHub Actions](https://github.com/ShristiLabs/madhyamas/actions/workflows/ci.yml)
2. Click on the latest successful workflow run
3. Download the artifact for your platform (e.g., `madhyamas-x86_64-unknown-linux-gnu`)
4. Extract and install using the instructions above

## Quick Start

```bash
# Terminal 1: Start the proxy server
cargo run --release

# Terminal 2: Start the web UI (for development)
cd web
npm run dev
```

1. Start Madhyamas: `madhyamas`
2. Configure your browser to use `localhost:8888` as proxy
3. Open `http://localhost:3001` in your browser
4. Install the root CA certificate when prompted

## CLI Options

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

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Logging level (trace/debug/info/warn/error) | `info` |
| `MADHYAMAS_HOST` | Bind host | `127.0.0.1` |
| `MADHYAMAS_API_PORT` | API port | `3001` |
| `MADHYAMAS_PROXY_PORT` | Proxy port | `8888` |
| `MADHYAMAS_PUBLIC_IP` | Public IP shown to users for remote access | auto-detected |
| `MADHYAMAS_API_URL` | API URL for CLI/MCP modes | `http://127.0.0.1:3001` |
| `MADHYAMAS_WEB_DIR` | Override web asset directory (dev only) | embedded |
| `MADHYAMAS_RATE_LIMIT` | Enable API rate limiting | `false` |
| `MADHYAMAS_RATE_LIMIT_RPS` | Rate limit requests per second | `600` |
| `MADHYAMAS_RATE_LIMIT_BURST` | Rate limit burst size | `1000` |

## Data Directory

Madhyamas stores all runtime data in `~/.madhyamas/` by default:

```
~/.madhyamas/
├── certs/              # TLS certificates (CA cert and keys)
│   ├── madhyamas-ca.pem
│   └── madhyamas-ca-key.pem
├── logs/               # Application logs
└── traffic.db          # SQLite database for traffic storage
```

## Configuring Your Client

1. Configure your browser or application to use the proxy:
   - **HTTP Proxy**: `localhost:8888`
   - **HTTPS Proxy**: `localhost:8888`

2. For HTTPS interception, install the CA certificate:
   - Certificate location: `~/.madhyamas/certs/madhyamas-ca.pem`
   - Download via API: `GET http://localhost:3001/api/cert/ca`

3. Open the web UI: `http://localhost:3001`

## Mobile Device Setup (Android/iOS)

### Option A: Madhyamas VPN App (Android, no root)

An Android companion app uses VpnService to transparently route traffic to the Madhyamas proxy — no manual proxy configuration needed.

1. Build and install the companion app:
   ```bash
   cd android
   echo "sdk.dir=$HOME/Library/Android/sdk" > local.properties
   ./gradlew assembleDebug
   adb install app/build/outputs/apk/debug/app-debug.apk
   ```
2. Open the app → Settings → set proxy host to your computer's IP
3. Tap "Install CA Certificate" to install the Madhyamas CA
4. Select apps to intercept (or leave on "All Apps")
5. Tap "Start VPN" and approve the VPN connection dialog

For apps with certificate pinning, see [ANDROID_CERT_PINNING.md](ANDROID_CERT_PINNING.md) for bypass guides (Frida, APK patching, Magisk modules, Flutter-specific approaches).

### Option B: Manual Proxy Configuration

1. Connect your mobile device to the same network as the machine running Madhyamas
2. Configure the device's Wi-Fi proxy to use the machine's IP and port 8888
3. Install the CA certificate on the device:
   - **Android**: Settings → Security → Install certificate → CA certificate → select `madhyamas-ca.pem`
   - **iOS**: Download the certificate via Safari, then Settings → Profile Downloaded → Install, then Settings → General → About → Certificate Trust Settings → enable trust
4. Note: Some Android apps use certificate pinning and will reject the proxy's CA. Failed TLS handshakes from these apps will appear in the traffic panel as 502 entries with an explanatory error message.

## Basic Usage

### View Traffic
1. Open the web UI at http://localhost:3001
2. Traffic appears in real-time in the list
3. Click any request to view details

### Set Breakpoints
1. Click "Breakpoints" in the toolbar
2. Add a new breakpoint rule
3. Enter URL pattern (e.g., `api.example.com/*`)
4. When traffic matches, the request pauses
5. Modify the request/response as needed
6. Click "Resume" to continue

### Mock Responses
1. Click "Mocks" in the toolbar
2. Create a new mock rule
3. Set URL pattern and response
4. Toggle the mock to enabled
5. Matching requests will receive the mock response

### Throttle Bandwidth
1. Click "Throttle" in the toolbar
2. Select a preset (3G, 4G, DSL)
3. Or create custom throttling profile
4. Toggle throttling on

## Tips
- Use filters to find specific requests quickly
- Export sessions as HAR files for sharing
- Use the search bar to find text in request/response bodies
- Set up rewrite rules to automate common modifications

## Troubleshooting

### Certificate Errors
If you see certificate warnings in your browser:
1. Open Madhyamas web UI
2. Click "Install Certificate" in the header
3. Follow the instructions for your OS

### Connection Issues
If traffic isn't appearing:
1. Check proxy settings in your browser/app
2. Verify Madhyamas is running: `curl http://localhost:3001/api/health`
3. Check firewall rules

### Performance Issues
If the proxy is slow:
1. Check memory usage in settings
2. Reduce max_entries if needed
3. Clear old traffic data

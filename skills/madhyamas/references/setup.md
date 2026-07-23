# Setup

## Installation

### From Source

```bash
git clone https://github.com/ShristiLabs/madhyamas.git
cd madhyamas
cargo build --release -p madhyamas
# Binary: target/release/madhyamas
```

Build the web UI first if modifying frontend:
```bash
cd web && npm install && npm run build
cargo build --release -p madhyamas
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/ShristiLabs/madhyamas/releases). Available platforms:

| Platform | Target | Binary |
|----------|--------|--------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `madhyamas` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `madhyamas` |
| Linux ARMv7 | `armv7-unknown-linux-gnueabihf` | `madhyamas` |
| macOS Intel | `x86_64-apple-darwin` | `madhyamas` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `madhyamas` |
| Windows x64 | `x86_64-pc-windows-msvc` | `madhyamas.exe` |

Install: `tar -xzf madhyamas-*.tar.gz && sudo mv madhyamas /usr/local/bin/`

### Docker

```bash
./startup.sh    # Build and start with Docker Compose
./stop.sh       # Stop containers
```

## Starting the Server

```bash
madhyamas              # Start proxy + web UI (default)
madhyamas serve        # Same as above
madhyamas mcp          # Run as MCP server (stdio transport)
madhyamas --help       # See all commands
```

### CLI Options

```
madhyamas [OPTIONS]

Options:
  -p, --proxy-port <PORT>     Proxy server port [default: 8888]
  -a, --api-port <PORT>       Web UI API port [default: 3001]
  -H, --host <HOST>           Bind host [default: 127.0.0.1]
  -c, --cert-path <PATH>      Certificate storage path [default: ~/.madhyamas/certs]
  -d, --db-path <PATH>        Database path [default: ~/.madhyamas/traffic.db]
  -l, --log-path <PATH>       Log file path [default: ~/.madhyamas/logs]
  -m, --max-requests <NUM>    Max requests in memory [default: 10000]
  -v, --verbose               Enable verbose logging
      --no-https              Disable HTTPS interception
      --rate-limit            Enable API rate limiting (disabled by default)
      --rate-limit-rps <NUM>  Rate limit: requests/sec [default: 600]
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
| `MADHYAMAS_PUBLIC_IP` | Public IP shown for remote access | auto-detect |
| `MADHYAMAS_API_URL` | API URL for CLI/MCP modes | `http://127.0.0.1:3001` |
| `MADHYAMAS_WEB_DIR` | Override web asset directory (dev only) | embedded |
| `MADHYAMAS_TIMEOUT` | MCP request timeout in seconds | `30` |

## Data Directory

```
~/.madhyamas/
├── certs/                  # TLS certificates
│   ├── madhyamas-ca.pem    # CA certificate (install in trust store)
│   └── madhyamas-ca-key.pem # CA private key
├── logs/                   # Application logs
└── traffic.db              # SQLite database for traffic storage
```

## CA Certificate Installation

For HTTPS interception, install the Madhyamas CA certificate in your system/browser trust store.

**Download the certificate:**
```bash
curl -o madhyamas-ca.pem http://localhost:3001/api/cert/ca
# Or find it at ~/.madhyamas/certs/madhyamas-ca.pem
```

### macOS

```bash
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ~/.madhyamas/certs/madhyamas-ca.pem
```

### Windows

```powershell
Import-Certificate -FilePath "$env:USERPROFILE\.madhyamas\certs\madhyamas-ca.pem" `
  -CertStoreLocation Cert:\LocalMachine\Root
```

### Linux (Ubuntu/Debian)

```bash
sudo cp ~/.madhyamas/certs/madhyamas-ca.pem /usr/local/share/ca-certificates/madhyamas-ca.crt
sudo update-ca-certificates
```

### Android

Settings → Security → Install certificate → CA certificate → select `madhyamas-ca.pem`.

For the companion VPN app (no root required):
```bash
cd android
echo "sdk.dir=$HOME/Library/Android/sdk" > local.properties
./gradlew assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk
```

### iOS

1. Download cert via Safari: `http://<host>:3001/api/cert/ca`
2. Settings → Profile Downloaded → Install
3. Settings → General → About → Certificate Trust Settings → enable trust

## Client Configuration

### Browser

Configure HTTP/HTTPS proxy to `localhost:8888`:
- **Firefox**: Settings → Network Settings → Manual proxy configuration
- **Chrome/Edge**: System proxy settings or launch with `--proxy-server=localhost:8888`
- **Safari**: System Preferences → Network → Advanced → Proxies

### Mobile Device

1. Connect device to same network as the machine running Madhyamas
2. Configure Wi-Fi proxy to machine's IP and port 8888
3. Install CA certificate on device (see above)

Note: Apps with certificate pinning will reject the proxy CA. Failed TLS handshakes appear as 502 entries in the traffic panel.

## Verification

```bash
# Check if proxy is running
curl http://localhost:3001/api/health
# Returns: OK

# Check configuration
curl http://localhost:3001/api/config

# View captured traffic count
curl http://localhost:3001/api/traffic/count

# CLI equivalent
madhyamas traffic count
```

## MCP Server Setup

The MCP server connects to a running Madhyamas proxy instance via its REST API.

```bash
# Start proxy in one terminal
madhyamas serve

# Run MCP server (connects to localhost:3001)
madhyamas mcp

# Or with custom API URL
MADHYAMAS_API_URL=http://192.168.1.100:3001 madhyamas mcp
```

See [harness-setup.md](harness-setup.md) for per-harness MCP configuration files (Claude Desktop, Windsurf, Cursor, etc.).

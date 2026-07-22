# Getting Started with Madhyamas

## Installation

### macOS (Homebrew)
```bash
brew tap madhyamas/tap
brew install madhyamas
```

### Linux (AUR)
```bash
yay -S madhyamas
```

### Ubuntu/Debian (Snap)
```bash
sudo snap install madhyamas
```

### Docker
```bash
docker run -d -p 3001:3001 -p 8888:8888 madhyamas/madhyamas
```

### From Source
```bash
git clone https://github.com/madhyamas/madhyamas.git
cd madhyamas
cargo build --release
```

## Quick Start
1. Start Madhyamas: `madhyamas`
2. Configure your browser to use `localhost:8888` as proxy
3. Open `http://localhost:3001` in your browser
4. Install the root CA certificate when prompted

## Configuration

> **Note:** Configuration file (`config.toml`) support is not yet implemented. Madhyamas is configured via CLI flags and environment variables.

### CLI Flags

```
madhyamas [OPTIONS]

Options:
  -p, --proxy-port <PORT>     Port for the proxy server [default: 8888]
  -a, --api-port <PORT>       Port for the web UI API [default: 3001]
  -H, --host <HOST>           Host to bind to [default: 127.0.0.1]
  -v, --verbose               Enable verbose logging
      --no-https              Disable HTTPS interception
  -h, --help                  Print help
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Logging level |
| `MADHYAMAS_HOST` | `127.0.0.1` | Bind host |
| `MADHYAMAS_API_PORT` | `3001` | API/Web UI port |
| `MADHYAMAS_PROXY_PORT` | `8888` | Proxy port |
| `MADHYAMAS_PUBLIC_IP` | auto-detected | Public IP for display |

## Basic Usage
### View Traffic
1. Open the web UI at http://localhost:3001
2. Traffic appears in real-time in the list
3. Click any request to view details

### Set Breakpoints
1. Click "Breakpoints" in the toolbar
2. Add a new breakpoint rule
3. Enter URL pattern (e.g., `api.example.com/*`)
4. When traffic matches, it request pauses
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
If traffic isn't appearing
1. Check proxy settings in your browser/app
2. Verify Madhyamas is running: `curl http://localhost:3001/api/health`
3. Check firewall rules

### Performance Issues
If the proxy is slow
1. Check memory usage in settings
2. Reduce max_entries if needed
3. Clear old traffic data

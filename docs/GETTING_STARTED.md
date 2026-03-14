# Getting Started with ProxyForge

## Installation

### macOS (Homebrew)
```bash
brew tap proxyforge/tap
brew install proxyforge
```

### Linux (AUR)
```bash
yay -S proxyforge
```

### Ubuntu/Debian (Snap)
```bash
sudo snap install proxyforge
```

### Docker
```bash
docker run -d -p 3001:3001 -p 8888:8888 proxyforge/proxyforge
```

### From Source
```bash
git clone https://github.com/proxyforge/proxyforge.git
cd proxyforge
cargo build --release
```

## Quick Start
1. Start ProxyForge: `proxyforge`
2. Configure your browser to use `localhost:8888` as proxy
3. Open `http://localhost:3001` in your browser
4. Install the root CA certificate when prompted

## Configuration
Configuration is stored in `~/.proxyforge/config.toml`:
```tom
[general]
api_port = 3001
proxy_port = 8888
log_level = "info"

[tls]
cert_dir = "~/.proxyforge/certs"
auto_install_cert = true

[storage]
data_dir = "~/.proxyforge/data"
max_entries = 100000
```

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
1. Open ProxyForge web UI
2. Click "Install Certificate" in the header
3. Follow the instructions for your OS

### Connection Issues
If traffic isn't appearing
1. Check proxy settings in your browser/app
2. Verify ProxyForge is running: `curl http://localhost:3001/api/health`
3. Check firewall rules

### Performance Issues
If the proxy is slow
1. Check memory usage in settings
2. Reduce max_entries if needed
3. Clear old traffic data

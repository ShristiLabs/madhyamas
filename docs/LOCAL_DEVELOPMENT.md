# Local Development Guide

This guide explains how to run Madhyamas directly on your machine without Docker.

## Prerequisites

### Required Software

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Node.js** (v18 or later)
   ```bash
   # macOS (using Homebrew)
   brew install node
   
   # Or download from https://nodejs.org/
   ```

3. **OpenSSL** (for HTTPS certificate generation)
   ```bash
   # macOS
   brew install openssl
   
   # Ubuntu/Debian
   sudo apt-get install libssl-dev
   
   # Fedora/RHEL
   sudo dnf install openssl-devel
   ```

## Quick Start

### 1. Start Madhyamas Locally

```bash
./startup-local.sh
```

This will:
- Build the web frontend (if not already built)
- Compile the Rust binary
- Create necessary data directories
- Start Madhyamas in the background

### 2. Access the Web UI

Open your browser and navigate to:
- **Web UI**: http://localhost:3001
- **HTTP Proxy**: Configure devices to use `localhost:8888`
- **HTTPS Proxy**: Configure devices to use `localhost:8443`

### 3. Stop Madhyamas

```bash
./stop-local.sh
```

## Advanced Usage

### Clean Build

To rebuild everything from scratch:

```bash
./startup-local.sh --clean
```

This removes all build artifacts and rebuilds:
- Web frontend (`web/dist`)
- Rust binary (`target/release/madhyamas`)
- Node modules (`web/node_modules`)

### Custom Configuration

Use environment variables to customize the setup:

```bash
# Bind to all network interfaces (for network access)
export MADHYAMAS_HOST=0.0.0.0

# Custom ports
export MADHYAMAS_API_PORT=3001
export MADHYAMAS_PROXY_PORT=8888

# Set public IP for remote access
export MADHYAMAS_PUBLIC_IP=192.168.1.100

# Start with custom config
./startup-local.sh
```

### Manual Build and Run

If you prefer to build and run manually:

```bash
# Build web frontend
cd web
npm install
npm run build
cd ..

# Build Rust binary
cargo build --release --bin madhyamas

# Run Madhyamas
./target/release/madhyamas --host 0.0.0.0 --api-port 3001 --proxy-port 8888
```

## Development Workflow

### Frontend Development

For frontend development with hot reload:

```bash
cd web
npm run dev
```

This starts the Vite dev server at http://localhost:5173 with hot module replacement.

### Backend Development

For backend development with auto-reload:

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload
cargo watch -x 'run --bin madhyamas -- --host 0.0.0.0'
```

### Running Both in Development

Terminal 1 (Backend):
```bash
cargo watch -x 'run --bin madhyamas -- --host 0.0.0.0'
```

Terminal 2 (Frontend):
```bash
cd web
npm run dev
```

Access the dev frontend at http://localhost:5173, which will proxy API requests to the backend at http://localhost:3001.

## Logs and Data

### Log Files

Madhyamas logs are stored at:
```
~/.madhyamas/logs/madhyamas.log
```

View logs in real-time:
```bash
tail -f ~/.madhyamas/logs/madhyamas.log
```

### Data Directory

All Madhyamas data is stored in:
```
~/.madhyamas/
├── certs/           # SSL certificates
├── logs/            # Log files
├── traffic.db       # Traffic database
└── madhyamas.pid   # Process ID file
```

### Clearing Data

To reset Madhyamas and clear all data:

```bash
rm -rf ~/.madhyamas/
```

## Troubleshooting

### Port Already in Use

If you get a "port already in use" error:

```bash
# Find process using port 3001
lsof -i :3001

# Kill the process
kill -9 <PID>

# Or use different ports
export MADHYAMAS_API_PORT=3002
export MADHYAMAS_PROXY_PORT=8889
./startup-local.sh
```

### Build Errors

If you encounter build errors:

```bash
# Clean rebuild
./startup-local.sh --clean

# Or manually clean
cargo clean
rm -rf web/dist web/node_modules
```

### Permission Errors

If you get permission errors:

```bash
# Ensure scripts are executable
chmod +x startup-local.sh stop-local.sh

# Check data directory permissions
ls -la ~/.madhyamas/
```

### Process Won't Stop

If `stop-local.sh` doesn't work:

```bash
# Force kill all madhyamas processes
pkill -9 -f madhyamas

# Remove stale PID file
rm ~/.madhyamas/madhyamas.pid
```

## Comparison: Local vs Docker

| Feature | Local | Docker |
|---------|-------|--------|
| **Setup** | Requires Rust + Node.js | Only requires Docker |
| **Build Time** | Faster (incremental) | Slower (full rebuild) |
| **Hot Reload** | Yes (with cargo-watch) | No |
| **Isolation** | No | Yes |
| **Network Access** | Direct | Requires port mapping |
| **Best For** | Development | Production/Testing |

## Tips

1. **Use Docker for production**: The Docker setup is more isolated and reproducible
2. **Use local for development**: Faster iteration with hot reload
3. **Set `MADHYAMAS_HOST=0.0.0.0`**: To allow network access from other devices
4. **Check logs**: Always check `~/.madhyamas/logs/madhyamas.log` for errors
5. **Clean builds**: Use `--clean` flag if you encounter weird build issues

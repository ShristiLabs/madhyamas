#!/bin/bash

# ProxyForge Local Startup Script
# This script builds and starts ProxyForge directly on the host machine (without Docker)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
CLEAN_BUILD=false
if [[ "$1" == "--clean" ]] || [[ "$1" == "-c" ]]; then
    CLEAN_BUILD=true
fi

echo -e "${GREEN}ProxyForge Local Startup Script${NC}"
echo "================================"

if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${BLUE}Running in CLEAN BUILD mode${NC}"
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust is not installed.${NC}"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo -e "${RED}Error: Node.js is not installed.${NC}"
    echo "Please install Node.js from https://nodejs.org/"
    exit 1
fi

# Clean build: Remove existing build artifacts
if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${YELLOW}Cleaning existing build artifacts...${NC}"
    
    # Remove web dist directory
    if [ -d "web/dist" ]; then
        echo "  • Removing web/dist..."
        rm -rf web/dist
    fi
    
    # Remove web node_modules for fresh install
    if [ -d "web/node_modules" ]; then
        echo "  • Removing web/node_modules..."
        rm -rf web/node_modules
    fi
    
    # Remove Rust target directory
    if [ -d "target" ]; then
        echo "  • Removing Rust target directory..."
        rm -rf target
    fi
    
    echo -e "${GREEN}✓ Cleanup complete${NC}"
fi

# Build web assets
if [ ! -d "web/dist" ] || [ ! -f "web/dist/index.html" ] || [ "$CLEAN_BUILD" = true ]; then
    echo -e "${YELLOW}Building frontend assets...${NC}"
    if [ -d "web" ]; then
        cd web
        if [ -f "package.json" ]; then
            # Install dependencies (fresh install if clean build)
            if [ ! -d "node_modules" ] || [ "$CLEAN_BUILD" = true ]; then
                echo "  • Installing npm dependencies..."
                npm install
            fi
            # Build web assets
            echo "  • Building web assets..."
            npm run build
            echo -e "${GREEN}✓ Frontend build complete${NC}"
        fi
        cd ..
    else
        echo -e "${RED}Error: web directory not found${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}✓ Web assets already built (use --clean to rebuild)${NC}"
fi

# Build Rust binary
echo -e "${YELLOW}Building Rust binary...${NC}"
if [ "$CLEAN_BUILD" = true ]; then
    cargo build --release --bin proxyforge
else
    # Check if binary exists
    if [ ! -f "target/release/proxyforge" ]; then
        cargo build --release --bin proxyforge
    else
        # Incremental build
        cargo build --release --bin proxyforge
    fi
fi
echo -e "${GREEN}✓ Rust binary built${NC}"

# Create data directories
echo -e "${YELLOW}Creating data directories...${NC}"
mkdir -p ~/.proxyforge/certs
mkdir -p ~/.proxyforge/logs
echo -e "${GREEN}✓ Data directories ready${NC}"

# Check if process is already running
if pgrep -f "target/release/proxyforge" > /dev/null; then
    echo -e "${YELLOW}ProxyForge is already running. Stopping it...${NC}"
    pkill -f "target/release/proxyforge" || true
    sleep 2
fi

# Start ProxyForge
echo -e "${GREEN}Starting ProxyForge...${NC}"
echo ""

# Determine host and port from environment or use defaults
HOST="${PROXYFORGE_HOST:-0.0.0.0}"
API_PORT="${PROXYFORGE_API_PORT:-3001}"
PROXY_PORT="${PROXYFORGE_PROXY_PORT:-8888}"

# Build command with arguments
CMD="./target/release/proxyforge --host $HOST --api-port $API_PORT --proxy-port $PROXY_PORT"

# Add public IP if set
if [ -n "$PROXYFORGE_PUBLIC_IP" ]; then
    CMD="$CMD --public-ip $PROXYFORGE_PUBLIC_IP"
fi

echo -e "${BLUE}Command: $CMD${NC}"
echo ""

# Run in background and save PID
nohup $CMD > ~/.proxyforge/logs/proxyforge.log 2>&1 &
PID=$!
echo $PID > ~/.proxyforge/proxyforge.pid

# Wait a moment for startup
sleep 3

# Check if process is still running
if ps -p $PID > /dev/null; then
    echo -e "${GREEN}✓ ProxyForge is running!${NC}"
    echo ""
    echo "Services:"
    echo "  • Web UI/API:    http://localhost:$API_PORT"
    echo "  • HTTP Proxy:    http://localhost:$PROXY_PORT"
    echo "  • HTTPS Proxy:   https://localhost:8443"
    echo ""
    echo "Process:"
    echo "  • PID:           $PID"
    echo "  • Log file:      ~/.proxyforge/logs/proxyforge.log"
    echo ""
    echo "Commands:"
    echo "  • Stop:          ./stop-local.sh"
    echo "  • View logs:     tail -f ~/.proxyforge/logs/proxyforge.log"
    echo "  • Clean rebuild: ./startup-local.sh --clean"
    echo ""
    echo "Environment variables:"
    echo "  • PROXYFORGE_HOST=$HOST"
    echo "  • PROXYFORGE_API_PORT=$API_PORT"
    echo "  • PROXYFORGE_PROXY_PORT=$PROXY_PORT"
    if [ -n "$PROXYFORGE_PUBLIC_IP" ]; then
        echo "  • PROXYFORGE_PUBLIC_IP=$PROXYFORGE_PUBLIC_IP"
    fi
else
    echo -e "${RED}Error: ProxyForge failed to start${NC}"
    echo "Check logs at: ~/.proxyforge/logs/proxyforge.log"
    cat ~/.proxyforge/logs/proxyforge.log
    exit 1
fi

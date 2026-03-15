#!/bin/bash

# Madhyamas Startup Script
# This script builds and starts the Madhyamas application using Docker Compose
# All detection (OS, IP) happens automatically
#
# Usage:
#   ./startup.sh          # Normal startup
#   ./startup.sh --clean  # Clean rebuild (no cache)
#
# Environment variables (optional):
#   MADHYAMAS_PUBLIC_IP  - Override auto-detected IP

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Detect OS
OS="$(uname -s)"

# Parse command line arguments
CLEAN_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --clean|-c)
            CLEAN_BUILD=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Usage: ./startup.sh [--clean]"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}Madhyamas Startup Script${NC}"
echo "================================"
echo -e "Detected OS: ${BLUE}$OS${NC}"

if [ "$CLEAN_BUILD" = true ]; then
    echo -e "Mode: ${BLUE}CLEAN BUILD (no cache)${NC}"
fi

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed. Please install Docker first.${NC}"
    exit 1
fi

# Check if Docker Compose is available
if ! docker compose version &> /dev/null; then
    echo -e "${RED}Error: Docker Compose is not available. Please install Docker Compose.${NC}"
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
    
    # Remove Rust target directory if it exists locally
    if [ -d "target" ]; then
        echo "  • Removing Rust target directory..."
        rm -rf target
    fi
    
    # Prune Docker build cache for this project
    echo "  • Pruning Docker build cache..."
    docker builder prune -f --filter "label=project=madhyamas" 2>/dev/null || true
    
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

# Create certs directory if it doesn't exist
mkdir -p certs

# Detect host IP for mobile device access (OS-aware)
detect_host_ip() {
    # macOS
    if [[ "$OS" == "Darwin" ]]; then
        for iface in en0 en1 en2 en3 en4 en5; do
            local ip=$(ipconfig getifaddr $iface 2>/dev/null || true)
            if [[ -n "$ip" ]] && [[ "$ip" =~ ^192\.168\. || "$ip" =~ ^10\. ]]; then
                echo "$ip"
                return
            fi
        done
    # Linux
    else
        local ip=$(ip -o -4 addr show 2>/dev/null | grep -v 'docker\|br-\|veth\|127.0.0.1' | grep -oP 'inet \K[\d.]+' | grep -E '^(192\.168\.|10\.)' | head -1)
        if [[ -n "$ip" ]]; then
            echo "$ip"
            return
        fi
    fi
}

# Determine the IP to use (env var takes precedence, then auto-detect)
if [[ -n "$MADHYAMAS_PUBLIC_IP" ]]; then
    HOST_IP="$MADHYAMAS_PUBLIC_IP"
    echo -e "Host IP: ${BLUE}$HOST_IP${NC} (from MADHYAMAS_PUBLIC_IP)"
else
    HOST_IP=$(detect_host_ip)
    if [[ -n "$HOST_IP" ]]; then
        echo -e "Host IP: ${BLUE}$HOST_IP${NC} (auto-detected)"
    else
        echo -e "${YELLOW}Warning: Could not auto-detect host IP.${NC}"
        echo -e "${YELLOW}Set MADHYAMAS_PUBLIC_IP env var for mobile device access.${NC}"
    fi
fi

# Export for Docker Compose
export MADHYAMAS_PUBLIC_IP="${HOST_IP:-}"

# Stop any existing containers
echo -e "${YELLOW}Stopping any existing containers...${NC}"
docker compose down 2>/dev/null || true

# Build and start containers
if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${GREEN}Building Docker images (no cache)...${NC}"
    docker compose build --no-cache
    echo -e "${GREEN}Starting Madhyamas containers...${NC}"
    docker compose up -d
else
    echo -e "${GREEN}Building and starting Madhyamas...${NC}"
    docker compose up -d --build
fi

# Wait for containers to be healthy
echo -e "${YELLOW}Waiting for containers to start...${NC}"
sleep 5

# Check if containers are running
if docker compose ps | grep -q "Up"; then
    echo -e "${GREEN}✓ Madhyamas is running!${NC}"
    echo ""
    echo "Services:"
    echo "  • Web UI/API:    http://localhost:3001"
    echo "  • HTTP Proxy:    http://localhost:8888"
    echo "  • HTTPS Proxy:   https://localhost:8443"
    echo ""
    if [[ -n "$HOST_IP" ]]; then
        echo -e "${BLUE}Mobile Device Setup:${NC}"
        echo "  • Proxy Address: $HOST_IP:8888"
        echo "  • CA Certificate: http://$HOST_IP:3001/api/cert/ca"
        echo ""
    fi
    echo "Commands:"
    echo "  • Stop:          ./stop.sh"
    echo "  • View logs:     docker compose logs -f"
    echo "  • Clean rebuild: ./startup.sh --clean"
    echo ""
    echo "MCP Integration:"
    echo "  • Extract MCP binary: ./scripts/extract-mcp.sh"
    echo "  • See docs/MCP-INTEGRATION.md for Windsurf setup"
else
    echo -e "${RED}Error: Containers failed to start. Check logs with: docker compose logs${NC}"
    exit 1
fi

#!/bin/bash

# ProxyForge Startup Script
# This script builds and starts the ProxyForge application using Docker Compose

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}ProxyForge Startup Script${NC}"
echo "================================"

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

# Build web assets if they don't exist
if [ ! -d "web/dist" ] || [ ! -f "web/dist/index.html" ]; then
    echo -e "${YELLOW}Web assets not found. Building frontend...${NC}"
    if [ -d "web" ]; then
        cd web
        if [ -f "package.json" ]; then
            if [ ! -d "node_modules" ]; then
                echo "Installing npm dependencies..."
                npm install
            fi
            echo "Building web assets..."
            npm run build
        fi
        cd ..
    else
        echo -e "${RED}Error: web directory not found${NC}"
        exit 1
    fi
fi

# Create certs directory if it doesn't exist
mkdir -p certs

# Stop any existing containers
echo -e "${YELLOW}Stopping any existing containers...${NC}"
docker compose down 2>/dev/null || true

# Build and start containers
echo -e "${GREEN}Building and starting ProxyForge...${NC}"
docker compose up -d --build

# Wait for containers to be healthy
echo -e "${YELLOW}Waiting for containers to start...${NC}"
sleep 5

# Check if containers are running
if docker compose ps | grep -q "Up"; then
    echo -e "${GREEN}✓ ProxyForge is running!${NC}"
    echo ""
    echo "Services:"
    echo "  • Web UI/API:    http://localhost:3001"
    echo "  • HTTP Proxy:    http://localhost:8888"
    echo "  • HTTPS Proxy:   https://localhost:8443"
    echo ""
    echo "Use './stop.sh' to stop the application"
    echo "View logs with: docker compose logs -f"
else
    echo -e "${RED}Error: Containers failed to start. Check logs with: docker compose logs${NC}"
    exit 1
fi

#!/bin/bash

# ProxyForge Stop Script
# This script stops the ProxyForge application and cleans up resources

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}ProxyForge Stop Script${NC}"
echo "==============================="

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed.${NC}"
    exit 1
fi

# Check if containers are running
if ! docker compose ps | grep -q "Up"; then
    echo -e "${YELLOW}No running containers found.${NC}"
    exit 0
fi

# Stop containers
echo -e "${YELLOW}Stopping ProxyForge containers...${NC}"
docker compose down

# Optional: Remove volumes (commented out by default to preserve data)
# echo -e "${YELLOW}Removing volumes...${NC}"
# docker compose down -v

# Optional: Remove images (commented out by default)
# echo -e "${YELLOW}Removing images...${NC}"
# docker compose down --rmi all

echo -e "${GREEN}✓ ProxyForge has been stopped successfully.${NC}"
echo ""
echo "To start again, run: ./startup.sh"
echo "To view logs, run: docker compose logs"

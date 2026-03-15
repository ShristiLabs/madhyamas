#!/bin/bash

# ProxyForge Local Stop Script
# This script stops the locally running ProxyForge instance

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}ProxyForge Local Stop Script${NC}"
echo "================================"

# Check if PID file exists
if [ -f ~/.proxyforge/proxyforge.pid ]; then
    PID=$(cat ~/.proxyforge/proxyforge.pid)
    
    # Check if process is running
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "${YELLOW}Stopping ProxyForge (PID: $PID)...${NC}"
        kill $PID
        
        # Wait for process to stop
        for i in {1..10}; do
            if ! ps -p $PID > /dev/null 2>&1; then
                break
            fi
            sleep 1
        done
        
        # Force kill if still running
        if ps -p $PID > /dev/null 2>&1; then
            echo -e "${YELLOW}Force stopping ProxyForge...${NC}"
            kill -9 $PID
        fi
        
        echo -e "${GREEN}✓ ProxyForge stopped${NC}"
    else
        echo -e "${YELLOW}ProxyForge is not running (stale PID file)${NC}"
    fi
    
    # Remove PID file
    rm ~/.proxyforge/proxyforge.pid
else
    # Try to find and kill any running proxyforge processes
    if pgrep -f "target/release/proxyforge" > /dev/null; then
        echo -e "${YELLOW}Found running ProxyForge process, stopping...${NC}"
        pkill -f "target/release/proxyforge"
        sleep 2
        echo -e "${GREEN}✓ ProxyForge stopped${NC}"
    else
        echo -e "${YELLOW}ProxyForge is not running${NC}"
    fi
fi

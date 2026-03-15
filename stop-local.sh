#!/bin/bash

# Madhyamas Local Stop Script
# This script stops the locally running Madhyamas instance

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Madhyamas Local Stop Script${NC}"
echo "================================"

# Check if PID file exists
if [ -f ~/.madhyamas/madhyamas.pid ]; then
    PID=$(cat ~/.madhyamas/madhyamas.pid)
    
    # Check if process is running
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "${YELLOW}Stopping Madhyamas (PID: $PID)...${NC}"
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
            echo -e "${YELLOW}Force stopping Madhyamas...${NC}"
            kill -9 $PID
        fi
        
        echo -e "${GREEN}✓ Madhyamas stopped${NC}"
    else
        echo -e "${YELLOW}Madhyamas is not running (stale PID file)${NC}"
    fi
    
    # Remove PID file
    rm ~/.madhyamas/madhyamas.pid
else
    # Try to find and kill any running madhyamas processes
    if pgrep -f "target/release/madhyamas" > /dev/null; then
        echo -e "${YELLOW}Found running Madhyamas process, stopping...${NC}"
        pkill -f "target/release/madhyamas"
        sleep 2
        echo -e "${GREEN}✓ Madhyamas stopped${NC}"
    else
        echo -e "${YELLOW}Madhyamas is not running${NC}"
    fi
fi

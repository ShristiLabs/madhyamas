#!/bin/bash

# Madhyamas Local Stop Script
# Stops locally running Madhyamas instances.
#
# For the enterprise tier, this stops the Docker Compose multi-instance
# stack (PostgreSQL, Redis, 2x Madhyamas, nginx). For OSS, it stops the
# local binary.
#
# Usage:
#   ./stop-local.sh                  Stop everything (default: --all)
#   ./stop-local.sh --tier enterprise  Stop enterprise Docker stack + local binaries
#   ./stop-local.sh --tier oss       Stop OSS local binary
#   ./stop-local.sh --all            Stop all instances (local + Docker)
#   ./stop-local.sh --help, -h

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

TIER="all"
STOP_ALL=false

print_help() {
    cat <<'EOF'
Madhyamas Local Stop Script

Usage:
  ./stop-local.sh [OPTIONS]

Options:
  --tier <oss|enterprise>   Which tier's instance to stop.
                            enterprise = Docker Compose multi-instance stack
                            + any local enterprise binary.
                            oss = local OSS binary only.
  --all                     Stop everything: all local binaries + Docker stack
                            (default behavior when no flags are given).
  --help, -h                Show this help and exit.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --tier)
            if [[ -z "$2" || "$2" == --* ]]; then
                echo -e "${RED}Error: --tier requires a value (oss|enterprise)${NC}"
                exit 1
            fi
            TIER="$2"
            shift 2
            ;;
        --tier=*)
            TIER="${1#--tier=}"
            shift
            ;;
        --all)
            STOP_ALL=true
            shift
            ;;
        --help|-h)
            print_help
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option: $1${NC}"
            echo "Run './stop-local.sh --help' for usage."
            exit 1
            ;;
    esac
done

if [ "$STOP_ALL" = true ]; then
    TIER="all"
fi

case "$TIER" in
    oss|enterprise|all) ;;
    *)
        echo -e "${RED}Error: Invalid tier '$TIER'. Must be 'oss', 'enterprise', or 'all'.${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}Madhyamas Local Stop Script${NC}"
echo "================================"

# ---------------------------------------------------------------------------
# Stop local binary (OSS or enterprise PID file)
# ---------------------------------------------------------------------------
stop_local_binary() {
    local pid_file="$1"
    local label="$2"

    if [ -f "$pid_file" ]; then
        local pid
        pid=$(cat "$pid_file" 2>/dev/null || true)

        if [ -n "$pid" ] && ps -p "$pid" > /dev/null 2>&1; then
            echo "  • Stopping $label (PID: $pid)"
            kill "$pid" 2>/dev/null || true

            for i in {1..10}; do
                if ! ps -p "$pid" > /dev/null 2>&1; then
                    break
                fi
                sleep 1
            done

            if ps -p "$pid" > /dev/null 2>&1; then
                echo -e "${YELLOW}    Force stopping...${NC}"
                kill -9 "$pid" 2>/dev/null || true
            fi

            echo -e "${GREEN}  ✓ $label stopped${NC}"
        else
            echo -e "${YELLOW}  • $label not running (stale PID file)${NC}"
        fi

        rm -f "$pid_file"
    else
        echo -e "${YELLOW}  • No $label PID file found${NC}"
    fi
}

# ---------------------------------------------------------------------------
# Stop Docker Compose multi-instance stack
# ---------------------------------------------------------------------------
stop_docker_stack() {
    if ! command -v docker &> /dev/null; then
        echo -e "${YELLOW}  • Docker not installed — skipping Docker stack${NC}"
        return
    fi

    local compose_file="docker/docker-compose.multi.yml"
    if [ ! -f "$compose_file" ]; then
        echo -e "${YELLOW}  • $compose_file not found — skipping${NC}"
        return
    fi

    # Check if any services from the multi-instance stack are running
    local running
    running=$(docker compose -f "$compose_file" ps --status running --format "{{.Service}}" 2>/dev/null || true)
    if [ -n "$running" ]; then
        echo "  • Stopping Docker multi-instance stack..."
        echo "    Services running: $(echo "$running" | tr '\n' ' ')"
        docker compose -f "$compose_file" down --remove-orphans 2>/dev/null || true
        echo -e "${GREEN}  ✓ Docker stack stopped${NC}"
    else
        echo -e "${YELLOW}  • Docker multi-instance stack not running${NC}"
    fi

    # Also stop any standalone madhyamas proxy containers (not postgres/redis
    # test infrastructure — only containers running the actual madhyamas binary).
    local standalone
    standalone=$(docker ps --filter "ancestor=madhyamas:latest" --filter "ancestor=madhyamas-enterprise:latest" --format "{{.Names}}" 2>/dev/null || true)
    if [ -n "$standalone" ]; then
        echo "  • Stopping standalone madhyamas proxy containers: $(echo "$standalone" | tr '\n' ' ')"
        echo "$standalone" | xargs docker stop 2>/dev/null || true
        echo -e "${GREEN}  ✓ Standalone containers stopped${NC}"
    fi
}

# ---------------------------------------------------------------------------
# Stop stray local processes
# ---------------------------------------------------------------------------
stop_stray_processes() {
    if pgrep -f "target/release/madhyamas" > /dev/null 2>&1; then
        echo -e "${YELLOW}  • Stopping stray local madhyamas processes...${NC}"
        pkill -f "target/release/madhyamas" 2>/dev/null || true
        sleep 2
        echo -e "${GREEN}  ✓ Stray processes stopped${NC}"
    fi
}

# ---------------------------------------------------------------------------
# Execute stops based on tier
# ---------------------------------------------------------------------------
case "$TIER" in
    oss)
        echo -e "${YELLOW}Stopping OSS instance...${NC}"
        stop_local_binary "$HOME/.madhyamas/madhyamas-oss.pid" "OSS instance"
        ;;
    enterprise)
        echo -e "${YELLOW}Stopping Enterprise instances...${NC}"
        stop_local_binary "$HOME/.madhyamas/madhyamas.pid" "Enterprise local binary"
        stop_docker_stack
        stop_stray_processes
        ;;
    all)
        echo -e "${YELLOW}Stopping ALL instances...${NC}"
        stop_local_binary "$HOME/.madhyamas/madhyamas.pid" "Enterprise local binary"
        stop_local_binary "$HOME/.madhyamas/madhyamas-oss.pid" "OSS local binary"
        stop_docker_stack
        stop_stray_processes
        ;;
esac

echo ""
echo -e "${GREEN}✓ Done${NC}"

#!/bin/bash

# Madhyamas Local Startup Script
# This script builds and starts Madhyamas directly on the host machine
# (without Docker for OSS) or via Docker Compose (for enterprise).
#
# Tiers:
#
#   --tier enterprise  (default)  Runs the FULL multi-instance stack via
#                                 Docker Compose (docker/docker-compose.multi.yml):
#                                   • PostgreSQL 16 (shared storage)
#                                   • Redis 7 (pub/sub, seat coordination)
#                                   • 2x Madhyamas enterprise instances
#                                   • nginx load balancer (round-robin)
#                                 All running proxy instances are stopped
#                                 first (local binaries + Docker containers).
#
#   --tier oss                    Builds and runs the OSS binary locally
#                                 (cargo build --no-default-features).
#                                 No Docker, no multi-instance — single
#                                 process with SQLite storage.
#
# Other flags:
#   --clean, -c                   Clean rebuild (removes target/, web/dist,
#                                 web/node_modules; prunes Docker images for
#                                 enterprise).
#   --help, -h                    Show this help.
#
# Enterprise Docker configuration is passed through via environment
# variables — see the help text for the full list.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
TIER="enterprise"
CLEAN_BUILD=false

print_help() {
    cat <<'EOF'
Madhyamas Local Startup Script

Usage:
  ./startup-local.sh [OPTIONS]

Options:
  --tier <oss|enterprise>   Build tier (default: enterprise).
                            enterprise = Docker Compose multi-instance stack
                            (PostgreSQL + Redis + 2x Madhyamas + nginx LB).
                            oss = local binary build with --no-default-features
                            (MIT/Apache, single instance, SQLite).
  --clean, -c               Clean rebuild. For OSS: removes target/, web/dist,
                            web/node_modules. For enterprise: also prunes the
                            Docker images and volumes.
  --help, -h                Show this help and exit.

Enterprise Docker configuration (via environment variables):
  POSTGRES_USER             PostgreSQL user (default: madhyamas).
  POSTGRES_PASSWORD         PostgreSQL password (default: testpass).
  POSTGRES_DB               PostgreSQL database name (default: madhyamas).
  MADHYAMAS_JWT_SECRET      JWT signing secret for auth.
  MADHYAMAS_ADMIN_PASSWORD  Bootstrap admin password.
  MADHYAMAS_ADMIN_USERNAME  Bootstrap admin username (default: admin).
  MADHYAMAS_PUBLIC_IP       Public IP for remote access (shown in UI).
  MADHYAMAS_LICENSE_FILE    Path to a license file (mounted into containers).
  RUST_LOG                  Log level (default: info).

  Port overrides:
  LB_PORT                   nginx load balancer port (default: 14000).
  INSTANCE1_API_PORT        Instance 1 API/Web UI port (default: 14001).
  INSTANCE2_API_PORT        Instance 2 API/Web UI port (default: 14002).
  PG_PORT                   PostgreSQL port (default: 15432).
  REDIS_PORT                Redis port (default: 16379).
  PROXY_PORT                Proxy port inside containers (default: 8888).
  PROXY_LB_PORT             Proxy load balancer port on host (default: 8888).

  Host/port for OSS mode:
  MADHYAMAS_HOST            Bind host (default: 0.0.0.0).
  MADHYAMAS_API_PORT        API port (default: 3001).
  MADHYAMAS_PROXY_PORT      Proxy port (default: 8888).

Examples:
  ./startup-local.sh                                    # enterprise Docker stack
  ./startup-local.sh --tier oss                         # OSS local binary
  ./startup-local.sh --tier enterprise --clean          # clean enterprise rebuild
  MADHYAMAS_ADMIN_PASSWORD=secret ./startup-local.sh    # custom admin password
  LB_PORT=80 INSTANCE1_API_PORT=8081 INSTANCE2_API_PORT=8082 ./startup-local.sh
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
        --clean|-c)
            CLEAN_BUILD=true
            shift
            ;;
        --help|-h)
            print_help
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option: $1${NC}"
            echo "Run './startup-local.sh --help' for usage."
            exit 1
            ;;
    esac
done

# Validate tier
case "$TIER" in
    oss|enterprise) ;;
    *)
        echo -e "${RED}Error: Invalid tier '$TIER'. Must be 'oss' or 'enterprise'.${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}Madhyamas Local Startup Script${NC}"
echo "================================"
echo -e "${CYAN}Tier:    ${TIER}${NC}"
echo -e "${CYAN}Clean:   ${CLEAN_BUILD}${NC}"
echo ""

# ---------------------------------------------------------------------------
# Prerequisite checks
# ---------------------------------------------------------------------------
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust is not installed.${NC}"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo -e "${RED}Error: Node.js is not installed.${NC}"
    echo "Please install Node.js from https://nodejs.org/"
    exit 1
fi

if [ "$TIER" = "enterprise" ] && ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed.${NC}"
    echo "The enterprise tier requires Docker Compose for the multi-instance stack."
    echo "Please install Docker from https://docs.docker.com/get-docker/"
    exit 1
fi

if [ "$TIER" = "enterprise" ] && ! docker compose version &> /dev/null; then
    echo -e "${RED}Error: Docker Compose v2 is not available.${NC}"
    echo "The enterprise tier requires 'docker compose' (v2 plugin)."
    echo "Please install Docker Compose v2."
    exit 1
fi

# ===========================================================================
# STOP ALL EXISTING INSTANCES
# ===========================================================================
# Stop every running Madhyamas process and Docker container, regardless of
# tier. This ensures a clean slate before starting the new instance.

stop_all_instances() {
    echo -e "${YELLOW}Stopping all existing Madhyamas instances...${NC}"

    # --- Stop local binaries (both tiers) ---
    local stopped_any=false

    for pid_file in "$HOME/.madhyamas/madhyamas.pid" "$HOME/.madhyamas/madhyamas-oss.pid"; do
        if [ -f "$pid_file" ]; then
            local pid
            pid=$(cat "$pid_file" 2>/dev/null || true)
            if [ -n "$pid" ] && ps -p "$pid" > /dev/null 2>&1; then
                echo "  • Stopping local process (PID: $pid, file: $(basename "$pid_file"))"
                kill "$pid" 2>/dev/null || true
                for i in {1..10}; do
                    if ! ps -p "$pid" > /dev/null 2>&1; then break; fi
                    sleep 1
                done
                if ps -p "$pid" > /dev/null 2>&1; then
                    echo -e "${YELLOW}    Force stopping...${NC}"
                    kill -9 "$pid" 2>/dev/null || true
                fi
                stopped_any=true
            fi
            rm -f "$pid_file"
        fi
    done

    # Catch any stray local processes not tracked by PID files
    if pgrep -f "target/release/madhyamas" > /dev/null 2>&1; then
        echo "  • Stopping stray local madhyamas processes..."
        pkill -f "target/release/madhyamas" 2>/dev/null || true
        sleep 2
        stopped_any=true
    fi

    # --- Stop Docker containers (multi-instance stack) ---
    if command -v docker &> /dev/null; then
        # Stop the multi-instance compose stack if it's running
        if [ -f "docker/docker-compose.multi.yml" ]; then
            if docker compose -f docker/docker-compose.multi.yml ps --status running 2>/dev/null | grep -q "madhyamas\|postgres\|redis\|nginx"; then
                echo "  • Stopping Docker multi-instance stack..."
                docker compose -f docker/docker-compose.multi.yml down --remove-orphans 2>/dev/null || true
                stopped_any=true
            fi
        fi

        # Stop any standalone madhyamas proxy containers (not postgres/redis
        # test infrastructure — only containers running the madhyamas binary).
        local standalone_containers
        standalone_containers=$(docker ps --filter "ancestor=madhyamas:latest" --filter "ancestor=madhyamas-enterprise:latest" --format "{{.Names}}" 2>/dev/null || true)
        if [ -n "$standalone_containers" ]; then
            echo "  • Stopping standalone madhyamas proxy containers: $standalone_containers"
            echo "$standalone_containers" | xargs docker stop 2>/dev/null || true
            stopped_any=true
        fi
    fi

    if [ "$stopped_any" = true ]; then
        echo -e "${GREEN}✓ All instances stopped${NC}"
    else
        echo -e "${GREEN}✓ No running instances found${NC}"
    fi
}

stop_all_instances

# ===========================================================================
# CLEAN BUILD
# ===========================================================================
if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${BLUE}Running in CLEAN build mode${NC}"
    echo -e "${YELLOW}Cleaning existing build artifacts...${NC}"

    # Web assets (needed for both tiers — embedded at compile time)
    if [ -d "web/dist" ]; then
        echo "  • Removing web/dist..."
        rm -rf web/dist
    fi
    if [ -d "web/node_modules" ]; then
        echo "  • Removing web/node_modules..."
        rm -rf web/node_modules
    fi

    # Rust target (OSS local build)
    if [ -d "target" ]; then
        echo "  • Removing Rust target directory..."
        rm -rf target
    fi

    # Enterprise: prune Docker images and volumes
    if [ "$TIER" = "enterprise" ]; then
        echo "  • Pruning Docker images (madhyamas-enterprise)..."
        docker image rm madhyamas-enterprise:latest 2>/dev/null || true
        echo "  • Pruning Docker volume (pg_data)..."
        docker volume rm docker_pg_data 2>/dev/null || true
        docker volume rm madhyamas_pg_data 2>/dev/null || true
    fi

    echo -e "${GREEN}✓ Cleanup complete${NC}"
fi

# ===========================================================================
# OSS TIER — LOCAL BINARY BUILD + RUN
# ===========================================================================
if [ "$TIER" = "oss" ]; then
    echo -e "${CYAN}=== OSS Tier: Local Binary Build ===${NC}"
    echo ""

    # --- Build web assets ---
    WEB_SRC_DIR="web"
    WEB_DIST_DIR="${WEB_SRC_DIR}/dist"

    if [ ! -d "$WEB_DIST_DIR" ] || [ ! -f "$WEB_DIST_DIR/index.html" ] || [ "$CLEAN_BUILD" = true ]; then
        echo -e "${YELLOW}Building frontend assets ($WEB_SRC_DIR)...${NC}"
        if [ -d "$WEB_SRC_DIR" ]; then
            cd "$WEB_SRC_DIR"
            if [ -f "package.json" ]; then
                if [ ! -d "node_modules" ] || [ "$CLEAN_BUILD" = true ]; then
                    echo "  • Installing npm dependencies..."
                    npm install
                fi
                echo "  • Building web assets..."
                npm run build
                echo -e "${GREEN}✓ Frontend build complete${NC}"
            fi
            cd ..
        else
            echo -e "${RED}Error: $WEB_SRC_DIR directory not found${NC}"
            exit 1
        fi
    else
        echo -e "${GREEN}✓ Web assets already built (use --clean to rebuild)${NC}"
    fi

    # --- Build Rust binary (OSS: no default features) ---
    echo -e "${YELLOW}Building Rust binary (OSS)...${NC}"
    BUILD_CMD="cargo build --release --bin madhyamas --no-default-features"
    echo -e "${BLUE}  $BUILD_CMD${NC}"
    $BUILD_CMD
    echo -e "${GREEN}✓ Rust binary built${NC}"

    # --- Data directories ---
    echo -e "${YELLOW}Creating data directories...${NC}"
    mkdir -p ~/.madhyamas/certs
    mkdir -p ~/.madhyamas/logs
    echo -e "${GREEN}✓ Data directories ready${NC}"

    # --- Start ---
    HOST="${MADHYAMAS_HOST:-0.0.0.0}"
    API_PORT="${MADHYAMAS_API_PORT:-3001}"
    PROXY_PORT="${MADHYAMAS_PROXY_PORT:-8888}"
    PID_FILE="$HOME/.madhyamas/madhyamas-oss.pid"

    CMD="./target/release/madhyamas --host $HOST --api-port $API_PORT --proxy-port $PROXY_PORT"
    if [ -n "$MADHYAMAS_PUBLIC_IP" ]; then
        CMD="$CMD --public-ip $MADHYAMAS_PUBLIC_IP"
    fi

    echo -e "${GREEN}Starting Madhyamas (OSS)...${NC}"
    echo -e "${BLUE}Command: $CMD${NC}"
    echo ""

    STDERR_LOG="$HOME/.madhyamas/logs/madhyamas-oss.stderr.log"
    nohup $CMD > /dev/null 2> "$STDERR_LOG" &
    PID=$!
    echo $PID > "$PID_FILE"

    sleep 3

    if ps -p $PID > /dev/null; then
        echo -e "${GREEN}✓ Madhyamas (OSS) is running!${NC}"
        echo ""
        echo "Services:"
        echo "  • Web UI/API:    http://localhost:$API_PORT"
        echo "  • HTTP Proxy:    http://localhost:$PROXY_PORT"
        echo "  • HTTPS Proxy:   http://localhost:$PROXY_PORT (HTTP and HTTPS on same port)"
        echo ""
        echo "Process:"
        echo "  • Tier:          OSS (MIT/Apache)"
        echo "  • PID:           $PID"
        echo "  • PID file:      $PID_FILE"
        echo "  • Log file:      ~/.madhyamas/logs/madhyamas.log (rotated automatically)"
        echo "  • Stderr:        $STDERR_LOG (crash diagnostics)"
        echo ""
        echo "Commands:"
        echo "  • Stop:          ./stop-local.sh --tier oss"
        echo "  • View logs:     tail -f ~/.madhyamas/logs/madhyamas.log"
        echo "  • Clean rebuild: ./startup-local.sh --clean --tier oss"
        echo ""
    else
        echo -e "${RED}Error: Madhyamas (OSS) failed to start${NC}"
        echo "Check stderr at: $STDERR_LOG"
        cat "$STDERR_LOG" 2>/dev/null || true
        rm -f "$PID_FILE"
        exit 1
    fi
    exit 0
fi

# ===========================================================================
# ENTERPRISE TIER — DOCKER COMPOSE MULTI-INSTANCE STACK
# ===========================================================================
echo -e "${CYAN}=== Enterprise Tier: Docker Multi-Instance Stack ===${NC}"
echo ""

COMPOSE_FILE="docker/docker-compose.multi.yml"

if [ ! -f "$COMPOSE_FILE" ]; then
    echo -e "${RED}Error: $COMPOSE_FILE not found${NC}"
    exit 1
fi

# --- Build web assets first (Docker build embeds them via rust-embed) ---
# The Dockerfile builds web assets inside the container, but we also build
# locally so the host has them for reference. The Docker build is
# self-contained — it copies web/ and builds inside the frontend-builder
# stage. This local build is optional but useful for debugging.
WEB_SRC_DIR="web"
if [ ! -d "$WEB_SRC_DIR/dist" ] || [ "$CLEAN_BUILD" = true ]; then
    echo -e "${YELLOW}Building frontend assets locally (for reference)...${NC}"
    if [ -d "$WEB_SRC_DIR" ] && [ -f "$WEB_SRC_DIR/package.json" ]; then
        cd "$WEB_SRC_DIR"
        if [ ! -d "node_modules" ] || [ "$CLEAN_BUILD" = true ]; then
            npm install
        fi
        npm run build
        cd ..
        echo -e "${GREEN}✓ Frontend build complete${NC}"
    fi
else
    echo -e "${GREEN}✓ Web assets already built (Docker will build its own copy)${NC}"
fi

# --- Resolve configuration with defaults ---
LB_PORT="${LB_PORT:-14000}"
INSTANCE1_API_PORT="${INSTANCE1_API_PORT:-14001}"
INSTANCE2_API_PORT="${INSTANCE2_API_PORT:-14002}"
PG_PORT="${PG_PORT:-15432}"
REDIS_PORT="${REDIS_PORT:-16379}"
PROXY_PORT="${PROXY_PORT:-8888}"
PROXY_LB_PORT="${PROXY_LB_PORT:-8888}"

export POSTGRES_USER="${POSTGRES_USER:-madhyamas}"
export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-testpass}"
export POSTGRES_DB="${POSTGRES_DB:-madhyamas}"
export MADHYAMAS_JWT_SECRET="${MADHYAMAS_JWT_SECRET:-multi-instance-dev-secret}"
export MADHYAMAS_ADMIN_PASSWORD="${MADHYAMAS_ADMIN_PASSWORD:-testpass123}"
export MADHYAMAS_ADMIN_USERNAME="${MADHYAMAS_ADMIN_USERNAME:-admin}"
export MADHYAMAS_PUBLIC_IP="${MADHYAMAS_PUBLIC_IP:-}"
export RUST_LOG="${RUST_LOG:-info}"

# Only export MADHYAMAS_LICENSE_FILE if it's non-empty — the binary treats
# an empty string as a provided-but-not-found file path and fails to start.
if [ -n "$MADHYAMAS_LICENSE_FILE" ]; then
    export MADHYAMAS_LICENSE_FILE
fi
export LB_PORT INSTANCE1_API_PORT INSTANCE2_API_PORT PG_PORT REDIS_PORT PROXY_PORT PROXY_LB_PORT

# --- Build the Docker image once (avoids race when two services share it) ---
echo -e "${YELLOW}Building Docker image (madhyamas-enterprise:latest)...${NC}"
echo -e "${BLUE}  docker compose -f $COMPOSE_FILE build${NC}"
echo ""
docker compose -f "$COMPOSE_FILE" build

# --- Start all services (no --build to avoid concurrent image export race) ---
echo -e "${YELLOW}Starting Docker multi-instance stack...${NC}"
echo -e "${BLUE}  docker compose -f $COMPOSE_FILE up -d${NC}"
echo ""
docker compose -f "$COMPOSE_FILE" up -d

# --- Wait for services to be healthy ---
echo -e "${YELLOW}Waiting for services to start...${NC}"

MAX_WAIT=120
WAITED=0

check_service_health() {
    local service="$1"
    local status
    status=$(docker compose -f "$COMPOSE_FILE" ps --format json "$service" 2>/dev/null \
             | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('Health','unknown'))" 2>/dev/null \
             || echo "unknown")
    if [ "$status" = "healthy" ]; then
        return 0
    fi
    return 1
}

# Simple polling: wait for containers to be running
all_healthy=false
while [ $WAITED -lt $MAX_WAIT ]; do
    # Check if all services are running
    RUNNING=$(docker compose -f "$COMPOSE_FILE" ps --status running --format "{{.Service}}" 2>/dev/null | sort)
    EXPECTED="madhyamas-1 madhyamas-2 nginx postgres redis"
    RUNNING_COUNT=$(echo "$RUNNING" | grep -c . 2>/dev/null || echo 0)

    if [ "$RUNNING_COUNT" -ge 5 ]; then
        # All containers are running. Give them a few seconds to become healthy.
        sleep 5
        # Check health endpoints
        H1=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$INSTANCE1_API_PORT/health" 2>/dev/null || echo "000")
        H2=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$INSTANCE2_API_PORT/health" 2>/dev/null || echo "000")
        HLB=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$LB_PORT/health" 2>/dev/null || echo "000")
        if [ "$H1" = "200" ] && [ "$H2" = "200" ]; then
            all_healthy=true
            break
        fi
    fi

    sleep 3
    WAITED=$((WAITED + 3))
    echo -n "."
done
echo ""

if [ "$all_healthy" = true ]; then
    echo -e "${GREEN}✓ All services are healthy (waited ${WAITED}s)${NC}"
else
    echo -e "${YELLOW}Warning: Services may still be starting up (waited ${WAITED}s)${NC}"
    echo -e "${YELLOW}Check status with: docker compose -f $COMPOSE_FILE ps${NC}"
fi

# --- Status report ---
echo ""
echo -e "${GREEN}✓ Madhyamas (Enterprise) multi-instance stack is running!${NC}"
echo ""
echo "Services (Docker Compose):"
echo "  • Load Balancer:  http://localhost:$LB_PORT  (nginx round-robin → instance 1/2)"
echo "  • Proxy (LB):     http://localhost:$PROXY_LB_PORT  (configure mobile/device proxy to this)"
echo "  • Instance 1:     http://localhost:$INSTANCE1_API_PORT  (madhyamas-1 API + Web UI)"
echo "  • Instance 2:     http://localhost:$INSTANCE2_API_PORT  (madhyamas-2 API + Web UI)"
echo "  • PostgreSQL:     localhost:$PG_PORT  (shared storage)"
echo "  • Redis:          localhost:$REDIS_PORT  (pub/sub + seat coordination)"
echo ""
echo "Container status:"
docker compose -f "$COMPOSE_FILE" ps --format "table {{.Service}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null || \
    docker compose -f "$COMPOSE_FILE" ps
echo ""
echo "Architecture:"
echo "  ┌─────────┐     ┌──────────────────┐"
echo "  │ Browser │────▶│  nginx (LB)      │:$LB_PORT (Web UI / API)"
echo "  │ Mobile  │────▶│  nginx (LB)      │:$PROXY_LB_PORT (HTTP/HTTPS Proxy)"
echo "  └─────────┘     │  round-robin     │"
echo "                  └──────┬───┬───────┘"
echo "                         │   │"
echo "              ┌──────────┘   └──────────┐"
echo "              ▼                         ▼"
echo "  ┌──────────────────┐       ┌──────────────────┐"
echo "  │ madhyamas-1      │       │ madhyamas-2      │"
echo "  │ API :$INSTANCE1_API_PORT      │       │ API :$INSTANCE2_API_PORT      │"
echo "  │ Proxy :8888      │       │ Proxy :8888      │"
echo "  └────────┬─────────┘       └────────┬─────────┘"
echo "           │                          │"
echo "           ▼                          ▼"
echo "  ┌──────────────────┐       ┌──────────────────┐"
echo "  │ PostgreSQL       │       │ Redis            │"
echo "  │ :$PG_PORT (shared)│       │ :$REDIS_PORT (shared)│"
echo "  └──────────────────┘       └──────────────────┘"
echo ""
echo "Commands:"
echo "  • Stop:          ./stop-local.sh --all"
echo "  • Stop enterprise: ./stop-local.sh --tier enterprise"
echo "  • View logs:     docker compose -f $COMPOSE_FILE logs -f"
echo "  • Instance 1:    docker compose -f $COMPOSE_FILE logs -f madhyamas-1"
echo "  • Instance 2:    docker compose -f $COMPOSE_FILE logs -f madhyamas-2"
echo "  • Restart:       docker compose -f $COMPOSE_FILE restart"
echo "  • Clean rebuild: ./startup-local.sh --clean"
echo ""
echo "Configuration:"
echo "  • Admin user:     ${MADHYAMAS_ADMIN_USERNAME}"
echo "  • Admin password: ${MADHYAMAS_ADMIN_PASSWORD}"
echo "  • JWT secret:     ${MADHYAMAS_JWT_SECRET}"
echo "  • PostgreSQL:     ${POSTGRES_USER}@localhost:$PG_PORT/${POSTGRES_DB}"
echo "  • Redis:          localhost:$REDIS_PORT"
echo "  • Auth enabled:   true"
[ -n "$MADHYAMAS_LICENSE_FILE" ] && echo "  • License file:   $MADHYAMAS_LICENSE_FILE" || echo "  • License file:   none (unlicensed enterprise mode)"
[ -n "$MADHYAMAS_PUBLIC_IP" ] && echo "  • Public IP:      $MADHYAMAS_PUBLIC_IP"
echo ""
echo "Note: Both instances share the same PostgreSQL database and Redis."
echo "      WebSocket connections are sticky (ip_hash) via nginx."
echo "      Traffic events are broadcast across instances via Redis pub/sub."

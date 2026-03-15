#!/bin/bash
# Detect the host machine's LAN IP address for Docker configuration
# This script helps find the correct IP to set as MADHYAMAS_PUBLIC_IP

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 Detecting host network IP addresses..."
echo ""

# Function to check if IP is private
is_private_ip() {
    local ip=$1
    # Check for 192.168.x.x, 10.x.x.x, 172.16-31.x.x
    if [[ $ip =~ ^192\.168\. ]] || [[ $ip =~ ^10\. ]] || [[ $ip =~ ^172\.(1[6-9]|2[0-9]|3[0-1])\. ]]; then
        return 0
    fi
    return 1
}

# Detect OS
OS="$(uname -s)"
RECOMMENDED_IP=""

case "$OS" in
    Darwin)
        # macOS
        echo "📱 Detected: macOS"
        echo ""
        
        # Get all network interfaces with IPs
        echo "Available network interfaces:"
        echo "-----------------------------"
        
        # Prefer en0 (usually WiFi or primary ethernet)
        for iface in en0 en1 en2 en3 en4 en5; do
            ip=$(ipconfig getifaddr $iface 2>/dev/null || true)
            if [ -n "$ip" ] && is_private_ip "$ip"; then
                echo -e "  ${GREEN}$iface${NC}: $ip"
                if [ -z "$RECOMMENDED_IP" ]; then
                    RECOMMENDED_IP=$ip
                fi
            fi
        done
        ;;
        
    Linux)
        # Linux
        echo "🐧 Detected: Linux"
        echo ""
        
        echo "Available network interfaces:"
        echo "-----------------------------"
        
        # Get IPs, excluding docker/bridge interfaces
        while IFS= read -r line; do
            iface=$(echo "$line" | awk '{print $1}' | tr -d ':')
            ip=$(echo "$line" | grep -oP 'inet \K[\d.]+' || true)
            
            # Skip loopback, docker, and bridge interfaces
            if [[ "$iface" == "lo" ]] || [[ "$iface" == docker* ]] || [[ "$iface" == br-* ]] || [[ "$iface" == veth* ]]; then
                continue
            fi
            
            if [ -n "$ip" ] && is_private_ip "$ip"; then
                echo -e "  ${GREEN}$iface${NC}: $ip"
                # Prefer eth0, ens*, enp* interfaces
                if [ -z "$RECOMMENDED_IP" ] || [[ "$iface" == eth0 ]] || [[ "$iface" == ens* ]] || [[ "$iface" == enp* ]]; then
                    RECOMMENDED_IP=$ip
                fi
            fi
        done < <(ip -o -4 addr show 2>/dev/null)
        ;;
        
    *)
        echo -e "${RED}Unsupported OS: $OS${NC}"
        exit 1
        ;;
esac

echo ""

if [ -n "$RECOMMENDED_IP" ]; then
    echo -e "✅ ${GREEN}Recommended IP:${NC} $RECOMMENDED_IP"
    echo ""
    echo "To use this IP with Docker, run:"
    echo ""
    echo -e "  ${YELLOW}export MADHYAMAS_PUBLIC_IP=$RECOMMENDED_IP${NC}"
    echo -e "  ${YELLOW}docker compose up -d${NC}"
    echo ""
    echo "Or run in one command:"
    echo ""
    echo -e "  ${YELLOW}MADHYAMAS_PUBLIC_IP=$RECOMMENDED_IP docker compose up -d${NC}"
    echo ""
    echo "Alternatively, use host network mode (Linux only):"
    echo ""
    echo -e "  ${YELLOW}docker compose --profile host up -d madhyamas-host${NC}"
else
    echo -e "${RED}❌ Could not detect a suitable LAN IP address${NC}"
    echo ""
    echo "Please manually find your IP and set it:"
    echo ""
    echo "  export MADHYAMAS_PUBLIC_IP=<your-ip>"
    echo "  docker compose up -d"
fi

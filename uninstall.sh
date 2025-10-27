#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Uninstalling Anxious Scroll Daemon...${NC}"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}This script must be run as root (use sudo)${NC}"
   exit 1
fi

# Stop the service if it's running
if systemctl is-active --quiet anxious-scroll-daemon.service; then
    echo -e "${YELLOW}Stopping service...${NC}"
    systemctl stop anxious-scroll-daemon.service
fi

# Disable the service
if systemctl is-enabled --quiet anxious-scroll-daemon.service; then
    echo -e "${YELLOW}Disabling service...${NC}"
    systemctl disable anxious-scroll-daemon.service
fi

# Remove service file
if [ -f "/etc/systemd/system/anxious-scroll-daemon.service" ]; then
    echo -e "${YELLOW}Removing service file...${NC}"
    rm /etc/systemd/system/anxious-scroll-daemon.service
fi

# Remove binary
if [ -f "/usr/local/bin/anxious-scroll-daemon" ]; then
    echo -e "${YELLOW}Removing binary...${NC}"
    rm /usr/local/bin/anxious-scroll-daemon
fi

# Reload systemd daemon
echo -e "${YELLOW}Reloading systemd daemon...${NC}"
systemctl daemon-reload

echo -e "${GREEN}Uninstallation complete!${NC}"
echo "The daemon has been completely removed from your system."

#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Installing Anxious Scroll Daemon...${NC}"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}This script must be run as root (use sudo)${NC}"
   exit 1
fi

# Find the original user
ORIGINAL_USER=""
if [ -n "$SUDO_USER" ]; then
    ORIGINAL_USER="$SUDO_USER"
elif [ -n "$USER" ] && [ "$USER" != "root" ]; then
    ORIGINAL_USER="$USER"
else
    # Last resort: try to find a non-root user
    ORIGINAL_USER=$(logname 2>/dev/null || echo "")
fi

if [ -z "$ORIGINAL_USER" ]; then
    echo -e "${RED}Error: Could not determine original user.${NC}"
    exit 1
fi

# Check if cargo is available in user environment
CARGO_CMD=""
USER_HOME=$(getent passwd "$ORIGINAL_USER" | cut -d: -f6)
USER_CARGO="$USER_HOME/.cargo/bin/cargo"

if [ -f "$USER_CARGO" ]; then
    CARGO_CMD="sudo -u $ORIGINAL_USER $USER_CARGO"
elif sudo -u "$ORIGINAL_USER" command -v cargo &> /dev/null; then
    CARGO_CMD="sudo -u $ORIGINAL_USER cargo"
fi

if [ -z "$CARGO_CMD" ]; then
    echo -e "${RED}Error: cargo not found in user environment.${NC}"
    echo ""
    echo "Would you like to install Rust toolchain now? (y/N)"
    read -r -n 1 -p "> " response
    echo
    if [[ $response =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Installing Rust toolchain for user $ORIGINAL_USER...${NC}"
        # Download and run rustup as the original user
        sudo -u "$ORIGINAL_USER" bash -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
        
        # Source the cargo environment and try again
        if [ -f "$USER_CARGO" ]; then
            CARGO_CMD="sudo -u $ORIGINAL_USER $USER_CARGO"
            echo -e "${GREEN}Rust toolchain installed successfully!${NC}"
        else
            echo -e "${RED}Failed to install Rust toolchain.${NC}"
            echo "Please install manually: https://rustup.rs/"
            exit 1
        fi
    else
        echo "Please install Rust toolchain manually: https://rustup.rs/"
        exit 1
    fi
fi

# Build the release binary
echo -e "${YELLOW}Building release binary...${NC}"
$CARGO_CMD build --release

# Check if build was successful
if [ ! -f "target/release/anxious-scroll-daemon" ]; then
    echo -e "${RED}Error: Build failed. Binary not found.${NC}"
    exit 1
fi

# Create directories if they don't exist
mkdir -p /usr/local/bin
mkdir -p /etc/systemd/system

# Copy binary
echo -e "${YELLOW}Installing binary to /usr/local/bin/...${NC}"
cp target/release/anxious-scroll-daemon /usr/local/bin/
chmod +x /usr/local/bin/anxious-scroll-daemon

# Copy service file
echo -e "${YELLOW}Installing systemd service...${NC}"
cp anxious-scroll-daemon.service /etc/systemd/system/

# Reload systemd daemon
echo -e "${YELLOW}Reloading systemd daemon...${NC}"
systemctl daemon-reload

# Enable service for auto-start on boot
echo -e "${YELLOW}Enabling service for auto-start...${NC}"
systemctl enable anxious-scroll-daemon.service

# Ask if user wants to start the service now
echo -e "${GREEN}Installation complete!${NC}"
echo ""
read -p "Do you want to start the service now? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Starting service...${NC}"
    systemctl start anxious-scroll-daemon.service
    echo -e "${GREEN}Service started!${NC}"
    echo ""
    echo "To check service status: sudo systemctl status anxious-scroll-daemon"
    echo "To view logs: sudo journalctl -u anxious-scroll-daemon -f"
else
    echo "Service installed but not started. Start it manually with:"
    echo "  sudo systemctl start anxious-scroll-daemon"
fi

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo "The daemon will now start automatically on system boot."

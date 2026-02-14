#!/bin/bash
# Ajime Agent Installation Script
# Usage: curl -sSL https://install.ajime.io | bash -s -- --token=<activation_token>

set -e

# Configuration
AJIME_VERSION="${AJIME_VERSION:-latest}"
AJIME_INSTALL_DIR="${AJIME_INSTALL_DIR:-/usr/local/bin}"
AJIME_CONFIG_DIR="${AJIME_CONFIG_DIR:-/etc/ajime}"
AJIME_DOWNLOAD_URL="${AJIME_DOWNLOAD_URL:-https://releases.ajime.io}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse command line arguments
ACTIVATION_TOKEN=""
DEVICE_NAME=""
BACKEND_URL=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --token=*)
            ACTIVATION_TOKEN="${1#*=}"
            shift
            ;;
        --name=*)
            DEVICE_NAME="${1#*=}"
            shift
            ;;
        --backend=*)
            BACKEND_URL="${1#*=}"
            shift
            ;;
        --version=*)
            AJIME_VERSION="${1#*=}"
            shift
            ;;
        --help|-h)
            echo "Ajime Agent Installer"
            echo ""
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --token=TOKEN     Activation token (required)"
            echo "  --name=NAME       Device name (optional, defaults to hostname)"
            echo "  --backend=URL     Backend URL (optional)"
            echo "  --version=VER     Agent version (optional, defaults to latest)"
            echo "  --help            Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check for activation token
if [ -z "$ACTIVATION_TOKEN" ]; then
    # Check environment variable
    if [ -n "$AJIME_ACTIVATION_TOKEN" ]; then
        ACTIVATION_TOKEN="$AJIME_ACTIVATION_TOKEN"
    else
        log_error "Activation token is required"
        echo "Usage: $0 --token=<activation_token>"
        exit 1
    fi
fi

# Detect platform
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case "$os" in
        linux)
            OS="linux"
            ;;
        darwin)
            OS="darwin"
            ;;
        *)
            log_error "Unsupported operating system: $os"
            exit 1
            ;;
    esac
    
    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        armv7l|armhf)
            ARCH="armv7"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac
    
    PLATFORM="${OS}-${ARCH}"
}

# Detect device type
detect_device_type() {
    DEVICE_TYPE="generic"
    
    # Check for Raspberry Pi
    if [ -f /proc/device-tree/model ]; then
        local model=$(cat /proc/device-tree/model | tr -d '\0')
        if echo "$model" | grep -qi "raspberry pi"; then
            DEVICE_TYPE="raspberry_pi"
            log_info "Detected Raspberry Pi: $model"
            return
        fi
        if echo "$model" | grep -qi "jetson"; then
            DEVICE_TYPE="jetson"
            log_info "Detected NVIDIA Jetson: $model"
            return
        fi
    fi
    
    # Check for Jetson via tegra
    if [ -f /etc/nv_tegra_release ]; then
        DEVICE_TYPE="jetson"
        log_info "Detected NVIDIA Jetson (via tegra)"
        return
    fi
    
    log_info "Device type: $DEVICE_TYPE"
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run as root"
        echo "Please run: sudo $0 $@"
        exit 1
    fi
}

# Download agent binary
download_agent() {
    log_info "Downloading Ajime Agent ${AJIME_VERSION} for ${PLATFORM}..."
    
    local download_url="${AJIME_DOWNLOAD_URL}/ajigent-${AJIME_VERSION}-${PLATFORM}"
    local temp_file="/tmp/ajigent-download"
    
    # Download binary
    if command -v curl &> /dev/null; then
        curl -fsSL "$download_url" -o "$temp_file" || {
            log_error "Failed to download agent from $download_url"
            exit 1
        }
    elif command -v wget &> /dev/null; then
        wget -q "$download_url" -O "$temp_file" || {
            log_error "Failed to download agent from $download_url"
            exit 1
        }
    else
        log_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
    
    # Verify download
    if [ ! -f "$temp_file" ]; then
        log_error "Download failed: file not found"
        exit 1
    fi
    
    # Make executable and move to install dir
    chmod +x "$temp_file"
    mv "$temp_file" "${AJIME_INSTALL_DIR}/ajigent"

    log_success "Agent binary installed to ${AJIME_INSTALL_DIR}/ajigent"
}

# Create directories
create_directories() {
    log_info "Creating directories..."
    
    mkdir -p "$AJIME_CONFIG_DIR"
    mkdir -p "$AJIME_CONFIG_DIR/cache"
    mkdir -p "$AJIME_CONFIG_DIR/cache/workflows"
    mkdir -p "$AJIME_CONFIG_DIR/cache/configs"
    mkdir -p "$AJIME_CONFIG_DIR/deployments"
    mkdir -p "$AJIME_CONFIG_DIR/logs"
    mkdir -p "$AJIME_CONFIG_DIR/tokens"
    mkdir -p /var/log/ajime
    
    # Set permissions
    chmod 700 "$AJIME_CONFIG_DIR"
    chmod 700 "$AJIME_CONFIG_DIR/tokens"
    
    log_success "Directories created"
}

# Install systemd service
install_systemd_service() {
    log_info "Installing systemd service..."
    
    cat > /lib/systemd/system/ajigent.service << 'EOF'
[Unit]
Description=Ajigent Edge Agent
Documentation=https://docs.ajime.io/agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root
ExecStart=/usr/local/bin/ajigent
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=ajigent

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/etc/ajime /var/log/ajime
PrivateTmp=true

# Resource limits
MemoryMax=256M
CPUQuota=50%

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    
    log_success "Systemd service installed"
}

# Activate the agent
activate_agent() {
    log_info "Activating agent..."
    
    local args="--install --token=$ACTIVATION_TOKEN"
    
    if [ -n "$DEVICE_NAME" ]; then
        args="$args --name=$DEVICE_NAME"
    fi
    
    if [ -n "$BACKEND_URL" ]; then
        args="$args --backend=$BACKEND_URL"
    fi
    
    args="$args --type=$DEVICE_TYPE"
    
    # Run activation
    ${AJIME_INSTALL_DIR}/ajigent $args
    
    if [ $? -eq 0 ]; then
        log_success "Agent activated successfully"
    else
        log_error "Agent activation failed"
        exit 1
    fi
}

# Start the service
start_service() {
    log_info "Starting Ajigent service..."
    
    systemctl enable ajigent
    systemctl start ajigent
    
    # Wait a moment and check status
    sleep 2
    
    if systemctl is-active --quiet ajigent; then
        log_success "Ajigent is running"
    else
        log_warn "Agent may not have started correctly. Check: journalctl -u ajigent"
    fi
}

# Main installation flow
main() {
    echo ""
    echo "======================================"
    echo "  Ajigent Installer"
    echo "======================================"
    echo ""
    
    check_root
    detect_platform
    detect_device_type
    
    log_info "Platform: $PLATFORM"
    log_info "Device type: $DEVICE_TYPE"
    echo ""
    
    create_directories
    download_agent
    install_systemd_service
    activate_agent
    start_service
    
    echo ""
    echo "======================================"
    log_success "Installation complete!"
    echo "======================================"
    echo ""
    echo "Useful commands:"
    echo "  Check status:  systemctl status ajigent"
    echo "  View logs:     journalctl -u ajigent -f"
    echo "  Restart:       systemctl restart ajigent"
    echo "  Stop:          systemctl stop ajigent"
    echo ""
}

# Run main
main

#!/bin/bash
# Alejandria HTTP Transport Deployment Script
# Usage: ./deploy-http.sh [--rollback]

set -euo pipefail

# Configuration
REMOTE_SERVER="your-server.example.com"
BUILD_DIR="/veritran/alejandria-build"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/var/lib/alejandria"
CONFIG_DIR="/etc/alejandria"
SYSTEMD_DIR="/etc/systemd/system"
NGINX_DIR="/etc/nginx/sites-available"
NGINX_ENABLED="/etc/nginx/sites-enabled"
SERVICE_NAME="alejandria-http"
USER="alejandria"
GROUP="alejandria"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running with --rollback flag
if [[ "${1:-}" == "--rollback" ]]; then
    log_warn "Rollback mode enabled"
    ROLLBACK=true
else
    ROLLBACK=false
fi

# Step 1: Sync code and build on remote server
if [[ "$ROLLBACK" == false ]]; then
    log_info "Step 1: Syncing code to $REMOTE_SERVER..."
    rsync -avz --exclude target --exclude .git --exclude .vtspec . "$REMOTE_SERVER:$BUILD_DIR/"

    log_info "Step 2: Building on remote server..."
    ssh "$REMOTE_SERVER" "cd $BUILD_DIR && source ~/.cargo/env && cargo build --release --bin alejandria --features http-transport,encryption"
    
    log_info "Step 3: Verifying build..."
    ssh "$REMOTE_SERVER" "test -f $BUILD_DIR/target/release/alejandria && ls -lh $BUILD_DIR/target/release/alejandria"
fi

# Step 4: Backup current installation (if exists)
log_info "Step 4: Backing up current installation..."
BACKUP_DIR="/tmp/alejandria-backup-$(date +%Y%m%d-%H%M%S)"
ssh "$REMOTE_SERVER" "sudo mkdir -p $BACKUP_DIR && \
    (test -f $INSTALL_DIR/alejandria && sudo cp $INSTALL_DIR/alejandria $BACKUP_DIR/ || true) && \
    (test -f $SYSTEMD_DIR/$SERVICE_NAME.service && sudo cp $SYSTEMD_DIR/$SERVICE_NAME.service $BACKUP_DIR/ || true)"
log_info "Backup created at $BACKUP_DIR"

# Step 5: Stop service if running
log_info "Step 5: Stopping $SERVICE_NAME service (if running)..."
ssh "$REMOTE_SERVER" "sudo systemctl stop $SERVICE_NAME || true"

# Step 6: Install or rollback binary
if [[ "$ROLLBACK" == false ]]; then
    log_info "Step 6: Installing new binary..."
    ssh "$REMOTE_SERVER" "sudo cp $BUILD_DIR/target/release/alejandria $INSTALL_DIR/alejandria && \
        sudo chmod +x $INSTALL_DIR/alejandria && \
        sudo chown root:root $INSTALL_DIR/alejandria"
else
    log_warn "Step 6: Rolling back to previous binary..."
    LATEST_BACKUP=$(ssh "$REMOTE_SERVER" "ls -td /tmp/alejandria-backup-* | head -1")
    ssh "$REMOTE_SERVER" "sudo cp $LATEST_BACKUP/alejandria $INSTALL_DIR/alejandria && \
        sudo chmod +x $INSTALL_DIR/alejandria"
fi

# Step 7: Create user and directories (if not exist)
log_info "Step 7: Creating user and directories..."
ssh "$REMOTE_SERVER" "sudo useradd -r -s /bin/false $USER 2>/dev/null || true && \
    sudo mkdir -p $DATA_DIR $CONFIG_DIR && \
    sudo chown $USER:$GROUP $DATA_DIR && \
    sudo chmod 750 $DATA_DIR"

# Step 8: Install systemd service
if [[ "$ROLLBACK" == false ]]; then
    log_info "Step 8: Installing systemd service..."
    scp deployment/alejandria-http.service "$REMOTE_SERVER:/tmp/"
    ssh "$REMOTE_SERVER" "sudo mv /tmp/alejandria-http.service $SYSTEMD_DIR/$SERVICE_NAME.service && \
        sudo chmod 644 $SYSTEMD_DIR/$SERVICE_NAME.service && \
        sudo systemctl daemon-reload"
fi

# Step 9: Install Nginx configuration
if [[ "$ROLLBACK" == false ]]; then
    log_info "Step 9: Installing Nginx configuration..."
    scp deployment/nginx-alejandria.conf "$REMOTE_SERVER:/tmp/"
    scp deployment/nginx-snippets-alejandria-locations.conf "$REMOTE_SERVER:/tmp/"
    ssh "$REMOTE_SERVER" "sudo mkdir -p /etc/nginx/snippets && \
        sudo mv /tmp/nginx-alejandria.conf $NGINX_DIR/alejandria.conf && \
        sudo mv /tmp/nginx-snippets-alejandria-locations.conf /etc/nginx/snippets/alejandria-locations.conf && \
        sudo ln -sf $NGINX_DIR/alejandria.conf $NGINX_ENABLED/alejandria.conf || true && \
        sudo nginx -t"
fi

# Step 10: Generate API key and instance ID (if not exist)
log_info "Step 10: Setting up API keys and instance ID..."
ssh "$REMOTE_SERVER" "if [ ! -f $CONFIG_DIR/api_keys.txt ]; then \
        API_KEY=\$(openssl rand -hex 32) && \
        echo \$API_KEY | sudo tee $CONFIG_DIR/api_keys.txt > /dev/null && \
        echo \$API_KEY | sha256sum | awk '{print \$1}' | sudo tee -a $CONFIG_DIR/api_keys.txt > /dev/null && \
        log_info 'Generated new API key (saved to $CONFIG_DIR/api_keys.txt)'; \
    fi && \
    if [ ! -f $CONFIG_DIR/instance_id.txt ]; then \
        uuidgen | sudo tee $CONFIG_DIR/instance_id.txt > /dev/null && \
        log_info 'Generated new instance ID'; \
    fi && \
    sudo chmod 600 $CONFIG_DIR/api_keys.txt $CONFIG_DIR/instance_id.txt && \
    sudo chown $USER:$GROUP $CONFIG_DIR/api_keys.txt $CONFIG_DIR/instance_id.txt"

# Step 11: Start service
log_info "Step 11: Starting $SERVICE_NAME service..."
ssh "$REMOTE_SERVER" "sudo systemctl enable $SERVICE_NAME && \
    sudo systemctl start $SERVICE_NAME"

# Step 12: Wait for service to start
log_info "Step 12: Waiting for service to start..."
sleep 5

# Step 13: Health check
log_info "Step 13: Running health check..."
HEALTH_CHECK=$(ssh "$REMOTE_SERVER" "curl -s http://127.0.0.1:8080/health || echo 'FAILED'")
if [[ "$HEALTH_CHECK" == *"healthy"* ]]; then
    log_info "Health check PASSED: $HEALTH_CHECK"
else
    log_error "Health check FAILED: $HEALTH_CHECK"
    log_error "Checking service status..."
    ssh "$REMOTE_SERVER" "sudo systemctl status $SERVICE_NAME --no-pager"
    exit 1
fi

# Step 14: Reload Nginx
if [[ "$ROLLBACK" == false ]]; then
    log_info "Step 14: Reloading Nginx..."
    ssh "$REMOTE_SERVER" "sudo systemctl reload nginx"
fi

# Step 15: Display service status
log_info "Step 15: Service status:"
ssh "$REMOTE_SERVER" "sudo systemctl status $SERVICE_NAME --no-pager | head -20"

# Success message
log_info "========================================="
if [[ "$ROLLBACK" == false ]]; then
    log_info "Deployment completed successfully!"
    log_info "Backup saved to: $BACKUP_DIR"
    log_info "To rollback: ./deploy-http.sh --rollback"
else
    log_info "Rollback completed successfully!"
fi
log_info "========================================="

# Display API key reminder
log_warn "IMPORTANT: Save your API key from $CONFIG_DIR/api_keys.txt on the server"
log_warn "The first line is the raw key (use this in clients)"
log_warn "The second line is the SHA-256 hash (for reference only)"

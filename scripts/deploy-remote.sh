#!/usr/bin/env bash
set -euo pipefail

# Alejandria Remote Deployment Script
# Builds on remote server to avoid local disk consumption

REMOTE_HOST="${1:-}"
DEPLOY_USER="${2:-alejandria}"
INSTALL_DIR="/opt/alejandria"
CONFIG_DIR="/etc/alejandria"
DATA_DIR="/var/lib/alejandria"
SERVICE_NAME="alejandria-mcp"

if [ -z "$REMOTE_HOST" ]; then
    echo "❌ Error: Remote host required"
    echo "Usage: $0 <remote-host> [deploy-user]"
    echo "Example: $0 server.example.com alejandria"
    exit 1
fi

echo "🚀 Alejandria Remote Deployment"
echo "================================"
echo "Remote host: $REMOTE_HOST"
echo "Deploy user: $DEPLOY_USER"
echo ""

# Check remote connection
echo "📡 Checking remote connection..."
if ! ssh -o ConnectTimeout=5 "$REMOTE_HOST" "echo 'Connected'"; then
    echo "❌ Cannot connect to $REMOTE_HOST"
    exit 1
fi

echo "✅ Connection successful"
echo ""

# Check disk space on remote
echo "💾 Checking remote disk space..."
DISK_USAGE=$(ssh "$REMOTE_HOST" "df -h / | tail -1 | awk '{print \$5}' | tr -d '%'")
if [ "$DISK_USAGE" -gt 80 ]; then
    echo "⚠️  Warning: Disk usage is ${DISK_USAGE}% on remote server"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "✅ Disk space: ${DISK_USAGE}% used"
echo ""

# Check if Rust is installed on remote
echo "🦀 Checking Rust installation on remote..."
if ! ssh "$REMOTE_HOST" "command -v cargo &>/dev/null"; then
    echo "⚠️  Rust not found on remote server"
    echo "Installing Rust..."
    ssh "$REMOTE_HOST" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    ssh "$REMOTE_HOST" "source \$HOME/.cargo/env"
else
    RUST_VERSION=$(ssh "$REMOTE_HOST" "cargo --version")
    echo "✅ Rust installed: $RUST_VERSION"
fi
echo ""

# Create deploy user if doesn't exist
echo "👤 Setting up deploy user..."
ssh "$REMOTE_HOST" "sudo useradd -m -s /bin/bash $DEPLOY_USER 2>/dev/null || echo 'User already exists'"
echo "✅ User $DEPLOY_USER ready"
echo ""

# Create directory structure
echo "📁 Creating directory structure..."
ssh "$REMOTE_HOST" "sudo mkdir -p $INSTALL_DIR $CONFIG_DIR $DATA_DIR"
ssh "$REMOTE_HOST" "sudo chown -R $DEPLOY_USER:$DEPLOY_USER $INSTALL_DIR $DATA_DIR"
ssh "$REMOTE_HOST" "sudo chmod 755 $INSTALL_DIR"
ssh "$REMOTE_HOST" "sudo chmod 700 $DATA_DIR"
echo "✅ Directories created"
echo ""

# Transfer source code (excluding target/)
echo "📦 Transferring source code..."
TEMP_DIR=$(mktemp -d)
git archive --format=tar HEAD | tar -C "$TEMP_DIR" -xf -
rsync -avz --exclude='target' --exclude='.git' \
    "$TEMP_DIR/" "$REMOTE_HOST:$INSTALL_DIR/"
rm -rf "$TEMP_DIR"
echo "✅ Source code transferred"
echo ""

# Build on remote server
echo "🔨 Building Alejandria on remote server (this may take 5-10 minutes)..."
echo "   Building with release optimizations and embeddings support..."
ssh "$REMOTE_HOST" "cd $INSTALL_DIR && source \$HOME/.cargo/env && cargo build --release --all-features" &
BUILD_PID=$!

# Show progress
while kill -0 $BUILD_PID 2>/dev/null; do
    echo -n "."
    sleep 5
done
wait $BUILD_PID
BUILD_EXIT=$?

if [ $BUILD_EXIT -ne 0 ]; then
    echo ""
    echo "❌ Build failed on remote server"
    exit 1
fi

echo ""
echo "✅ Build successful"
echo ""

# Check binary size
BINARY_SIZE=$(ssh "$REMOTE_HOST" "du -h $INSTALL_DIR/target/release/alejandria | cut -f1")
echo "📊 Binary size: $BINARY_SIZE"
echo ""

# Install binary
echo "📥 Installing binary..."
ssh "$REMOTE_HOST" "sudo cp $INSTALL_DIR/target/release/alejandria /usr/local/bin/"
ssh "$REMOTE_HOST" "sudo chmod 755 /usr/local/bin/alejandria"
echo "✅ Binary installed to /usr/local/bin/alejandria"
echo ""

# Install default config
echo "⚙️  Installing default configuration..."
ssh "$REMOTE_HOST" "sudo cp $INSTALL_DIR/config/default.toml $CONFIG_DIR/config.toml"
ssh "$REMOTE_HOST" "sudo chown root:root $CONFIG_DIR/config.toml"
ssh "$REMOTE_HOST" "sudo chmod 644 $CONFIG_DIR/config.toml"
echo "✅ Config installed to $CONFIG_DIR/config.toml"
echo ""

# Create systemd service
echo "🔧 Creating systemd service..."
ssh "$REMOTE_HOST" "sudo tee /etc/systemd/system/$SERVICE_NAME.service" > /dev/null << EOF
[Unit]
Description=Alejandria MCP Server
After=network.target

[Service]
Type=simple
User=$DEPLOY_USER
Environment=ALEJANDRIA_DB_PATH=$DATA_DIR/memories.db
ExecStart=/usr/local/bin/alejandria serve
Restart=on-failure
RestartSec=5s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR

[Install]
WantedBy=multi-user.target
EOF

ssh "$REMOTE_HOST" "sudo systemctl daemon-reload"
echo "✅ Systemd service created"
echo ""

# Run verification script
echo "🔍 Running deployment verification..."
ssh "$REMOTE_HOST" "cd $INSTALL_DIR && bash scripts/verify-deployment.sh" || true
echo ""

# Start service
echo "🚀 Starting Alejandria service..."
ssh "$REMOTE_HOST" "sudo systemctl enable $SERVICE_NAME"
ssh "$REMOTE_HOST" "sudo systemctl start $SERVICE_NAME"
sleep 2
if ssh "$REMOTE_HOST" "sudo systemctl is-active --quiet $SERVICE_NAME"; then
    echo "✅ Service started successfully"
else
    echo "⚠️  Service failed to start. Checking logs..."
    ssh "$REMOTE_HOST" "sudo journalctl -u $SERVICE_NAME -n 20 --no-pager"
    exit 1
fi
echo ""

# Show service status
echo "📊 Service Status:"
ssh "$REMOTE_HOST" "sudo systemctl status $SERVICE_NAME --no-pager -l"
echo ""

# Cleanup build artifacts on remote to free space
echo "🧹 Cleaning up build artifacts on remote..."
ssh "$REMOTE_HOST" "cd $INSTALL_DIR && cargo clean"
echo "✅ Build artifacts cleaned"
echo ""

echo "🎉 Deployment Complete!"
echo "======================"
echo ""
echo "Service: $SERVICE_NAME"
echo "Binary: /usr/local/bin/alejandria"
echo "Config: $CONFIG_DIR/config.toml"
echo "Data: $DATA_DIR/memories.db"
echo ""
echo "Useful commands:"
echo "  sudo systemctl status $SERVICE_NAME"
echo "  sudo journalctl -u $SERVICE_NAME -f"
echo "  alejandria --version"
echo ""
echo "Next steps:"
echo "1. Edit $CONFIG_DIR/config.toml if needed"
echo "2. Test with: alejandria stats"
echo "3. Configure client to connect via stdio"

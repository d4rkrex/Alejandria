# Deployment Guide

Comprehensive deployment guide for Alejandria covering all deployment scenarios from Claude Desktop integration to production containerized deployments.

**Last Updated**: April 4, 2026  
**Alejandria Version**: 0.1.0

## Table of Contents

- [Prerequisites](#prerequisites)
- [Building from Source](#building-from-source)
- [Deployment Scenarios](#deployment-scenarios)
  - [Claude Desktop Integration (Recommended)](#claude-desktop-integration-recommended)
  - [Standalone Daemon Mode](#standalone-daemon-mode)
  - [Library Integration](#library-integration)
  - [Docker Deployment](#docker-deployment)
- [Configuration](#configuration)
- [Verification](#verification)
- [Upgrading](#upgrading)
- [Troubleshooting](#troubleshooting)
- [Production Deployment](#production-deployment)

---

## Prerequisites

### System Requirements

- **Operating System**: Linux (Ubuntu 20.04+), macOS (10.15+), or Windows (10/11)
- **Rust**: 1.70+ (2021 edition)
- **Memory**: 512MB minimum, 1GB recommended (with embeddings feature)
- **Disk**: 100MB for binary + database storage (scales with usage)

### Rust Installation

```bash
# Install Rust via rustup (all platforms)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version  # Should be 1.70 or higher
cargo --version
```

**Platform-Specific Notes**:

<details>
<summary><b>macOS</b></summary>

```bash
# Install Xcode Command Line Tools (if not already installed)
xcode-select --install

# Verify
cc --version  # Should show Apple clang version
```
</details>

<details>
<summary><b>Windows</b></summary>

1. Download and install Rust from https://rustup.rs/
2. Install Visual Studio Build Tools with C++ workload
   - Download: https://visualstudio.microsoft.com/downloads/
   - Select "Desktop development with C++" workload

Use PowerShell or CMD (not Git Bash) for best compatibility.
</details>

<details>
<summary><b>Linux</b></summary>

```bash
# Ubuntu/Debian - SQLite is bundled, but system headers speed up builds
sudo apt-get update
sudo apt-get install build-essential libsqlite3-dev  # libsqlite3-dev is optional

# Fedora/RHEL
sudo dnf install gcc sqlite-devel

# Arch
sudo pacman -S base-devel sqlite
```
</details>

---

## Building from Source

### Standard Build (Recommended)

```bash
# 1. Clone repository
git clone https://github.com/yourusername/alejandria.git
cd alejandria

# 2. Build CLI and MCP server with embeddings feature
cargo build --release --all-features

# 3. Verify build
./target/release/alejandria --version
# Should output: alejandria 0.1.0

# 4. Run tests (optional but recommended)
cargo test --all-features
```

**Build artifacts**:
- CLI binary: `target/release/alejandria` (~50MB with embeddings)
- Database will be created on first run at `~/.local/share/alejandria/alejandria.db`

### Minimal Build (Without Embeddings)

For smaller binary size and faster compilation:

```bash
# Build without embeddings feature
cargo build --release --no-default-features

# Binary size: ~10MB (vs ~50MB with embeddings)
# Trade-off: No vector similarity search, BM25 keyword search only
```

### Installation Locations

**Recommended installation paths**:

```bash
# Linux/macOS - Install to user bin directory
mkdir -p ~/.local/bin
cp target/release/alejandria ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"  # Add to ~/.bashrc or ~/.zshrc

# macOS - Alternative system-wide install
sudo cp target/release/alejandria /usr/local/bin/

# Windows - Add to PATH
# Copy alejandria.exe to C:\Users\<YourName>\bin\
# Add C:\Users\<YourName>\bin to System PATH via System Properties
```

**Verify installation**:

```bash
alejandria --version
alejandria --help
```

---

## Deployment Scenarios

### Claude Desktop Integration (Recommended)

**Use Case**: AI agent memory for Claude Desktop app via MCP protocol.

**Setup Steps**:

1. **Build Alejandria** (see [Building from Source](#building-from-source))

2. **Locate your Claude Desktop configuration**:

```bash
# macOS
~/Library/Application Support/Claude/claude_desktop_config.json

# Windows
%APPDATA%\Claude\claude_desktop_config.json

# Linux
~/.config/Claude/claude_desktop_config.json
```

3. **Add Alejandria MCP server to config** (see full example in `examples/claude-desktop/claude_desktop_config.json.example`):

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/home/yourusername/.local/share/alejandria/claude.db",
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Important**:
- Use **absolute paths** for both `command` and `ALEJANDRIA_DB_PATH`
- Replace `/home/yourusername` with your actual home directory
- On Windows, use forward slashes or escaped backslashes: `C:/Users/YourName/...`

4. **Restart Claude Desktop** to load the MCP server

5. **Verify in Claude**:

```
You: Can you list my memory topics?
Claude: [Uses mem_list_topics tool] You don't have any topics yet...
```

**Advanced Configuration**:

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/yourusername/.local/bin/alejandria",
      "args": ["serve"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/home/yourusername/.local/share/alejandria/claude.db",
        "ALEJANDRIA_SEARCH_LIMIT": "20",
        "ALEJANDRIA_SEARCH_MIN_SCORE": "0.3",
        "ALEJANDRIA_DECAY_AUTO_DECAY": "true",
        "RUST_LOG": "warn"
      }
    }
  }
}
```

See `examples/claude-desktop/claude_desktop_config.json.example` for full configuration reference.

---

### Standalone Daemon Mode

**Use Case**: Run Alejandria as a background service for custom MCP clients.

**Direct Execution** (testing):

```bash
# Start MCP server on stdio
alejandria serve

# Server will read JSON-RPC from stdin and write to stdout
# Example request:
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | alejandria serve
```

**Systemd Service** (Linux production):

1. Create service file `/etc/systemd/system/alejandria.service`:

```ini
[Unit]
Description=Alejandria MCP Server
After=network.target

[Service]
Type=simple
User=alejandria
Group=alejandria
WorkingDirectory=/var/lib/alejandria
ExecStart=/usr/local/bin/alejandria serve
Environment="ALEJANDRIA_DB_PATH=/var/lib/alejandria/memories.db"
Environment="RUST_LOG=info"
StandardInput=socket
StandardOutput=socket
Restart=on-failure
RestartSec=10s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/alejandria

[Install]
WantedBy=multi-user.target
```

2. Enable and start:

```bash
# Create user and data directory
sudo useradd -r -s /bin/false alejandria
sudo mkdir -p /var/lib/alejandria
sudo chown alejandria:alejandria /var/lib/alejandria

# Install binary
sudo cp target/release/alejandria /usr/local/bin/
sudo chmod 755 /usr/local/bin/alejandria

# Enable service
sudo systemctl daemon-reload
sudo systemctl enable alejandria.service
sudo systemctl start alejandria.service

# Check status
sudo systemctl status alejandria.service
sudo journalctl -u alejandria.service -f
```

**macOS LaunchAgent**:

Create `~/Library/LaunchAgents/com.alejandria.mcp.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.alejandria.mcp</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/yourusername/.local/bin/alejandria</string>
        <string>serve</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>ALEJANDRIA_DB_PATH</key>
        <string>/Users/yourusername/.local/share/alejandria/memories.db</string>
    </dict>
    <key>StandardInPath</key>
    <string>/tmp/alejandria.stdin</string>
    <key>StandardOutPath</key>
    <string>/tmp/alejandria.stdout</string>
    <key>StandardErrorPath</key>
    <string>/tmp/alejandria.stderr</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Load with:

```bash
launchctl load ~/Library/LaunchAgents/com.alejandria.mcp.plist
launchctl start com.alejandria.mcp
```

---

### Library Integration

**Use Case**: Embed Alejandria directly in your Rust application.

**Add to `Cargo.toml`**:

```toml
[dependencies]
alejandria-core = { path = "/path/to/alejandria/crates/alejandria-core" }
alejandria-storage = { path = "/path/to/alejandria/crates/alejandria-storage" }

# Or from crates.io once published:
# alejandria-core = "0.1"
# alejandria-storage = "0.1"
```

**Basic Usage**:

```rust
use alejandria_storage::SqliteStore;
use alejandria_core::{Memory, Importance, MemoryStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open database
    let store = SqliteStore::open("memories.db")?;
    
    // Store a memory
    let mut memory = Memory::new(
        "rust-patterns".to_string(),
        "Builder pattern for complex object construction".to_string(),
    );
    memory.importance = Importance::High;
    memory.topic_key = Some("rust/patterns/builder".to_string());
    
    let id = store.store(memory)?;
    println!("Stored memory: {}", id);
    
    // Search memories
    let results = store.search_by_keywords("builder pattern", 5)?;
    println!("Found {} memories", results.len());
    
    for memory in results {
        println!("- [{}] {} (weight: {:.2})", 
            memory.id, memory.summary, memory.weight);
    }
    
    Ok(())
}
```

**Advanced: Custom Memory Store Implementation**:

```rust
use alejandria_core::{MemoryStore, Memory, RecallOptions};

struct MyCustomStore {
    // Your storage backend (PostgreSQL, Redis, etc.)
}

impl MemoryStore for MyCustomStore {
    fn store(&self, memory: Memory) -> Result<String> {
        // Your implementation
        todo!()
    }
    
    fn recall(&self, query: &str, opts: RecallOptions) -> Result<Vec<Memory>> {
        // Your implementation
        todo!()
    }
    
    // ... implement other trait methods
}
```

See `crates/alejandria-core/src/lib.rs` for complete `MemoryStore` trait definition.

---

### Docker Deployment

**Use Case**: Containerized deployment for production, CI/CD, or multi-platform distribution.

#### Quick Start

```bash
# Run CLI with example query
docker run --rm alejandria-cli:latest recall "authentication" --limit 5

# Check version
docker run --rm alejandria-cli:latest --version

# View all available commands
docker run --rm alejandria-cli:latest --help
```

#### Running MCP Server with Docker

```bash
# Start MCP server with persistent volume
docker run -d \
  --name alejandria-mcp \
  -v alejandria-data:/data \
  -e ALEJANDRIA_DB_PATH=/data/alejandria.db \
  alejandria-mcp:latest

# Check server logs
docker logs alejandria-mcp

# Stop server
docker stop alejandria-mcp && docker rm alejandria-mcp
```

#### Using Docker Compose (Recommended)

The recommended way to run Alejandria with Docker:

```bash
# Start MCP server in background
docker-compose up -d

# Check service status
docker-compose ps

# View logs
docker-compose logs -f alejandria-mcp

# Run one-off CLI commands
docker-compose run --rm cli recall "authentication" --limit 5
docker-compose run --rm cli topics
docker-compose run --rm cli stats

# Stop all services
docker-compose down
```

**Development mode** with bind mounts (see your data files):

```bash
# Uses docker-compose.dev.yml override
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# Data is now in ./local-data/ directory
ls -la ./local-data/
```

#### Volume Persistence

**Named volumes** (recommended for production):
```bash
# Docker manages storage location
docker volume create alejandria-prod-data
docker run -d -v alejandria-prod-data:/data alejandria-mcp:latest

# Backup volume
docker run --rm -v alejandria-prod-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/alejandria-backup.tar.gz /data

# Restore volume
docker run --rm -v alejandria-prod-data:/data -v $(pwd):/backup \
  alpine tar xzf /backup/alejandria-backup.tar.gz -C /
```

**Bind mounts** (for development):
```bash
# Mount local directory (mind UID/GID permissions)
docker run -d -v $(pwd)/local-data:/data alejandria-mcp:latest

# Fix permissions if needed (container runs as root by default)
docker run --rm -v $(pwd)/local-data:/data alpine chown -R 1000:1000 /data
```

#### Building Images

Using the provided `justfile`:

```bash
# Build both CLI and MCP images
just docker-build

# Build specific image
just docker-build-cli
just docker-build-mcp

# Build with custom tag
TAG=v1.2.3 just docker-buildx  # Multi-platform build (linux/amd64, linux/arm64)

# Clean up all images
just docker-clean
```

Manual builds:

```bash
# Build CLI image
docker build -f Dockerfile.cli -t alejandria-cli:latest .

# Build MCP image
docker build -f Dockerfile.mcp -t alejandria-mcp:latest .

# Multi-platform build (requires buildx)
docker buildx build --platform linux/amd64,linux/arm64 \
  -f Dockerfile.cli -t alejandria-cli:latest .
```

#### Image Size & Optimization

**Current sizes** (Debian/GNU libc base):
- CLI image: **~89MB** (includes ONNX Runtime for embeddings)
- MCP image: **~89MB** (includes ONNX Runtime for embeddings)

**Why larger than typical Rust images?**
- Alejandria uses `fastembed` for local ML embeddings
- `fastembed` requires ONNX Runtime (~50MB of ML inference libraries)
- ONNX Runtime doesn't support Alpine/musl builds (GNU libc required)

**Building without embeddings** (smaller images, ~15-20MB):

```bash
# Modify Dockerfile.cli and Dockerfile.mcp builder stage:
# Change: cargo build --release --bin alejandria
# To:     cargo build --release --bin alejandria --no-default-features

# Then rebuild
docker build -f Dockerfile.cli -t alejandria-cli:slim .
```

**Trade-offs**:
- ✅ Without embeddings: Smaller images, faster builds, pure keyword search (BM25/FTS5)
- ❌ Without embeddings: No vector similarity search capability

See **Production Deployment** section below for registry workflows and security hardening.

---

## Configuration

### Configuration File

**Default location**: `~/.config/alejandria/config.toml`

**Create default configuration**:

```bash
mkdir -p ~/.config/alejandria
cp config/default.toml ~/.config/alejandria/config.toml
```

**Minimal configuration**:

```toml
# Database path (supports ~ expansion)
db_path = "~/.local/share/alejandria/alejandria.db"

[search]
limit = 10        # Default search result limit
min_score = 0.3   # Minimum relevance score (0.0-1.0)

[decay]
auto_decay = true          # Auto-decay before search
prune_threshold = 0.1      # Prune memories with weight < 0.1
```

**Full configuration** (with embeddings and advanced settings):

```toml
db_path = "~/.local/share/alejandria/alejandria.db"

[search]
limit = 20
min_score = 0.5
bm25_weight = 0.3      # Keyword search weight (hybrid mode)
cosine_weight = 0.7    # Vector similarity weight (hybrid mode)

[decay]
auto_decay = true
prune_threshold = 0.1
auto_decay_hours = 24  # Auto-decay interval

[embeddings]
enabled = true
model = "intfloat/multilingual-e5-base"  # 768 dimensions
batch_size = 32

[mcp]
stdio = true           # Use stdio transport
log_requests = false   # Enable request/response logging
```

See `config/default.toml` for complete reference with detailed comments.

### Environment Variables

**Override configuration** (highest priority):

```bash
export ALEJANDRIA_DB_PATH="~/.local/share/alejandria/prod.db"
export ALEJANDRIA_SEARCH_LIMIT=20
export ALEJANDRIA_SEARCH_MIN_SCORE=0.5
export ALEJANDRIA_DECAY_AUTO_DECAY=true
export ALEJANDRIA_DECAY_PRUNE_THRESHOLD=0.1
export RUST_LOG=info  # Logging level (trace, debug, info, warn, error)
```

**Configuration priority** (highest to lowest):
1. Environment variables (`ALEJANDRIA_*`)
2. Configuration file (`~/.config/alejandria/config.toml`)
3. Built-in defaults

---

## Verification

### Basic Health Check

```bash
# 1. Verify installation
alejandria --version
# Expected: alejandria 0.1.0

# 2. Initialize database and check stats
alejandria stats
# Expected: JSON output with total_memories: 0

# 3. Store a test memory
alejandria store "Test memory for verification" \
  --topic test \
  --importance high

# 4. Recall the memory
alejandria recall "verification" --limit 5
# Expected: Should find the test memory with high score

# 5. Check health
alejandria stats --json
```

**Expected output** (healthy system):

```json
{
  "total_memories": 1,
  "active_memories": 1,
  "deleted_memories": 0,
  "total_size_mb": 0.1,
  "by_importance": {
    "Critical": 0,
    "High": 1,
    "Medium": 0,
    "Low": 0
  },
  "avg_weight": 1.0,
  "embeddings_enabled": true,
  "last_decay_at": null
}
```

### MCP Server Verification

```bash
# Start MCP server (will wait for stdin)
alejandria serve &
SERVER_PID=$!

# Send tools/list request
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | alejandria serve

# Expected: JSON response with 20 tools listed

# Kill server
kill $SERVER_PID
```

### Automated Verification Script

Use the provided verification script:

```bash
./scripts/verify-deployment.sh

# Expected output:
# ✓ Binary exists and is executable
# ✓ Configuration file is valid
# ✓ Database connectivity OK
# ✓ MCP server starts correctly
# ✓ Basic tool tests passed
# All checks passed!
```

See `scripts/verify-deployment.sh` for details.

---

## Upgrading

### From 0.0.x to 0.1.0

```bash
# 1. Backup database
cp ~/.local/share/alejandria/alejandria.db \
   ~/.local/share/alejandria/alejandria.db.backup

# 2. Pull latest code
cd alejandria
git pull origin main

# 3. Rebuild
cargo build --release --all-features

# 4. Install new binary
cp target/release/alejandria ~/.local/bin/

# 5. Verify upgrade
alejandria --version  # Should show 0.1.0

# 6. Test database migration (automatic on first run)
alejandria stats
```

**Schema migrations** are handled automatically on startup. The database schema version is tracked in the `icm_metadata` table.

### Rolling Back

If upgrade fails:

```bash
# 1. Restore binary
cp ~/.local/bin/alejandria.backup ~/.local/bin/alejandria

# 2. Restore database
cp ~/.local/share/alejandria/alejandria.db.backup \
   ~/.local/share/alejandria/alejandria.db

# 3. Verify rollback
alejandria --version
alejandria stats
```

---

## Troubleshooting

### Binary Not Found

```bash
# Check installation
which alejandria
# If empty, binary not in PATH

# Fix: Add to PATH
export PATH="$HOME/.local/bin:$PATH"
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Database Permission Errors

```bash
# Check database directory permissions
ls -la ~/.local/share/alejandria/

# Fix: Ensure directory exists and is writable
mkdir -p ~/.local/share/alejandria
chmod 755 ~/.local/share/alejandria
```

### Build Errors

**Error**: `linker 'cc' not found`

```bash
# Linux: Install build tools
sudo apt-get install build-essential

# macOS: Install Xcode Command Line Tools
xcode-select --install

# Windows: Install Visual Studio Build Tools
```

**Error**: `sqlite3.h not found`

```bash
# This shouldn't happen (SQLite is bundled), but if it does:
# Ubuntu/Debian
sudo apt-get install libsqlite3-dev
```

### MCP Server Exits Immediately

This is **expected behavior** when run without a client. MCP servers are stdio-based and exit when no client is connected.

**Verification**:
```bash
# Check that server initializes correctly
alejandria serve < /dev/null 2>&1
# Should see: "Starting Alejandria MCP server..."
```

To keep server running for testing:
```bash
# Pipe dummy input (Ctrl+C to stop)
alejandria serve < /dev/zero
```

### Claude Desktop Integration Not Working

1. **Check configuration path**:
   ```bash
   # macOS
   cat ~/Library/Application\ Support/Claude/claude_desktop_config.json
   
   # Verify "alejandria" section exists
   ```

2. **Check binary path** (must be absolute):
   ```bash
   # Test binary path from config
   /home/yourusername/.local/bin/alejandria --version
   ```

3. **Check logs** (macOS):
   ```bash
   # Claude Desktop logs
   tail -f ~/Library/Logs/Claude/mcp*.log
   ```

4. **Restart Claude Desktop** completely (quit and relaunch)

### Memory/Embedding Issues

**Issue**: "sqlite-vec not available" warning

```
Warning: Could not create vec_memories table (sqlite-vec not available): no such module: vec0
```

**Impact**: Vector similarity search disabled, BM25 keyword search still works.

**Solutions**:
1. Accept degraded functionality (keyword search is often sufficient)
2. Build without embeddings: `cargo build --release --no-default-features`
3. Report issue if embeddings feature was enabled during build

---

## Production Deployment

### Container Registry

#### Tagging Strategy

```bash
# Semantic versioning
docker tag alejandria-mcp:latest myregistry.com/alejandria-mcp:1.2.3
docker tag alejandria-mcp:latest myregistry.com/alejandria-mcp:1.2
docker tag alejandria-mcp:latest myregistry.com/alejandria-mcp:1
docker tag alejandria-mcp:latest myregistry.com/alejandria-mcp:latest

# Environment-specific tags
docker tag alejandria-mcp:latest myregistry.com/alejandria-mcp:staging
docker tag alejandria-mcp:latest myregistry.com/alejandria-mcp:production

# Git commit SHA (for traceability)
docker tag alejandria-mcp:latest myregistry.com/alejandria-mcp:sha-$(git rev-parse --short HEAD)
```

#### Pushing to Registry

**Docker Hub**:

```bash
# Login
docker login

# Tag and push
docker tag alejandria-mcp:latest yourusername/alejandria-mcp:1.0.0
docker push yourusername/alejandria-mcp:1.0.0
docker push yourusername/alejandria-mcp:latest
```

**GitHub Container Registry (ghcr.io)**:

```bash
# Create personal access token with write:packages scope
# Login
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# Tag and push
docker tag alejandria-mcp:latest ghcr.io/yourusername/alejandria-mcp:1.0.0
docker push ghcr.io/yourusername/alejandria-mcp:1.0.0
docker push ghcr.io/yourusername/alejandria-mcp:latest
```

**Multi-Platform Images**:

```bash
# Create and use buildx builder
docker buildx create --name alejandria-builder --use

# Build and push multi-platform images
docker buildx build --platform linux/amd64,linux/arm64 \
  -f Dockerfile.mcp \
  -t myregistry.com/alejandria-mcp:1.0.0 \
  --push .

# Verify manifest
docker buildx imagetools inspect myregistry.com/alejandria-mcp:1.0.0
```

### Production Configuration

**Environment variables** (`.env` file for docker-compose):

```bash
# Database
ALEJANDRIA_DB_PATH=/data/alejandria.db

# Search configuration
ALEJANDRIA_SEARCH_LIMIT=50
ALEJANDRIA_SEARCH_MIN_SCORE=0.5
ALEJANDRIA_SEARCH_BM25_WEIGHT=0.3
ALEJANDRIA_SEARCH_COSINE_WEIGHT=0.7

# Decay configuration
ALEJANDRIA_DECAY_AUTO_DECAY=true
ALEJANDRIA_DECAY_AUTO_DECAY_HOURS=24
ALEJANDRIA_DECAY_BASE_RATE=0.01
ALEJANDRIA_DECAY_PRUNE_THRESHOLD=0.1

# Embeddings
ALEJANDRIA_EMBEDDINGS_ENABLED=true
ALEJANDRIA_EMBEDDINGS_MODEL=intfloat/multilingual-e5-base
ALEJANDRIA_EMBEDDINGS_BATCH_SIZE=32

# Logging
RUST_LOG=info
RUST_BACKTRACE=1
```

**Resource limits** (docker-compose.yml):

```yaml
services:
  alejandria-mcp:
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 1G
        reservations:
          cpus: '0.25'
          memory: 256M
```

### Security Hardening

**Run as non-root user** (add to Dockerfile):

```dockerfile
# Create non-root user
RUN groupadd -r alejandria && useradd -r -g alejandria alejandria
RUN chown -R alejandria:alejandria /data
USER alejandria
```

**Read-only root filesystem**:

```bash
docker run -d \
  --read-only \
  --tmpfs /tmp \
  -v alejandria-data:/data \
  alejandria-mcp:latest
```

**Security scanning**:

```bash
# Scan images for vulnerabilities (using Trivy)
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image alejandria-mcp:latest

# Scan with Docker Scout (if available)
docker scout cves alejandria-mcp:latest
```

### Monitoring & Health Checks

**Health check configuration**:

```yaml
healthcheck:
  test: ["CMD", "alejandria", "stats", "--json"]
  interval: 30s
  timeout: 5s
  retries: 3
  start_period: 10s
```

**Metrics collection**:

```bash
# Periodic stats export (cron or systemd timer)
docker exec alejandria-mcp alejandria stats --json > /var/metrics/alejandria-stats.json
```

### Backup & Disaster Recovery

**Automated backup script**:

```bash
#!/bin/bash
# backup-alejandria.sh

BACKUP_DIR="/backups/alejandria"
DATE=$(date +%Y%m%d_%H%M%S)
VOLUME="alejandria-prod-data"

mkdir -p "$BACKUP_DIR"

# Backup volume
docker run --rm \
  -v ${VOLUME}:/data:ro \
  -v ${BACKUP_DIR}:/backup \
  alpine tar czf /backup/alejandria-${DATE}.tar.gz /data

# Keep last 30 days
find "$BACKUP_DIR" -name "alejandria-*.tar.gz" -mtime +30 -delete

echo "Backup completed: alejandria-${DATE}.tar.gz"
```

Schedule with cron:
```cron
# Daily at 2 AM
0 2 * * * /opt/scripts/backup-alejandria.sh
```

**Restore from backup**:

```bash
# 1. Stop running container
docker-compose down

# 2. Create new volume
docker volume create alejandria-restore

# 3. Restore data
docker run --rm \
  -v alejandria-restore:/data \
  -v /backups/alejandria:/backup \
  alpine tar xzf /backup/alejandria-20260329_020000.tar.gz -C /

# 4. Start with restored volume
docker-compose up -d
```

---

## Support

For deployment support:

- **Issues**: https://github.com/yourusername/alejandria/issues
- **Discussions**: https://github.com/yourusername/alejandria/discussions
- **Documentation**: https://github.com/yourusername/alejandria/tree/main/docs

For security vulnerabilities, please see `SECURITY.md`.

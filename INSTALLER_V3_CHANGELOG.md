# Alejandría MCP Installer v3.0 - Changelog

## 🎯 Overview

This document describes the changes from v2.1 to v3.0 of the Alejandría MCP installer.

**Major Change:** v3.0 introduces **multi-mode support**, allowing users to choose between Local (stdio), Remote Client (SSE), or Server deployment modes.

---

## 🚀 New Features

### 1. Multi-Mode Architecture

**v2.1 (Old):**
- ❌ Only supported Local mode (stdio transport)
- ❌ Hard-coded configuration
- ❌ No server deployment option

**v3.0 (New):**
- ✅ **Mode 1:** Local (stdio) - Same as v2.1
- ✅ **Mode 2:** Remote Client (MCP SSE) - Connect to server
- ✅ **Mode 3:** Server Installation - Deploy as team server

### 2. Interactive Mode Selection

**v3.0 adds:**
```
¿Cómo quieres usar Alejandría?

  1) 🏠 Local (stdio)           [DEFAULT - RECOMENDADO]
  2) 🌐 Cliente Remoto (MCP SSE)
  3) 🖥️  Servidor MCP (instalar servidor)

Opción [1]:
```

### 3. Automatic CA Certificate Download

**New in v3.0:**

When selecting Remote Client mode, the installer automatically attempts to download the server's CA certificate using two methods:

1. **SSH method** (preferred):
   ```bash
   ssh user@server "docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt"
   ```

2. **HTTP endpoint** (fallback):
   ```bash
   curl https://server/alejandria/ca-cert
   ```

**Benefits:**
- ✅ Eliminates manual certificate installation
- ✅ Ensures TLS verification works out-of-the-box
- ✅ Reduces user error

### 4. Server Installation Support

**New in v3.0:**

Complete server deployment automation including:

- ✅ Systemd service creation
- ✅ Automatic API key generation
- ✅ Configurable database location
- ✅ Caddy reverse proxy integration
- ✅ TLS certificate management
- ✅ Directory structure creation
- ✅ Permission management
- ✅ Service health checks

**Files created:**
```
/etc/alejandria/
├── config.toml
└── api.env

/etc/systemd/system/
└── alejandria.service

/var/lib/alejandria/
└── alejandria.db

/var/log/alejandria/
├── alejandria.log
└── error.log
```

### 5. Enhanced Configuration Functions

**v2.1:**
- Hard-coded configuration blocks
- Manual JSON manipulation
- No validation

**v3.0:**
- Modular configuration functions:
  - `configure_local_mode()`
  - `configure_remote_mode()`
  - `configure_mcp_client_opencode()`
  - `configure_mcp_client_claude_cli()`
  - `configure_mcp_client_claude_desktop()`
  - `configure_mcp_client_vscode()`
  - `configure_mcp_client_copilot()`
- Mode-aware (local vs remote)
- Dynamic JSON generation based on mode

### 6. Input Validation

**New in v3.0:**

- `validate_url()` - Ensures URLs start with http:// or https://
- `validate_port()` - Validates port numbers (1-65535)
- Non-empty API key validation
- File path validation
- Server reachability testing

### 7. Improved Error Handling

**v3.0 enhancements:**

- Clear error messages with colors
- Actionable troubleshooting steps
- Graceful fallbacks (e.g., CA cert download)
- Service status verification
- Connection testing before finalizing

### 8. Progress Indicators

**v2.1:**
```
[1/8] Creating directories...
[2/8] Creating Alejandría configuration...
```

**v3.0:**
```
[1/7] Validating binary...
✓ Binary found: /home/user/.local/bin/alejandria

[2/7] Downloading CA certificate...
  Trying SSH method...
  ✓ CA certificate downloaded via SSH

[3/7] Configuring MCP clients (Remote mode)...
  ✓ OpenCode configured
  ✓ Claude Code CLI configured
  ...
```

### 9. Comprehensive Help System

**New in v3.0:**

```bash
./install-mcp-v3.sh --help
```

Displays:
- Usage syntax
- Available options
- Mode descriptions
- Examples
- Client list
- Documentation references

### 10. TLS Configuration Wizard

**New in v3.0 (Server mode only):**

```
Configuración de TLS:
  1) Sin TLS (HTTP)                    [NO RECOMENDADO]
  2) TLS con reverse proxy (Caddy)     [RECOMENDADO]
  3) Ya tengo reverse proxy            [Manual]

Opción [2]:
```

Features:
- Automatic Caddy installation (if missing)
- Caddyfile generation
- Self-signed certificate creation
- Service reload
- Configuration validation

---

## 🔄 Modified Features

### 1. Binary Validation

**v2.1:**
```bash
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: Binary not found"
    exit 1
fi
```

**v3.0:**
```bash
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: Binary not found at $BINARY_PATH"
    read -p "Download from GitHub releases? (y/N): " DOWNLOAD_BINARY
    # Placeholder for future GitHub download feature
    exit 1
fi
```

### 2. Configuration File Locations

**v2.1:**
- Only configured for local mode
- Hard-coded paths

**v3.0:**
- **Local mode:**
  - Config: `~/.config/alejandria/config.toml`
  - Database: `~/.local/share/alejandria/alejandria.db`
  
- **Server mode:**
  - Config: `/etc/alejandria/config.toml`
  - Database: `/var/lib/alejandria/alejandria.db` (configurable)
  - API Key: `/etc/alejandria/api.env`
  - Logs: `/var/log/alejandria/`

### 3. MCP Client Configuration

**v2.1:**
- Only `stdio` transport
- Only `command` and `args` fields

**v3.0 (Local mode):**
```json
{
  "command": ["alejandria", "serve"],
  "environment": {"ALEJANDRIA_CONFIG": "..."},
  "enabled": true,
  "type": "local"
}
```

**v3.0 (Remote mode):**
```json
{
  "url": "https://server/alejandria",
  "apiKey": "...",
  "transport": "sse",
  "tlsCert": "~/.alejandria/ca-cert.pem",
  "enabled": true,
  "type": "remote"
}
```

### 4. Summary Output

**v2.1:**
- Basic installation summary
- Restart instructions
- Test script location

**v3.0:**
- **Mode-specific summaries** (different for Local/Remote/Server)
- **Security information** (TLS status, encryption details)
- **Management commands** (for Server mode)
- **Next steps** (user-specific and admin-specific)
- **Troubleshooting section**
- **Colored, formatted output**

---

## 🗑️ Removed Features

None. v3.0 is **fully backward-compatible** with v2.1 for Local mode.

Running v3.0 and selecting option 1 produces the same result as v2.1.

---

## 📊 File Structure Comparison

### v2.1

```
scripts/
└── install-mcp.sh          # Single-mode installer
```

### v3.0

```
scripts/
├── install-mcp.sh          # Legacy (v2.1) - DEPRECATED
└── install-mcp-v3.sh       # New multi-mode installer

# New documentation
INSTALLER_V3_GUIDE.md       # Comprehensive user guide
INSTALLER_V3_CHANGELOG.md   # This file
```

---

## 🔐 Security Improvements

### 1. API Key Protection

**v2.1:**
- Hard-coded API key in script
- No secure storage

**v3.0:**
- Interactive API key input (not echoed)
- Stored in config files with proper permissions
- **Server mode:** API key stored in `/etc/alejandria/api.env` (chmod 600)

### 2. TLS by Default

**v2.1:**
- No TLS support
- HTTP only (if remote mode existed)

**v3.0:**
- HTTPS required for Remote Client mode
- CA certificate verification
- Automatic certificate download
- Caddy integration for Server mode

### 3. Secure Defaults

**v3.0:**
- Default mode: Local (most secure - no network exposure)
- Default TLS option: Caddy with auto-HTTPS
- File permissions: 600 for sensitive files
- Service runs with minimal privileges

---

## 🐛 Bug Fixes

### 1. jq Dependency Handling

**v2.1:**
- Silently failed if jq not installed
- Left partial configurations

**v3.0:**
- Detects jq presence
- Falls back to manual JSON creation
- Warns user but continues installation

### 2. Backup File Naming

**v2.1:**
```bash
cp file file.backup
# Could overwrite existing backups
```

**v3.0:**
```bash
cp file file.backup-$(date +%Y%m%d-%H%M%S)
# Unique timestamp prevents overwrites
```

### 3. Configuration Validation

**v2.1:**
- No validation of config file syntax
- No check if clients were configured successfully

**v3.0:**
- URL format validation
- Port number validation
- Connection testing (Remote mode)
- Service status verification (Server mode)

---

## 📈 Performance Improvements

### 1. Parallel Configuration

**v3.0:**
- All 5 MCP clients configured in sequence but efficiently
- Reduced redundant operations
- Reusable configuration functions

### 2. Conditional Execution

**v3.0:**
- Only runs mode-specific steps
- No unnecessary file operations
- Skips unavailable clients gracefully

---

## 🔮 Future Enhancements (Roadmap)

### Planned for v3.1

- [ ] GitHub release download integration
- [ ] Automatic binary updates
- [ ] Migration wizard (Local ↔ Remote)
- [ ] Database export/import for mode switching
- [ ] Health check dashboard
- [ ] Automated TLS certificate renewal

### Planned for v3.2

- [ ] Docker deployment option
- [ ] Kubernetes manifests
- [ ] Multi-region server support
- [ ] Load balancer configuration
- [ ] Monitoring integration (Prometheus/Grafana)

### Planned for v4.0

- [ ] WebUI for server management
- [ ] User management (multi-key support)
- [ ] RBAC (Role-Based Access Control)
- [ ] Audit logging
- [ ] SSO integration (SAML, OAuth)

---

## 📝 Migration Guide: v2.1 → v3.0

### For Existing Local Users

**No action required.** Your existing installation continues to work.

To upgrade to v3.0:

1. Backup your configs:
   ```bash
   cp -r ~/.config/opencode ~/.config/opencode.backup
   cp ~/.claude.json ~/.claude.json.backup
   # etc.
   ```

2. Run new installer:
   ```bash
   ./scripts/install-mcp-v3.sh
   ```

3. Select option `1` (Local)

4. Verify configuration:
   ```bash
   test-alejandria
   ```

### For Teams (New Server Deployment)

1. **Choose a server machine** (dedicated VM/container recommended)

2. **Install Alejandría binary** on server

3. **Run installer as root:**
   ```bash
   sudo ./scripts/install-mcp-v3.sh
   ```

4. **Select option `3`** (Server)

5. **Configure settings:**
   - Listen address: `0.0.0.0:8080`
   - Auto-generate API key: `Yes`
   - Database path: Default
   - TLS: Option 2 (Caddy)

6. **Share credentials with team:**
   - Server URL
   - API key (use secure method!)
   - CA certificate (if needed)

7. **Team members run:**
   ```bash
   ./scripts/install-mcp-v3.sh
   # Select option 2 (Remote Client)
   ```

---

## 🧪 Testing Changes

### Test Matrix

| Scenario | v2.1 | v3.0 Mode 1 | v3.0 Mode 2 | v3.0 Mode 3 |
|----------|------|-------------|-------------|-------------|
| Fresh install | ✅ | ✅ | N/A | N/A |
| Upgrade install | ✅ | ✅ | N/A | N/A |
| OpenCode | ✅ | ✅ | ✅ | N/A |
| Claude CLI | ✅ | ✅ | ✅ | N/A |
| Claude Desktop | ✅ | ✅ | ✅ | N/A |
| VSCode | ✅ | ✅ | ✅ | N/A |
| Copilot | ✅ | ✅ | ✅ | N/A |
| No jq | ⚠️ | ✅ | ✅ | N/A |
| TLS | N/A | N/A | ✅ | ✅ |
| API Key | N/A | N/A | ✅ | ✅ |
| Systemd | N/A | N/A | N/A | ✅ |

### Test Commands

```bash
# Test help
./scripts/install-mcp-v3.sh --help

# Test Mode 1 (Local)
./scripts/install-mcp-v3.sh
# Select option 1
test-alejandria

# Test Mode 2 (Remote Client) - requires server
./scripts/install-mcp-v3.sh
# Select option 2
# Provide test server URL and API key

# Test Mode 3 (Server) - requires root
sudo ./scripts/install-mcp-v3.sh
# Select option 3
sudo systemctl status alejandria
```

---

## 📚 Related Documents

- **User Guide:** `INSTALLER_V3_GUIDE.md`
- **HTTPS Update Plan:** `INSTALADOR_ACTUALIZACION_HTTPS.md`
- **API Key Management:** `API_KEY_MANAGEMENT.md`
- **TLS Implementation:** `TLS_IMPLEMENTADO_RESUMEN.md`
- **Security Review:** `SECURITY_REMEDIATION_PLAN.md`

---

## 🙏 Acknowledgments

- Original v2.1 installer by AppSec Team
- HTTPS integration requirements from Security Review
- Multi-mode architecture inspired by industry best practices

---

**Version:** 3.0  
**Release Date:** 2026-04-11  
**Breaking Changes:** None (backward-compatible)  
**Deprecations:** v2.1 installer (install-mcp.sh) is now legacy  
**Maintainer:** AppSec Team - Veritran

# Alejandría MCP Installer v3.0 - User Guide

## 📖 Overview

The Alejandría MCP Installer v3.0 is a comprehensive, multi-mode installer that supports three deployment scenarios:

1. **🏠 Local (stdio)** - Single-user, local execution
2. **🌐 Remote Client (MCP SSE)** - Connect to shared team server
3. **🖥️ Server Installation** - Deploy Alejandría as a server for teams

---

## 🚀 Quick Start

### Installation

```bash
cd ~/repos/AppSec/Alejandria
./scripts/install-mcp-v3.sh
```

The installer will present an interactive menu to select your deployment mode.

### Help

```bash
./scripts/install-mcp-v3.sh --help
```

---

## 📋 Mode 1: Local (stdio)

**Use case:** Individual developers who want private memory storage without network dependencies.

### Features

- ✅ Binary runs locally on your machine
- ✅ Private database in `~/.local/share/alejandria/`
- ✅ No server or network required
- ✅ Fastest performance (no network latency)
- ✅ Complete privacy (data never leaves your machine)

### Installation Steps

1. Run the installer:
   ```bash
   ./scripts/install-mcp-v3.sh
   ```

2. Select option `1` (Local)

3. The installer will:
   - Validate the Alejandría binary exists
   - Create configuration in `~/.config/alejandria/config.toml`
   - Configure all 5 MCP clients
   - Create a test script

4. Restart your MCP clients:
   ```bash
   # OpenCode
   pkill -9 opencode && opencode
   
   # Claude Code CLI
   /exit  # then reopen
   
   # Claude Desktop
   # Close and reopen the application
   
   # VSCode
   # Ctrl+Shift+P → "Developer: Reload Window"
   ```

5. Test the installation:
   ```bash
   test-alejandria
   ```

### Example Configuration (OpenCode)

```json
{
  "mcp": {
    "alejandria": {
      "command": ["/home/user/.local/bin/alejandria", "serve"],
      "environment": {
        "ALEJANDRIA_CONFIG": "/home/user/.config/alejandria/config.toml"
      },
      "enabled": true,
      "type": "local"
    }
  }
}
```

### Troubleshooting

**Problem:** Binary not found

**Solution:**
```bash
# Download from GitHub releases or build from source
# Then specify path:
./scripts/install-mcp-v3.sh --binary /path/to/alejandria
```

**Problem:** MCP clients don't see Alejandría

**Solution:** Restart ALL clients (they only load MCP config at startup)

---

## 🌐 Mode 2: Remote Client (MCP SSE)

**Use case:** Teams sharing a centralized Alejandría server for collaborative memory.

### Features

- ✅ Connect to shared team server
- ✅ Shared memory across team members
- ✅ Automatic TLS certificate download
- ✅ Encrypted communication (HTTPS/TLS)
- ✅ API key authentication

### Prerequisites

- Alejandría server running (see Mode 3)
- Server URL
- Valid API key from server administrator

### Installation Steps

1. Run the installer:
   ```bash
   ./scripts/install-mcp-v3.sh
   ```

2. Select option `2` (Cliente Remoto)

3. Provide server information:
   ```
   URL del servidor MCP [https://ar-appsec-01.veritran.net/alejandria]: 
   https://your-server.com/alejandria
   
   API Key: ******************************
   
   ¿Descargar CA cert desde servidor? (S/n): S
   ```

4. The installer will:
   - Download CA certificate (if available)
   - Test connection to server
   - Configure all 5 MCP clients with SSE transport
   - Store API key securely in configs

5. Restart your MCP clients (same as Mode 1)

### Example Configuration (OpenCode)

```json
{
  "mcp": {
    "alejandria": {
      "url": "https://ar-appsec-01.veritran.net/alejandria",
      "apiKey": "alejandria-abc123...",
      "transport": "sse",
      "tlsCert": "/home/user/.alejandria/ca-cert.pem",
      "enabled": true,
      "type": "remote"
    }
  }
}
```

### CA Certificate Download

The installer attempts to download the CA certificate using two methods:

1. **SSH Method** (preferred):
   ```bash
   ssh user@server "docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt"
   ```

2. **HTTP Endpoint** (fallback):
   ```bash
   curl https://server/alejandria/ca-cert
   ```

If automatic download fails, manually copy the certificate to:
```bash
~/.alejandria/ca-cert.pem
```

### Troubleshooting

**Problem:** TLS connection fails

**Solution:**
```bash
# 1. Verify CA cert exists
ls -lh ~/.alejandria/ca-cert.pem

# 2. Test connection manually
curl -k -H 'X-API-Key: YOUR_KEY' \
     https://your-server.com/alejandria/health

# Expected: {"status":"healthy"}

# 3. Re-download CA cert
./scripts/install-mcp-v3.sh  # Select option 2 again
```

**Problem:** API key rejected

**Solution:** Contact your server administrator to verify:
- API key is active
- API key matches server configuration
- Server is running and accessible

**Problem:** Connection timeout

**Solution:**
```bash
# Check if server is reachable
ping ar-appsec-01.veritran.net

# Check if port is open
nc -zv ar-appsec-01.veritran.net 443

# Check server logs (ask admin)
ssh user@server "sudo journalctl -u alejandria -n 50"
```

---

## 🖥️ Mode 3: Server Installation

**Use case:** System administrators deploying Alejandría server for team access.

### Features

- ✅ Systemd service integration
- ✅ Automatic API key generation
- ✅ Optional Caddy reverse proxy with TLS
- ✅ Configurable database location
- ✅ Multi-user support (shared memory)

### Prerequisites

- Root/sudo access
- Alejandría binary installed
- (Optional) Caddy for TLS reverse proxy

### Installation Steps

1. Run installer with sudo:
   ```bash
   sudo ./scripts/install-mcp-v3.sh
   ```

2. Select option `3` (Servidor MCP)

3. Configure server settings:
   ```
   IP/Puerto de escucha [0.0.0.0:8080]: 
   10.233.0.14:8080
   
   Generar API key automática? (S/n): S
   ✓ API Key generated: alejandria-a3f8b2c1...
   
   Base de datos [/var/lib/alejandria/alejandria.db]: 
   /var/lib/alejandria/alejandria.db
   ```

4. Configure TLS:
   ```
   Configuración de TLS:
     1) Sin TLS (HTTP)                    [NO RECOMENDADO]
     2) TLS con reverse proxy (Caddy)     [RECOMENDADO]
     3) Ya tengo reverse proxy            [Manual]
   
   Opción [2]: 2
   
   Hostname para TLS [ar-appsec-01.veritran.net]: 
   ar-appsec-01.veritran.net
   ```

5. The installer will:
   - Create systemd service
   - Generate configuration files
   - Install/configure Caddy (if selected)
   - Start the Alejandría service
   - Display API key for distribution

### Files Created

```
/etc/alejandria/
├── config.toml          # Server configuration
└── api.env              # API key (chmod 600)

/etc/systemd/system/
└── alejandria.service   # Systemd service

/var/lib/alejandria/
└── alejandria.db        # Database file

/var/log/alejandria/
├── alejandria.log       # stdout logs
└── error.log            # stderr logs

/etc/caddy/
└── Caddyfile            # Caddy config (if TLS enabled)
```

### Management Commands

```bash
# Start service
sudo systemctl start alejandria

# Stop service
sudo systemctl stop alejandria

# Restart service
sudo systemctl restart alejandria

# Check status
sudo systemctl status alejandria

# View logs (live)
sudo journalctl -u alejandria -f

# View last 100 lines
sudo journalctl -u alejandria -n 100
```

### Sharing Access with Users

1. **Share the API Key** (use secure method!):
   ```
   API Key: alejandria-a3f8b2c1d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9
   ```

   **✅ Secure methods:**
   - 1Password/Bitwarden shared vault
   - Signal/Telegram encrypted message
   - In-person (show screen)

   **❌ Insecure methods (NEVER):**
   - Email
   - Slack/Teams plain text
   - SMS
   - Git commit

2. **Share the Server URL**:
   ```
   https://ar-appsec-01.veritran.net/alejandria
   ```

3. **Extract CA Certificate** (if using Caddy):
   ```bash
   # Method 1: From Caddy container
   sudo docker exec veriscan-proxy \
       cat /data/caddy/pki/authorities/local/root.crt
   
   # Method 2: From Caddy data directory
   sudo cat /var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt
   ```

4. **Instruct users**:
   - Run `./install-mcp-v3.sh`
   - Select option `2` (Remote Client)
   - Provide server URL and API key

### TLS Configuration Details

#### Option 1: No TLS (NOT RECOMMENDED)

- ⚠️ Data transmitted in plain text
- ⚠️ API keys visible to network sniffers
- ⚠️ Vulnerable to MITM attacks
- Only use in trusted, isolated networks

#### Option 2: Caddy Reverse Proxy (RECOMMENDED)

**Auto-configuration:**
- Installer adds route to Caddyfile
- Caddy generates self-signed certificates
- Automatic HTTPS with TLS 1.3

**Manual Caddyfile example:**
```caddyfile
https://ar-appsec-01.veritran.net {
    route /alejandria/* {
        uri strip_prefix /alejandria
        reverse_proxy 10.233.0.14:8080
    }
    
    tls internal {
        on_demand
    }
}
```

**Reload Caddy:**
```bash
sudo systemctl reload caddy
# or
sudo systemctl restart caddy
```

#### Option 3: Manual Reverse Proxy

If you already have nginx, Apache, or another proxy:

**Nginx example:**
```nginx
location /alejandria/ {
    proxy_pass http://10.233.0.14:8080/;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection 'upgrade';
    proxy_set_header Host $host;
    proxy_cache_bypass $http_upgrade;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
}
```

**Apache example:**
```apache
<Location /alejandria>
    ProxyPass http://10.233.0.14:8080/
    ProxyPassReverse http://10.233.0.14:8080/
    ProxyPreserveHost On
</Location>
```

### Troubleshooting

**Problem:** Service fails to start

**Solution:**
```bash
# Check logs
sudo journalctl -u alejandria -n 50

# Common issues:
# - Binary not found: Check BINARY_PATH in install-mcp-v3.sh
# - Permission denied: Check /var/lib/alejandria ownership
# - Port in use: Change listen_addr in /etc/alejandria/config.toml
```

**Problem:** Caddy fails to start

**Solution:**
```bash
# Check Caddy logs
sudo journalctl -u caddy -n 50

# Validate Caddyfile
sudo caddy validate --config /etc/caddy/Caddyfile

# Common issues:
# - Syntax error in Caddyfile
# - Port 443 already in use
# - Missing permissions
```

**Problem:** Users can't connect

**Solution:**
```bash
# 1. Verify service is running
sudo systemctl status alejandria

# 2. Check if port is open
sudo netstat -tlnp | grep 8080

# 3. Test local connection
curl -H 'X-API-Key: YOUR_KEY' http://localhost:8080/health

# 4. Test external connection
curl -k -H 'X-API-Key: YOUR_KEY' \
     https://ar-appsec-01.veritran.net/alejandria/health

# 5. Check firewall
sudo ufw status
sudo iptables -L -n | grep 8080
```

---

## 🔄 Migrating Between Modes

### Local → Remote Client

1. Backup local database:
   ```bash
   cp ~/.local/share/alejandria/alejandria.db \
      ~/.local/share/alejandria/alejandria.db.backup
   ```

2. Re-run installer:
   ```bash
   ./scripts/install-mcp-v3.sh
   ```

3. Select option `2` (Remote Client)

4. Your local database remains intact (for manual migration if needed)

### Remote Client → Local

1. Re-run installer:
   ```bash
   ./scripts/install-mcp-v3.sh
   ```

2. Select option `1` (Local)

3. You'll get a fresh local database (memories from server are not transferred)

### Local → Server

This requires root access and is a significant architectural change. Recommended approach:

1. Install server on a dedicated machine (Mode 3)
2. Keep local installation for personal use
3. Optionally export/import memories manually (future feature)

---

## 📊 Comparison Table

| Feature | Local | Remote Client | Server |
|---------|-------|---------------|--------|
| **Network Required** | ❌ | ✅ | ✅ |
| **Shared Memory** | ❌ | ✅ | N/A (provides to clients) |
| **TLS/HTTPS** | N/A | ✅ | ✅ (recommended) |
| **API Key Auth** | ❌ | ✅ | ✅ |
| **Root Access** | ❌ | ❌ | ✅ |
| **Performance** | 🚀 Fastest | ⚡ Fast (network latency) | ⚡ Fast |
| **Privacy** | 🔒 Complete | 🤝 Shared with team | 🤝 Shared |
| **Setup Complexity** | ⭐ Easy | ⭐⭐ Moderate | ⭐⭐⭐ Advanced |
| **Ideal For** | Individual devs | Small teams (2-20) | Teams/organizations |

---

## 🔐 Security Best Practices

### API Key Management

1. **Never commit API keys to Git**
2. **Use secure sharing methods** (1Password, Signal)
3. **Rotate keys every 90 days**
4. **Revoke keys when team members leave**
5. **Use different keys per environment** (dev/staging/prod)

### TLS Configuration

1. **Always use HTTPS in production**
2. **Install CA certificates properly**
3. **Don't disable certificate verification**
4. **Keep Caddy/nginx updated**
5. **Monitor certificate expiration**

### Server Hardening

1. **Run with minimal permissions**
2. **Use firewall to restrict access**
3. **Enable audit logging**
4. **Regular security updates**
5. **Monitor server logs**

---

## 📚 Additional Resources

- **API Key Management:** `API_KEY_MANAGEMENT.md`
- **TLS Setup Guide:** `TLS_IMPLEMENTADO_RESUMEN.md`
- **Security Review:** `SECURITY_REMEDIATION_PLAN.md`
- **Changelog:** `INSTALLER_V3_CHANGELOG.md`

---

## 🆘 Getting Help

### Issues

Report bugs or request features:
```
https://github.com/your-org/alejandria/issues
```

### Logs

When reporting issues, include:

**For Local mode:**
```bash
# Binary version
alejandria --version

# Config file
cat ~/.config/alejandria/config.toml

# Test output
test-alejandria 2>&1 | tee test-output.log
```

**For Remote Client mode:**
```bash
# Connection test
curl -v -k -H 'X-API-Key: YOUR_KEY' \
     https://your-server/alejandria/health

# MCP client logs (OpenCode example)
tail -100 ~/.local/state/opencode/logs/mcp-*.log
```

**For Server mode:**
```bash
# Service status
sudo systemctl status alejandria

# Logs
sudo journalctl -u alejandria -n 100 --no-pager

# Configuration
sudo cat /etc/alejandria/config.toml
```

---

**Version:** 3.0  
**Last Updated:** 2026-04-11  
**Maintainer:** AppSec Team - Veritran

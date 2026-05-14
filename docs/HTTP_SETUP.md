# Alejandria HTTP Transport Setup Guide

This guide covers deploying Alejandria with HTTP/SSE transport for multi-team remote access.

## Table of Contents
1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Server Setup](#server-setup)
4. [API Key Generation](#api-key-generation)
5. [Multi-Team Isolation](#multi-team-isolation)
6. [TLS Configuration](#tls-configuration)
7. [Nginx Configuration](#nginx-configuration)
8. [Firewall Rules](#firewall-rules)
9. [Automated Deployment](#automated-deployment)
10. [Monitoring & Troubleshooting](#monitoring--troubleshooting)

## Overview

Alejandria HTTP transport enables:
- **Remote access** via JSON-RPC over HTTP
- **Real-time notifications** via Server-Sent Events (SSE)
- **Multi-team isolation** with per-team API keys and databases
- **Enterprise security** with TLS, rate limiting, and authentication

**Architecture:**
```
[Client] --> [Nginx (TLS + Rate Limit)] --> [Alejandria HTTP Server] --> [SQLCipher Database]
```

## Prerequisites

- **Server**: Linux (RHEL/AlmaLinux/Ubuntu) with systemd
- **Rust toolchain**: 1.70+ with cargo
- **Nginx**: 1.18+ for reverse proxy
- **OpenSSL**: For TLS certificates
- **SQLCipher**: For database encryption (bundled with Alejandria)
- **Firewall access**: Ports 80 (HTTP redirect) and 443 (HTTPS)

## Server Setup

### 1. Build Alejandria with HTTP Transport

```bash
# On remote server (your-server.example.com)
cd /opt/alejandria-build
source ~/.cargo/env

# Build with HTTP transport and encryption features
cargo build --release --features http-transport,encryption

# Verify binary
ls -lh target/release/alejandria
```

### 2. Install Binary

```bash
sudo cp target/release/alejandria /usr/local/bin/
sudo chmod +x /usr/local/bin/alejandria
sudo chown root:root /usr/local/bin/alejandria
```

### 3. Create User and Directories

```bash
# Create service user (no login shell)
sudo useradd -r -s /bin/false alejandria

# Create directories
sudo mkdir -p /var/lib/alejandria /etc/alejandria
sudo chown alejandria:alejandria /var/lib/alejandria
sudo chmod 750 /var/lib/alejandria
```

## API Key Generation

### Generate API Key

```bash
# Generate 256-bit random key (hex encoded)
API_KEY=$(openssl rand -hex 32)
echo "Raw API Key: $API_KEY"

# Compute SHA-256 hash (this is what the server compares)
API_KEY_HASH=$(echo -n "$API_KEY" | sha256sum | awk '{print $1}')
echo "SHA-256 Hash: $API_KEY_HASH"

# Save to file (first line: raw key, second line: hash)
echo "$API_KEY" | sudo tee /etc/alejandria/api_keys.txt > /dev/null
echo "$API_KEY_HASH" | sudo tee -a /etc/alejandria/api_keys.txt > /dev/null
sudo chmod 600 /etc/alejandria/api_keys.txt
sudo chown alejandria:alejandria /etc/alejandria/api_keys.txt
```

**IMPORTANT**: Save the raw API key (`$API_KEY`) - this is what clients send in the `X-API-Key` header. The hash is stored server-side for constant-time comparison.

### Generate Instance ID

```bash
uuidgen | sudo tee /etc/alejandria/instance_id.txt
sudo chmod 600 /etc/alejandria/instance_id.txt
sudo chown alejandria:alejandria /etc/alejandria/instance_id.txt
```

## Multi-Team Isolation

### Strategy 1: Multiple Instances (Recommended)

Run separate Alejandria instances per team with isolated databases:

**Team A (Security Team):**
```bash
# Port 8081, separate database
ALEJANDRIA_DB_PATH=/var/lib/alejandria/security-team.db \
ALEJANDRIA_API_KEY=$(cat /etc/alejandria/security-team-key.txt) \
ALEJANDRIA_INSTANCE_ID=$(cat /etc/alejandria/security-team-id.txt) \
alejandria serve --http --bind 127.0.0.1:8081
```

**Team B (Dev Team):**
```bash
# Port 8082, separate database
ALEJANDRIA_DB_PATH=/var/lib/alejandria/dev-team.db \
ALEJANDRIA_API_KEY=$(cat /etc/alejandria/dev-team-key.txt) \
ALEJANDRIA_INSTANCE_ID=$(cat /etc/alejandria/dev-team-id.txt) \
alejandria serve --http --bind 127.0.0.1:8082
```

Create separate systemd services: `alejandria-security-team.service`, `alejandria-dev-team.service`.

### Strategy 2: Session-Based Isolation

Use a single instance with session-based isolation (API key → session_id → isolated data):
- Each team gets a unique API key
- SSE events are filtered by session_id
- Database queries filter by team identifier (requires schema changes)

**Recommendation**: Use Strategy 1 (multiple instances) for strongest isolation.

## TLS Configuration

### Let's Encrypt Setup

```bash
# Install certbot
sudo yum install certbot python3-certbot-nginx  # RHEL/AlmaLinux
# sudo apt install certbot python3-certbot-nginx  # Ubuntu

# Generate certificate for team subdomain
sudo certbot certonly --nginx -d security-team.alejandria.example.com

# Certificate files will be at:
# /etc/letsencrypt/live/security-team.alejandria.example.com/fullchain.pem
# /etc/letsencrypt/live/security-team.alejandria.example.com/privkey.pem

# Auto-renewal (certbot installs cron job automatically)
sudo certbot renew --dry-run
```

### Manual Certificate Setup

If using corporate CA or self-signed certificates:

```bash
# Generate private key
sudo openssl genrsa -out /etc/nginx/ssl/alejandria.key 4096

# Generate CSR
sudo openssl req -new -key /etc/nginx/ssl/alejandria.key \
    -out /etc/nginx/ssl/alejandria.csr

# Submit CSR to your CA and install signed certificate
sudo cp certificate.crt /etc/nginx/ssl/alejandria.crt

# Set permissions
sudo chmod 600 /etc/nginx/ssl/alejandria.key
sudo chmod 644 /etc/nginx/ssl/alejandria.crt
```

## Nginx Configuration

### Install Configuration Files

```bash
# Copy main config
sudo cp deployment/nginx-alejandria.conf /etc/nginx/sites-available/alejandria.conf

# Copy reusable snippets
sudo mkdir -p /etc/nginx/snippets
sudo cp deployment/nginx-snippets-alejandria-locations.conf /etc/nginx/snippets/alejandria-locations.conf

# Enable site
sudo ln -s /etc/nginx/sites-available/alejandria.conf /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Reload Nginx
sudo systemctl reload nginx
```

### Key Configuration Points

1. **Rate Limiting**: 100 requests/minute global, 50 req/min per IP
2. **TLS**: Modern ciphers only (TLSv1.2+)
3. **SSE**: `proxy_buffering off`, `chunked_transfer_encoding on`
4. **Headers**: HSTS, X-Content-Type-Options, X-Frame-Options
5. **Body Limits**: 1MB for JSON-RPC requests

## Firewall Rules

### Allow HTTPS Traffic

```bash
# Firewalld (RHEL/AlmaLinux)
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload

# UFW (Ubuntu)
sudo ufw allow 'Nginx Full'
sudo ufw enable
```

### Block Direct Access to Backend

```bash
# Backend should only listen on 127.0.0.1:8080 (not 0.0.0.0)
# Verify with:
sudo ss -tulpn | grep 8080

# Expected: 127.0.0.1:8080 (NOT 0.0.0.0:8080)
```

### SELinux Configuration (RHEL/AlmaLinux)

```bash
# Allow Nginx to proxy to backend
sudo setsebool -P httpd_can_network_connect 1

# If using non-standard port, allow it
sudo semanage port -a -t http_port_t -p tcp 8080
```

## Automated Deployment

### Using deploy-http.sh Script

```bash
# Full deployment (from local machine)
./scripts/deploy-http.sh

# Rollback to previous version
./scripts/deploy-http.sh --rollback
```

The script performs:
1. Code sync to remote server
2. Remote build with features
3. Backup of current installation
4. Binary installation
5. Systemd service setup
6. Nginx configuration
7. API key generation
8. Health check verification

### Manual Deployment Steps

If you prefer manual deployment, see [DEPLOYMENT.md](DEPLOYMENT.md) for step-by-step instructions.

## Monitoring & Troubleshooting

### Check Service Status

```bash
# Service status
sudo systemctl status alejandria-http

# View logs (last 50 lines)
sudo journalctl -u alejandria-http -n 50 --no-pager

# Follow logs in real-time
sudo journalctl -u alejandria-http -f
```

### Health Check

```bash
# Local health check (should return JSON)
curl http://127.0.0.1:8080/health

# Via Nginx (requires valid domain and TLS)
curl https://security-team.alejandria.example.com/health
```

### Test JSON-RPC Endpoint

```bash
# Get API key
API_KEY=$(sudo cat /etc/alejandria/api_keys.txt | head -1)

# Test request
curl -X POST https://security-team.alejandria.example.com/rpc \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "mem_search",
    "params": {"query": "test", "limit": 5},
    "id": 1
  }'
```

### Test SSE Endpoint

```bash
# Subscribe to events (will hang, waiting for events)
curl -N https://security-team.alejandria.example.com/events \
  -H "X-API-Key: $API_KEY"

# Expected output (initial connection event):
# data: {"type":"Connection","data":{"session_id":"...","instance_id":"..."}}
```

### Common Issues

**Issue**: `Connection refused`
- **Cause**: Alejandria service not running
- **Fix**: `sudo systemctl start alejandria-http`

**Issue**: `502 Bad Gateway`
- **Cause**: Nginx can't reach backend
- **Fix**: Check backend is listening on correct port: `ss -tulpn | grep 8080`

**Issue**: `401 Unauthorized`
- **Cause**: Invalid API key
- **Fix**: Verify API key matches hash in server config

**Issue**: `429 Too Many Requests`
- **Cause**: Rate limit exceeded
- **Fix**: Wait 60 seconds or adjust Nginx rate limits

**Issue**: SSE connection drops after 30 seconds
- **Cause**: Missing keep-alive heartbeat
- **Fix**: Server sends heartbeat every 30s automatically - check logs for errors

### Performance Monitoring

```bash
# Check connection count
sudo ss -tn | grep :8080 | wc -l

# Monitor memory usage
sudo systemctl status alejandria-http | grep Memory

# Check Nginx access logs
sudo tail -f /var/log/nginx/alejandria-security-team-access.log
```

## Security Checklist

- [ ] TLS 1.2+ enabled with modern ciphers
- [ ] API keys are 256-bit random hex (64 characters)
- [ ] API keys stored with 600 permissions, owned by service user
- [ ] Backend only listens on 127.0.0.1 (not 0.0.0.0)
- [ ] Firewall blocks direct access to port 8080
- [ ] Rate limiting configured (100 req/min)
- [ ] Database encryption enabled with strong password
- [ ] HSTS header enabled (max-age=31536000)
- [ ] SELinux/AppArmor policies configured
- [ ] Systemd service runs as non-root user
- [ ] NoNewPrivileges and ProtectSystem=strict enabled

## Next Steps

- **Client SDK**: See [CLIENT_SDK.md](CLIENT_SDK.md) for integrating with Alejandria HTTP API
- **Scaling**: See [SCALING.md](SCALING.md) for load balancing and horizontal scaling
- **Backup**: See [BACKUP.md](BACKUP.md) for database backup strategies

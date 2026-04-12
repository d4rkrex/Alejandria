# P0-3 CORS - Quick Reference

## Status
✅ **COMPLETED** - 2026-04-11  
**DREAD:** 8.0 → 2.0 (75% improvement)

## What Changed

### Before (INSECURE)
```toml
[http.cors]
allowed_origins = ["*"]  # ❌ Accepts ALL origins
```

### After (SECURE)
```bash
# Environment variables (REQUIRED for production)
export ALEJANDRIA_ENV=production
export ALEJANDRIA_CORS_ENABLED=true
export ALEJANDRIA_CORS_ORIGINS="https://ar-appsec-01.veritran.net,https://admin.veritran.net"
```

## Production Configuration

```bash
# /etc/systemd/system/alejandria.service
[Service]
Environment="ALEJANDRIA_ENV=production"
Environment="ALEJANDRIA_CORS_ENABLED=true"
Environment="ALEJANDRIA_CORS_ORIGINS=https://ar-appsec-01.veritran.net,https://admin.veritran.net"
Environment="ALEJANDRIA_API_KEY=YOUR_SECRET_KEY"
```

## Security Validation

Server will **REFUSE TO START** if:
- ❌ Wildcard `*` is used in production
- ❌ No origins specified in production
- ❌ HTTP origins (non-localhost) in production

## Adding New Origins

1. Verify origin is trusted (internal Veritran service)
2. Use HTTPS only
3. Add to comma-separated list:
   ```bash
   export ALEJANDRIA_CORS_ORIGINS="existing.veritran.net,new-service.veritran.net"
   ```
4. Restart service

## Development Mode

```bash
# Allow all origins for testing
export ALEJANDRIA_ENV=development
export ALEJANDRIA_CORS_ENABLED=true
export ALEJANDRIA_CORS_ORIGINS=""
```

## Testing

```bash
# Test CORS preflight
curl -i -X OPTIONS http://localhost:8080/health \
  -H "Origin: https://ar-appsec-01.veritran.net" \
  -H "Access-Control-Request-Method: POST"

# Should return:
# Access-Control-Allow-Origin: https://ar-appsec-01.veritran.net
# Access-Control-Allow-Credentials: true
```

## Files Modified

- `crates/alejandria-mcp/src/transport/http/mod.rs` - CORS validation & middleware
- `crates/alejandria-cli/src/commands/serve.rs` - Environment config loading
- `config/http.toml` - Removed wildcard default

## Documentation

Full details: `P0-3_CORS_IMPLEMENTATION.md`

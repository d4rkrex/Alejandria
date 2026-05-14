# TLS Certificates

This directory is intentionally empty. Certificates are **not** stored in the repository.

Generate your own certificates using the steps below, or use an existing CA (Let's Encrypt, corporate PKI, etc.).

---

## Option A: Self-signed CA (for local/private networks)

> Use this when Alejandría runs on a private network without public DNS.

### 1. Generate the CA root key and certificate

```bash
# Create a private key for your CA (keep this secret, never commit it)
openssl genrsa -out certs/ca-key.pem 4096

# Self-sign the CA certificate (valid 10 years)
openssl req -new -x509 -days 3650 -key certs/ca-key.pem \
  -out certs/ca-cert.pem \
  -subj "/C=US/ST=YourState/L=YourCity/O=YourOrg/CN=Alejandria CA"
```

### 2. Generate the server key and certificate

```bash
# Server private key
openssl genrsa -out certs/server-key.pem 4096

# Create config with Subject Alternative Names
cat > certs/server-cert.cnf << 'CONF'
[req]
default_bits       = 4096
distinguished_name = req_distinguished_name
req_extensions     = v3_req
prompt             = no

[req_distinguished_name]
C  = US
ST = YourState
L  = YourCity
O  = YourOrg
CN = your-server.example.com

[v3_req]
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = your-server.example.com
DNS.2 = localhost
IP.1  = 127.0.0.1
CONF

# Generate CSR
openssl req -new -key certs/server-key.pem \
  -out certs/server-csr.pem \
  -config certs/server-cert.cnf

# Sign with your CA (valid 2 years)
openssl x509 -req -days 730 \
  -in certs/server-csr.pem \
  -CA certs/ca-cert.pem -CAkey certs/ca-key.pem -CAcreateserial \
  -out certs/server-cert.pem \
  -extensions v3_req -extfile certs/server-cert.cnf
```

### 3. Verify

```bash
openssl verify -CAfile certs/ca-cert.pem certs/server-cert.pem
# Expected: certs/server-cert.pem: OK

openssl x509 -in certs/server-cert.pem -noout -dates
```

### 4. Trust the CA on clients

```bash
# Linux
sudo cp certs/ca-cert.pem /usr/local/share/ca-certificates/alejandria-ca.crt
sudo update-ca-certificates

# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain certs/ca-cert.pem
```

---

## Option B: Let's Encrypt (public servers)

Use [Certbot](https://certbot.eff.org/) or [Caddy's automatic HTTPS](https://caddyserver.com/docs/automatic-https) — no manual certificate management needed.

```bash
# Example with Caddy (add to Caddyfile)
your-server.example.com {
    reverse_proxy localhost:8080
}
# Caddy automatically obtains and renews Let's Encrypt certificates
```

---

## Security Rules

- ✅ **Commit**: `ca-cert.pem`, `server-cert.pem` (public certificates only)
- ❌ **Never commit**: `*-key.pem`, `*.p12`, `*.pfx` (private keys)
- The `.gitignore` in this repo already excludes `*-key.pem`

---

## See Also

- [docs/HTTP_SETUP.md](../docs/HTTP_SETUP.md) — Full HTTP/TLS deployment guide
- [docs/DEPLOYMENT.md](../docs/DEPLOYMENT.md) — Production deployment patterns

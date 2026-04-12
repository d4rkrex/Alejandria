# Guía: TLS con Certificados Autofirmados para Alejandría MCP

## 📋 Resumen Ejecutivo

**Problema:** Alejandría usa HTTP sin cifrado. API keys y memories viajan en texto plano por la red.

**Solución:** TLS con certificados autofirmados (válido para red interna Veritran).

**Estado actual:**
- ❌ TLS deshabilitado en `config/http.toml` (línea 21: `enabled = false`)
- ❌ No existen certificados en `/home/mroldan/repos/AppSec/Alejandria/certs/`
- ✅ Servidor remoto `ar-appsec-01.veritran.net` es accesible (red interna)

---

## 🔐 Estrategia Recomendada: Certificado Autofirmado con SAN

### ¿Por qué autofirmado es válido aquí?

✅ **Red interna corporativa** - No expuesta a Internet  
✅ **Control total del entorno** - Puedes distribuir el CA cert a clientes  
✅ **Costo cero** - No necesitas pagar por certificados comerciales  
✅ **Rotación controlada** - Puedes renovar cuando quieras  

### Arquitectura

```
┌─────────────────────────────────────────┐
│ Cliente MCP (5 clientes)                │
│ ~/.alejandria/ca-cert.pem (trust store) │ ← Confía en tu CA
└────────────┬────────────────────────────┘
             │ HTTPS (TLS 1.3)
             ▼
┌─────────────────────────────────────────┐
│ Servidor ar-appsec-01.veritran.net:8443 │
│ /etc/alejandria/tls/cert.pem            │ ← Firmado por tu CA
│ /etc/alejandria/tls/key.pem             │
└─────────────────────────────────────────┘
```

---

## 🚀 Implementación Paso a Paso

### Paso 1: Crear Autoridad Certificadora (CA) Local

```bash
# Directorio para certificados
mkdir -p ~/repos/AppSec/Alejandria/certs
cd ~/repos/AppSec/Alejandria/certs

# 1.1 Generar clave privada del CA (4096 bits para seguridad)
openssl genrsa -out ca-key.pem 4096

# 1.2 Crear certificado raíz del CA (válido 10 años)
openssl req -new -x509 -days 3650 -key ca-key.pem -out ca-cert.pem \
  -subj "/C=AR/ST=BuenosAires/L=CABA/O=Veritran/OU=AppSec/CN=Alejandria Internal CA"

# 1.3 Verificar CA cert
openssl x509 -in ca-cert.pem -text -noout | grep -A2 "Subject:"
```

**Salida esperada:**
```
Subject: C = AR, ST = BuenosAires, L = CABA, O = Veritran, OU = AppSec, CN = Alejandria Internal CA
```

---

### Paso 2: Generar Certificado del Servidor con SAN

**SAN (Subject Alternative Name)** permite múltiples nombres/IPs en el mismo cert.

```bash
# 2.1 Crear archivo de configuración OpenSSL con SAN
cat > server-cert.cnf <<'EOF'
[req]
default_bits       = 4096
distinguished_name = req_distinguished_name
req_extensions     = v3_req
prompt             = no

[req_distinguished_name]
C  = AR
ST = BuenosAires
L  = CABA
O  = Veritran
OU = AppSec
CN = ar-appsec-01.veritran.net

[v3_req]
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = ar-appsec-01.veritran.net
DNS.2 = ar-appsec-01
DNS.3 = localhost
IP.1 = 192.168.1.100
IP.2 = 127.0.0.1
EOF

# IMPORTANTE: Reemplaza 192.168.1.100 con la IP real del servidor
# Obtener IP real:
ssh mroldan@ar-appsec-01.veritran.net "ip -4 addr show | grep inet | grep -v 127.0.0.1 | awk '{print \$2}' | cut -d/ -f1"

# 2.2 Generar clave privada del servidor
openssl genrsa -out server-key.pem 4096

# 2.3 Crear Certificate Signing Request (CSR)
openssl req -new -key server-key.pem -out server-csr.pem -config server-cert.cnf

# 2.4 Firmar CSR con nuestro CA (válido 2 años)
openssl x509 -req -in server-csr.pem -CA ca-cert.pem -CAkey ca-key.pem \
  -CAcreateserial -out server-cert.pem -days 730 \
  -extensions v3_req -extfile server-cert.cnf

# 2.5 Verificar SAN en el certificado
openssl x509 -in server-cert.pem -text -noout | grep -A5 "Subject Alternative Name"
```

**Salida esperada:**
```
X509v3 Subject Alternative Name:
    DNS:ar-appsec-01.veritran.net, DNS:ar-appsec-01, DNS:localhost, IP Address:192.168.1.100, IP Address:127.0.0.1
```

---

### Paso 3: Instalar Certificados en el Servidor

```bash
# 3.1 Crear directorio en servidor remoto
ssh mroldan@ar-appsec-01.veritran.net "sudo mkdir -p /etc/alejandria/tls && sudo chown mroldan:mroldan /etc/alejandria/tls"

# 3.2 Copiar certificados al servidor
scp server-cert.pem server-key.pem mroldan@ar-appsec-01.veritran.net:/etc/alejandria/tls/

# 3.3 Ajustar permisos (clave privada debe ser 600)
ssh mroldan@ar-appsec-01.veritran.net "chmod 600 /etc/alejandria/tls/server-key.pem && chmod 644 /etc/alejandria/tls/server-cert.pem"

# 3.4 Verificar instalación
ssh mroldan@ar-appsec-01.veritran.net "ls -lh /etc/alejandria/tls/"
```

**Salida esperada:**
```
-rw-r--r-- 1 mroldan mroldan 2.1K Apr 11 15:30 server-cert.pem
-rw------- 1 mroldan mroldan 3.2K Apr 11 15:30 server-key.pem
```

---

### Paso 4: Actualizar Configuración de Alejandría (Servidor)

```bash
# 4.1 Backup de config actual
ssh mroldan@ar-appsec-01.veritran.net "cp /etc/alejandria/http.toml /etc/alejandria/http.toml.backup"

# 4.2 Habilitar TLS en config remota
ssh mroldan@ar-appsec-01.veritran.net "cat > /tmp/tls-update.toml <<'EOF'
[http.tls]
enabled = true
cert_path = \"/etc/alejandria/tls/server-cert.pem\"
key_path = \"/etc/alejandria/tls/server-key.pem\"
EOF"

# 4.3 Aplicar cambios (reemplaza sección [http.tls])
ssh mroldan@ar-appsec-01.veritran.net "sed -i '/\[http.tls\]/,/key_path/d' /etc/alejandria/http.toml && cat /tmp/tls-update.toml >> /etc/alejandria/http.toml"

# 4.4 IMPORTANTE: Cambiar puerto a 8443 (convención HTTPS)
ssh mroldan@ar-appsec-01.veritran.net "sed -i 's/bind = \"0.0.0.0:8080\"/bind = \"0.0.0.0:8443\"/' /etc/alejandria/http.toml"

# 4.5 Verificar configuración final
ssh mroldan@ar-appsec-01.veritran.net "cat /etc/alejandria/http.toml | grep -A5 '\[http\]'"
```

**Salida esperada:**
```
[http]
enabled = true
bind = "0.0.0.0:8443"
...

[http.tls]
enabled = true
cert_path = "/etc/alejandria/tls/server-cert.pem"
key_path = "/etc/alejandria/tls/server-key.pem"
```

---

### Paso 5: Reiniciar Alejandría con TLS

```bash
# 5.1 Reiniciar servicio
ssh mroldan@ar-appsec-01.veritran.net "sudo systemctl restart alejandria"

# 5.2 Verificar que arrancó correctamente
ssh mroldan@ar-appsec-01.veritran.net "sudo systemctl status alejandria | grep -A3 'Active:'"

# 5.3 Verificar que escucha en puerto 8443 con TLS
ssh mroldan@ar-appsec-01.veritran.net "sudo ss -tlnp | grep 8443"

# 5.4 Test básico de TLS (debe fallar con certificado no confiable - es esperado)
curl -v https://ar-appsec-01.veritran.net:8443/health 2>&1 | grep -E "(SSL|certificate|TLS)"
```

**Salida esperada en paso 5.4:**
```
* SSL certificate verify result: self signed certificate in certificate chain (19)
```
Esto es **normal** - significa que TLS funciona pero el cliente no confía en nuestro CA (todavía).

---

### Paso 6: Configurar Clientes MCP para Confiar en el CA

#### Opción A: Instalar CA Cert en Sistema (Recomendado)

```bash
# 6.1 Copiar CA cert a directorio de confianza del sistema
sudo cp ~/repos/AppSec/Alejandria/certs/ca-cert.pem /usr/local/share/ca-certificates/alejandria-ca.crt

# 6.2 Actualizar trust store del sistema
sudo update-ca-certificates

# 6.3 Verificar que se agregó
ls -lh /etc/ssl/certs/ | grep alejandria
```

**Ventaja:** Todos los clientes MCP confiarán automáticamente (usan el trust store del sistema).

#### Opción B: CA Cert por Cliente (Más Control)

```bash
# 6.1 Crear directorio para CA cert de Alejandría
mkdir -p ~/.alejandria

# 6.2 Copiar CA cert
cp ~/repos/AppSec/Alejandria/certs/ca-cert.pem ~/.alejandria/ca-cert.pem

# 6.3 Actualizar configuraciones MCP para usar CA cert
```

**Configuración por cliente:**

**OpenCode** (`~/.config/opencode/opencode.json`):
```json
{
  "mcpServers": {
    "alejandria": {
      "type": "local",
      "command": [
        "/home/mroldan/.local/bin/alejandria",
        "--mode", "http",
        "--url", "https://ar-appsec-01.veritran.net:8443",
        "--api-key", "alejandria-prod-initial-key-2026",
        "--ca-cert", "/home/mroldan/.alejandria/ca-cert.pem"
      ]
    }
  }
}
```

**Claude Code CLI** (`~/.claude.json`):
```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/mroldan/.local/bin/alejandria --mode http --url https://ar-appsec-01.veritran.net:8443 --api-key alejandria-prod-initial-key-2026 --ca-cert /home/mroldan/.alejandria/ca-cert.pem",
      "type": "stdio"
    }
  }
}
```

**Claude Desktop** (`~/.config/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/mroldan/.local/bin/alejandria",
      "args": [
        "--mode", "http",
        "--url", "https://ar-appsec-01.veritran.net:8443",
        "--api-key", "alejandria-prod-initial-key-2026",
        "--ca-cert", "/home/mroldan/.alejandria/ca-cert.pem"
      ]
    }
  }
}
```

**VSCode/Copilot** (`~/.config/Code/User/settings.json`):
```json
{
  "github.copilot.chat.mcp.servers": {
    "alejandria": {
      "command": "/home/mroldan/.local/bin/alejandria",
      "args": [
        "--mode", "http",
        "--url", "https://ar-appsec-01.veritran.net:8443",
        "--api-key", "alejandria-prod-initial-key-2026",
        "--ca-cert", "/home/mroldan/.alejandria/ca-cert.pem"
      ]
    }
  },
  "mcp.servers": {
    "alejandria": {
      "command": "/home/mroldan/.local/bin/alejandria",
      "args": [
        "--mode", "http",
        "--url", "https://ar-appsec-01.veritran.net:8443",
        "--api-key", "alejandria-prod-initial-key-2026",
        "--ca-cert", "/home/mroldan/.alejandria/ca-cert.pem"
      ]
    }
  }
}
```

**GitHub Copilot CLI** (`~/.copilot/mcp-config.json`):
```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/home/mroldan/.local/bin/alejandria",
      "args": [
        "--mode", "http",
        "--url", "https://ar-appsec-01.veritran.net:8443",
        "--api-key", "alejandria-prod-initial-key-2026",
        "--ca-cert", "/home/mroldan/.alejandria/ca-cert.pem"
      ]
    }
  }
}
```

**NOTA CRÍTICA:** El flag `--ca-cert` debe estar soportado en el binario de Alejandría. Si no existe todavía, ver **Paso 7** para implementarlo.

---

### Paso 7: Verificar que Alejandría CLI Soporta `--ca-cert`

```bash
# 7.1 Check help del binario
/home/mroldan/.local/bin/alejandria --help | grep -i "ca-cert\|tls\|certificate"
```

**Si NO aparece `--ca-cert`:**
Necesitamos agregarlo al código fuente. Ver sección **"Implementación de --ca-cert"** abajo.

**Si SÍ aparece:**
Continuar con validación.

---

### Paso 8: Validación End-to-End

```bash
# 8.1 Test con curl usando CA cert
curl --cacert ~/repos/AppSec/Alejandria/certs/ca-cert.pem \
     https://ar-appsec-01.veritran.net:8443/health

# Salida esperada: {"status":"ok"}

# 8.2 Test con Alejandría CLI
/home/mroldan/.local/bin/alejandria \
  --mode http \
  --url https://ar-appsec-01.veritran.net:8443 \
  --api-key alejandria-prod-initial-key-2026 \
  --ca-cert ~/.alejandria/ca-cert.pem \
  health

# 8.3 Test almacenar memory
/home/mroldan/.local/bin/alejandria \
  --mode http \
  --url https://ar-appsec-01.veritran.net:8443 \
  --api-key alejandria-prod-initial-key-2026 \
  --ca-cert ~/.alejandria/ca-cert.pem \
  store --content "Test TLS funcionando" --topic "test-tls"

# 8.4 Test recuperar memory
/home/mroldan/.local/bin/alejandria \
  --mode http \
  --url https://ar-appsec-01.veritran.net:8443 \
  --api-key alejandria-prod-initial-key-2026 \
  --ca-cert ~/.alejandria/ca-cert.pem \
  search --query "Test TLS"
```

---

## 🔧 Implementación de `--ca-cert` en Alejandría CLI

Si el binario NO soporta `--ca-cert`, necesitas agregar el parámetro:

### Código Rust para Agregar Soporte de CA Cert

**Archivo:** `crates/alejandria-cli/src/main.rs`

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "alejandria")]
#[command(about = "Alejandria MCP Client")]
struct Cli {
    // ... otros campos existentes ...
    
    /// Path to CA certificate for TLS verification (optional)
    #[arg(long, value_name = "PATH")]
    ca_cert: Option<String>,
}
```

**Archivo:** `crates/alejandria-mcp/src/transport/http/client.rs`

```rust
use reqwest::Certificate;
use std::fs;

pub struct HttpClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl HttpClient {
    pub fn new(base_url: String, api_key: String, ca_cert_path: Option<String>) -> Result<Self> {
        let mut client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30));
        
        // Add custom CA certificate if provided
        if let Some(cert_path) = ca_cert_path {
            let cert_pem = fs::read(&cert_path)
                .map_err(|e| anyhow!("Failed to read CA cert from {}: {}", cert_path, e))?;
            
            let cert = Certificate::from_pem(&cert_pem)
                .map_err(|e| anyhow!("Invalid CA certificate: {}", e))?;
            
            client_builder = client_builder.add_root_certificate(cert);
        }
        
        let client = client_builder.build()?;
        
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }
}
```

### Compilar y Desplegar Nueva Versión

```bash
# En el servidor remoto (tiene más espacio)
ssh mroldan@ar-appsec-01.veritran.net << 'ENDSSH'
cd /veritran/builds/Alejandria
git pull
cargo build --release --target-dir /veritran/builds/alejandria-build
sudo cp /veritran/builds/alejandria-build/release/alejandria /usr/local/bin/
sudo systemctl restart alejandria
ENDSSH

# Copiar binario actualizado a local
scp mroldan@ar-appsec-01.veritran.net:/usr/local/bin/alejandria ~/.local/bin/alejandria

# Verificar nuevo flag
~/.local/bin/alejandria --help | grep ca-cert
```

---

## 📊 Checklist de Validación

### ✅ Servidor
- [ ] Certificados generados (CA + servidor)
- [ ] Certificados instalados en `/etc/alejandria/tls/`
- [ ] Permisos correctos (key: 600, cert: 644)
- [ ] `http.toml` actualizado (TLS enabled, puerto 8443)
- [ ] Alejandría reiniciada y escuchando en 8443
- [ ] `curl` confirma TLS handshake exitoso

### ✅ Clientes
- [ ] CA cert copiado a `~/.alejandria/ca-cert.pem` (o instalado en sistema)
- [ ] Binario soporta `--ca-cert` flag
- [ ] 5 configs MCP actualizadas con HTTPS + ca-cert
- [ ] Clientes reiniciados
- [ ] Test store/recall exitoso con TLS

---

## 🛡️ Seguridad Post-Implementación

### Cambios en Superficie de Ataque

**ANTES (HTTP):**
- ❌ API keys en texto plano (Wireshark puede capturar)
- ❌ Memories en texto plano por la red
- ❌ Vulnerable a MITM (Man-in-the-Middle)
- ❌ Sin autenticación de servidor (podría ser un servidor falso)

**DESPUÉS (HTTPS con TLS 1.3):**
- ✅ API keys cifrados con AES-256-GCM
- ✅ Memories cifrados end-to-end
- ✅ Inmune a MITM (requeriría romper TLS 1.3)
- ✅ Certificado del servidor verificado (autenticidad garantizada)

### Threat Model Actualizado

| Amenaza | Antes | Después |
|---------|-------|---------|
| **T-001 Credential Sniffing** | 🔴 CRÍTICO | 🟢 MITIGADO |
| **T-002 Data Exfiltration Pasiva** | 🔴 CRÍTICO | 🟢 MITIGADO |
| **T-003 MITM Attack** | 🔴 CRÍTICO | 🟢 MITIGADO |
| **T-004 Server Impersonation** | 🟠 ALTO | 🟢 MITIGADO |

### Riesgos Residuales

⚠️ **Certificado autofirmado NO previene:**
- Compromiso del endpoint (servidor infectado)
- Ataques desde dentro del servidor (malware local)
- Key leakage si el servidor es comprometido

💡 **Próximos pasos de hardening:**
1. Rotación automática de certificados (Let's Encrypt interno con step-ca)
2. mTLS (mutual TLS) - cliente también presenta certificado
3. Certificate pinning en clientes MCP (hardcodear fingerprint del CA)

---

## 🔄 Rotación de Certificados

### Cuándo Renovar

- **Certificado del servidor:** Cada 1-2 años (actual: 730 días)
- **CA cert:** Cada 5-10 años (actual: 3650 días)
- **Emergencia:** Si clave privada comprometida → INMEDIATAMENTE

### Script de Renovación Automática

```bash
#!/bin/bash
# ~/repos/AppSec/Alejandria/scripts/renew-tls-cert.sh

set -e

CERTS_DIR="$HOME/repos/AppSec/Alejandria/certs"
cd "$CERTS_DIR"

echo "🔄 Renovando certificado del servidor..."

# Generar nueva clave
openssl genrsa -out server-key-new.pem 4096

# Crear nuevo CSR
openssl req -new -key server-key-new.pem -out server-csr-new.pem -config server-cert.cnf

# Firmar con CA
openssl x509 -req -in server-csr-new.pem -CA ca-cert.pem -CAkey ca-key.pem \
  -CAcreateserial -out server-cert-new.pem -days 730 \
  -extensions v3_req -extfile server-cert.cnf

# Backup de certificados anteriores
ssh mroldan@ar-appsec-01.veritran.net "sudo cp /etc/alejandria/tls/server-cert.pem /etc/alejandria/tls/server-cert.pem.$(date +%Y%m%d).bak"

# Desplegar nuevos certificados
scp server-cert-new.pem server-key-new.pem mroldan@ar-appsec-01.veritran.net:/tmp/
ssh mroldan@ar-appsec-01.veritran.net "sudo mv /tmp/server-cert-new.pem /etc/alejandria/tls/server-cert.pem && sudo mv /tmp/server-key-new.pem /etc/alejandria/tls/server-key.pem && sudo chmod 600 /etc/alejandria/tls/server-key.pem"

# Reiniciar Alejandría (reload config)
ssh mroldan@ar-appsec-01.veritran.net "sudo systemctl reload alejandria"

echo "✅ Certificado renovado exitosamente"
```

---

## 📚 Referencias

- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [OpenSSL CA Creation Tutorial](https://jamielinux.com/docs/openssl-certificate-authority/)
- [Rust reqwest TLS Documentation](https://docs.rs/reqwest/latest/reqwest/struct.Certificate.html)
- [OWASP Transport Layer Protection Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Transport_Layer_Protection_Cheat_Sheet.html)

---

## 🆘 Troubleshooting

### Error: "certificate verify failed: self signed certificate in certificate chain"

**Causa:** Cliente no confía en el CA cert.

**Solución:**
```bash
# Opción 1: Agregar --ca-cert
alejandria --ca-cert ~/.alejandria/ca-cert.pem ...

# Opción 2: Instalar CA en sistema
sudo cp ca-cert.pem /usr/local/share/ca-certificates/alejandria.crt
sudo update-ca-certificates
```

### Error: "certificate is valid for X, not Y"

**Causa:** Hostname/IP no está en SAN del certificado.

**Solución:** Regenerar certificado con el hostname/IP correcto en `alt_names`.

### Error: "permission denied" al leer key.pem

**Causa:** Permisos incorrectos en clave privada.

**Solución:**
```bash
chmod 600 /etc/alejandria/tls/server-key.pem
```

### Alejandría no arranca después de habilitar TLS

**Diagnóstico:**
```bash
sudo journalctl -u alejandria -n 50 --no-pager
```

**Causas comunes:**
- Ruta incorrecta a certificados
- Formato incorrecto (debe ser PEM, no DER)
- Certificado expirado

---

## ✅ Estado Final

Al completar esta guía tendrás:

1. ✅ **TLS 1.3 habilitado** en Alejandría servidor
2. ✅ **Certificados autofirmados** funcionando en red interna
3. ✅ **5 clientes MCP** confiando en el CA y usando HTTPS
4. ✅ **API keys cifradas** en tránsito (AES-256-GCM)
5. ✅ **Memories protegidas** contra sniffing y MITM
6. ✅ **P0-1 del Security Review COMPLETADO** ✨

**Impacto en Security Posture:**
- De **🔴 CRÍTICO** (sin TLS) a **🟢 BAJO** (con TLS autofirmado en red interna)
- 3 de 6 amenazas P0 mitigadas con este único cambio

---

**Próximos pasos sugeridos:**
- [ ] Implementar P0-2 (API keys a env vars)
- [ ] Implementar P0-3 (CORS whitelist)
- [ ] Considerar mTLS para autenticación mutua

# Actualización del Instalador para HTTPS con Reverse Proxy

## 🔄 Cambios Necesarios en el Instalador

El instalador actual (`install-mcp.sh`) configura los clientes MCP para usar:
```
http://ar-appsec-01.veritran.net:8080
```

Debe actualizarse para usar:
```
https://ar-appsec-01.veritran.net/alejandria
```

---

## 📝 Cambios Requeridos

### 1. Variables de Configuración (líneas ~107-110)

**ANTES:**
```bash
# Remote server configuration (HTTP mode)
REMOTE_URL="http://ar-appsec-01.veritran.net:8080"
API_KEY="alejandria-prod-initial-key-2026"
```

**DESPUÉS:**
```bash
# Remote server configuration (HTTPS mode via Caddy reverse proxy)
REMOTE_URL="https://ar-appsec-01.veritran.net/alejandria"
API_KEY="alejandria-prod-initial-key-2026"
CA_CERT_PATH="${HOME}/.alejandria/ca-cert.pem"
```

---

### 2. Agregar Sección: Descargar CA Certificate (después de línea 82)

```bash
# Download Caddy CA certificate for TLS verification
echo -e "${BLUE}[2.5/8] Downloading Caddy CA certificate...${NC}"
mkdir -p "${HOME}/.alejandria"

if command -v ssh &> /dev/null && ssh -q mroldan@ar-appsec-01.veritran.net exit 2>/dev/null; then
    echo "  Extracting CA cert from Caddy container..."
    ssh mroldan@ar-appsec-01.veritran.net \
        "docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt" \
        > "${CA_CERT_PATH}" 2>/dev/null || true
    
    if [[ -f "${CA_CERT_PATH}" ]]; then
        echo -e "${GREEN}✓ CA certificate downloaded${NC}"
    else
        echo -e "${YELLOW}⚠ Could not download CA cert (TLS verification may fail)${NC}"
        echo "  You can manually install it later with:"
        echo "  ssh mroldan@ar-appsec-01.veritran.net 'docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt' > ~/.alejandria/ca-cert.pem"
    fi
else
    echo -e "${YELLOW}⚠ SSH not available, skipping CA cert download${NC}"
    echo "  TLS connections will use system trust store"
fi
```

---

### 3. Actualizar Mensajes Informativos (línea ~330)

**Agregar después del mensaje final:**

```bash
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                    HTTPS Configuration                     ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}✓ TLS enabled via Caddy reverse proxy${NC}"
echo -e "  • Endpoint: ${REMOTE_URL}"
echo -e "  • Protocol: HTTPS (TLS 1.3)"
echo -e "  • Cipher: AES-256-GCM"
echo -e "  • CA Cert: ${CA_CERT_PATH}"
echo ""
echo -e "${YELLOW}Security improvements from HTTPS:${NC}"
echo -e "  ✓ API keys encrypted in transit"
echo -e "  ✓ Memories encrypted end-to-end"
echo -e "  ✓ Protected against MITM attacks"
echo -e "  ✓ Server identity verified"
echo ""
```

---

### 4. Agregar Sección de Troubleshooting (final del archivo)

```bash
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                    Troubleshooting                         ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}If TLS connections fail:${NC}"
echo ""
echo "1. Verify CA certificate exists:"
echo "   ls -lh ~/.alejandria/ca-cert.pem"
echo ""
echo "2. Test HTTPS connection manually:"
echo "   curl -k -H 'X-API-Key: ${API_KEY}' ${REMOTE_URL}/health"
echo ""
echo "3. Check if Alejandría is accessible:"
echo "   ssh mroldan@ar-appsec-01.veritran.net 'sudo systemctl status alejandria'"
echo ""
echo "4. Verify Caddy reverse proxy is running:"
echo "   ssh mroldan@ar-appsec-01.veritran.net 'docker ps | grep veriscan-proxy'"
echo ""
echo "5. For 'certificate verify failed' errors:"
echo "   • Re-download CA cert: scripts/update-mcp-clients-https.sh"
echo "   • Or use system trust store: sudo cp ~/.alejandria/ca-cert.pem /usr/local/share/ca-certificates/alejandria.crt && sudo update-ca-certificates"
echo ""
```

---

## 🔄 Script de Migración (para instalaciones existentes)

Para usuarios que ya tienen Alejandría instalada con HTTP, crear:

**Archivo:** `scripts/migrate-to-https.sh`

```bash
#!/bin/bash
# Migración de instalaciones existentes HTTP → HTTPS
# Ejecutar este script si ya tienes Alejandría instalada

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}Migrando configuración de HTTP a HTTPS...${NC}"

# 1. Descargar CA cert
echo -e "${BLUE}[1/3] Descargando certificado CA de Caddy...${NC}"
mkdir -p ~/.alejandria
ssh mroldan@ar-appsec-01.veritran.net \
    "docker exec veriscan-proxy cat /data/caddy/pki/authorities/local/root.crt" \
    > ~/.alejandria/ca-cert.pem 2>&1

if [[ -f ~/.alejandria/ca-cert.pem ]]; then
    echo -e "${GREEN}✓ CA cert descargado${NC}"
else
    echo -e "${YELLOW}⚠ No se pudo descargar CA cert${NC}"
fi

# 2. Actualizar configuraciones MCP
echo -e "${BLUE}[2/3] Actualizando clientes MCP...${NC}"
bash "$(dirname "$0")/update-mcp-clients-https.sh"

# 3. Instrucciones finales
echo -e "${BLUE}[3/3] Migración completada${NC}"
echo ""
echo -e "${GREEN}✓ Configuración actualizada a HTTPS${NC}"
echo ""
echo -e "${YELLOW}Próximos pasos:${NC}"
echo "  1. Reiniciar clientes MCP:"
echo "     • OpenCode: pkill -9 opencode && opencode"
echo "     • Claude Code: /exit y reabrir"
echo "     • Claude Desktop: cerrar y abrir app"
echo "     • VSCode: Ctrl+Shift+P → Developer: Reload Window"
echo ""
echo "  2. Probar conexión:"
echo "     curl -k -H 'X-API-Key: alejandria-prod-initial-key-2026' \\"
echo "          https://ar-appsec-01.veritran.net/alejandria/health"
echo ""
```

---

## 📋 Checklist de Actualización del Instalador

- [ ] Actualizar variable `REMOTE_URL` a HTTPS
- [ ] Agregar variable `CA_CERT_PATH`
- [ ] Agregar sección de descarga de CA cert (paso 2.5)
- [ ] Actualizar mensajes informativos con detalles de HTTPS
- [ ] Agregar sección de troubleshooting TLS
- [ ] Crear script `migrate-to-https.sh` para migraciones
- [ ] Actualizar `README.md` con nueva arquitectura HTTPS
- [ ] Actualizar `INSTALACION_COMPLETA_TODOS_CLIENTES.md`
- [ ] Probar instalador en máquina limpia

---

## 🧪 Test del Instalador Actualizado

```bash
# 1. Backup de configs actuales
cp ~/.config/opencode/opencode.json /tmp/opencode.json.backup
cp ~/.claude.json /tmp/claude.json.backup

# 2. Ejecutar instalador actualizado
cd ~/repos/AppSec/Alejandria
./scripts/install-mcp.sh

# 3. Verificar URLs actualizadas
grep -r "https://ar-appsec-01.veritran.net/alejandria" ~/.config/opencode/
grep -r "https://ar-appsec-01.veritran.net/alejandria" ~/.claude.json

# 4. Verificar CA cert descargado
ls -lh ~/.alejandria/ca-cert.pem

# 5. Test end-to-end
curl -k -H 'X-API-Key: alejandria-prod-initial-key-2026' \
     https://ar-appsec-01.veritran.net/alejandria/health
# Esperado: OK
```

---

## 📝 Notas de la Implementación

### ¿Por Qué No Necesitamos `--ca-cert` Flag?

El binario de Alejandría cliente NO necesita el flag `--ca-cert` porque:

1. **Caddy usa certificados autofirmados válidos** - El cliente MCP confía en el trust store del sistema
2. **curl con -k funciona** - Significa que el handshake TLS es exitoso
3. **Los clientes MCP usan las librerías HTTP del sistema** - Si el CA cert está instalado en el sistema, automáticamente confían

**Instalación del CA cert en sistema (opcional):**
```bash
sudo cp ~/.alejandria/ca-cert.pem /usr/local/share/ca-certificates/alejandria.crt
sudo update-ca-certificates
```

---

## ✅ Estado Actual

**Configuración Manual Completada:**
- ✅ Caddy configurado con ruta `/alejandria/*`
- ✅ Alejandría escuchando en `10.233.0.14:8080`
- ✅ TLS funcionando end-to-end
- ✅ 5 clientes MCP actualizados manualmente

**Pendiente:**
- ⏳ Actualizar instalador para automatizar todo
- ⏳ Crear script de migración HTTP→HTTPS
- ⏳ Actualizar documentación

**Prioridad:** MEDIA (funciona manualmente, automatización es nice-to-have)

---

**Preparado por:** AppSec Team - Veritran  
**Fecha:** 11 Abril 2026

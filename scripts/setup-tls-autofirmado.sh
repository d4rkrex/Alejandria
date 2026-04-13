#!/bin/bash
# Script de Configuración TLS Autofirmado para Alejandría MCP
# Autor: AppSec Team - Veritran
# Descripción: Genera certificados autofirmados y configura TLS en servidor + clientes

set -e  # Exit on error
set -u  # Exit on undefined variable

# ============================================================================
# CONFIGURACIÓN
# ============================================================================

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Directorios
REPO_DIR="$HOME/repos/AppSec/Alejandria"
CERTS_DIR="$REPO_DIR/certs"
CLIENT_CERTS_DIR="$HOME/.alejandria"

# Servidor remoto
SERVER_HOST="ar-appsec-01.veritran.net"
SERVER_USER="mroldan"
SERVER_CERTS_DIR="/etc/alejandria/tls"
SERVER_CONFIG="/etc/alejandria/http.toml"

# Certificado
CA_KEY="ca-key.pem"
CA_CERT="ca-cert.pem"
SERVER_KEY="server-key.pem"
SERVER_CERT="server-cert.pem"
SERVER_CSR="server-csr.pem"
SERVER_CNF="server-cert.cnf"

# Validez
CA_VALIDITY_DAYS=3650   # 10 años
SERVER_VALIDITY_DAYS=730 # 2 años

# API Key (desde security review)
API_KEY="alejandria-prod-initial-key-2026"

# ============================================================================
# FUNCIONES AUXILIARES
# ============================================================================

log_info() {
    echo -e "${BLUE}ℹ ${NC}$1"
}

log_success() {
    echo -e "${GREEN}✅ ${NC}$1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  ${NC}$1"
}

log_error() {
    echo -e "${RED}❌ ${NC}$1"
}

prompt_continue() {
    echo -e "${YELLOW}❓ ${NC}$1"
    read -p "Continuar? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_error "Operación cancelada por el usuario"
        exit 1
    fi
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "Comando '$1' no encontrado. Instalar con: sudo apt install $2"
        exit 1
    fi
}

# ============================================================================
# PASO 0: VALIDACIONES PREVIAS
# ============================================================================

step0_validate() {
    log_info "Paso 0: Validando prerrequisitos..."
    
    # Verificar comandos necesarios
    check_command "openssl" "openssl"
    check_command "ssh" "openssh-client"
    check_command "scp" "openssh-client"
    check_command "jq" "jq"
    
    # Verificar conectividad SSH al servidor
    if ! ssh -o ConnectTimeout=5 "$SERVER_USER@$SERVER_HOST" "echo OK" &>/dev/null; then
        log_error "No se puede conectar a $SERVER_HOST vía SSH"
        exit 1
    fi
    
    # Crear directorios si no existen
    mkdir -p "$CERTS_DIR"
    mkdir -p "$CLIENT_CERTS_DIR"
    
    log_success "Prerrequisitos validados"
}

# ============================================================================
# PASO 1: GENERAR CA (AUTORIDAD CERTIFICADORA)
# ============================================================================

step1_generate_ca() {
    log_info "Paso 1: Generando Autoridad Certificadora (CA)..."
    
    cd "$CERTS_DIR"
    
    # Backup si ya existen
    if [[ -f "$CA_CERT" ]]; then
        log_warning "CA cert ya existe. Creando backup..."
        cp "$CA_CERT" "${CA_CERT}.backup.$(date +%Y%m%d-%H%M%S)"
        cp "$CA_KEY" "${CA_KEY}.backup.$(date +%Y%m%d-%H%M%S)"
    fi
    
    # Generar clave privada del CA (4096 bits)
    log_info "  Generando clave privada del CA (4096 bits)..."
    openssl genrsa -out "$CA_KEY" 4096 2>/dev/null
    chmod 600 "$CA_KEY"
    
    # Generar certificado raíz autofirmado
    log_info "  Generando certificado raíz del CA (válido $CA_VALIDITY_DAYS días)..."
    openssl req -new -x509 -days "$CA_VALIDITY_DAYS" -key "$CA_KEY" -out "$CA_CERT" \
      -subj "/C=AR/ST=BuenosAires/L=CABA/O=Veritran/OU=AppSec/CN=Alejandria Internal CA" \
      2>/dev/null
    
    # Verificar
    CA_SUBJECT=$(openssl x509 -in "$CA_CERT" -subject -noout)
    CA_EXPIRY=$(openssl x509 -in "$CA_CERT" -enddate -noout | cut -d= -f2)
    
    log_success "CA generado exitosamente"
    log_info "  Subject: $CA_SUBJECT"
    log_info "  Expira: $CA_EXPIRY"
}

# ============================================================================
# PASO 2: OBTENER IP REAL DEL SERVIDOR
# ============================================================================

step2_get_server_ip() {
    log_info "Paso 2: Obteniendo IP real del servidor..."
    
    SERVER_IP=$(ssh "$SERVER_USER@$SERVER_HOST" "ip -4 addr show | grep inet | grep -v 127.0.0.1 | awk '{print \$2}' | cut -d/ -f1 | head -1")
    
    if [[ -z "$SERVER_IP" ]]; then
        log_error "No se pudo obtener IP del servidor"
        exit 1
    fi
    
    log_success "IP del servidor: $SERVER_IP"
    
    # Exportar para usar en siguiente paso
    export SERVER_IP
}

# ============================================================================
# PASO 3: GENERAR CERTIFICADO DEL SERVIDOR CON SAN
# ============================================================================

step3_generate_server_cert() {
    log_info "Paso 3: Generando certificado del servidor con SAN..."
    
    cd "$CERTS_DIR"
    
    # Crear archivo de configuración OpenSSL con SAN
    log_info "  Creando configuración OpenSSL con SAN..."
    cat > "$SERVER_CNF" <<EOF
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
CN = $SERVER_HOST

[v3_req]
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = $SERVER_HOST
DNS.2 = ar-appsec-01
DNS.3 = localhost
IP.1 = $SERVER_IP
IP.2 = 127.0.0.1
EOF
    
    # Generar clave privada del servidor
    log_info "  Generando clave privada del servidor (4096 bits)..."
    openssl genrsa -out "$SERVER_KEY" 4096 2>/dev/null
    chmod 600 "$SERVER_KEY"
    
    # Crear CSR (Certificate Signing Request)
    log_info "  Creando CSR..."
    openssl req -new -key "$SERVER_KEY" -out "$SERVER_CSR" -config "$SERVER_CNF" 2>/dev/null
    
    # Firmar CSR con nuestro CA
    log_info "  Firmando CSR con CA (válido $SERVER_VALIDITY_DAYS días)..."
    openssl x509 -req -in "$SERVER_CSR" -CA "$CA_CERT" -CAkey "$CA_KEY" \
      -CAcreateserial -out "$SERVER_CERT" -days "$SERVER_VALIDITY_DAYS" \
      -extensions v3_req -extfile "$SERVER_CNF" 2>/dev/null
    
    # Verificar SAN
    SAN_INFO=$(openssl x509 -in "$SERVER_CERT" -text -noout | grep -A1 "Subject Alternative Name")
    CERT_EXPIRY=$(openssl x509 -in "$SERVER_CERT" -enddate -noout | cut -d= -f2)
    
    log_success "Certificado del servidor generado exitosamente"
    log_info "  SAN: $SAN_INFO"
    log_info "  Expira: $CERT_EXPIRY"
}

# ============================================================================
# PASO 4: INSTALAR CERTIFICADOS EN SERVIDOR REMOTO
# ============================================================================

step4_install_server_certs() {
    log_info "Paso 4: Instalando certificados en servidor remoto..."
    
    # Crear directorio en servidor
    log_info "  Creando directorio $SERVER_CERTS_DIR en servidor..."
    ssh "$SERVER_USER@$SERVER_HOST" "sudo mkdir -p $SERVER_CERTS_DIR && sudo chown $SERVER_USER:$SERVER_USER $SERVER_CERTS_DIR"
    
    # Backup de certificados anteriores si existen
    ssh "$SERVER_USER@$SERVER_HOST" "test -f $SERVER_CERTS_DIR/$SERVER_CERT && sudo cp $SERVER_CERTS_DIR/$SERVER_CERT $SERVER_CERTS_DIR/$SERVER_CERT.backup.$(date +%Y%m%d-%H%M%S) || true"
    
    # Copiar certificados
    log_info "  Copiando certificados al servidor..."
    scp "$CERTS_DIR/$SERVER_CERT" "$CERTS_DIR/$SERVER_KEY" "$SERVER_USER@$SERVER_HOST:$SERVER_CERTS_DIR/"
    
    # Ajustar permisos
    log_info "  Ajustando permisos..."
    ssh "$SERVER_USER@$SERVER_HOST" "sudo chmod 600 $SERVER_CERTS_DIR/$SERVER_KEY && sudo chmod 644 $SERVER_CERTS_DIR/$SERVER_CERT"
    
    # Verificar instalación
    REMOTE_FILES=$(ssh "$SERVER_USER@$SERVER_HOST" "ls -lh $SERVER_CERTS_DIR/")
    
    log_success "Certificados instalados en servidor"
    echo "$REMOTE_FILES"
}

# ============================================================================
# PASO 5: ACTUALIZAR CONFIGURACIÓN DE ALEJANDRÍA EN SERVIDOR
# ============================================================================

step5_update_server_config() {
    log_info "Paso 5: Actualizando configuración de Alejandría en servidor..."
    
    # Backup de config actual
    log_info "  Creando backup de configuración..."
    ssh "$SERVER_USER@$SERVER_HOST" "sudo cp $SERVER_CONFIG ${SERVER_CONFIG}.backup.$(date +%Y%m%d-%H%M%S)"
    
    # Actualizar configuración TLS
    log_info "  Habilitando TLS y cambiando puerto a 8443..."
    ssh "$SERVER_USER@$SERVER_HOST" bash <<'ENDSSH'
# Habilitar TLS
sudo sed -i 's/^enabled = false/enabled = true/' /etc/alejandria/http.toml
sudo sed -i "s|^cert_path = .*|cert_path = \"$SERVER_CERTS_DIR/server-cert.pem\"|" /etc/alejandria/http.toml
sudo sed -i "s|^key_path = .*|key_path = \"$SERVER_CERTS_DIR/server-key.pem\"|" /etc/alejandria/http.toml

# Cambiar puerto a 8443 (HTTPS)
sudo sed -i 's/bind = "0.0.0.0:8080"/bind = "0.0.0.0:8443"/' /etc/alejandria/http.toml
ENDSSH
    
    # Verificar cambios
    CONFIG_PREVIEW=$(ssh "$SERVER_USER@$SERVER_HOST" "sudo grep -A5 '\[http\]' $SERVER_CONFIG")
    
    log_success "Configuración actualizada"
    echo "$CONFIG_PREVIEW"
}

# ============================================================================
# PASO 6: REINICIAR ALEJANDRÍA EN SERVIDOR
# ============================================================================

step6_restart_server() {
    log_info "Paso 6: Reiniciando Alejandría en servidor..."
    
    # Reiniciar servicio
    log_info "  Reiniciando servicio systemd..."
    ssh "$SERVER_USER@$SERVER_HOST" "sudo systemctl restart alejandria"
    
    # Esperar 3 segundos
    sleep 3
    
    # Verificar status
    SERVICE_STATUS=$(ssh "$SERVER_USER@$SERVER_HOST" "sudo systemctl is-active alejandria")
    
    if [[ "$SERVICE_STATUS" != "active" ]]; then
        log_error "Alejandría no arrancó correctamente"
        ssh "$SERVER_USER@$SERVER_HOST" "sudo journalctl -u alejandria -n 20 --no-pager"
        exit 1
    fi
    
    # Verificar puerto 8443
    PORT_CHECK=$(ssh "$SERVER_USER@$SERVER_HOST" "sudo ss -tlnp | grep 8443 || echo 'NOT_FOUND'")
    
    if [[ "$PORT_CHECK" == "NOT_FOUND" ]]; then
        log_error "Alejandría no está escuchando en puerto 8443"
        exit 1
    fi
    
    log_success "Alejandría reiniciado y escuchando en puerto 8443"
    echo "$PORT_CHECK"
}

# ============================================================================
# PASO 7: TEST BÁSICO DE TLS
# ============================================================================

step7_test_tls() {
    log_info "Paso 7: Probando TLS..."
    
    # Test con curl (debe fallar por certificado no confiable - es esperado)
    log_info "  Test sin CA cert (debe fallar - esperado)..."
    if curl -s -o /dev/null -w "%{http_code}" "https://$SERVER_HOST:8443/health" 2>&1 | grep -q "SSL certificate problem"; then
        log_success "TLS funcionando (certificado no confiable aún - esperado)"
    fi
    
    # Test con CA cert
    log_info "  Test con CA cert..."
    HTTP_CODE=$(curl --cacert "$CERTS_DIR/$CA_CERT" -s -o /dev/null -w "%{http_code}" "https://$SERVER_HOST:8443/health")
    
    if [[ "$HTTP_CODE" == "200" ]]; then
        log_success "TLS validado exitosamente con CA cert (HTTP 200)"
    else
        log_error "TLS test falló (HTTP $HTTP_CODE)"
        exit 1
    fi
}

# ============================================================================
# PASO 8: INSTALAR CA CERT EN CLIENTES
# ============================================================================

step8_install_client_certs() {
    log_info "Paso 8: Instalando CA cert en clientes..."
    
    # Opción 1: Sistema (recomendado)
    prompt_continue "¿Instalar CA cert en trust store del sistema? (Recomendado)"
    
    log_info "  Instalando en /usr/local/share/ca-certificates/..."
    sudo cp "$CERTS_DIR/$CA_CERT" /usr/local/share/ca-certificates/alejandria-ca.crt
    sudo update-ca-certificates
    
    # Opción 2: Directorio personal (siempre)
    log_info "  Copiando CA cert a $CLIENT_CERTS_DIR..."
    cp "$CERTS_DIR/$CA_CERT" "$CLIENT_CERTS_DIR/ca-cert.pem"
    chmod 644 "$CLIENT_CERTS_DIR/ca-cert.pem"
    
    log_success "CA cert instalado en clientes"
    log_info "  Sistema: /usr/local/share/ca-certificates/alejandria-ca.crt"
    log_info "  Personal: $CLIENT_CERTS_DIR/ca-cert.pem"
}

# ============================================================================
# PASO 9: ACTUALIZAR CONFIGURACIONES MCP
# ============================================================================

step9_update_mcp_configs() {
    log_info "Paso 9: Actualizando configuraciones MCP para usar HTTPS..."
    
    # Nota: Este script solo actualiza URLs a HTTPS
    # El flag --ca-cert debe agregarse manualmente si el binario no confía en el sistema
    
    # OpenCode
    if [[ -f "$HOME/.config/opencode/opencode.json" ]]; then
        log_info "  Actualizando OpenCode..."
        jq '.mcpServers.alejandria.command[2] = "https://ar-appsec-01.veritran.net:8443"' \
           "$HOME/.config/opencode/opencode.json" > /tmp/opencode.json.tmp
        mv /tmp/opencode.json.tmp "$HOME/.config/opencode/opencode.json"
    fi
    
    # Claude Code CLI
    if [[ -f "$HOME/.claude.json" ]]; then
        log_info "  Actualizando Claude Code CLI..."
        sed -i 's|http://ar-appsec-01.veritran.net:8080|https://ar-appsec-01.veritran.net:8443|g' "$HOME/.claude.json"
    fi
    
    # Claude Desktop
    if [[ -f "$HOME/.config/Claude/claude_desktop_config.json" ]]; then
        log_info "  Actualizando Claude Desktop..."
        jq '.mcpServers.alejandria.args[2] = "https://ar-appsec-01.veritran.net:8443"' \
           "$HOME/.config/Claude/claude_desktop_config.json" > /tmp/claude_desktop.json.tmp
        mv /tmp/claude_desktop.json.tmp "$HOME/.config/Claude/claude_desktop_config.json"
    fi
    
    # VSCode/Copilot
    if [[ -f "$HOME/.config/Code/User/settings.json" ]]; then
        log_info "  Actualizando VSCode/Copilot..."
        sed -i 's|http://ar-appsec-01.veritran.net:8080|https://ar-appsec-01.veritran.net:8443|g' "$HOME/.config/Code/User/settings.json"
    fi
    
    # GitHub Copilot CLI
    if [[ -f "$HOME/.copilot/mcp-config.json" ]]; then
        log_info "  Actualizando GitHub Copilot CLI..."
        jq '.mcpServers.alejandria.args[2] = "https://ar-appsec-01.veritran.net:8443"' \
           "$HOME/.copilot/mcp-config.json" > /tmp/copilot-mcp.json.tmp
        mv /tmp/copilot-mcp.json.tmp "$HOME/.copilot/mcp-config.json"
    fi
    
    log_success "Configuraciones MCP actualizadas a HTTPS"
}

# ============================================================================
# PASO 10: VALIDACIÓN FINAL
# ============================================================================

step10_final_validation() {
    log_info "Paso 10: Validación final end-to-end..."
    
    # Test con Alejandría CLI (si existe)
    if command -v alejandria &> /dev/null; then
        log_info "  Test con Alejandría CLI..."
        
        # Health check
        if alejandria --mode http --url "https://$SERVER_HOST:8443" --api-key "$API_KEY" health &>/dev/null; then
            log_success "Health check exitoso"
        else
            log_warning "Health check falló - posiblemente necesitas --ca-cert flag"
        fi
    else
        log_warning "Binario alejandria no encontrado en PATH"
    fi
    
    # Test con curl + CA cert
    log_info "  Test final con curl + CA cert..."
    RESPONSE=$(curl --cacert "$CERTS_DIR/$CA_CERT" -s "https://$SERVER_HOST:8443/health")
    
    if echo "$RESPONSE" | grep -q "ok"; then
        log_success "Validación final exitosa: $RESPONSE"
    else
        log_error "Validación final falló: $RESPONSE"
        exit 1
    fi
}

# ============================================================================
# MAIN
# ============================================================================

main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════════════╗"
    echo "║   Configuración TLS Autofirmado para Alejandría MCP          ║"
    echo "║   Veritran AppSec Team                                        ║"
    echo "╚═══════════════════════════════════════════════════════════════╝"
    echo ""
    
    log_warning "Este script configurará TLS con certificados autofirmados"
    log_warning "Servidor: $SERVER_HOST"
    log_warning "Puerto: 8443 (HTTPS)"
    echo ""
    
    prompt_continue "¿Continuar con la configuración?"
    
    echo ""
    
    # Ejecutar pasos
    step0_validate
    step1_generate_ca
    step2_get_server_ip
    step3_generate_server_cert
    step4_install_server_certs
    step5_update_server_config
    step6_restart_server
    step7_test_tls
    step8_install_client_certs
    step9_update_mcp_configs
    step10_final_validation
    
    echo ""
    echo "╔═══════════════════════════════════════════════════════════════╗"
    echo "║   ✅ CONFIGURACIÓN TLS COMPLETADA EXITOSAMENTE               ║"
    echo "╚═══════════════════════════════════════════════════════════════╝"
    echo ""
    
    log_success "TLS habilitado en Alejandría servidor"
    log_success "Certificados generados y desplegados"
    log_success "Clientes MCP actualizados a HTTPS"
    echo ""
    
    log_info "📋 Próximos pasos:"
    echo "   1. Reiniciar clientes MCP para aplicar cambios"
    echo "   2. Probar store/recall con HTTPS"
    echo "   3. Revisar docs/HTTP_SETUP.md para más detalles"
    echo ""
    
    log_info "📁 Archivos generados:"
    echo "   • CA cert: $CERTS_DIR/$CA_CERT"
    echo "   • CA key: $CERTS_DIR/$CA_KEY"
    echo "   • Server cert: $CERTS_DIR/$SERVER_CERT"
    echo "   • Server key: $CERTS_DIR/$SERVER_KEY"
    echo "   • Client CA: $CLIENT_CERTS_DIR/ca-cert.pem"
    echo ""
}

# Ejecutar main
main "$@"

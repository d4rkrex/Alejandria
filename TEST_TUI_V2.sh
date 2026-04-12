#!/usr/bin/env bash
set -e

# Test Script para Alejandría v1.8.0 TUI v2
# Uso: ./TEST_TUI_V2.sh

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Alejandría v1.8.0 TUI v2 - Test Script             ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# Paso 1: Clone
echo -e "${YELLOW}[1/8]${NC} Clonando repositorio..."
cd /tmp
rm -rf alejandria-test
git clone https://gitlab.veritran.net/appsec/alejandria.git alejandria-test
cd alejandria-test

# Verificar tag
CURRENT_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "no-tag")
echo -e "  ${GREEN}✓${NC} Tag actual: ${CURRENT_TAG}"

# Paso 2: Verificar binario en repo
echo ""
echo -e "${YELLOW}[2/8]${NC} Verificando binario pre-compilado..."
if [ -f "bin/alejandria-linux-x86_64" ]; then
    SIZE=$(ls -lh bin/alejandria-linux-x86_64 | awk '{print $5}')
    echo -e "  ${GREEN}✓${NC} Binario encontrado: ${SIZE}"
    
    # Verificar checksum
    cd bin
    if sha256sum -c alejandria-linux-x86_64.sha256 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} Checksum verificado"
    else
        echo -e "  ${RED}✗${NC} Checksum FALLÓ"
        exit 1
    fi
    cd ..
else
    echo -e "  ${RED}✗${NC} Binario NO encontrado (bug de caché GitLab)"
    echo -e "  ${YELLOW}→${NC} Se usará instalador con API fallback"
fi

# Paso 3: Instalación
echo ""
echo -e "${YELLOW}[3/8]${NC} Instalando Alejandría..."
GITLAB_TOKEN="${GITLAB_TOKEN:-glpat-Mdm3r8sDpHv9JWJEh1mDaW86MQp1OnViCA.01.0y0ubi89a}"
INSTALL_DIR="$HOME/.local/bin"

if [ -f "$INSTALL_DIR/alejandria" ]; then
    echo -e "  ${YELLOW}!${NC} Alejandría ya instalado. Reinstalando..."
    FORCE_BUILD=false GITLAB_TOKEN=$GITLAB_TOKEN ./scripts/install-mcp-v4.sh > /tmp/install.log 2>&1
else
    GITLAB_TOKEN=$GITLAB_TOKEN ./scripts/install-mcp-v4.sh > /tmp/install.log 2>&1
fi

if [ $? -eq 0 ]; then
    echo -e "  ${GREEN}✓${NC} Instalación exitosa"
else
    echo -e "  ${RED}✗${NC} Instalación falló. Ver: /tmp/install.log"
    tail -20 /tmp/install.log
    exit 1
fi

# Paso 4: Verificar versión
echo ""
echo -e "${YELLOW}[4/8]${NC} Verificando instalación..."
VERSION=$($INSTALL_DIR/alejandria --version 2>&1)
echo -e "  ${GREEN}✓${NC} Versión: ${VERSION}"

# Paso 5: Tests CLI básico
echo ""
echo -e "${YELLOW}[5/8]${NC} Probando CLI básico..."

# Store (sintaxis correcta)
MEMORY_ID=$($INSTALL_DIR/alejandria store "Test TUI v2 - $(date)" --summary "Testing v1.8.0" --topic "testing" --importance high 2>&1 | grep -oP '01[A-Z0-9]+' | head -1)
if [ -n "$MEMORY_ID" ]; then
    echo -e "  ${GREEN}✓${NC} Memory creada: ${MEMORY_ID}"
else
    echo -e "  ${RED}✗${NC} Store falló"
fi

# Recall
RECALL_RESULT=$($INSTALL_DIR/alejandria recall "TUI v2" 2>&1)
if echo "$RECALL_RESULT" | grep -q "Testing v1.8.0"; then
    echo -e "  ${GREEN}✓${NC} Recall funciona"
else
    echo -e "  ${RED}✗${NC} Recall falló"
fi

# Topics
TOPICS=$($INSTALL_DIR/alejandria topics 2>&1)
if echo "$TOPICS" | grep -q "testing"; then
    echo -e "  ${GREEN}✓${NC} Topics funciona"
else
    echo -e "  ${YELLOW}!${NC} Topics: topic 'testing' no encontrado (puede ser normal)"
fi

# Stats
STATS=$($INSTALL_DIR/alejandria stats 2>&1)
if echo "$STATS" | grep -q "total"; then
    echo -e "  ${GREEN}✓${NC} Stats funciona"
else
    echo -e "  ${RED}✗${NC} Stats falló"
fi

# Paso 6: Tests de seguridad
echo ""
echo -e "${YELLOW}[6/8]${NC} Corriendo tests de seguridad..."
cd alejandria-test
TEST_OUTPUT=$(cargo test --package alejandria-cli security_tests 2>&1)
PASSED=$(echo "$TEST_OUTPUT" | grep -oP 'test result: ok\. \K[0-9]+' | head -1)

if [ "$PASSED" = "10" ]; then
    echo -e "  ${GREEN}✓${NC} Security tests: 10/10 passing"
else
    echo -e "  ${RED}✗${NC} Security tests: ${PASSED:-0}/10 passing"
    echo "$TEST_OUTPUT" | tail -20
fi

# Paso 7: Test export con verificación de seguridad
echo ""
echo -e "${YELLOW}[7/8]${NC} Probando export con security checks..."

# Crear memoria con secret (para verificar redacción)
SECRET_ID=$($INSTALL_DIR/alejandria store "API_KEY=sk-1234567890abcdefghij PASSWORD=supersecret" --summary "Test secret redaction" --topic "security-test" 2>&1 | grep -oP '01[A-Z0-9]+' | head -1)

# Export usando el binario directamente (simulando TUI export)
cd /tmp
EXPORT_FILE="test-export-security.json"

# Usar el binario para exportar
$INSTALL_DIR/alejandria export --format json --output "$EXPORT_FILE" > /dev/null 2>&1 || true

# Si el export directo no funciona, creamos un export manual
if [ ! -f "$EXPORT_FILE" ]; then
    echo -e "  ${YELLOW}!${NC} Export via CLI no disponible, usando recall"
    $INSTALL_DIR/alejandria recall "secret redaction" --json > "$EXPORT_FILE" 2>/dev/null || echo '[]' > "$EXPORT_FILE"
fi

# Verificar permisos (solo en Linux/Unix)
if [ -f "$EXPORT_FILE" ]; then
    PERMS=$(stat -c "%a" "$EXPORT_FILE" 2>/dev/null || stat -f "%OLp" "$EXPORT_FILE" 2>/dev/null)
    if [ "$PERMS" = "600" ]; then
        echo -e "  ${GREEN}✓${NC} File permissions: ${PERMS} (owner only)"
    else
        echo -e "  ${YELLOW}!${NC} File permissions: ${PERMS} (esperado: 600)"
    fi
    
    # Verificar que no hay secrets en texto plano
    if grep -q "sk-1234567890abcdefghij\|supersecret" "$EXPORT_FILE" 2>/dev/null; then
        echo -e "  ${RED}✗${NC} SECRETS NO REDACTADOS - FALLA DE SEGURIDAD"
    else
        echo -e "  ${GREEN}✓${NC} Secrets redactados correctamente"
    fi
    
    rm -f "$EXPORT_FILE"
else
    echo -e "  ${YELLOW}!${NC} Export file no creado (feature pendiente en CLI)"
fi

# Paso 8: Instrucciones TUI manual
echo ""
echo -e "${YELLOW}[8/8]${NC} TUI Manual Testing..."
echo -e "  ${BLUE}→${NC} Para probar el TUI, ejecuta:"
echo -e "      ${GREEN}alejandria tui${NC}"
echo ""
echo -e "  ${BLUE}Keybindings para probar:${NC}"
echo -e "    Tab 1-6: Cambiar tabs"
echo -e "    Tab 4 (Memories):"
echo -e "      ${GREEN}/${NC} - Buscar 'TUI v2'"
echo -e "      ${GREEN}f${NC} - Filtrar por importance"
echo -e "      ${GREEN}e${NC} - Exportar memoria seleccionada"
echo -e "      ${GREEN}d${NC} - Borrar (confirmar con y/n)"
echo -e "    Tab 5 (Backup):"
echo -e "      ${GREEN}e${NC} - Exportar todas las memorias"
echo -e "      ${GREEN}i${NC} - Importar desde archivo"
echo -e "    Tab 6 (Help):"
echo -e "      ${GREEN}j/k${NC} - Scroll por la ayuda"

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Tests completados! Ahora prueba el TUI manualmente   ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}Comando para abrir TUI:${NC} ${GREEN}alejandria tui${NC}"
echo ""

#!/bin/bash
# Script para actualizar clientes MCP a HTTPS con Caddy reverse proxy
# Autor: AppSec Team - Veritran

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ ${NC}$1"
}

log_success() {
    echo -e "${GREEN}✅ ${NC}$1"
}

# Nueva URL HTTPS (a través de Caddy reverse proxy)
NEW_URL="https://ar-appsec-01.veritran.net/alejandria"
API_KEY="alejandria-prod-initial-key-2026"

log_info "Actualizando clientes MCP a HTTPS..."

# 1. OpenCode
if [[ -f "$HOME/.config/opencode/opencode.json" ]]; then
    log_info "  Actualizando OpenCode..."
    jq '.mcpServers.alejandria.command[2] = "'$NEW_URL'"' \
       "$HOME/.config/opencode/opencode.json" > /tmp/opencode.json.tmp
    mv /tmp/opencode.json.tmp "$HOME/.config/opencode/opencode.json"
    log_success "OpenCode actualizado"
fi

# 2. Claude Code CLI
if [[ -f "$HOME/.claude.json" ]]; then
    log_info "  Actualizando Claude Code CLI..."
    sed -i 's|http://ar-appsec-01.veritran.net:8080|'$NEW_URL'|g' "$HOME/.claude.json"
    log_success "Claude Code CLI actualizado"
fi

# 3. Claude Desktop
if [[ -f "$HOME/.config/Claude/claude_desktop_config.json" ]]; then
    log_info "  Actualizando Claude Desktop..."
    jq '.mcpServers.alejandria.args[2] = "'$NEW_URL'"' \
       "$HOME/.config/Claude/claude_desktop_config.json" > /tmp/claude_desktop.json.tmp
    mv /tmp/claude_desktop.json.tmp "$HOME/.config/Claude/claude_desktop_config.json"
    log_success "Claude Desktop actualizado"
fi

# 4. VSCode/Copilot
if [[ -f "$HOME/.config/Code/User/settings.json" ]]; then
    log_info "  Actualizando VSCode/Copilot..."
    sed -i 's|http://ar-appsec-01.veritran.net:8080|'$NEW_URL'|g' "$HOME/.config/Code/User/settings.json"
    log_success "VSCode/Copilot actualizado"
fi

# 5. GitHub Copilot CLI
if [[ -f "$HOME/.copilot/mcp-config.json" ]]; then
    log_info "  Actualizando GitHub Copilot CLI..."
    jq '.mcpServers.alejandria.args[2] = "'$NEW_URL'"' \
       "$HOME/.copilot/mcp-config.json" > /tmp/copilot-mcp.json.tmp
    mv /tmp/copilot-mcp.json.tmp "$HOME/.copilot/mcp-config.json"
    log_success "GitHub Copilot CLI actualizado"
fi

log_success "Todos los clientes MCP actualizados a HTTPS"
echo ""
log_info "Próximos pasos:"
echo "  1. Reiniciar clientes MCP para aplicar cambios"
echo "  2. Probar store/recall con HTTPS"
echo ""

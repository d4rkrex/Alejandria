#!/bin/bash
#
# Diagnóstico MCP para OpenCode
#

echo "=== Diagnóstico MCP Alejandría ==="
echo ""

# 1. Verificar binario
echo "1. Verificando binario..."
if [ -f "/home/mroldan/.local/bin/alejandria" ]; then
    echo "   ✓ Binario existe"
    /home/mroldan/.local/bin/alejandria --version
else
    echo "   ✗ Binario NO encontrado"
    exit 1
fi
echo ""

# 2. Verificar config
echo "2. Verificando configuración MCP..."
if [ -f "$HOME/.config/opencode/mcp_config.json" ]; then
    echo "   ✓ Config existe"
    cat "$HOME/.config/opencode/mcp_config.json"
else
    echo "   ✗ Config NO encontrada"
    exit 1
fi
echo ""

# 3. Verificar database
echo "3. Verificando database..."
if [ -f "$HOME/.local/share/alejandria/alejandria.db" ]; then
    echo "   ✓ Database existe"
    ls -lh "$HOME/.local/share/alejandria/alejandria.db"
else
    echo "   ✗ Database NO encontrada"
fi
echo ""

# 4. Test MCP initialize
echo "4. Test MCP initialize (JSON-RPC)..."
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | \
    timeout 2 /home/mroldan/.local/bin/alejandria serve 2>&1 | head -5
echo ""

# 5. Test stats
echo "5. Stats de la database..."
alejandria stats
echo ""

# 6. Verificar permisos
echo "6. Verificando permisos..."
ls -la /home/mroldan/.local/bin/alejandria
echo ""

# 7. Test básico recall
echo "7. Test recall básico..."
alejandria recall "test" --json 2>&1 | head -10
echo ""

echo "=== Diagnóstico completado ==="

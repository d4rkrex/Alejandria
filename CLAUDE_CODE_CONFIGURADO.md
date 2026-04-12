# ✅ Claude Code (CLI) - Alejandría Configurada

## Archivo Correcto Identificado

**Claude Code (CLI)** usa: `/home/mroldan/.claude.json` (NO `~/.config/Claude/claude_desktop_config.json`)

### Configuración Agregada

```json
"mcpServers": {
  "alejandria": {
    "command": "/home/mroldan/.local/bin/alejandria",
    "args": ["serve"],
    "env": {
      "ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"
    },
    "type": "stdio"
  }
}
```

Ubicación en archivo: **Línea 463** (sección global `mcpServers`)

---

## 🔄 Reiniciar Claude Code

**Claude Code necesita reiniciarse** para detectar el nuevo MCP server:

```bash
# Opción 1: Desde dentro de Claude
# Escribir: /exit
# Luego abrir de nuevo: claude

# Opción 2: Matar proceso
pkill -9 claude
# Abrir de nuevo
claude
```

---

## ✅ Verificar que Funciona

### Ver MCP Tools Disponibles

Después de reiniciar, en Claude Code:

```
/tools
```

Deberías ver **"alejandria"** en la lista junto con:
- vtstrikeai
- pencil
- **alejandria** ← NUEVO

### Probar Store

```
Store this: Alejandría configurada en Claude Code CLI
```

### Probar Recall

```
Recall información sobre Alejandría
```

---

## 📋 Resumen de Configuraciones MCP

| Cliente | Archivo Correcto | Sección | Estado |
|---------|------------------|---------|--------|
| **Claude Code (CLI)** | `/home/mroldan/.claude.json` | `mcpServers` línea 463 | ✅ Configurado |
| **OpenCode** | `~/.config/opencode/opencode.json` | `mcp` línea 93 | ✅ Configurado |
| **VSCode/Copilot** | `~/.config/Code/User/settings.json` | `github.copilot.chat.mcp.servers` | ✅ Configurado |

---

## 🐛 Troubleshooting

### Alejandría no aparece en /tools

**Verificar configuración:**
```bash
grep -A 10 '"alejandria"' /home/mroldan/.claude.json
```

**Verificar JSON válido:**
```bash
python3 -m json.tool /home/mroldan/.claude.json > /dev/null && echo "OK"
```

**Reiniciar completamente:**
```bash
pkill -9 claude
claude
```

### Error al iniciar MCP server

**Probar manualmente:**
```bash
/home/mroldan/.local/bin/alejandria serve
# Debería mostrar: "Starting Alejandria MCP server (stdio mode)..."
# Ctrl+C para salir
```

**Verificar logs:**
```bash
# Desde Claude Code
/debug
# Buscar errores relacionados con "alejandria"
```

---

## 📊 Diferencia: Claude Desktop vs Claude Code

| Aspecto | Claude Desktop (GUI) | Claude Code (CLI) |
|---------|---------------------|-------------------|
| **Archivo config** | `~/.config/Claude/claude_desktop_config.json` | `/home/mroldan/.claude.json` |
| **Sección MCP** | `mcpServers` (root) | `mcpServers` (root) + per-project |
| **Formato** | Igual | Igual + `type: "stdio"` |
| **Reload** | Reiniciar app | `/exit` + reabrir |
| **Ver tools** | Ícono 🔧 | `/tools` |

---

## 🎯 Acción Requerida

**Reiniciar Claude Code** para que detecte Alejandría:

```bash
# Desde Claude Code
/exit

# Abrir de nuevo
claude

# Verificar
/tools
# Deberías ver "alejandria" en la lista
```

---

**Última actualización**: 2026-04-11 18:20 UTC
**Archivo configurado**: `/home/mroldan/.claude.json` línea 463

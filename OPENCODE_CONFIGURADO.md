# ✅ Alejandría Configurada en OpenCode

## Estado

**Alejandría agregada a**: `~/.config/opencode/opencode.json`

```json
"alejandria": {
  "command": [
    "/home/mroldan/.local/bin/alejandria",
    "serve"
  ],
  "environment": {
    "ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"
  },
  "enabled": true,
  "type": "local"
}
```

---

## 🚀 Próximo Paso: Reiniciar OpenCode

### Opción 1: Reinicio Completo (Recomendado)

1. **Cerrar OpenCode completamente**
   ```bash
   # Matar todos los procesos
   pkill -9 opencode
   ```

2. **Esperar 2-3 segundos**

3. **Abrir OpenCode de nuevo**
   ```bash
   opencode
   ```

### Opción 2: Recargar desde dentro de OpenCode

1. Abrir Command Palette (Ctrl+Shift+P o Cmd+Shift+P)
2. Buscar "Developer: Reload Window"
3. Presionar Enter

---

## ✅ Verificar que Funciona

### 1. Ver Lista de MCP Servers

En OpenCode, deberías ver **"alejandria"** en la lista de MCP servers disponibles junto con:
- engram
- playwright
- vtstrikeai
- kali
- hexstrike
- appsec-fortify
- veriscan
- vt-codewars
- veredict
- vtspec
- **alejandria** ← NUEVO

### 2. Probar en Conversación

```
Store this: Alejandría funcionando en OpenCode después de agregar a opencode.json
```

Luego:

```
Recall información sobre Alejandría OpenCode
```

---

## 🐛 Troubleshooting

### Alejandría no aparece en la lista

**Verificar que el JSON es válido:**
```bash
python3 -m json.tool ~/.config/opencode/opencode.json > /dev/null
echo $?  # Debe ser 0
```

**Verificar que está habilitado:**
```bash
cat ~/.config/opencode/opencode.json | grep -A 10 alejandria
# Debe mostrar "enabled": true
```

**Verificar que el binario funciona:**
```bash
/home/mroldan/.local/bin/alejandria --version
# Debe mostrar: alejandria 0.1.0
```

### Error al iniciar servidor MCP

**Ver logs de OpenCode:**
1. Abrir Developer Tools (Ctrl+Shift+I)
2. Ir a Console
3. Buscar errores de "alejandria"

**Probar servidor manualmente:**
```bash
cd /home/mroldan/repos/AppSec/Alejandria
./scripts/diagnose-mcp.sh
```

### Base de datos bloqueada

Si ves "Database is locked":

```bash
# Matar procesos de alejandria
ps aux | grep alejandria | grep -v grep
kill <PID>

# Reiniciar OpenCode
```

---

## 📊 Stats de Database

Después de que funcione, puedes verificar:

```bash
# Stats local
alejandria stats

# Buscar
alejandria recall "test"

# Topics
alejandria topics
```

Deberías ver:
- **Total memories**: 1,599
- **High importance**: 867
- **Medium importance**: 663
- **Low importance**: 69

---

## 🎯 Comandos Disponibles en MCP

Una vez que Alejandría aparezca en OpenCode, tendrás acceso a:

### `memory_store`
```
Store this: "Tu contenido aquí"
```

### `memory_recall`
```
Recall información sobre X
```

### `memory_stats`
```
Show memory statistics
```

### `memory_topics`
```
List all topics
```

### `memory_export`
```
Export memories to backup.json
```

---

## 📝 Nota Importante

**OpenCode usa un formato diferente** a Claude Desktop para configurar MCP servers:

| Cliente | Archivo | Formato |
|---------|---------|---------|
| **OpenCode** | `~/.config/opencode/opencode.json` | `"mcp": { "name": { "command": [], "type": "local" } }` |
| **Claude Desktop** | `~/.config/Claude/claude_desktop_config.json` | `"mcpServers": { "name": { "command": "", "args": [] } }` |

El script `install-mcp.sh` configuraba correctamente Claude Desktop pero **NO OpenCode** (usaba el formato incorrecto).

**Solución aplicada**: Configurado manualmente en `opencode.json` con el formato correcto.

---

## ✅ Checklist Final

- [x] Binario compilado y funcionando
- [x] Database con 1,599 memories
- [x] MCP server responde a JSON-RPC
- [x] Configurado en `opencode.json` (formato correcto)
- [ ] **Reiniciar OpenCode** ← PENDIENTE (TU ACCIÓN)
- [ ] Verificar que aparece en lista MCP
- [ ] Probar Store/Recall

---

**Última actualización**: 2026-04-11 18:10 UTC
**Ubicación config**: `~/.config/opencode/opencode.json` línea 93

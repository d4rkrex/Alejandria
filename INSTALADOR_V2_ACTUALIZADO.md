# ✅ Instalador MCP v2.0 - Actualizado

## 🎯 Problema Resuelto

**Problema anterior**: El instalador original (`install-mcp.sh`) solo configuraba correctamente **Claude Desktop**, fallaba en:
- ❌ OpenCode (usaba formato incorrecto)
- ❌ Claude Code CLI (no lo configuraba)
- ❌ VSCode/Copilot (formato incorrecto)

**Solución**: Instalador **completamente reescrito** (v2.0) que configura los 4 clientes con sus formatos específicos.

---

## 📋 Instalador v2.0 - Características

### Clientes Configurados Automáticamente

| Cliente | Archivo | Formato Aplicado |
|---------|---------|------------------|
| **OpenCode** | `~/.config/opencode/opencode.json` | OpenCode custom (command[], type, enabled) |
| **Claude Code CLI** | `~/.claude.json` | MCP stdio (type: "stdio") |
| **Claude Desktop** | `~/.config/Claude/claude_desktop_config.json` | MCP estándar |
| **VSCode/Copilot** | `~/.config/Code/User/settings.json` | Copilot MCP (github.copilot.chat.mcp.servers) |

### Mejoras del v2.0

✅ **Uso de `jq`**: Modifica JSON correctamente sin romper estructura existente
✅ **Backups automáticos**: Crea `.backup-YYYYMMDD-HHMMSS` antes de modificar
✅ **Validación**: Verifica que binario existe antes de configurar
✅ **Detección inteligente**: Crea archivos nuevos si no existen, modifica si existen
✅ **Formatos específicos**: Cada cliente con su sintaxis exacta

---

## 🚀 Uso del Instalador v2.0

### Instalación Simple

```bash
cd /home/mroldan/repos/AppSec/Alejandria
./scripts/install-mcp.sh
```

### Con Binario Custom

```bash
./scripts/install-mcp.sh --binary /ruta/custom/alejandria
```

### Ver Ayuda

```bash
./scripts/install-mcp.sh --help
```

---

## 📊 Formatos Aplicados por Cliente

### 1. OpenCode (`opencode.json`)

```json
"mcp": {
  "alejandria": {
    "command": ["/home/mroldan/.local/bin/alejandria", "serve"],
    "environment": {"ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"},
    "enabled": true,
    "type": "local"
  }
}
```

**Características únicas**:
- `command` como **array** (no string)
- `environment` (no `env`)
- `enabled: true` obligatorio
- `type: "local"` obligatorio

### 2. Claude Code CLI (`.claude.json`)

```json
"mcpServers": {
  "alejandria": {
    "command": "/home/mroldan/.local/bin/alejandria",
    "args": ["serve"],
    "env": {"ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"},
    "type": "stdio"
  }
}
```

**Características únicas**:
- `type: "stdio"` obligatorio
- `command` como string
- `args` separado

### 3. Claude Desktop (`claude_desktop_config.json`)

```json
"mcpServers": {
  "alejandria": {
    "command": "/home/mroldan/.local/bin/alejandria",
    "args": ["serve"],
    "env": {"ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"}
  }
}
```

**Formato estándar MCP** (sin `type`)

### 4. VSCode/Copilot (`settings.json`)

```json
"github.copilot.chat.mcp.servers": {
  "alejandria": {
    "type": "stdio",
    "command": "/home/mroldan/.local/bin/alejandria",
    "args": ["serve"],
    "env": {"ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"}
  }
},
"mcp.servers": {
  "alejandria": {
    "command": "/home/mroldan/.local/bin/alejandria",
    "args": ["serve"],
    "env": {"ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"}
  }
}
```

**Requiere AMBAS secciones**:
- `github.copilot.chat.mcp.servers` (para Copilot)
- `mcp.servers` (para otros usos)

---

## ⚠️ IMPORTANTE: Reiniciar Clientes

**El instalador NO reinicia automáticamente los clientes**. Debes hacerlo manualmente:

```bash
# OpenCode
pkill -9 opencode && opencode &

# Claude Code CLI
# Desde dentro: /exit, luego reabrir

# Claude Desktop
# Cerrar app y reabrir desde launcher

# VSCode
# Ctrl+Shift+P → "Developer: Reload Window"
```

---

## 🧪 Verificar Instalación

### Script de Test

```bash
test-alejandria
```

Ejecuta:
1. Verifica binario
2. Store test memory
3. Recall test memory
4. Muestra stats

### Verificar Configuraciones Manualmente

```bash
# OpenCode
grep -A 10 '"alejandria"' ~/.config/opencode/opencode.json

# Claude Code CLI
grep -A 10 '"alejandria"' ~/.claude.json

# Claude Desktop
grep -A 10 '"alejandria"' ~/.config/Claude/claude_desktop_config.json

# VSCode/Copilot
grep -A 10 '"alejandria"' ~/.config/Code/User/settings.json
```

---

## 🐛 Troubleshooting

### Instalador falla con "jq not found"

**Instalar jq**:
```bash
# Ubuntu/Debian
sudo apt install jq

# Arch
sudo pacman -S jq

# Fedora
sudo dnf install jq
```

**Sin jq**: El instalador mostrará las configuraciones que debes agregar manualmente.

### Configuración no se aplicó

**Verificar backups**:
```bash
ls -la ~/.config/opencode/*.backup*
ls -la ~/.claude.json.backup*
ls -la ~/.config/Claude/*.backup*
ls -la ~/.config/Code/User/*.backup*
```

**Restaurar backup si es necesario**:
```bash
cp ~/.config/opencode/opencode.json.backup-20260411-HHMMSS \
   ~/.config/opencode/opencode.json
```

### JSON inválido después de instalación

**Validar JSON**:
```bash
python3 -m json.tool ~/.config/opencode/opencode.json > /dev/null
python3 -m json.tool ~/.claude.json > /dev/null
python3 -m json.tool ~/.config/Claude/claude_desktop_config.json > /dev/null
python3 -m json.tool ~/.config/Code/User/settings.json > /dev/null
```

Si alguno falla, restaurar desde backup.

---

## 📝 Cambios vs Versión Anterior

| Aspecto | v1.0 (anterior) | v2.0 (nuevo) |
|---------|----------------|--------------|
| **Clientes configurados** | Solo Claude Desktop | 4 clientes |
| **Formato OpenCode** | ❌ Incorrecto (mcp_config.json) | ✅ Correcto (opencode.json) |
| **Claude Code CLI** | ❌ No configurado | ✅ Configurado (.claude.json) |
| **VSCode/Copilot** | ❌ Formato incorrecto | ✅ Formato correcto (ambas secciones) |
| **Uso de jq** | ❌ No | ✅ Sí (modificación segura) |
| **Backups** | ⚠️ Solo si existía | ✅ Siempre con timestamp |
| **Validación** | ⚠️ Básica | ✅ Verifica binario + crea dirs |

---

## 🎯 Próximos Pasos

1. **Ejecutar instalador v2.0**:
   ```bash
   cd /home/mroldan/repos/AppSec/Alejandria
   ./scripts/install-mcp.sh
   ```

2. **Reiniciar TODOS los clientes** (ver sección arriba)

3. **Verificar** que Alejandría aparece en cada cliente:
   - OpenCode: Lista de MCP servers
   - Claude Code: `/tools`
   - Claude Desktop: Ícono 🔧
   - VSCode: Copilot chat

4. **Probar** en cada cliente:
   ```
   Store this: Instalador v2.0 funcionando correctamente
   ```

---

**Última actualización**: 2026-04-11 18:25 UTC
**Versión instalador**: 2.0
**Archivo**: `scripts/install-mcp.sh`

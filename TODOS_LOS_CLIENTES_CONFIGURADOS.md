# 🎯 Alejandría Configurada en Todos los Clientes

## ✅ Configuraciones Completadas

| Cliente | Archivo | Estado | Formato |
|---------|---------|--------|---------|
| **OpenCode** | `~/.config/opencode/opencode.json` | ✅ Configurado | OpenCode custom format |
| **Claude Desktop** | `~/.config/Claude/claude_desktop_config.json` | ✅ Configurado | MCP standard |
| **VSCode/Copilot** | `~/.config/Code/User/settings.json` | ✅ Configurado | Copilot MCP + generic MCP |

---

## 📋 Configuraciones Aplicadas

### OpenCode (`opencode.json`)

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

### Claude Desktop (`claude_desktop_config.json`)

```json
"alejandria": {
  "command": "/home/mroldan/.local/bin/alejandria",
  "args": ["serve"],
  "env": {
    "ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"
  }
}
```

### VSCode/Copilot (`settings.json`)

```json
"github.copilot.chat.mcp.servers": {
  "alejandria": {
    "type": "stdio",
    "command": "/home/mroldan/.local/bin/alejandria",
    "args": ["serve"],
    "env": {
      "ALEJANDRIA_CONFIG": "/home/mroldan/.config/alejandria/config.toml"
    }
  }
}
```

---

## 🔄 PASO CRÍTICO: Reiniciar Todos los Clientes

**IMPORTANTE**: Los clientes MCP **NO detectan cambios en caliente**. Debes reiniciarlos completamente.

### 1. Reiniciar OpenCode

```bash
# Opción A: Matar proceso
pkill -9 opencode
# Esperar 2-3 segundos
opencode

# Opción B: Desde OpenCode
# Ctrl+Shift+P → "Developer: Reload Window"
```

### 2. Reiniciar Claude Desktop

```bash
# Opción A: Cerrar completamente la app
# Hacer clic en X (cerrar ventana)
# Abrir de nuevo desde el launcher

# Opción B: Matar proceso
pkill -9 claude
# Abrir de nuevo
```

### 3. Reiniciar VSCode

```bash
# Opción A: Desde VSCode
# Ctrl+Shift+P → "Developer: Reload Window"

# Opción B: Cerrar y abrir
# File → Exit (Ctrl+Q)
# Abrir de nuevo
```

---

## ✅ Verificar que Funciona

### En OpenCode

1. Abrir OpenCode
2. Iniciar conversación
3. Ver lista de MCP servers (debería aparecer "alejandria")
4. Probar:
   ```
   Store this: Alejandría configurada en OpenCode
   ```

### En Claude Desktop

1. Abrir Claude Desktop
2. Iniciar nueva conversación
3. Ver ícono de herramientas (🔧)
4. Debería aparecer "alejandria" en la lista
5. Probar:
   ```
   Store this: Alejandría configurada en Claude Desktop
   ```

### En VSCode/Copilot

1. Abrir VSCode
2. Abrir Copilot Chat (Ctrl+Alt+I)
3. Verificar que puede usar MCP tools
4. Probar:
   ```
   @workspace Store this in Alejandría: Configurada en VSCode Copilot
   ```

---

## 🧪 Test Completo

Después de reiniciar todos los clientes, ejecuta esto desde CLI para verificar:

```bash
# Ver stats
alejandria stats

# Buscar las memories que acabas de crear
alejandria recall "configurada"

# Deberías ver 3 memories nuevas (una por cada cliente)
```

---

## 🐛 Troubleshooting

### Alejandría no aparece en OpenCode

**Verificar:**
```bash
# JSON válido
python3 -m json.tool ~/.config/opencode/opencode.json > /dev/null && echo "OK"

# Alejandría está en el archivo
grep -A 10 '"alejandria"' ~/.config/opencode/opencode.json

# Reiniciar completamente
pkill -9 opencode && sleep 3 && opencode
```

### Alejandría no aparece en Claude Desktop

**Verificar:**
```bash
# JSON válido
python3 -m json.tool ~/.config/Claude/claude_desktop_config.json > /dev/null && echo "OK"

# Alejandría está configurada
cat ~/.config/Claude/claude_desktop_config.json

# Reiniciar completamente Claude Desktop
pkill -9 claude
# Abrir de nuevo
```

### Alejandría no aparece en VSCode/Copilot

**Verificar:**
```bash
# JSON válido
python3 -m json.tool ~/.config/Code/User/settings.json > /dev/null && echo "OK"

# Copilot MCP config
grep -A 10 '"github.copilot.chat.mcp.servers"' ~/.config/Code/User/settings.json

# Reload VSCode
# Ctrl+Shift+P → "Developer: Reload Window"
```

### Error "Database is locked"

Si múltiples clientes intentan usar la misma DB simultáneamente:

```bash
# Ver procesos de alejandria
ps aux | grep alejandria

# Matar procesos huérfanos
pkill alejandria

# Reiniciar clientes uno por uno
```

---

## 📊 Diferencias de Formato por Cliente

### ¿Por qué formatos diferentes?

Cada cliente tiene su propia implementación de MCP:

| Cliente | Spec MCP | Formato Config |
|---------|----------|----------------|
| **OpenCode** | Custom fork | JSON con `command[]`, `type`, `enabled` |
| **Claude Desktop** | Anthropic oficial | JSON estándar MCP |
| **VSCode/Copilot** | GitHub fork | JSON con `type: "stdio"`, `command`, `args[]` |

### Campos Clave por Cliente

| Campo | OpenCode | Claude Desktop | VSCode/Copilot |
|-------|----------|----------------|----------------|
| **Comando** | `command: []` (array) | `command: ""` (string) | `command: ""` (string) |
| **Argumentos** | Incluidos en `command[]` | `args: []` | `args: []` |
| **Tipo** | `type: "local"` requerido | No tiene | `type: "stdio"` requerido |
| **Enabled** | `enabled: true` requerido | No tiene | No tiene |
| **Environment** | `environment: {}` | `env: {}` | `env: {}` |

---

## 📝 Resumen de Cambios

### Antes (script install-mcp.sh)

❌ **Problema**: Script configuraba formato genérico que NO funcionaba para:
- OpenCode (usa `opencode.json` con formato custom)
- VSCode/Copilot (usa `github.copilot.chat.mcp.servers`)

✅ **Solo funcionaba**: Claude Desktop

### Ahora (configuración manual)

✅ **OpenCode**: Agregado a `opencode.json` línea 93
✅ **Claude Desktop**: Ya estaba configurado correctamente
✅ **VSCode/Copilot**: Agregado a `github.copilot.chat.mcp.servers`

---

## 🎯 Acción Requerida (TÚ)

**Para que Alejandría aparezca en TODOS los clientes**:

1. **Cerrar completamente**:
   - OpenCode
   - Claude Desktop
   - VSCode

2. **Esperar 3-5 segundos**

3. **Abrir de nuevo**:
   - OpenCode
   - Claude Desktop
   - VSCode

4. **Verificar** que "alejandria" aparece en la lista de MCP tools

5. **Probar** Store/Recall en cada uno

---

## 📚 Documentación

- **Diagnóstico**: `scripts/diagnose-mcp.sh`
- **OpenCode**: `OPENCODE_CONFIGURADO.md`
- **Este archivo**: `TODOS_LOS_CLIENTES_CONFIGURADOS.md`

---

**Última actualización**: 2026-04-11 18:15 UTC
**Estado**: ✅ Configuraciones aplicadas, pendiente reinicio de clientes

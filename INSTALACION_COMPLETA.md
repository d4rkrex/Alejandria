# Alejandría - Instalación Completada

## ✅ Estado de la Instalación

### Binario
- **Ubicación**: `/home/mroldan/.local/bin/alejandria`
- **Versión**: 0.1.0
- **Tamaño**: 27MB
- **Compilado en**: ar-appsec-01.veritran.net

### Configuraciones Disponibles

Tienes **DOS configuraciones** instaladas que puedes alternar fácilmente:

#### 1️⃣ Modo Local (stdio) - ACTUAL
- **Base de datos**: `~/.local/share/alejandria/alejandria.db`
- **Config**: `~/.config/alejandria/config.toml`
- **Total memories**: 96 (de sesiones anteriores)
- **Embeddings**: Deshabilitados (más rápido, solo keyword search)
- **Uso**: Ejecuta localmente, memoria privada en tu máquina

#### 2️⃣ Modo Remoto (HTTP/SSE) - DISPONIBLE
- **Servidor**: http://ar-appsec-01.veritran.net:8080
- **API Key**: `alejandria-prod-initial-key-2026`
- **Wrapper**: `/home/mroldan/.local/bin/alejandria-mcp-http`
- **Uso**: Conecta al servidor compartido, memoria centralizada para todo el equipo

---

## 🔄 Cómo Cambiar Entre Modos

### Opción A: Reinstalar con el script

```bash
# Cambiar a modo local
cd /home/mroldan/repos/AppSec/Alejandria
./scripts/install-mcp.sh --binary /home/mroldan/.local/bin/alejandria

# Cambiar a modo remoto
./scripts/install-mcp.sh \
  --remote http://ar-appsec-01.veritran.net:8080 \
  --api-key alejandria-prod-initial-key-2026
```

**Ventajas**:
- ✅ Automático, actualiza todos los clientes MCP
- ✅ Crea backups de configuraciones anteriores

### Opción B: Restaurar desde backup

```bash
# Ver backups disponibles
ls -la ~/.config/opencode/*.backup*
ls -la ~/.config/Claude/*.backup*

# Restaurar un backup específico (ejemplo)
cp ~/.config/opencode/mcp_config.json.backup-20260411-144017 \
   ~/.config/opencode/mcp_config.json
```

---

## 🧪 Pruebas

### Probar instalación actual

```bash
/home/mroldan/.local/bin/test-alejandria
```

### Probar servidor remoto manualmente

```bash
# Health check
curl http://ar-appsec-01.veritran.net:8080/health \
  -H 'X-API-Key: alejandria-prod-initial-key-2026'

# JSON-RPC initialize
curl -X POST http://ar-appsec-01.veritran.net:8080/rpc \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: alejandria-prod-initial-key-2026' \
  -d '{
    "jsonrpc":"2.0",
    "method":"initialize",
    "params":{
      "protocolVersion":"2024-11-05",
      "capabilities":{},
      "clientInfo":{"name":"test","version":"1.0"}
    },
    "id":1
  }'
```

### Probar CLI local

```bash
# Almacenar una memoria
alejandria store "Prueba desde CLI" --topic "test" --importance "high"

# Buscar
alejandria recall "prueba"

# Ver estadísticas
alejandria stats

# Ver topics
alejandria topics

# Exportar todo
alejandria export --output ~/alejandria-backup.json
```

---

## 🎯 Clientes MCP Configurados

### OpenCode
- **Config**: `~/.config/opencode/mcp_config.json`
- **Estado**: ✅ Configurado (modo remoto actualmente)
- **Backups**: `~/.config/opencode/mcp_config.json.backup-*`

### Claude Desktop
- **Config**: `~/.config/Claude/claude_desktop_config.json`
- **Estado**: ✅ Configurado (modo remoto actualmente)
- **Backups**: `~/.config/Claude/claude_desktop_config.json.backup-*`

### VSCode
- **Config**: `~/.config/Code/User/settings.json`
- **Estado**: ✅ Configurado (modo remoto actualmente)

### ⚠️ Importante: Reiniciar Clientes

Después de cambiar la configuración, **debes reiniciar** el cliente MCP:
- **OpenCode**: Cerrar y abrir de nuevo
- **Claude Desktop**: Cerrar y abrir de nuevo
- **VSCode**: Recargar ventana (`Ctrl+Shift+P` → "Developer: Reload Window")

---

## 🔍 Verificar Qué Modo Está Activo

```bash
# Ver configuración actual de OpenCode
cat ~/.config/opencode/mcp_config.json | jq '.mcpServers.alejandria.command'

# Si responde "/home/mroldan/.local/bin/alejandria" → Modo local
# Si responde "/home/mroldan/.local/bin/alejandria-mcp-http" → Modo remoto
```

---

## 📊 Diferencias Entre Modos

| Aspecto | Modo Local (stdio) | Modo Remoto (HTTP/SSE) |
|---------|-------------------|------------------------|
| **Database** | `~/.local/share/alejandria/alejandria.db` | `/var/lib/alejandria/alejandria.db` (servidor) |
| **Compartida** | ❌ Solo tú | ✅ Todo el equipo |
| **Latencia** | ~10ms (local) | ~50ms (red interna) |
| **Disponibilidad** | Solo cuando tu máquina está encendida | 24/7 (servidor) |
| **Seguridad** | Local, no requiere auth | API key + encryption |
| **Backups** | Manual | Centralizado (servidor) |
| **Embeddings** | Deshabilitado (configurable) | Habilitado (servidor) |

---

## 🚀 Próximos Pasos

### 1. Decidir qué modo usar

**Recomendación**:
- **Modo local** si trabajas solo y quieres máxima privacidad/velocidad
- **Modo remoto** si quieres compartir memoria con el equipo AppSec

### 2. Habilitar embeddings (opcional, solo modo local)

Si quieres búsqueda semántica en modo local:

```bash
# Editar config
nano ~/.config/alejandria/config.toml

# Cambiar:
[embeddings]
enabled = true  # era false

# Generar embeddings para memories existentes
alejandria embed
```

**Nota**: Embeddings incrementan latencia de ~10ms a ~30ms, pero permiten búsqueda semántica.

### 3. Probar en OpenCode

1. Reiniciar OpenCode
2. Abrir una conversación
3. Verificar que Alejandría aparece en la lista de MCP tools
4. Probar comandos:
   - "Store this: La arquitectura usa JWT con RS256"
   - "Recall información sobre JWT"

### 4. Migrar datos entre modos (opcional)

Si quieres mover tus 96 memories locales al servidor remoto:

```bash
# 1. Exportar desde local
alejandria export --output ~/local-memories.json

# 2. Cambiar a modo remoto (reinstalar)
./scripts/install-mcp.sh \
  --remote http://ar-appsec-01.veritran.net:8080 \
  --api-key alejandria-prod-initial-key-2026

# 3. Importar al servidor remoto
# (esto requiere que el servidor tenga el comando import habilitado)
# Por ahora, esto no está implementado vía HTTP, solo via CLI local
```

---

## 🐛 Troubleshooting

### OpenCode no muestra Alejandría en MCP tools

1. Verificar que el proceso no falló:
   ```bash
   # Probar manualmente
   ALEJANDRIA_CONFIG=~/.config/alejandria/config.toml \
     /home/mroldan/.local/bin/alejandria serve
   ```

2. Ver logs de OpenCode:
   - Abrir Developer Tools
   - Buscar errores relacionados con MCP

### Error "Database is locked"

Esto pasa si hay múltiples instancias corriendo:
```bash
ps aux | grep alejandria
# Matar procesos duplicados
kill <PID>
```

### Modo remoto no conecta

1. Verificar conectividad:
   ```bash
   curl http://ar-appsec-01.veritran.net:8080/health \
     -H 'X-API-Key: alejandria-prod-initial-key-2026'
   ```

2. Verificar que el wrapper tiene permisos:
   ```bash
   ls -la /home/mroldan/.local/bin/alejandria-mcp-http
   chmod +x /home/mroldan/.local/bin/alejandria-mcp-http
   ```

---

## 📚 Documentación Completa

- **Guía completa**: `docs/ALEJANDRIA_EXPLICACION.md`
- **CLI help**: `alejandria --help`
- **Comando específico**: `alejandria <comando> --help`

---

## 🎉 Resumen

✅ **Binario compilado** y funcionando (27MB)
✅ **Modo local** instalado y testeado (96 memories existentes)
✅ **Modo remoto** instalado y listo para usar
✅ **OpenCode, Claude Desktop, VSCode** configurados
✅ **Backups** de configuraciones anteriores creados
✅ **Test script** disponible (`test-alejandria`)
✅ **Servidor remoto** funcionando en ar-appsec-01.veritran.net:8080

**Modo actualmente activo**: Remoto (HTTP/SSE)

**Para empezar**: Reinicia OpenCode y prueba almacenar una memoria!

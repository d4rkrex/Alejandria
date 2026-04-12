# Gestión de API Keys para Alejandría MCP Server

## 📋 Resumen

Este documento describe cómo generar y gestionar API keys para usuarios que se conectan al servidor Alejandría MCP SSE.

---

## 🔑 Generación de API Keys

### Script de Generación

Usa el script `generate-api-key.sh` para crear nuevas API keys:

```bash
cd ~/repos/AppSec/Alejandria

# Generar key sin expiración
./scripts/generate-api-key.sh juan.perez

# Generar key con expiración de 90 días
./scripts/generate-api-key.sh maria.garcia --expires 90

# Generar key para app móvil (30 días)
./scripts/generate-api-key.sh mobile-app --expires 30
```

### Salida del Script

El script genera:

1. **API Key aleatoria** de 40 caracteres hexadecimales
2. **Registro local** en `~/.alejandria/generated-keys.txt`
3. **Instrucciones completas** para el usuario y el admin

Ejemplo de salida:

```
═══════════════════════════════════════════════════════════════
              API Key Generated Successfully! 🔑
═══════════════════════════════════════════════════════════════

User:       juan.perez
API Key:    alejandria-a3f8b2c1d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9
Created:    2026-04-11 18:30:00 UTC
Expires:    2026-07-10 (90 days)
```

---

## ⚠️ LIMITACIÓN ACTUAL (v1.x)

**Alejandría v1.x soporta SOLO UNA API key activa a la vez.**

Esto significa que:

- ❌ **NO puedes** tener múltiples usuarios con keys diferentes simultáneamente
- ❌ **NO puedes** revocar acceso individual
- ❌ **NO puedes** auditar uso por usuario
- ✅ **SÍ puedes** rotar la key (pero afecta a TODOS los usuarios)

### Workflow Actual

```
┌─────────────────────────────────────────────────────────────┐
│                     Servidor Alejandría                     │
│                                                             │
│  API Key activa: alejandria-abc123...                      │
│                                                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐      │
│  │ User 1  │  │ User 2  │  │ User 3  │  │ User 4  │      │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘      │
│      │             │             │             │           │
│      └─────────────┴─────────────┴─────────────┘           │
│                    TODOS usan                               │
│              la MISMA API key                               │
└─────────────────────────────────────────────────────────────┘
```

**Implicación:** Si cambias la API key, todos los usuarios necesitan actualizar su configuración.

---

## 🚀 Activar API Key en el Servidor

### Paso 1: SSH al Servidor

```bash
ssh mroldan@ar-appsec-01.veritran.net
```

### Paso 2: Actualizar Systemd Service

```bash
# Editar el service file
sudo nano /etc/systemd/system/alejandria.service

# Reemplazar la línea:
# Environment="ALEJANDRIA_API_KEY=old-key-here"
# Con:
Environment="ALEJANDRIA_API_KEY=alejandria-nueva-key-generada"
```

### Paso 3: Reload y Restart

```bash
sudo systemctl daemon-reload
sudo systemctl restart alejandria
sudo systemctl status alejandria
```

### Paso 4: Verificar

```bash
curl -H 'X-API-Key: alejandria-nueva-key-generada' \
     https://ar-appsec-01.veritran.net/alejandria/health

# Esperado: {"status":"healthy"}
```

---

## 📤 Compartir API Key con Usuario

### ✅ Métodos SEGUROS:

1. **1Password / Bitwarden**
   - Crear item compartido
   - Enviar link de acceso temporal

2. **Signal / Telegram (encrypted)**
   - Mensaje directo cifrado
   - Auto-destruir después de leer

3. **En persona**
   - Mostrar pantalla
   - No dejar registro escrito

### ❌ Métodos INSEGUROS (NUNCA usar):

- ❌ Email corporativo
- ❌ Slack / Teams (texto plano)
- ❌ WhatsApp
- ❌ SMS
- ❌ Commit a Git
- ❌ Documentación pública

---

## 👥 Configuración para Usuario Final

El usuario debe configurar su cliente MCP con la API key generada.

### OpenCode (`~/.config/opencode/opencode.json`)

```json
{
  "mcp": {
    "alejandria": {
      "url": "https://ar-appsec-01.veritran.net/alejandria",
      "apiKey": "alejandria-key-del-usuario-aqui",
      "transport": "sse",
      "tlsCert": "~/.alejandria/ca-cert.pem",
      "enabled": true,
      "type": "remote"
    }
  }
}
```

### Claude Code CLI (`~/.claude.json`)

```json
{
  "mcpServers": {
    "alejandria": {
      "url": "https://ar-appsec-01.veritran.net/alejandria",
      "apiKey": "alejandria-key-del-usuario-aqui",
      "transport": "sse"
    }
  }
}
```

### Claude Desktop (`~/.config/Claude/claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "alejandria": {
      "url": "https://ar-appsec-01.veritran.net/alejandria",
      "apiKey": "alejandria-key-del-usuario-aqui",
      "transport": "sse"
    }
  }
}
```

### Descargar CA Certificate

```bash
mkdir -p ~/.alejandria
curl -o ~/.alejandria/ca-cert.pem \
     https://ar-appsec-01.veritran.net/alejandria/ca-cert
```

### Reiniciar Clientes MCP

```bash
# OpenCode
pkill -9 opencode && opencode

# Claude Code CLI
# Ejecutar /exit y reabrir

# Claude Desktop
# Cerrar y abrir aplicación

# VSCode
# Ctrl+Shift+P → "Developer: Reload Window"
```

---

## 📊 Registro de Keys Generadas

Todas las keys generadas se guardan en:

```
~/.alejandria/generated-keys.txt
```

### Formato del Registro

```
2026-04-11 18:30:00 UTC | juan.perez | alejandria-abc123... | Expires: 2026-07-10 (90 days) | Status: ACTIVE
2026-04-12 09:15:00 UTC | maria.garcia | alejandria-def456... | Expires: Never | Status: ACTIVE
2026-04-12 14:22:00 UTC | mobile-app | alejandria-ghi789... | Expires: 2026-05-12 (30 days) | Status: REVOKED
```

### Comandos de Gestión

```bash
# Listar todas las keys
cat ~/.alejandria/generated-keys.txt

# Buscar key de usuario específico
grep 'juan.perez' ~/.alejandria/generated-keys.txt

# Listar keys activas
grep 'ACTIVE' ~/.alejandria/generated-keys.txt

# Listar keys revocadas
grep 'REVOKED' ~/.alejandria/generated-keys.txt

# Marcar key como revocada (manual)
sed -i 's/| juan.perez |.*ACTIVE/| juan.perez |.*REVOKED/' ~/.alejandria/generated-keys.txt
```

---

## 🔄 Rotación de API Keys

### Cuándo Rotar

- ✅ **90 días** (buena práctica de seguridad)
- ✅ **Usuario sale del equipo** (obligatorio)
- ✅ **Key comprometida** (inmediato)
- ✅ **Reorganización de equipo**
- ✅ **Auditoría de seguridad**

### Proceso de Rotación

```bash
# 1. Generar nueva key
./scripts/generate-api-key.sh team-key --expires 90

# 2. Notificar a TODOS los usuarios
#    (vía Slack, email, reunión)

# 3. Actualizar servidor (ver sección "Activar API Key")

# 4. Usuarios actualizan sus configs

# 5. Verificar que todos se conectaron exitosamente

# 6. Marcar key antigua como REVOKED
sed -i 's/| old-key |.*ACTIVE/| old-key |.*REVOKED/' ~/.alejandria/generated-keys.txt
```

---

## 🔒 Mejoras Futuras (v2.0)

### Multi-Key Support (Sprint 1 - P0-2)

Alejandría v2.0 implementará:

✅ **Base de datos de API keys**
```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    key_hash TEXT NOT NULL UNIQUE,
    username TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT,
    revoked_at TEXT,
    last_used_at TEXT,
    usage_count INTEGER DEFAULT 0
);
```

✅ **CLI de gestión**
```bash
# Generar key (con persistencia en DB)
alejandria admin generate-key --user juan.perez --expires 90d

# Listar keys activas
alejandria admin list-keys

# Revocar key individual
alejandria admin revoke-key --user juan.perez

# Ver actividad por usuario
alejandria admin key-activity --user juan.perez --last 7d
```

✅ **Validación multi-key**
- Múltiples keys activas simultáneamente
- Revocación individual (sin afectar otros usuarios)
- Expiración automática
- Auditoría por usuario

✅ **TUI Admin Console (opcional)**
```
┌──────────────────────────────────────────────────────┐
│          Alejandría Admin Console                    │
├──────────────────────────────────────────────────────┤
│ API Keys (5 active)                  [G] Generate    │
│ ┌────────────────────────────────────────────────┐  │
│ │ User      │ Created │ Last Used │ Requests │ ⚡│  │
│ ├────────────────────────────────────────────────┤  │
│ │ ▶ juan     │ 10 Apr  │ 5m ago    │ 1,234    │ ✓│  │
│ │   maria    │ 09 Apr  │ 2h ago    │ 856      │ ✓│  │
│ └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### Timeline

| Feature | Sprint | Estimación | Prioridad |
|---------|--------|------------|-----------|
| Multi-key DB schema | Sprint 1 (P0-2) | 1 día | P0 |
| CLI commands | Sprint 1 (P0-2) | 1.5 días | P0 |
| Expiration + rotation | Sprint 1 (P0-2) | 1 día | P0 |
| Usage auditing | Sprint 2 (P1) | 0.5 días | P1 |
| TUI Console | Sprint 3 (P2) | 4 días | P2 |

---

## 📚 Referencias

- **Security Review:** `SECURITY_REMEDIATION_PLAN.md`
- **Hallazgo P0-2:** API Keys Rotation & Management (DREAD 7.2)
- **TLS Setup:** `TLS_IMPLEMENTADO_RESUMEN.md`
- **Instalador:** `scripts/install-mcp.sh` (pendiente actualización)

---

**Última actualización:** 2026-04-11  
**Versión:** 1.0 (single-key limitation)  
**Próxima versión:** 2.0 (multi-key support, Sprint 1)

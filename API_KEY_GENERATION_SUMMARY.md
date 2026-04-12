# API Key Management - Resumen Ejecutivo

**Fecha:** 2026-04-11  
**Versión:** 1.0 (single-key limitation)  
**Sprint:** Pre-Sprint 1 (preparación para P0-2)

---

## ✅ Implementado AHORA

### 1. Script de Generación: `scripts/generate-api-key.sh`

**Características:**
- ✅ Genera API keys aleatorias de 40 caracteres hex
- ✅ Soporte de expiración (días personalizables)
- ✅ Registro local en `~/.alejandria/generated-keys.txt`
- ✅ Validación de formato de username
- ✅ Instrucciones completas para usuario y admin
- ✅ Warnings de seguridad prominentes

**Uso:**
```bash
# Key sin expiración
./scripts/generate-api-key.sh juan.perez

# Key con expiración de 90 días
./scripts/generate-api-key.sh maria.garcia --expires 90
```

**Output:**
- API key generada
- Fecha de creación
- Fecha de expiración (si aplica)
- Instrucciones de configuración para 5 clientes MCP
- Comandos de activación en servidor
- Comandos de gestión de registry

---

### 2. Documentación: `API_KEY_MANAGEMENT.md`

**Contenido:**

#### Sección 1: Generación
- Ejemplos de uso del script
- Formato de output
- Explicación de parámetros

#### Sección 2: Limitación Actual (v1.x)
- ⚠️ Solo UNA key activa simultáneamente
- Diagrama de arquitectura single-key
- Implicaciones para el equipo

#### Sección 3: Activación en Servidor
- Paso a paso para SSH
- Actualización de systemd service
- Comandos de reload y restart
- Verificación de funcionamiento

#### Sección 4: Distribución Segura
- ✅ Métodos SEGUROS (1Password, Signal, en persona)
- ❌ Métodos INSEGUROS (email, Slack, WhatsApp)
- Best practices de seguridad

#### Sección 5: Configuración Usuario
- OpenCode config example
- Claude Code CLI config example
- Claude Desktop config example
- VSCode/Copilot config example
- Instrucciones de descarga de CA cert
- Comandos de restart de clientes

#### Sección 6: Registro de Keys
- Formato del archivo de registry
- Comandos de consulta (grep, cat)
- Marcado de keys como REVOKED

#### Sección 7: Rotación
- Cuándo rotar (90 días, usuario sale, compromiso)
- Proceso paso a paso
- Notificación a usuarios

#### Sección 8: Roadmap v2.0
- Esquema de DB para multi-key
- CLI commands planeados
- TUI console mockup
- Timeline de implementación (Sprint 1-3)

---

## ⚠️ Limitación Actual

**CRÍTICO:** Alejandría v1.x **NO soporta múltiples API keys simultáneas**.

### Implicaciones:

| Escenario | Comportamiento Actual | Workaround |
|-----------|----------------------|------------|
| **Usuario nuevo** | Admin genera key, actualiza servidor, TODOS actualizan config | Coordinar rotación con equipo |
| **Usuario sale** | Rotar key, TODOS actualizan config | Avisar con anticipación |
| **Key comprometida** | Rotar key INMEDIATAMENTE, TODOS actualizan | Plan de emergencia |
| **Auditoría por usuario** | ❌ NO POSIBLE | Esperar v2.0 |
| **Revocación individual** | ❌ NO POSIBLE | Esperar v2.0 |

### Workflow Temporal:

```
OPCIÓN 1 (Más Simple):
  • Todos los usuarios del equipo usan LA MISMA key
  • Rotar cada 90 días (coordinar con equipo)
  • Si alguien sale, rotar key

OPCIÓN 2 (Más Controlado):
  • Generar keys por usuario (registro local)
  • Activar la key del usuario "activo"
  • Rotar keys cuando cambie el usuario activo
  • Nota: Solo 1 usuario puede conectarse a la vez
```

**Recomendación:** Usar **Opción 1** hasta implementar multi-key support.

---

## 📁 Archivos Creados

| Archivo | Descripción | Ubicación |
|---------|-------------|-----------|
| `generate-api-key.sh` | Script de generación | `scripts/` |
| `API_KEY_MANAGEMENT.md` | Documentación completa | Root del repo |
| `generated-keys.txt` | Registro de keys (local) | `~/.alejandria/` (gitignored) |

---

## 🔒 Seguridad

### Protecciones Implementadas:

✅ **Keys aleatorias:** OpenSSL rand (40 caracteres hex)  
✅ **Registry local:** NO versionado en Git (.gitignore)  
✅ **Warnings visibles:** Output del script destaca riesgos  
✅ **Métodos seguros documentados:** 1Password, Signal, etc.  
✅ **Validación de username:** Regex para prevenir inyección  
✅ **Formato estructurado:** Fácil de parsear/auditar  

### Riesgos Residuales:

⚠️ **Single-key limitation:** No isolation entre usuarios  
⚠️ **No expiration enforcement:** Manual (hasta v2.0)  
⚠️ **No automatic revocation:** Manual via systemd restart  
⚠️ **No usage auditing:** No logs por usuario (hasta v2.0)  

---

## 🚀 Próximos Pasos

### Fase 2: Multi-Key Support (Sprint 1 - P0-2)

**Estimación:** 2-3 días de desarrollo

**Componentes:**

1. **DB Schema** (0.5 días)
   ```sql
   CREATE TABLE api_keys (
       id TEXT PRIMARY KEY,
       key_hash TEXT NOT NULL UNIQUE,
       username TEXT NOT NULL,
       created_at TEXT NOT NULL,
       expires_at TEXT,
       revoked_at TEXT,
       last_used_at TEXT,
       usage_count INTEGER DEFAULT 0
   );
   ```

2. **Auth Middleware Update** (1 día)
   - Validar contra DB (no env var)
   - Verificar expiración automática
   - Actualizar last_used_at
   - Incrementar usage_count

3. **CLI Commands** (1 día)
   ```bash
   alejandria admin generate-key --user <name> --expires <days>
   alejandria admin list-keys [--active|--expired|--revoked]
   alejandria admin revoke-key --user <name>
   alejandria admin key-activity --user <name> --last <period>
   ```

4. **Migration Script** (0.5 días)
   - Migrar key actual a DB
   - Backward compatibility
   - Documentación de upgrade

**Hallazgo asociado:** P0-2 (API Keys Rotation, DREAD 7.2)

---

### Fase 3: TUI Console (Sprint 2-3 - Opcional)

**Estimación:** 4-5 días

**Tecnologías:**
- `ratatui` (TUI framework)
- `crossterm` (terminal control)

**Features:**
- Dashboard visual de keys activas
- Generación interactiva
- Revocación con confirmación
- Gráficos de uso
- Logs en tiempo real

**Prioridad:** P2 (nice-to-have si >10 usuarios)

---

## 📊 Métricas de Éxito

### Fase 1 (Actual):

- ✅ Script funcional y documentado
- ✅ Flujo completo usuario → admin → servidor
- ✅ Warnings de seguridad visibles
- ✅ Registry local para tracking
- ✅ Integración con instalador (pendiente)

### Fase 2 (Sprint 1):

- ⏳ Multi-key validation en producción
- ⏳ Expiración automática funcionando
- ⏳ Revocación individual sin downtime
- ⏳ Auditoría por usuario habilitada
- ⏳ Migration exitosa sin pérdida de servicio

### Fase 3 (Sprint 2-3):

- ⏳ TUI console operacional
- ⏳ Feedback positivo de admins
- ⏳ Reducción de tiempo de gestión (>50%)

---

## 🎯 Casos de Uso Actuales

### Caso 1: Agregar Usuario Nuevo (Juan)

```bash
# Admin
./scripts/generate-api-key.sh juan.perez --expires 90
# Output: alejandria-abc123...

# Compartir key via 1Password
# (Enviar link temporal a Juan)

# Juan configura sus clientes MCP
# (Ver API_KEY_MANAGEMENT.md sección 5)

# Admin activa key en servidor
ssh mroldan@ar-appsec-01.veritran.net
sudo nano /etc/systemd/system/alejandria.service
# Actualizar ALEJANDRIA_API_KEY=alejandria-abc123...
sudo systemctl daemon-reload && sudo systemctl restart alejandria

# Notificar a TODOS los usuarios del equipo
# (Todos deben actualizar a nueva key)
```

### Caso 2: Usuario Sale del Equipo (María)

```bash
# Admin genera nueva key para reemplazar
./scripts/generate-api-key.sh team-key --expires 90

# Marcar key de María como REVOKED
sed -i 's/| maria.garcia |.*ACTIVE/| maria.garcia |.*REVOKED/' \
    ~/.alejandria/generated-keys.txt

# Activar nueva key en servidor
# (Ver Caso 1)

# Notificar a usuarios restantes
# (Actualizar configs con nueva key)
```

### Caso 3: Rotación Programada (90 días)

```bash
# Admin genera nueva key
./scripts/generate-api-key.sh team-q2-2026 --expires 90

# Notificar al equipo con 7 días de anticipación
# "La key expira el 10-Jul-2026, nueva key disponible"

# Día de rotación:
# 1. Activar nueva key en servidor
# 2. Usuarios actualizan configs
# 3. Verificar conectividad
# 4. Marcar key anterior como REVOKED
```

---

## 📚 Referencias

- **Security Review:** `SECURITY_REMEDIATION_PLAN.md`
- **Hallazgo P0-2:** API Keys Rotation & Management (DREAD 7.2)
- **TLS Implementation:** `TLS_IMPLEMENTADO_RESUMEN.md`
- **Script source:** `scripts/generate-api-key.sh`
- **Full docs:** `API_KEY_MANAGEMENT.md`

---

## ✅ Checklist de Implementación

### Completado:

- [x] Script de generación funcional
- [x] Documentación completa
- [x] Ejemplos de uso
- [x] Registry local (.gitignore)
- [x] Security warnings prominentes
- [x] Instrucciones de distribución segura
- [x] Comandos de activación
- [x] Comandos de rotación
- [x] Commit a Git

### Pendiente (Sprint 1):

- [ ] Multi-key DB schema
- [ ] Auth middleware update
- [ ] CLI admin commands
- [ ] Migration script
- [ ] Actualizar instalador
- [ ] Tests de integración
- [ ] Documentación de upgrade

### Pendiente (Sprint 2-3):

- [ ] TUI console
- [ ] Gráficos de uso
- [ ] Alertas de expiración
- [ ] Export/import de registry

---

**Preparado por:** AppSec Team - Veritran  
**Hallazgo relacionado:** P0-2 (DREAD 7.2)  
**Estado:** ✅ Fase 1 Completada, Fase 2 en Sprint 1

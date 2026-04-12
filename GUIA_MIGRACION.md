# Alejandría - Guía de Migración y Sincronización

## 📊 Estado Actual

| Fuente | Cantidad | Ubicación |
|--------|----------|-----------|
| **Engram** | 1501 observations | `~/.engram/engram.db` |
| **Alejandría Local** | 96 memories | `~/.local/share/alejandria/alejandria.db` |
| **Alejandría Remote** | 0 memories | `ar-appsec-01.veritran.net:/var/lib/alejandria/alejandria.db` |

---

## 🔄 Herramientas Disponibles

### 1. Script de Migración Completo (`scripts/migrate.py`)

```bash
# Ver ayuda
python3 scripts/migrate.py --help

# Preview migración de Engram
python3 scripts/migrate.py migrate --dry-run

# Migrar TODO de Engram a Alejandría
python3 scripts/migrate.py migrate --execute

# Migrar solo un proyecto
python3 scripts/migrate.py migrate --execute --project Alejandria

# Exportar Alejandría a JSON
python3 scripts/migrate.py export --output backup.json

# Exportar solo un proyecto
python3 scripts/migrate.py export --output backup.json --project Argos
```

### 2. Helper de Sincronización (`scripts/sync.sh`)

Comandos rápidos para tareas comunes:

```bash
# Ver ayuda
./scripts/sync.sh help

# Ver stats de local, remoto y Engram
./scripts/sync.sh stats

# Preview migración de Engram
./scripts/sync.sh engram-preview

# Migrar de Engram a Alejandría
./scripts/sync.sh engram-migrate

# Backup local a JSON
./scripts/sync.sh backup-local

# Backup remoto a JSON
./scripts/sync.sh backup-remote

# Push: Local → Remoto
./scripts/sync.sh push

# Pull: Remoto → Local
./scripts/sync.sh pull
```

---

## 🎯 Escenarios Comunes

### Escenario 1: Migrar TODO de Engram a Alejandría Local

```bash
# 1. Preview para ver qué se migrará
./scripts/sync.sh engram-preview

# 2. Ejecutar migración
./scripts/sync.sh engram-migrate

# 3. Verificar resultado
alejandria stats
```

**Resultado esperado**: ~1501 memories nuevas en Alejandría local (96 actuales + 1501 de Engram = ~1597 total)

---

### Escenario 2: Sincronizar Local → Remoto

```bash
# Opción A: Usar el helper (recomendado)
./scripts/sync.sh push

# Opción B: Manual
# 1. Exportar local
python3 scripts/migrate.py export --output /tmp/local.json

# 2. Copiar al servidor
scp /tmp/local.json mroldan@ar-appsec-01.veritran.net:/tmp/

# 3. Importar en servidor
ssh mroldan@ar-appsec-01.veritran.net \
  "alejandria import --input /tmp/local.json"

# 4. Verificar
./scripts/sync.sh stats
```

---

### Escenario 3: Backup Completo de Todo

```bash
# 1. Backup Engram (manual - SQLite)
cp ~/.engram/engram.db ~/backups/engram-$(date +%Y%m%d).db

# 2. Backup Alejandría Local
./scripts/sync.sh backup-local
# Guarda en: ~/alejandria-local-backup-YYYYMMDD-HHMMSS.json

# 3. Backup Alejandría Remote
./scripts/sync.sh backup-remote
# Guarda en: ~/alejandria-remote-backup-YYYYMMDD-HHMMSS.json
```

---

### Escenario 4: Migrar Solo Proyecto Específico

```bash
# Solo migrar proyecto "Alejandria" de Engram
python3 scripts/migrate.py migrate --execute --project Alejandria

# Exportar solo proyecto "Argos" de Alejandría
python3 scripts/migrate.py export \
  --output argos-backup.json \
  --project Argos
```

---

## ⚠️ Consideraciones Importantes

### Duplicados

Por defecto, el script **NO sobrescribe** memories existentes:
- Usa `topic_key` para detectar duplicados
- Duplicados se **skipean** automáticamente
- Para permitir duplicados: `--allow-duplicates`

### Modos de Import

El comando `alejandria import` soporta 3 modos:

```bash
# skip (default): Ignorar duplicados
alejandria import --input backup.json --mode skip

# update: Actualizar existentes
alejandria import --input backup.json --mode update

# replace: Borrar todo y reemplazar
alejandria import --input backup.json --mode replace
```

### Mapeo de Campos Engram → Alejandría

| Campo Engram | Campo Alejandría | Notas |
|--------------|-----------------|-------|
| `title` | `summary` | Título de la memory |
| `content` | `raw_excerpt` | Contenido completo |
| `project` | `topic` | Nombre del proyecto |
| `type` | `importance` | Mapeado: decision→high, manual→medium, etc. |
| `topic_key` | `topic_key` | Preservado para upserts |
| `created_at` | `created_at` | Convertido a timestamp Unix |

---

## 📈 Workflow Recomendado

### Para Empezar (Migración Inicial)

```bash
# Día 1: Preview
./scripts/sync.sh engram-preview
./scripts/sync.sh stats

# Día 1: Migrar Engram → Alejandría Local
./scripts/sync.sh engram-migrate
alejandria stats

# Día 1: Push Local → Remoto (para compartir con equipo)
./scripts/sync.sh push
./scripts/sync.sh stats
```

### Uso Diario (después de migración)

```bash
# Opción A: Solo local (privado, rápido)
# - Usar modo local en MCP config
# - No hacer push/pull

# Opción B: Solo remoto (compartido con equipo)
# - Usar modo remoto en MCP config
# - Todo se almacena directo en servidor

# Opción C: Híbrido (lo mejor de ambos)
# - Trabajar local durante el día
# - Push al final del día para compartir
./scripts/sync.sh push
```

---

## 🐛 Troubleshooting

### Error: "Database is locked"

Múltiples instancias accediendo a la DB:

```bash
# Ver procesos
ps aux | grep alejandria

# Matar procesos
kill <PID>
```

### Error: "Topic key collision"

Intentando importar una memory que ya existe:

```bash
# Usar modo update en lugar de skip
alejandria import --input backup.json --mode update
```

### Import falla silenciosamente

Verificar formato JSON:

```bash
# Validar JSON
python3 -m json.tool backup.json > /dev/null

# Ver primeras líneas
head -50 backup.json
```

### Remote import no funciona

El servidor HTTP aún no tiene endpoint de import batch. Alternativas:

```bash
# Opción 1: SSH + CLI import (recomendado)
./scripts/sync.sh push

# Opción 2: Manual
scp backup.json mroldan@ar-appsec-01.veritran.net:/tmp/
ssh mroldan@ar-appsec-01.veritran.net "alejandria import --input /tmp/backup.json"
```

---

## 📊 Comparar Antes y Después

```bash
# ANTES de migrar
./scripts/sync.sh stats

# Migrar
./scripts/sync.sh engram-migrate

# DESPUÉS de migrar
./scripts/sync.sh stats

# Verificar que las memories se importaron
alejandria recall "algún término que sabes que estaba en Engram"
```

---

## 🎓 Ejemplos Prácticos

### Ejemplo 1: Migración completa Engram → Local → Remote

```bash
# Paso 1: Migrar Engram a Alejandría local
cd /home/mroldan/repos/AppSec/Alejandria
./scripts/sync.sh engram-migrate

# Paso 2: Verificar local
alejandria stats
alejandria recall "test"

# Paso 3: Push a remoto
./scripts/sync.sh push

# Paso 4: Verificar remoto
ssh mroldan@ar-appsec-01.veritran.net "alejandria stats"

# Paso 5: Cambiar MCP config a remoto
./scripts/install-mcp.sh \
  --remote http://ar-appsec-01.veritran.net:8080 \
  --api-key alejandria-prod-initial-key-2026

# Paso 6: Reiniciar OpenCode y probar
# En OpenCode: "Recall información sobre X"
```

### Ejemplo 2: Backup semanal automático

Crear script en `~/bin/alejandria-backup-weekly.sh`:

```bash
#!/bin/bash
BACKUP_DIR="$HOME/backups/alejandria"
mkdir -p "$BACKUP_DIR"
DATE=$(date +%Y%m%d)

# Backup local
cd /home/mroldan/repos/AppSec/Alejandria
python3 scripts/migrate.py export \
  --output "$BACKUP_DIR/local-$DATE.json"

# Backup remoto
./scripts/sync.sh backup-remote

# Limpiar backups viejos (> 30 días)
find "$BACKUP_DIR" -name "*.json" -mtime +30 -delete

echo "Backup completado: $DATE"
```

Agregar a crontab:

```bash
# Ejecutar todos los domingos a las 23:00
0 23 * * 0 $HOME/bin/alejandria-backup-weekly.sh
```

---

## 🔍 Verificación de Integridad

Después de migrar, verifica que todo esté bien:

```bash
# 1. Contar memories
sqlite3 ~/.local/share/alejandria/alejandria.db \
  "SELECT COUNT(*) FROM memories WHERE deleted_at IS NULL;"

# 2. Ver distribución por topic
sqlite3 ~/.local/share/alejandria/alejandria.db \
  "SELECT topic, COUNT(*) FROM memories WHERE deleted_at IS NULL GROUP BY topic ORDER BY COUNT(*) DESC LIMIT 20;"

# 3. Verificar que se pueden buscar
alejandria recall "Alejandria" | head -30

# 4. Verificar que los topic_keys se preservaron
sqlite3 ~/.local/share/alejandria/alejandria.db \
  "SELECT COUNT(*) FROM memories WHERE topic_key IS NOT NULL;"
```

---

## 🚀 Próximos Pasos

1. **Migrar Engram a Alejandría Local**
   ```bash
   ./scripts/sync.sh engram-migrate
   ```

2. **Decidir modo de trabajo**:
   - **Local**: Privado, rápido, no compartido
   - **Remoto**: Compartido con equipo, 24/7, centralizado

3. **Si eliges modo remoto**:
   ```bash
   # Push local → remoto
   ./scripts/sync.sh push
   
   # Configurar MCP para remoto
   ./scripts/install-mcp.sh \
     --remote http://ar-appsec-01.veritran.net:8080 \
     --api-key alejandria-prod-initial-key-2026
   
   # Reiniciar OpenCode
   ```

4. **Probar en OpenCode**:
   - Store: "Alejandría migration completada exitosamente"
   - Recall: "migration Alejandria"

---

## 📚 Recursos

- **Documentación completa**: `docs/ALEJANDRIA_EXPLICACION.md`
- **Estado instalación**: `INSTALACION_COMPLETA.md`
- **CLI help**: `alejandria --help`
- **Script migración**: `scripts/migrate.py --help`
- **Script sync**: `./scripts/sync.sh help`

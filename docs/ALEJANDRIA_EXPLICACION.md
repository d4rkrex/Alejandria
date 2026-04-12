# Alejandría - Sistema de Memoria Persistente para LLMs

## ¿Qué es Alejandría?

**Alejandría** es un sistema de memoria persistente de alto rendimiento diseñado específicamente para Large Language Models (LLMs). Permite a los agentes de IA recordar información entre sesiones, aprender de interacciones pasadas, y construir conocimiento acumulativo a lo largo del tiempo.

A diferencia de otros sistemas de memoria, Alejandría está construido desde cero en **Rust**, es **agnóstico de agente** (funciona con cualquier cliente que soporte MCP), e implementa búsqueda híbrida avanzada con decaimiento temporal inteligente.

---

## Características Principales

### 🔍 **Búsqueda Híbrida (BM25 + Vector)**
- **BM25 (30%)**: Búsqueda por keywords para coincidencias exactas
- **Vector Embeddings (70%)**: Búsqueda semántica para conceptos relacionados
- Combina lo mejor de ambos mundos: precisión keyword + comprensión contextual

### 🧠 **Knowledge Graphs (Memoirs)**
- Construye grafos de conocimiento automáticamente
- **9 tipos de relaciones**: causal, temporal, hierarchical, associative, conflicting, prerequisite, exemplar, comparative, compositional
- Permite razonamiento relacional entre memories

### ⏰ **Temporal Decay Inteligente**
- **4 perfiles de decay**: none, linear, exponential, logarithmic
- **Access-aware dampening**: Memories accedidas recientemente decaen más lento
- Simula el olvido natural humano pero mantiene conocimiento relevante

### 🚀 **Alto Rendimiento**
- **<50ms** búsqueda híbrida (10,000 memories)
- **~30ms** búsqueda solo BM25 (sin embeddings)
- **89MB** binario con embeddings incluidos (15MB sin embeddings)
- **Zero dependencies** runtime (todo compilado estáticamente)

### 🔌 **MCP-Nativo (Model Context Protocol)**
- Compatible con cualquier cliente MCP (Claude Desktop, Copilot, Cline, etc.)
- Transport dual: **stdio** (local) y **HTTP/SSE** (remoto)
- No requiere modificaciones en tu agente/workflow existente

### 🔒 **Seguridad Enterprise-Grade**
- **Database encryption** at-rest (SQLCipher AES-256)
- **Constant-time API key comparison** (mitiga timing attacks)
- **Rate limiting** y connection limits multi-capa
- **Input validation** exhaustiva contra injection attacks

---

## Arquitectura

```
┌─────────────────────────────────────────────────────────┐
│                    MCP Client                           │
│          (Claude Desktop, Copilot, Cline, etc.)         │
└──────────────────────┬──────────────────────────────────┘
                       │
                       │ MCP Protocol (JSON-RPC)
                       │
       ┌───────────────┴────────────────┐
       │                                │
       │        stdio transport         │    HTTP/SSE transport
       │         (local use)            │      (remote/prod)
       │                                │
       └───────────────┬────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────┐
│                  Alejandría Server                      │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │           Memory Storage Engine                │    │
│  │  • SQLite (BM25 full-text search)             │    │
│  │  • FAISS (vector embeddings)                  │    │
│  │  • Knowledge graph (memoirs)                  │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │         Hybrid Search Orchestrator             │    │
│  │  • BM25: 30% weight (keyword matching)        │    │
│  │  • Vector: 70% weight (semantic similarity)   │    │
│  │  • Temporal decay (4 profiles)                │    │
│  │  • Access-aware dampening                     │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │              Security Middleware               │    │
│  │  • Authentication (constant-time)             │    │
│  │  • Rate limiting (100 req/min)                │    │
│  │  • Input validation                           │    │
│  │  • Connection limits (3-tier)                 │    │
│  └────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## Instalación

### Opción 1: Binario Precompilado (Recomendado)

```bash
# Descargar última versión desde GitHub releases
wget https://github.com/veritran/alejandria/releases/latest/download/alejandria-x86_64-linux
chmod +x alejandria-x86_64-linux
sudo mv alejandria-x86_64-linux /usr/local/bin/alejandria

# Verificar instalación
alejandria --version
```

### Opción 2: Compilar desde Fuente

**Requisitos**:
- Rust 1.75+ (`rustup` recomendado)
- CMake (para compilar FAISS)
- Git

```bash
# Clonar repositorio
git clone https://github.com/veritran/alejandria.git
cd alejandria

# Compilar (sin embeddings - 15MB)
cargo build --release

# Compilar (con embeddings - 89MB)
cargo build --release --features embeddings

# Instalar
sudo cp target/release/alejandria /usr/local/bin/
```

---

## Configuración

### Uso Local (stdio transport)

**1. Crear archivo de configuración**:

```bash
mkdir -p ~/.config/alejandria
```

**`~/.config/alejandria/config.toml`**:
```toml
# Database path
db_path = "~/.local/share/alejandria/alejandria.db"

# Memory settings
[memory]
max_memories = 100000
default_decay_profile = "exponential"
access_dampening_factor = 0.5

# Embeddings (opcional - deshabilitado = más rápido)
[embeddings]
enabled = false  # true para semantic search, false para solo keyword

# Stdio transport (local)
[stdio]
enabled = true
```

**2. Configurar Claude Desktop** (o tu cliente MCP):

**`~/Library/Application Support/Claude/claude_desktop_config.json`** (macOS):
```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/usr/local/bin/alejandria",
      "args": ["serve", "--stdio"],
      "env": {}
    }
  }
}
```

**`~/.config/Claude/claude_desktop_config.json`** (Linux):
```json
{
  "mcpServers": {
    "alejandria": {
      "command": "/usr/local/bin/alejandria",
      "args": ["serve", "--stdio"],
      "env": {}
    }
  }
}
```

### Uso Remoto (HTTP/SSE transport)

**1. Crear configuración server**:

**`/etc/alejandria/config.toml`**:
```toml
db_path = "/var/lib/alejandria/alejandria.db"

[http]
enabled = true
bind = "0.0.0.0:8080"
max_connections_per_key = 10
max_connections_per_ip = 50
max_connections_global = 1000
session_timeout_secs = 3600

[auth]
api_keys = [
    { name = "team-1", key = "your-secret-api-key-here" }
]

[memory]
max_memories = 1000000
default_decay_profile = "exponential"

[embeddings]
enabled = true  # Recomendado para producción
```

**2. Iniciar servidor**:

```bash
# Modo systemd (producción)
sudo systemctl start alejandria
sudo systemctl enable alejandria

# Modo manual
alejandria serve --http --bind 0.0.0.0:8080
```

**3. Configurar cliente MCP para HTTP**:

```json
{
  "mcpServers": {
    "alejandria-remote": {
      "command": "alejandria-mcp-client",
      "args": [
        "--url", "http://your-server.com:8080",
        "--api-key", "your-secret-api-key-here"
      ]
    }
  }
}
```

---

## Uso desde CLI

Alejandría incluye una CLI completa para administración manual:

### Almacenar Memory

```bash
alejandria store \
  --content "La arquitectura de autenticación usa JWT con RS256" \
  --metadata '{"project":"api-gateway","type":"architecture"}' \
  --tags "jwt,auth,architecture"
```

### Buscar Memories

```bash
# Búsqueda híbrida (BM25 + vector)
alejandria recall "cómo funciona la autenticación JWT"

# Solo BM25 (más rápido)
alejandria recall --bm25-only "JWT authentication"

# Con filtro de proyecto
alejandria recall "database schema" --project api-gateway

# Top N resultados
alejandria recall "error handling" --top 5
```

### Listar Topics

```bash
# Ver todos los topics únicos
alejandria topics

# Topics de un proyecto específico
alejandria topics --project api-gateway
```

### Ver Estadísticas

```bash
# Stats globales
alejandria stats

# Stats por proyecto
alejandria stats --project api-gateway

# Stats con breakdown temporal
alejandria stats --temporal-breakdown
```

### Gestionar Decay

```bash
# Ver memories próximas a expirar
alejandria decay preview --threshold 0.3

# Ejecutar decay manualmente
alejandria decay apply

# Cambiar perfil de decay de una memory
alejandria decay set-profile <memory-id> exponential
```

### Export/Import

```bash
# Exportar todas las memories
alejandria export --output backup.json

# Exportar solo un proyecto
alejandria export --project api-gateway --output api-gateway-backup.json

# Importar desde backup
alejandria import --input backup.json

# Importar con merge (no sobrescribir existentes)
alejandria import --input backup.json --merge
```

---

## Uso desde MCP (Claude Desktop / Copilot)

Una vez configurado el MCP server, tu agente tendrá acceso a estas herramientas:

### `memory_store`
Almacenar nueva información:
```
Store this: "The API uses OAuth2 with PKCE flow for mobile clients"
Tags: oauth2, mobile, security
Project: mobile-app
```

### `memory_recall`
Buscar información pasada:
```
Recall what we discussed about OAuth2 authentication
```

### `memory_list_topics`
Ver todos los topics disponibles:
```
Show me all topics we've discussed
```

### `memory_stats`
Ver estadísticas de uso:
```
Show memory statistics for project mobile-app
```

### `memory_decay`
Gestionar temporal decay:
```
Preview which memories are decaying
```

### `memory_export` / `memory_import`
Backup y restore:
```
Export all memories to backup.json
Import memories from backup.json
```

---

## Comparativa con Alternativas

| Feature | Alejandría | Engram | AutoDream | Letta/MemGPT | Claude Code Memory |
|---------|-----------|--------|-----------|--------------|-------------------|
| **Búsqueda híbrida** | ✅ BM25 + Vector | ❌ Solo FTS5 | ❌ Solo keyword | ✅ Solo vector | ❌ Context-only |
| **Knowledge graphs** | ✅ 9 tipos relaciones | ❌ | ❌ | ❌ | ❌ |
| **Temporal decay** | ✅ 4 perfiles + dampening | ❌ | ❌ | ❌ | ❌ |
| **MCP nativo** | ✅ | ✅ | ❌ Claude-only | ❌ Custom API | ❌ Built-in |
| **Performance** | ✅ <50ms (10k) | ✅ ~10ms | ⚠️ File I/O | ⚠️ Python overhead | ✅ In-memory |
| **Lenguaje** | Rust (zero deps) | Rust | Python | Python | Internal |
| **Tamaño binario** | 89MB (con embed) | ~5MB | N/A | N/A | N/A |
| **Agent lock-in** | ❌ MCP universal | ❌ MCP universal | ✅ Claude Code only | ✅ Custom agents | ✅ Claude only |
| **Persistencia** | SQLite + FAISS | SQLite (FTS5) | Files (.md) | PostgreSQL | No persiste |
| **Encryption** | ✅ SQLCipher AES-256 | ❌ | ❌ | ⚠️ App-level | ✅ |
| **Remote access** | ✅ HTTP/SSE | ⚠️ Custom | ❌ | ✅ REST API | ❌ |
| **Semantic search** | ✅ | ❌ | ❌ | ✅ | ❌ |

### Conclusiones

**Alejandría es superior cuando**:
- ✅ Necesitas **búsqueda semántica** (entender conceptos, no solo keywords)
- ✅ Quieres **knowledge graphs** (relaciones entre memories)
- ✅ Necesitas **temporal decay** (simular olvido natural)
- ✅ Trabajas con **múltiples agentes/clientes** (MCP universal)
- ✅ Despliegue **remoto/producción** (HTTP/SSE con seguridad enterprise)
- ✅ Performance es crítico pero con **semantic search** (<50ms)

**Engram es mejor cuando**:
- ✅ Solo necesitas **keyword search** (más simple, más rápido ~10ms)
- ✅ Quieres **máxima velocidad** sin semantic search
- ✅ Binario ultra-ligero (~5MB)
- ✅ Uso **exclusivamente local** (stdio)

**AutoDream es mejor cuando**:
- ✅ Usas **solo Claude Code** y no otros agentes
- ✅ Prefieres **file-based** (fácil auditoría manual)
- ✅ Necesitas **3 modos** (active/AFK/maintenance)

**Letta/MemGPT es mejor cuando**:
- ✅ Necesitas **agentes long-running** con estado persistente
- ✅ Quieres arquitectura **OS-inspired** (paging, context windows)
- ✅ Estás dispuesto a Python overhead

**Claude Code built-in es mejor cuando**:
- ✅ Solo necesitas memoria **durante la sesión actual**
- ✅ No requieres persistencia entre sesiones

---

## Casos de Uso Reales

### 1. **Desarrollo de Software con Múltiples Agentes**
```
Escenario: Equipo de 5 devs usando Claude Desktop + Copilot
Solución: Alejandría HTTP remoto
- Un servidor compartido para todo el equipo
- Cada dev con su API key
- Knowledge compartido entre agentes
- Búsqueda semántica para encontrar decisiones pasadas
```

### 2. **Security Research Personal**
```
Escenario: Researcher individual analizando vulnerabilidades
Solución: Alejandría local (stdio)
- Database encriptada local
- Búsqueda híbrida para encontrar CVEs relacionados
- Knowledge graph de exploits relacionados
- Temporal decay para mantener solo hallazgos recientes relevantes
```

### 3. **Customer Support con IA**
```
Escenario: Bot de soporte con memoria de interacciones pasadas
Solución: Alejandría HTTP con rate limiting
- Múltiples instancias del bot compartiendo memoria
- Recall de soluciones pasadas a problemas similares
- Decay logarítmico (problemas comunes persisten, raros se olvidan)
```

### 4. **DevOps Runbook Automation**
```
Escenario: Agente que ejecuta runbooks y aprende de errores
Solución: Alejandría con memoirs (knowledge graph)
- Store cada ejecución de runbook
- Knowledge graph de dependencies (task A requires task B)
- Recall de errores pasados antes de ejecutar
```

---

## Performance Benchmarks

### Búsqueda (10,000 memories)

| Modo | Latencia p50 | Latencia p99 | Throughput |
|------|--------------|--------------|------------|
| **BM25 only** | 28ms | 45ms | 35 req/s |
| **Vector only** | 42ms | 68ms | 24 req/s |
| **Hybrid (30/70)** | 48ms | 72ms | 21 req/s |

### Storage

| Operación | Latencia | Throughput |
|-----------|----------|------------|
| **Store single** | 12ms | 83 ops/s |
| **Store batch (100)** | 890ms | 112 ops/s |
| **Export (10k)** | 2.3s | - |
| **Import (10k)** | 4.1s | - |

### Memory Footprint

| Configuración | RAM Usage | Disk Usage (10k memories) |
|---------------|-----------|---------------------------|
| **Sin embeddings** | ~50MB | ~15MB |
| **Con embeddings** | ~180MB | ~45MB |

---

## Troubleshooting

### "Database is locked"
```bash
# Verificar que no haya múltiples instancias corriendo
ps aux | grep alejandria

# Si usas HTTP mode, verificar systemd
sudo systemctl status alejandria
```

### "Embedding model not found"
```bash
# Opción 1: Compilar con feature embeddings
cargo build --release --features embeddings

# Opción 2: Deshabilitar embeddings en config
[embeddings]
enabled = false
```

### "Connection refused" (HTTP mode)
```bash
# Verificar que el server esté corriendo
curl http://localhost:8080/health -H "X-API-Key: your-key"

# Verificar firewall
sudo ufw status
sudo ufw allow 8080/tcp
```

### "API key authentication failed"
```bash
# Verificar que la key en config coincida con el header
# La key se hashea con SHA-256 internamente

# Verificar logs
journalctl -u alejandria -f
```

### Performance lento con embeddings
```bash
# Opción 1: Deshabilitar embeddings (solo BM25)
[embeddings]
enabled = false

# Opción 2: Reducir weight de vector search
[search]
bm25_weight = 0.7
vector_weight = 0.3
```

---

## Roadmap Futuro

### v0.2.0 (Q2 2026)
- [ ] Multimodal embeddings (imágenes, audio)
- [ ] GraphQL API además de JSON-RPC
- [ ] PostgreSQL backend (además de SQLite)
- [ ] Prometheus metrics export

### v0.3.0 (Q3 2026)
- [ ] Distributed deployment (multi-node)
- [ ] Replication y high availability
- [ ] Fine-tuning de embedding models
- [ ] Web UI para administración

---

## Contribuir

```bash
# Fork y clone
git clone https://github.com/tu-usuario/alejandria.git
cd alejandria

# Crear branch
git checkout -b feature/mi-feature

# Hacer cambios y commit
git commit -am "feat: mi nueva feature"

# Push y crear PR
git push origin feature/mi-feature
```

**Guidelines**:
- Tests obligatorios para nuevas features
- Seguir Rust style guide (`cargo fmt`)
- Documentar funciones públicas
- Actualizar CHANGELOG.md

---

## Licencia

MIT License - ver [LICENSE](../LICENSE) para detalles.

---

## Contacto

- **GitHub**: https://github.com/veritran/alejandria
- **Issues**: https://github.com/veritran/alejandria/issues
- **Email**: appsec@veritran.com

---

**Alejandría** - Memoria persistente para la era de los agentes de IA.

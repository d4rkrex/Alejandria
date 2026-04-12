# Alejandria

> Persistent memory system for AI agents combining sophisticated dual-memory architecture with agent-centric workflows

[![CI](https://github.com/yourusername/alejandria/workflows/CI/badge.svg)](https://github.com/yourusername/alejandria/actions)
[![codecov](https://codecov.io/gh/yourusername/alejandria/branch/main/graph/badge.svg)](https://codecov.io/gh/yourusername/alejandria)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

## Overview

Alejandria is a production-ready memory system for AI agents built in Rust, combining:

- **ICM's sophisticated dual-memory architecture**: Episodic memories with temporal decay + semantic knowledge graphs (memoirs)
- **Engram's agent-centric workflows**: Topic-based organization, deduplication via topic_keys, progressive disclosure
- **Modern search capabilities**: BM25 full-text search (FTS5), with vector similarity search planned via sqlite-vec
- **MCP integration**: Model Context Protocol server for seamless AI agent integration

## Features

### Episodic Memory (Memories)
- ✅ **Temporal decay** with access-aware dampening (Critical/High/Medium/Low importance)
- ✅ **BM25 keyword search**: Full-text search via FTS5 on topic + summary
- ✅ **Automatic deduplication**: Topic-key based upsert with revision tracking
- ✅ **Consolidation**: Merge related memories into high-level summaries
- ✅ **Lifecycle management**: Soft-delete, pruning, decay scheduling
- ✅ **Hybrid search**: BM25 + cosine similarity blending (Phase 3 complete)

### Semantic Memory (Memoirs)
- ✅ **Knowledge graphs**: Named containers with typed concept relations
- ✅ **9 relation types**: IsA, HasProperty, Causes, PrerequisiteOf, ExampleOf, etc.
- ✅ **Graph traversal**: BFS neighborhood inspection with depth control
- ✅ **FTS search**: Full-text search across all concepts or within specific memoirs

### Integration
- ✅ **MCP Server**: 20 tools (11 memory + 9 memoir) via JSON-RPC 2.0 over stdio
- ✅ **CLI**: Full-featured command-line interface with JSON output mode
- ✅ **Embeddings**: fastembed integration complete (Phase 3 — wired into hybrid search pipeline)

## Quick Start

### 30-Second Installation (Recommended)

Get started in under 2 minutes with pre-built binaries:

```bash
curl -fsSL https://raw.githubusercontent.com/mroldan/alejandria/main/scripts/install-mcp-v4.sh | bash
```

The installer automatically:
- Downloads the right binary for your platform (Linux/macOS, Intel/ARM)
- Detects your MCP clients (OpenCode, Claude Desktop, VSCode)
- Configures them with backup/rollback support
- No compilation required!

See [QUICKSTART.md](QUICKSTART.md) for detailed instructions.

### Manual Installation (from source)

If you prefer building from source or need a custom configuration:

```bash
# Clone the repository
git clone https://github.com/yourusername/alejandria.git
cd alejandria

# Build and install CLI
cargo install --path crates/alejandria-cli --features embeddings

# Or build without embeddings (faster, smaller binary)
cargo install --path crates/alejandria-cli
```

### Docker Deployment

**Prerequisites**: Docker 20.10+ (docker-compose 2.0+ optional)

#### Quick Start with Docker

```bash
# Run CLI with example query
docker run --rm alejandria-cli:latest recall "authentication" --limit 5

# Check version
docker run --rm alejandria-cli:latest --version

# View all available commands
docker run --rm alejandria-cli:latest --help
```

#### Running MCP Server with Docker

```bash
# Start MCP server with persistent volume
docker run -d \
  --name alejandria-mcp \
  -v alejandria-data:/data \
  -e ALEJANDRIA_DB_PATH=/data/alejandria.db \
  alejandria-mcp:latest

# Check server logs
docker logs alejandria-mcp

# Stop server
docker stop alejandria-mcp && docker rm alejandria-mcp
```

#### Using Docker Compose

The recommended way to run Alejandria with Docker:

```bash
# Start MCP server in background
docker-compose up -d

# Check service status
docker-compose ps

# View logs
docker-compose logs -f alejandria-mcp

# Run one-off CLI commands
docker-compose run --rm cli recall "authentication" --limit 5
docker-compose run --rm cli topics
docker-compose run --rm cli stats

# Stop all services
docker-compose down
```

**Development mode** with bind mounts (see your data files):

```bash
# Uses docker-compose.dev.yml override
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# Data is now in ./local-data/ directory
ls -la ./local-data/
```

#### Volume Persistence

**Named volumes** (recommended for production):
```bash
# Docker manages storage location
docker volume create alejandria-prod-data
docker run -d -v alejandria-prod-data:/data alejandria-mcp:latest

# Backup volume
docker run --rm -v alejandria-prod-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/alejandria-backup.tar.gz /data

# Restore volume
docker run --rm -v alejandria-prod-data:/data -v $(pwd):/backup \
  alpine tar xzf /backup/alejandria-backup.tar.gz -C /
```

**Bind mounts** (for development):
```bash
# Mount local directory (mind UID/GID permissions)
docker run -d -v $(pwd)/local-data:/data alejandria-mcp:latest

# Fix permissions if needed (container runs as root by default)
docker run --rm -v $(pwd)/local-data:/data alpine chown -R 1000:1000 /data
```

#### Building Images

Using the provided `justfile`:

```bash
# Build both CLI and MCP images
just docker-build

# Build specific image
just docker-build-cli
just docker-build-mcp

# Build with custom tag
TAG=v1.2.3 just docker-buildx  # Multi-platform build (linux/amd64, linux/arm64)

# Clean up all images
just docker-clean
```

Manual builds:

```bash
# Build CLI image
docker build -f Dockerfile.cli -t alejandria-cli:latest .

# Build MCP image
docker build -f Dockerfile.mcp -t alejandria-mcp:latest .

# Multi-platform build (requires buildx)
docker buildx build --platform linux/amd64,linux/arm64 \
  -f Dockerfile.cli -t alejandria-cli:latest .
```

#### Image Size & Optimization

**Current sizes** (Debian/GNU libc base):
- CLI image: **~89MB** (includes ONNX Runtime for embeddings)
- MCP image: **~89MB** (includes ONNX Runtime for embeddings)

**Why larger than typical Rust images?**
- Alejandria uses `fastembed` for local ML embeddings
- `fastembed` requires ONNX Runtime (~50MB of ML inference libraries)
- ONNX Runtime doesn't support Alpine/musl builds (GNU libc required)

**Building without embeddings** (smaller images, ~15-20MB):

```bash
# Modify Dockerfile.cli and Dockerfile.mcp builder stage:
# Change: cargo build --release --bin alejandria
# To:     cargo build --release --bin alejandria --no-default-features

# Then rebuild
docker build -f Dockerfile.cli -t alejandria-cli:slim .
```

**Trade-offs**:
- ✅ Without embeddings: Smaller images, faster builds, pure keyword search (BM25/FTS5)
- ❌ Without embeddings: No future vector similarity search when Phase 3 lands

**Production recommendations**:
- For now, both builds use BM25 keyword search (embeddings feature compiles but is not yet wired into the search pipeline)
- Enable the embeddings feature if you want to be ready for Phase 3 hybrid search
- Disable embeddings for smaller images when you only need keyword search
- Image size is acceptable for most modern deployments (Docker registry compression helps)

See `docs/DEPLOYMENT.md` for production deployment patterns, registry workflows, and security hardening tips.

### Basic Usage

```bash
# Store a memory
alejandria store "Fixed authentication bug in user service" \
  --topic development \
  --importance high

# Recall memories
alejandria recall "authentication" --limit 5

# List topics
alejandria topics

# View statistics
alejandria stats

# Start MCP server (for AI agent integration)
alejandria serve
```

### MCP Client Integration

Integrate Alejandria into your AI agents using the provided client examples in **4 languages**:

| Language | Min Version | Key Features |
|----------|-------------|--------------|
| **Python** | 3.10+ | Official MCP SDK, async/await |
| **TypeScript** | Node 18+ | Type-safe parameters, official SDK |
| **Go** | 1.21+ | Idiomatic patterns, stdlib only |
| **Rust** | 1.70+ | Tokio async, zero-copy deserialization |

**Quick example** (Python):
```python
from client import AlejandriaClient

with AlejandriaClient() as client:
    # Store a memory
    memory_id = client.mem_store(
        content="Learned about Alejandria MCP integration",
        topic="learning",
        importance="high"
    )
    
    # Recall similar memories
    results = client.mem_recall(query="Alejandria", limit=5)
    for memory in results:
        print(f"- {memory['id']}: {memory['summary']} (score: {memory['score']:.2f})")
```

**Get started in 15 minutes**: See [examples/README.md](examples/README.md) for full setup instructions, environment configuration, and complete working examples for all languages.

**What you get**:
- ✅ Memory operations (store, recall, list topics)
- ✅ Memoir operations (create graphs, add concepts, link relations)  
- ✅ Error handling patterns for production use
- ✅ Subprocess lifecycle management
- ✅ Environment-based configuration

### Library Usage

```rust
use alejandria_storage::SqliteStore;
use alejandria_core::{Memory, Importance, MemoryStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open database
    let store = SqliteStore::open("memories.db")?;
    
    // Store a memory
    let mut memory = Memory::new(
        "rust-patterns".to_string(),
        "Builder pattern for complex object construction".to_string(),
    );
    memory.importance = Importance::High;
    memory.topic_key = Some("rust/patterns/builder".to_string());
    
    let id = store.store(memory)?;
    println!("Stored memory: {}", id);
    
    // Search memories
    let results = store.search_by_keywords("builder pattern", 5)?;
    println!("Found {} memories", results.len());
    
    Ok(())
}
```

## Architecture

Alejandria is organized as a 4-crate Cargo workspace:

```
crates/
├── alejandria-core/      # Pure abstractions: types, traits, errors
├── alejandria-storage/   # SQLite backend (FTS5 + sqlite-vec)
├── alejandria-mcp/       # Model Context Protocol server
└── alejandria-cli/       # Command-line interface
```

### Database Schema

- **memories**: Episodic entries with temporal decay (weight, access_count, importance)
- **memories_fts**: FTS5 virtual table for BM25 keyword search
- **vec_memories**: Vector storage for semantic search *(schema exists, populated in Phase 3)*
- **memoirs**: Knowledge graph containers
- **concepts**: Graph nodes with definitions and labels
- **concept_links**: Typed relations between concepts
- **concepts_fts**: FTS5 search for concept definitions

### Search Pipeline

**Hybrid search** is now fully operational, combining keyword and semantic ranking:

1. ✅ **Keyword search** (FTS5): BM25 ranking on topic + summary
2. ✅ **Vector search** (sqlite-vec): Cosine similarity on embeddings
3. ✅ **Score normalization**: Scale both scores to [0, 1]
4. ✅ **Weighted merge**: 30% BM25 + 70% cosine (configurable via `search.bm25_weight` / `search.cosine_weight`)
5. ✅ **Access tracking**: Update access_count and last_accessed
6. ✅ **Decay trigger**: Auto-decay if >24h since last run

When `embeddings` feature is disabled, search automatically falls back to BM25-only mode.

### Temporal Decay Formula

```
effective_rate = base_rate × importance_multiplier / (1 + access_count × 0.1)
new_weight = old_weight × (1 - effective_rate)
```

## Documentation

- **[Architecture Guide](docs/ARCHITECTURE.md)**: Technical deep dive into system design
- **[User Guide](docs/GUIDE.md)**: Detailed workflows and usage patterns
- **[API Reference](https://docs.rs/alejandria-core)**: Rustdoc for all public APIs
- **[MCP Tools](tools/)**: JSON schemas for all MCP tools

## Configuration

Create `~/.config/alejandria/config.toml`:

```toml
[database]
path = "~/.local/share/alejandria/memories.db"

[embeddings]
enabled = true
model = "intfloat/multilingual-e5-base"  # 768 dimensions (Phase 3)
batch_size = 32

[decay]
base_rate = 0.01        # 1% daily decay
min_weight = 0.1        # Prune threshold
auto_decay_hours = 24   # Auto-decay interval

[search]
bm25_weight = 0.3       # Keyword search weight  (Phase 3 hybrid blending)
cosine_weight = 0.7     # Vector similarity weight (Phase 3 hybrid blending)
```

Environment variables override config:
```bash
export ALEJANDRIA_DB_PATH="./custom.db"
export ALEJANDRIA_EMBEDDINGS_ENABLED="false"
```

## Development

### Prerequisites

- Rust 2021 edition (1.70+)
- SQLite 3.35+ with FTS5 support

### Cross-Platform Support

Alejandria builds and runs on **Linux**, **macOS**, and **Windows**:

| Platform | Status | Notes |
|----------|--------|-------|
| Linux (Ubuntu 20.04+) | ✅ Fully tested | 188 tests (183 passing) |
| macOS (10.15+) | ✅ Supported | Install via rustup, uses bundled SQLite |
| Windows (10/11) | ✅ Supported | Requires VS Build Tools or MinGW |

**Platform-Specific Setup:**

<details>
<summary><b>macOS</b></summary>

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Xcode Command Line Tools (if not already installed)
xcode-select --install

# Build Alejandria
cargo build --all-features
cargo test --all-features
```
</details>

<details>
<summary><b>Windows</b></summary>

```powershell
# 1. Install Rust from https://rustup.rs/
# 2. Install Visual Studio Build Tools with C++ workload
#    Download: https://visualstudio.microsoft.com/downloads/

# Build Alejandria
cargo build --all-features
cargo test --all-features
```

**Note**: Use PowerShell or CMD, not Git Bash for best compatibility.
</details>

<details>
<summary><b>Linux</b></summary>

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ubuntu/Debian: SQLite is bundled, but system headers can speed up builds
sudo apt-get install libsqlite3-dev  # Optional

# Build Alejandria
cargo build --all-features
cargo test --all-features
```
</details>

### Build

```bash
# Build all crates
cargo build --all-features

# Run tests
cargo test --all-features

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt --all
```

### Testing

```bash
# Unit + integration tests (188 tests)
cargo test --all-features

# Specific crate
cargo test -p alejandria-storage

# With logging
RUST_LOG=debug cargo test --all-features -- --nocapture
```

### Benchmarks

```bash
# Run benchmarks
cargo bench --all-features

# Specific benchmark
cargo bench --bench performance
```

### Profiling

```bash
# Profile all benchmarks (generates flamegraphs)
./scripts/profiling/profile-benchmarks.sh

# Profile specific benchmark
./scripts/profiling/profile-single.sh hybrid_search

# Compare performance between git refs
./scripts/profiling/compare-profiles.sh main HEAD
```

See [docs/profiling.md](docs/profiling.md) for detailed profiling guide, flamegraph interpretation, and troubleshooting.

## Performance

- **Keyword search** (BM25): <50ms for 10k memories, <200ms for 100k
- **Decay operation**: <2s for 10k memories
- **Binary size**: <50MB with embeddings feature, <10MB without

## Roadmap

### What works now

- **Full episodic memory system**: Store, recall, update, forget, consolidate memories
- **BM25 keyword search**: Fast full-text search via FTS5 with relevance ranking
- **Temporal decay**: 4 configurable decay profiles (exponential, spaced-repetition, importance-weighted, context-sensitive)
- **Knowledge graphs (Memoirs)**: Create, inspect, search, and traverse semantic knowledge graphs with 9 relation types
- **MCP server**: 20 tools over JSON-RPC 2.0 stdio for AI agent integration
- **CLI**: 13 commands with JSON output mode
- **Export/Import**: CSV and JSON formats with conflict resolution
- **Docker**: Production-ready multi-stage builds for CLI and MCP server
- **CI**: Full pipeline with cross-platform tests, clippy, coverage, and security audit

### ✅ Phase 3 — Hybrid Search & Embeddings (Complete)

Hybrid search combining BM25 keyword ranking with vector similarity is now fully implemented:

- ✅ Integrated fastembed (multilingual-e5-base, 768d) into store/recall flow
- ✅ Connected sqlite-vec vector storage to search results  
- ✅ Implemented hybrid score merging (30% BM25 + 70% cosine similarity, configurable)
- ✅ Batch embedding generation for existing memories via `mem_embed_all` MCP tool
- ✅ Exposed embedding controls via MCP (`mem_embed_all`, `mem_health`) and CLI

> All 10 Phase 3 tasks complete. Search now uses hybrid BM25+vector ranking by default when embeddings are enabled, with automatic fallback to BM25-only when disabled.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgments

Alejandria combines ideas from:
- **ICM (Rust)**: Sophisticated dual-memory architecture
- **Engram (Go)**: Agent-centric workflow patterns
- **SQLite FTS5**: Fast full-text search
- **fastembed**: Efficient local embeddings
- **MCP**: Model Context Protocol for agent integration

# Copilot Instructions for Alejandria

> Persistent memory system for AI agents combining ICM's dual-memory architecture with agent-centric workflows

## Project Overview

**Alejandria** is a production-ready Rust memory system combining:
- **ICM's dual-memory architecture**: Episodic memories with temporal decay + semantic knowledge graphs (memoirs)
- **Engram's agent-centric workflows**: Topic-based organization, deduplication, progressive disclosure
- **Modern search**: Hybrid BM25 + vector similarity search
- **MCP integration**: Model Context Protocol server for AI agent integration

**Language**: Rust (edition 2021, min version 1.70+)
**License**: MIT OR Apache-2.0 dual-licensed

## Build, Test, and Lint Commands

### Essential Commands

```bash
# Build all crates
cargo build --all-features

# Build release (optimized)
cargo build --release --all-features

# Run all tests
cargo test --all-features --verbose

# Run tests for specific crate
cargo test -p alejandria-storage
cargo test -p alejandria-core
cargo test -p alejandria-mcp
cargo test -p alejandria-cli

# Run single test by name
cargo test test_hybrid_search_with_boost -- --nocapture

# Run tests with logging enabled
RUST_LOG=debug cargo test --all-features -- --nocapture

# Run doctests only
cargo test --all-features --doc

# Lint with clippy (must pass with zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all

# Check formatting without modifying files
cargo fmt --all -- --check

# Run CLI locally (without installing)
cargo run -p alejandria-cli -- --help
cargo run -p alejandria-cli -- store "Test memory" --topic dev
cargo run -p alejandria-cli -- recall "test" --limit 5

# Generate and view coverage report
make coverage             # Opens in browser
make coverage-no-open     # Generate without opening
make coverage-clean       # Remove artifacts

# Performance profiling (requires cargo-flamegraph + perf)
./scripts/profiling/profile-benchmarks.sh
./scripts/profiling/profile-single.sh hybrid_search
./scripts/profiling/compare-profiles.sh main feature-branch
```

### Using Just (optional)

If `just` is installed (via `cargo install just`):

```bash
just build      # Build release
just test       # Run tests
just lint       # Run clippy
just fmt        # Format code
just check      # Check without building

# Docker builds
just docker-build          # Build CLI + MCP images
just docker-build-cli      # CLI only
just docker-build-mcp      # MCP only
```

## Architecture

### Crate Organization (Layered)

```
alejandria-cli      → User-facing CLI (clap v4)
alejandria-mcp      → MCP server (JSON-RPC 2.0 over stdio)
alejandria-storage  → SQLite implementation (rusqlite + sqlite-vec)
alejandria-core     → Pure traits + types (no I/O)
```

**Key principle**: Dependencies flow downward. Core is pure Rust abstractions with zero I/O.

### Core Traits (alejandria-core)

- **`MemoryStore`**: Episodic memory operations (store, search, decay, consolidate)
- **`MemoirStore`**: Semantic knowledge graph operations (concepts, links, traversal)
- **`Embedder`**: Vector embedding generation (fastembed integration)

### Data Models

**Memory** (episodic):
- Identity: `id` (ULID), `topic`, `topic_key` (for deduplication)
- Content: `summary`, `raw_excerpt`, `keywords`, `embedding` (768d vector)
- Lifecycle: `created_at`, `updated_at`, `last_accessed`, `deleted_at`
- Decay: `weight` (0.0-1.0), `access_count`, `importance` (Critical/High/Medium/Low)

**Memoir** (semantic knowledge graphs):
- Contains `Concept`s with typed `ConceptLink`s
- 9 relation types: IsA, HasProperty, Causes, PrerequisiteOf, ExampleOf, etc.
- FTS search + graph traversal (BFS)

### Storage Layer (alejandria-storage)

- **SQLite + rusqlite** with `bundled` feature (no system dependency)
- **FTS5** virtual tables for BM25 keyword search
- **sqlite-vec** extension for cosine similarity
- **Migrations**: Schema versioning and validation
- **Indexes**: B-tree on timestamps, hash on `topic_key`

### Search Capabilities

1. **BM25 (keyword)**: Full-text search via FTS5 on `topic + summary`
2. **Vector similarity**: Cosine similarity via sqlite-vec (Phase 3 complete)
3. **Hybrid search**: Weighted blend of BM25 + vector scores with configurable boost

## Key Conventions

### Topic-Based Organization

**Topics** are high-level categories (e.g., "rust", "security", "database"). 
**topic_key** is a semantic handle for deduplication (e.g., "rust/error-handling/result-type").

- Use `topic_key` to enable intelligent upsert (automatically merges duplicates)
- When storing a memory with existing `topic_key`, it updates `last_seen_at` and increments `duplicate_count`

### Temporal Decay with Dampening

Memories decay over time based on:
- **Time since last access** (exponential decay)
- **Access frequency** (dampening factor prevents over-prioritizing popular items)
- **Importance level** (Critical memories decay slower)

Formula: `new_weight = old_weight * e^(-decay_rate * days) * dampening_factor`

### Error Handling

- Use `IcmResult<T>` (alias for `Result<T, IcmError>`)
- **Never use `.unwrap()`** in production code
- Prefer `?` operator or explicit error handling
- Use `context()` from `anyhow` for descriptive error chains

### ULID over UUID

- **ULIDs** are lexicographically sortable and embed timestamps
- Generated via `ulid::Ulid::new().to_string()`
- Format: 26 characters, base32-encoded (e.g., `01HZQK8X6F9VKQT2Z3RJGN5CWM`)

### Soft Deletes

- Use `deleted_at: Option<DateTime<Utc>>` instead of hard deletion
- Queries filter `WHERE deleted_at IS NULL` by default
- Enables data recovery and audit trails

### Test Placement

- **Unit tests**: In same file as code, inside `#[cfg(test)] mod tests`
- **Integration tests**: In `crates/<crate>/tests/` directory
- **Doctests**: In `///` doc comments for public APIs

### Coverage Requirements

- **70% minimum** line coverage (enforced in CI via `cargo-tarpaulin`)
- **90% for critical paths** (auth, data storage, security)
- **100% for security fixes**
- CI fails if coverage drops below 70%
- Download coverage reports from failed CI runs (GitHub Actions artifacts)

## Development Workflow

### 1. MANDATORY: Load Skills Before Coding

Alejandria uses **skill-based development**. Check `AGENTS.md` for triggers:

| Skill | Trigger | Path |
|-------|---------|------|
| `alejandria-testing` | Any behavior change | `skills/testing/SKILL.md` |
| `alejandria-tui-quality` | TUI rendering/navigation | `skills/tui-quality/SKILL.md` |
| `alejandria-commit-hygiene` | Creating commits/branches | `skills/commit-hygiene/SKILL.md` |
| `alejandria-memory-discipline` | Session end/decision made | `skills/memory-discipline/SKILL.md` |

**CRITICAL**: Read the relevant skill BEFORE writing code. Skills define mandatory patterns.

### 2. TDD Loop (from alejandria-testing skill)

```
1. RED:    Write failing test for target behavior
2. GREEN:  Implement smallest code to make test pass
3. REFACTOR: Clean up while keeping tests green
4. EDGE:   Add error path and edge case tests
```

### 3. Commit Message Format (from alejandria-commit-hygiene skill)

Follow **Conventional Commits**:

```
<type>(<optional-scope>): <description>

[optional body]

[optional footer(s)]
```

**Types**: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

**Breaking changes**: Add `!` after type or include `BREAKING CHANGE:` footer

**Examples**:
```
feat(storage): add hybrid search with BM25 + vector blending
fix(mcp): handle topic_key null values in mem_store tool
docs(readme): update quick start with Docker instructions
test(core): add edge cases for memory decay with zero weight
```

### 4. Branch Naming

Format: `type/description` (lowercase, kebab-case)

**Examples**:
- `feat/hybrid-search-boost`
- `fix/mcp-null-topic-key`
- `docs/architecture-update`
- `refactor/storage-query-builder`

### 5. PR Checklist

Before submitting:
- [ ] All tests pass (`cargo test --all-features`)
- [ ] Clippy passes with zero warnings
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] Coverage ≥70% (run `make coverage` to verify)
- [ ] Relevant skill(s) loaded and followed
- [ ] Commit messages follow Conventional Commits
- [ ] Documentation updated (if public API changed)
- [ ] No secrets or generated files committed

## TUI Development (Ratatui)

When working on `crates/alejandria-cli/src/commands/tui.rs`:

### Keyboard Conventions (MANDATORY)

- **Arrow keys + vim keys** both work for navigation
  - `j` / `↓` → move down
  - `k` / `↑` → move up
  - `h` / `←` → (reserved)
  - `l` / `→` → (reserved)
- `gg` → jump to first item
- `G` → jump to last item
- `Enter` → select/drill into details
- `Esc` → back/cancel
- `q` → quit (top level only)
- `?` → toggle help overlay
- `Ctrl+T` → cycle themes

### Scroll Behavior (MANDATORY)

Auto-adjust scroll when cursor moves out of visible area:

```rust
if cursor >= scroll_offset + visible_lines {
    scroll_offset = cursor - visible_lines + 1;
}
if cursor < scroll_offset {
    scroll_offset = cursor;
}
// Clamp to valid range
let max_scroll = total_lines.saturating_sub(visible_lines);
scroll_offset = scroll_offset.min(max_scroll);
```

### Empty States (MANDATORY)

Never show empty lists without explanation. Include:
- Icon or header
- Reason why empty
- Next action suggestion

**Example**:
```
┌─ No memories found ─┐
│                     │
│  Try searching with │
│  different keywords │
│                     │
│  Press 'q' to quit  │
└─────────────────────┘
```

See `skills/tui-quality/SKILL.md` for full rules.

## MCP Server

### Communication Protocol

- **Transport**: Stdio (line-delimited JSON)
- **Protocol**: JSON-RPC 2.0
- **Tools**: 20 tools (11 memory + 9 memoir operations)

### Error Codes

- `-32602`: Invalid params
- `-32001`: Database error
- `-32002`: Not found
- `-32603`: Internal error

### Testing MCP Tools

Use `alejandria serve` to start the server, then interact via MCP clients (see `examples/` for Python/TypeScript/Go/Rust clients).

## Performance Considerations

### Embedding Storage

- Embeddings are **768-dimensional f32 vectors** (~3KB per memory)
- Stored as JSON in SQLite (lazy-loaded)
- Enable with `--features embeddings` during build
- Disable for smaller binaries (~15-20MB vs ~89MB)

### Query Optimization

- Use `LIMIT` clauses for large result sets
- Leverage FTS5 indexes for keyword search (faster than LIKE)
- Batch operations when possible (consolidation, decay)

### Profiling

Use provided scripts to identify bottlenecks:

```bash
./scripts/profiling/profile-benchmarks.sh    # All benchmarks → flamegraphs
./scripts/profiling/profile-single.sh <name> # Single benchmark
./scripts/profiling/compare-profiles.sh main feature-branch
```

Requires: `cargo install flamegraph` + `perf` (Linux) or `dtrace` (macOS)

## Docker Deployment

### Image Types

- **alejandria-cli**: Standalone CLI tool (~89MB with embeddings)
- **alejandria-mcp**: MCP server for agent integration (~89MB)

### Quick Start

```bash
# Run CLI
docker run --rm alejandria-cli:latest recall "authentication" --limit 5

# Start MCP server with persistent volume
docker run -d \
  --name alejandria-mcp \
  -v alejandria-data:/data \
  -e ALEJANDRIA_DB_PATH=/data/alejandria.db \
  alejandria-mcp:latest

# Using docker-compose
docker-compose up -d
docker-compose run --rm cli topics
```

See `docs/DEPLOYMENT.md` for production patterns.

## Common Pitfalls

### ❌ Don't use `.unwrap()` or `.expect()`

**Bad**:
```rust
let memory = store.get(&id).unwrap();
```

**Good**:
```rust
let memory = store.get(&id)
    .context("Failed to retrieve memory")?;
```

### ❌ Don't commit without testing

**Always run before committing**:
```bash
cargo test --all-features && cargo clippy --all-targets --all-features
```

### ❌ Don't skip skills

**Bad**: Write code first, check skill later

**Good**: Read relevant skill from `AGENTS.md` BEFORE writing code

### ❌ Don't hardcode paths

**Bad**:
```rust
let db_path = "/home/user/alejandria.db";
```

**Good**:
```rust
let config = Config::load()?;
let db_path = config.expand_db_path()?;
```

### ❌ Don't forget edge cases in tests

Cover:
- Happy path (normal execution)
- Error paths (not found, invalid input)
- Edge cases (empty strings, null values, boundary conditions)
- Security cases (SQL injection, path traversal)

### ❌ Don't ignore clippy warnings

CI fails on clippy warnings (`-D warnings` flag). Fix them before pushing.

## Additional Resources

- **Architecture deep dive**: `docs/ARCHITECTURE.md`
- **Contribution guide**: `CONTRIBUTING.md`
- **Quick start**: `QUICKSTART.md`
- **Agent setup**: `docs/AGENT_INSTRUCTIONS.md`
- **Performance profiling**: `docs/profiling.md`
- **Deployment patterns**: `docs/DEPLOYMENT.md`
- **Database migrations**: `docs/MIGRATIONS.md`

## Release Process (Maintainers Only)

1. Update version in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Create git tag: `git tag -a v0.x.0 -m "Release v0.x.0"`
4. Push tag: `git push origin v0.x.0`
5. CI automatically builds and creates GitHub release

---

**Version**: Alejandria v1.9.5
**Last updated**: 2026-04-13

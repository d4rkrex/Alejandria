# Changelog

All notable changes to Alejandria are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.9.6] — 2026-05-14

### Added
- GitHub Actions CI/CD pipeline with automated release builds
- Pre-built binaries for Linux x86_64, macOS Intel, and macOS Apple Silicon
- One-line installer (`scripts/install.sh`) with GitHub Releases support
- HTTP transport MCP server mode (`alejandria serve --http`)
- SSE (Server-Sent Events) support for real-time MCP streaming
- Hybrid BM25 + vector similarity search with configurable boost
- Temporal decay with dampening factor for episodic memories
- Memoir (semantic knowledge graph) operations: concepts, typed links, BFS traversal
- 20 MCP tools (11 memory + 9 memoir operations)
- Docker support: CLI and MCP server images
- TUI (terminal UI) with vim keybindings, themes, and detail views
- Context-sensitive memory decay based on importance levels

### Changed
- Migrated from internal GitLab to public GitHub
- Binaries now distributed via GitHub Releases (not tracked in repo)
- `alejandria-cli` uses `alejandria-storage` embeddings feature by default

### Fixed
- `Transport` trait import in `serve.rs` for HTTP transport
- Clippy warnings: redundant closures, `push` char vs `push_str`, doc indentation
- macOS BSD `sed` portability in installer (`\s` → `[[:space:]]`)
- Release workflow runner (`macos-13` → `macos-latest` with cross-compilation)

---

## [1.9.0] — Initial public-facing version

### Added
- Core episodic memory system with ULID-based IDs
- SQLite + FTS5 full-text search
- sqlite-vec vector embeddings (768-dimensional)
- Topic-based organization with `topic_key` deduplication
- Soft deletes with `deleted_at` timestamp
- MCP server (JSON-RPC 2.0 over stdio)
- CLI commands: `store`, `recall`, `topics`, `decay`, `consolidate`, `export`, `tui`
- Dual-license: MIT OR Apache-2.0

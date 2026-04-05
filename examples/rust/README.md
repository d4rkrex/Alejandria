# Alejandria Rust MCP Client Example

This directory contains Rust example programs demonstrating how to integrate with the Alejandria MCP server using async Rust, Tokio, and Serde.

## Prerequisites

- **Rust 1.70+** (2021 edition)
- **Alejandria server** built in release mode
- **SQLite database** (created automatically if missing)

## Installation

1. Ensure Alejandria is built:
   ```bash
   cd ../../
   cargo build --release --bin alejandria
   ```

2. Copy environment template:
   ```bash
   cd examples/rust
   cp .env.example .env
   ```

3. Edit `.env` with your paths:
   ```
   ALEJANDRIA_BIN=/path/to/target/release/alejandria
   ALEJANDRIA_DB=/path/to/memories.db
   ```

## Building Examples

```bash
cargo build --release
```

## Running Examples

### Memory Operations Example

Demonstrates storing, recalling, and listing memories:

```bash
cargo run --release --bin example_memory
```

**Expected Output**:
```
Storing 3 memories...
✓ Stored memory #1 with ID: mem_01HZYX...
✓ Stored memory #2 with ID: mem_01HZYY...
✓ Stored memory #3 with ID: mem_01HZYZ...

Recalling memories about 'Rust async patterns'...
Found 2 memories:
  - [mem_01HZYX...] Rust async tutorial (score: 0.92)
  - [mem_01HZYY...] Tokio runtime guide (score: 0.85)

Listing topics...
Topics:
  - rust-learning (2 memories)
  - tokio-patterns (1 memory)
```

### Memoir Operations Example

Demonstrates creating a knowledge graph with linked concepts:

```bash
cargo run --release --bin example_memoir
```

**Expected Output**:
```
Creating memoir: 'Rust Async Architecture'...
✓ Created memoir with ID: memoir_01HZYX...

Adding 5 concepts in parallel...
✓ Added concept: Tokio Runtime (concept_01HZYX...)
✓ Added concept: Futures (concept_01HZYY...)
✓ Added concept: Async/Await (concept_01HZYZ...)
✓ Added concept: Channels (concept_01HZZA...)
✓ Added concept: Select Macro (concept_01HZZB...)

Linking concepts sequentially...
✓ Linked: Tokio Runtime → Futures
✓ Linked: Futures → Async/Await
✓ Linked: Async/Await → Channels
✓ Linked: Channels → Select Macro
```

## Code Structure

- `src/client.rs` - Core async client implementation with JSON-RPC protocol
- `src/example_memory.rs` - Memory operations demonstration
- `src/example_memoir.rs` - Memoir operations demonstration with concurrent concept creation

## Key Features

- **Async/await with Tokio** - Non-blocking I/O for MCP communication
- **Type-safe parameters** - Serde-driven structs for all tool parameters
- **Error handling** - Custom `ClientError` enum using `thiserror`
- **Resource cleanup** - Automatic server process termination on `Drop`
- **Concurrent operations** - Parallel concept creation using `tokio::spawn`

## Troubleshooting

### "No such file or directory" when spawning server

Ensure `ALEJANDRIA_BIN` points to the correct binary path:
```bash
which alejandria  # if installed globally
# or
ls ../../target/release/alejandria  # if using workspace build
```

### "Database is locked" errors

Close any other processes using the database:
```bash
lsof ~/.alejandria/memories.db  # Linux/macOS
```

### Timeout errors during recall

The `mem_recall` operation may timeout with large databases. Example includes 10-second timeout:
```rust
tokio::time::timeout(Duration::from_secs(10), client.mem_recall(...)).await
```

### Build errors with serde

Ensure you have the latest stable Rust toolchain:
```bash
rustup update stable
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `serde` | 1.0 | Serialization framework |
| `serde_json` | 1.0 | JSON encoding/decoding |
| `tokio` | 1.35 | Async runtime with full features |
| `anyhow` | 1.0 | Flexible error handling |
| `thiserror` | 1.0 | Custom error types |
| `dotenv` | 0.15 | Environment variable loading |

## Further Reading

- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Serde Documentation](https://serde.rs/)
- [Alejandria Documentation](../../docs/GUIDE.md)

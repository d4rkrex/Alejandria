# Alejandria MCP Client Examples

Comprehensive example MCP clients demonstrating how to integrate with the Alejandria MCP server from multiple programming languages.

## 15-Minute Quick Start

Pick your language and follow the link to get started in under 15 minutes:

| Language | Min Version | Key Dependencies | Async Model | README |
|----------|-------------|------------------|-------------|---------|
| **Python** | 3.10+ | `mcp`, `python-dotenv` | async/await (internal) | [python/README.md](python/README.md) |
| **Node.js/TypeScript** | Node 18+ | `@modelcontextprotocol/sdk` | async/await | [nodejs/README.md](nodejs/README.md) |
| **Go** | 1.21+ | stdlib only | blocking calls, goroutines | [go/README.md](go/README.md) |
| **Rust** | 1.70+ (2021 edition) | `tokio`, `serde_json` | async/await (tokio) | [rust/README.md](rust/README.md) |

## What These Examples Demonstrate

All clients demonstrate **5 representative MCP tools** covering both memory systems:

### Memory Operations (Episodic Memory)
- `mem_store` - Store memories with optional topic keys for upsert behavior
- `mem_recall` - Hybrid search (BM25 + vector embeddings) for relevant memories  
- `mem_list_topics` - List all topics with memory counts

### Memoir Operations (Semantic Knowledge Graph)
- `memoir_create` - Initialize a new knowledge graph
- `memoir_add_concept` - Add concepts to the graph
- `memoir_link` - Create relationships between concepts

Each example includes:
- ✅ JSON-RPC 2.0 communication over stdio transport
- ✅ Subprocess management for MCP server lifecycle
- ✅ Error handling for connection failures and protocol errors
- ✅ Environment-based configuration (.env files)
- ✅ Clear example scripts showing real-world usage patterns

## Out of Scope

These are **example clients**, not production-ready libraries:
- ❌ Full coverage of all 18 MCP tools (focuses on representative subset)
- ❌ Advanced features like connection pooling or retry logic
- ❌ Authentication/authorization (assumes local, trusted server)
- ❌ Client-side embedding generation (uses server-side embeddings)
- ❌ Performance benchmarking across languages

## Architecture Overview

All clients follow the same pattern:

```
┌─────────────────┐
│ Example Script  │
│  (your code)    │
└────────┬────────┘
         │
         ├──[spawn]──→ alejandria serve (subprocess)
         │
         ├──[stdin]───→ JSON-RPC requests
         │
         └──[stdout]──← JSON-RPC responses
```

**Transport**: Line-delimited JSON over stdin/stdout  
**Protocol**: JSON-RPC 2.0 with `tools/call` method  
**Server**: Built Alejandria binary from `cargo build --release`

## Quick Example

Python example storing and recalling a memory:

```python
from client import AlejandriaClient

with AlejandriaClient() as client:
    # Store a memory
    memory_id = client.mem_store(
        content="Learned how to use MCP protocol",
        topic_key="learning/mcp"
    )
    print(f"✓ Stored memory with ID: {memory_id}")
    
    # Recall similar memories
    results = client.mem_recall(query="MCP protocol", limit=5)
    for memory in results:
        print(f"- [{memory['id']}] {memory['title']} (score: {memory['similarity']:.2f})")
```

## Environment Setup

All clients require two environment variables:

```bash
# Path to Alejandria MCP server binary
export ALEJANDRIA_BIN=/path/to/alejandria/target/release/alejandria

# Path to database (optional, uses default if not set)
export ALEJANDRIA_DB=~/.alejandria/memories.db
```

Each language directory includes a `.env.example` file you can copy and customize:

```bash
cd examples/python
cp .env.example .env
# Edit .env with your paths
```

## Integration Testing

Run all example clients with the integration test script:

```bash
./examples/test_integration.sh
```

This builds the server, runs all four language examples, validates output, and cleans up test data.

## Common Issues

### "MCP server binary not found"
- Verify `ALEJANDRIA_BIN` environment variable is set correctly
- Check the binary exists: `ls -l $ALEJANDRIA_BIN`
- Build if missing: `cargo build --release --bin alejandria`

### "Database locked" errors
- Ensure no other Alejandria processes are running: `ps aux | grep alejandria`
- Check file permissions on the database path
- Try a fresh database with a new `ALEJANDRIA_DB` path

### Import/module not found errors
- Python: Run `pip install -r requirements.txt`
- Node.js: Run `npm install`
- Go: Run `go mod tidy`
- Rust: Dependencies install automatically with `cargo build`

## Contributing New Language Examples

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines on adding examples for additional languages.

## License

Same as main Alejandria project (Apache-2.0 OR MIT).

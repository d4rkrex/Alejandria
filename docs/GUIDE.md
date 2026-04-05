# Alejandria User Guide

Complete guide to using Alejandria for persistent memory management in AI agent workflows.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Memory Operations](#memory-operations)
- [Memoir Operations](#memoir-operations)
- [Advanced Workflows](#advanced-workflows)
- [MCP Integration](#mcp-integration)
- [Troubleshooting](#troubleshooting)

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/yourusername/alejandria.git
cd alejandria

# Build with embeddings (recommended)
cargo install --path crates/alejandria-cli --features embeddings

# Or build without embeddings (faster, smaller binary)
cargo install --path crates/alejandria-cli
```

### Prerequisites

- **Rust** 1.70+ with 2021 edition
- **SQLite** 3.35+ with FTS5 support (usually pre-installed)
- **~250MB disk space** for embedding models (if enabled)

### Verify Installation

```bash
alejandria --version
# Output: alejandria 0.1.0

alejandria --help
# Shows all available commands
```

## Quick Start

### Basic Workflow

```bash
# 1. Store your first memory
alejandria store "Fixed authentication bug in login endpoint" \
  --topic development \
  --importance high

# Output: Stored memory: 01HN5K3V7XBDFGJKPQRSTUVW

# 2. Recall memories
alejandria recall "authentication"

# Output:
# Found 1 memory:
# 
# [01HN5K3V7XBDFGJKPQRSTUVW] (score: 0.95)
# Topic: development | Importance: high | Weight: 1.00
# Created: 2024-01-15 10:30:45 | Accessed: 0 times
# 
# Fixed authentication bug in login endpoint

# 3. List all topics
alejandria topics

# Output:
# Topics (1 total):
# 
# development    1 memory    avg_weight: 1.00

# 4. View statistics
alejandria stats

# Output:
# Total memories: 1 (1 active, 0 deleted)
# Database size: 0.12 MB
# Embeddings: enabled
# ...
```

## Configuration

### Config File

Create `~/.config/alejandria/config.toml`:

```toml
[database]
# Database path (~ expands to home directory)
path = "~/.local/share/alejandria/memories.db"

[embeddings]
# Enable semantic search via embeddings
enabled = true

# Embedding model (768 dimensions, multilingual)
model = "intfloat/multilingual-e5-base"

# Batch size for embedding generation
batch_size = 32

[decay]
# Base decay rate (0.01 = 1% per day)
base_rate = 0.01

# Minimum weight before pruning (0.1 = 10%)
min_weight = 0.1

# Auto-decay interval in hours
auto_decay_hours = 24

[search]
# Hybrid search weights (must sum to 1.0)
bm25_weight = 0.3      # Keyword search weight
cosine_weight = 0.7    # Semantic similarity weight

# Default number of search results
default_limit = 5
```

### Environment Variables

Override config values with environment variables:

```bash
# Database path
export ALEJANDRIA_DB_PATH="./project-memories.db"

# Disable embeddings for faster operation
export ALEJANDRIA_EMBEDDINGS_ENABLED="false"

# Adjust decay rate
export ALEJANDRIA_DECAY_BASE_RATE="0.02"

# Change default search limit
export ALEJANDRIA_SEARCH_DEFAULT_LIMIT="10"
```

Priority: **Environment Variables** > **Config File** > **Built-in Defaults**

## Memory Operations

### Storing Memories

#### Basic Storage

```bash
alejandria store "Implemented JWT-based authentication"
```

#### With Topic and Importance

```bash
alejandria store "Database migration script for user table" \
  --topic database \
  --importance high
```

**Importance Levels**:
- `critical`: Never decays, never pruned
- `high`: Slow decay (0.5x), never pruned
- `medium`: Normal decay (1.0x), pruned at weight < 0.1 **(default)**
- `low`: Fast decay (2.0x), pruned at weight < 0.3

#### With Topic Key (Deduplication)

```bash
# First store
alejandria store "API key: sk-abc123..." \
  --topic-key "service-x-api-key"

# Later update (upserts instead of duplicating)
alejandria store "API key updated: sk-def456..." \
  --topic-key "service-x-api-key"
# Updates existing memory, increments revision_count
```

**Topic Key Format**: Use hierarchical paths like:
- `rust/error-handling/result-type`
- `api/authentication/jwt-flow`
- `config/database/connection-string`

### Searching Memories

#### Basic Search

```bash
alejandria recall "authentication"
```

#### With Filters

```bash
# Limit results
alejandria recall "database" --limit 10

# Filter by topic
alejandria recall "bug" --topic development

# Set minimum relevance score (0.0-1.0)
alejandria recall "API" --min-score 0.7

# Combine filters
alejandria recall "error" \
  --topic development \
  --limit 20 \
  --min-score 0.5
```

#### JSON Output

```bash
alejandria --json recall "authentication" --limit 5

# Output: JSON array of Memory objects
[
  {
    "id": "01HN5K3V7XBDFGJKPQRSTUVW",
    "topic": "development",
    "summary": "Fixed authentication bug",
    "importance": "high",
    "weight": 1.0,
    ...
  }
]
```

### Updating Memories

```bash
# Update summary
alejandria update 01HN5K3V7XBDFGJKPQRSTUVW \
  --summary "Fixed critical authentication bug in login endpoint"

# Change importance
alejandria update 01HN5K3V7XBDFGJKPQRSTUVW \
  --importance critical

# Change topic
alejandria update 01HN5K3V7XBDFGJKPQRSTUVW \
  --topic security

# Multiple updates
alejandria update 01HN5K3V7XBDFGJKPQRSTUVW \
  --summary "New summary" \
  --importance high \
  --topic development
```

### Deleting Memories

```bash
# Soft delete (sets deleted_at timestamp)
alejandria forget 01HN5K3V7XBDFGJKPQRSTUVW
```

Soft-deleted memories:
- Excluded from search results
- Not counted in statistics
- Remain in database (no hard deletion in MVP)

### Topic Management

#### List All Topics

```bash
alejandria topics

# Output:
# Topics (3 total):
# 
# development    15 memories    avg_weight: 0.87
# database        8 memories    avg_weight: 0.92
# security        3 memories    avg_weight: 1.00
```

#### Filter Topics

```bash
# Minimum memory count
alejandria topics --min-count 10

# Pagination
alejandria topics --limit 20 --offset 10
```

### Consolidation

Merge multiple memories in a topic into a high-level summary:

```bash
alejandria consolidate development \
  --min-memories 5 \
  --min-weight 0.5

# Output: Consolidated memory: 01HN5K3V7XBDFGJKPQRSTUVW
```

**How it works**:
1. Fetches all memories in topic with weight >= min_weight
2. Extracts all unique keywords
3. Creates new High-importance memory with:
   - Summary: "Consolidated N memories from topic 'X': keyword1, keyword2, ..."
   - related_ids: List of source memory IDs
   - source: System

### Lifecycle Operations

#### Apply Decay

```bash
# Manual decay (usually automatic)
alejandria decay --rate 0.01

# Output: Decayed 42 memories
```

Decay formula:
```
effective_rate = base_rate × importance_multiplier / (1 + access_count × 0.1)
new_weight = old_weight × (1 - effective_rate)
```

#### Prune Low-Weight Memories

```bash
# Soft-delete memories below threshold
alejandria prune --threshold 0.1

# Output: Pruned 5 memories (Critical and High never pruned)
```

#### Generate Embeddings

```bash
# Embed all memories without embeddings
alejandria embed

# Output: Embedded 50 memories
```

### Statistics

```bash
alejandria stats

# Output:
# Total memories: 127 (120 active, 7 deleted)
# Database size: 15.3 MB
# Embeddings: enabled
# 
# By Importance:
#   Critical: 5
#   High: 23
#   Medium: 87
#   Low: 12
# 
# By Source:
#   User: 115
#   Agent: 8
#   System: 4
# 
# Average weight: 0.78
# Last decay: 2024-01-15 08:00:00 UTC
```

## Memoir Operations

Memoirs are knowledge graphs for permanent, structured knowledge.

### Creating Memoirs

```bash
alejandria memoir create rust-patterns \
  --description "Common Rust design patterns and idioms"

# Output: Created memoir: rust-patterns
```

### Listing Memoirs

```bash
alejandria memoir list

# Output:
# Memoirs (2 total):
# 
# rust-patterns          5 concepts, 7 links
#   Common Rust design patterns and idioms
# 
# api-architecture       12 concepts, 18 links
#   RESTful API design patterns
```

### Viewing Memoir Details

```bash
alejandria memoir show rust-patterns

# Output: Full memoir with all concepts and links
```

### Adding Concepts

```bash
alejandria memoir add-concept rust-patterns \
  --name "Builder Pattern" \
  --definition "A creational design pattern for constructing complex objects step-by-step" \
  --labels "design-pattern,creational"

# Output: Added concept: Builder Pattern
```

### Updating Concepts

```bash
alejandria memoir refine rust-patterns "Builder Pattern" \
  --definition "Updated definition with more details..." \
  --labels "design-pattern,creational,fluent-api"
```

### Linking Concepts

```bash
alejandria memoir link rust-patterns \
  --source "Builder Pattern" \
  --target "Creational Pattern" \
  --relation is_a

# Output: Created link: Builder Pattern is_a Creational Pattern
```

**Relation Types**:
- `is_a`: Taxonomy (e.g., "Rust is_a Programming Language")
- `has_property`: Attributes (e.g., "JWT has_property Stateless")
- `related_to`: Generic association
- `causes`: Causal (e.g., "Memory leak causes Performance degradation")
- `prerequisite_of`: Dependencies
- `example_of`: Instantiation
- `contradicts`: Conflicts
- `similar_to`: Similarity
- `part_of`: Composition

### Searching Concepts

```bash
# Search within specific memoir
alejandria memoir search rust-patterns "builder"

# Search across all memoirs
alejandria memoir search-all "pattern"
```

### Inspecting Concept Neighborhoods

```bash
# View concept and its immediate neighbors
alejandria memoir inspect rust-patterns "Builder Pattern" \
  --depth 1

# Output: Shows concept + all directly connected concepts
```

## Advanced Workflows

### Daily Memory Review

```bash
#!/bin/bash
# review-memories.sh

# Search recent work
alejandria recall "yesterday's work" --limit 10

# Check low-weight memories
alejandria --json stats | jq '.avg_weight'

# Apply decay if needed
alejandria decay --rate 0.01

# View topics needing consolidation
alejandria topics --min-count 10
```

### Project Memory Snapshots

```bash
# Store project snapshot
alejandria store "$(git log --oneline -1)" \
  --topic project-snapshot \
  --importance high \
  --topic-key "project/$(basename $(pwd))/$(date +%Y-%m-%d)"
```

### Automated Memory Capture

```bash
# Git commit hook (.git/hooks/post-commit)
#!/bin/bash
COMMIT_MSG=$(git log -1 --pretty=%B)
alejandria store "$COMMIT_MSG" \
  --topic git-commits \
  --importance medium
```

### Memory Export

```bash
# Export all memories as JSON
alejandria --json recall "" --limit 999999 > memories-backup.json

# Export specific topic
alejandria --json recall "" --topic development --limit 999999 > dev-memories.json
```

### Bulk Operations

```bash
# Generate embeddings for all memories
alejandria embed

# Apply aggressive decay
alejandria decay --rate 0.05

# Prune aggressively
alejandria prune --threshold 0.3

# Consolidate all large topics
for topic in $(alejandria --json topics | jq -r '.[].topic'); do
  alejandria consolidate "$topic" --min-memories 5 --min-weight 0.5
done
```

## MCP Integration

Alejandria implements the Model Context Protocol for AI agent integration.

### Starting the MCP Server

```bash
# Start server (communicates via stdio)
alejandria serve

# Server waits for JSON-RPC 2.0 requests on stdin
# Writes JSON-RPC responses to stdout
```

### Available MCP Tools

**Memory Tools** (9):
1. `store_memory`: Store a new memory
2. `recall_memories`: Search memories
3. `update_memory`: Update existing memory
4. `forget_memory`: Soft-delete memory
5. `list_topics`: List all topics
6. `consolidate_topic`: Consolidate topic
7. `stats`: Get system statistics
8. `health`: Check system health
9. `embed_all`: Generate embeddings

**Memoir Tools** (6):
1. `create_memoir`: Create knowledge graph
2. `list_memoirs`: List all memoirs
3. `get_memoir`: Get memoir details
4. `add_concept`: Add concept to memoir
5. `link_concepts`: Create typed relation
6. `search_concepts`: Search concepts

### Example MCP Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "store_memory",
  "params": {
    "topic": "development",
    "summary": "Implemented caching layer with Redis",
    "importance": "high"
  }
}
```

### Example MCP Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "memory_id": "01HN5K3V7XBDFGJKPQRSTUVW",
    "message": "Memory stored successfully"
  }
}
```

### Integrating with AI Agents

**Claude Desktop** (config.json):
```json
{
  "mcpServers": {
    "alejandria": {
      "command": "alejandria",
      "args": ["serve"]
    }
  }
}
```

**OpenAI Assistants** (via MCP proxy):
```python
import subprocess
import json

# Start MCP server
process = subprocess.Popen(
    ["alejandria", "serve"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    text=True
)

# Send request
request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "store_memory",
    "params": {"topic": "test", "summary": "Test memory"}
}
process.stdin.write(json.dumps(request) + "\n")
process.stdin.flush()

# Read response
response = json.loads(process.stdout.readline())
print(response)
```

### Multi-Language Client Examples

For production-ready client implementations with comprehensive error handling, see the [examples/](../examples/) directory:

- **[Python](../examples/python/)** - Official MCP SDK with async/await patterns
- **[TypeScript](../examples/nodejs/)** - Type-safe interfaces with Node.js SDK
- **[Go](../examples/go/)** - Idiomatic Go with native JSON-RPC implementation
- **[Rust](../examples/rust/)** - Async Tokio-based client with zero-copy deserialization

Each example includes:
- Complete working code for memory and memoir operations
- Environment-based configuration (.env files)
- Error handling patterns for production use
- Subprocess lifecycle management
- Integration test scripts

**Quick start**: Follow the 15-minute setup guide in [examples/README.md](../examples/README.md).

## Troubleshooting

### Database Issues

**Problem**: "Database is locked" error

```bash
# Solution: Close other connections or use WAL mode
sqlite3 ~/.local/share/alejandria/memories.db "PRAGMA journal_mode=WAL;"
```

**Problem**: Corrupted database

```bash
# Solution: Export and reimport
alejandria --json recall "" --limit 999999 > backup.json
rm ~/.local/share/alejandria/memories.db
# Restore from backup programmatically
```

### Embedding Issues

**Problem**: "Model download failed"

```bash
# Solution: Check internet connection, manually download
export ALEJANDRIA_EMBEDDINGS_ENABLED="false"
alejandria embed  # Will skip embeddings
```

**Problem**: Embeddings too slow

```bash
# Solution: Use smaller model or disable
export ALEJANDRIA_EMBEDDINGS_MODEL="intfloat/multilingual-e5-small"
# Or disable entirely
export ALEJANDRIA_EMBEDDINGS_ENABLED="false"
```

### Performance Issues

**Problem**: Search is slow

```bash
# Solution 1: Run VACUUM to optimize database
sqlite3 ~/.local/share/alejandria/memories.db "VACUUM;"

# Solution 2: Prune old memories
alejandria prune --threshold 0.1

# Solution 3: Disable embeddings for faster keyword-only search
export ALEJANDRIA_EMBEDDINGS_ENABLED="false"
```

**Problem**: Large database size

```bash
# Solution: Clean up deleted memories (not in MVP, manual SQL)
sqlite3 ~/.local/share/alejandria/memories.db \
  "DELETE FROM memories WHERE deleted_at IS NOT NULL; VACUUM;"
```

### Common Errors

**Error**: `IcmError::NotFound`

```bash
# Memory ID doesn't exist or is deleted
alejandria recall "" --limit 999999  # Find correct ID
```

**Error**: `IcmError::AlreadyExists`

```bash
# Memoir or concept name already exists
alejandria memoir list  # Check existing names
```

**Error**: `IcmError::InvalidInput`

```bash
# Invalid importance level or relation type
# Valid importance: critical, high, medium, low
# Valid relations: is_a, has_property, related_to, causes, etc.
```

### Debug Mode

```bash
# Enable verbose logging
RUST_LOG=debug alejandria recall "test"

# Output includes SQL queries and timing
```

### Getting Help

```bash
# Command-specific help
alejandria store --help
alejandria memoir --help

# Full documentation
man alejandria  # If installed system-wide
```

## Best Practices

### Memory Hygiene

1. **Use topic_keys** for recurring information (API keys, configs)
2. **Set importance appropriately** (reserve Critical for truly essential)
3. **Run decay regularly** (automatic, but can force with `alejandria decay`)
4. **Consolidate large topics** (keeps database manageable)
5. **Enable embeddings** (significantly improves search quality)

### Topic Organization

```
Good topic names:
  - "development"
  - "security-incidents"
  - "api-documentation"
  - "project-decisions"

Avoid:
  - Too specific: "bug-in-line-42"
  - Too generic: "misc"
  - Inconsistent: "dev" vs "development"
```

### Importance Guidelines

- **Critical**: Security credentials, critical system configs
- **High**: Architectural decisions, important bug fixes
- **Medium**: General development notes, feature implementations
- **Low**: Temporary notes, experimental ideas

### Search Tips

1. **Use specific terms**: "JWT authentication" better than "auth"
2. **Combine filters**: Use --topic, --min-score together
3. **Check weight**: Low-weight results may be stale
4. **Review access_count**: Frequently accessed = important

### Memoir Design

1. **One memoir per domain**: "rust-patterns", "api-architecture"
2. **Hierarchical concepts**: Use IsA relations for taxonomy
3. **Rich labels**: Tag concepts for easier discovery
4. **Bidirectional links**: Link A→B and B→A for traversal
5. **Regular refinement**: Update definitions as understanding evolves

---

**Need more help?** Check the [Architecture Guide](ARCHITECTURE.md) for technical details or open an issue on GitHub.

# Alejandria Python MCP Client Example

Python client demonstrating MCP protocol communication with the Alejandria memory server.

## Prerequisites

- **Python 3.10 or higher**
- **pip** package manager
- **Alejandria MCP server binary** (built from main repository)

## Installation

1. Install Python dependencies:

```bash
cd examples/python
pip install -r requirements.txt
```

2. Configure environment variables:

```bash
cp .env.example .env
# Edit .env with your actual paths
```

Your `.env` file should look like:

```bash
ALEJANDRIA_BIN=/path/to/alejandria/target/release/alejandria
ALEJANDRIA_DB=~/.alejandria/memories.db
```

## Usage

### Memory Operations Example

Demonstrates storing, recalling, and listing memories:

```bash
python example_memory.py
```

**Expected Output:**

```
=== Alejandria Memory Operations Example ===

Storing memory: Meeting notes from project sync
✓ Stored memory with ID: 01J2X3Y4Z5A6B7C8D9

Storing memory: Research findings on vector embeddings
✓ Stored memory with ID: 01J2X3Y4Z5A6B7C8E0

Storing memory: Code review feedback for PR #123
✓ Stored memory with ID: 01J2X3Y4Z5A6B7C8F1

Recalling memories with query: "project"
Found 1 memories:
  - [01J2X3Y4Z5A6B7C8D9] Meeting notes from project sync (score: 0.95)

Listing all topics...
Topics:
  - work/meetings (1 memory)
  - research/ml (1 memory)
  - development/reviews (1 memory)

✓ Example completed successfully
```

### Memoir Operations Example

Demonstrates creating knowledge graphs with concepts and relationships:

```bash
python example_memoir.py
```

**Expected Output:**

```
=== Alejandria Memoir Operations Example ===

Creating memoir: Machine Learning Knowledge Base
✓ Created memoir with ID: memoir_01J2X3Y4Z5

Adding concepts to memoir...
✓ Added concept: Machine Learning (ID: concept_01J2X3Y4Z6)
✓ Added concept: Neural Networks (ID: concept_01J2X3Y4Z7)
✓ Added concept: Deep Learning (ID: concept_01J2X3Y4Z8)
✓ Added concept: Supervised Learning (ID: concept_01J2X3Y4Z9)
✓ Added concept: Unsupervised Learning (ID: concept_01J2X3Y4ZA)

Linking concepts...
✓ Linked: Machine Learning --[includes]--> Neural Networks
✓ Linked: Neural Networks --[enables]--> Deep Learning
✓ Linked: Machine Learning --[includes]--> Supervised Learning
✓ Linked: Machine Learning --[includes]--> Unsupervised Learning

✓ Example completed successfully
```

## Client API

The `AlejandriaClient` class provides a clean interface for MCP operations:

```python
from client import AlejandriaClient

# Initialize client (use as context manager for automatic cleanup)
with AlejandriaClient() as client:
    
    # Store a memory
    memory_id = client.mem_store(
        content="Important information to remember",
        summary="Short summary",
        importance="high",
        topic="category/subcategory",
        topic_key="unique-key-for-upsert"  # Optional: enables upsert behavior
    )
    
    # Recall memories using hybrid search
    memories = client.mem_recall(
        query="search terms",
        limit=10,
        min_score=0.7,
        topic="category"
    )
    
    # List all topics
    topics = client.mem_list_topics()
    
    # Create a memoir (knowledge graph)
    memoir_id = client.memoir_create(
        name="My Knowledge Base",
        description="A structured knowledge graph"
    )
    
    # Add concepts
    concept_id = client.memoir_add_concept(
        memoir_id=memoir_id,
        concept="Concept Name",
        description="Concept description"
    )
    
    # Link concepts
    client.memoir_link(
        memoir_id=memoir_id,
        from_concept=concept_id_1,
        to_concept=concept_id_2,
        relationship="relates_to"
    )
```

## Architecture

The Python client uses the official `mcp` library from Anthropic and communicates with the Alejandria server via stdio transport:

```
┌──────────────────┐
│ example_memory.py│
│   (your code)    │
└────────┬─────────┘
         │
         v
┌────────────────────┐
│ AlejandriaClient   │
│    (client.py)     │
└────────┬───────────┘
         │
         ├──[spawn]──→ alejandria serve (subprocess)
         │
         ├──[stdin]───→ JSON-RPC requests
         │
         └──[stdout]──← JSON-RPC responses
```

## Error Handling

The client handles common error scenarios:

- **FileNotFoundError**: MCP server binary not found at configured path
- **MCPToolError**: Server returned an error (invalid parameters, tool failure, etc.)
- **MCPConnectionError**: Connection to server lost or broken pipe

Example:

```python
try:
    memory_id = client.mem_store(content="Test")
except FileNotFoundError as e:
    print(f"Error: {e}")
    print("Check your ALEJANDRIA_BIN environment variable")
except MCPToolError as e:
    print(f"Tool error: {e.message} (code: {e.code})")
except MCPConnectionError as e:
    print(f"Connection error: {e}")
```

## Common Issues

### "MCP server binary not found"

**Problem**: `FileNotFoundError: MCP server binary not found at: /path/to/binary`

**Solution**:
1. Check that `ALEJANDRIA_BIN` is set: `echo $ALEJANDRIA_BIN`
2. Verify the binary exists: `ls -l $ALEJANDRIA_BIN`
3. Build if missing: `cargo build --release --bin alejandria` from repository root

### "Database locked" error

**Problem**: `MCPToolError: Database is locked`

**Solution**:
1. Check for other Alejandria processes: `ps aux | grep alejandria`
2. Kill any stray processes: `pkill -f alejandria`
3. Try a fresh database: set `ALEJANDRIA_DB` to a new path in `.env`

### Import errors

**Problem**: `ModuleNotFoundError: No module named 'mcp'`

**Solution**:
```bash
pip install -r requirements.txt
```

### Server crashes on startup

**Problem**: Client hangs or reports "Server exited unexpectedly"

**Solution**:
1. Test server manually: `$ALEJANDRIA_BIN serve`
2. Check stderr output for error messages
3. Verify database path is writable: `touch $ALEJANDRIA_DB`

## Development

To modify or extend the client:

1. **Client core** (`client.py`): MCP protocol implementation and subprocess management
2. **Example scripts**: Reference implementations showing usage patterns

The client uses line-delimited JSON over stdio following the MCP specification. Each request is a JSON-RPC 2.0 message sent to stdin, with responses read from stdout.

## Testing

Run the client with a test database:

```bash
export ALEJANDRIA_DB=/tmp/test_memories.db
python example_memory.py
python example_memoir.py
```

Clean up test data:

```bash
rm /tmp/test_memories.db*
```

## Next Steps

- Explore the other [language examples](../README.md) (Node.js, Go, Rust)
- Read the [MCP specification](https://github.com/modelcontextprotocol/specification) for protocol details
- Check out [Alejandria documentation](../../README.md) for server features

## License

Same as main Alejandria project (Apache-2.0 OR MIT).

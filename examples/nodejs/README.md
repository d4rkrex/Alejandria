# Alejandria Node.js/TypeScript MCP Client Example

TypeScript client demonstrating MCP protocol communication with the Alejandria memory server using modern async patterns and type safety.

## Prerequisites

- **Node.js 18 or higher**
- **npm** package manager
- **Alejandria MCP server binary** (built from main repository)

## Installation

1. Install Node.js dependencies:

```bash
cd examples/nodejs
npm install
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
npm run example:memory
```

**Expected Output:**

```
=== Alejandria Memory Operations Example (TypeScript) ===

Storing memory: Meeting notes from project sync
✓ Stored memory with ID: 01J2X3Y4Z5A6B7C8D9

Storing memory: Research findings on vector embeddings
✓ Stored memory with ID: 01J2X3Y4Z5A6B7C8E0

Storing memory: Code review feedback for PR #123
✓ Stored memory with ID: 01J2X3Y4Z5A6B7C8F1

Listing all topics...
Topics:
  - work/meetings (1 memory)
  - research/ml (1 memory)
  - development/reviews (1 memory)

Recalling memories with query: "project"
⚠ Search encountered FTS5 syntax error (known server issue)
Note: This is a server-side bug, not a client error

✓ Example completed successfully
```

### Memoir Operations Example

Demonstrates creating knowledge graphs with concepts and relationships:

```bash
npm run example:memoir
```

**Expected Output:**

```
=== Alejandria Memoir Operations Example (TypeScript) ===

Creating memoir: Machine Learning Knowledge Base
✓ Created memoir: Machine Learning Knowledge Base
  ID: mlkb_01J2X3Y4Z5

Adding 5 concepts in parallel...
✓ Added concept: Machine Learning (ID: concept_01J2X3Y4Z6)
✓ Added concept: Neural Networks (ID: concept_01J2X3Y4Z7)
✓ Added concept: Deep Learning (ID: concept_01J2X3Y4Z8)
✓ Added concept: Supervised Learning (ID: concept_01J2X3Y4Z9)
✓ Added concept: Unsupervised Learning (ID: concept_01J2X3Y4ZA)

Creating relationships...
✓ Linked: Neural Networks --[is_a]--> Machine Learning
✓ Linked: Deep Learning --[is_a]--> Neural Networks
✓ Linked: Supervised Learning --[related_to]--> Unsupervised Learning
✓ Linked: Machine Learning --[has_property]--> Deep Learning

✓ Example completed successfully
```

## Client API

The `AlejandriaClient` class provides a type-safe interface for MCP operations:

```typescript
import { AlejandriaClient } from './client.js';

// Initialize client
const client = new AlejandriaClient();

try {
  // Store a memory
  const result = await client.memStore({
    content: 'Important information to remember',
    summary: 'Short summary',
    importance: 'high',
    topic: 'category/subcategory',
    topic_key: 'unique-key-for-upsert'  // Optional: enables upsert behavior
  });
  console.log(`Memory ID: ${result.id}, Action: ${result.action}`);
  
  // Recall memories using hybrid search
  const memories = await client.memRecall({
    query: 'search terms',
    limit: 10,
    min_score: 0.7,
    topic: 'category'
  });
  
  // List all topics
  const topics = await client.memListTopics();
  
  // Create a memoir (knowledge graph)
  const memoir = await client.memoirCreate(
    'My Knowledge Base',
    'A structured knowledge graph'
  );
  console.log(`Memoir ID: ${memoir.id}`);
  
  // Add concepts (in parallel using Promise.all)
  const concepts = await Promise.all([
    client.memoirAddConcept('My Knowledge Base', 'Concept 1', 'Description 1'),
    client.memoirAddConcept('My Knowledge Base', 'Concept 2', 'Description 2'),
    client.memoirAddConcept('My Knowledge Base', 'Concept 3', 'Description 3')
  ]);
  
  // Link concepts (use snake_case for relation types)
  await client.memoirLink({
    memoir: 'My Knowledge Base',
    source: 'Concept 1',
    target: 'Concept 2',
    relation: 'is_a'  // Use snake_case: is_a, has_property, related_to
  });
  
} finally {
  // Clean shutdown
  await client.close();
}
```

### TypeScript Type Definitions

The client provides full type safety with interfaces for all operations:

```typescript
// Request parameter types
interface MemStoreParams {
  content: string;
  summary?: string;
  importance?: 'critical' | 'high' | 'medium' | 'low';
  topic?: string;
  topic_key?: string;
  source?: string;
  related_ids?: string[];
}

interface MemRecallParams {
  query: string;
  limit?: number;
  min_score?: number;
  topic?: string;
}

interface MemoirLinkParams {
  memoir: string;      // Memoir name (not ID)
  source: string;      // Source concept name (not ID)
  target: string;      // Target concept name (not ID)
  relation: string;    // Relation type (snake_case: is_a, has_property, related_to)
}

// Response types
interface MemStoreResponse {
  id: string;
  action: 'created' | 'updated';
}

interface Memoir {
  id: string;
  name: string;
  description: string;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

interface Concept {
  id: string;
  memoir_id: string;
  name: string;
  definition: string;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

interface Memory {
  id: string;
  content: string;
  summary: string;
  importance: string;
  topic: string;
  score: number;
}

interface Topic {
  topic: string;
  count: number;
}
```

## Architecture

The TypeScript client uses custom JSON-RPC 2.0 implementation over stdio transport:

```
┌──────────────────────┐
│ exampleMemory.ts     │
│ exampleMemoir.ts     │
│   (your code)        │
└──────────┬───────────┘
           │
           v
┌──────────────────────┐
│ AlejandriaClient     │
│    (client.ts)       │
│  - JSON-RPC 2.0      │
│  - stdio transport   │
│  - Type safety       │
└──────────┬───────────┘
           │
           ├──[spawn]──→ alejandria serve (subprocess)
           │
           ├──[stdin]───→ JSON-RPC requests
           │
           └──[stdout]──← JSON-RPC responses
```

### Key Implementation Details

1. **Dual Response Format Handling**: The client handles two different response structures:
   - Memory tools (`mem_store`, `mem_recall`, `mem_list_topics`): MCP-wrapped responses with `{content: [{type: "text", text: "..."}]}`
   - Memoir tools (`memoir_create`, `memoir_add_concept`, `memoir_link`): Direct JSON object responses

2. **ES Modules**: Uses modern ES module syntax with `.js` extensions in import paths (TypeScript requirement)

3. **Async/Await**: All operations use async/await for clean asynchronous code

4. **Signal Handlers**: Graceful shutdown on SIGINT (Ctrl+C) and SIGTERM

5. **Line-Buffered I/O**: Uses readline for proper JSON message parsing from stdout

## Error Handling

The client provides custom error classes for different failure scenarios:

```typescript
try {
  const result = await client.memStore({ content: 'Test' });
} catch (error) {
  if (error instanceof MCPToolError) {
    console.error(`Tool error: ${error.message} (code: ${error.code})`);
  } else if (error instanceof MCPConnectionError) {
    console.error(`Connection error: ${error.message}`);
  } else {
    console.error(`Unexpected error: ${error}`);
  }
}
```

### Error Classes

- **MCPToolError**: Server returned an error (invalid parameters, tool failure, etc.)
  - Properties: `message`, `code`, `data`
- **MCPConnectionError**: Connection to server lost or broken pipe
  - Thrown when subprocess exits unexpectedly or communication fails

## Common Issues

### "MCP server binary not found"

**Problem**: `Error: spawn ENOENT` or similar spawn errors

**Solution**:
1. Check that `ALEJANDRIA_BIN` is set: `echo $ALEJANDRIA_BIN`
2. Verify the binary exists: `ls -l $ALEJANDRIA_BIN`
3. Build if missing: `cargo build --release --bin alejandria` from repository root
4. Ensure `.env` file is in `examples/nodejs/` directory

### FTS5 Search Syntax Error

**Problem**: `mem_recall` fails with "fts5: syntax error near '?'"

**Solution**: This is a **known server-side bug**, not a client error. The server's FTS5 query construction has issues with certain search terms. The client handles this gracefully and reports the error. This affects all language clients (Python, TypeScript, etc.).

### "Relation type not found" error

**Problem**: `memoir_link` fails with relation type errors

**Solution**: The server expects **snake_case** relation types despite the tool schema showing PascalCase. Use:
- `is_a` (not `IsA`)
- `has_property` (not `HasProperty`)
- `related_to` (not `RelatedTo`)

This is a documentation bug in the tool schema.

### "Cannot find module" errors

**Problem**: Import errors when running examples

**Solution**:
```bash
npm install
```

Ensure all dependencies are installed, including TypeScript 5.6.3 and `tsx`.

### TypeScript compilation errors

**Problem**: Type errors or ES module issues

**Solution**: The project uses:
- ES2020 target with ES modules
- `.js` extensions in imports (required for ES modules in TypeScript)
- `tsx` for running TypeScript directly without compilation

Don't run `tsc` manually; use `npm run example:memory` or `npm run example:memoir`.

### Parameter type confusion

**Problem**: Memoir operations fail with "memoir not found" or "concept not found"

**Solution**: Memoir tools use **entity names** (not IDs) for referencing:
- `memoirAddConcept(memoir_name, concept_name, definition)` - use memoir NAME
- `memoirLink({ memoir: memoir_name, source: concept_name, target: concept_name })` - use NAMES

This is by design for the semantic/knowledge-graph approach.

## Development

To modify or extend the client:

1. **Client core** (`src/client.ts`): JSON-RPC 2.0 protocol, subprocess management, type definitions
2. **Example scripts** (`src/exampleMemory.ts`, `src/exampleMemoir.ts`): Reference implementations

### Running TypeScript Directly

The project uses `tsx` to run TypeScript without compilation:

```bash
npm run example:memory   # Runs tsx src/exampleMemory.ts
npm run example:memoir   # Runs tsx src/exampleMemoir.ts
```

### Type Checking

```bash
npx tsc --noEmit
```

## Testing

Run the client with a test database:

```bash
export ALEJANDRIA_DB=/tmp/test_memories.db
npm run example:memory
npm run example:memoir
```

Clean up test data:

```bash
rm /tmp/test_memories.db*
```

## Advanced Usage

### Parallel Operations with Promise.all

The client's async nature allows efficient parallel operations:

```typescript
// Add multiple concepts in parallel
const concepts = await Promise.all([
  client.memoirAddConcept('MyMemoir', 'Concept1', 'Definition1'),
  client.memoirAddConcept('MyMemoir', 'Concept2', 'Definition2'),
  client.memoirAddConcept('MyMemoir', 'Concept3', 'Definition3')
]);

// Create multiple memories in parallel
const results = await Promise.all([
  client.memStore({ content: 'Memory 1', topic: 'topic1' }),
  client.memStore({ content: 'Memory 2', topic: 'topic2' }),
  client.memStore({ content: 'Memory 3', topic: 'topic3' })
]);
```

### Custom Request Timeout

The client uses a 30-second timeout by default. For long-running operations, you may need to extend this (future enhancement).

### Graceful Shutdown

The client automatically handles SIGINT and SIGTERM for clean shutdown:

```typescript
process.on('SIGINT', async () => {
  console.log('\nShutting down gracefully...');
  await client.close();
  process.exit(0);
});
```

## Next Steps

- Explore the other [language examples](../README.md) (Python, Go, Rust)
- Read the [MCP specification](https://github.com/modelcontextprotocol/specification) for protocol details
- Check out [Alejandria documentation](../../README.md) for server features
- See [client.ts](./src/client.ts) for complete type definitions and implementation

## License

Same as main Alejandria project (Apache-2.0 OR MIT).

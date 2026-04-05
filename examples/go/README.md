# Alejandria Go Client

Pure Go client implementation for the Alejandria MCP (Model Context Protocol) server. This client demonstrates JSON-RPC 2.0 communication over stdio transport using only the Go standard library.

## Features

- **Stdlib-only implementation** - No external dependencies beyond Go standard library
- **Full MCP support** - Implements complete JSON-RPC 2.0 protocol for Memory and Memoir operations
- **Idiomatic Go** - Demonstrates Go best practices:
  - Context for cancellation and timeouts
  - Goroutines with WaitGroup for concurrent operations
  - Proper error handling with `error` interface
  - Resource cleanup with `defer`
  - Struct tags for JSON serialization
- **Type-safe API** - Strongly-typed parameter structs and response types
- **Graceful shutdown** - Signal handling (SIGINT/SIGTERM) for clean process termination

## Prerequisites

- **Go 1.21 or later**
- **Alejandria MCP server** - Built from the main Rust codebase:
  ```bash
  cd ../..
  cargo build --release
  ```

## Project Structure

```
examples/go/
├── go.mod                          # Go module definition
├── pkg/
│   └── client/
│       └── client.go               # Core MCP client (442 lines)
├── cmd/
│   ├── example_memory/
│   │   └── main.go                 # Memory operations demo (126 lines)
│   └── example_memoir/
│       └── main.go                 # Memoir knowledge graph demo (167 lines)
├── .env.example                    # Environment variable template
└── README.md                       # This file
```

## Installation

1. **Clone and build the Alejandria server**:
   ```bash
   git clone <alejandria-repo-url>
   cd Alejandria
   cargo build --release
   ```

2. **Set up environment variables**:
   ```bash
   cd examples/go
   cp .env.example .env
   # Edit .env to set ALEJANDRIA_BIN and ALEJANDRIA_DB paths
   ```

3. **Build the example programs**:
   ```bash
   # Build both examples
   go build ./cmd/example_memory
   go build ./cmd/example_memoir
   
   # Or build all at once
   go build ./...
   ```

## Usage

### Memory Operations Example

Demonstrates storing, recalling, and listing memories by topic:

```bash
# Load environment variables and run
source .env
./example_memory
```

**What it does:**
1. Initializes MCP client with stdio transport
2. Stores 3 memories about Go programming concepts
3. Attempts to recall memories (gracefully handles known FTS5 bug)
4. Lists all topics with memory counts

**Expected Output:**
```
Initializing Alejandria MCP client...
✓ Client initialized successfully

=== Storing Memories ===
✓ Stored memory 1 with ID: mem_123...
✓ Stored memory 2 with ID: mem_456...
✓ Stored memory 3 with ID: mem_789...

=== Recalling Memories ===
⚠ Recall failed (known FTS5 bug): rpc error ...
  This is a server-side issue and does not affect storage operations

=== Listing Topics ===
Found 1 topics:
  - golang: 3 memories

✓ Memory operations completed successfully!
```

### Memoir Knowledge Graph Example

Demonstrates creating a knowledge graph with concurrent concept addition and sequential linking:

```bash
# Load environment variables and run
source .env
./example_memoir
```

**What it does:**
1. Creates a memoir named "Programming Paradigms"
2. Adds 5 programming paradigm concepts **concurrently** using goroutines
3. Links concepts with relationships (`is_a`, `related_to`)
4. Demonstrates idiomatic Go concurrency patterns

**Expected Output:**
```
Initializing Alejandria MCP client...
✓ Client initialized successfully

=== Creating Memoir ===
✓ Created memoir with ID: Programming Paradigms

=== Adding Concepts (concurrent) ===
✓ Added concept: Functional Programming (ID: concept_123...)
✓ Added concept: Object-Oriented Programming (ID: concept_456...)
✓ Added concept: Procedural Programming (ID: concept_789...)
✓ Added concept: Declarative Programming (ID: concept_abc...)
✓ Added concept: Imperative Programming (ID: concept_def...)

=== Linking Concepts ===
✓ Linked: Functional Programming -> is_a -> Declarative Programming
✓ Linked: Object-Oriented Programming -> is_a -> Imperative Programming
✓ Linked: Procedural Programming -> is_a -> Imperative Programming
✓ Linked: Functional Programming -> related_to -> Object-Oriented Programming

✓ Memoir operations completed successfully!
  Created memoir 'Programming Paradigms' with 5 concepts and 4 relationships
```

## API Reference

### Client Initialization

```go
import "alejandria-go-examples/pkg/client"

ctx := context.Background()
c, err := client.NewAlejandriaClient(ctx, serverPath, dbPath)
if err != nil {
    log.Fatal(err)
}
defer c.Close()
```

### Memory Operations

```go
// Store a memory
memoryID, err := c.MemStore(ctx, client.MemStoreParams{
    Content:    "Memory content here",
    Summary:    "Brief summary",
    Importance: "high", // "high", "medium", or "low"
    Topic:      "golang",
    TopicKey:   "go/learning",
    Source:     "my-app",
})

// Recall memories (FTS5 search)
memories, err := c.MemRecall(ctx, client.MemRecallParams{
    Query: "search terms",
    Limit: 10,
})

// List topics
topics, err := c.MemListTopics(ctx)
```

### Memoir Operations

```go
// Create a memoir
memoirID, err := c.MemoirCreate(ctx, client.MemoirCreateParams{
    Name:        "My Knowledge Graph",
    Description: "Description here",
})

// Add a concept
conceptID, err := c.MemoirAddConcept(ctx, client.MemoirAddConceptParams{
    Memoir:     memoirID,
    Concept:    "Concept Name",
    Definition: "Concept definition",
})

// Link concepts
err = c.MemoirLink(ctx, client.MemoirLinkParams{
    Memoir:       memoirID,
    FromConcept:  "Concept A",
    ToConcept:    "Concept B",
    Relationship: "is_a", // "is_a", "related_to", "has_property"
})
```

## Configuration

The client reads these environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `ALEJANDRIA_BIN` | `./target/release/alejandria` | Path to Alejandria server binary |
| `ALEJANDRIA_DB` | *(none)* | Optional: Path to SQLite database file |

## Architecture Notes

### Dual Response Format Handling

The client automatically handles two response formats:

1. **MCP-wrapped format** (Memory tools):
   ```json
   {
     "content": [
       {"type": "text", "text": "{\"id\":\"mem_123\"}"}
     ]
   }
   ```

2. **Direct JSON format** (Memoir tools):
   ```json
   {"id": "memoir_123", "name": "My Memoir"}
   ```

The `callTool()` method tries MCP format first, then falls back to direct parsing.

### Concurrency Pattern

The memoir example demonstrates idiomatic Go concurrency:

```go
var wg sync.WaitGroup
for i, item := range items {
    wg.Add(1)
    go func(idx int, data SomeType) {
        defer wg.Done()
        // Do concurrent work here
    }(i, item)
}
wg.Wait() // Wait for all goroutines to complete
```

### Error Handling

Errors are wrapped with context using `fmt.Errorf`:

```go
if err != nil {
    return fmt.Errorf("failed to parse response: %w", err)
}
```

This preserves the error chain for debugging.

## Troubleshooting

### "exec: \"alejandria\": executable file not found"

**Solution**: Set `ALEJANDRIA_BIN` to the correct path:
```bash
export ALEJANDRIA_BIN=/full/path/to/alejandria/target/release/alejandria
```

### "FTS5 syntax error" when recalling memories

**Status**: Known server-side bug in `mem_recall` tool. This is documented and does not affect storage operations. The examples handle this gracefully with a warning message.

### Goroutines not running concurrently

**Check**: Ensure you're using `go func()` with proper closure variables:
```go
// Correct - passes variables to goroutine
go func(idx int, val string) { ... }(i, value)

// Wrong - uses loop variables directly (race condition)
go func() { use(i, value) }()
```

### Context deadline exceeded

**Solution**: Increase timeouts in the client or check server responsiveness. The client uses a 5-second shutdown timeout by default.

## Comparison with Other Clients

| Feature | Python | Node.js/TypeScript | Go |
|---------|--------|-------------------|-----|
| Lines of code | 456 | 525 | 442 |
| Dependencies | stdlib only | stdlib only | stdlib only |
| Type safety | Runtime (minimal) | Compile-time (TypeScript) | Compile-time |
| Concurrency model | asyncio | Promises/async-await | Goroutines |
| Error handling | Exceptions | try/catch | Error values |
| JSON serialization | Built-in | Built-in | Struct tags |

## Development

### Running Tests

```bash
go test ./...
```

### Code Style

The code follows standard Go conventions:
- `gofmt` formatting
- Exported types/functions start with uppercase
- Struct fields use `json` tags for serialization
- Error messages use lowercase (no trailing punctuation)

### Adding New MCP Tools

1. Add parameter struct in `client.go`:
   ```go
   type MyNewToolParams struct {
       Field string `json:"field"`
   }
   ```

2. Add method to `AlejandriaClient`:
   ```go
   func (c *AlejandriaClient) MyNewTool(ctx context.Context, params MyNewToolParams) (string, error) {
       return c.callTool(ctx, "my_new_tool", params)
   }
   ```

3. Handle response format in `callTool()` if needed.

## License

Same as main Alejandria project.

## Contributing

Contributions welcome! Please follow Go best practices and maintain stdlib-only approach.

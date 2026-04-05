# MCP Tool Schemas

This directory contains JSON schema definitions for all Alejandria MCP (Model Context Protocol) tools.

## Memory Tools

### Core Operations
- **mem_store.json** - Store a new memory or update existing via topic_key upsert
- **mem_recall.json** - Search and recall memories using hybrid search (BM25 + vector similarity)
- **mem_update.json** - Update an existing memory by ID
- **mem_forget.json** - Soft-delete a memory by ID

### Organization & Maintenance
- **mem_consolidate.json** - Consolidate memories in a topic into a high-level summary
- **mem_list_topics.json** - List all topics with counts and statistics
- **mem_stats.json** - Get memory statistics
- **mem_health.json** - Check system health (database, FTS, vector search, embeddings)
- **mem_embed_all.json** - Batch embed existing memories that lack embeddings

## Memoir Tools (Knowledge Graph)

### Graph Management
- **memoir_create.json** - Create a new memoir (knowledge graph)
- **memoir_list.json** - List all memoirs with concept and link counts
- **memoir_show.json** - Get full memoir graph with all concepts and links

### Concept Operations
- **memoir_add_concept.json** - Add a concept to a memoir
- **memoir_refine.json** - Update concept definition and/or labels
- **memoir_search.json** - Search concepts within a memoir using FTS5
- **memoir_search_all.json** - Search concepts across all memoirs using FTS5

### Link Operations
- **memoir_link.json** - Create typed link between two concepts
- **memoir_inspect.json** - Inspect concept neighborhood using BFS traversal

## Schema Format

All schemas follow the MCP tool schema format:

```json
{
  "name": "tool_name",
  "description": "Tool description",
  "inputSchema": {
    "type": "object",
    "properties": {
      "param_name": {
        "type": "string|integer|number|boolean|array",
        "description": "Parameter description",
        "default": "optional_default_value"
      }
    },
    "required": ["param1", "param2"]
  }
}
```

## Usage

These schemas are used by:
1. The MCP server (`alejandria-mcp`) to validate tool calls
2. AI agents to understand available tools and their parameters
3. Documentation generation
4. Client libraries for type checking

For implementation details, see `crates/alejandria-mcp/src/server.rs` and `crates/alejandria-mcp/src/tools/`.

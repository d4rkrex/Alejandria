# Alejandria Agent Instructions

**Version:** 1.0.0  
**Last Updated:** 2026-04-04

This document provides complete instructions for AI agents using Alejandria, a persistent memory system with episodic memories, temporal decay, and semantic knowledge graphs. Use this as your system prompt to enable long-term memory capabilities.

---

## Overview

Alejandria is a **persistent memory system** that allows you to:

1. **Store and recall memories** across sessions using hybrid search (BM25 + vector similarity)
2. **Organize knowledge** into topics with automatic temporal decay
3. **Build knowledge graphs** (memoirs) with typed relationships between concepts
4. **Consolidate information** to prevent memory fragmentation
5. **Track importance** with decay profiles (critical memories persist longer)

**Key Architecture:**
- **18 MCP tools** via JSON-RPC 2.0
- **Hybrid search:** BM25 (keyword) + 768d vector embeddings (semantic)
- **Temporal decay:** Memories fade over time based on importance and access patterns
- **Knowledge graphs:** Build and traverse concept networks with 9 relation types

---

## Memory Tools (9 tools)

### Core Operations

#### `mem_store` - Store or Update Memory

**When to use:**
- Store new information you need to remember across sessions
- Update existing memory using `topic_key` for idempotent upserts
- Capture important decisions, discoveries, or learned patterns

**Parameters:**
```json
{
  "content": "string (required) - Full memory content",
  "summary": "string (optional) - Brief summary for quick reference",
  "importance": "critical|high|medium|low (optional) - Affects decay rate",
  "topic": "string (optional) - Organizational topic",
  "topic_key": "string (optional) - Unique key for upsert (same key updates existing)",
  "source": "string (optional) - Source identifier",
  "related_ids": ["id1", "id2"] (optional) - Related memory IDs
}
```

**Best practices:**
- Use `topic_key` for evolving information (e.g., "project/architecture", "user/preferences")
- Set `importance: "critical"` for decisions, `"high"` for discoveries, `"medium"` for patterns
- Include `summary` for faster recall
- Group related memories with consistent `topic` values

**Example:**
```json
{
  "content": "User prefers React over Vue for new projects. Reason: team expertise and ecosystem maturity.",
  "summary": "User tech preference: React",
  "importance": "high",
  "topic": "user_preferences",
  "topic_key": "preferences/frontend_framework"
}
```

---

#### `mem_recall` - Search Memories

**When to use:**
- User asks to recall something ("remember when...", "what did we decide...")
- Starting a new task related to previous work
- Need context from past sessions

**Parameters:**
```json
{
  "query": "string (required) - Search query (natural language or keywords)",
  "limit": "integer (optional, default: 10) - Max results",
  "min_score": "number (optional, default: 0.0) - Min similarity (0.0-1.0)",
  "topic": "string (optional) - Filter by topic"
}
```

**Search strategy:**
1. **Hybrid search:** Combines BM25 (keyword matching) + vector similarity (semantic)
2. **Automatic fallback:** If vector search fails, falls back to BM25-only
3. **Temporal decay:** Recent and important memories rank higher

**Best practices:**
- Use natural language queries: "authentication implementation details"
- Add `topic` filter when you know the category: `"topic": "bugs"`
- Set `min_score: 0.3` to filter low-relevance results
- Results include `weight` (0.0-1.0) indicating relevance after decay

**Example:**
```json
{
  "query": "bug fix for database connection timeout",
  "limit": 5,
  "topic": "bugs",
  "min_score": 0.2
}
```

---

#### `mem_update` - Update Existing Memory

**When to use:**
- Correct or refine a specific memory by ID
- Add new information to existing memory
- Change importance level

**Parameters:**
```json
{
  "id": "string (required) - Memory ID (ULID format)",
  "content": "string (optional) - New content",
  "summary": "string (optional) - New summary",
  "importance": "critical|high|medium|low (optional) - New importance",
  "topic": "string (optional) - New topic"
}
```

**Note:** Use `mem_store` with `topic_key` for upserts. Use `mem_update` when you have the exact memory ID and need to modify it directly.

---

#### `mem_forget` - Delete Memory

**When to use:**
- Remove incorrect or obsolete information
- Clean up temporary memories
- User requests deletion

**Parameters:**
```json
{
  "id": "string (required) - Memory ID to delete"
}
```

**Note:** This is a **soft delete**. Memory is marked as deleted but not physically removed.

---

### Organization & Maintenance

#### `mem_list_topics` - List All Topics

**When to use:**
- Explore what topics exist
- Find related memories by topic
- Understand memory organization

**Parameters:**
```json
{
  "limit": "integer (optional, default: 100) - Max topics",
  "offset": "integer (optional, default: 0) - Pagination offset",
  "min_count": "integer (optional, default: 1) - Min memory count per topic"
}
```

**Returns:** Topic names with memory counts and average importance.

---

#### `mem_consolidate` - Consolidate Topic

**When to use:**
- Topic has many fragmented memories (>10)
- Need a high-level summary of a topic
- Reduce memory clutter

**Parameters:**
```json
{
  "topic": "string (required) - Topic to consolidate",
  "min_weight": "number (optional, default: 0.5) - Min weight threshold",
  "min_memories": "integer (optional, default: 3) - Min memories required"
}
```

**What it does:**
1. Loads all memories in topic with `weight >= min_weight`
2. Creates a consolidated summary memory
3. Original memories remain but can be cleaned up later

**Best practice:** Run periodically on active topics to maintain organized memory.

---

#### `mem_stats` - Get Statistics

**When to use:**
- Check memory system health
- Monitor memory growth
- Debug issues

**Returns:**
```json
{
  "total_memories": 1234,
  "active_memories": 1150,
  "deleted_memories": 84,
  "total_topics": 42,
  "avg_importance": "medium",
  "embedder_model": "multilingual-e5-base"
}
```

---

#### `mem_health` - System Health Check

**When to use:**
- Startup verification
- Debug search/embedding issues
- Monitor system status

**Returns:**
```json
{
  "database": "ok",
  "fts_index": "ok",
  "vector_search": "ok",
  "embeddings": "ok",
  "details": {...}
}
```

---

#### `mem_embed_all` - Batch Embed Memories

**When to use:**
- After importing memories without embeddings
- Upgrading from keyword-only to hybrid search
- Fixing missing embeddings

**Parameters:**
```json
{
  "batch_size": "integer (optional, default: 100) - Batch processing size",
  "skip_existing": "boolean (optional, default: true) - Skip memories with embeddings"
}
```

**Note:** This is a maintenance operation. Normal `mem_store` calls automatically generate embeddings.

---

## Memoir Tools (Knowledge Graph - 9 tools)

Memoirs are **knowledge graphs** that capture semantic relationships between concepts. Use memoirs for:
- Building domain knowledge models
- Tracking concept hierarchies
- Discovering hidden relationships
- Organizing structured knowledge

### Graph Management

#### `memoir_create` - Create New Memoir

**When to use:**
- Start a new knowledge domain
- Organize concepts into a separate graph

**Parameters:**
```json
{
  "name": "string (required, unique) - Memoir name",
  "description": "string (optional) - Memoir purpose"
}
```

**Example:**
```json
{
  "name": "architecture_patterns",
  "description": "Software architecture patterns and relationships"
}
```

---

#### `memoir_list` - List All Memoirs

**When to use:**
- Discover existing knowledge graphs
- Check memoir sizes

**Returns:** List of memoirs with concept and link counts.

---

#### `memoir_show` - Get Full Memoir Graph

**When to use:**
- Visualize entire knowledge graph
- Export memoir for analysis
- Debug graph structure

**Parameters:**
```json
{
  "name": "string (required) - Memoir name"
}
```

**Returns:** All concepts with definitions, labels, and all links with relations.

---

### Concept Operations

#### `memoir_add_concept` - Add Concept

**When to use:**
- Add new concept to knowledge graph
- Define domain terms

**Parameters:**
```json
{
  "memoir": "string (required) - Memoir name",
  "name": "string (required) - Concept name",
  "definition": "string (optional) - Concept definition",
  "labels": ["label1", "label2"] (optional) - Categorization labels
}
```

**Example:**
```json
{
  "memoir": "architecture_patterns",
  "name": "Microservices",
  "definition": "Architectural style structuring application as collection of loosely coupled services",
  "labels": ["pattern", "distributed"]
}
```

---

#### `memoir_refine` - Update Concept

**When to use:**
- Update concept definition
- Change labels
- Refine understanding

**Parameters:**
```json
{
  "memoir": "string (required) - Memoir name",
  "concept": "string (required) - Concept name",
  "definition": "string (optional) - New definition",
  "labels": ["label1", "label2"] (optional) - New labels"
}
```

---

#### `memoir_search` - Search Within Memoir

**When to use:**
- Find concepts in specific memoir
- Discover related concepts

**Parameters:**
```json
{
  "memoir": "string (required) - Memoir name",
  "query": "string (required) - Search query",
  "limit": "integer (optional, default: 10) - Max results"
}
```

**Uses FTS5 full-text search** on concept names, definitions, and labels.

---

#### `memoir_search_all` - Search All Memoirs

**When to use:**
- Find concepts across all knowledge graphs
- Don't know which memoir contains concept

**Parameters:**
```json
{
  "query": "string (required) - Search query",
  "limit": "integer (optional, default: 10) - Max results"
}
```

---

### Link Operations

#### `memoir_link` - Create Relationship

**When to use:**
- Connect related concepts
- Build knowledge structure
- Capture semantic relationships

**Parameters:**
```json
{
  "memoir": "string (required) - Memoir name",
  "source": "string (required) - Source concept name",
  "target": "string (required) - Target concept name",
  "relation": "IsA|HasProperty|RelatedTo|Causes|PrerequisiteOf|ExampleOf|Contradicts|SimilarTo|PartOf (required)",
  "weight": "number (optional, default: 1.0) - Link strength (0.0-1.0)"
}
```

**Relation types:**
- `IsA`: Subtype relationship (Dog IsA Animal)
- `HasProperty`: Attribute (HTTP HasProperty Stateless)
- `RelatedTo`: General association
- `Causes`: Causal relationship
- `PrerequisiteOf`: Dependency (Auth PrerequisiteOf Dashboard)
- `ExampleOf`: Instance (Redis ExampleOf Cache)
- `Contradicts`: Opposing concepts
- `SimilarTo`: Analogous concepts
- `PartOf`: Component relationship (Engine PartOf Car)

**Example:**
```json
{
  "memoir": "architecture_patterns",
  "source": "Microservices",
  "target": "Service-Oriented Architecture",
  "relation": "RelatedTo",
  "weight": 0.9
}
```

---

#### `memoir_inspect` - Explore Neighborhood

**When to use:**
- Discover connected concepts
- Understand concept context
- Find related information

**Parameters:**
```json
{
  "memoir": "string (required) - Memoir name",
  "concept": "string (required) - Starting concept",
  "depth": "integer (optional, default: 1) - BFS traversal depth"
}
```

**What it does:**
- Performs BFS traversal from starting concept
- Returns concepts at each depth level
- Includes link relations and weights

**Example with depth=2:**
```
Microservices (depth 0)
  → Service-Oriented Architecture (RelatedTo, depth 1)
    → Distributed Systems (PartOf, depth 2)
  → API Gateway (HasProperty, depth 1)
```

---

## When to Use What

### Memory vs Memoir Decision Tree

```
Need to remember something?
├─ Is it a fact, decision, or observation?
│  └─ Use Memory tools (mem_store, mem_recall)
│
└─ Is it a concept with relationships?
   └─ Use Memoir tools (memoir_create, memoir_add_concept, memoir_link)
```

**Examples:**
- ✅ Memory: "Fixed bug #123 by adding null check in UserService.validate()"
- ✅ Memory: "User prefers dark mode and compact UI"
- ✅ Memoir: Concept "Authentication" linked to "JWT" via "ExampleOf"
- ✅ Memoir: "SOLID" HasProperty "Single Responsibility Principle"

---

## Best Practices

### 1. When to Store Memories

**ALWAYS store:**
- ✅ Decisions made (architecture, tech choices, trade-offs)
- ✅ Bugs fixed (what was wrong, root cause, solution)
- ✅ User preferences and constraints
- ✅ Non-obvious discoveries about the codebase
- ✅ Patterns established (naming conventions, code structure)

**DON'T store:**
- ❌ Temporary information (current task status)
- ❌ Information already in documentation
- ❌ Trivial facts that won't be recalled

### 2. Memory Format Standards

**Good memory structure:**
```
Problem: [What was the issue]
Solution: [How it was fixed]
Context: [Where/when, relevant details]
Learned: [Key takeaways, gotchas]
```

**Example:**
```json
{
  "content": "Problem: API timeout on large dataset queries\nSolution: Added pagination with cursor-based navigation\nContext: UserController.listUsers(), triggered by 10K+ user base\nLearned: SQLite performs poorly without LIMIT/OFFSET on large tables",
  "importance": "high",
  "topic": "performance_optimizations"
}
```

### 3. Search Strategies

**Progressive search:**
1. Start broad: `"authentication error"`
2. If too many results, add topic filter: `"topic": "bugs"`
3. If still too many, increase `min_score`: `"min_score": 0.3`
4. If too few, reduce specificity or remove filters

**Use natural language:**
- ✅ "how to implement OAuth flow"
- ✅ "database migration best practices"
- ❌ "oauth impl" (too terse, loses context)

### 4. Topic Organization

**Recommended topics:**
- `bugs` - Bug fixes and issues
- `architecture` - Design decisions
- `user_preferences` - User settings and preferences
- `discoveries` - Non-obvious findings
- `patterns` - Established conventions
- `security` - Security-related decisions
- `performance` - Performance optimizations

**Topic naming:**
- Use lowercase with underscores
- Be consistent (don't mix `user_prefs` and `user_preferences`)
- Keep topics broad enough to group related memories

### 5. Importance Levels

| Level | Use For | Decay Rate |
|-------|---------|------------|
| `critical` | Core architecture, security decisions, breaking changes | Slowest |
| `high` | Important discoveries, major bug fixes, user preferences | Slow |
| `medium` | Patterns, optimizations, minor decisions | Medium |
| `low` | Experiments, temporary notes, minor observations | Fast |

---

## Anti-Patterns (What NOT to Do)

### ❌ Memory Fragmentation
**Wrong:**
```json
// Storing 10 separate memories:
{"content": "Added user validation"}
{"content": "Added email validation"}
{"content": "Added password validation"}
...
```

**Right:**
```json
// One consolidated memory with topic_key for updates:
{
  "content": "Implemented user input validation: email format, password strength (8+ chars, special char), username uniqueness",
  "topic_key": "features/user_validation"
}
```

### ❌ Vague Queries
**Wrong:** `"that thing we did last week"`
**Right:** `"database schema migration for user profiles"`

### ❌ Missing Context
**Wrong:**
```json
{
  "content": "Fixed it by adding a check"
}
```

**Right:**
```json
{
  "content": "Fixed NullPointerException in UserService.authenticate() by adding null check before JWT decode. Root cause: optional email field was undefined for OAuth users.",
  "topic": "bugs"
}
```

### ❌ Ignoring Importance
**Wrong:** Everything marked `"critical"` or everything left as default
**Right:** Calibrate importance based on long-term value

### ❌ Not Using Topics
**Wrong:** All memories without topics → hard to filter/organize
**Right:** Consistent topic taxonomy for easy retrieval

---

## Session Protocols

### Session Start

1. **Check for prior context:**
   ```json
   {
     "query": "current project status",
     "topic": "session_summaries",
     "limit": 3
   }
   ```

2. **If user references past work, recall it:**
   ```json
   {
     "query": "authentication implementation details",
     "topic": "architecture"
   }
   ```

### Session End

**MANDATORY:** Store session summary before ending.

```json
{
  "content": "Session Summary:\n\nGoal: Implement OAuth2 authentication\n\nAccomplished:\n- Added OAuth2 client configuration\n- Implemented authorization code flow\n- Added JWT token validation\n\nNext Steps:\n- Add refresh token rotation\n- Implement token revocation endpoint\n\nRelevant Files:\n- auth/oauth2.rs - OAuth2 client\n- middleware/jwt.rs - JWT validation",
  "summary": "Implemented OAuth2 authentication",
  "importance": "high",
  "topic": "session_summaries",
  "topic_key": "session/2026-04-04"
}
```

---

## Recovery After Context Compaction

If your context is reset (compaction, restart, new session):

1. **Recall last session:**
   ```json
   {
     "query": "session summary",
     "topic": "session_summaries",
     "limit": 1
   }
   ```

2. **Recall project context:**
   ```json
   {
     "query": "project architecture overview",
     "topic": "architecture"
   }
   ```

3. **Continue work** using recalled information.

---

## Example Workflows

### 1. Bug Fix Workflow

```json
// 1. Recall if similar bug was fixed before
{"query": "null pointer exception user service", "topic": "bugs"}

// 2. Fix the bug (code work)

// 3. Store the fix
{
  "content": "Fixed NPE in UserService.authenticate() by adding null check before JWT decode. Root cause: OAuth users have undefined email field. Solution: Use Optional<String> for email and handle None case.",
  "importance": "high",
  "topic": "bugs",
  "topic_key": "bugs/user_service_npe"
}
```

### 2. Architecture Decision

```json
// 1. Store decision
{
  "content": "Chose PostgreSQL over MongoDB for user data. Reasons: ACID guarantees for financial transactions, strong relational queries for reporting, team expertise. Trade-off: Less flexible schema, requires migrations.",
  "summary": "Database choice: PostgreSQL",
  "importance": "critical",
  "topic": "architecture",
  "topic_key": "architecture/database_selection"
}

// 2. Add to knowledge graph
{"memoir": "tech_stack", "name": "PostgreSQL", "definition": "ACID-compliant relational database"}
{"memoir": "tech_stack", "source": "PostgreSQL", "target": "Relational Database", "relation": "IsA"}
{"memoir": "tech_stack", "source": "PostgreSQL", "target": "ACID Compliance", "relation": "HasProperty"}
```

### 3. User Preference

```json
// Store user preference (upsert pattern)
{
  "content": "User prefers TypeScript over JavaScript for all new code. Reason: type safety reduces runtime errors. Exception: Quick prototypes can use JS.",
  "importance": "high",
  "topic": "user_preferences",
  "topic_key": "preferences/typescript"
}

// Later, update the preference
{
  "content": "User now allows JavaScript for test utilities. Updated preference: TypeScript for production code, JavaScript allowed for tests and build scripts.",
  "importance": "high",
  "topic": "user_preferences",
  "topic_key": "preferences/typescript"
}
```

### 4. Knowledge Graph Building

```json
// Create memoir
{"memoir": "design_patterns", "description": "Software design patterns and relationships"}

// Add concepts
{"memoir": "design_patterns", "name": "Singleton", "definition": "Ensures class has only one instance"}
{"memoir": "design_patterns", "name": "Factory", "definition": "Creates objects without specifying exact class"}
{"memoir": "design_patterns", "name": "Creational Pattern", "definition": "Patterns dealing with object creation"}

// Link concepts
{"memoir": "design_patterns", "source": "Singleton", "target": "Creational Pattern", "relation": "IsA"}
{"memoir": "design_patterns", "source": "Factory", "target": "Creational Pattern", "relation": "IsA"}
{"memoir": "design_patterns", "source": "Singleton", "target": "Global State", "relation": "Causes"}

// Explore neighborhood
{"memoir": "design_patterns", "concept": "Singleton", "depth": 2}
```

---

## Performance Tips

1. **Batch operations:** Use `mem_embed_all` for bulk embedding instead of individual `mem_store` calls
2. **Topic filtering:** Always use `topic` parameter when you know the category
3. **Score thresholds:** Set `min_score` to reduce irrelevant results
4. **Consolidation:** Run `mem_consolidate` periodically on large topics
5. **Pagination:** Use `limit` and `offset` for large result sets

---

## Troubleshooting

### Search Returns No Results

**Possible causes:**
1. Query too specific → Try broader terms
2. Temporal decay → Old, low-importance memories may have decayed
3. Topic filter too narrow → Remove or broaden topic
4. Vector search issue → Check `mem_health`, fallback to BM25-only

**Solution:**
```json
// Remove filters and reduce specificity
{"query": "authentication", "min_score": 0.0}
```

### Memory Not Updating

**Issue:** Using `mem_store` without `topic_key` creates new memory instead of updating.

**Solution:**
```json
// Use consistent topic_key for upserts
{
  "content": "Updated content",
  "topic_key": "my_unique_key"
}
```

### Memoir Link Fails

**Issue:** Concepts don't exist yet.

**Solution:** Add concepts before linking:
```json
{"memoir": "my_memoir", "name": "Concept A"}
{"memoir": "my_memoir", "name": "Concept B"}
{"memoir": "my_memoir", "source": "Concept A", "target": "Concept B", "relation": "RelatedTo"}
```

---

## Limits and Constraints

- **Memory ID format:** ULID (26 characters, sortable by time)
- **Topic key:** Max 255 characters, use for stable upsert keys
- **Embedding dimensions:** 768 (multilingual-e5-base model)
- **Relation types:** Fixed set of 9 types (see `memoir_link`)
- **Search limit:** Default 10, max configurable in server settings
- **Decay profiles:** 4 levels (critical, high, medium, low)

---

## Integration Examples

### Claude Desktop Configuration

```json
{
  "mcpServers": {
    "alejandria": {
      "command": "alejandria-mcp",
      "args": ["--config", "/path/to/alejandria.toml"],
      "env": {
        "ALEJANDRIA_DB_PATH": "/Users/me/.alejandria/memories.db",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Standalone Usage

```bash
# Start MCP server
alejandria-mcp --config ~/.config/alejandria/config.toml

# Or with environment variables
ALEJANDRIA_DB_PATH=/data/memories.db alejandria-mcp
```

---

## Additional Resources

- **Full Documentation:** `/docs/DEPLOYMENT.md`
- **Claude Desktop Setup:** `/examples/claude-desktop/README.md`
- **API Reference:** Tool schemas in `/tools/*.json`
- **Examples:** `/examples/` directory (Python, Node, Rust clients)

---

## Version History

- **1.0.0** (2026-04-04): Initial release with 18 MCP tools, hybrid search, and knowledge graphs

---

## Summary Checklist for AI Agents

Before each session ends, ensure:
- [ ] Stored important decisions via `mem_store`
- [ ] Captured bug fixes with root cause and solution
- [ ] Updated user preferences if learned
- [ ] Created session summary with `topic: "session_summaries"`
- [ ] Organized memories into appropriate topics
- [ ] Set correct importance levels
- [ ] Used `topic_key` for evolving information

**Remember:** Alejandria is your long-term memory. Use it proactively, not just when asked to "remember." Store context as you work, and you'll build a rich knowledge base over time.

# Alejandria Architecture

## Overview

Alejandria implements a hybrid memory system combining:
- **ICM's sophisticated dual-memory architecture**: Episodic memories with temporal decay + semantic knowledge graphs
- **Engram's agent-centric workflow patterns**: Topic-based organization, deduplication, progressive disclosure
- **Modern search capabilities**: Hybrid BM25 + vector similarity

This document provides a technical deep dive into the system's architecture, data models, and key algorithms.

## System Architecture

### Crate Organization

Alejandria follows a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│         alejandria-cli (clap v4)        │  ← User-facing commands
├─────────────────────────────────────────┤
│     alejandria-mcp (JSON-RPC 2.0)       │  ← MCP server for AI agents
├─────────────────────────────────────────┤
│  alejandria-storage (SQLite + rusqlite) │  ← Concrete implementation
├─────────────────────────────────────────┤
│   alejandria-core (traits + types)      │  ← Pure abstractions
└─────────────────────────────────────────┘
```

#### alejandria-core

**Purpose**: Pure Rust abstractions with zero I/O

**Exports**:
- Types: `Memory`, `Memoir`, `Concept`, `ConceptLink`
- Traits: `MemoryStore`, `MemoirStore`, `Embedder`
- Errors: `IcmError`, `IcmResult`
- Enums: `Importance`, `RelationType`, `MemorySource`

**Dependencies**: Only `chrono`, `serde`, `ulid`, `thiserror`

#### alejandria-storage

**Purpose**: SQLite-based storage implementation

**Features**:
- Full implementation of `MemoryStore` and `MemoirStore` traits
- FTS5 virtual tables for BM25 ranking
- sqlite-vec extension for cosine similarity
- Schema migrations and validation
- Efficient indexing (B-tree on timestamps, hash on topic_key)

**Dependencies**: `rusqlite`, `serde_json`, `fastembed` (optional)

#### alejandria-mcp

**Purpose**: Model Context Protocol server over stdio

**Features**:
- JSON-RPC 2.0 protocol handler
- 15 tools (9 memory + 6 memoir operations)
- Stdio transport (line-delimited JSON)
- Proper error codes (-32602, -32001, etc.)

**Dependencies**: `serde_json`, `anyhow`

#### alejandria-cli

**Purpose**: Command-line interface for human users

**Features**:
- clap v4 with derive API
- Config file loading (TOML + env overrides)
- JSON output mode for scripting
- Rich help text with examples

**Dependencies**: `clap`, `toml`, `serde`, `anyhow`, `chrono`

## Data Models

### Episodic Memory (Memory)

```rust
pub struct Memory {
    // Identity
    pub id: String,                    // ULID
    pub topic: String,                 // High-level category
    pub topic_key: Option<String>,     // Semantic handle for upsert
    
    // Content
    pub summary: String,               // Main content
    pub raw_excerpt: Option<String>,   // Full original text
    pub keywords: Vec<String>,         // Extracted terms
    pub embedding: Option<Vec<f32>>,   // 768d vector
    
    // Lifecycle
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    
    // Decay
    pub weight: f32,                   // 0.0 - 1.0
    pub access_count: u32,
    pub importance: Importance,        // Critical | High | Medium | Low
    
    // Deduplication
    pub revision_count: u32,
    pub duplicate_count: u32,
    
    // Metadata
    pub source: MemorySource,          // User | Agent | System | External
    pub related_ids: Vec<String>,      // Cross-references
}
```

**Key Design Decisions**:

1. **ULID over UUID**: Lexicographically sortable, timestamp-embedded
2. **topic_key for deduplication**: Semantic handles like "rust/error-handling/result-type" enable intelligent upsert
3. **Separate timestamps**: `last_accessed` (read ops) vs `last_seen_at` (duplicate detection)
4. **Optional embedding**: Large payload (~3KB per memory), lazy-loaded
5. **Soft delete**: `deleted_at` field instead of hard deletion

### Semantic Memory (Memoir)

```rust
pub struct Memoir {
    pub id: String,                    // ULID
    pub name: String,                  // Unique name
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,   // Extensible
}

pub struct Concept {
    pub id: String,                    // ULID
    pub memoir_id: String,             // Parent memoir
    pub name: String,                  // Unique within memoir
    pub definition: String,            // Main content
    pub labels: Vec<String>,           // Classification tags
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

pub struct ConceptLink {
    pub id: String,                    // ULID
    pub memoir_id: String,
    pub source_id: String,             // Source concept
    pub target_id: String,             // Target concept
    pub relation: RelationType,        // Typed edge
    pub weight: f32,                   // 0.0 - 1.0
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}
```

**Relation Types** (9 types):
- `IsA`: Taxonomy/inheritance (e.g., "Rust is_a Programming Language")
- `HasProperty`: Attributes (e.g., "JWT has_property Stateless")
- `RelatedTo`: Generic association
- `Causes`: Causal relationship
- `PrerequisiteOf`: Dependencies
- `ExampleOf`: Instantiation
- `Contradicts`: Conflicts
- `SimilarTo`: Similarity
- `PartOf`: Composition

## Database Schema

### SQLite Schema

```sql
-- Episodic memories
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    weight REAL NOT NULL DEFAULT 1.0,
    topic TEXT NOT NULL,
    summary TEXT NOT NULL,
    raw_excerpt TEXT,
    keywords TEXT,  -- JSON array
    embedding BLOB,  -- F32 vector
    importance TEXT NOT NULL DEFAULT 'medium',
    source TEXT NOT NULL DEFAULT 'user',
    related_ids TEXT,  -- JSON array
    topic_key TEXT UNIQUE,
    revision_count INTEGER NOT NULL DEFAULT 1,
    duplicate_count INTEGER NOT NULL DEFAULT 0,
    last_seen_at TEXT NOT NULL,
    deleted_at TEXT
);

-- Indexes
CREATE INDEX idx_memories_topic ON memories(topic);
CREATE INDEX idx_memories_created_at ON memories(created_at);
CREATE INDEX idx_memories_importance ON memories(importance);
CREATE INDEX idx_memories_weight ON memories(weight);
CREATE INDEX idx_memories_topic_key ON memories(topic_key);
CREATE INDEX idx_memories_deleted_at ON memories(deleted_at);

-- FTS5 virtual table for keyword search
CREATE VIRTUAL TABLE memories_fts USING fts5(
    id UNINDEXED,
    topic,
    summary,
    content='memories',
    content_rowid='rowid'
);

-- Vector search (sqlite-vec)
CREATE VIRTUAL TABLE vec_memories USING vec0(
    embedding FLOAT[768]
);

-- Knowledge graphs
CREATE TABLE memoirs (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata TEXT  -- JSON
);

CREATE TABLE concepts (
    id TEXT PRIMARY KEY,
    memoir_id TEXT NOT NULL,
    name TEXT NOT NULL,
    definition TEXT NOT NULL,
    labels TEXT,  -- JSON array
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata TEXT,  -- JSON
    UNIQUE(memoir_id, name),
    FOREIGN KEY(memoir_id) REFERENCES memoirs(id) ON DELETE CASCADE
);

CREATE INDEX idx_concepts_memoir_id ON concepts(memoir_id);

CREATE VIRTUAL TABLE concepts_fts USING fts5(
    id UNINDEXED,
    memoir_id UNINDEXED,
    name,
    definition,
    content='concepts',
    content_rowid='rowid'
);

CREATE TABLE concept_links (
    id TEXT PRIMARY KEY,
    memoir_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    created_at TEXT NOT NULL,
    metadata TEXT,  -- JSON
    FOREIGN KEY(memoir_id) REFERENCES memoirs(id) ON DELETE CASCADE,
    FOREIGN KEY(source_id) REFERENCES concepts(id) ON DELETE CASCADE,
    FOREIGN KEY(target_id) REFERENCES concepts(id) ON DELETE CASCADE
);

CREATE INDEX idx_concept_links_source ON concept_links(source_id);
CREATE INDEX idx_concept_links_target ON concept_links(target_id);
CREATE INDEX idx_concept_links_memoir ON concept_links(memoir_id);

-- System metadata
CREATE TABLE system_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### Migration System

Versioned migrations in `migrations.rs`:
- `V1`: Initial schema creation
- `V2`: Add FTS5 triggers
- `V3`: Add vector search support
- Future: `V4`, `V5`, etc.

Migrations are idempotent and can be rolled back for testing.

## Search Pipeline

### Hybrid Search Algorithm

```
┌──────────────────────────────────────────────┐
│         User Query: "authentication bug"     │
└──────────────────┬───────────────────────────┘
                   │
         ┌─────────┴─────────┐
         │                   │
         ▼                   ▼
┌─────────────────┐   ┌──────────────────┐
│ Keyword Search  │   │  Vector Search   │
│  (FTS5 BM25)    │   │ (Cosine Sim.)    │
└────────┬────────┘   └────────┬─────────┘
         │                     │
         ▼                     ▼
┌─────────────────┐   ┌──────────────────┐
│ Top 50 results  │   │  Top 50 results  │
│ Raw BM25 scores │   │ Cosine distances │
└────────┬────────┘   └────────┬─────────┘
         │                     │
         └─────────┬───────────┘
                   ▼
         ┌─────────────────────┐
         │ Normalize scores    │
         │ BM25: max -> 1.0    │
         │ Cosine: 1-dist      │
         └──────────┬──────────┘
                    ▼
         ┌─────────────────────┐
         │  Weighted merge     │
         │  0.3×BM25 + 0.7×cos │
         └──────────┬──────────┘
                    ▼
         ┌─────────────────────┐
         │ Deduplicate by ID   │
         │ Sort by final score │
         │ Take top N          │
         └──────────┬──────────┘
                    ▼
         ┌─────────────────────┐
         │  Update access      │
         │  - access_count++   │
         │  - last_accessed    │
         │  - trigger decay    │
         └─────────────────────┘
```

### Implementation Details

**1. Keyword Search (BM25)**

Uses SQLite FTS5 with default parameters:
- k1 = 1.2 (term frequency saturation)
- b = 0.75 (length normalization)

Query:
```sql
SELECT id, rank
FROM memories_fts
WHERE memories_fts MATCH ?
ORDER BY rank
LIMIT 50
```

**2. Vector Search (Cosine Similarity)**

Uses sqlite-vec with L2-normalized embeddings:
```sql
SELECT rowid, distance
FROM vec_memories
WHERE embedding MATCH ?
  AND k = 50
ORDER BY distance
```

**3. Score Normalization**

```rust
// BM25: Normalize by max score
let max_bm25 = bm25_results.iter()
    .map(|r| r.score)
    .fold(f32::NEG_INFINITY, f32::max);
let normalized_bm25 = score / max_bm25;

// Cosine: Convert distance to similarity
let cosine_sim = 1.0 - distance;  // distance in [0, 2]
```

**4. Weighted Merge**

```rust
const BM25_WEIGHT: f32 = 0.3;
const COSINE_WEIGHT: f32 = 0.7;

let final_score = BM25_WEIGHT * normalized_bm25 
                + COSINE_WEIGHT * cosine_sim;
```

### Fallback Strategy

If embeddings are unavailable:
1. Attempt hybrid search
2. On error, fall back to FTS5-only search
3. Log warning for operator awareness

## Temporal Decay System

### Decay Formula

```
effective_rate = base_rate × importance_multiplier / (1 + access_count × damping_factor)
new_weight = old_weight × (1 - effective_rate)
```

Where:
- `base_rate`: Configured decay rate (default: 0.01 = 1% per day)
- `importance_multiplier`: 
  - Critical: 0.0 (never decays)
  - High: 0.5
  - Medium: 1.0
  - Low: 2.0
- `damping_factor`: 0.1 (10% reduction per access)

### Example Decay Rates

| Importance | Access Count | Effective Rate | Days to 50% | Days to 10% |
|------------|--------------|----------------|-------------|-------------|
| Critical   | any          | 0.00%          | ∞           | ∞           |
| High       | 0            | 0.50%          | 138 days    | 459 days    |
| High       | 10           | 0.25%          | 277 days    | 919 days    |
| Medium     | 0            | 1.00%          | 69 days     | 229 days    |
| Medium     | 5            | 0.67%          | 103 days    | 343 days    |
| Low        | 0            | 2.00%          | 34 days     | 114 days    |
| Low        | 3            | 1.54%          | 45 days     | 149 days    |

### Pruning Thresholds

Memories below weight threshold are soft-deleted:
- **Critical**: Never pruned (regardless of weight)
- **High**: Never pruned (regardless of weight)
- **Medium**: Pruned when weight < 0.1
- **Low**: Pruned when weight < 0.3

### Decay Scheduling

Decay is triggered automatically during hybrid_search if:
```rust
let last_decay = get_last_decay_timestamp()?;
let hours_since = (Utc::now() - last_decay).num_hours();
if hours_since >= config.auto_decay_hours {
    apply_decay(config.base_rate)?;
}
```

Default: Every 24 hours during first search operation

## Deduplication System

### Topic Key Mechanism

```rust
pub struct Memory {
    pub topic_key: Option<String>,  // e.g., "rust/error-handling/result-type"
    pub revision_count: u32,
    pub duplicate_count: u32,
    pub last_seen_at: DateTime<Utc>,
    // ...
}
```

### Upsert Logic

```rust
fn store(&self, memory: Memory) -> IcmResult<String> {
    if let Some(topic_key) = &memory.topic_key {
        // Check for existing memory with same topic_key
        if let Some(existing) = self.get_by_topic_key(topic_key)? {
            // Update existing memory
            let updated = Memory {
                id: existing.id,
                summary: memory.summary,  // New content
                revision_count: existing.revision_count + 1,
                last_seen_at: Utc::now(),
                updated_at: Utc::now(),
                weight: 1.0,  // Reset decay on update
                ..existing
            };
            self.update(updated)?;
            return Ok(existing.id);
        }
    }
    
    // No existing memory, create new
    let new_memory = Memory {
        id: ulid::Ulid::new().to_string(),
        ..memory
    };
    // INSERT INTO memories ...
    Ok(new_memory.id)
}
```

### Benefits

1. **Prevents duplicates**: Same logical memory stored once
2. **Tracks evolution**: `revision_count` shows update frequency
3. **Freshness signal**: Recent updates boost relevance
4. **Decay reset**: Updates reset weight to 1.0

## Consolidation

### Algorithm

```rust
fn consolidate_topic(&self, topic: &str, min_memories: usize, min_weight: f32) 
    -> IcmResult<String> 
{
    // 1. Fetch all memories in topic above weight threshold
    let memories = self.get_by_topic(topic, None, None)?
        .into_iter()
        .filter(|m| m.weight >= min_weight)
        .collect::<Vec<_>>();
    
    if memories.len() < min_memories {
        return Err(IcmError::InvalidInput("Insufficient memories".into()));
    }
    
    // 2. Extract all unique keywords
    let all_keywords: HashSet<String> = memories
        .iter()
        .flat_map(|m| &m.keywords)
        .cloned()
        .collect();
    
    // 3. Generate consolidated summary
    let summary = format!(
        "Consolidated {} memories from topic '{}': {}",
        memories.len(),
        topic,
        all_keywords.iter().take(10).join(", ")
    );
    
    // 4. Create consolidated memory
    let consolidated = Memory {
        id: ulid::Ulid::new().to_string(),
        topic: topic.to_string(),
        summary,
        importance: Importance::High,  // Consolidations are important
        source: MemorySource::System,
        related_ids: memories.iter().map(|m| m.id.clone()).collect(),
        keywords: all_keywords.into_iter().collect(),
        ..Default::default()
    };
    
    self.store(consolidated)
}
```

## Performance Characteristics

### Complexity Analysis

| Operation | Time Complexity | Notes |
|-----------|-----------------|-------|
| store() | O(log n) | B-tree insert + FTS5 update |
| get() | O(1) | Primary key lookup |
| search_by_keywords() | O(k + m log m) | FTS5 scan + sort (k=matches, m=limit) |
| search_by_embedding() | O(n + m log m) | Vector scan + sort |
| hybrid_search() | O(n + k log k) | Both searches in parallel |
| apply_decay() | O(n) | Full table scan, batch update |
| list_topics() | O(n) | Aggregate scan with GROUP BY |

### Benchmark Results (10k memories)

```
hybrid_search/10k:     42 ms
hybrid_search/50k:    178 ms
hybrid_search/100k:   312 ms
apply_decay/10k:       1.2 s
embedding_single:      28 ms
embedding_batch_10:     5 ms/item
```

### Optimization Strategies

1. **Lazy embedding**: Embeddings generated on-demand, not at store()
2. **Batch embedding**: CLI `embed` command processes in batches
3. **Index tuning**: B-tree on (topic, created_at), hash on topic_key
4. **FTS5 optimization**: Separate virtual table with triggers
5. **Connection pooling**: Arc<Mutex<Connection>> for thread safety
6. **Prepared statements**: Cached via rusqlite::CachedStatement

## Configuration

### Config File (TOML)

```toml
[database]
path = "~/.local/share/alejandria/memories.db"

[embeddings]
enabled = true
model = "intfloat/multilingual-e5-base"
dimensions = 768
batch_size = 32

[decay]
base_rate = 0.01         # 1% daily
min_weight = 0.1         # Pruning threshold
auto_decay_hours = 24    # Auto-decay interval

[search]
bm25_weight = 0.3        # Keyword weight
cosine_weight = 0.7      # Vector weight
default_limit = 5        # Default result count
```

### Environment Variables

Priority: Env vars > Config file > Defaults

```bash
ALEJANDRIA_DB_PATH="./custom.db"
ALEJANDRIA_EMBEDDINGS_ENABLED="false"
ALEJANDRIA_DECAY_BASE_RATE="0.02"
ALEJANDRIA_SEARCH_DEFAULT_LIMIT="10"
```

## Extension Points

### Custom Embedder

```rust
use alejandria_core::{Embedder, IcmResult};

pub struct OpenAIEmbedder {
    client: OpenAIClient,
    model: String,
}

impl Embedder for OpenAIEmbedder {
    fn embed(&self, text: &str) -> IcmResult<Vec<f32>> {
        // Call OpenAI API...
    }
    
    fn embed_batch(&self, texts: &[&str]) -> IcmResult<Vec<Vec<f32>>> {
        // Batch embedding...
    }
    
    fn dimensions(&self) -> usize { 1536 }
    fn model_name(&self) -> &str { &self.model }
}
```

### Custom Storage Backend

```rust
use alejandria_core::{MemoryStore, Memory, IcmResult};

pub struct PostgresStore {
    pool: PgPool,
}

impl MemoryStore for PostgresStore {
    fn store(&self, memory: Memory) -> IcmResult<String> {
        // Postgres implementation...
    }
    
    fn get(&self, id: &str) -> IcmResult<Option<Memory>> {
        // ...
    }
    
    // ... implement all trait methods
}
```

## Future Enhancements

### Planned Features

1. **Async runtime**: Tokio support for concurrent operations
2. **Distributed storage**: Multi-node PostgreSQL backend
3. **Advanced embeddings**: Support for OpenAI, Cohere, Anthropic APIs
4. **Graph algorithms**: PageRank, community detection on memoirs
5. **Compression**: LZ4 for raw_excerpt and embeddings
6. **Replication**: Git-based sync between databases
7. **Web UI**: Browser-based visualization and management

### Scalability Targets

- **1M memories**: SQLite performance remains acceptable
- **10M memories**: Recommend PostgreSQL migration
- **100M+ memories**: Distributed architecture with sharding

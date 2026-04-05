//! Database schema and migrations for Alejandria.
//!
//! This module defines the complete SQLite schema including:
//! - Core tables (memories, memoirs, concepts, concept_links)
//! - FTS5 virtual tables for full-text search
//! - sqlite-vec virtual table for vector search
//! - Triggers for auto-sync FTS indexes
//! - Performance indexes

use alejandria_core::error::{IcmError, IcmResult};
use rusqlite::Connection;

/// Current schema version
pub const SCHEMA_VERSION: u32 = 2;

/// Embedding dimensions for multilingual-e5-base model
pub const EMBEDDING_DIMS: u32 = 768;

/// Initialize the database schema.
///
/// Creates all tables, indexes, triggers, and virtual tables if they don't exist.
/// This is idempotent and safe to call multiple times.
///
/// # Arguments
///
/// * `conn` - SQLite database connection
///
/// # Returns
///
/// Returns Ok(()) if successful, or IcmError on failure.
///
/// # Examples
///
/// ```ignore
/// use rusqlite::Connection;
/// use alejandria_storage::schema::init_db;
///
/// let conn = Connection::open_in_memory().unwrap();
/// init_db(&conn).unwrap();
/// ```
pub fn init_db(conn: &Connection) -> IcmResult<()> {
    // Enable foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| IcmError::Database(e.to_string()))?;

    // Enable WAL mode for better concurrency
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|e| IcmError::Database(e.to_string()))?;

    // Create all tables
    create_memories_table(conn)?;
    create_memories_fts_table(conn)?;
    create_vec_memories_table(conn)?;
    create_memoirs_table(conn)?;
    create_concepts_table(conn)?;
    create_concepts_fts_table(conn)?;
    create_concept_links_table(conn)?;
    create_metadata_table(conn)?;

    // Create indexes
    create_indexes(conn)?;

    // Initialize metadata
    init_metadata(conn)?;

    Ok(())
}

/// Create the memories table (episodic storage).
///
/// Stores all memory entries with full metadata, timestamps, and content fields.
///
/// ## Timestamp Precision
///
/// All timestamp columns (created_at, updated_at, last_accessed, last_seen_at, deleted_at)
/// store milliseconds since Unix epoch (1970-01-01 00:00:00 UTC) as INTEGER.
/// Use `.timestamp_millis()` when storing and `from_timestamp_millis()` when reading.
fn create_memories_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY NOT NULL,
            created_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            updated_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            last_accessed INTEGER NOT NULL,     -- Timestamp in milliseconds since Unix epoch
            access_count INTEGER NOT NULL DEFAULT 0,
            weight REAL NOT NULL DEFAULT 1.0,
            
            topic TEXT NOT NULL,
            summary TEXT NOT NULL,
            raw_excerpt TEXT,
            keywords TEXT NOT NULL,
            
            embedding BLOB,
            importance TEXT NOT NULL CHECK(importance IN ('critical', 'high', 'medium', 'low')),
            source TEXT NOT NULL,
            related_ids TEXT NOT NULL DEFAULT '[]',
            
            topic_key TEXT,
            revision_count INTEGER NOT NULL DEFAULT 1,
            duplicate_count INTEGER NOT NULL DEFAULT 0,
            last_seen_at INTEGER NOT NULL,      -- Timestamp in milliseconds since Unix epoch
            deleted_at INTEGER,                 -- Timestamp in milliseconds since Unix epoch (NULL if not deleted)
            
            decay_profile TEXT,                 -- Decay strategy name (NULL = default exponential)
            decay_params TEXT                   -- JSON parameters for decay strategy (NULL = use defaults)
        ) STRICT;
        "#,
    ).map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Create the FTS5 virtual table for memories.
///
/// Enables full-text search with BM25 ranking on topic, summary, and keywords.
fn create_memories_fts_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
            id UNINDEXED,
            topic,
            summary,
            keywords,
            content='memories',
            content_rowid='rowid'
        );
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    // Create triggers for auto-sync
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS memories_fts_insert AFTER INSERT ON memories BEGIN
            INSERT INTO memories_fts(rowid, id, topic, summary, keywords)
            VALUES (new.rowid, new.id, new.topic, new.summary, new.keywords);
        END;
        
        CREATE TRIGGER IF NOT EXISTS memories_fts_update AFTER UPDATE ON memories BEGIN
            INSERT OR REPLACE INTO memories_fts(rowid, id, topic, summary, keywords)
            VALUES (new.rowid, new.id, new.topic, new.summary, new.keywords);
        END;
        
        CREATE TRIGGER IF NOT EXISTS memories_fts_delete AFTER DELETE ON memories BEGIN
            DELETE FROM memories_fts WHERE rowid = old.rowid;
        END;
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Create the vec_memories virtual table for vector search.
///
/// Uses sqlite-vec extension for cosine similarity search on embeddings.
/// Note: This requires sqlite-vec extension to be available.
fn create_vec_memories_table(conn: &Connection) -> IcmResult<()> {
    // Try to create the vec table, but don't fail if sqlite-vec is not available
    let result = conn.execute_batch(&format!(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS vec_memories USING vec0(
            memory_id TEXT PRIMARY KEY,
            embedding float[{}]
        );
        "#,
        EMBEDDING_DIMS
    ));

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            // Log warning but don't fail - embeddings are optional
            eprintln!(
                "Warning: Could not create vec_memories table (sqlite-vec not available): {}",
                e
            );
            Ok(())
        }
    }
}

/// Create the memoirs table (knowledge graph containers).
///
/// ## Timestamp Precision
///
/// All timestamp columns store milliseconds since Unix epoch as INTEGER.
fn create_memoirs_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS memoirs (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            created_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            updated_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            metadata TEXT NOT NULL DEFAULT '{}'
        ) STRICT;
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Create the concepts table (knowledge graph nodes).
///
/// ## Timestamp Precision
///
/// All timestamp columns store milliseconds since Unix epoch as INTEGER.
fn create_concepts_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS concepts (
            id TEXT PRIMARY KEY NOT NULL,
            memoir_id TEXT NOT NULL,
            name TEXT NOT NULL,
            definition TEXT NOT NULL,
            labels TEXT NOT NULL DEFAULT '[]',
            created_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            updated_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            metadata TEXT NOT NULL DEFAULT '{}',
            
            FOREIGN KEY (memoir_id) REFERENCES memoirs(id) ON DELETE CASCADE,
            UNIQUE(memoir_id, name)
        ) STRICT;
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Create the FTS5 virtual table for concepts.
fn create_concepts_fts_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS concepts_fts USING fts5(
            id UNINDEXED,
            memoir_id UNINDEXED,
            name,
            definition,
            labels,
            content='concepts',
            content_rowid='rowid'
        );
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    // Create triggers for auto-sync
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS concepts_fts_insert AFTER INSERT ON concepts BEGIN
            INSERT INTO concepts_fts(rowid, id, memoir_id, name, definition, labels)
            VALUES (new.rowid, new.id, new.memoir_id, new.name, new.definition, new.labels);
        END;
        
        CREATE TRIGGER IF NOT EXISTS concepts_fts_update AFTER UPDATE ON concepts BEGIN
            INSERT OR REPLACE INTO concepts_fts(rowid, id, memoir_id, name, definition, labels)
            VALUES (new.rowid, new.id, new.memoir_id, new.name, new.definition, new.labels);
        END;
        
        CREATE TRIGGER IF NOT EXISTS concepts_fts_delete AFTER DELETE ON concepts BEGIN
            DELETE FROM concepts_fts WHERE rowid = old.rowid;
        END;
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Create the concept_links table (knowledge graph edges).
///
/// ## Timestamp Precision
///
/// All timestamp columns store milliseconds since Unix epoch as INTEGER.
fn create_concept_links_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS concept_links (
            id TEXT PRIMARY KEY NOT NULL,
            memoir_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            relation TEXT NOT NULL CHECK(relation IN (
                'is_a', 'has_property', 'related_to', 'causes',
                'prerequisite_of', 'example_of', 'contradicts',
                'similar_to', 'part_of'
            )),
            weight REAL NOT NULL DEFAULT 1.0,
            created_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            metadata TEXT NOT NULL DEFAULT '{}',
            
            FOREIGN KEY (memoir_id) REFERENCES memoirs(id) ON DELETE CASCADE,
            FOREIGN KEY (source_id) REFERENCES concepts(id) ON DELETE CASCADE,
            FOREIGN KEY (target_id) REFERENCES concepts(id) ON DELETE CASCADE,
            CHECK(source_id != target_id),
            UNIQUE(memoir_id, source_id, target_id, relation)
        ) STRICT;
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Create the icm_metadata table for system state.
///
/// ## Timestamp Precision
///
/// All timestamp columns store milliseconds since Unix epoch as INTEGER.
fn create_metadata_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS icm_metadata (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL         -- Timestamp in milliseconds since Unix epoch
        ) STRICT;
        "#,
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Create all performance indexes.
fn create_indexes(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        -- Memories indexes
        CREATE INDEX IF NOT EXISTS idx_memories_topic ON memories(topic);
        CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance);
        CREATE INDEX IF NOT EXISTS idx_memories_topic_key ON memories(topic_key) WHERE topic_key IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memories_last_accessed ON memories(last_accessed DESC);
        CREATE INDEX IF NOT EXISTS idx_memories_deleted_at ON memories(deleted_at) WHERE deleted_at IS NULL;
        
        -- Memoirs indexes
        CREATE INDEX IF NOT EXISTS idx_memoirs_name ON memoirs(name);
        
        -- Concepts indexes
        CREATE INDEX IF NOT EXISTS idx_concepts_memoir_id ON concepts(memoir_id);
        CREATE INDEX IF NOT EXISTS idx_concepts_name ON concepts(name);
        
        -- Concept links indexes
        CREATE INDEX IF NOT EXISTS idx_concept_links_source_id ON concept_links(source_id);
        CREATE INDEX IF NOT EXISTS idx_concept_links_target_id ON concept_links(target_id);
        CREATE INDEX IF NOT EXISTS idx_concept_links_relation ON concept_links(relation);
        CREATE INDEX IF NOT EXISTS idx_concept_links_memoir_id ON concept_links(memoir_id);
        "#,
    ).map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Initialize metadata table with default values.
fn init_metadata(conn: &Connection) -> IcmResult<()> {
    // Timestamp in milliseconds since Unix epoch
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT OR IGNORE INTO icm_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
        rusqlite::params!["schema_version", SCHEMA_VERSION.to_string(), now],
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    conn.execute(
        "INSERT OR IGNORE INTO icm_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
        rusqlite::params!["embedding_dims", EMBEDDING_DIMS.to_string(), now],
    )
    .map_err(|e| IcmError::Database(e.to_string()))?;

    Ok(())
}

/// Verify schema integrity.
///
/// Checks that all required tables exist and schema version matches.
pub fn verify_schema(conn: &Connection) -> IcmResult<bool> {
    let required_tables = vec![
        "memories",
        "memories_fts",
        "memoirs",
        "concepts",
        "concepts_fts",
        "concept_links",
        "icm_metadata",
    ];

    for table in required_tables {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .map_err(|e| IcmError::Database(e.to_string()))?;

        if count == 0 {
            return Ok(false);
        }
    }

    // Check schema version
    let version: String = conn
        .query_row(
            "SELECT value FROM icm_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| IcmError::Database(e.to_string()))?;

    let version_num: u32 = version
        .parse()
        .map_err(|e| IcmError::Database(format!("Invalid schema version: {}", e)))?;

    Ok(version_num == SCHEMA_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_init_db_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Check all tables exist
        let tables = vec![
            "memories",
            "memories_fts",
            "memoirs",
            "concepts",
            "concepts_fts",
            "concept_links",
            "icm_metadata",
        ];

        for table in tables {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(count, 1, "Table {} should exist", table);
        }
    }

    #[test]
    fn test_memories_table_structure() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Insert a test memory
        conn.execute(
            r#"
            INSERT INTO memories (
                id, created_at, updated_at, last_accessed, access_count, weight,
                topic, summary, keywords, importance, source, related_ids, last_seen_at
            ) VALUES (
                'test123', 1000, 1000, 1000, 0, 1.0,
                'test', 'test summary', '["test"]', 'medium', 'user', '[]', 1000
            )
            "#,
            [],
        )
        .unwrap();

        // Verify it was inserted
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_fts_triggers_sync_correctly() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Check database integrity before test
        // Insert a memory
        conn.execute(
            r#"
            INSERT INTO memories (
                id, created_at, updated_at, last_accessed, access_count, weight,
                topic, summary, keywords, importance, source, related_ids, last_seen_at
            ) VALUES (
                'test123', 1000, 1000, 1000, 0, 1.0,
                'authentication', 'JWT implementation', '["jwt","auth"]', 'high', 'user', '[]', 1000
            )
            "#,
            [],
        )
        .unwrap();

        // Check FTS table was updated via trigger
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories_fts WHERE id = 'test123'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Update the memory
        conn.execute(
            "UPDATE memories SET summary = 'Updated JWT implementation' WHERE id = 'test123'",
            [],
        )
        .unwrap();

        // Verify FTS was updated
        let summary: String = conn
            .query_row(
                "SELECT summary FROM memories_fts WHERE id = 'test123'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(summary, "Updated JWT implementation");

        // Delete the memory
        conn.execute("DELETE FROM memories WHERE id = 'test123'", [])
            .unwrap();

        // Verify FTS entry was deleted
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories_fts WHERE id = 'test123'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_memoirs_unique_name_constraint() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Insert first memoir
        conn.execute(
            "INSERT INTO memoirs (id, name, description, created_at, updated_at) VALUES ('id1', 'test', 'desc', 1000, 1000)",
            [],
        )
        .unwrap();

        // Try to insert duplicate name - should fail
        let result = conn.execute(
            "INSERT INTO memoirs (id, name, description, created_at, updated_at) VALUES ('id2', 'test', 'desc', 1000, 1000)",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_concepts_unique_within_memoir() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Create memoir
        conn.execute(
            "INSERT INTO memoirs (id, name, description, created_at, updated_at) VALUES ('memoir1', 'test', 'desc', 1000, 1000)",
            [],
        )
        .unwrap();

        // Insert first concept
        conn.execute(
            "INSERT INTO concepts (id, memoir_id, name, definition, created_at, updated_at) VALUES ('c1', 'memoir1', 'concept1', 'def', 1000, 1000)",
            [],
        )
        .unwrap();

        // Try to insert duplicate name in same memoir - should fail
        let result = conn.execute(
            "INSERT INTO concepts (id, memoir_id, name, definition, created_at, updated_at) VALUES ('c2', 'memoir1', 'concept1', 'def', 1000, 1000)",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_concept_links_no_self_loops() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Create memoir and concept
        conn.execute(
            "INSERT INTO memoirs (id, name, description, created_at, updated_at) VALUES ('m1', 'test', 'desc', 1000, 1000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO concepts (id, memoir_id, name, definition, created_at, updated_at) VALUES ('c1', 'm1', 'concept1', 'def', 1000, 1000)",
            [],
        )
        .unwrap();

        // Try to create self-loop - should fail
        let result = conn.execute(
            "INSERT INTO concept_links (id, memoir_id, source_id, target_id, relation, created_at) VALUES ('l1', 'm1', 'c1', 'c1', 'is_a', 1000)",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_cascade_delete_memoir() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Create memoir with concepts and links
        conn.execute(
            "INSERT INTO memoirs (id, name, description, created_at, updated_at) VALUES ('m1', 'test', 'desc', 1000, 1000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO concepts (id, memoir_id, name, definition, created_at, updated_at) VALUES ('c1', 'm1', 'concept1', 'def', 1000, 1000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO concepts (id, memoir_id, name, definition, created_at, updated_at) VALUES ('c2', 'm1', 'concept2', 'def', 1000, 1000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO concept_links (id, memoir_id, source_id, target_id, relation, created_at) VALUES ('l1', 'm1', 'c1', 'c2', 'is_a', 1000)",
            [],
        )
        .unwrap();

        // Delete memoir
        conn.execute("DELETE FROM memoirs WHERE id = 'm1'", [])
            .unwrap();

        // Verify concepts were deleted
        let concept_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM concepts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(concept_count, 0);

        // Verify links were deleted
        let link_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM concept_links", [], |row| row.get(0))
            .unwrap();
        assert_eq!(link_count, 0);
    }

    #[test]
    fn test_metadata_initialization() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Check schema version
        let version: String = conn
            .query_row(
                "SELECT value FROM icm_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION.to_string());

        // Check embedding dims
        let dims: String = conn
            .query_row(
                "SELECT value FROM icm_metadata WHERE key = 'embedding_dims'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dims, EMBEDDING_DIMS.to_string());
    }

    #[test]
    fn test_verify_schema() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let is_valid = verify_schema(&conn).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_indexes_created() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Check some key indexes exist
        let index_names = vec![
            "idx_memories_topic",
            "idx_memories_topic_key",
            "idx_concepts_memoir_id",
            "idx_concept_links_source_id",
        ];

        for index_name in index_names {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    [index_name],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(count, 1, "Index {} should exist", index_name);
        }
    }
}

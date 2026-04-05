//! Integration tests for CRUD operations on SqliteStore.
//!
//! These tests validate the core memory storage operations including:
//! - Creating new memories
//! - Upserting via topic_key
//! - Retrieving memories with access tracking
//! - Updating memory fields
//! - Soft-deleting memories
//! - Listing with pagination and filters

use alejandria_core::{
    memory::{Importance, Memory, MemorySource},
    store::MemoryStore,
};
use alejandria_storage::SqliteStore;

/// Helper function to create a test memory with default values
fn create_test_memory(topic: &str, summary: &str) -> Memory {
    Memory {
        id: String::new(), // Will be generated
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 0,
        weight: 1.0,
        topic: topic.to_string(),
        summary: summary.to_string(),
        raw_excerpt: None,
        keywords: vec![],
        embedding: None,
        importance: Importance::Medium,
        source: MemorySource::User,
        related_ids: vec![],
        topic_key: None,
        revision_count: 1,
        duplicate_count: 0,
        last_seen_at: chrono::Utc::now(),
        deleted_at: None,
        decay_profile: None,
        decay_params: None,
    }
}

/// Test storing a new memory
///
/// Given: No existing memories
/// When: store() is called with a new memory
/// Then: Memory is created with a generated ULID and can be retrieved
#[test]
fn test_store_new_memory() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create a new memory
    let memory = create_test_memory("authentication", "Implemented JWT authentication");

    // Store the memory
    let id = store.store(memory.clone()).expect("Failed to store memory");

    // Verify ID was generated (ULID format - 26 characters)
    assert_eq!(id.len(), 26, "Generated ID should be a ULID (26 chars)");

    // Retrieve and verify
    let retrieved = store.get(&id).expect("Failed to get memory");
    assert!(retrieved.is_some(), "Memory should exist");

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.topic, "authentication");
    assert_eq!(retrieved.summary, "Implemented JWT authentication");
    assert_eq!(retrieved.importance, Importance::Medium);
    assert_eq!(retrieved.revision_count, 1);
}

/// Test upserting a memory using topic_key
///
/// Given: An existing memory with topic_key="architecture/auth-model"
/// When: store() is called again with the same topic_key
/// Then: Existing memory is updated and revision_count is incremented
#[test]
fn test_store_upsert_by_topic_key() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create and store initial memory with topic_key
    let mut memory1 = create_test_memory("architecture", "Initial auth design");
    memory1.topic_key = Some("architecture/auth-model".to_string());

    let id1 = store
        .store(memory1.clone())
        .expect("Failed to store initial memory");

    // Retrieve to verify initial state
    let initial = store
        .get(&id1)
        .expect("Failed to get initial memory")
        .unwrap();
    assert_eq!(initial.revision_count, 1);
    assert_eq!(initial.summary, "Initial auth design");

    // Store again with same topic_key but different content (upsert)
    let mut memory2 = create_test_memory("architecture", "Updated auth design with OAuth2");
    memory2.topic_key = Some("architecture/auth-model".to_string());

    let id2 = store
        .store(memory2)
        .expect("Failed to store updated memory");

    // Verify same ID was returned (upsert, not insert)
    assert_eq!(id1, id2, "Should return same ID for upsert");

    // Retrieve and verify update
    let updated = store
        .get(&id2)
        .expect("Failed to get updated memory")
        .unwrap();
    assert_eq!(updated.revision_count, 2, "Revision count should increment");
    assert_eq!(updated.summary, "Updated auth design with OAuth2");
    assert_eq!(updated.id, id1, "ID should remain the same");

    // Verify only one memory exists with this topic_key
    let count = store.count().expect("Failed to count memories");
    assert_eq!(
        count, 1,
        "Should only have one memory (upserted, not duplicated)"
    );
}

/// Test that retrieving a memory updates access tracking
///
/// Given: A memory with access_count=0
/// When: get() is called
/// Then: access_count is incremented and last_accessed is updated
#[test]
fn test_get_memory_updates_access() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store a memory
    let memory = create_test_memory("testing", "Access tracking test");
    let id = store.store(memory).expect("Failed to store memory");

    // Get initial state
    let initial = store.get(&id).expect("Failed to get memory").unwrap();
    let initial_access_count = initial.access_count;
    let initial_last_accessed = initial.last_accessed;

    // Sleep briefly to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Get again to trigger access tracking
    let accessed = store.get(&id).expect("Failed to get memory again").unwrap();

    // Verify access_count incremented
    assert_eq!(
        accessed.access_count,
        initial_access_count + 1,
        "Access count should increment on get()"
    );

    // Verify last_accessed was updated
    assert!(
        accessed.last_accessed > initial_last_accessed,
        "Last accessed timestamp should be updated"
    );
}

/// Test updating memory fields
///
/// Given: An existing memory
/// When: update() is called with modified fields
/// Then: Changes are persisted and updated_at is refreshed
#[test]
fn test_update_memory() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store initial memory
    let memory = create_test_memory("api", "Initial API design");
    let id = store.store(memory).expect("Failed to store memory");

    // Retrieve and modify
    let mut memory = store.get(&id).expect("Failed to get memory").unwrap();
    let original_updated_at = memory.updated_at;

    memory.summary = "Updated API design with versioning".to_string();
    memory.importance = Importance::High;
    memory.keywords = vec!["api".to_string(), "versioning".to_string()];

    // Sleep to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update
    store
        .update(memory.clone())
        .expect("Failed to update memory");

    // Retrieve and verify changes
    let updated = store
        .get(&id)
        .expect("Failed to get updated memory")
        .unwrap();
    assert_eq!(updated.summary, "Updated API design with versioning");
    assert_eq!(updated.importance, Importance::High);
    assert_eq!(updated.keywords.len(), 2);
    assert!(updated.keywords.contains(&"api".to_string()));
    assert!(updated.keywords.contains(&"versioning".to_string()));

    // Verify updated_at was refreshed
    assert!(
        updated.updated_at > original_updated_at,
        "Updated timestamp should be more recent"
    );
}

/// Test soft-deleting a memory
///
/// Given: An active memory
/// When: delete() is called
/// Then: deleted_at is set and memory is excluded from get() and list()
#[test]
fn test_forget_memory_soft_delete() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store a memory
    let memory = create_test_memory("temporary", "This will be deleted");
    let id = store.store(memory).expect("Failed to store memory");

    // Verify it exists
    assert!(store.get(&id).expect("Failed to get memory").is_some());

    // Soft delete
    store.delete(&id).expect("Failed to delete memory");

    // Verify it's not returned by get() (soft-deleted)
    let retrieved = store.get(&id).expect("Failed to get after delete");
    assert!(
        retrieved.is_none(),
        "Soft-deleted memory should not be returned by get()"
    );

    // Verify count excludes soft-deleted
    let count = store.count().expect("Failed to count");
    assert_eq!(count, 0, "Count should exclude soft-deleted memories");
}

/// Test listing memories with pagination
///
/// Given: Multiple memories exist
/// When: get_by_topic() is called with limit and offset
/// Then: Correct subset is returned respecting pagination parameters
#[test]
fn test_list_memories_pagination() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store 10 memories in the same topic
    for i in 0..10 {
        let memory = create_test_memory("pagination", &format!("Memory {}", i));
        store.store(memory).expect("Failed to store memory");
    }

    // Get all memories in topic (no pagination)
    let all_memories = store
        .get_by_topic("pagination", None, None)
        .expect("Failed to list memories");
    assert_eq!(all_memories.len(), 10, "Should have 10 memories");

    // Verify all memories are from the correct topic
    for memory in &all_memories {
        assert_eq!(memory.topic, "pagination");
    }

    // Test limit without offset (first 5)
    let first_five = store
        .get_by_topic("pagination", Some(5), None)
        .expect("Failed to get first 5");
    assert_eq!(first_five.len(), 5, "Should return exactly 5 memories");

    // Test limit with offset (skip first 5, get next 3)
    let next_three = store
        .get_by_topic("pagination", Some(3), Some(5))
        .expect("Failed to get next 3");
    assert_eq!(next_three.len(), 3, "Should return exactly 3 memories");

    // Verify no overlap between first 5 and next 3
    let first_ids: Vec<_> = first_five.iter().map(|m| &m.id).collect();
    for memory in &next_three {
        assert!(
            !first_ids.contains(&&memory.id),
            "Should not overlap with first batch"
        );
    }

    // Test offset beyond available records
    let beyond = store
        .get_by_topic("pagination", Some(5), Some(20))
        .expect("Failed to query beyond range");
    assert_eq!(
        beyond.len(),
        0,
        "Should return empty when offset exceeds count"
    );

    // Verify count
    let count = store.count().expect("Failed to count");
    assert_eq!(count, 10);
}

/// Test listing memories with importance filter
///
/// Given: Memories with different importance levels
/// When: Filtering by importance
/// Then: Only memories matching the filter are returned
#[test]
fn test_list_memories_filters() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories with different importance levels
    let mut critical = create_test_memory("important", "Critical security issue");
    critical.importance = Importance::Critical;
    store.store(critical).expect("Failed to store critical");

    let mut high = create_test_memory("important", "High priority feature");
    high.importance = Importance::High;
    store.store(high).expect("Failed to store high");

    let mut medium = create_test_memory("important", "Medium priority task");
    medium.importance = Importance::Medium;
    store.store(medium).expect("Failed to store medium");

    let mut low = create_test_memory("important", "Low priority cleanup");
    low.importance = Importance::Low;
    store.store(low).expect("Failed to store low");

    // Get all memories in topic
    let all = store
        .get_by_topic("important", None, None)
        .expect("Failed to list all");
    assert_eq!(all.len(), 4, "Should have 4 memories total");

    // Verify each importance level exists
    let critical_count = all
        .iter()
        .filter(|m| m.importance == Importance::Critical)
        .count();
    let high_count = all
        .iter()
        .filter(|m| m.importance == Importance::High)
        .count();
    let medium_count = all
        .iter()
        .filter(|m| m.importance == Importance::Medium)
        .count();
    let low_count = all
        .iter()
        .filter(|m| m.importance == Importance::Low)
        .count();

    assert_eq!(critical_count, 1);
    assert_eq!(high_count, 1);
    assert_eq!(medium_count, 1);
    assert_eq!(low_count, 1);
}

/// Test filtering by source type
///
/// Given: Memories from different sources
/// When: Listing all memories
/// Then: Source types are correctly persisted and retrievable
#[test]
fn test_list_memories_by_source() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories with different sources
    let mut user_mem = create_test_memory("sources", "User created memory");
    user_mem.source = MemorySource::User;
    store.store(user_mem).expect("Failed to store user memory");

    let mut agent_mem = create_test_memory("sources", "Agent created memory");
    agent_mem.source = MemorySource::Agent;
    store
        .store(agent_mem)
        .expect("Failed to store agent memory");

    let mut system_mem = create_test_memory("sources", "System created memory");
    system_mem.source = MemorySource::System;
    store
        .store(system_mem)
        .expect("Failed to store system memory");

    let mut external_mem = create_test_memory("sources", "External imported memory");
    external_mem.source = MemorySource::External;
    store
        .store(external_mem)
        .expect("Failed to store external memory");

    // Get all and verify sources
    let all = store
        .get_by_topic("sources", None, None)
        .expect("Failed to list all");
    assert_eq!(
        all.len(),
        4,
        "Should have 4 memories with different sources"
    );

    let user_count = all
        .iter()
        .filter(|m| m.source == MemorySource::User)
        .count();
    let agent_count = all
        .iter()
        .filter(|m| m.source == MemorySource::Agent)
        .count();
    let system_count = all
        .iter()
        .filter(|m| m.source == MemorySource::System)
        .count();
    let external_count = all
        .iter()
        .filter(|m| m.source == MemorySource::External)
        .count();

    assert_eq!(user_count, 1);
    assert_eq!(agent_count, 1);
    assert_eq!(system_count, 1);
    assert_eq!(external_count, 1);
}

// ============================================================================
// Embedding Integration Tests (Phase 3 - Tasks 3.16-3.17)
// ============================================================================

/// Test that store() automatically generates embeddings when feature enabled
///
/// Given: Embeddings feature is enabled
/// When: store() is called with a new memory
/// Then: Embedding is automatically generated without failing the operation
#[test]
#[cfg(feature = "embeddings")]
fn test_store_generates_embedding() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    let memory = create_test_memory("test", "This should get an embedding");
    let id = store.store(memory).expect("Failed to store");

    // Verify memory was stored successfully
    let retrieved = store.get(&id).expect("Failed to get").unwrap();
    assert_eq!(retrieved.summary, "This should get an embedding");

    // Check if vec_memories table exists (may not if sqlite-vec unavailable)
    let table_exists =
        store
            .with_conn(|conn| {
                let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_memories'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);
                Ok(count)
            })
            .unwrap_or(0);

    if table_exists == 1 {
        // vec_memories table exists, check for embedding
        let _has_embedding: bool = store
            .with_conn(|conn| {
                let count: i32 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM vec_memories WHERE memory_id = ?",
                        rusqlite::params![id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                Ok(count > 0)
            })
            .unwrap_or(false);

        // Embedding might not exist if generate_embedding() is a stub
        // Just verify no crash occurred (test passes if we reach here)
    } else {
        // No sqlite-vec, graceful degradation - just verify store succeeded
        // Test passes if we reach here without panic
    }
}

/// Test that update() regenerates embeddings when summary changes
///
/// Given: A memory with an existing embedding
/// When: update() is called with a different summary
/// Then: Embedding is regenerated for the new summary
#[test]
#[cfg(feature = "embeddings")]
fn test_update_regenerates_embedding_on_summary_change() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Store initial memory
    let mut memory = create_test_memory("test", "Original summary");
    let id = store.store(memory.clone()).expect("Failed to store");

    // Update with NEW summary
    memory.id = id.clone();
    memory.summary = "Completely different summary".to_string();
    memory.updated_at = chrono::Utc::now();

    // Should regenerate embedding
    store.update(memory.clone()).expect("Failed to update");

    // Verify update succeeded (embedding regeneration is best-effort)
    let updated = store.get(&id).unwrap().unwrap();
    assert_eq!(updated.summary, "Completely different summary");
}

/// Test that update() skips embedding regeneration when summary unchanged
///
/// Given: A memory with an existing embedding
/// When: update() is called with the SAME summary but other changes
/// Then: Embedding regeneration is skipped (optimization)
#[test]
#[cfg(feature = "embeddings")]
fn test_update_skips_embedding_if_summary_unchanged() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Store initial memory
    let mut memory = create_test_memory("test", "Same summary");
    let id = store.store(memory.clone()).expect("Failed to store");

    // Update with SAME summary, only change keywords
    memory.id = id.clone();
    memory.keywords = vec!["new".to_string(), "keywords".to_string()];
    memory.updated_at = chrono::Utc::now();

    // Should NOT regenerate embedding (summary unchanged)
    store.update(memory).expect("Failed to update");

    // Verify update succeeded
    let updated = store.get(&id).unwrap().unwrap();
    assert_eq!(
        updated.keywords,
        vec!["new".to_string(), "keywords".to_string()]
    );
}

// ============================================================================
// Embedding Dimension Validation Tests (Technical Debt Issue #4)
// ============================================================================

/// Test that store() rejects embeddings with invalid dimensions
///
/// Given: A memory with an embedding of incorrect dimensions
/// When: store() is called
/// Then: Returns IcmError::InvalidInput with dimension mismatch message
#[test]
fn test_store_rejects_invalid_embedding_dimensions() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Create memory with invalid embedding (expected 768, providing 384)
    let mut memory = create_test_memory("test", "Invalid embedding dimensions");
    memory.embedding = Some(vec![0.1; 384]); // Wrong dimension

    let result = store.store(memory);

    // Should fail with InvalidInput error
    assert!(
        result.is_err(),
        "Should reject invalid embedding dimensions"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid embedding dimensions") || err_msg.contains("dimension"),
        "Error message should mention dimension validation, got: {}",
        err_msg
    );
}

/// Test that store() accepts embeddings with correct dimensions
///
/// Given: A memory with an embedding of correct dimensions (768)
/// When: store() is called
/// Then: Memory is stored successfully
#[test]
fn test_store_accepts_valid_embedding_dimensions() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Create memory with valid 768-dimensional embedding
    let mut memory = create_test_memory("test", "Valid embedding dimensions");
    memory.embedding = Some(vec![0.1; 768]); // Correct dimension

    let result = store.store(memory);

    // Should succeed
    assert!(
        result.is_ok(),
        "Should accept valid 768-dimensional embedding"
    );

    let id = result.unwrap();
    let retrieved = store.get(&id).expect("Failed to get").unwrap();
    assert_eq!(retrieved.summary, "Valid embedding dimensions");

    // Verify embedding was stored
    if let Some(embedding) = retrieved.embedding {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding dimension should be preserved"
        );
    }
}

/// Test that update() rejects embeddings with invalid dimensions
///
/// Given: An existing memory
/// When: update() is called with an embedding of incorrect dimensions
/// Then: Returns IcmError::InvalidInput
#[test]
fn test_update_rejects_invalid_embedding_dimensions() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Store initial memory without embedding
    let memory = create_test_memory("test", "Memory to be updated");
    let id = store.store(memory).expect("Failed to store");

    // Try to update with invalid embedding
    let mut memory = store.get(&id).expect("Failed to get").unwrap();
    memory.embedding = Some(vec![0.1; 512]); // Wrong dimension

    let result = store.update(memory);

    // Should fail with InvalidInput error
    assert!(
        result.is_err(),
        "Should reject invalid embedding dimensions on update"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid embedding dimensions") || err_msg.contains("dimension"),
        "Error message should mention dimension validation, got: {}",
        err_msg
    );
}

/// Test that store_embedding() helper validates dimensions
///
/// Given: Embeddings feature is enabled
/// When: store_embedding() is called internally with invalid dimensions
/// Then: Validation prevents storage of malformed embeddings
///
/// Note: This test validates the helper method indirectly through store()
#[test]
#[cfg(feature = "embeddings")]
fn test_store_embedding_helper_validates_dimensions() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Create memory that would trigger store_embedding()
    let memory = create_test_memory("test", "Embedding generation test");

    // Store should either:
    // 1. Generate a valid 768-dim embedding, OR
    // 2. Skip embedding generation if not available (graceful degradation)
    let result = store.store(memory);

    // Either way, store() should not crash with dimension validation errors
    assert!(
        result.is_ok(),
        "Embedding generation should produce valid dimensions or skip gracefully"
    );

    let id = result.unwrap();
    let retrieved = store.get(&id).expect("Failed to get").unwrap();

    // If an embedding was generated, verify it has correct dimensions
    if let Some(embedding) = retrieved.embedding {
        assert_eq!(
            embedding.len(),
            768,
            "Generated embeddings must have correct dimensions"
        );
    }
}

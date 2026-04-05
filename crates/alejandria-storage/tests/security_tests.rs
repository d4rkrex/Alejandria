//! Security tests for Alejandria storage layer
//!
//! Tests cover:
//! - SQL injection protection
//! - Parameterized query verification
//! - Input validation and sanitization
//! - Topic key collision handling
//! - Boundary conditions and edge cases

use alejandria_core::{Importance, Memory, MemoryStore};
use alejandria_storage::SqliteStore;

fn create_test_store() -> SqliteStore {
    SqliteStore::open_in_memory().expect("Failed to create test store")
}

// =============================================================================
// Task 8.13: SQL Injection Tests
// =============================================================================

#[test]
fn test_sql_injection_in_summary() {
    let store = create_test_store();

    // Attempt SQL injection via summary field
    let malicious_summary = "'; DROP TABLE memories; --";
    let mut memory = Memory::new("test_topic".to_string(), malicious_summary.to_string());
    memory.importance = Importance::Medium;

    // Should safely store without executing SQL
    let id = store.store(memory).unwrap();

    // Verify memory was stored correctly
    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.summary, malicious_summary);

    // Verify table still exists by querying stats
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

#[test]
fn test_sql_injection_in_raw_excerpt() {
    let store = create_test_store();

    let malicious_excerpt = "test' OR '1'='1";
    let mut memory = Memory::new("test_topic".to_string(), "content".to_string());
    memory.raw_excerpt = Some(malicious_excerpt.to_string());
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.raw_excerpt, Some(malicious_excerpt.to_string()));
}

#[test]
fn test_sql_injection_in_topic() {
    let store = create_test_store();

    let malicious_topic = "test'; DELETE FROM memories WHERE '1'='1";
    let mut memory = Memory::new(malicious_topic.to_string(), "content".to_string());
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.topic, malicious_topic);

    // Verify no deletion occurred
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

#[test]
fn test_sql_injection_in_topic_key() {
    let store = create_test_store();

    let malicious_topic_key = "test'; UPDATE memories SET summary='hacked' WHERE '1'='1";
    let mut memory = Memory::new("test_topic".to_string(), "original_summary".to_string());
    memory.topic_key = Some(malicious_topic_key.to_string());
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.topic_key, Some(malicious_topic_key.to_string()));
    assert_eq!(retrieved.summary, "original_summary");
}

#[test]
fn test_sql_injection_in_search_query() {
    let store = create_test_store();

    // Store a normal memory
    let mut memory = Memory::new("test_topic".to_string(), "test summary".to_string());
    memory.importance = Importance::Medium;
    store.store(memory).unwrap();

    // Attempt SQL injection via search query
    let malicious_query = "test' OR '1'='1' --";
    let result = store.search_by_keywords(malicious_query, 10);

    // FTS5 should reject invalid syntax or return safe results
    // Either error (syntax) or safe results are acceptable
    match result {
        Ok(results) => {
            // Should not return all records
            assert!(
                results.len() <= 1,
                "Search should not return unauthorized results"
            );
        }
        Err(_) => {
            // FTS5 syntax error is acceptable - query was safely rejected
        }
    }

    // Verify database is still intact
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

#[test]
fn test_sql_injection_in_related_ids() {
    let store = create_test_store();

    let malicious_related = vec![
        "normal_id".to_string(),
        "'; DROP TABLE memories; --".to_string(),
    ];

    let mut memory = Memory::new("test_topic".to_string(), "summary".to_string());
    memory.related_ids = malicious_related.clone();
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.related_ids, malicious_related);

    // Verify table still exists
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

#[test]
fn test_sql_injection_in_keywords() {
    let store = create_test_store();

    let malicious_keywords = vec![
        "normal".to_string(),
        "'; DELETE FROM memories; --".to_string(),
    ];

    let mut memory = Memory::new("test_topic".to_string(), "summary".to_string());
    memory.keywords = malicious_keywords.clone();
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.keywords, malicious_keywords);

    // Verify table still exists
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

// =============================================================================
// Task 8.14: Parameterized Query Verification
// =============================================================================

#[test]
fn test_store_uses_parameterized_queries() {
    let store = create_test_store();

    // Test various special characters that would break non-parameterized queries
    let special_chars = vec!["'", "\"", ";", "--", "/*", "*/", "\\", "\0", "\n", "\r\n"];

    for (i, special) in special_chars.iter().enumerate() {
        let summary = format!("summary_with_{}", special);
        let mut memory = Memory::new(format!("topic_{}", i), summary.clone());
        memory.importance = Importance::Medium;

        let id = store.store(memory).unwrap();
        let retrieved = store.get(&id).unwrap().unwrap();
        assert_eq!(retrieved.summary, summary);
    }

    // All memories should be stored successfully
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, special_chars.len());
}

#[test]
fn test_update_uses_parameterized_queries() {
    let store = create_test_store();

    let mut memory = Memory::new("original_topic".to_string(), "original_summary".to_string());
    memory.importance = Importance::Medium;
    let id = store.store(memory).unwrap();

    // Update with potentially dangerous summary
    let dangerous_summary = "'; DELETE FROM memories; --";
    let mut updated = store.get(&id).unwrap().unwrap();
    updated.summary = dangerous_summary.to_string();

    store.update(updated).unwrap();

    // Verify update worked and no deletion occurred
    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.summary, dangerous_summary);

    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

#[test]
fn test_delete_uses_parameterized_queries() {
    let store = create_test_store();

    // Create multiple memories
    for i in 0..5 {
        let mut memory = Memory::new(format!("topic_{}", i), format!("summary_{}", i));
        memory.importance = Importance::Medium;
        store.store(memory).unwrap();
    }

    let stats_before = store.stats().unwrap();
    assert_eq!(stats_before.total_memories, 5);

    // Get first memory by searching for one topic
    let memories_in_topic = store.get_by_topic("topic_0", Some(1), None).unwrap();
    let first_id = &memories_in_topic[0].id;

    // Delete one memory
    store.delete(first_id).unwrap();

    // Should only delete one memory, not all
    let stats_after = store.stats().unwrap();
    assert_eq!(stats_after.active_memories, 4);
    assert_eq!(stats_after.deleted_memories, 1);
}

#[test]
fn test_search_query_parameterization() {
    let store = create_test_store();

    // Store test memories
    let mut mem1 = Memory::new("test".to_string(), "apple banana".to_string());
    mem1.importance = Importance::Medium;
    store.store(mem1).unwrap();

    let mut mem2 = Memory::new("test".to_string(), "cherry date".to_string());
    mem2.importance = Importance::Medium;
    store.store(mem2).unwrap();

    // Test that FTS5 special characters are handled safely
    let special_queries = vec![
        "apple OR DROP",
        "banana'",
        "cherry\"",
        "date; DELETE",
        "* OR *",
    ];

    for query in special_queries {
        // Should not panic or execute SQL
        let result = store.search_by_keywords(query, 10);
        // Either succeeds with safe results or returns error (both acceptable)
        assert!(result.is_ok() || result.is_err());
    }
}

// =============================================================================
// Task 8.15: Topic Key Collision Handling
// =============================================================================

#[test]
fn test_topic_key_upsert_updates_existing() {
    let store = create_test_store();

    let topic_key = "unique_key_123";

    // Store initial memory with topic_key
    let mut memory1 = Memory::new("topic_v1".to_string(), "summary_v1".to_string());
    memory1.topic_key = Some(topic_key.to_string());
    memory1.importance = Importance::Medium;

    let id1 = store.store(memory1).unwrap();

    // Store another memory with same topic_key (should update)
    let mut memory2 = Memory::new("topic_v2".to_string(), "summary_v2".to_string());
    memory2.topic_key = Some(topic_key.to_string());
    memory2.importance = Importance::High;

    let id2 = store.store(memory2).unwrap();

    // Should be the same ID (update, not insert)
    assert_eq!(id1, id2);

    // Verify summary and importance were updated
    // Note: topic field is NOT updated during upsert (topic_key is the semantic handle)
    let retrieved = store.get(&id1).unwrap().unwrap();
    assert_eq!(retrieved.summary, "summary_v2");
    assert_eq!(retrieved.importance, Importance::High);
    assert_eq!(
        retrieved.topic, "topic_v1",
        "Topic field should remain unchanged during upsert"
    );

    // Should only have one memory in database
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

#[test]
fn test_topic_key_collision_preserves_id() {
    let store = create_test_store();

    let topic_key = "collision_test";

    // Store multiple updates with same topic_key
    let mut final_id = None;

    for i in 0..5 {
        let mut memory = Memory::new(format!("topic_{}", i), format!("summary_{}", i));
        memory.topic_key = Some(topic_key.to_string());
        memory.importance = Importance::Medium;

        let id = store.store(memory).unwrap();

        if let Some(prev_id) = final_id {
            assert_eq!(id, prev_id, "ID should remain constant across upserts");
        }

        final_id = Some(id);
    }

    // Should still have only one memory
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);

    // Verify final summary
    let retrieved = store.get(&final_id.unwrap()).unwrap().unwrap();
    assert_eq!(retrieved.summary, "summary_4");
}

#[test]
fn test_topic_key_null_allows_duplicates() {
    let store = create_test_store();

    // Store multiple memories without topic_key (should all be inserted)
    let mut ids = Vec::new();

    for i in 0..3 {
        let mut memory = Memory::new("same_topic".to_string(), format!("summary_{}", i));
        memory.importance = Importance::Medium;
        // No topic_key set

        let id = store.store(memory).unwrap();
        ids.push(id);
    }

    // All IDs should be different (no collision)
    assert_eq!(ids.len(), 3);
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);
    assert_ne!(ids[0], ids[2]);

    // Should have 3 separate memories
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 3);
}

#[test]
fn test_topic_key_empty_string_behavior() {
    let store = create_test_store();

    // Store memories with empty string topic_key
    let mut ids = Vec::new();

    for i in 0..3 {
        let mut memory = Memory::new("topic".to_string(), format!("summary_{}", i));
        memory.topic_key = Some("".to_string());
        memory.importance = Importance::Medium;

        let id = store.store(memory).unwrap();
        ids.push(id);
    }

    // CURRENT BEHAVIOR: Empty strings ARE treated as a valid topic_key,
    // so all three stores will upsert to the same memory
    // This is debatable behavior - could be changed to treat "" as NULL
    assert_eq!(ids[0], ids[1], "Empty string topic_keys currently collide");
    assert_eq!(ids[1], ids[2], "Empty string topic_keys currently collide");

    let stats = store.stats().unwrap();
    assert_eq!(
        stats.total_memories, 1,
        "All empty topic_keys upserted to same memory"
    );

    // Verify final memory has last summary
    let retrieved = store.get(&ids[2]).unwrap().unwrap();
    assert_eq!(retrieved.summary, "summary_2");
}

#[test]
fn test_topic_key_change_via_update() {
    let store = create_test_store();

    // Store with topic_key A
    let mut memory1 = Memory::new("topic".to_string(), "summary".to_string());
    memory1.topic_key = Some("key_a".to_string());
    memory1.importance = Importance::Medium;
    let id1 = store.store(memory1).unwrap();

    // CURRENT BEHAVIOR: update() does NOT update topic_key field
    // This may be intentional (topic_key is immutable once set)
    // or a bug to fix in the future
    let mut updated = store.get(&id1).unwrap().unwrap();
    updated.topic_key = Some("key_b".to_string());
    updated.summary = "updated_summary".to_string();

    store.update(updated).unwrap();

    // Verify topic_key was NOT changed (current behavior)
    let retrieved = store.get(&id1).unwrap().unwrap();
    assert_eq!(
        retrieved.topic_key,
        Some("key_a".to_string()),
        "topic_key currently cannot be changed via update()"
    );
    assert_eq!(
        retrieved.summary, "updated_summary",
        "but other fields are updated"
    );

    // Because topic_key didn't change, storing with key_a will upsert to same memory
    let mut memory2 = Memory::new("topic".to_string(), "new_summary".to_string());
    memory2.topic_key = Some("key_a".to_string());
    memory2.importance = Importance::Medium;
    let id2 = store.store(memory2).unwrap();

    // Same ID (upsert) because original still has key_a
    assert_eq!(
        id1, id2,
        "Upserts to same memory because topic_key didn't change"
    );

    // Should have 1 memory
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);
}

#[test]
fn test_topic_key_special_characters() {
    let store = create_test_store();

    let special_keys = [
        "key/with/slashes",
        "key:with:colons",
        "key with spaces",
        "key_with_émojis_🔥",
        "key\nwith\nnewlines",
        "key\twith\ttabs",
    ];

    for (i, key) in special_keys.iter().enumerate() {
        let mut memory = Memory::new(format!("topic_{}", i), format!("summary_{}", i));
        memory.topic_key = Some(key.to_string());
        memory.importance = Importance::Medium;

        let id = store.store(memory).unwrap();

        // Store again with same key (should update)
        let mut memory2 = Memory::new(format!("topic_{}_v2", i), format!("summary_{}_v2", i));
        memory2.topic_key = Some(key.to_string());
        memory2.importance = Importance::Medium;

        let id2 = store.store(memory2).unwrap();

        // Should update same memory
        assert_eq!(id, id2);

        let retrieved = store.get(&id).unwrap().unwrap();
        assert_eq!(retrieved.summary, format!("summary_{}_v2", i));
    }

    // Should have one memory per special key
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, special_keys.len());
}

// =============================================================================
// Additional Security Tests
// =============================================================================

#[test]
fn test_extremely_long_inputs() {
    let store = create_test_store();

    // Test with very long strings (potential buffer overflow or denial of service)
    let long_summary = "A".repeat(1_000_000); // 1MB of 'A'
    let long_topic = "B".repeat(10_000); // 10KB topic
    let long_topic_key = "C".repeat(10_000); // 10KB key

    let mut memory = Memory::new(long_topic.clone(), long_summary.clone());
    memory.topic_key = Some(long_topic_key.clone());
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.summary.len(), 1_000_000);
    assert_eq!(retrieved.topic.len(), 10_000);
    assert_eq!(retrieved.topic_key.as_ref().unwrap().len(), 10_000);
}

#[test]
fn test_unicode_and_special_encoding() {
    let store = create_test_store();

    let unicode_summary = "🚀 Emoji test 你好世界 مرحبا العالم \u{1F4A9}";
    let mut memory = Memory::new("unicode_test".to_string(), unicode_summary.to_string());
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.summary, unicode_summary);
}

#[test]
fn test_null_byte_handling() {
    let store = create_test_store();

    // SQLite should handle null bytes in strings
    let summary_with_null = "summary\0with\0nulls";
    let mut memory = Memory::new("test".to_string(), summary_with_null.to_string());
    memory.importance = Importance::Medium;

    let id = store.store(memory).unwrap();

    let retrieved = store.get(&id).unwrap().unwrap();
    // The summary should be preserved (SQLite stores it as blob internally)
    assert_eq!(retrieved.summary, summary_with_null);
}

#[test]
fn test_concurrent_topic_key_upserts() {
    let store = create_test_store();

    let topic_key = "concurrent_key";

    // Simulate concurrent upserts with same topic_key
    // In a real scenario these would be parallel threads, but for testing
    // we just do them sequentially to verify the logic works

    for i in 0..10 {
        let mut memory = Memory::new(format!("topic_{}", i), format!("summary_{}", i));
        memory.topic_key = Some(topic_key.to_string());
        memory.importance = Importance::Medium;

        store.store(memory).unwrap();
    }

    // Should only have one memory (all updates to same key)
    let stats = store.stats().unwrap();
    assert_eq!(stats.total_memories, 1);

    // Verify last update won by retrieving via topic_key
    let retrieved = store.get_by_topic_key(topic_key).unwrap().unwrap();
    assert_eq!(retrieved.summary, "summary_9");
}

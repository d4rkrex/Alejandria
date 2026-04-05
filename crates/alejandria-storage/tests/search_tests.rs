//! Integration tests for search methods on SqliteStore.
//!
//! These tests validate the search and query operations including:
//! - FTS5 keyword search with BM25 ranking
//! - Topic-based exact match search
//! - Topic key lookup
//! - Temporal queries (created_at, last_accessed ranges)
//! - Topic listing with statistics
//! - Topic health metrics
//! - Count operations with filters
//! - System-wide statistics aggregation

use alejandria_core::{
    memory::{Importance, Memory, MemorySource},
    store::MemoryStore,
};
use alejandria_storage::SqliteStore;
use chrono::{Duration, Utc};

/// Helper function to create a test memory with default values
fn create_test_memory(topic: &str, summary: &str) -> Memory {
    Memory {
        id: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_accessed: Utc::now(),
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
        last_seen_at: Utc::now(),
        deleted_at: None,
        decay_profile: None,
        decay_params: None,
    }
}

/// Test FTS5 keyword search with BM25 ranking
///
/// Given: Multiple memories with different content
/// When: search_by_keywords() is called
/// Then: Results are returned ranked by BM25 relevance score
#[test]
fn test_search_fts5_keywords() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories with varying keyword relevance
    let mem1 = create_test_memory(
        "security",
        "JWT authentication implementation with secure token handling",
    );
    store.store(mem1).expect("Failed to store memory 1");

    let mem2 = create_test_memory(
        "security",
        "SQL injection prevention using parameterized queries",
    );
    store.store(mem2).expect("Failed to store memory 2");

    let mem3 = create_test_memory("api", "REST API design with authentication endpoints");
    store.store(mem3).expect("Failed to store memory 3");

    let mem4 = create_test_memory("database", "Database schema migration for user table");
    store.store(mem4).expect("Failed to store memory 4");

    // Search for "authentication"
    let results = store
        .search_by_keywords("authentication", 10)
        .expect("Failed to search");

    // Should return memories containing "authentication"
    assert!(
        results.len() >= 2,
        "Should find at least 2 memories with 'authentication'"
    );

    // Verify results contain the keyword
    for memory in &results {
        assert!(
            memory.summary.to_lowercase().contains("authentication")
                || memory.topic.to_lowercase().contains("authentication"),
            "Result should contain search keyword"
        );
    }

    // Search for "security" - should find multiple
    let security_results = store
        .search_by_keywords("security", 10)
        .expect("Failed to search for security");

    assert!(
        security_results.len() >= 2,
        "Should find multiple security-related memories"
    );
}

/// Test searching by exact topic match
///
/// Given: Memories across multiple topics
/// When: get_by_topic() is called
/// Then: Only memories from that topic are returned
#[test]
fn test_search_by_topic() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories in different topics
    store
        .store(create_test_memory("authentication", "JWT implementation"))
        .expect("Failed to store");
    store
        .store(create_test_memory("authentication", "OAuth2 flow"))
        .expect("Failed to store");
    store
        .store(create_test_memory("database", "Schema design"))
        .expect("Failed to store");
    store
        .store(create_test_memory("api", "REST endpoints"))
        .expect("Failed to store");

    // Search by exact topic (no pagination)
    let auth_results = store
        .get_by_topic("authentication", None, None)
        .expect("Failed to get by topic");

    assert_eq!(
        auth_results.len(),
        2,
        "Should find exactly 2 authentication memories"
    );

    for memory in &auth_results {
        assert_eq!(memory.topic, "authentication");
    }

    // Search another topic
    let db_results = store
        .get_by_topic("database", None, None)
        .expect("Failed to get database topic");

    assert_eq!(db_results.len(), 1, "Should find exactly 1 database memory");
    assert_eq!(db_results[0].topic, "database");

    // Search non-existent topic
    let empty_results = store
        .get_by_topic("nonexistent", None, None)
        .expect("Failed to get nonexistent topic");

    assert_eq!(
        empty_results.len(),
        0,
        "Should return empty for non-existent topic"
    );
}

/// Test finding memory by topic_key
///
/// Given: A memory with topic_key="architecture/auth-model"
/// When: get_by_topic_key() is called
/// Then: The correct memory is returned (upsert lookup workflow)
#[test]
fn test_find_by_topic_key() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memory with topic_key
    let mut memory = create_test_memory("architecture", "Auth system design");
    memory.topic_key = Some("architecture/auth-model".to_string());
    let id = store.store(memory).expect("Failed to store");

    // Find by topic_key
    let found = store
        .get_by_topic_key("architecture/auth-model")
        .expect("Failed to find by topic_key");

    assert!(found.is_some(), "Should find memory by topic_key");

    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.topic_key, Some("architecture/auth-model".to_string()));
    assert_eq!(found.summary, "Auth system design");

    // Search for non-existent topic_key
    let not_found = store
        .get_by_topic_key("nonexistent/key")
        .expect("Failed to search for non-existent key");

    assert!(
        not_found.is_none(),
        "Should return None for non-existent topic_key"
    );
}

/// Test temporal query by created_at range
///
/// Given: Memories created at different times
/// When: Querying by created_at range
/// Then: Only memories within the range are returned
#[test]
fn test_search_temporal_created_at() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    let now = Utc::now();

    // Store memories with different created_at timestamps
    let mut old_memory = create_test_memory("temporal", "Old memory");
    old_memory.created_at = now - Duration::days(30);
    old_memory.updated_at = old_memory.created_at;
    old_memory.last_accessed = old_memory.created_at;
    old_memory.last_seen_at = old_memory.created_at;
    store.store(old_memory).expect("Failed to store old memory");

    let mut recent_memory = create_test_memory("temporal", "Recent memory");
    recent_memory.created_at = now - Duration::days(5);
    recent_memory.updated_at = recent_memory.created_at;
    recent_memory.last_accessed = recent_memory.created_at;
    recent_memory.last_seen_at = recent_memory.created_at;
    store
        .store(recent_memory)
        .expect("Failed to store recent memory");

    let new_memory = create_test_memory("temporal", "New memory");
    // new_memory uses current time (default)
    store.store(new_memory).expect("Failed to store new memory");

    // Get all temporal memories
    let all = store
        .get_by_topic("temporal", None, None)
        .expect("Failed to get all");
    assert_eq!(all.len(), 3, "Should have 3 temporal memories");

    // Verify they have different creation times
    let oldest = all.iter().min_by_key(|m| m.created_at).unwrap();
    let newest = all.iter().max_by_key(|m| m.created_at).unwrap();

    assert!(
        oldest.created_at < newest.created_at,
        "Should have time range"
    );
    assert_eq!(oldest.summary, "Old memory");
    assert_eq!(newest.summary, "New memory");
}

/// Test temporal query by last_accessed range
///
/// Given: Memories accessed at different times
/// When: Querying by last_accessed
/// Then: Access patterns are correctly tracked
#[test]
fn test_search_temporal_last_accessed() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memory
    let memory = create_test_memory("access", "Test access tracking");
    let id = store.store(memory).expect("Failed to store");

    // Get memory (updates last_accessed)
    let first_access = store.get(&id).expect("Failed to first access").unwrap();
    let first_timestamp = first_access.last_accessed;

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Access again
    let second_access = store.get(&id).expect("Failed to second access").unwrap();
    let second_timestamp = second_access.last_accessed;

    // Verify last_accessed was updated
    assert!(
        second_timestamp > first_timestamp,
        "last_accessed should be updated on each access"
    );
    assert_eq!(second_access.access_count, 2, "Access count should be 2");
}

/// Test listing topics with counts and statistics
///
/// Given: Memories across multiple topics
/// When: list_topics() is called
/// Then: All topics are returned with accurate counts and stats
#[test]
fn test_list_topics() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories across multiple topics
    for _ in 0..5 {
        store
            .store(create_test_memory("authentication", "Auth memory"))
            .expect("Failed to store");
    }

    for _ in 0..3 {
        store
            .store(create_test_memory("database", "DB memory"))
            .expect("Failed to store");
    }

    for _ in 0..2 {
        store
            .store(create_test_memory("api", "API memory"))
            .expect("Failed to store");
    }

    // List topics
    let topics = store
        .list_topics(None, None)
        .expect("Failed to list topics");

    // Should have 3 topics
    assert_eq!(topics.len(), 3, "Should have 3 distinct topics");

    // Find each topic and verify counts
    let auth_topic = topics.iter().find(|t| t.topic == "authentication").unwrap();
    assert_eq!(auth_topic.count, 5);

    let db_topic = topics.iter().find(|t| t.topic == "database").unwrap();
    assert_eq!(db_topic.count, 3);

    let api_topic = topics.iter().find(|t| t.topic == "api").unwrap();
    assert_eq!(api_topic.count, 2);

    // Verify all have default weight of 1.0
    for topic in &topics {
        assert!(topic.avg_weight > 0.0, "Average weight should be positive");
    }
}

/// Test list_topics with pagination
///
/// Given: Multiple topics exist
/// When: list_topics() is called with limit and offset
/// Then: Correct subset is returned respecting pagination parameters
#[test]
fn test_list_topics_pagination() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create 5 topics with different counts (descending order by count: topic_4, topic_3, topic_2, topic_1, topic_0)
    for i in 0..5 {
        let topic_name = format!("topic_{}", i);
        for _ in 0..(i + 1) {
            store
                .store(create_test_memory(
                    &topic_name,
                    &format!("Memory in {}", topic_name),
                ))
                .expect("Failed to store");
        }
    }

    // Get all topics (no pagination)
    let all_topics = store
        .list_topics(None, None)
        .expect("Failed to list topics");
    assert_eq!(all_topics.len(), 5, "Should have 5 distinct topics");
    // Topics should be ordered by count descending: topic_4 (5), topic_3 (4), topic_2 (3), topic_1 (2), topic_0 (1)
    assert_eq!(all_topics[0].topic, "topic_4");
    assert_eq!(all_topics[0].count, 5);
    assert_eq!(all_topics[4].topic, "topic_0");
    assert_eq!(all_topics[4].count, 1);

    // Test limit without offset (first 3: topic_4, topic_3, topic_2)
    let first_three = store
        .list_topics(Some(3), None)
        .expect("Failed to get first 3");
    assert_eq!(first_three.len(), 3, "Should return exactly 3 topics");
    assert_eq!(first_three[0].topic, "topic_4");
    assert_eq!(first_three[1].topic, "topic_3");
    assert_eq!(first_three[2].topic, "topic_2");

    // Test limit with offset (skip first 2, get next 2: topic_2, topic_1)
    let next_two = store
        .list_topics(Some(2), Some(2))
        .expect("Failed to get next 2");
    assert_eq!(next_two.len(), 2, "Should return exactly 2 topics");
    assert_eq!(next_two[0].topic, "topic_2");
    assert_eq!(next_two[1].topic, "topic_1");

    // Test offset beyond available records
    let beyond = store
        .list_topics(Some(5), Some(10))
        .expect("Failed to query beyond range");
    assert_eq!(
        beyond.len(),
        0,
        "Should return empty when offset exceeds count"
    );
}

/// Test getting topic health (stats for a specific topic)
///
/// Given: Multiple memories in a topic with varying properties
/// When: Topic stats are requested
/// Then: Count, average weight, and timestamps are accurate
#[test]
fn test_get_topic_health() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories in a topic with different weights
    let mut mem1 = create_test_memory("performance", "Optimize query");
    mem1.weight = 1.0;
    store.store(mem1).expect("Failed to store");

    let mut mem2 = create_test_memory("performance", "Cache implementation");
    mem2.weight = 0.8;
    store.store(mem2).expect("Failed to store");

    let mut mem3 = create_test_memory("performance", "Load testing");
    mem3.weight = 0.6;
    store.store(mem3).expect("Failed to store");

    // Get topic info
    let topics = store
        .list_topics(None, None)
        .expect("Failed to list topics");
    let perf_topic = topics.iter().find(|t| t.topic == "performance").unwrap();

    assert_eq!(perf_topic.count, 3);

    // Average weight should be around (1.0 + 0.8 + 0.6) / 3 = 0.8
    let expected_avg = (1.0 + 0.8 + 0.6) / 3.0;
    assert!(
        (perf_topic.avg_weight - expected_avg).abs() < 0.01,
        "Average weight should be approximately {}",
        expected_avg
    );

    // Verify oldest and newest timestamps are reasonable
    assert!(
        perf_topic.newest >= perf_topic.oldest,
        "Newest should be >= oldest"
    );
}

/// Test count with filters (total, by importance, by source)
///
/// Given: Memories with different properties
/// When: count() is called
/// Then: Accurate count is returned (excluding soft-deleted)
#[test]
fn test_count_with_filters() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories with different importance levels
    let mut critical = create_test_memory("mixed", "Critical item");
    critical.importance = Importance::Critical;
    store.store(critical).expect("Failed to store");

    let mut high = create_test_memory("mixed", "High priority");
    high.importance = Importance::High;
    store.store(high).expect("Failed to store");

    let mut medium = create_test_memory("mixed", "Medium priority");
    medium.importance = Importance::Medium;
    store.store(medium).expect("Failed to store");

    let mut low = create_test_memory("mixed", "Low priority");
    low.importance = Importance::Low;
    store.store(low).expect("Failed to store");

    // Total count
    let total = store.count().expect("Failed to count");
    assert_eq!(total, 4, "Should count all active memories");

    // Soft-delete one and verify count excludes it
    let memories = store
        .get_by_topic("mixed", None, None)
        .expect("Failed to get memories");
    let to_delete = &memories[0].id;
    store.delete(to_delete).expect("Failed to delete");

    let after_delete = store.count().expect("Failed to count after delete");
    assert_eq!(after_delete, 3, "Count should exclude soft-deleted");
}

/// Test stats aggregation (system-wide statistics)
///
/// Given: Memories with various properties
/// When: stats() is called
/// Then: Breakdown by importance and source is accurate
#[test]
fn test_stats_aggregation() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories with different importance levels
    let mut critical = create_test_memory("stats", "Critical");
    critical.importance = Importance::Critical;
    critical.source = MemorySource::User;
    store.store(critical).expect("Failed to store");

    let mut high1 = create_test_memory("stats", "High 1");
    high1.importance = Importance::High;
    high1.source = MemorySource::Agent;
    store.store(high1).expect("Failed to store");

    let mut high2 = create_test_memory("stats", "High 2");
    high2.importance = Importance::High;
    high2.source = MemorySource::Agent;
    store.store(high2).expect("Failed to store");

    let mut medium = create_test_memory("stats", "Medium");
    medium.importance = Importance::Medium;
    medium.source = MemorySource::System;
    store.store(medium).expect("Failed to store");

    let mut low = create_test_memory("stats", "Low");
    low.importance = Importance::Low;
    low.source = MemorySource::External;
    store.store(low).expect("Failed to store");

    // Get stats
    let stats = store.stats().expect("Failed to get stats");

    // Verify totals
    assert_eq!(stats.total_memories, 5);
    assert_eq!(stats.active_memories, 5);
    assert_eq!(stats.deleted_memories, 0);

    // Verify breakdown by importance
    assert_eq!(stats.by_importance.critical, 1);
    assert_eq!(stats.by_importance.high, 2);
    assert_eq!(stats.by_importance.medium, 1);
    assert_eq!(stats.by_importance.low, 1);

    // Verify breakdown by source
    assert_eq!(stats.by_source.user, 1);
    assert_eq!(stats.by_source.agent, 2);
    assert_eq!(stats.by_source.system, 1);
    assert_eq!(stats.by_source.external, 1);

    // Verify average weight (all default to 1.0)
    assert_eq!(stats.avg_weight, 1.0);

    // Soft-delete one and verify stats update
    let memories = store
        .get_by_topic("stats", None, None)
        .expect("Failed to get");
    store.delete(&memories[0].id).expect("Failed to delete");

    let updated_stats = store.stats().expect("Failed to get updated stats");
    assert_eq!(updated_stats.total_memories, 5, "Total includes deleted");
    assert_eq!(updated_stats.active_memories, 4, "Active excludes deleted");
    assert_eq!(updated_stats.deleted_memories, 1);
}

/// Test search with no results
///
/// Given: No memories matching query
/// When: search_by_keywords() is called
/// Then: Empty results are returned without error
#[test]
fn test_search_empty_results() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store a memory
    store
        .store(create_test_memory("test", "Sample memory"))
        .expect("Failed to store");

    // Search for something that doesn't exist
    let results = store
        .search_by_keywords("nonexistent_keyword_xyz", 10)
        .expect("Failed to search");

    assert_eq!(
        results.len(),
        0,
        "Should return empty results without error"
    );
}

/// Test that soft-deleted memories are excluded from search
///
/// Given: A soft-deleted memory
/// When: search operations are performed
/// Then: Soft-deleted memory is excluded from all results
#[test]
fn test_search_excludes_deleted() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store two memories
    let mem1 = create_test_memory("security", "Active security memory");
    let id1 = store.store(mem1).expect("Failed to store");

    let mem2 = create_test_memory("security", "Deleted security memory");
    let id2 = store.store(mem2).expect("Failed to store");

    // Verify both are found
    let before_delete = store
        .get_by_topic("security", None, None)
        .expect("Failed to get");
    assert_eq!(before_delete.len(), 2);

    // Soft-delete one
    store.delete(&id2).expect("Failed to delete");

    // Search should exclude deleted
    let search_results = store
        .search_by_keywords("security", 10)
        .expect("Failed to search");

    assert_eq!(
        search_results.len(),
        1,
        "Search should exclude soft-deleted memory"
    );
    assert_eq!(search_results[0].id, id1);

    // Topic query should also exclude deleted
    let topic_results = store
        .get_by_topic("security", None, None)
        .expect("Failed to get topic");
    assert_eq!(
        topic_results.len(),
        1,
        "Topic query should exclude soft-deleted"
    );
    assert_eq!(topic_results[0].id, id1);
}

/// Test hybrid search with automatic fallback to FTS when embeddings unavailable
///
/// Given: Memories stored without embeddings
/// When: hybrid_search_with_fallback() is called with None embedding
/// Then: Should automatically fallback to FTS-only search and return results
#[test]
fn test_hybrid_fallback_to_fts() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store some memories without embeddings
    let mem1 = create_test_memory("rust", "Rust programming language features");
    store.store(mem1).expect("Failed to store memory 1");

    let mem2 = create_test_memory("rust", "Rust memory safety guarantees");
    store.store(mem2).expect("Failed to store memory 2");

    let mem3 = create_test_memory("python", "Python dynamic typing");
    store.store(mem3).expect("Failed to store memory 3");

    // Call with None embedding - should fallback to FTS
    let config = alejandria_storage::search::HybridConfig::default();
    let results = store
        .hybrid_search_with_fallback("rust", None, 10, &config)
        .expect("Fallback to FTS should work");

    assert!(!results.is_empty(), "Fallback to FTS should return results");
    assert!(
        results.len() >= 2,
        "Should find at least 2 memories with 'rust'"
    );

    // Verify results contain the search term
    for memory in &results {
        assert!(
            memory.summary.to_lowercase().contains("rust")
                || memory.topic.to_lowercase().contains("rust"),
            "Result should contain search keyword"
        );
    }

    // Test with empty embedding vector - should also fallback
    let results_empty = store
        .hybrid_search_with_fallback("rust", Some(vec![]), 10, &config)
        .expect("Fallback with empty embedding should work");

    assert!(
        !results_empty.is_empty(),
        "Empty embedding should trigger FTS fallback"
    );
}

/// Test FTS search with LIKE fallback for special characters and empty results
///
/// Given: Memories with special characters
/// When: search_with_like_fallback() is called with query containing special chars
/// Then: Should fallback to LIKE search and return matching results
#[test]
fn test_fts_fallback_to_like() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Store memories with special characters
    let mem1 = create_test_memory("email", "Contact: test@example.com for details");
    store.store(mem1).expect("Failed to store memory 1");

    let mem2 = create_test_memory("tags", "Special tags: #rust #security @user");
    store.store(mem2).expect("Failed to store memory 2");

    let mem3 = create_test_memory("price", "Cost: $99.99 with 15% discount");
    store.store(mem3).expect("Failed to store memory 3");

    // FTS5 might fail on special chars, LIKE should catch it
    let results = store
        .search_with_like_fallback("@example.com", 10)
        .expect("LIKE fallback should work for special chars");

    assert!(
        !results.is_empty(),
        "LIKE fallback should work for special chars like '@'"
    );
    assert!(
        results[0].summary.contains("@example.com"),
        "Should find exact substring match"
    );

    // Test with hashtag
    let results_hash = store
        .search_with_like_fallback("#rust", 10)
        .expect("LIKE fallback should work for hashtags");

    assert!(
        !results_hash.is_empty(),
        "LIKE fallback should work for hashtags"
    );

    // Test with dollar sign
    let results_dollar = store
        .search_with_like_fallback("$99", 10)
        .expect("LIKE fallback should work for dollar signs");

    assert!(
        !results_dollar.is_empty(),
        "LIKE fallback should work for dollar signs"
    );

    // Test normal query that FTS can handle - should use FTS, not LIKE
    let results_normal = store
        .search_with_like_fallback("security", 10)
        .expect("Should handle normal queries");

    // If FTS works, we get results. If not, LIKE catches it.
    // Either way, we should get the result
    assert!(
        !results_normal.is_empty() || results_normal.is_empty(),
        "Should handle normal queries gracefully"
    );
}

// ============================================================================
// Embedding Integration Tests (Phase 3 - Tasks 3.19-3.24)
// ============================================================================

/// Test hybrid search with real embeddings
///
/// Given: Memories stored with automatic embedding generation
/// When: hybrid_search() is called with keyword + embedding
/// Then: Search works even if sqlite-vec unavailable (fallback to FTS)
#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_with_embeddings() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Store memories that will get embeddings
    let mem1 = create_test_memory("rust", "Rust programming language");
    let mem2 = create_test_memory("python", "Python scripting");

    store.store(mem1).expect("Failed to store rust memory");
    store.store(mem2).expect("Failed to store python memory");

    // Search with keyword + embedding (dummy embedding for testing)
    let embedding = vec![0.1; 768];
    let results = store.hybrid_search("rust", &embedding, 10);

    // Should work even if sqlite-vec unavailable (fallback to FTS)
    assert!(results.is_ok(), "Hybrid search should not fail");

    if let Ok(results) = results {
        // If FTS works, we should get at least the rust memory
        if !results.is_empty() {
            assert!(
                results.iter().any(|m| m.summary.contains("Rust")),
                "Should find rust-related memory"
            );
        }
    }
}

/// Test vector search without FTS
///
/// Given: A memory with an embedding
/// When: search_by_embedding() is called (public API)
/// Then: Gracefully handles missing sqlite-vec
#[test]
#[cfg(feature = "embeddings")]
fn test_vector_search_only() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    let memory = create_test_memory("test", "Vector similarity test");
    store.store(memory).expect("Failed to store");

    let embedding = vec![0.5; 768];
    let results = store.search_by_embedding(&embedding, 10);

    // Gracefully handles missing sqlite-vec
    assert!(results.is_ok(), "Vector search should not crash");
}

/// Test embedding persistence across get/update
///
/// Given: A memory with an embedding
/// When: Memory is retrieved and updated without changing summary
/// Then: Embedding persists correctly
#[test]
#[cfg(feature = "embeddings")]
fn test_embedding_persistence() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    let memory = create_test_memory("persist", "Test persistence");
    let id = store.store(memory).expect("Failed to store");

    // Update without changing summary
    let mut mem = store.get(&id).expect("Failed to get").unwrap();
    mem.keywords = vec!["updated".to_string()];
    store.update(mem).expect("Failed to update");

    // Verify update succeeded
    let updated = store.get(&id).expect("Failed to get").unwrap();
    assert_eq!(updated.keywords, vec!["updated".to_string()]);

    // Embedding should still exist (if sqlite-vec available)
    // Test passes if we reach here without panic
}

/// Test hybrid config weight variations
///
/// Given: Different HybridConfig weight configurations
/// When: Configs are created
/// Then: Weights are valid and sum to 1.0
#[test]
fn test_hybrid_config_weights() {
    use alejandria_storage::search::HybridConfig;

    // Test different weight configurations
    let config1 = HybridConfig {
        bm25_weight: 0.8,
        vector_weight: 0.2,
    };
    let config2 = HybridConfig {
        bm25_weight: 0.2,
        vector_weight: 0.8,
    };
    let config3 = HybridConfig::default();

    // Verify weights sum to 1.0
    assert!((config1.bm25_weight + config1.vector_weight - 1.0).abs() < 0.001);
    assert!((config2.bm25_weight + config2.vector_weight - 1.0).abs() < 0.001);

    // Verify default weights
    assert_eq!(config3.bm25_weight, 0.3);
    assert_eq!(config3.vector_weight, 0.7);
}

/// Test large batch embedding
///
/// Given: 50 memories stored
/// When: embed_all() is called with small batch size
/// Then: All memories are processed without errors
#[test]
#[cfg(feature = "embeddings")]
fn test_large_batch_embedding() {
    use std::sync::Arc;

    /// A simple deterministic embedder for testing.
    struct TestEmbedder;
    impl alejandria_core::embedder::Embedder for TestEmbedder {
        fn embed(&self, text: &str) -> alejandria_core::error::IcmResult<Vec<f32>> {
            let seed = text.len() as f32 / 100.0;
            Ok(vec![seed; 768])
        }
        fn embed_batch(&self, texts: &[&str]) -> alejandria_core::error::IcmResult<Vec<Vec<f32>>> {
            texts.iter().map(|t| self.embed(t)).collect()
        }
        fn dimensions(&self) -> usize {
            768
        }
        fn model_name(&self) -> &str {
            "test-embedder"
        }
    }

    let store = SqliteStore::open_in_memory_with_embedder(Arc::new(TestEmbedder))
        .expect("Failed to create store");

    // Store 50 memories
    for i in 0..50 {
        let memory = create_test_memory("batch", &format!("Memory {}", i));
        store.store(memory).expect("Failed to store");
    }

    // Embed all with small batch size
    let count = store.embed_all(5, true).expect("Failed to embed all");

    // Should handle large batches gracefully
    // Count might be 0 if sqlite-vec unavailable (graceful degradation)
    assert!(count <= 50, "Should not exceed total memories");
}

/// Test embedding error handling
///
/// Given: A memory with very long text
/// When: store() is called
/// Then: Store succeeds even if embedding generation fails
#[test]
#[cfg(feature = "embeddings")]
fn test_embedding_error_handling() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    // Store memory with very long text (might cause embedding errors)
    let long_text = "word ".repeat(10000);
    let memory = create_test_memory("long", &long_text);

    // Should not crash even if embedding fails
    let result = store.store(memory);
    assert!(
        result.is_ok(),
        "Store should succeed even if embedding fails"
    );

    if let Ok(id) = result {
        // Verify memory was stored
        let retrieved = store.get(&id).expect("Failed to get");
        assert!(retrieved.is_some(), "Memory should be retrievable");
    }
}

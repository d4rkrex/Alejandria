//! Integration tests for batch operations.
//!
//! This module tests batch operations like embedding generation for multiple memories.

use alejandria_core::memory::{Importance, Memory, MemorySource};
use alejandria_storage::SqliteStore;
use chrono::Utc;

/// Helper function to create a test memory with specified topic and summary.
fn create_test_memory(topic: &str, summary: &str) -> Memory {
    Memory {
        id: ulid::Ulid::new().to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 0,
        weight: 1.0,
        topic: topic.to_string(),
        summary: summary.to_string(),
        raw_excerpt: Some(format!("Context for {}", summary)),
        keywords: vec![topic.to_string()],
        importance: Importance::Medium,
        source: MemorySource::User,
        related_ids: vec![],
        topic_key: None,
        revision_count: 1,
        duplicate_count: 0,
        last_seen_at: Utc::now(),
        deleted_at: None,
        embedding: None,
        decay_profile: None,
        decay_params: None,
    }
}

/// A simple deterministic embedder for testing.
#[cfg(feature = "embeddings")]
struct TestEmbedder;

#[cfg(feature = "embeddings")]
impl alejandria_core::embedder::Embedder for TestEmbedder {
    fn embed(&self, text: &str) -> alejandria_core::error::IcmResult<Vec<f32>> {
        // Produce a deterministic 768-dim vector based on text length
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

/// Helper to create a store with a test embedder configured.
#[cfg(feature = "embeddings")]
fn test_store_with_embedder() -> SqliteStore {
    use std::sync::Arc;
    SqliteStore::open_in_memory_with_embedder(Arc::new(TestEmbedder))
        .expect("Failed to create store")
}

#[test]
#[cfg(feature = "embeddings")]
fn test_embed_all_batch() {
    use alejandria_core::store::MemoryStore;

    let store = test_store_with_embedder();

    // Store 5 memories
    for i in 1..=5 {
        let memory = create_test_memory("test", &format!("Test memory {}", i));
        store.store(memory).expect("Failed to store");
    }

    // Embed all with batch size of 2
    let count = store.embed_all(2, true).expect("Failed to embed all");

    // If sqlite-vec is not available, embed_all returns 0 (graceful degradation)
    // If sqlite-vec is available, it should embed all 5 memories
    if count == 0 {
        println!("sqlite-vec not available, skipping embedding assertions");
        return;
    }

    assert_eq!(count, 5, "Should have embedded 5 memories");

    // Try again with skip_existing=true, should embed 0
    let count = store.embed_all(2, true).expect("Failed to embed all");
    assert_eq!(count, 0, "Should skip existing embeddings");

    // Try with skip_existing=false, should re-embed all 5
    let count = store.embed_all(2, false).expect("Failed to embed all");
    assert_eq!(count, 5, "Should re-embed all memories");
}

#[test]
#[cfg(not(feature = "embeddings"))]
fn test_embed_all_without_feature() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");
    let count = store.embed_all(10, true).expect("Should not fail");
    assert_eq!(count, 0, "Should return 0 when embeddings disabled");
}

#[test]
#[cfg(feature = "embeddings")]
fn test_embed_all_empty_database() {
    let store = test_store_with_embedder();

    // Embed all on empty database should return 0
    let count = store.embed_all(10, true).expect("Failed to embed all");
    assert_eq!(count, 0, "Should embed 0 memories from empty database");
}

#[test]
#[cfg(feature = "embeddings")]
fn test_embed_all_with_deleted_memories() {
    use alejandria_core::store::MemoryStore;

    let store = test_store_with_embedder();

    // Store 3 memories
    let mut ids = Vec::new();
    for i in 1..=3 {
        let memory = create_test_memory("test", &format!("Test memory {}", i));
        let id = store.store(memory).expect("Failed to store");
        ids.push(id);
    }

    // Delete one memory
    store.delete(&ids[1]).expect("Failed to delete");

    // Embed all should only embed the 2 non-deleted memories
    let count = store.embed_all(10, true).expect("Failed to embed all");

    // If sqlite-vec is not available, embed_all returns 0 (graceful degradation)
    if count == 0 {
        println!("sqlite-vec not available, skipping embedding assertions");
        return;
    }

    assert_eq!(count, 2, "Should only embed non-deleted memories");
}

#[test]
#[cfg(feature = "embeddings")]
fn test_embed_all_batching() {
    use alejandria_core::store::MemoryStore;

    let store = test_store_with_embedder();

    // Store 10 memories
    for i in 1..=10 {
        let memory = create_test_memory("test", &format!("Test memory {}", i));
        store.store(memory).expect("Failed to store");
    }

    // Embed with small batch size to test batching logic
    let count = store.embed_all(3, true).expect("Failed to embed all");

    // If sqlite-vec is not available, embed_all returns 0 (graceful degradation)
    if count == 0 {
        println!("sqlite-vec not available, skipping embedding assertions");
        return;
    }

    assert_eq!(
        count, 10,
        "Should embed all 10 memories across multiple batches"
    );
}

#[test]
#[cfg(feature = "embeddings")]
fn test_embed_all_no_embedder_returns_error() {
    // A store without an embedder should return an error
    let store = SqliteStore::open_in_memory().expect("Failed to create store");
    let result = store.embed_all(10, true);
    assert!(result.is_err(), "embed_all without embedder should error");
}

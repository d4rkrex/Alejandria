//! Integration tests for the hybrid search pipeline (Phase 3 vector search).
//!
//! These tests verify the end-to-end hybrid search functionality:
//! - Store → embed → recall with semantic query → hybrid scoring
//! - `hybrid_search_with_fallback` and `hybrid_search_with_fallback_scored`
//! - `get()` loading embeddings from `vec_memories`
//! - `stats()` reporting `embeddings_enabled` correctly
//! - FTS fallback when no embedding is provided
//! - Graceful degradation when sqlite-vec is unavailable

use alejandria_core::{
    memory::{Importance, Memory, MemorySource},
    store::MemoryStore,
};
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
        owner_key_hash: "LEGACY_SYSTEM".to_string(),
    }
}

/// A deterministic embedder for testing. Produces 768-dim vectors based on text length.
/// Different text lengths → different vectors → different cosine distances.
#[cfg(feature = "embeddings")]
struct TestEmbedder;

#[cfg(feature = "embeddings")]
impl alejandria_core::embedder::Embedder for TestEmbedder {
    fn embed(&self, text: &str) -> alejandria_core::error::IcmResult<Vec<f32>> {
        // Produce a deterministic 768-dim vector based on text length.
        // This ensures different texts get different embeddings, which lets us
        // verify that vector search returns different scores for different texts.
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

// =============================================================================
// stats() reports embeddings_enabled correctly
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_stats_embeddings_enabled_with_embedder() {
    let store = test_store_with_embedder();
    let stats = store.stats().expect("Failed to get stats");
    assert!(
        stats.embeddings_enabled,
        "stats.embeddings_enabled should be true when embedder is configured"
    );
}

#[test]
fn test_stats_embeddings_disabled_without_embedder() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");
    let stats = store.stats().expect("Failed to get stats");
    assert!(
        !stats.embeddings_enabled,
        "stats.embeddings_enabled should be false when no embedder is configured"
    );
}

// =============================================================================
// get() loads embeddings from vec_memories
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_get_loads_embedding_after_embed_all() {
    let store = test_store_with_embedder();

    let mem = create_test_memory("rust", "Rust ownership and borrowing rules");
    let id = store.store(mem).expect("Failed to store");

    // Before embed_all, get() should return the memory (embedding may or may not be set
    // depending on whether store() auto-embeds)
    let _before = store.get(&id).expect("Failed to get").expect("Not found");

    // Run embed_all to ensure embeddings are in vec_memories
    let count = store.embed_all(10, true).expect("embed_all failed");

    if count == 0 {
        println!("sqlite-vec not available, skipping embedding load assertions");
        return;
    }

    // After embed_all, get() should return the memory WITH embedding loaded
    let after = store.get(&id).expect("Failed to get").expect("Not found");
    assert!(
        after.embedding.is_some(),
        "get() should load embedding from vec_memories after embed_all"
    );
    assert_eq!(
        after.embedding.as_ref().unwrap().len(),
        768,
        "Loaded embedding should have 768 dimensions"
    );
}

#[test]
fn test_get_returns_none_embedding_without_embedder() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    let mem = create_test_memory("test", "Simple test memory");
    let id = store.store(mem).expect("Failed to store");

    let loaded = store.get(&id).expect("Failed to get").expect("Not found");
    assert!(
        loaded.embedding.is_none(),
        "get() should return None embedding when no embedder is configured"
    );
}

// =============================================================================
// hybrid_search_with_fallback: FTS fallback when no embedding
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_fts_fallback_without_embedding() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    // Store memories with different topics
    let mem1 = create_test_memory("security", "JWT authentication with token validation");
    let mem2 = create_test_memory("database", "PostgreSQL index optimization techniques");
    let mem3 = create_test_memory("security", "OAuth2 authentication flow implementation");

    store.store(mem1).expect("Failed to store");
    store.store(mem2).expect("Failed to store");
    store.store(mem3).expect("Failed to store");

    let config = HybridConfig::default();

    // Search without embedding → should use FTS fallback
    let results = store
        .hybrid_search_with_fallback("authentication", None, 10, &config)
        .expect("Search failed");

    assert!(
        !results.is_empty(),
        "FTS fallback should return results for 'authentication'"
    );

    // All results should contain "authentication" in summary
    for r in &results {
        assert!(
            r.summary.to_lowercase().contains("authentication"),
            "FTS fallback results should match keyword: got '{}'",
            r.summary
        );
    }
}

// =============================================================================
// hybrid_search_with_fallback_scored: returns ScoredMemory
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_scored_returns_scores() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    let mem1 = create_test_memory("security", "JWT authentication with token validation");
    let mem2 = create_test_memory("testing", "Unit testing best practices for Rust");

    store.store(mem1).expect("Failed to store");
    store.store(mem2).expect("Failed to store");

    let config = HybridConfig::default();

    // Search without embedding → FTS fallback with scores
    let results = store
        .hybrid_search_with_fallback_scored("authentication", None, 10, &config)
        .expect("Scored search failed");

    assert!(
        !results.is_empty(),
        "Scored search should return results for 'authentication'"
    );

    // All results should have score > 0
    for r in &results {
        assert!(r.score > 0.0, "Score should be positive, got {}", r.score);
        assert!(
            r.score <= 1.0,
            "FTS fallback score should be <= 1.0, got {}",
            r.score
        );
    }
}

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_scored_fts_fallback_preserves_order() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    // Store memories — only one matches "authentication"
    let mem1 = create_test_memory("security", "JWT authentication with token validation");
    let mem2 = create_test_memory("database", "Database schema migration strategies");

    store.store(mem1).expect("Failed to store");
    store.store(mem2).expect("Failed to store");

    let config = HybridConfig::default();

    let results = store
        .hybrid_search_with_fallback_scored("authentication", None, 10, &config)
        .expect("Scored search failed");

    // Scores should be in descending order
    for window in results.windows(2) {
        assert!(
            window[0].score >= window[1].score,
            "Scores should be in descending order: {} >= {}",
            window[0].score,
            window[1].score
        );
    }
}

// =============================================================================
// Full hybrid pipeline: store → embed → search with embedding
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_full_hybrid_search_pipeline() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    // Store several memories with different content lengths (so TestEmbedder
    // generates different vectors for each)
    let mem1 = create_test_memory("security", "JWT authentication");
    let mem2 = create_test_memory(
        "security",
        "OAuth2 authentication flow with refresh tokens and session management",
    );
    let mem3 = create_test_memory("database", "SQL query optimization and indexing");
    let mem4 = create_test_memory(
        "architecture",
        "Microservices communication patterns and service mesh",
    );

    store.store(mem1).expect("Failed to store");
    store.store(mem2).expect("Failed to store");
    store.store(mem3).expect("Failed to store");
    store.store(mem4).expect("Failed to store");

    // Embed all memories
    let count = store.embed_all(10, true).expect("embed_all failed");
    if count == 0 {
        println!("sqlite-vec not available, skipping full hybrid pipeline test");
        return;
    }
    assert_eq!(count, 4, "Should embed all 4 memories");

    let config = HybridConfig::default();

    // Generate a query embedding (same TestEmbedder logic)
    let query_embedding = vec!["authentication".len() as f32 / 100.0; 768];

    // Full hybrid search WITH embedding
    let results = store
        .hybrid_search_with_fallback("authentication", Some(query_embedding.clone()), 10, &config)
        .expect("Hybrid search failed");

    assert!(
        !results.is_empty(),
        "Full hybrid search should return results"
    );

    // Scored variant
    let scored_results = store
        .hybrid_search_with_fallback_scored("authentication", Some(query_embedding), 10, &config)
        .expect("Scored hybrid search failed");

    assert!(
        !scored_results.is_empty(),
        "Scored hybrid search should return results"
    );

    // All scored results should have positive scores
    for r in &scored_results {
        assert!(r.score > 0.0, "Hybrid score should be positive");
    }
}

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_with_empty_embedding_falls_back() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    let mem = create_test_memory("security", "Authentication best practices");
    store.store(mem).expect("Failed to store");

    let config = HybridConfig::default();

    // Empty embedding vector → should fall back to FTS
    let results = store
        .hybrid_search_with_fallback("authentication", Some(vec![]), 10, &config)
        .expect("Search failed");

    assert!(
        !results.is_empty(),
        "Empty embedding should trigger FTS fallback"
    );
}

// =============================================================================
// embed_all + search integration
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_embed_all_then_search() {
    let store = test_store_with_embedder();

    // Store memories
    for i in 1..=5 {
        let mem = create_test_memory(
            "programming",
            &format!("Programming concept number {} with detailed explanation", i),
        );
        store.store(mem).expect("Failed to store");
    }

    let count = store.embed_all(10, true).expect("embed_all failed");
    if count == 0 {
        println!("sqlite-vec not available, skipping embed+search test");
        return;
    }

    // All 5 should now have embeddings
    assert_eq!(count, 5);

    // Verify we can still search via FTS after embedding
    let results = store
        .search_by_keywords("programming", 10)
        .expect("FTS search failed");
    assert_eq!(results.len(), 5, "FTS should still find all 5 memories");
}

#[test]
#[cfg(feature = "embeddings")]
fn test_embed_all_skip_existing_works() {
    let store = test_store_with_embedder();

    let mem = create_test_memory("test", "Memory to embed once");
    store.store(mem).expect("Failed to store");

    let count1 = store.embed_all(10, true).expect("embed_all failed");
    if count1 == 0 {
        println!("sqlite-vec not available, skipping test");
        return;
    }
    assert_eq!(count1, 1);

    // Second call with skip_existing=true → should embed 0
    let count2 = store.embed_all(10, true).expect("embed_all failed");
    assert_eq!(count2, 0, "Should skip already-embedded memory");

    // With skip_existing=false → should re-embed
    let count3 = store.embed_all(10, false).expect("embed_all failed");
    assert_eq!(count3, 1, "Should re-embed with skip_existing=false");
}

// =============================================================================
// Graceful degradation without embedder
// =============================================================================

#[test]
fn test_hybrid_search_fallback_without_embedder() {
    use alejandria_storage::search::HybridConfig;

    let store = SqliteStore::open_in_memory().expect("Failed to create store");

    let mem = create_test_memory("security", "Authentication security patterns");
    store.store(mem).expect("Failed to store");

    let config = HybridConfig::default();

    // Without embedder, search should still work via FTS fallback
    let results = store
        .hybrid_search_with_fallback("authentication", None, 10, &config)
        .expect("Search should not fail without embedder");

    assert!(
        !results.is_empty(),
        "FTS fallback should work without embedder"
    );
}

#[test]
fn test_embed_all_without_embedder_returns_error() {
    let store = SqliteStore::open_in_memory().expect("Failed to create store");
    let result = store.embed_all(10, true);
    assert!(
        result.is_err(),
        "embed_all without embedder should return error"
    );
}

// =============================================================================
// Scored search: min_score filtering behavior
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_scored_search_all_results_have_valid_scores() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    // Store multiple memories
    let summaries = vec![
        "Rust ownership model with borrowing and lifetimes",
        "Python garbage collection and reference counting",
        "JavaScript event loop and async await patterns",
        "Go goroutines and channels for concurrency",
    ];

    for (i, summary) in summaries.iter().enumerate() {
        let topic = if i < 2 { "systems" } else { "scripting" };
        let mem = create_test_memory(topic, summary);
        store.store(mem).expect("Failed to store");
    }

    let config = HybridConfig::default();

    let results = store
        .hybrid_search_with_fallback_scored("Rust", None, 10, &config)
        .expect("Scored search failed");

    // All scores should be in valid range [0, 1]
    for r in &results {
        assert!(
            r.score >= 0.0 && r.score <= 1.0,
            "Score should be in [0, 1], got {}",
            r.score
        );
    }
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_empty_database() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();
    let config = HybridConfig::default();

    let results = store
        .hybrid_search_with_fallback("anything", None, 10, &config)
        .expect("Search on empty DB should not error");

    assert!(
        results.is_empty(),
        "Empty database should return no results"
    );
}

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_scored_empty_database() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();
    let config = HybridConfig::default();

    let results = store
        .hybrid_search_with_fallback_scored("anything", None, 10, &config)
        .expect("Scored search on empty DB should not error");

    assert!(
        results.is_empty(),
        "Empty database should return no scored results"
    );
}

#[test]
#[cfg(feature = "embeddings")]
fn test_get_nonexistent_returns_none() {
    let store = test_store_with_embedder();

    let result = store
        .get("nonexistent-id-12345")
        .expect("get should not error for missing ID");
    assert!(
        result.is_none(),
        "get() for nonexistent ID should return None"
    );
}

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_respects_limit() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    // Store 10 memories with the keyword "pattern"
    for i in 1..=10 {
        let mem = create_test_memory(
            "design",
            &format!("Design pattern number {} for software architecture", i),
        );
        store.store(mem).expect("Failed to store");
    }

    let config = HybridConfig::default();

    // Request limit of 3
    let results = store
        .hybrid_search_with_fallback("pattern", None, 3, &config)
        .expect("Search failed");

    assert!(
        results.len() <= 3,
        "Should respect limit of 3, got {} results",
        results.len()
    );
}

#[test]
#[cfg(feature = "embeddings")]
fn test_hybrid_search_scored_respects_limit() {
    use alejandria_storage::search::HybridConfig;

    let store = test_store_with_embedder();

    // Store 10 memories with the keyword "testing"
    for i in 1..=10 {
        let mem = create_test_memory(
            "qa",
            &format!("Testing strategy number {} for quality assurance", i),
        );
        store.store(mem).expect("Failed to store");
    }

    let config = HybridConfig::default();

    let results = store
        .hybrid_search_with_fallback_scored("testing", None, 3, &config)
        .expect("Scored search failed");

    assert!(
        results.len() <= 3,
        "Scored search should respect limit of 3, got {} results",
        results.len()
    );
}

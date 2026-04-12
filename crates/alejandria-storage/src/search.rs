//! Hybrid search implementation combining BM25 and cosine similarity.
//!
//! This module provides the core hybrid search logic that merges results from:
//! - FTS5 full-text search (BM25 algorithm)
//! - sqlite-vec cosine similarity (vector embeddings)
//!
//! The hybrid algorithm follows these steps:
//! 1. Execute both BM25 and cosine searches independently
//! 2. Normalize scores from both to [0, 1] range
//! 3. Compute weighted score: 30% BM25 + 70% cosine (configurable)
//! 4. Deduplicate by memory ID, keeping highest score
//! 5. Sort by final score descending

use alejandria_core::memory::Memory;
use std::collections::HashMap;

/// Search result with normalized score.
#[derive(Debug, Clone)]
pub struct ScoredMemory {
    pub memory: Memory,
    pub score: f32,
}

/// Configuration for hybrid search scoring.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Weight for BM25 (keyword) score (default: 0.3)
    pub bm25_weight: f32,
    /// Weight for cosine (vector) score (default: 0.7)
    pub vector_weight: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            bm25_weight: 0.3,
            vector_weight: 0.7,
        }
    }
}

impl HybridConfig {
    pub fn new(bm25_weight: f32, vector_weight: f32) -> Self {
        Self {
            bm25_weight,
            vector_weight,
        }
    }
}

/// Normalize BM25 rank scores to [0, 1] range.
///
/// BM25 ranks from FTS5 are negative values where MORE NEGATIVE = BETTER match.
/// This function normalizes them to 0-1 where:
/// - Best rank (most negative) → 1.0
/// - Worst rank (least negative/closest to 0) → 0.0
///
/// # Arguments
/// * `results` - Vector of (Memory, rank) tuples where rank is negative BM25 score
///
/// # Returns
/// Vector of ScoredMemory with normalized scores in [0, 1]
pub fn normalize_bm25_scores(results: Vec<(Memory, f32)>) -> Vec<ScoredMemory> {
    if results.is_empty() {
        return Vec::new();
    }

    // Find min and max ranks
    // min_rank will be most negative (best), max_rank will be least negative (worst)
    let min_rank = results
        .iter()
        .map(|(_, rank)| *rank)
        .fold(f32::INFINITY, f32::min);
    let max_rank = results
        .iter()
        .map(|(_, rank)| *rank)
        .fold(f32::NEG_INFINITY, f32::max);

    // Handle edge case: all scores are identical
    if (max_rank - min_rank).abs() < f32::EPSILON {
        return results
            .into_iter()
            .map(|(memory, _)| ScoredMemory { memory, score: 1.0 })
            .collect();
    }

    // Normalize to [0, 1]: (rank - min) / (max - min)
    // Since min is most negative (best) and max is least negative (worst),
    // this naturally gives us 0.0 for best and 1.0 for worst, so we invert
    results
        .into_iter()
        .map(|(memory, rank)| {
            let normalized = (rank - min_rank) / (max_rank - min_rank);
            ScoredMemory {
                memory,
                score: 1.0 - normalized, // Invert: 0 → 1, 1 → 0
            }
        })
        .collect()
}

/// Normalize cosine distance to [0, 1] similarity score.
///
/// Cosine distance from sqlite-vec is in range [0, 2] where:
/// - 0 = identical vectors
/// - 2 = opposite vectors
///
/// This converts to similarity score [0, 1] where:
/// - 1.0 = identical (distance 0)
/// - 0.0 = opposite (distance 2)
///
/// # Arguments
/// * `results` - Vector of (Memory, distance) tuples where distance is cosine distance
///
/// # Returns
/// Vector of ScoredMemory with normalized similarity scores in [0, 1]
pub fn normalize_cosine_scores(results: Vec<(Memory, f32)>) -> Vec<ScoredMemory> {
    results
        .into_iter()
        .map(|(memory, distance)| {
            // Convert distance [0, 2] to similarity [1, 0]
            // similarity = 1 - (distance / 2)
            let score = 1.0 - (distance / 2.0).clamp(0.0, 1.0);
            ScoredMemory { memory, score }
        })
        .collect()
}

/// Merge and score results from BM25 and cosine searches.
///
/// This function:
/// 1. Creates a map of memory_id → (memory, bm25_score, cosine_score)
/// 2. For memories in both results, computes weighted score
/// 3. For memories in only one result, uses that score weighted
/// 4. Sorts by final score descending
///
/// # Arguments
/// * `bm25_results` - Results from FTS5 BM25 search with normalized scores
/// * `cosine_results` - Results from vector search with normalized scores
/// * `config` - Hybrid search configuration (weights)
/// * `limit` - Maximum number of results to return
///
/// # Returns
/// Vector of up to `limit` memories sorted by hybrid score (highest first)
pub fn merge_hybrid_results(
    bm25_results: Vec<ScoredMemory>,
    cosine_results: Vec<ScoredMemory>,
    config: &HybridConfig,
    limit: usize,
) -> Vec<Memory> {
    // Build map: memory_id → (memory, bm25_score, cosine_score)
    let mut scores: HashMap<String, (Memory, f32, f32)> = HashMap::new();

    // Add BM25 results
    for scored in bm25_results {
        scores.insert(scored.memory.id.clone(), (scored.memory, scored.score, 0.0));
    }

    // Add/merge cosine results
    for scored in cosine_results {
        scores
            .entry(scored.memory.id.clone())
            .and_modify(|e| e.2 = scored.score)
            .or_insert((scored.memory, 0.0, scored.score));
    }

    // Compute weighted scores and collect
    let mut results: Vec<(Memory, f32)> = scores
        .into_values()
        .map(|(memory, bm25_score, cosine_score)| {
            let final_score =
                (config.bm25_weight * bm25_score) + (config.vector_weight * cosine_score);
            (memory, final_score)
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N and extract memories
    results
        .into_iter()
        .take(limit)
        .map(|(memory, _score)| memory)
        .collect()
}

/// Merge BM25 and cosine similarity results with weighted scoring, returning scores.
///
/// Same algorithm as `merge_hybrid_results` but returns `ScoredMemory` with the
/// final weighted score attached for downstream filtering (e.g. min_score threshold).
pub fn merge_hybrid_results_scored(
    bm25_results: Vec<ScoredMemory>,
    cosine_results: Vec<ScoredMemory>,
    config: &HybridConfig,
    limit: usize,
) -> Vec<ScoredMemory> {
    let mut scores: HashMap<String, (Memory, f32, f32)> = HashMap::new();

    for scored in bm25_results {
        scores.insert(scored.memory.id.clone(), (scored.memory, scored.score, 0.0));
    }

    for scored in cosine_results {
        scores
            .entry(scored.memory.id.clone())
            .and_modify(|e| e.2 = scored.score)
            .or_insert((scored.memory, 0.0, scored.score));
    }

    let mut results: Vec<ScoredMemory> = scores
        .into_values()
        .map(|(memory, bm25_score, cosine_score)| {
            let final_score =
                (config.bm25_weight * bm25_score) + (config.vector_weight * cosine_score);
            ScoredMemory {
                memory,
                score: final_score,
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results.into_iter().take(limit).collect()
}

// === Embedding Generation ===

/// Generate embedding vector for text using the provided embedder.
///
/// Delegates to the given `Embedder` implementation (e.g. `FastembedEmbedder`).
/// Returns an error when the embeddings feature is disabled.
///
/// # Arguments
/// * `text` - Text to generate embedding for
/// * `embedder` - The embedder implementation to use
///
/// # Returns
/// Embedding vector whose dimensionality depends on the model
///
/// # Errors
/// Returns `IcmError::Embedding` on model failure or feature-disabled
#[cfg(feature = "embeddings")]
pub fn generate_embedding(
    text: &str,
    embedder: &dyn alejandria_core::embedder::Embedder,
) -> alejandria_core::error::IcmResult<Vec<f32>> {
    embedder.embed(text)
}

/// Version without embeddings feature - always returns error.
#[cfg(not(feature = "embeddings"))]
pub fn generate_embedding(
    _text: &str,
    _embedder: &dyn alejandria_core::embedder::Embedder,
) -> alejandria_core::error::IcmResult<Vec<f32>> {
    use alejandria_core::error::IcmError;
    Err(IcmError::Embedding(
        "Embeddings feature not enabled".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alejandria_core::memory::{Importance, Memory, MemorySource};
    use chrono::Utc;

    fn create_test_memory(id: &str, summary: &str) -> Memory {
        Memory {
            id: id.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
            weight: 1.0,
            topic: "test".to_string(),
            summary: summary.to_string(),
            raw_excerpt: None,
            keywords: vec![],
            embedding: None,
            importance: Importance::Medium,
            source: MemorySource::User,
            related_ids: vec![],
            topic_key: None,
            revision_count: 0,
            duplicate_count: 0,
            last_seen_at: Utc::now(),
            deleted_at: None,
            decay_profile: None,
            decay_params: None,
            owner_key_hash: "LEGACY_SYSTEM".to_string(),
        }
    }

    #[test]
    fn test_normalize_bm25_scores() {
        // BM25 scores from FTS5 are negative, with MORE NEGATIVE = BETTER match
        // SQL ORDER BY score ASC returns most negative (best) first
        let results = vec![
            (create_test_memory("1", "first"), -1.5), // Best match (most negative)
            (create_test_memory("2", "second"), -1.0), // Medium match
            (create_test_memory("3", "third"), -0.5), // Worst match (least negative)
        ];

        let normalized = normalize_bm25_scores(results);

        assert_eq!(normalized.len(), 3);
        // Best rank (-1.5) should have highest score
        assert!(normalized[0].score > normalized[1].score);
        assert!(normalized[1].score > normalized[2].score);
        // All scores in [0, 1]
        assert!(normalized[0].score <= 1.0 && normalized[0].score >= 0.0);
    }

    #[test]
    fn test_normalize_cosine_scores() {
        let results = vec![
            (create_test_memory("1", "first"), 0.0),  // Identical
            (create_test_memory("2", "second"), 1.0), // Halfway
            (create_test_memory("3", "third"), 2.0),  // Opposite
        ];

        let normalized = normalize_cosine_scores(results);

        assert_eq!(normalized.len(), 3);
        assert!((normalized[0].score - 1.0).abs() < 0.01); // Distance 0 → score 1.0
        assert!((normalized[1].score - 0.5).abs() < 0.01); // Distance 1 → score 0.5
        assert!((normalized[2].score - 0.0).abs() < 0.01); // Distance 2 → score 0.0
    }

    #[test]
    fn test_merge_hybrid_results() {
        let bm25 = vec![
            ScoredMemory {
                memory: create_test_memory("1", "match both"),
                score: 0.8,
            },
            ScoredMemory {
                memory: create_test_memory("2", "only bm25"),
                score: 0.6,
            },
        ];

        let cosine = vec![
            ScoredMemory {
                memory: create_test_memory("1", "match both"),
                score: 0.9,
            },
            ScoredMemory {
                memory: create_test_memory("3", "only cosine"),
                score: 0.7,
            },
        ];

        let config = HybridConfig::default(); // 30% BM25, 70% cosine
        let results = merge_hybrid_results(bm25, cosine, &config, 10);

        assert_eq!(results.len(), 3);

        // Memory "1" should be first (appears in both with high scores)
        assert_eq!(results[0].id, "1");
        // Final score for "1": 0.3 * 0.8 + 0.7 * 0.9 = 0.24 + 0.63 = 0.87

        // Memory "3" (only cosine 0.7): 0.0 * 0.3 + 0.7 * 0.7 = 0.49
        // Memory "2" (only bm25 0.6): 0.6 * 0.3 + 0.0 * 0.7 = 0.18
        // So "3" should be second, "2" third
        assert_eq!(results[1].id, "3");
        assert_eq!(results[2].id, "2");
    }

    #[test]
    fn test_merge_respects_limit() {
        let bm25 = vec![
            ScoredMemory {
                memory: create_test_memory("1", "first"),
                score: 0.9,
            },
            ScoredMemory {
                memory: create_test_memory("2", "second"),
                score: 0.8,
            },
            ScoredMemory {
                memory: create_test_memory("3", "third"),
                score: 0.7,
            },
        ];

        let cosine = vec![];
        let config = HybridConfig::default();
        let results = merge_hybrid_results(bm25, cosine, &config, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "1");
        assert_eq!(results[1].id, "2");
    }
}

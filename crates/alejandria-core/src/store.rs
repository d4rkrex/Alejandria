//! MemoryStore trait definition for memory CRUD and search operations.
//!
//! This trait defines the core storage interface for episodic memories with
//! hybrid search, temporal decay, and topic management capabilities.

use crate::{IcmResult, Memory};
use chrono::{DateTime, Utc};

/// Abstraction for memory storage operations.
///
/// This trait provides a complete interface for managing episodic memories including:
/// - CRUD operations (store, get, update, delete)
/// - Hybrid search (BM25 + cosine similarity)
/// - Temporal decay and pruning
/// - Topic organization and consolidation
/// - System statistics and health checks
///
/// Implementations must be thread-safe (`Send + Sync`).
///
/// # Examples
///
/// ```ignore
/// use alejandria_core::{MemoryStore, Memory, Importance};
///
/// async fn example(store: &dyn MemoryStore) -> IcmResult<()> {
///     // Store a new memory
///     let memory = Memory::new(
///         "architecture".to_string(),
///         "Chose SQLite for storage".to_string(),
///         Importance::High,
///     )?;
///     let id = store.store(memory).await?;
///
///     // Search for memories
///     let results = store.hybrid_search("database", &embedding, 5).await?;
///
///     Ok(())
/// }
/// ```
pub trait MemoryStore: Send + Sync {
    // === CRUD Operations ===

    /// Store a new memory or update existing via topic_key upsert.
    ///
    /// If the memory has a `topic_key` set, checks for an existing memory with the same
    /// topic_key. If found, updates the existing memory and increments revision_count.
    /// Otherwise, creates a new memory with a generated ULID.
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory to store (id will be generated if empty)
    ///
    /// # Returns
    ///
    /// The ULID of the stored memory (existing or newly created)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let memory = Memory::new("topic".to_string(), "content".to_string(), Importance::Medium)?;
    /// let id = store.store(memory).await?;
    /// ```
    fn store(&self, memory: Memory) -> IcmResult<String>;

    /// Retrieve a memory by ID.
    ///
    /// Returns `None` if the memory doesn't exist or has been soft-deleted.
    ///
    /// # Arguments
    ///
    /// * `id` - The ULID of the memory to retrieve
    ///
    /// # Returns
    ///
    /// `Some(Memory)` if found and active, `None` otherwise
    fn get(&self, id: &str) -> IcmResult<Option<Memory>>;

    /// Update an existing memory with new values.
    ///
    /// Only provided fields in the memory are updated. The `updated_at` timestamp
    /// is automatically set to the current time. If the summary changes, embeddings
    /// are regenerated.
    ///
    /// # Arguments
    ///
    /// * `memory` - Memory with updated fields (must have valid id)
    ///
    /// # Errors
    ///
    /// Returns `IcmError::NotFound` if the memory doesn't exist or is soft-deleted.
    fn update(&self, memory: Memory) -> IcmResult<()>;

    /// Soft-delete a memory by setting its deleted_at timestamp.
    ///
    /// Soft-deleted memories are excluded from search results but remain in the database.
    /// Permanent deletion is not provided in the MVP.
    ///
    /// # Arguments
    ///
    /// * `id` - The ULID of the memory to delete
    ///
    /// # Errors
    ///
    /// Returns `IcmError::NotFound` if the memory doesn't exist.
    fn delete(&self, id: &str) -> IcmResult<()>;

    // === Search Operations ===

    /// Search memories using FTS5 keyword search (BM25 ranking).
    ///
    /// # Arguments
    ///
    /// * `query` - Search query text
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Vector of memories ranked by BM25 score (descending)
    fn search_by_keywords(&self, query: &str, limit: usize) -> IcmResult<Vec<Memory>>;

    /// Search memories using vector similarity (cosine distance).
    ///
    /// # Arguments
    ///
    /// * `embedding` - Query embedding vector (768 dimensions)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Vector of memories ranked by cosine similarity (descending)
    fn search_by_embedding(&self, embedding: &[f32], limit: usize) -> IcmResult<Vec<Memory>>;

    /// Hybrid search combining BM25 and cosine similarity with weighted scoring.
    ///
    /// Executes both keyword search (FTS5) and vector search, normalizes scores to [0, 1],
    /// computes weighted sum (30% BM25, 70% cosine by default), and returns merged results.
    ///
    /// Updates `access_count` and `last_accessed` for returned memories.
    /// Triggers decay if >24h since last decay run.
    ///
    /// Falls back to FTS-only if embeddings unavailable.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query text
    /// * `embedding` - Query embedding vector (768 dimensions)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Vector of memories ranked by weighted hybrid score (descending)
    fn hybrid_search(&self, query: &str, embedding: &[f32], limit: usize)
        -> IcmResult<Vec<Memory>>;

    // === Lifecycle Operations ===

    /// Apply temporal decay to all non-Critical memories.
    ///
    /// Updates memory weights based on importance multipliers and access patterns:
    /// - Critical: never decays (0.0x)
    /// - High: 0.5x base rate
    /// - Medium: 1.0x base rate
    /// - Low: 2.0x base rate
    ///
    /// Access count dampens decay: `effective_rate = base_rate × mult / (1 + access_count × 0.1)`
    ///
    /// # Arguments
    ///
    /// * `base_rate` - Base decay rate per day (e.g., 0.01 = 1% daily)
    ///
    /// # Returns
    ///
    /// Number of memories updated
    fn apply_decay(&self, base_rate: f32) -> IcmResult<usize>;

    /// Prune (soft-delete) memories below a weight threshold.
    ///
    /// Automatically excludes Critical and High importance memories regardless of weight.
    ///
    /// # Arguments
    ///
    /// * `weight_threshold` - Memories with weight below this are pruned
    ///
    /// # Returns
    ///
    /// Number of memories pruned
    fn prune(&self, weight_threshold: f32) -> IcmResult<usize>;

    // === Organization Operations ===

    /// Retrieve all memories in a specific topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name to filter by
    /// * `limit` - Optional maximum number of memories to return (default: None = all)
    /// * `offset` - Optional number of memories to skip (default: None = 0)
    ///
    /// # Returns
    ///
    /// Vector of active memories in the topic, paginated if limit is provided
    fn get_by_topic(
        &self,
        topic: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> IcmResult<Vec<Memory>>;

    /// Retrieve a memory by its topic_key.
    ///
    /// Topic keys provide semantic handles for upsert workflows.
    ///
    /// # Arguments
    ///
    /// * `topic_key` - The topic key to lookup
    ///
    /// # Returns
    ///
    /// `Some(Memory)` if found, `None` otherwise
    fn get_by_topic_key(&self, topic_key: &str) -> IcmResult<Option<Memory>>;

    /// List all distinct topics with memory counts and statistics.
    ///
    /// # Arguments
    ///
    /// * `limit` - Optional maximum number of topics to return (default: None = all)
    /// * `offset` - Optional number of topics to skip (default: None = 0)
    ///
    /// # Returns
    ///
    /// Vector of topic information sorted by memory count (descending), paginated if limit is provided
    fn list_topics(&self, limit: Option<usize>, offset: Option<usize>)
        -> IcmResult<Vec<TopicInfo>>;

    /// Consolidate memories in a topic into a high-level summary memory.
    ///
    /// Creates a new High-importance memory containing aggregated keywords and themes
    /// from source memories. Source memory IDs are stored in `related_ids`.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to consolidate
    /// * `min_memories` - Minimum number of memories required for consolidation
    /// * `min_weight` - Minimum weight threshold for including memories
    ///
    /// # Returns
    ///
    /// The ULID of the consolidated memory
    ///
    /// # Errors
    ///
    /// Returns `IcmError::InvalidInput` if insufficient memories meet criteria.
    fn consolidate_topic(
        &self,
        topic: &str,
        min_memories: usize,
        min_weight: f32,
    ) -> IcmResult<String>;

    // === Statistics Operations ===

    /// Count total active memories (excluding soft-deleted).
    ///
    /// # Returns
    ///
    /// Number of active memories in the store
    fn count(&self) -> IcmResult<usize>;

    /// Get comprehensive system statistics.
    ///
    /// # Returns
    ///
    /// Statistics including counts by importance/source, avg weight, timestamps
    fn stats(&self) -> IcmResult<StoreStats>;

    // === Decay Profile Management ===

    /// Set the decay profile for a specific memory.
    ///
    /// Updates the decay strategy and parameters for a memory. The decay profile
    /// determines how the memory's weight decays over time based on access patterns.
    ///
    /// # Arguments
    ///
    /// * `memory_id` - The ULID of the memory to update
    /// * `profile_name` - Name of the decay profile (e.g., "exponential", "spaced-repetition")
    /// * `params` - Optional JSON parameters for the decay strategy (uses defaults if None)
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    ///
    /// # Errors
    ///
    /// Returns `IcmError::NotFound` if the memory doesn't exist or is soft-deleted.
    /// Returns `IcmError::InvalidInput` if the profile name or parameters are invalid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use serde_json::json;
    ///
    /// // Set spaced repetition profile with custom parameters
    /// let params = json!({
    ///     "ease_factor": 2.5,
    ///     "interval_days": 7.0,
    ///     "repetitions": 3
    /// });
    /// store.set_decay_profile(&memory_id, "spaced-repetition", Some(params)).await?;
    /// ```
    fn set_decay_profile(
        &self,
        memory_id: &str,
        profile_name: &str,
        params: Option<serde_json::Value>,
    ) -> IcmResult<()>;

    /// Get decay statistics across all memories.
    ///
    /// Returns aggregated statistics about decay profiles in use, average weights
    /// per profile, and temporal decay trends.
    ///
    /// # Returns
    ///
    /// DecayStats with breakdown by profile type and temporal analysis
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let stats = store.get_decay_stats().await?;
    /// println!("Exponential decay: {} memories", stats.by_profile.get("exponential").unwrap_or(&0));
    /// ```
    fn get_decay_stats(&self) -> IcmResult<DecayStats>;

    // === Import/Export Operations ===

    /// Import memories from a file.
    ///
    /// Loads memories from JSON or CSV format and applies the specified conflict
    /// resolution strategy when imported memories conflict with existing ones.
    ///
    /// # Arguments
    ///
    /// * `input_path` - Path to the file to import
    /// * `mode` - How to handle conflicts (Skip, Update, Replace)
    ///
    /// # Returns
    ///
    /// ImportResult with counts of imported/skipped/updated memories and any errors
    ///
    /// # Errors
    ///
    /// Returns `IcmError::Io` if file cannot be read.
    /// Returns `IcmError::Validation` if imported data is invalid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::path::Path;
    /// use alejandria_core::ImportMode;
    ///
    /// let result = store.import_memories(Path::new("backup.json"), ImportMode::Update)?;
    /// println!("Imported: {}, Skipped: {}, Updated: {}", 
    ///     result.imported, result.skipped, result.updated);
    /// ```
    fn import_memories(
        &self,
        input_path: &std::path::Path,
        mode: crate::ImportMode,
    ) -> IcmResult<crate::ImportResult>;
}

/// Information about a topic with aggregated statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopicInfo {
    /// Topic name
    pub topic: String,

    /// Number of active memories in this topic
    pub count: usize,

    /// Average weight of memories in this topic
    pub avg_weight: f32,

    /// Timestamp of oldest memory in topic
    pub oldest: DateTime<Utc>,

    /// Timestamp of newest memory in topic
    pub newest: DateTime<Utc>,
}

/// System-wide memory store statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoreStats {
    /// Total number of memories (including soft-deleted)
    pub total_memories: usize,

    /// Number of active memories (not soft-deleted)
    pub active_memories: usize,

    /// Number of soft-deleted memories
    pub deleted_memories: usize,

    /// Total database size in megabytes
    pub total_size_mb: f64,

    /// Counts by importance level
    pub by_importance: ImportanceStats,

    /// Counts by source type
    pub by_source: SourceStats,

    /// Average weight across all active memories
    pub avg_weight: f32,

    /// Whether embeddings feature is enabled
    pub embeddings_enabled: bool,

    /// Timestamp of last decay run (if any)
    pub last_decay_at: Option<DateTime<Utc>>,
}

/// Statistics breakdown by importance level.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportanceStats {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

/// Statistics breakdown by memory source.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceStats {
    pub user: usize,
    pub agent: usize,
    pub system: usize,
    pub external: usize,
}

/// Statistics about decay profiles across all memories.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DecayStats {
    /// Total number of memories with explicit decay profiles
    pub total_with_profile: usize,

    /// Number of memories using default decay (NULL profile)
    pub total_default: usize,

    /// Breakdown by profile name (e.g., "exponential" -> count)
    pub by_profile: std::collections::HashMap<String, usize>,

    /// Average weight by profile name
    pub avg_weight_by_profile: std::collections::HashMap<String, f32>,

    /// Number of memories that have decayed below threshold (weight < 0.1)
    pub low_weight_count: usize,

    /// Average weight across all active memories
    pub overall_avg_weight: f32,
}

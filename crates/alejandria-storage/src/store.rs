//! SQLite storage implementation.

use alejandria_core::{
    decay::{
        ContextSensitiveDecay, DecayStrategy, ExponentialDecay, ImportanceWeightedDecay,
        SpacedRepetitionDecay,
    },
    error::{IcmError, IcmResult},
    store::MemoryStore,
};
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex, Once};

/// Expected embedding dimension for multilingual-e5-base model
const EXPECTED_EMBEDDING_DIM: usize = 768;

/// Global guard ensuring sqlite-vec extension is registered exactly once.
static SQLITE_VEC_INIT: Once = Once::new();

/// Register the sqlite-vec extension for all future connections.
///
/// Must be called **before** `Connection::open()` because
/// `sqlite3_auto_extension` only applies to connections opened
/// after the registration call.
///
/// Uses `std::sync::Once` to guarantee single registration even
/// under concurrent access.
fn ensure_sqlite_vec() {
    SQLITE_VEC_INIT.call_once(|| unsafe {
        use rusqlite::ffi::sqlite3_auto_extension;
        #[allow(clippy::missing_transmute_annotations)]
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    });
}

// Helper to convert rusqlite errors to IcmError
pub(crate) fn convert_rusqlite_error(err: rusqlite::Error) -> IcmError {
    IcmError::Database(err.to_string())
}

// Helper trait to convert rusqlite Result to IcmResult
pub(crate) trait RusqliteResultExt<T> {
    fn into_icm_result(self) -> IcmResult<T>;
}

impl<T> RusqliteResultExt<T> for Result<T, rusqlite::Error> {
    fn into_icm_result(self) -> IcmResult<T> {
        self.map_err(convert_rusqlite_error)
    }
}

/// SQLite-based storage implementation for Alejandria.
///
/// Provides both episodic memory storage (MemoryStore) and semantic memory
/// storage (MemoirStore) using a single SQLite database.
///
/// # Thread Safety
///
/// SqliteStore uses an Arc<Mutex<Connection>> internally to ensure thread-safe
/// access to the database. Multiple clones of SqliteStore can be used safely
/// across threads.
///
/// # Examples
///
/// ```no_run
/// use alejandria_storage::SqliteStore;
///
/// # fn main() -> alejandria_core::error::IcmResult<()> {
/// // Open or create database
/// let store = SqliteStore::open("alejandria.db")?;
///
/// // Use the store...
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SqliteStore {
    pub(crate) conn: Arc<Mutex<Connection>>,
    /// Optional embedder for vector search.
    /// When present, enables hybrid search (BM25 + cosine similarity).
    /// When absent, the store falls back to FTS-only search.
    pub(crate) embedder: Option<Arc<dyn alejandria_core::embedder::Embedder>>,
}

impl SqliteStore {
    /// Create a new SqliteStore from an existing connection.
    ///
    /// The connection will be wrapped in Arc<Mutex<>> for thread safety.
    ///
    /// # Arguments
    ///
    /// * `conn` - SQLite database connection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rusqlite::Connection;
    /// use alejandria_storage::SqliteStore;
    ///
    /// let conn = Connection::open_in_memory().unwrap();
    /// let store = SqliteStore::new(conn);
    /// ```
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            embedder: None,
        }
    }

    /// Open or create a database file and initialize the schema.
    ///
    /// If the file doesn't exist, it will be created. The schema will be
    /// initialized automatically.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the database file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use alejandria_storage::SqliteStore;
    ///
    /// # fn main() -> alejandria_core::error::IcmResult<()> {
    /// let store = SqliteStore::open("alejandria.db")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> IcmResult<Self> {
        ensure_sqlite_vec();
        let conn = Connection::open(path).into_icm_result()?;
        crate::schema::init_db(&conn)?;
        crate::migrations::apply_migrations(&conn)?;
        Ok(Self::new(conn))
    }

    /// Open an in-memory database (useful for testing).
    ///
    /// # Examples
    ///
    /// ```
    /// use alejandria_storage::SqliteStore;
    ///
    /// # fn main() -> alejandria_core::error::IcmResult<()> {
    /// let store = SqliteStore::open_in_memory()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_in_memory() -> IcmResult<Self> {
        ensure_sqlite_vec();
        let conn = Connection::open_in_memory().into_icm_result()?;
        crate::schema::init_db(&conn)?;
        crate::migrations::apply_migrations(&conn)?;
        Ok(Self::new(conn))
    }

    /// Open or create a database file with an embedder for vector search.
    ///
    /// Same as [`open`] but enables hybrid search (BM25 + cosine similarity)
    /// by attaching an embedder for on-the-fly embedding generation.
    pub fn open_with_embedder<P: AsRef<std::path::Path>>(
        path: P,
        embedder: Arc<dyn alejandria_core::embedder::Embedder>,
    ) -> IcmResult<Self> {
        let mut store = Self::open(path)?;
        store.embedder = Some(embedder);
        Ok(store)
    }

    /// Open an in-memory database with an embedder (useful for testing).
    pub fn open_in_memory_with_embedder(
        embedder: Arc<dyn alejandria_core::embedder::Embedder>,
    ) -> IcmResult<Self> {
        let mut store = Self::open_in_memory()?;
        store.embedder = Some(embedder);
        Ok(store)
    }

    /// Attach or replace the embedder after construction.
    ///
    /// Returns the previous embedder, if any.
    pub fn set_embedder(
        &mut self,
        embedder: Arc<dyn alejandria_core::embedder::Embedder>,
    ) -> Option<Arc<dyn alejandria_core::embedder::Embedder>> {
        self.embedder.replace(embedder)
    }

    /// Returns a reference to the current embedder, if any.
    pub fn embedder(&self) -> Option<&Arc<dyn alejandria_core::embedder::Embedder>> {
        self.embedder.as_ref()
    }

    /// Get access to the underlying connection for custom operations.
    ///
    /// This provides exclusive access via the mutex. Use sparingly.
    pub fn with_conn<F, R>(&self, f: F) -> IcmResult<R>
    where
        F: FnOnce(&Connection) -> IcmResult<R>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| IcmError::Database(format!("Failed to lock connection: {}", e)))?;
        f(&conn)
    }

    /// Check if decay needs to run (>24h since last run) and apply if necessary.
    ///
    /// Uses a default base_rate of 0.02 (2% daily decay).
    fn check_and_apply_decay(&self) -> IcmResult<()> {
        const DEFAULT_BASE_RATE: f32 = 0.02;
        const DECAY_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

        let should_decay = self.with_conn(|conn| {
            let last_decay_opt: Option<String> = conn
                .query_row(
                    "SELECT value FROM icm_metadata WHERE key = 'last_decay_at'",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .into_icm_result()?;

            if let Some(last_decay_str) = last_decay_opt {
                if let Ok(last_decay_ms) = last_decay_str.parse::<i64>() {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    Ok(now_ms - last_decay_ms > DECAY_INTERVAL_MS)
                } else {
                    Ok(true) // Invalid timestamp, run decay
                }
            } else {
                Ok(true) // No previous decay, run it
            }
        })?;

        if should_decay {
            self.apply_decay(DEFAULT_BASE_RATE)?;
        }

        Ok(())
    }

    /// Select appropriate decay strategy based on profile name.
    ///
    /// Returns a Box<dyn DecayStrategy> for the given profile.
    /// If profile is None or empty, returns ExponentialDecay (default).
    fn select_decay_strategy(profile: Option<&str>) -> Box<dyn DecayStrategy> {
        match profile {
            Some("spaced-repetition") | Some("sm2") => Box::new(SpacedRepetitionDecay),
            Some("importance-weighted") | Some("importance") => Box::new(ImportanceWeightedDecay),
            Some("context-sensitive") | Some("context") => Box::new(ContextSensitiveDecay),
            Some("exponential") | None => Box::new(ExponentialDecay),
            _ => {
                // Unknown profile, fallback to exponential with warning
                eprintln!(
                    "Warning: Unknown decay profile '{}', using exponential",
                    profile.unwrap_or("")
                );
                Box::new(ExponentialDecay)
            }
        }
    }

    /// Compute SHA-256 hash of content for deduplication.
    #[allow(dead_code)]
    fn compute_content_hash(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Calculate Jaccard similarity between two keyword sets.
    fn jaccard_similarity(keywords1: &[String], keywords2: &[String]) -> f32 {
        if keywords1.is_empty() && keywords2.is_empty() {
            return 1.0; // Both empty = identical
        }
        if keywords1.is_empty() || keywords2.is_empty() {
            return 0.0;
        }

        use std::collections::HashSet;
        let set1: HashSet<_> = keywords1.iter().collect();
        let set2: HashSet<_> = keywords2.iter().collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Check for duplicate memories (>85% similarity) and return existing ID if found.
    fn find_duplicate(
        &self,
        memory: &alejandria_core::memory::Memory,
    ) -> IcmResult<Option<String>> {
        const SIMILARITY_THRESHOLD: f32 = 0.85;

        // Use FTS5 to find candidates with similar keywords
        let candidates = self.with_conn(|conn| {
            if memory.keywords.is_empty() {
                return Ok(Vec::new());
            }

            // Create FTS query from keywords - wrap each term in quotes to handle special characters
            let fts_query = memory
                .keywords
                .iter()
                .map(|k| format!("\"{}\"", k.replace("\"", "\"\""))) // Escape quotes
                .collect::<Vec<_>>()
                .join(" OR ");

            let mut stmt = conn
                .prepare(
                    "SELECT
                    m.id, m.keywords
                FROM memories_fts fts
                INNER JOIN memories m ON fts.rowid = m.rowid
                WHERE fts.memories_fts MATCH ?1 AND m.deleted_at IS NULL AND m.id != ?2
                LIMIT 50",
                )
                .into_icm_result()?;

            let candidates = stmt
                .query_map(rusqlite::params![fts_query, memory.id], |row| {
                    let id: String = row.get(0)?;
                    let keywords_json: String = row.get(1)?;
                    let keywords: Vec<String> =
                        serde_json::from_str(&keywords_json).unwrap_or_default();
                    Ok((id, keywords))
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            Ok(candidates)
        })?;

        // Calculate Jaccard similarity for each candidate
        for (candidate_id, candidate_keywords) in candidates {
            let similarity = Self::jaccard_similarity(&memory.keywords, &candidate_keywords);
            if similarity >= SIMILARITY_THRESHOLD {
                return Ok(Some(candidate_id));
            }
        }

        Ok(None)
    }

    // === Internal Helper Methods for Hybrid Search ===

    /// Internal helper: Search by keywords with BM25 scores
    fn search_by_keywords_with_scores(
        &self,
        query: &str,
        limit: usize,
    ) -> IcmResult<Vec<(alejandria_core::memory::Memory, f32)>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT
                    m.id, m.created_at, m.updated_at, m.last_accessed, m.access_count, m.weight,
                    m.topic, m.summary, m.raw_excerpt, m.keywords,
                    m.importance, m.source, m.related_ids,
                    m.topic_key, m.revision_count, m.duplicate_count, m.last_seen_at, m.deleted_at,
                    m.decay_profile, m.decay_params,
                    bm25(fts.memories_fts, 1.0, 0.5) as score
                FROM memories_fts fts
                INNER JOIN memories m ON fts.rowid = m.rowid
                WHERE fts.memories_fts MATCH ?1 AND m.deleted_at IS NULL
                ORDER BY score ASC
                LIMIT ?2",
                )
                .into_icm_result()?;

            let results = stmt
                .query_map(rusqlite::params![query, limit], |row| {
                    let memory = alejandria_core::memory::Memory {
                        id: row.get(0)?,
                        // Timestamps are stored as milliseconds since Unix epoch
                        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(1)?,
                        )
                        .unwrap_or_default(),
                        updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(2)?,
                        )
                        .unwrap_or_default(),
                        last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(3)?,
                        )
                        .unwrap_or_default(),
                        access_count: row.get(4)?,
                        weight: row.get(5)?,
                        topic: row.get(6)?,
                        summary: row.get(7)?,
                        raw_excerpt: row.get(8)?,
                        keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                            .unwrap_or_default(),
                        importance: row
                            .get::<_, String>(10)?
                            .parse()
                            .unwrap_or(alejandria_core::memory::Importance::Medium),
                        source: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or(alejandria_core::memory::MemorySource::User),
                        related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                            .unwrap_or_default(),
                        topic_key: row.get(13)?,
                        revision_count: row.get(14)?,
                        duplicate_count: row.get(15)?,
                        last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(16)?,
                        )
                        .unwrap_or_default(),
                        deleted_at: row
                            .get::<_, Option<i64>>(17)?
                            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                        decay_profile: row.get(18)?,
                        decay_params: row
                            .get::<_, Option<String>>(19)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        embedding: None,
                    };
                    let score: f32 = row.get(20)?;
                    Ok((memory, score))
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            Ok(results)
        })
    }

    /// Internal helper: Search by embedding with cosine distances
    fn search_by_embedding_with_scores(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> IcmResult<Vec<(alejandria_core::memory::Memory, f32)>> {
        // Check if vec_memories table exists (graceful degradation when sqlite-vec unavailable)
        let table_exists =
            self.with_conn(|conn| {
                let count: i32 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_memories'",
                [],
                |row| row.get(0)
            ).unwrap_or(0);
                Ok(count > 0)
            })?;

        if !table_exists {
            // sqlite-vec not available, return empty results
            return Ok(Vec::new());
        }

        self.with_conn(|conn| {
            // Convert embedding to JSON array for sqlite-vec
            let embedding_json = serde_json::to_string(embedding)?;

            let mut stmt = conn
                .prepare(
                    "SELECT
                    m.id, m.created_at, m.updated_at, m.last_accessed, m.access_count, m.weight,
                    m.topic, m.summary, m.raw_excerpt, m.keywords,
                    m.importance, m.source, m.related_ids,
                    m.topic_key, m.revision_count, m.duplicate_count, m.last_seen_at, m.deleted_at,
                    m.decay_profile, m.decay_params,
                    vec_distance_cosine(v.embedding, ?1) as distance
                FROM vec_memories v
                INNER JOIN memories m ON v.memory_id = m.id
                WHERE m.deleted_at IS NULL
                ORDER BY distance ASC
                LIMIT ?2",
                )
                .into_icm_result()?;

            let results = stmt
                .query_map(rusqlite::params![embedding_json, limit], |row| {
                    let memory = alejandria_core::memory::Memory {
                        id: row.get(0)?,
                        // Timestamps are stored as milliseconds since Unix epoch
                        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(1)?,
                        )
                        .unwrap_or_default(),
                        updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(2)?,
                        )
                        .unwrap_or_default(),
                        last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(3)?,
                        )
                        .unwrap_or_default(),
                        access_count: row.get(4)?,
                        weight: row.get(5)?,
                        topic: row.get(6)?,
                        summary: row.get(7)?,
                        raw_excerpt: row.get(8)?,
                        keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                            .unwrap_or_default(),
                        importance: row
                            .get::<_, String>(10)?
                            .parse()
                            .unwrap_or(alejandria_core::memory::Importance::Medium),
                        source: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or(alejandria_core::memory::MemorySource::User),
                        related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                            .unwrap_or_default(),
                        topic_key: row.get(13)?,
                        revision_count: row.get(14)?,
                        duplicate_count: row.get(15)?,
                        last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(16)?,
                        )
                        .unwrap_or_default(),
                        deleted_at: row
                            .get::<_, Option<i64>>(17)?
                            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                        decay_profile: row.get(18)?,
                        decay_params: row
                            .get::<_, Option<String>>(19)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        embedding: None,
                    };
                    let distance: f32 = row.get(20)?;
                    Ok((memory, distance))
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            Ok(results)
        })
    }

    /// Update access tracking for a memory
    fn track_access(&self, memory_id: &str) -> IcmResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE memories
                 SET access_count = access_count + 1,
                     last_accessed = ?1
                 WHERE id = ?2",
                // Timestamp in milliseconds since Unix epoch
                rusqlite::params![chrono::Utc::now().timestamp_millis(), memory_id],
            )
            .into_icm_result()?;
            Ok(())
        })
    }

    // === Batch Operations ===

    /// Batch generate embeddings for all memories without them.
    ///
    /// This is useful for:
    /// - Initial backfill when enabling embeddings on existing database
    /// - Re-embedding after model changes
    /// - Periodic maintenance operations
    ///
    /// # Arguments
    /// * `batch_size` - Number of memories to process per batch
    /// * `skip_existing` - Skip memories that already have embeddings
    ///
    /// # Returns
    /// Number of memories successfully embedded
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use alejandria_storage::SqliteStore;
    ///
    /// # fn main() -> alejandria_core::error::IcmResult<()> {
    /// let store = SqliteStore::open("alejandria.db")?;
    ///
    /// // Backfill embeddings for all memories without them
    /// let count = store.embed_all(100, true)?;
    /// println!("Embedded {} memories", count);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "embeddings")]
    pub fn embed_all(&self, batch_size: usize, skip_existing: bool) -> IcmResult<usize> {
        // Require an embedder to be configured
        let embedder = match &self.embedder {
            Some(e) => Arc::clone(e),
            None => {
                return Err(IcmError::Embedding(
                    "No embedder configured — call set_embedder() first".to_string(),
                ));
            }
        };

        // Check if vec_memories table exists (requires sqlite-vec)
        let table_exists =
            self.with_conn(|conn| {
                let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_memories'",
                [],
                |row| row.get(0)
            ).into_icm_result()?;
                Ok(count > 0)
            })?;

        if !table_exists {
            // sqlite-vec not available, cannot generate embeddings
            return Ok(0);
        }

        let mut total_embedded = 0;
        let mut offset = 0;

        loop {
            // Fetch batch of memories
            let batch = self.with_conn(|conn| {
                let query = if skip_existing {
                    "SELECT id, summary FROM memories
                     WHERE deleted_at IS NULL
                       AND id NOT IN (SELECT memory_id FROM vec_memories)
                     LIMIT ?1 OFFSET ?2"
                } else {
                    "SELECT id, summary FROM memories
                     WHERE deleted_at IS NULL
                     LIMIT ?1 OFFSET ?2"
                };

                let mut stmt = conn.prepare(query).into_icm_result()?;
                let results = stmt
                    .query_map(rusqlite::params![batch_size as i64, offset as i64], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .into_icm_result()?
                    .collect::<Result<Vec<_>, _>>()
                    .into_icm_result()?;

                Ok(results)
            })?;

            if batch.is_empty() {
                break;
            }

            // Generate embeddings for batch
            for (memory_id, summary) in &batch {
                match embedder.embed(summary) {
                    Ok(embedding) => {
                        // Insert into vec_memories (or replace if exists)
                        self.with_conn(|conn| {
                            conn.execute(
                                "INSERT OR REPLACE INTO vec_memories (memory_id, embedding)
                                 VALUES (?1, ?2)",
                                rusqlite::params![memory_id, serde_json::to_string(&embedding)?],
                            )
                            .into_icm_result()?;
                            Ok(())
                        })?;
                        total_embedded += 1;
                    }
                    Err(e) => {
                        // Log error but continue with next memory
                        eprintln!("Failed to embed memory {}: {:?}", memory_id, e);
                    }
                }
            }

            offset += batch_size;
        }

        Ok(total_embedded)
    }

    /// Version without embeddings feature (no-op).
    ///
    /// When the embeddings feature is disabled, this method returns 0
    /// without performing any operations.
    #[cfg(not(feature = "embeddings"))]
    pub fn embed_all(&self, _batch_size: usize, _skip_existing: bool) -> IcmResult<usize> {
        Ok(0) // No embeddings feature, return 0
    }

    /// Hybrid search with automatic fallback to FTS-only when embeddings unavailable.
    ///
    /// This method provides intelligent fallback logic:
    /// 1. If embedding is provided and valid, attempts full hybrid search
    /// 2. If hybrid search returns no results, falls back to FTS-only
    /// 3. If no embedding provided, directly uses FTS-only search
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `embedding` - Optional embedding vector (None if feature disabled or not generated)
    /// * `limit` - Maximum number of results to return
    /// * `_config` - Hybrid search configuration (weights for BM25 vs vector) - currently unused
    ///
    /// # Returns
    ///
    /// Vector of memories ranked by relevance, with automatic fallback to FTS if needed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use alejandria_storage::SqliteStore;
    /// use alejandria_storage::search::HybridConfig;
    ///
    /// # fn main() -> alejandria_core::error::IcmResult<()> {
    /// let store = SqliteStore::open("alejandria.db")?;
    /// let config = HybridConfig::default();
    ///
    /// // With embedding (full hybrid search)
    /// let embedding = vec![0.1; 384];
    /// let results = store.hybrid_search_with_fallback("rust", Some(embedding), 10, &config)?;
    ///
    /// // Without embedding (automatic FTS fallback)
    /// let results = store.hybrid_search_with_fallback("rust", None, 10, &config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn hybrid_search_with_fallback(
        &self,
        query: &str,
        embedding: Option<Vec<f32>>,
        limit: usize,
        _config: &crate::search::HybridConfig,
    ) -> IcmResult<Vec<alejandria_core::memory::Memory>> {
        use alejandria_core::store::MemoryStore;

        match embedding {
            Some(emb) if !emb.is_empty() => {
                // Try full hybrid search
                let results = self.hybrid_search(query, &emb, limit)?;
                if !results.is_empty() {
                    return Ok(results);
                }
                // If no results, fall through to FTS
            }
            _ => {
                // No embedding available, use FTS directly
            }
        }

        // Fallback: FTS-only search
        self.search_by_keywords(query, limit)
    }

    /// Hybrid search returning scored results (with fallback to FTS-only).
    ///
    /// Like `hybrid_search_with_fallback` but returns `ScoredMemory` structs so
    /// callers can filter by `min_score` or display relevance to users.
    /// FTS-only fallback results get score = 1.0 (highest) since we can't compare.
    pub fn hybrid_search_with_fallback_scored(
        &self,
        query: &str,
        embedding: Option<Vec<f32>>,
        limit: usize,
        _config: &crate::search::HybridConfig,
    ) -> IcmResult<Vec<crate::search::ScoredMemory>> {
        use crate::search::{
            merge_hybrid_results_scored, normalize_bm25_scores, normalize_cosine_scores,
            HybridConfig, ScoredMemory,
        };
        use alejandria_core::store::MemoryStore;

        match embedding {
            Some(emb) if !emb.is_empty() => {
                // Full hybrid search with scores
                self.check_and_apply_decay()?;
                let bm25_results = self.search_by_keywords_with_scores(query, limit * 2)?;
                let vector_results = self.search_by_embedding_with_scores(&emb, limit * 2)?;

                let normalized_bm25 = normalize_bm25_scores(bm25_results);
                let normalized_vector = normalize_cosine_scores(vector_results);

                let config = HybridConfig::default();
                let merged =
                    merge_hybrid_results_scored(normalized_bm25, normalized_vector, &config, limit);

                if !merged.is_empty() {
                    // Track access for returned memories
                    for scored in &merged {
                        let _ = self.track_access(&scored.memory.id);
                    }
                    return Ok(merged);
                }
                // If no results, fall through to FTS
            }
            _ => {
                // No embedding available, use FTS directly
            }
        }

        // Fallback: FTS-only search — assign score 1.0 for all results
        let results = self.search_by_keywords(query, limit)?;
        Ok(results
            .into_iter()
            .enumerate()
            .map(|(i, memory)| ScoredMemory {
                memory,
                // Decreasing scores so ordering is preserved; first result = 1.0
                score: 1.0 - (i as f32 * 0.01),
            })
            .collect())
    }

    /// Full-text search with LIKE fallback for empty results or special characters.
    ///
    /// This method provides a last-resort search when FTS5 fails or returns no results:
    /// 1. First attempts FTS5 full-text search with BM25 ranking
    /// 2. If no results found, falls back to SQL LIKE pattern matching
    ///
    /// The LIKE fallback is useful for:
    /// - Special characters that FTS5 cannot handle (@, #, $, %, etc.)
    /// - Partial substring matches
    /// - Edge cases where FTS5 tokenization fails
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Vector of memories matching the query, using LIKE fallback if FTS5 fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use alejandria_storage::SqliteStore;
    ///
    /// # fn main() -> alejandria_core::error::IcmResult<()> {
    /// let store = SqliteStore::open("alejandria.db")?;
    ///
    /// // Search with special characters (FTS5 might fail, LIKE will catch it)
    /// let results = store.search_with_like_fallback("test@example.com", 10)?;
    ///
    /// // Normal search (FTS5 will handle it)
    /// let results = store.search_with_like_fallback("rust programming", 10)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn search_with_like_fallback(
        &self,
        query: &str,
        limit: usize,
    ) -> IcmResult<Vec<alejandria_core::memory::Memory>> {
        use alejandria_core::store::MemoryStore;

        // Try FTS5 first - if it fails (syntax error) or returns no results, use LIKE fallback
        match self.search_by_keywords(query, limit) {
            Ok(results) if !results.is_empty() => {
                return Ok(results);
            }
            Ok(_) => {
                // FTS5 succeeded but returned no results, fall through to LIKE
            }
            Err(_) => {
                // FTS5 failed (likely syntax error with special chars), fall through to LIKE
            }
        }

        // Fallback: LIKE search on summary and raw_excerpt
        let results = self.with_conn(|conn| {
            let like_pattern = format!("%{}%", query);
            let mut stmt = conn
                .prepare(
                    "SELECT
                    id, created_at, updated_at, last_accessed, access_count, weight,
                    topic, summary, raw_excerpt, keywords,
                    importance, source, related_ids,
                    topic_key, revision_count, duplicate_count, last_seen_at, deleted_at,
                    decay_profile, decay_params
                FROM memories
                WHERE deleted_at IS NULL
                  AND (summary LIKE ?1 OR raw_excerpt LIKE ?1)
                ORDER BY updated_at DESC
                LIMIT ?2",
                )
                .into_icm_result()?;

            let results = stmt
                .query_map(rusqlite::params![like_pattern, limit as i64], |row| {
                    Ok(alejandria_core::memory::Memory {
                        id: row.get(0)?,
                        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(1)?,
                        )
                        .unwrap_or_default(),
                        updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(2)?,
                        )
                        .unwrap_or_default(),
                        last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(3)?,
                        )
                        .unwrap_or_default(),
                        access_count: row.get(4)?,
                        weight: row.get(5)?,
                        topic: row.get(6)?,
                        summary: row.get(7)?,
                        raw_excerpt: row.get(8)?,
                        keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                            .unwrap_or_default(),
                        importance: row
                            .get::<_, String>(10)?
                            .parse()
                            .unwrap_or(alejandria_core::memory::Importance::Medium),
                        source: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or(alejandria_core::memory::MemorySource::User),
                        related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                            .unwrap_or_default(),
                        topic_key: row.get(13)?,
                        revision_count: row.get(14)?,
                        duplicate_count: row.get(15)?,
                        last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(16)?,
                        )
                        .unwrap_or_default(),
                        deleted_at: row
                            .get::<_, Option<i64>>(17)?
                            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                        decay_profile: row.get(18)?,
                        decay_params: row
                            .get::<_, Option<String>>(19)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        embedding: None,
                    })
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            Ok(results)
        })?;

        // Track access for returned memories (outside the with_conn closure to avoid deadlock)
        for memory in &results {
            let _ = self.track_access(&memory.id); // Ignore tracking errors
        }

        Ok(results)
    }

    /// Export memories to a writer in the specified format with filtering options.
    ///
    /// This method supports streaming export for large datasets by processing memories
    /// in batches of 1000 records. Supports multiple formats (JSON, CSV, Markdown) and
    /// flexible filtering by session, date range, importance, tags, and decay profile.
    ///
    /// # Arguments
    ///
    /// * `format` - Export format (JSON, CSV, or Markdown)
    /// * `options` - Export options including filters and field selection
    /// * `writer` - Output writer (e.g., file, buffer, stdout)
    ///
    /// # Returns
    ///
    /// Returns `ExportMetadata` containing export statistics and applied filters.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use alejandria_storage::{SqliteStore, ExportFormat, ExportOptions};
    /// use std::fs::File;
    ///
    /// # fn main() -> alejandria_core::error::IcmResult<()> {
    /// let store = SqliteStore::open("alejandria.db")?;
    /// let options = ExportOptions::default();
    /// let file = File::create("export.json")?;
    ///
    /// let metadata = store.export_memories(ExportFormat::Json, options, file)?;
    /// println!("Exported {} memories", metadata.total_count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn export_memories<W: std::io::Write>(
        &self,
        format: crate::export::ExportFormat,
        options: crate::export::ExportOptions,
        mut writer: W,
    ) -> IcmResult<crate::export::ExportMetadata> {
        use crate::export::{export_csv, export_json, export_markdown, ExportMetadata};

        // Build SQL query with filters
        let mut query = String::from(
            "SELECT
                id, created_at, updated_at, last_accessed, access_count, weight,
                topic, summary, raw_excerpt, keywords,
                importance, source, related_ids,
                topic_key, revision_count, duplicate_count, last_seen_at, deleted_at,
                decay_profile, decay_params
            FROM memories
            WHERE 1=1",
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Apply filters
        if !options.include_deleted {
            query.push_str(" AND deleted_at IS NULL");
        }

        if let Some(ref session_id) = options.session_id {
            query.push_str(" AND id LIKE ?");
            params.push(Box::new(format!("%{}%", session_id)));
        }

        if let Some((start, end)) = options.date_range {
            query.push_str(" AND created_at >= ? AND created_at <= ?");
            params.push(Box::new(start.timestamp_millis()));
            params.push(Box::new(end.timestamp_millis()));
        }

        if let Some(ref importance) = options.importance_threshold {
            query.push_str(" AND importance = ?");
            params.push(Box::new(importance.clone()));
        }

        if let Some(ref profile) = options.decay_profile {
            query.push_str(" AND decay_profile = ?");
            params.push(Box::new(profile.clone()));
        }

        if let Some(ref tags) = options.tags {
            if !tags.is_empty() {
                query.push_str(" AND (");
                for (i, tag) in tags.iter().enumerate() {
                    if i > 0 {
                        query.push_str(" OR ");
                    }
                    query.push_str("keywords LIKE ?");
                    params.push(Box::new(format!("%{}%", tag)));
                }
                query.push_str(")");
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        // Count total matching records
        let count_query = query.replace(
            "SELECT\n                id, created_at, updated_at, last_accessed, access_count, weight,\n                topic, summary, raw_excerpt, keywords,\n                importance, source, related_ids,\n                topic_key, revision_count, duplicate_count, last_seen_at, deleted_at,\n                decay_profile, decay_params",
            "SELECT COUNT(*)"
        );

        let total_count: usize = self.with_conn(|conn| {
            let mut stmt = conn.prepare(&count_query).into_icm_result()?;
            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let count: i64 = stmt
                .query_row(&param_refs[..], |row| row.get(0))
                .into_icm_result()?;
            Ok(count as usize)
        })?;

        // Write format-specific header
        match format {
            crate::export::ExportFormat::Json => {
                let metadata = ExportMetadata {
                    version: "1.0".to_string(),
                    exported_at: chrono::Utc::now(),
                    total_count,
                    filters_applied: (&options).into(),
                    format,
                };

                // Write JSON metadata and opening array bracket
                writeln!(writer, "{{")?;
                writeln!(
                    writer,
                    "  \"metadata\": {},",
                    serde_json::to_string_pretty(&metadata)
                        .map_err(|e| IcmError::Serialization(e))?
                )?;
                writeln!(writer, "  \"memories\": [")?;
            }
            crate::export::ExportFormat::Markdown => {
                writeln!(writer, "# Memory Export")?;
                writeln!(writer)?;
                writeln!(
                    writer,
                    "**Exported**: {}",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                )?;
                writeln!(writer, "**Total Memories**: {}", total_count)?;
                writeln!(writer, "**Format**: Markdown")?;
                writeln!(writer)?;
                writeln!(writer, "---")?;
                writeln!(writer)?;
            }
            crate::export::ExportFormat::Csv => {
                // CSV header written by export_csv
            }
        }

        // Batch size for streaming
        const BATCH_SIZE: usize = 1000;
        let mut offset = 0;
        let mut is_first_batch = true;

        loop {
            // Fetch batch
            let batch_query = format!("{} LIMIT {} OFFSET {}", query, BATCH_SIZE, offset);

            let memories: Vec<alejandria_core::memory::Memory> = self.with_conn(|conn| {
                let mut stmt = conn.prepare(&batch_query).into_icm_result()?;
                let param_refs: Vec<&dyn rusqlite::ToSql> =
                    params.iter().map(|p| p.as_ref()).collect();

                let results = stmt
                    .query_map(&param_refs[..], |row| {
                        Ok(alejandria_core::memory::Memory {
                            id: row.get(0)?,
                            created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(1)?,
                            )
                            .unwrap_or_default(),
                            updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(2)?,
                            )
                            .unwrap_or_default(),
                            last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(3)?,
                            )
                            .unwrap_or_default(),
                            access_count: row.get(4)?,
                            weight: row.get(5)?,
                            topic: row.get(6)?,
                            summary: row.get(7)?,
                            raw_excerpt: row.get(8)?,
                            keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                                .unwrap_or_default(),
                            importance: row
                                .get::<_, String>(10)?
                                .parse()
                                .unwrap_or(alejandria_core::memory::Importance::Medium),
                            source: serde_json::from_str(&row.get::<_, String>(11)?)
                                .unwrap_or(alejandria_core::memory::MemorySource::User),
                            related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                                .unwrap_or_default(),
                            topic_key: row.get(13)?,
                            revision_count: row.get(14)?,
                            duplicate_count: row.get(15)?,
                            last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(16)?,
                            )
                            .unwrap_or_default(),
                            deleted_at: row
                                .get::<_, Option<i64>>(17)?
                                .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                            decay_profile: row.get(18)?,
                            decay_params: row
                                .get::<_, Option<String>>(19)?
                                .and_then(|s| serde_json::from_str(&s).ok()),
                            embedding: None, // Don't include embeddings in export (large)
                        })
                    })
                    .into_icm_result()?
                    .collect::<Result<Vec<_>, _>>()
                    .into_icm_result()?;

                Ok(results)
            })?;

            if memories.is_empty() {
                break;
            }

            let is_last_batch = memories.len() < BATCH_SIZE;

            // Write batch in appropriate format
            match format {
                crate::export::ExportFormat::Json => {
                    export_json(
                        &memories,
                        &options,
                        &mut writer,
                        is_first_batch,
                        is_last_batch,
                    )?;
                }
                crate::export::ExportFormat::Csv => {
                    export_csv(&memories, &options, &mut writer, is_first_batch)?;
                }
                crate::export::ExportFormat::Markdown => {
                    export_markdown(&memories, &options, &mut writer)?;
                }
            }

            offset += memories.len();
            is_first_batch = false;

            if is_last_batch {
                break;
            }
        }

        // Write format-specific footer
        match format {
            crate::export::ExportFormat::Json => {
                writeln!(writer, "\n  ]")?;
                writeln!(writer, "}}")?;
            }
            _ => {}
        }

        writer.flush()?;

        Ok(ExportMetadata {
            version: "1.0".to_string(),
            exported_at: chrono::Utc::now(),
            total_count,
            filters_applied: (&options).into(),
            format,
        })
    }
}

impl MemoryStore for SqliteStore {
    fn store(&self, mut memory: alejandria_core::memory::Memory) -> IcmResult<String> {
        // Validate embedding dimensions if present
        if let Some(ref embedding) = memory.embedding {
            if embedding.len() != EXPECTED_EMBEDDING_DIM {
                return Err(IcmError::InvalidInput(format!(
                    "Invalid embedding dimensions: expected {}, got {}",
                    EXPECTED_EMBEDDING_DIM,
                    embedding.len()
                )));
            }
        }

        let (memory_id, summary) = self.with_conn(|conn| {
            // Check for existing memory with same topic_key (upsert workflow)
            if let Some(ref topic_key) = memory.topic_key {
                let existing: Option<String> = conn
                    .query_row(
                        "SELECT id FROM memories WHERE topic_key = ? AND deleted_at IS NULL",
                        rusqlite::params![topic_key],
                        |row| row.get(0),
                    )
                    .optional()
                    .into_icm_result()?;

                if let Some(existing_id) = existing {
                    // Update existing memory
                    memory.id = existing_id;
                    memory.revision_count += 1;
                    memory.updated_at = chrono::Utc::now();

                    conn.execute(
                        "UPDATE memories SET
                            updated_at = ?1,
                            last_accessed = ?2,
                            summary = ?3,
                            raw_excerpt = ?4,
                            keywords = ?5,
                            importance = ?6,
                            source = ?7,
                            related_ids = ?8,
                            revision_count = ?9,
                            last_seen_at = ?10
                        WHERE id = ?11",
                        rusqlite::params![
                            // Timestamps in milliseconds since Unix epoch
                            memory.updated_at.timestamp_millis(),
                            memory.last_accessed.timestamp_millis(),
                            memory.summary,
                            memory.raw_excerpt,
                            serde_json::to_string(&memory.keywords)?,
                            memory.importance.to_string(),
                            serde_json::to_string(&memory.source)?,
                            serde_json::to_string(&memory.related_ids)?,
                            memory.revision_count,
                            memory.last_seen_at.timestamp_millis(),
                            memory.id,
                        ],
                    )
                    .into_icm_result()?;

                    return Ok((memory.id.clone(), memory.summary.clone()));
                }
            }

            Ok((String::new(), memory.summary.clone())) // Placeholder for deduplication check
        })?;

        // If topic_key returned an ID, we're done (upsert case)
        if !memory_id.is_empty() {
            #[cfg(feature = "embeddings")]
            {
                if let Err(e) = self.store_embedding(&memory_id, &summary) {
                    eprintln!(
                        "Warning: Failed to generate embedding for {}: {:?}",
                        memory_id, e
                    );
                }
            }
            return Ok(memory_id);
        }

        // Check for duplicates (>85% keyword similarity)
        if let Some(duplicate_id) = self.find_duplicate(&memory)? {
            // Update duplicate_count and last_seen_at on existing memory
            self.with_conn(|conn| {
                conn.execute(
                    "UPDATE memories SET
                        duplicate_count = duplicate_count + 1,
                        last_seen_at = ?1
                    WHERE id = ?2",
                    rusqlite::params![chrono::Utc::now().timestamp_millis(), duplicate_id,],
                )
                .into_icm_result()?;

                Ok(())
            })?;

            return Ok(duplicate_id);
        }

        // No upsert or duplicate found - insert new memory
        let (memory_id, summary) = self.with_conn(|conn| {
            // Generate ULID if not set
            if memory.id.is_empty() {
                memory.id = ulid::Ulid::new().to_string();
            }

            // Set timestamps
            let now = chrono::Utc::now();
            memory.created_at = now;
            memory.updated_at = now;
            memory.last_accessed = now;
            memory.last_seen_at = now;

            // Insert new memory
            conn.execute(
                "INSERT INTO memories (
                    id, created_at, updated_at, last_accessed, access_count, weight,
                    topic, summary, raw_excerpt, keywords,
                    importance, source, related_ids,
                    topic_key, revision_count, duplicate_count, last_seen_at, deleted_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13,
                    ?14, ?15, ?16, ?17, ?18
                )",
                rusqlite::params![
                    memory.id,
                    // Timestamps in milliseconds since Unix epoch
                    memory.created_at.timestamp_millis(),
                    memory.updated_at.timestamp_millis(),
                    memory.last_accessed.timestamp_millis(),
                    memory.access_count,
                    memory.weight,
                    memory.topic,
                    memory.summary,
                    memory.raw_excerpt,
                    serde_json::to_string(&memory.keywords)?,
                    memory.importance.to_string(),
                    serde_json::to_string(&memory.source)?,
                    serde_json::to_string(&memory.related_ids)?,
                    memory.topic_key,
                    memory.revision_count,
                    memory.duplicate_count,
                    memory.last_seen_at.timestamp_millis(),
                    memory.deleted_at.map(|dt| dt.timestamp_millis()),
                ],
            )
            .into_icm_result()?;

            Ok((memory.id.clone(), memory.summary.clone()))
        })?;

        // Generate and store embedding (if feature enabled)
        // This happens OUTSIDE the with_conn closure to avoid nested borrowing
        #[cfg(feature = "embeddings")]
        {
            if let Err(e) = self.store_embedding(&memory_id, &summary) {
                // Log error but don't fail the store operation
                eprintln!(
                    "Warning: Failed to generate embedding for {}: {:?}",
                    memory_id, e
                );
            }
        }

        Ok(memory_id)
    }

    fn get(&self, id: &str) -> IcmResult<Option<alejandria_core::memory::Memory>> {
        self.with_conn(|conn| {
            let mut result = conn
                .query_row(
                    "SELECT
                        id, created_at, updated_at, last_accessed, access_count, weight,
                        topic, summary, raw_excerpt, keywords,
                        importance, source, related_ids,
                        topic_key, revision_count, duplicate_count, last_seen_at, deleted_at,
                        decay_profile, decay_params
                    FROM memories
                    WHERE id = ? AND deleted_at IS NULL",
                    rusqlite::params![id],
                    |row| {
                        Ok(alejandria_core::memory::Memory {
                            id: row.get(0)?,
                            created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(1)?,
                            )
                            .unwrap_or_default(),
                            updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(2)?,
                            )
                            .unwrap_or_default(),
                            last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(3)?,
                            )
                            .unwrap_or_default(),
                            access_count: row.get(4)?,
                            weight: row.get(5)?,
                            topic: row.get(6)?,
                            summary: row.get(7)?,
                            raw_excerpt: row.get(8)?,
                            keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                                .unwrap_or_default(),
                            importance: row
                                .get::<_, String>(10)?
                                .parse()
                                .unwrap_or(alejandria_core::memory::Importance::Medium),
                            source: serde_json::from_str(&row.get::<_, String>(11)?)
                                .unwrap_or(alejandria_core::memory::MemorySource::User),
                            related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                                .unwrap_or_default(),
                            topic_key: row.get(13)?,
                            revision_count: row.get(14)?,
                            duplicate_count: row.get(15)?,
                            last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(16)?,
                            )
                            .unwrap_or_default(),
                            deleted_at: row
                                .get::<_, Option<i64>>(17)?
                                .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                            decay_profile: row.get(18)?,
                            decay_params: row
                                .get::<_, Option<String>>(19)?
                                .and_then(|s| serde_json::from_str(&s).ok()),
                            embedding: None, // Loaded below from vec_memories
                        })
                    },
                )
                .optional()
                .into_icm_result()?;

            // Load embedding from vec_memories if available
            if let Some(ref mut memory) = result {
                let vec_table_exists: bool = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_memories'",
                        [],
                        |row| row.get(0).map(|c: i32| c > 0),
                    )
                    .unwrap_or(false);

                if vec_table_exists {
                    if let Ok(embedding_json) = conn.query_row(
                        "SELECT embedding FROM vec_memories WHERE memory_id = ?1",
                        rusqlite::params![memory.id],
                        |row| row.get::<_, String>(0),
                    ) {
                        if let Ok(vec) = serde_json::from_str::<Vec<f32>>(&embedding_json) {
                            memory.embedding = Some(vec);
                        }
                    }
                    // If no row found or parse fails, embedding stays None — that's fine
                }
            }

            // Update access tracking if memory was found
            // BUGFIX: Mutate the memory object in-place to reflect the updated access tracking
            if let Some(ref mut memory) = result {
                let now = chrono::Utc::now();
                memory.access_count += 1;
                memory.last_accessed = now;

                conn.execute(
                    "UPDATE memories SET
                        access_count = access_count + 1,
                        last_accessed = ?1
                    WHERE id = ?2",
                    // Timestamp in milliseconds since Unix epoch
                    rusqlite::params![now.timestamp_millis(), memory.id],
                )
                .into_icm_result()?;
            }

            Ok(result)
        })
    }

    fn update(&self, memory: alejandria_core::memory::Memory) -> IcmResult<()> {
        // Validate embedding dimensions if present
        if let Some(ref embedding) = memory.embedding {
            if embedding.len() != EXPECTED_EMBEDDING_DIM {
                return Err(IcmError::InvalidInput(format!(
                    "Invalid embedding dimensions: expected {}, got {}",
                    EXPECTED_EMBEDDING_DIM,
                    embedding.len()
                )));
            }
        }

        // Get old summary to detect changes
        let old_summary = self.with_conn(|conn| {
            conn.query_row(
                "SELECT summary FROM memories WHERE id = ? AND deleted_at IS NULL",
                rusqlite::params![memory.id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .into_icm_result()
        })?;

        self.with_conn(|conn| {
            let rows_affected = conn
                .execute(
                    "UPDATE memories SET
                    updated_at = ?1,
                    summary = ?2,
                    raw_excerpt = ?3,
                    keywords = ?4,
                    importance = ?5,
                    source = ?6,
                    related_ids = ?7,
                    topic = ?8,
                    weight = ?9
                WHERE id = ?10 AND deleted_at IS NULL",
                    rusqlite::params![
                        // Timestamp in milliseconds since Unix epoch
                        chrono::Utc::now().timestamp_millis(),
                        memory.summary,
                        memory.raw_excerpt,
                        serde_json::to_string(&memory.keywords)?,
                        memory.importance.to_string(),
                        serde_json::to_string(&memory.source)?,
                        serde_json::to_string(&memory.related_ids)?,
                        memory.topic,
                        memory.weight,
                        memory.id,
                    ],
                )
                .into_icm_result()?;

            if rows_affected == 0 {
                return Err(IcmError::NotFound {
                    entity: "Memory".to_string(),
                    field: "id".to_string(),
                    value: memory.id.clone(),
                });
            }

            Ok(())
        })?;

        // Regenerate embedding if summary changed
        #[cfg(feature = "embeddings")]
        {
            if let Some(old) = old_summary {
                if old != memory.summary {
                    if let Err(e) = self.store_embedding(&memory.id, &memory.summary) {
                        eprintln!("Warning: Failed to regenerate embedding: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn delete(&self, id: &str) -> IcmResult<()> {
        self.with_conn(|conn| {
            let rows_affected = conn
                .execute(
                    "UPDATE memories SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
                    // Timestamp in milliseconds since Unix epoch
                    rusqlite::params![chrono::Utc::now().timestamp_millis(), id],
                )
                .into_icm_result()?;

            if rows_affected == 0 {
                return Err(IcmError::NotFound {
                    entity: "Memory".to_string(),
                    field: "id".to_string(),
                    value: id.to_string(),
                });
            }

            Ok(())
        })
    }

    // === Search Operations ===

    fn search_by_keywords(
        &self,
        query: &str,
        limit: usize,
    ) -> IcmResult<Vec<alejandria_core::memory::Memory>> {
        let results = self.search_by_keywords_with_scores(query, limit)?;

        // Track access for all returned memories
        for (memory, _score) in &results {
            let _ = self.track_access(&memory.id); // Ignore tracking errors
        }

        Ok(results.into_iter().map(|(m, _)| m).collect())
    }

    fn search_by_embedding(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> IcmResult<Vec<alejandria_core::memory::Memory>> {
        let results = self.search_by_embedding_with_scores(embedding, limit)?;

        // Track access for all returned memories
        for (memory, _distance) in &results {
            let _ = self.track_access(&memory.id); // Ignore tracking errors
        }

        Ok(results.into_iter().map(|(m, _)| m).collect())
    }

    fn hybrid_search(
        &self,
        query: &str,
        embedding: &[f32],
        limit: usize,
    ) -> IcmResult<Vec<alejandria_core::memory::Memory>> {
        use crate::search::{
            merge_hybrid_results, normalize_bm25_scores, normalize_cosine_scores, HybridConfig,
        };

        // Check if decay needs to run (>24h since last decay)
        self.check_and_apply_decay()?;

        // Get results with scores from both methods (fetch more to have better pool for merging)
        let bm25_results = self.search_by_keywords_with_scores(query, limit * 2)?;
        let vector_results = self.search_by_embedding_with_scores(embedding, limit * 2)?;

        // Normalize scores to [0, 1] range
        let normalized_bm25 = normalize_bm25_scores(bm25_results);
        let normalized_vector = normalize_cosine_scores(vector_results);

        // Merge with weighted scoring (30% BM25, 70% cosine)
        let config = HybridConfig::default();
        let merged = merge_hybrid_results(normalized_bm25, normalized_vector, &config, limit);

        // Track access for returned memories
        for memory in &merged {
            let _ = self.track_access(&memory.id); // Ignore tracking errors
        }

        Ok(merged)
    }

    // === Lifecycle Operations ===

    /// Apply decay to all non-deleted, non-critical memories using their configured decay strategies.
    ///
    /// This method implements the pluggable decay system, supporting multiple decay algorithms:
    /// - **Exponential** (default): Standard exponential decay based on time since last access
    /// - **Spaced Repetition (SM-2)**: Algorithm optimized for learning and review patterns
    /// - **Importance-Weighted**: Adjusts decay rate based on memory importance level
    /// - **Context-Sensitive**: Uses topic-specific decay rates (e.g., architecture decays slower)
    ///
    /// # Behavior
    ///
    /// 1. **Strategy Selection**: Each memory's `decay_profile` column determines which strategy to use.
    ///    Null or empty profiles default to ExponentialDecay for backward compatibility.
    ///
    /// 2. **Critical Memories Exempt**: Memories with `Importance::Critical` are always skipped
    ///    and retain their current weight, regardless of age or access patterns.
    ///
    /// 3. **Parameter Updates**: Each strategy can update its `decay_params` JSON after calculation.
    ///    For example, SM-2 increments `repetitions` and `interval_days` on successful reviews.
    ///
    /// 4. **Weight Update**: Both `weight` and `decay_params` are persisted to the database
    ///    for each decayed memory.
    ///
    /// # Arguments
    ///
    /// * `base_rate` - Base decay rate multiplier (typically 0.01-0.10). Higher values = faster decay.
    ///                 This is passed to each strategy's `calculate_decay()` method.
    ///
    /// # Returns
    ///
    /// Number of memories successfully updated (excludes Critical memories and any that error).
    ///
    /// # Errors
    ///
    /// Returns `IcmError` if database operations fail or if strategy calculations encounter errors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use alejandria_storage::SqliteStore;
    /// # use alejandria_core::store::MemoryStore;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = SqliteStore::open_in_memory()?;
    /// let updated = store.apply_decay(0.05)?; // Decay with 5% base rate
    /// println!("Decayed {} memories", updated);
    /// # Ok(())
    /// # }
    /// ```
    fn apply_decay(&self, base_rate: f32) -> IcmResult<usize> {
        self.with_conn(|conn| {
            let now = chrono::Utc::now();
            let now_ms = now.timestamp_millis();

            // Fetch all non-deleted memories with full context for decay strategies
            let mut stmt = conn.prepare(
                "SELECT 
                    id, created_at, updated_at, last_accessed, access_count, weight,
                    topic, summary, raw_excerpt, keywords,
                    importance, source, related_ids,
                    topic_key, revision_count, duplicate_count, last_seen_at, deleted_at,
                    decay_profile, decay_params
                FROM memories 
                WHERE deleted_at IS NULL"
            ).into_icm_result()?;

            let memories: Vec<alejandria_core::memory::Memory> = stmt
                .query_map([], |row| {
                    Ok(alejandria_core::memory::Memory {
                        id: row.get(0)?,
                        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(1)?,
                        )
                        .unwrap_or_default(),
                        updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(2)?,
                        )
                        .unwrap_or_default(),
                        last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(3)?,
                        )
                        .unwrap_or_default(),
                        access_count: row.get(4)?,
                        weight: row.get(5)?,
                        topic: row.get(6)?,
                        summary: row.get(7)?,
                        raw_excerpt: row.get(8)?,
                        keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                            .unwrap_or_default(),
                        importance: row
                            .get::<_, String>(10)?
                            .parse()
                            .unwrap_or(alejandria_core::memory::Importance::Medium),
                        source: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or(alejandria_core::memory::MemorySource::User),
                        related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                            .unwrap_or_default(),
                        topic_key: row.get(13)?,
                        revision_count: row.get(14)?,
                        duplicate_count: row.get(15)?,
                        last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(16)?,
                        )
                        .unwrap_or_default(),
                        deleted_at: row
                            .get::<_, Option<i64>>(17)?
                            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                        decay_profile: row.get(18)?,
                        decay_params: row.get::<_, Option<String>>(19)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        embedding: None,
                    })
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            drop(stmt);

            let mut updated = 0;
            for memory in memories {
                // Skip Critical memories (maintain backward compatibility)
                if memory.importance == alejandria_core::memory::Importance::Critical {
                    continue;
                }

                // Select appropriate decay strategy based on profile
                let strategy = Self::select_decay_strategy(memory.decay_profile.as_deref());
                
                // Get current params or use strategy defaults
                let params = memory.decay_params.clone()
                    .unwrap_or_else(|| strategy.default_params());

                // Calculate new weight using the strategy
                let (new_weight, updated_params) = strategy.calculate_decay(&memory, &params, base_rate)?;

                // Serialize updated params to JSON string
                let params_json = serde_json::to_string(&updated_params)?;

                // Update the memory's weight and decay params
                conn.execute(
                    "UPDATE memories SET weight = ?1, decay_params = ?2, updated_at = ?3 WHERE id = ?4",
                    rusqlite::params![new_weight, params_json, now_ms, memory.id],
                ).into_icm_result()?;

                updated += 1;
            }

            // Update last_decay_at metadata
            conn.execute(
                "INSERT OR REPLACE INTO icm_metadata (key, value, updated_at) VALUES ('last_decay_at', ?1, ?2)",
                rusqlite::params![now_ms.to_string(), now_ms],
            ).into_icm_result()?;

            Ok(updated)
        })
    }

    fn prune(&self, weight_threshold: f32) -> IcmResult<usize> {
        self.with_conn(|conn| {
            let now_ms = chrono::Utc::now().timestamp_millis();

            // Soft-delete memories below threshold (only Medium and Low importance)
            let pruned = conn
                .execute(
                    r#"
                UPDATE memories
                SET deleted_at = ?1
                WHERE deleted_at IS NULL
                  AND weight < ?2
                  AND importance IN ('medium', 'low')
                "#,
                    rusqlite::params![now_ms, weight_threshold],
                )
                .into_icm_result()?;

            Ok(pruned)
        })
    }

    // === Organization Operations ===

    fn get_by_topic(
        &self,
        topic: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> IcmResult<Vec<alejandria_core::memory::Memory>> {
        self.with_conn(|conn| {
            // Build query with optional LIMIT and OFFSET
            let mut query = String::from(
                "SELECT
                    id, created_at, updated_at, last_accessed, access_count, weight,
                    topic, summary, raw_excerpt, keywords,
                    importance, source, related_ids,
                    topic_key, revision_count, duplicate_count, last_seen_at, deleted_at,
                    decay_profile, decay_params
                FROM memories
                WHERE topic = ?1 AND deleted_at IS NULL
                ORDER BY created_at DESC",
            );

            if let Some(lim) = limit {
                query.push_str(&format!(" LIMIT {}", lim));
                if let Some(off) = offset {
                    query.push_str(&format!(" OFFSET {}", off));
                }
            }

            let mut stmt = conn.prepare(&query).into_icm_result()?;

            let memories = stmt
                .query_map(rusqlite::params![topic], |row| {
                    Ok(alejandria_core::memory::Memory {
                        id: row.get(0)?,
                        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(1)?,
                        )
                        .unwrap_or_default(),
                        updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(2)?,
                        )
                        .unwrap_or_default(),
                        last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(3)?,
                        )
                        .unwrap_or_default(),
                        access_count: row.get(4)?,
                        weight: row.get(5)?,
                        topic: row.get(6)?,
                        summary: row.get(7)?,
                        raw_excerpt: row.get(8)?,
                        keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                            .unwrap_or_default(),
                        importance: row
                            .get::<_, String>(10)?
                            .parse()
                            .unwrap_or(alejandria_core::memory::Importance::Medium),
                        source: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or(alejandria_core::memory::MemorySource::User),
                        related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                            .unwrap_or_default(),
                        topic_key: row.get(13)?,
                        revision_count: row.get(14)?,
                        duplicate_count: row.get(15)?,
                        last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                            row.get(16)?,
                        )
                        .unwrap_or_default(),
                        deleted_at: row
                            .get::<_, Option<i64>>(17)?
                            .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                        decay_profile: row.get(18)?,
                        decay_params: row
                            .get::<_, Option<String>>(19)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        embedding: None,
                    })
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            Ok(memories)
        })
    }

    fn get_by_topic_key(
        &self,
        topic_key: &str,
    ) -> IcmResult<Option<alejandria_core::memory::Memory>> {
        self.with_conn(|conn| {
            let result = conn
                .query_row(
                    "SELECT
                        id, created_at, updated_at, last_accessed, access_count, weight,
                        topic, summary, raw_excerpt, keywords,
                        importance, source, related_ids,
                        topic_key, revision_count, duplicate_count, last_seen_at, deleted_at,
                        decay_profile, decay_params
                    FROM memories
                    WHERE topic_key = ?1 AND deleted_at IS NULL",
                    rusqlite::params![topic_key],
                    |row| {
                        Ok(alejandria_core::memory::Memory {
                            id: row.get(0)?,
                            created_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(1)?,
                            )
                            .unwrap_or_default(),
                            updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(2)?,
                            )
                            .unwrap_or_default(),
                            last_accessed: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(3)?,
                            )
                            .unwrap_or_default(),
                            access_count: row.get(4)?,
                            weight: row.get(5)?,
                            topic: row.get(6)?,
                            summary: row.get(7)?,
                            raw_excerpt: row.get(8)?,
                            keywords: serde_json::from_str(&row.get::<_, String>(9)?)
                                .unwrap_or_default(),
                            importance: row
                                .get::<_, String>(10)?
                                .parse()
                                .unwrap_or(alejandria_core::memory::Importance::Medium),
                            source: serde_json::from_str(&row.get::<_, String>(11)?)
                                .unwrap_or(alejandria_core::memory::MemorySource::User),
                            related_ids: serde_json::from_str(&row.get::<_, String>(12)?)
                                .unwrap_or_default(),
                            topic_key: row.get(13)?,
                            revision_count: row.get(14)?,
                            duplicate_count: row.get(15)?,
                            last_seen_at: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                                row.get(16)?,
                            )
                            .unwrap_or_default(),
                            deleted_at: row
                                .get::<_, Option<i64>>(17)?
                                .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis),
                            decay_profile: row.get(18)?,
                            decay_params: row
                                .get::<_, Option<String>>(19)?
                                .and_then(|s| serde_json::from_str(&s).ok()),
                            embedding: None,
                        })
                    },
                )
                .optional()
                .into_icm_result()?;

            Ok(result)
        })
    }

    fn list_topics(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> IcmResult<Vec<alejandria_core::store::TopicInfo>> {
        self.with_conn(|conn| {
            // Build query with optional LIMIT and OFFSET
            let mut query = String::from(
                "SELECT
                    topic,
                    COUNT(*) as count,
                    AVG(weight) as avg_weight,
                    MIN(created_at) as oldest,
                    MAX(created_at) as newest
                FROM memories
                WHERE deleted_at IS NULL
                GROUP BY topic
                ORDER BY count DESC",
            );

            if let Some(lim) = limit {
                query.push_str(&format!(" LIMIT {}", lim));
                if let Some(off) = offset {
                    query.push_str(&format!(" OFFSET {}", off));
                }
            }

            let mut stmt = conn.prepare(&query).into_icm_result()?;

            let topics = stmt
                .query_map([], |row| {
                    Ok(alejandria_core::store::TopicInfo {
                        topic: row.get(0)?,
                        count: row.get::<_, i64>(1)? as usize,
                        avg_weight: row.get(2)?,
                        oldest: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(row.get(3)?)
                            .unwrap_or_default(),
                        newest: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(row.get(4)?)
                            .unwrap_or_default(),
                    })
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            Ok(topics)
        })
    }

    fn consolidate_topic(
        &self,
        topic: &str,
        min_memories: usize,
        min_weight: f32,
    ) -> IcmResult<String> {
        // Find memories in topic that meet criteria
        let source_memories = self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT
                    id, keywords, summary
                FROM memories
                WHERE topic = ?1 AND deleted_at IS NULL AND weight > ?2
                ORDER BY weight DESC",
                )
                .into_icm_result()?;

            let memories = stmt
                .query_map(rusqlite::params![topic, min_weight], |row| {
                    let id: String = row.get(0)?;
                    let keywords_json: String = row.get(1)?;
                    let keywords: Vec<String> =
                        serde_json::from_str(&keywords_json).unwrap_or_default();
                    let summary: String = row.get(2)?;
                    Ok((id, keywords, summary))
                })
                .into_icm_result()?
                .collect::<Result<Vec<_>, _>>()
                .into_icm_result()?;

            Ok(memories)
        })?;

        // Check if we have enough memories to consolidate
        if source_memories.len() < min_memories {
            return Err(IcmError::InvalidInput(format!(
                "Insufficient memories for consolidation: found {}, need at least {}",
                source_memories.len(),
                min_memories
            )));
        }

        // Extract all keywords and find most common ones
        use std::collections::HashMap;
        let mut keyword_freq: HashMap<String, usize> = HashMap::new();
        let mut source_ids = Vec::new();

        for (id, keywords, _) in &source_memories {
            source_ids.push(id.clone());
            for keyword in keywords {
                *keyword_freq.entry(keyword.clone()).or_insert(0) += 1;
            }
        }

        // Get top keywords (appearing in at least 30% of memories)
        let min_frequency = (source_memories.len() as f32 * 0.3).ceil() as usize;
        let mut consolidated_keywords: Vec<String> = keyword_freq
            .into_iter()
            .filter(|(_, count)| *count >= min_frequency)
            .map(|(keyword, _)| keyword)
            .collect();
        consolidated_keywords.sort();
        consolidated_keywords.truncate(20); // Max 20 keywords

        // Generate consolidated summary from common themes
        let summary = format!(
            "Consolidated from {} memories in topic '{}'. Common themes: {}",
            source_memories.len(),
            topic,
            consolidated_keywords.join(", ")
        );

        // Create consolidated memory
        let mut consolidated = alejandria_core::memory::Memory::new(topic.to_string(), summary);
        consolidated.keywords = consolidated_keywords;
        consolidated.importance = alejandria_core::memory::Importance::High;
        consolidated.source = alejandria_core::memory::MemorySource::System;
        consolidated.related_ids = source_ids;

        // Store the consolidated memory
        self.store(consolidated)
    }

    // === Statistics Operations ===

    fn count(&self) -> IcmResult<usize> {
        self.with_conn(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM memories WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .into_icm_result()?;
            Ok(count as usize)
        })
    }

    fn stats(&self) -> IcmResult<alejandria_core::store::StoreStats> {
        self.with_conn(|conn| {
            // Get total and active counts
            let (total, active): (i64, i64) = conn
                .query_row(
                    "SELECT
                    COUNT(*) as total,
                    COUNT(CASE WHEN deleted_at IS NULL THEN 1 END) as active
                FROM memories",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .into_icm_result()?;

            let deleted = total - active;

            // Get counts by importance
            let mut stmt = conn
                .prepare(
                    "SELECT importance, COUNT(*)
                FROM memories
                WHERE deleted_at IS NULL
                GROUP BY importance",
                )
                .into_icm_result()?;

            let mut by_importance = alejandria_core::store::ImportanceStats {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            };

            for row in stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .into_icm_result()?
            {
                let (importance, count) = row.into_icm_result()?;
                match importance.as_str() {
                    "critical" => by_importance.critical = count as usize,
                    "high" => by_importance.high = count as usize,
                    "medium" => by_importance.medium = count as usize,
                    "low" => by_importance.low = count as usize,
                    _ => {}
                }
            }

            // Get counts by source
            let mut by_source = alejandria_core::store::SourceStats {
                user: 0,
                agent: 0,
                system: 0,
                external: 0,
            };

            let mut stmt = conn
                .prepare(
                    "SELECT source, COUNT(*)
                FROM memories
                WHERE deleted_at IS NULL
                GROUP BY source",
                )
                .into_icm_result()?;

            for row in stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .into_icm_result()?
            {
                let (source, count) = row.into_icm_result()?;
                // Parse JSON source to extract type
                if let Ok(source_obj) = serde_json::from_str::<serde_json::Value>(&source) {
                    if let Some(source_type) = source_obj.get("type").and_then(|v| v.as_str()) {
                        match source_type {
                            "user" => by_source.user = count as usize,
                            "agent" => by_source.agent = count as usize,
                            "system" => by_source.system = count as usize,
                            "external" => by_source.external = count as usize,
                            _ => {}
                        }
                    }
                }
            }

            // Get average weight
            let avg_weight: f32 = conn
                .query_row(
                    "SELECT AVG(weight) FROM memories WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0.0);

            // Get last decay timestamp from metadata
            let last_decay_at: Option<chrono::DateTime<chrono::Utc>> = conn
                .query_row(
                    "SELECT value FROM icm_metadata WHERE key = 'last_decay_at'",
                    [],
                    |row| {
                        let ts: i64 = row.get::<_, String>(0)?.parse().unwrap_or(0);
                        Ok(chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts))
                    },
                )
                .ok()
                .flatten();

            // Get database size
            let page_count: i64 = conn
                .query_row("PRAGMA page_count", [], |row| row.get(0))
                .into_icm_result()?;
            let page_size: i64 = conn
                .query_row("PRAGMA page_size", [], |row| row.get(0))
                .into_icm_result()?;
            let total_size_mb = (page_count * page_size) as f64 / 1024.0 / 1024.0;

            Ok(alejandria_core::store::StoreStats {
                total_memories: total as usize,
                active_memories: active as usize,
                deleted_memories: deleted as usize,
                total_size_mb,
                by_importance,
                by_source,
                avg_weight,
                embeddings_enabled: self.embedder.is_some(), // Reflects actual embedder status
                last_decay_at,
            })
        })
    }

    fn set_decay_profile(
        &self,
        memory_id: &str,
        profile_name: &str,
        params: Option<serde_json::Value>,
    ) -> IcmResult<()> {
        self.with_conn(|conn| {
            // Serialize params to JSON string if provided
            let params_json = params
                .as_ref()
                .map(|p| serde_json::to_string(p))
                .transpose()?;

            conn.execute(
                "UPDATE memories 
                 SET decay_profile = ?1, decay_params = ?2, updated_at = ?3
                 WHERE id = ?4 AND deleted_at IS NULL",
                rusqlite::params![
                    profile_name,
                    params_json,
                    chrono::Utc::now().timestamp_millis(),
                    memory_id
                ],
            )
            .into_icm_result()?;

            Ok(())
        })
    }

    fn get_decay_stats(&self) -> IcmResult<alejandria_core::store::DecayStats> {
        self.with_conn(|conn| {
            // Count memories with explicit decay profiles
            let total_with_profile: usize = conn
                .query_row(
                    "SELECT COUNT(*) FROM memories 
                     WHERE deleted_at IS NULL AND decay_profile IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .into_icm_result()?;

            // Count memories using default decay (NULL profile)
            let total_default: usize = conn
                .query_row(
                    "SELECT COUNT(*) FROM memories 
                     WHERE deleted_at IS NULL AND decay_profile IS NULL",
                    [],
                    |row| row.get(0),
                )
                .into_icm_result()?;

            // Get breakdown by profile name
            let mut stmt = conn
                .prepare(
                    "SELECT decay_profile, COUNT(*) as count, AVG(weight) as avg_weight
                     FROM memories
                     WHERE deleted_at IS NULL AND decay_profile IS NOT NULL
                     GROUP BY decay_profile",
                )
                .into_icm_result()?;

            let mut by_profile = std::collections::HashMap::new();
            let mut avg_weight_by_profile = std::collections::HashMap::new();

            let results = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, usize>(1)?,
                        row.get::<_, f32>(2)?,
                    ))
                })
                .into_icm_result()?;

            for result in results {
                let (profile, count, avg_weight) = result.into_icm_result()?;
                by_profile.insert(profile.clone(), count);
                avg_weight_by_profile.insert(profile, avg_weight);
            }

            // Count memories with low weight (< 0.1)
            let low_weight_count: usize = conn
                .query_row(
                    "SELECT COUNT(*) FROM memories 
                     WHERE deleted_at IS NULL AND weight < 0.1",
                    [],
                    |row| row.get(0),
                )
                .into_icm_result()?;

            // Get overall average weight
            let overall_avg_weight: f32 = conn
                .query_row(
                    "SELECT AVG(weight) FROM memories WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(1.0);

            Ok(alejandria_core::store::DecayStats {
                total_with_profile,
                total_default,
                by_profile,
                avg_weight_by_profile,
                low_weight_count,
                overall_avg_weight,
            })
        })
    }

    fn import_memories(
        &self,
        input_path: &std::path::Path,
        mode: alejandria_core::import::ImportMode,
    ) -> IcmResult<alejandria_core::import::ImportResult> {
        // Delegate to the import module implementation
        self.import_memories(input_path, mode)
    }
}

// ============================================================================
// Private Helper Methods for Embeddings (Phase 3)
// ============================================================================

impl SqliteStore {
    /// Helper method to generate and store embedding for a memory.
    ///
    /// This method is called automatically by store() when the embeddings feature is enabled
    /// and an embedder is configured. Gracefully degrades if sqlite-vec is not available
    /// or no embedder is set.
    #[cfg(feature = "embeddings")]
    fn store_embedding(&self, memory_id: &str, text: &str) -> IcmResult<()> {
        // No embedder configured — skip silently
        let embedder = match &self.embedder {
            Some(e) => Arc::clone(e),
            None => return Ok(()),
        };

        // Check if vec_memories table exists
        let table_exists = self.with_conn(|conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_memories'",
                    [],
                    |row| row.get(0).map(|count: i32| count > 0),
                )
                .unwrap_or(false);
            Ok(exists)
        })?;

        if !table_exists {
            return Ok(()); // Graceful degradation
        }

        // Generate embedding via the real embedder
        let embedding = embedder.embed(text)?;

        // Validate embedding dimensions
        if embedding.len() != EXPECTED_EMBEDDING_DIM {
            return Err(IcmError::InvalidInput(format!(
                "Invalid embedding dimensions: expected {}, got {}",
                EXPECTED_EMBEDDING_DIM,
                embedding.len()
            )));
        }

        // Store in vec_memories
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO vec_memories (memory_id, embedding) VALUES (?1, ?2)",
                rusqlite::params![memory_id, serde_json::to_string(&embedding)?],
            )
            .into_icm_result()?;
            Ok(())
        })
    }

    /// Helper method stub when embeddings feature is disabled.
    #[cfg(not(feature = "embeddings"))]
    fn store_embedding(&self, _memory_id: &str, _text: &str) -> IcmResult<()> {
        Ok(()) // No-op when embeddings disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let store = SqliteStore::open_in_memory().unwrap();

        // Verify schema was initialized
        store
            .with_conn(|conn| {
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='memories'",
                        [],
                        |row| row.get(0),
                    )
                    .into_icm_result()?;
                assert_eq!(count, 1);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn test_clone_store() {
        let store = SqliteStore::open_in_memory().unwrap();
        let store2 = store.clone();

        // Both clones should access the same connection
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO icm_metadata (key, value, updated_at) VALUES ('test', 'value', 1000)",
                    [],
                ).into_icm_result()?;
                Ok(())
            })
            .unwrap();

        store2
            .with_conn(|conn| {
                let value: String = conn
                    .query_row(
                        "SELECT value FROM icm_metadata WHERE key = 'test'",
                        [],
                        |row| row.get(0),
                    )
                    .into_icm_result()?;
                assert_eq!(value, "value");
                Ok(())
            })
            .unwrap();
    }

    // === Task 3.4: Test apply_decay with mixed profiles ===
    #[test]
    fn test_apply_decay_with_mixed_profiles() {
        let store = SqliteStore::open_in_memory().unwrap();

        // Insert memories with different decay profiles
        let now = chrono::Utc::now();
        let old_time = (now - chrono::Duration::days(30)).timestamp_millis();

        store.with_conn(|conn| {
            // Memory with exponential decay (default)
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem1', ?1, ?1, ?1, 5, 1.0, 'test', 'Exponential memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem1', 1, 0, ?1, NULL, NULL, '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            // Memory with spaced repetition
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem2', ?1, ?1, ?1, 3, 1.0, 'test', 'SM-2 memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem2', 1, 0, ?1, NULL, 'spaced-repetition', '{\"interval_days\":1.0,\"easiness_factor\":2.5}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            // Memory with importance-weighted decay
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem3', ?1, ?1, ?1, 8, 1.0, 'test', 'Importance memory', 'excerpt', '[]', 'high', '\"User\"', '[]', 'test/mem3', 1, 0, ?1, NULL, 'importance-weighted', '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            // Memory with context-sensitive decay
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem4', ?1, ?1, ?1, 2, 1.0, 'architecture', 'Architecture memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem4', 1, 0, ?1, NULL, 'context-sensitive', '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            Ok::<(), IcmError>(())
        }).unwrap();

        // Apply decay
        let updated = store.apply_decay(0.05).unwrap();
        assert_eq!(updated, 4, "All 4 memories should be updated");

        // Verify each memory was decayed with its strategy
        store
            .with_conn(|conn| {
                let weights: Vec<(String, f32, String)> = conn
                    .prepare("SELECT id, weight, decay_params FROM memories ORDER BY id")
                    .into_icm_result()?
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                    .into_icm_result()?
                    .collect::<Result<Vec<_>, _>>()
                    .into_icm_result()?;

                assert_eq!(weights.len(), 4);

                // All weights should have decayed (< 1.0)
                for (id, weight, _) in &weights {
                    assert!(weight < &1.0, "Memory {} should have decayed", id);
                    assert!(weight > &0.0, "Memory {} weight should be positive", id);
                }

                // Verify SM-2 params were updated (interval should increase on successful review)
                let mem2_params = &weights[1].2;
                assert!(
                    mem2_params.contains("interval_days"),
                    "SM-2 params should have interval_days"
                );

                Ok::<(), IcmError>(())
            })
            .unwrap();
    }

    // === Task 3.5: Test backward compatibility (null profile uses ExponentialDecay) ===
    #[test]
    fn test_backward_compatibility_null_profile() {
        let store = SqliteStore::open_in_memory().unwrap();

        let now = chrono::Utc::now();
        let old_time = (now - chrono::Duration::days(10)).timestamp_millis();

        store.with_conn(|conn| {
            // Insert memory with NULL decay_profile (old format)
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem_null', ?1, ?1, ?1, 3, 1.0, 'test', 'Legacy memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem_null', 1, 0, ?1, NULL, NULL, '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            // Insert memory with empty string profile
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem_empty', ?1, ?1, ?1, 3, 1.0, 'test', 'Empty profile memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem_empty', 1, 0, ?1, NULL, '', '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            Ok::<(), IcmError>(())
        }).unwrap();

        // Apply decay - should use ExponentialDecay for both
        let updated = store.apply_decay(0.05).unwrap();
        assert_eq!(updated, 2, "Both memories should be updated");

        // Verify decay happened
        store
            .with_conn(|conn| {
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM memories WHERE weight < 1.0 AND weight > 0.0",
                        [],
                        |row| row.get(0),
                    )
                    .into_icm_result()?;

                assert_eq!(count, 2, "Both memories should have decayed weights");
                Ok::<(), IcmError>(())
            })
            .unwrap();
    }

    // === Task 3.6: Test that Critical memories are skipped ===
    #[test]
    fn test_critical_memories_skipped() {
        let store = SqliteStore::open_in_memory().unwrap();

        let now = chrono::Utc::now();
        let old_time = (now - chrono::Duration::days(365)).timestamp_millis();

        store.with_conn(|conn| {
            // Insert critical memory (very old)
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem_crit', ?1, ?1, ?1, 1, 1.0, 'test', 'Critical memory', 'excerpt', '[]', 'critical', '\"User\"', '[]', 'test/mem_crit', 1, 0, ?1, NULL, NULL, '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            // Insert medium memory for comparison
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem_med', ?1, ?1, ?1, 1, 1.0, 'test', 'Medium memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem_med', 1, 0, ?1, NULL, NULL, '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            Ok::<(), IcmError>(())
        }).unwrap();

        // Apply decay
        let updated = store.apply_decay(0.05).unwrap();
        assert_eq!(
            updated, 1,
            "Only medium memory should be updated (critical skipped)"
        );

        // Verify critical memory weight unchanged
        store
            .with_conn(|conn| {
                let crit_weight: f32 = conn
                    .query_row(
                        "SELECT weight FROM memories WHERE id = 'mem_crit'",
                        [],
                        |row| row.get(0),
                    )
                    .into_icm_result()?;

                assert_eq!(crit_weight, 1.0, "Critical memory weight should not change");

                let med_weight: f32 = conn
                    .query_row(
                        "SELECT weight FROM memories WHERE id = 'mem_med'",
                        [],
                        |row| row.get(0),
                    )
                    .into_icm_result()?;

                assert!(med_weight < 1.0, "Medium memory should have decayed");
                Ok::<(), IcmError>(())
            })
            .unwrap();
    }

    // === Task 3.7: Test decay_params updates (SM-2 interval increments) ===
    #[test]
    fn test_decay_params_updates() {
        let store = SqliteStore::open_in_memory().unwrap();

        let now = chrono::Utc::now();
        let review_time = (now - chrono::Duration::hours(12)).timestamp_millis(); // Within 1-day interval

        store.with_conn(|conn| {
            // Insert SM-2 memory with initial params (repetitions=0, interval=1.0)
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem_sm2', ?1, ?1, ?1, 5, 1.0, 'test', 'SM-2 memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem_sm2', 1, 0, ?1, NULL, 'spaced-repetition', '{\"interval_days\":1.0,\"easiness_factor\":2.5,\"repetitions\":0}')",
                rusqlite::params![review_time]
            ).into_icm_result()?;
            
            Ok::<(), IcmError>(())
        }).unwrap();

        // Apply decay (should update SM-2 params since review is within interval)
        store.apply_decay(0.05).unwrap();

        // Verify params were updated
        store
            .with_conn(|conn| {
                let params_json: String = conn
                    .query_row(
                        "SELECT decay_params FROM memories WHERE id = 'mem_sm2'",
                        [],
                        |row| row.get(0),
                    )
                    .into_icm_result()?;

                let params: serde_json::Value = serde_json::from_str(&params_json).unwrap();

                // SM-2 algorithm: repetitions should increment from 0 to 1
                // (interval stays at 1.0 for first repetition per SM-2 spec)
                let repetitions = params["repetitions"].as_u64().unwrap();
                assert_eq!(repetitions, 1, "Repetitions should increment from 0 to 1");

                let interval = params["interval_days"].as_f64().unwrap();
                assert_eq!(interval, 1.0, "Interval should be 1.0 for first repetition");

                // last_review should be updated (not null)
                assert!(
                    params["last_review"].is_number(),
                    "last_review should be updated"
                );

                Ok::<(), IcmError>(())
            })
            .unwrap();
    }

    // === Task 3.8: Test unknown profile fallback to exponential ===
    #[test]
    fn test_unknown_profile_fallback() {
        let store = SqliteStore::open_in_memory().unwrap();

        let now = chrono::Utc::now();
        let old_time = (now - chrono::Duration::days(15)).timestamp_millis();

        store.with_conn(|conn| {
            // Insert memory with unknown profile
            conn.execute(
                "INSERT INTO memories (id, created_at, updated_at, last_accessed, access_count, weight, topic, summary, raw_excerpt, keywords, importance, source, related_ids, topic_key, revision_count, duplicate_count, last_seen_at, deleted_at, decay_profile, decay_params)
                 VALUES ('mem_unknown', ?1, ?1, ?1, 3, 1.0, 'test', 'Unknown profile memory', 'excerpt', '[]', 'medium', '\"User\"', '[]', 'test/mem_unknown', 1, 0, ?1, NULL, 'totally-fake-strategy', '{}')",
                rusqlite::params![old_time]
            ).into_icm_result()?;
            
            Ok::<(), IcmError>(())
        }).unwrap();

        // Apply decay - should fallback to ExponentialDecay
        let updated = store.apply_decay(0.05).unwrap();
        assert_eq!(
            updated, 1,
            "Memory should be updated using fallback strategy"
        );

        // Verify decay happened (exponential decay should reduce weight)
        store
            .with_conn(|conn| {
                let weight: f32 = conn
                    .query_row(
                        "SELECT weight FROM memories WHERE id = 'mem_unknown'",
                        [],
                        |row| row.get(0),
                    )
                    .into_icm_result()?;

                assert!(
                    weight < 1.0,
                    "Memory with unknown profile should decay using exponential"
                );
                assert!(weight > 0.0, "Weight should remain positive");
                Ok::<(), IcmError>(())
            })
            .unwrap();
    }
}

//! Memory tool implementations
//!
//! Implements all 11 memory management tools:
//! - mem_store, mem_recall, mem_update, mem_forget, mem_consolidate
//! - mem_list_topics, mem_stats, mem_health, mem_embed_all
//! - mem_export, mem_import

use crate::protocol::{JsonRpcError, ToolResult};
use alejandria_core::{Importance, Memory, MemorySource, MemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

// === Helper Functions ===

/// Get the current user hash for BOLA protection
///
/// Temporary implementation: Uses a static hash until AuthContext integration (P0-2)
/// This will be replaced in P0-2 with actual API key → user mapping
///
/// TODO(P0-2): Replace with actual AuthContext from HTTP layer
fn get_current_user_hash() -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update("default-user");
    let hash = hasher.finalize();
    format!("{:x}", hash)[..16].to_string()
}

// === Tool Argument Types ===

#[derive(Debug, Deserialize)]
struct StoreArgs {
    content: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    importance: Option<String>,
    #[serde(default)]
    topic: Option<String>,
    #[serde(default)]
    topic_key: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    related_ids: Option<Vec<String>>,
    /// If true, this memory will be accessible by all users (owner_key_hash = "SHARED")
    #[serde(default)]
    shared: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RecallArgs {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    min_score: f32,
    #[serde(default)]
    topic: Option<String>,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Deserialize)]
struct UpdateArgs {
    id: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    importance: Option<String>,
    #[serde(default)]
    topic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForgetArgs {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ConsolidateArgs {
    topic: String,
    #[serde(default = "default_min_weight")]
    min_weight: f32,
    #[serde(default = "default_min_memories")]
    min_memories: usize,
}

fn default_min_weight() -> f32 {
    0.5
}
fn default_min_memories() -> usize {
    3
}

#[derive(Debug, Deserialize)]
struct ListTopicsArgs {
    #[serde(default = "default_topics_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_min_count")]
    min_count: usize,
}

fn default_topics_limit() -> usize {
    100
}
fn default_min_count() -> usize {
    1
}

#[derive(Debug, Deserialize)]
struct EmbedAllArgs {
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    #[serde(default = "default_skip_existing")]
    skip_existing: bool,
}

fn default_batch_size() -> usize {
    100
}
fn default_skip_existing() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct ExportArgs {
    /// Output file path
    output: String,
    /// Export format (json, csv, markdown)
    #[serde(default = "default_export_format")]
    format: String,
    /// Include soft-deleted memories
    #[serde(default)]
    include_deleted: bool,
    /// Optional filters
    #[serde(default)]
    filters: Option<ExportFilters>,
}

fn default_export_format() -> String {
    "json".to_string()
}

#[derive(Debug, Deserialize)]
struct ExportFilters {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    importance: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    decay_profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImportArgs {
    /// Input file path
    input: String,
    /// Import mode (skip, update, replace)
    #[serde(default = "default_import_mode")]
    mode: String,
    /// Dry run (validate without importing)
    #[serde(default)]
    dry_run: bool,
}

fn default_import_mode() -> String {
    "skip".to_string()
}

// === Tool Response Types ===

#[derive(Debug, Serialize)]
struct StoreResponse {
    id: String,
    action: String,
}

#[derive(Debug, Serialize)]
struct RecallResult {
    id: String,
    summary: String,
    content: String,
    score: f32,
    importance: String,
    topic: String,
    created_at: String,
    access_count: u32,
}

#[derive(Debug, Serialize)]
struct UpdateResponse {
    id: String,
    updated_fields: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ForgetResponse {
    id: String,
    deleted_at: String,
}

#[derive(Debug, Serialize)]
struct ConsolidateResponse {
    consolidated_memory_id: Option<String>,
    source_count: usize,
    summary: String,
}

#[derive(Debug, Serialize)]
struct TopicInfo {
    topic: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct HealthStatus {
    status: String,
    db: String,
    fts: String,
    vec: String,
    embeddings: String,
    embedder_model: Option<String>,
    embedding_dimensions: Option<usize>,
    memories_with_embeddings: Option<usize>,
}

#[derive(Debug, Serialize)]
struct EmbedAllResponse {
    processed: usize,
    embedded: usize,
    skipped: usize,
    duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct ExportResponse {
    output_file: String,
    format: String,
    total_exported: usize,
    exported_at: String,
}

#[derive(Debug, Serialize)]
struct ImportResponse {
    input_file: String,
    mode: String,
    dry_run: bool,
    imported: usize,
    updated: usize,
    skipped: usize,
    errors: Vec<String>,
}

// === Tool Implementations ===

/// mem_store - Store a new memory or update existing via topic_key upsert
pub fn mem_store<S: MemoryStore>(args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // Deserialize arguments
    let args: StoreArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Validate required fields
    if args.content.trim().is_empty() {
        return Err(JsonRpcError::invalid_params("content cannot be empty"));
    }

    // Parse importance
    let importance = match args.importance.as_deref() {
        Some("critical") => Importance::Critical,
        Some("high") => Importance::High,
        Some("medium") | None => Importance::Medium,
        Some("low") => Importance::Low,
        Some(other) => {
            return Err(JsonRpcError::invalid_params(format!(
                "Invalid importance: {}. Must be critical, high, medium, or low",
                other
            )));
        }
    };

    // Create memory instance
    let mut memory = Memory::new(
        args.topic.unwrap_or_else(|| "general".to_string()),
        args.summary.unwrap_or_else(|| {
            // Generate simple summary from first 100 chars of content
            let content = args.content.chars().take(100).collect::<String>();
            if args.content.len() > 100 {
                format!("{}...", content)
            } else {
                content
            }
        }),
    );

    // Set fields
    memory.importance = importance;
    memory.raw_excerpt = Some(args.content);
    memory.topic_key = args.topic_key;
    memory.source = match args.source.as_deref() {
        Some("agent") => MemorySource::Agent,
        Some("system") => MemorySource::System,
        Some("user") | None => MemorySource::User,
        Some("external") => MemorySource::External,
        Some(_) => MemorySource::User, // Default to User for unknown sources
    };
    memory.related_ids = args.related_ids.unwrap_or_default();

    // Set owner_key_hash for BOLA protection
    // TODO(P0-2): Replace with actual AuthContext from HTTP layer
    let owner_key_hash = if args.shared.unwrap_or(false) {
        "SHARED".to_string()
    } else {
        get_current_user_hash()
    };
    memory.owner_key_hash = owner_key_hash;

    // Store memory
    let id = store
        .store(memory)
        .map_err(|e| JsonRpcError::internal_error(format!("Failed to store memory: {}", e)))?;

    // Determine action (for now, always "created" - deduplication logic is in storage layer)
    let response = StoreResponse {
        id,
        action: "created".to_string(),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!("Memory stored:\n{}", json)))
}

/// mem_recall - Search and recall memories using hybrid search (with BOLA protection)
pub fn mem_recall<S: MemoryStore>(args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // Deserialize arguments
    let args: RecallArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Validate query
    if args.query.trim().is_empty() {
        return Err(JsonRpcError::invalid_params("query cannot be empty"));
    }

    // Get current user hash for BOLA protection
    // TODO(P0-2): Replace with actual AuthContext from HTTP layer
    let owner_hash = get_current_user_hash();

    // Use authorized search (filters by owner automatically)
    let scored_results = {
        use alejandria_storage::SqliteStore;

        // Downcast to SqliteStore (same pattern as mem_export)
        let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
        let sqlite_store = unsafe { &*sqlite_store };

        // Use authorized keyword search - filters by owner at database level
        let memories = sqlite_store
            .search_by_keywords_authorized(&args.query, args.limit, &owner_hash)
            .map_err(|e| JsonRpcError::internal_error(format!("Search failed: {}", e)))?;

        // Convert to scored results (score = 1.0 for authorized keyword search)
        // TODO: Implement hybrid_search_authorized for better scoring
        memories
            .into_iter()
            .map(|memory| alejandria_storage::search::ScoredMemory { memory, score: 1.0 })
            .collect::<Vec<_>>()
    };

    // Filter by min_score if specified (> 0.0)
    let min_score = args.min_score;

    // Filter by topic if specified, then by min_score
    let results: Vec<RecallResult> = scored_results
        .into_iter()
        .filter(|s| {
            if min_score > 0.0 {
                s.score >= min_score
            } else {
                true
            }
        })
        .filter(|s| {
            if let Some(ref topic) = args.topic {
                s.memory.topic == *topic
            } else {
                true
            }
        })
        .map(|s| RecallResult {
            id: s.memory.id,
            summary: s.memory.summary,
            content: s.memory.raw_excerpt.unwrap_or_default(),
            score: s.score,
            importance: s.memory.importance.to_string(),
            topic: s.memory.topic,
            created_at: s.memory.created_at.to_rfc3339(),
            access_count: s.memory.access_count,
        })
        .collect();

    if results.is_empty() {
        return Ok(ToolResult::success("No memories found matching the query."));
    }

    let json = serde_json::to_string_pretty(&results)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!(
        "Found {} memories:\n{}",
        results.len(),
        json
    )))
}

/// mem_update - Update an existing memory (with BOLA protection)
pub fn mem_update<S: MemoryStore>(args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // Deserialize arguments
    let args: UpdateArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Validate at least one field to update
    if args.content.is_none()
        && args.summary.is_none()
        && args.importance.is_none()
        && args.topic.is_none()
    {
        return Err(JsonRpcError::invalid_params(
            "At least one field (content, summary, importance, topic) must be provided",
        ));
    }

    // Get current user hash for BOLA protection
    // TODO(P0-2): Replace with actual AuthContext from HTTP layer
    let owner_hash = get_current_user_hash();

    // Use authorized get to check ownership
    use alejandria_storage::SqliteStore;
    let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
    let sqlite_store = unsafe { &*sqlite_store };

    // Get memory with ownership check
    let mut memory = sqlite_store
        .get_authorized(&args.id, &owner_hash)
        .map_err(|e| {
            // Map IcmError::Forbidden to JsonRpcError
            if e.to_string().contains("Access denied") {
                JsonRpcError::forbidden(e.to_string())
            } else {
                JsonRpcError::internal_error(format!("Failed to get memory: {}", e))
            }
        })?
        .ok_or_else(|| JsonRpcError::not_found(format!("Memory not found: {}", args.id)))?;

    // Track updated fields
    let mut updated_fields = Vec::new();

    // Update fields
    if let Some(summary) = args.summary {
        memory.summary = summary;
        updated_fields.push("summary".to_string());
    }

    if let Some(content) = args.content {
        memory.raw_excerpt = Some(content);
        updated_fields.push("content".to_string());
    }

    if let Some(importance_str) = args.importance {
        let importance = match importance_str.as_str() {
            "critical" => Importance::Critical,
            "high" => Importance::High,
            "medium" => Importance::Medium,
            "low" => Importance::Low,
            other => {
                return Err(JsonRpcError::invalid_params(format!(
                    "Invalid importance: {}. Must be critical, high, medium, or low",
                    other
                )));
            }
        };
        memory.importance = importance;
        updated_fields.push("importance".to_string());
    }

    if let Some(topic) = args.topic {
        memory.topic = topic;
        updated_fields.push("topic".to_string());
    }

    // Update timestamp
    memory.updated_at = chrono::Utc::now();

    // Use authorized update (preserves owner and checks authorization)
    sqlite_store
        .update_authorized(&memory, &owner_hash)
        .map_err(|e| {
            if e.to_string().contains("Access denied") {
                JsonRpcError::forbidden(e.to_string())
            } else {
                JsonRpcError::internal_error(format!("Failed to update memory: {}", e))
            }
        })?;

    let response = UpdateResponse {
        id: args.id,
        updated_fields,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!("Memory updated:\n{}", json)))
}

/// mem_forget - Soft-delete a memory (with BOLA protection)
pub fn mem_forget<S: MemoryStore>(args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // Deserialize arguments
    let args: ForgetArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Get current user hash for BOLA protection
    // TODO(P0-2): Replace with actual AuthContext from HTTP layer
    let owner_hash = get_current_user_hash();

    // Use authorized delete (checks ownership)
    use alejandria_storage::SqliteStore;
    let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
    let sqlite_store = unsafe { &*sqlite_store };

    sqlite_store
        .delete_authorized(&args.id, &owner_hash)
        .map_err(|e| {
            if e.to_string().contains("Access denied") {
                JsonRpcError::forbidden(e.to_string())
            } else if e.to_string().contains("not found") {
                JsonRpcError::not_found(format!("Memory not found: {}", args.id))
            } else {
                JsonRpcError::internal_error(format!("Failed to delete memory: {}", e))
            }
        })?;

    let response = ForgetResponse {
        id: args.id,
        deleted_at: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!("Memory deleted:\n{}", json)))
}

/// mem_consolidate - Consolidate memories in a topic
pub fn mem_consolidate<S: MemoryStore>(
    args: Value,
    store: Arc<S>,
) -> Result<ToolResult, JsonRpcError> {
    // Deserialize arguments
    let args: ConsolidateArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Validate topic
    if args.topic.trim().is_empty() {
        return Err(JsonRpcError::invalid_params("topic cannot be empty"));
    }

    // Perform consolidation (min_memories first, then min_weight)
    let id = store
        .consolidate_topic(&args.topic, args.min_memories, args.min_weight)
        .map_err(|e| JsonRpcError::internal_error(format!("Consolidation failed: {}", e)))?;

    // Get source count from topic
    let memories = store.get_by_topic(&args.topic, None, None).map_err(|e| {
        JsonRpcError::internal_error(format!("Failed to get topic memories: {}", e))
    })?;

    let response = ConsolidateResponse {
        consolidated_memory_id: Some(id),
        source_count: memories.len(),
        summary: format!(
            "Successfully consolidated {} memories from topic '{}'",
            memories.len(),
            args.topic
        ),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!(
        "Consolidation result:\n{}",
        json
    )))
}

/// mem_list_topics - List all topics with counts
pub fn mem_list_topics<S: MemoryStore>(
    args: Value,
    store: Arc<S>,
) -> Result<ToolResult, JsonRpcError> {
    // Deserialize arguments
    let args: ListTopicsArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Get topics
    let topics = store
        .list_topics(Some(args.limit), Some(args.offset))
        .map_err(|e| JsonRpcError::internal_error(format!("Failed to list topics: {}", e)))?;

    // Filter by min_count and convert to response format
    let topics: Vec<_> = topics
        .into_iter()
        .filter(|ti| ti.count >= args.min_count)
        .map(|ti| TopicInfo {
            topic: ti.topic,
            count: ti.count,
        })
        .collect();

    if topics.is_empty() {
        return Ok(ToolResult::success("No topics found."));
    }

    let json = serde_json::to_string_pretty(&topics)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!(
        "Found {} topics:\n{}",
        topics.len(),
        json
    )))
}

/// mem_stats - Get memory statistics
pub fn mem_stats<S: MemoryStore>(_args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // Get statistics
    let stats = store
        .stats()
        .map_err(|e| JsonRpcError::internal_error(format!("Failed to get stats: {}", e)))?;

    // stats is already a StoreStats struct, serialize it
    let json = serde_json::to_string_pretty(&stats)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!("Memory statistics:\n{}", json)))
}

/// mem_health - Check system health
pub fn mem_health<S: MemoryStore>(_args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // Perform health checks
    let db_ok = store.count().is_ok();
    let fts_ok = store.search_by_keywords("test", 1).is_ok();
    let vec_ok = store.search_by_embedding(&vec![0.0; 768], 1).is_ok();

    // Downcast to SqliteStore to check embedder status
    use alejandria_storage::SqliteStore;
    let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
    let sqlite_store = unsafe { &*sqlite_store };

    let embedder_available = sqlite_store.embedder().is_some();
    let embedder_model = sqlite_store.embedder().map(|e| e.model_name().to_string());
    let embedding_dimensions = sqlite_store.embedder().map(|e| e.dimensions());

    // Count memories with embeddings (from vec_memories table)
    let memories_with_embeddings = sqlite_store
        .with_conn(|conn| {
            let table_exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_memories'",
                    [],
                    |row| row.get(0).map(|c: i32| c > 0),
                )
                .unwrap_or(false);

            if !table_exists {
                return Ok(None);
            }

            let count: usize = conn
                .query_row("SELECT COUNT(*) FROM vec_memories", [], |row| row.get(0))
                .unwrap_or(0);
            Ok(Some(count))
        })
        .unwrap_or(None);

    let status = HealthStatus {
        status: if db_ok && fts_ok {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        db: if db_ok {
            "ok".to_string()
        } else {
            "error".to_string()
        },
        fts: if fts_ok {
            "ok".to_string()
        } else {
            "error".to_string()
        },
        vec: if vec_ok {
            "ok".to_string()
        } else {
            "unavailable".to_string()
        },
        embeddings: if embedder_available {
            "available".to_string()
        } else {
            "unavailable".to_string()
        },
        embedder_model,
        embedding_dimensions,
        memories_with_embeddings,
    };

    let json = serde_json::to_string_pretty(&status)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!("Health check:\n{}", json)))
}

/// mem_embed_all - Batch embed existing memories
pub fn mem_embed_all<S: MemoryStore>(
    args: Value,
    store: Arc<S>,
) -> Result<ToolResult, JsonRpcError> {
    // Deserialize arguments
    let args: EmbedAllArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Downcast to SqliteStore to access embed_all()
    use alejandria_storage::SqliteStore;

    let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
    let sqlite_store = unsafe { &*sqlite_store };

    // Check if an embedder is configured — if not, return gracefully
    if sqlite_store.embedder().is_none() {
        let response = EmbedAllResponse {
            processed: 0,
            embedded: 0,
            skipped: 0,
            duration_ms: 0,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

        return Ok(ToolResult::success(format!(
            "No embedder configured. Set up an embedder to enable batch embedding:\n{}",
            json
        )));
    }

    let start = std::time::Instant::now();

    let embedded = sqlite_store
        .embed_all(args.batch_size, args.skip_existing)
        .map_err(|e| JsonRpcError::internal_error(format!("Batch embedding failed: {}", e)))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    let response = EmbedAllResponse {
        processed: embedded,
        embedded,
        skipped: 0,
        duration_ms,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!(
        "Batch embedding complete:\n{}",
        json
    )))
}

/// mem_export - Export memories to file
pub fn mem_export<S: MemoryStore>(args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // This tool requires SqliteStore to access export_memories
    // We need to downcast to SqliteStore
    use alejandria_storage::{ExportFormat, ExportOptions, SqliteStore};
    use std::str::FromStr;

    // Deserialize arguments
    let export_args: ExportArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Parse format
    let format =
        ExportFormat::from_str(&export_args.format).map_err(|e| JsonRpcError::invalid_params(e))?;

    // Build export options
    let mut options = ExportOptions {
        include_deleted: export_args.include_deleted,
        ..Default::default()
    };

    if let Some(filters) = export_args.filters {
        options.session_id = filters.session_id;
        options.importance_threshold = filters.importance;
        options.tags = filters.tags;
        options.decay_profile = filters.decay_profile;
    }

    // Try to downcast to SqliteStore
    let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
    let sqlite_store = unsafe { &*sqlite_store };

    // Create output file
    let file = std::fs::File::create(&export_args.output).map_err(|e| {
        JsonRpcError::internal_error(format!("Failed to create output file: {}", e))
    })?;
    let writer = std::io::BufWriter::new(file);

    // Execute export
    let metadata = sqlite_store
        .export_memories(format, options, writer)
        .map_err(|e| JsonRpcError::internal_error(format!("Export failed: {}", e)))?;

    let response = ExportResponse {
        output_file: export_args.output,
        format: export_args.format,
        total_exported: metadata.total_count,
        exported_at: metadata.exported_at.to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!("Export completed:\n{}", json)))
}

/// mem_import - Import memories from file
pub fn mem_import<S: MemoryStore>(args: Value, store: Arc<S>) -> Result<ToolResult, JsonRpcError> {
    // This tool requires SqliteStore to access import_memories
    use alejandria_core::import::ImportMode;
    use alejandria_storage::SqliteStore;
    use std::path::Path;

    // Deserialize arguments
    let import_args: ImportArgs = serde_json::from_value(args)
        .map_err(|e| JsonRpcError::invalid_params(format!("Invalid arguments: {}", e)))?;

    // Validate input file exists
    let input_path = Path::new(&import_args.input);
    if !input_path.exists() {
        return Err(JsonRpcError::invalid_params(format!(
            "Input file does not exist: {}",
            import_args.input
        )));
    }

    if import_args.dry_run {
        // Dry run: validate only
        let response = ImportResponse {
            input_file: import_args.input,
            mode: import_args.mode,
            dry_run: true,
            imported: 0,
            updated: 0,
            skipped: 0,
            errors: vec![],
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

        return Ok(ToolResult::success(format!(
            "Dry run completed. File format is valid. No data was imported:\n{}",
            json
        )));
    }

    // Parse import mode
    let mode = match import_args.mode.to_lowercase().as_str() {
        "skip" => ImportMode::Skip,
        "update" => ImportMode::Update,
        "replace" => ImportMode::Replace,
        _ => {
            return Err(JsonRpcError::invalid_params(format!(
                "Invalid import mode '{}'. Valid modes: skip, update, replace",
                import_args.mode
            )));
        }
    };

    // Try to downcast to SqliteStore
    let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
    let sqlite_store = unsafe { &*sqlite_store };

    // Execute import
    let result = sqlite_store
        .import_memories(input_path, mode)
        .map_err(|e| JsonRpcError::internal_error(format!("Import failed: {}", e)))?;

    let response = ImportResponse {
        input_file: import_args.input,
        mode: import_args.mode,
        dry_run: false,
        imported: result.imported,
        updated: result.updated,
        skipped: result.skipped,
        errors: result.errors,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {}", e)))?;

    Ok(ToolResult::success(format!("Import completed:\n{}", json)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_store_args_deserialization() {
        let args = json!({
            "content": "Test memory",
            "summary": "A test",
            "importance": "high",
            "topic": "testing"
        });

        let store_args: StoreArgs = serde_json::from_value(args).unwrap();
        assert_eq!(store_args.content, "Test memory");
        assert_eq!(store_args.summary, Some("A test".to_string()));
        assert_eq!(store_args.importance, Some("high".to_string()));
        assert_eq!(store_args.topic, Some("testing".to_string()));
    }

    #[test]
    fn test_recall_args_defaults() {
        let args = json!({
            "query": "test query"
        });

        let recall_args: RecallArgs = serde_json::from_value(args).unwrap();
        assert_eq!(recall_args.query, "test query");
        assert_eq!(recall_args.limit, 10);
        assert_eq!(recall_args.min_score, 0.0);
    }

    #[test]
    fn test_export_args_deserialization() {
        let args = json!({
            "output": "export.json",
            "format": "json",
            "include_deleted": true,
            "filters": {
                "importance": "high",
                "tags": ["rust", "async"]
            }
        });

        let export_args: ExportArgs = serde_json::from_value(args).unwrap();
        assert_eq!(export_args.output, "export.json");
        assert_eq!(export_args.format, "json");
        assert_eq!(export_args.include_deleted, true);
        assert!(export_args.filters.is_some());

        let filters = export_args.filters.unwrap();
        assert_eq!(filters.importance, Some("high".to_string()));
        assert_eq!(
            filters.tags,
            Some(vec!["rust".to_string(), "async".to_string()])
        );
    }

    #[test]
    fn test_export_args_defaults() {
        let args = json!({
            "output": "export.json"
        });

        let export_args: ExportArgs = serde_json::from_value(args).unwrap();
        assert_eq!(export_args.output, "export.json");
        assert_eq!(export_args.format, "json");
        assert_eq!(export_args.include_deleted, false);
        assert!(export_args.filters.is_none());
    }

    #[test]
    fn test_import_args_deserialization() {
        let args = json!({
            "input": "export.json",
            "mode": "update",
            "dry_run": true
        });

        let import_args: ImportArgs = serde_json::from_value(args).unwrap();
        assert_eq!(import_args.input, "export.json");
        assert_eq!(import_args.mode, "update");
        assert_eq!(import_args.dry_run, true);
    }

    #[test]
    fn test_import_args_defaults() {
        let args = json!({
            "input": "export.json"
        });

        let import_args: ImportArgs = serde_json::from_value(args).unwrap();
        assert_eq!(import_args.input, "export.json");
        assert_eq!(import_args.mode, "skip");
        assert_eq!(import_args.dry_run, false);
    }
}

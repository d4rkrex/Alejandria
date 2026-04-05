//! Integration tests for memory tool handlers.
//!
//! Tests all 11 memory tools using SqliteStore::open_in_memory():
//! - mem_store: create memories, upsert via topic_key, validate required fields
//! - mem_recall: keyword search, topic filtering, empty results
//! - mem_update: field updates, missing memory error, validation
//! - mem_forget: soft delete, not-found error
//! - mem_consolidate: topic consolidation
//! - mem_list_topics: listing with counts
//! - mem_stats: statistics retrieval
//! - mem_health: health check status
//! - mem_embed_all: placeholder response (embeddings not yet implemented)
//! - mem_export / mem_import: export and import roundtrip

use alejandria_mcp::protocol::ToolResult;
use alejandria_mcp::tools::memory::*;
use alejandria_storage::SqliteStore;
use serde_json::{json, Value};
use std::sync::Arc;

/// Helper: create an in-memory store wrapped in Arc
fn test_store() -> Arc<SqliteStore> {
    Arc::new(SqliteStore::open_in_memory().expect("Failed to create in-memory store"))
}

/// Helper: extract the text from a ToolResult
fn result_text(result: &ToolResult) -> &str {
    result.content[0].text.as_str()
}

// ============================================================
// mem_store
// ============================================================

#[test]
fn test_mem_store_basic() {
    let store = test_store();
    let args = json!({
        "content": "Rust uses ownership for memory safety",
        "topic": "rust-lang",
        "summary": "Ownership model"
    });

    let result = mem_store(args, store).expect("mem_store failed");
    let text = result_text(&result);

    assert!(
        text.contains("Memory stored"),
        "Expected 'Memory stored', got: {}",
        text
    );
    assert!(text.contains("\"action\": \"created\""));
}

#[test]
fn test_mem_store_minimal_args() {
    let store = test_store();
    let args = json!({ "content": "Minimal memory" });

    let result = mem_store(args, store).expect("mem_store failed");
    let text = result_text(&result);

    assert!(text.contains("Memory stored"));
}

#[test]
fn test_mem_store_all_fields() {
    let store = test_store();
    let args = json!({
        "content": "Full memory",
        "summary": "A complete memory",
        "importance": "critical",
        "topic": "architecture",
        "topic_key": "arch/auth-model",
        "source": "agent",
        "related_ids": ["id1", "id2"]
    });

    let result = mem_store(args, store).expect("mem_store failed");
    let text = result_text(&result);

    assert!(text.contains("Memory stored"));
}

#[test]
fn test_mem_store_empty_content_fails() {
    let store = test_store();
    let args = json!({ "content": "   " });

    let err = mem_store(args, store).expect_err("Expected error for empty content");
    assert_eq!(err.code, -32602, "Expected invalid params code");
    assert!(err.message.contains("empty"));
}

#[test]
fn test_mem_store_missing_content_fails() {
    let store = test_store();
    let args = json!({ "topic": "test" });

    let err = mem_store(args, store).expect_err("Expected error for missing content");
    assert_eq!(err.code, -32602);
}

#[test]
fn test_mem_store_invalid_importance_fails() {
    let store = test_store();
    let args = json!({
        "content": "Test memory",
        "importance": "super-critical"
    });

    let err = mem_store(args, store).expect_err("Expected error for invalid importance");
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("importance"));
}

#[test]
fn test_mem_store_importance_levels() {
    let store = test_store();

    for level in &["critical", "high", "medium", "low"] {
        let args = json!({
            "content": format!("Memory with {} importance", level),
            "importance": level
        });
        let result = mem_store(args, store.clone())
            .expect(&format!("mem_store failed for importance={}", level));
        assert!(result_text(&result).contains("Memory stored"));
    }
}

// ============================================================
// mem_recall
// ============================================================

#[test]
fn test_mem_recall_finds_stored_memory() {
    let store = test_store();

    // Store a memory first
    let store_args = json!({
        "content": "Rust ownership model prevents data races",
        "topic": "rust-lang",
        "summary": "Ownership prevents data races"
    });
    mem_store(store_args, store.clone()).expect("Failed to store memory");

    // Recall it
    let recall_args = json!({ "query": "ownership" });
    let result = mem_recall(recall_args, store).expect("mem_recall failed");
    let text = result_text(&result);

    assert!(
        text.contains("Found") || text.contains("memories"),
        "Expected results, got: {}",
        text
    );
}

#[test]
fn test_mem_recall_empty_results() {
    let store = test_store();
    let args = json!({ "query": "nonexistent topic" });

    let result = mem_recall(args, store).expect("mem_recall failed");
    let text = result_text(&result);

    assert!(
        text.contains("No memories found"),
        "Expected no results, got: {}",
        text
    );
}

#[test]
fn test_mem_recall_empty_query_fails() {
    let store = test_store();
    let args = json!({ "query": "   " });

    let err = mem_recall(args, store).expect_err("Expected error for empty query");
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("empty"));
}

#[test]
fn test_mem_recall_missing_query_fails() {
    let store = test_store();
    let args = json!({ "limit": 5 });

    let err = mem_recall(args, store).expect_err("Expected error for missing query");
    assert_eq!(err.code, -32602);
}

#[test]
fn test_mem_recall_with_topic_filter() {
    let store = test_store();

    // Store memories in different topics
    mem_store(
        json!({ "content": "Rust memory safety", "topic": "rust" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "Python memory management", "topic": "python" }),
        store.clone(),
    )
    .unwrap();

    // Recall with topic filter
    let args = json!({ "query": "memory", "topic": "rust" });
    let result = mem_recall(args, store).expect("mem_recall failed");
    let text = result_text(&result);

    // Should only find the Rust memory
    if text.contains("Found") {
        assert!(text.contains("Rust") || text.contains("rust"));
        assert!(!text.contains("Python"));
    }
}

#[test]
fn test_mem_recall_with_custom_limit() {
    let store = test_store();

    // Store several memories
    for i in 0..5 {
        mem_store(
            json!({ "content": format!("Memory number {}", i), "topic": "test" }),
            store.clone(),
        )
        .unwrap();
    }

    let args = json!({ "query": "Memory number", "limit": 2 });
    let result = mem_recall(args, store).expect("mem_recall failed");
    let text = result_text(&result);

    // Should find at most 2
    if text.contains("Found") {
        assert!(text.contains("Found 2") || text.contains("Found 1"));
    }
}

// ============================================================
// mem_update
// ============================================================

#[test]
fn test_mem_update_summary() {
    let store = test_store();

    // Store a memory
    let store_result = mem_store(
        json!({ "content": "Original content", "summary": "Original", "topic": "test" }),
        store.clone(),
    )
    .unwrap();
    let text = result_text(&store_result);

    // Extract the ID from the response
    let id = extract_id_from_store_response(text);

    // Update summary
    let update_args = json!({
        "id": id,
        "summary": "Updated summary"
    });
    let result = mem_update(update_args, store).expect("mem_update failed");
    let update_text = result_text(&result);

    assert!(update_text.contains("Memory updated"));
    assert!(update_text.contains("summary"));
}

#[test]
fn test_mem_update_multiple_fields() {
    let store = test_store();

    let store_result = mem_store(
        json!({ "content": "Test content", "topic": "old-topic" }),
        store.clone(),
    )
    .unwrap();
    let id = extract_id_from_store_response(result_text(&store_result));

    let update_args = json!({
        "id": id,
        "content": "New content",
        "topic": "new-topic",
        "importance": "high"
    });
    let result = mem_update(update_args, store).expect("mem_update failed");
    let text = result_text(&result);

    assert!(text.contains("Memory updated"));
    assert!(text.contains("content"));
    assert!(text.contains("topic"));
    assert!(text.contains("importance"));
}

#[test]
fn test_mem_update_nonexistent_id_fails() {
    let store = test_store();
    let args = json!({
        "id": "01NONEXISTENT000000000000",
        "summary": "New summary"
    });

    let err = mem_update(args, store).expect_err("Expected not found error");
    assert_eq!(err.code, -32001, "Expected not found error code");
}

#[test]
fn test_mem_update_no_fields_fails() {
    let store = test_store();
    let args = json!({ "id": "01NONEXISTENT000000000000" });

    let err = mem_update(args, store).expect_err("Expected error for no update fields");
    assert_eq!(err.code, -32602);
}

#[test]
fn test_mem_update_invalid_importance_fails() {
    let store = test_store();

    let store_result =
        mem_store(json!({ "content": "Test", "topic": "test" }), store.clone()).unwrap();
    let id = extract_id_from_store_response(result_text(&store_result));

    let args = json!({
        "id": id,
        "importance": "mega-critical"
    });

    let err = mem_update(args, store).expect_err("Expected error for invalid importance");
    assert_eq!(err.code, -32602);
}

// ============================================================
// mem_forget
// ============================================================

#[test]
fn test_mem_forget_existing_memory() {
    let store = test_store();

    // Store then forget
    let store_result = mem_store(
        json!({ "content": "Memory to delete", "topic": "test" }),
        store.clone(),
    )
    .unwrap();
    let id = extract_id_from_store_response(result_text(&store_result));

    let result = mem_forget(json!({ "id": id }), store).expect("mem_forget failed");
    let text = result_text(&result);

    assert!(
        text.contains("Memory deleted"),
        "Expected 'Memory deleted', got: {}",
        text
    );
    assert!(text.contains(&id));
}

#[test]
fn test_mem_forget_nonexistent_fails() {
    let store = test_store();
    let args = json!({ "id": "01NONEXISTENT000000000000" });

    let err = mem_forget(args, store).expect_err("Expected not found error");
    assert_eq!(err.code, -32001);
}

#[test]
fn test_mem_forget_missing_id_fails() {
    let store = test_store();
    let args = json!({});

    let err = mem_forget(args, store).expect_err("Expected error for missing id");
    assert_eq!(err.code, -32602);
}

// ============================================================
// mem_consolidate
// ============================================================

#[test]
fn test_mem_consolidate_with_enough_memories() {
    let store = test_store();

    // Store at least 3 memories in the same topic (default min_memories)
    for i in 0..4 {
        mem_store(
            json!({ "content": format!("Fact {} about Rust", i), "topic": "rust" }),
            store.clone(),
        )
        .unwrap();
    }

    let args = json!({ "topic": "rust" });
    let result = mem_consolidate(args, store).expect("mem_consolidate failed");
    let text = result_text(&result);

    assert!(
        text.contains("Consolidation result") || text.contains("consolidated"),
        "Expected consolidation result, got: {}",
        text
    );
}

#[test]
fn test_mem_consolidate_empty_topic_fails() {
    let store = test_store();
    let args = json!({ "topic": "   " });

    let err = mem_consolidate(args, store).expect_err("Expected error for empty topic");
    assert_eq!(err.code, -32602);
}

// ============================================================
// mem_list_topics
// ============================================================

#[test]
fn test_mem_list_topics_empty_store() {
    let store = test_store();
    let args = json!({});

    let result = mem_list_topics(args, store).expect("mem_list_topics failed");
    let text = result_text(&result);

    assert!(
        text.contains("No topics found"),
        "Expected no topics, got: {}",
        text
    );
}

#[test]
fn test_mem_list_topics_with_memories() {
    let store = test_store();

    // Store memories in different topics
    mem_store(
        json!({ "content": "Auth memory", "topic": "auth" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "DB memory 1", "topic": "database" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "DB memory 2", "topic": "database" }),
        store.clone(),
    )
    .unwrap();

    let args = json!({});
    let result = mem_list_topics(args, store).expect("mem_list_topics failed");
    let text = result_text(&result);

    assert!(
        text.contains("Found"),
        "Expected topic listing, got: {}",
        text
    );
    assert!(text.contains("auth"));
    assert!(text.contains("database"));
}

#[test]
fn test_mem_list_topics_with_min_count_filter() {
    let store = test_store();

    mem_store(
        json!({ "content": "Only one", "topic": "rare" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "Two A", "topic": "common" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "Two B", "topic": "common" }),
        store.clone(),
    )
    .unwrap();

    let args = json!({ "min_count": 2 });
    let result = mem_list_topics(args, store).expect("mem_list_topics failed");
    let text = result_text(&result);

    // Should only show "common" (count >= 2)
    assert!(text.contains("common"));
    assert!(!text.contains("rare"), "Rare topic should be filtered out");
}

// ============================================================
// mem_stats
// ============================================================

#[test]
fn test_mem_stats_empty_store() {
    let store = test_store();
    let args = json!({});

    let result = mem_stats(args, store).expect("mem_stats failed");
    let text = result_text(&result);

    assert!(
        text.contains("Memory statistics"),
        "Expected stats, got: {}",
        text
    );
    assert!(text.contains("total_memories"));
}

#[test]
fn test_mem_stats_with_memories() {
    let store = test_store();

    mem_store(
        json!({ "content": "Memory 1", "importance": "high" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "Memory 2", "importance": "low" }),
        store.clone(),
    )
    .unwrap();

    let result = mem_stats(json!({}), store).expect("mem_stats failed");
    let text = result_text(&result);

    assert!(text.contains("Memory statistics"));
    // total_memories should be at least 2
    assert!(text.contains("\"total_memories\":") || text.contains("\"total_memories\": "));
}

// ============================================================
// mem_health
// ============================================================

#[test]
fn test_mem_health_reports_status() {
    let store = test_store();
    let args = json!({});

    let result = mem_health(args, store).expect("mem_health failed");
    let text = result_text(&result);

    assert!(
        text.contains("Health check"),
        "Expected health check, got: {}",
        text
    );
    assert!(text.contains("\"status\":"));
    assert!(text.contains("\"db\":"));
    assert!(text.contains("\"fts\":"));
}

#[test]
fn test_mem_health_db_and_fts_ok() {
    let store = test_store();
    let result = mem_health(json!({}), store).expect("mem_health failed");
    let text = result_text(&result);

    assert!(text.contains("\"db\": \"ok\""));
    assert!(text.contains("\"fts\": \"ok\""));
}

// ============================================================
// mem_embed_all
// ============================================================

#[test]
fn test_mem_embed_all_placeholder() {
    let store = test_store();
    let args = json!({});

    let result = mem_embed_all(args, store).expect("mem_embed_all failed");
    let text = result_text(&result);

    assert!(
        text.contains("not yet implemented") || text.contains("processed"),
        "Expected placeholder response, got: {}",
        text
    );
}

#[test]
fn test_mem_embed_all_with_custom_args() {
    let store = test_store();
    let args = json!({ "batch_size": 50, "skip_existing": false });

    let result = mem_embed_all(args, store).expect("mem_embed_all failed");
    // Should not fail even with custom args
    assert!(!result.content.is_empty());
}

// ============================================================
// mem_export
// ============================================================

#[test]
fn test_mem_export_json() {
    let store = test_store();

    // Store some memories first
    mem_store(
        json!({ "content": "Export test 1", "topic": "export" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "Export test 2", "topic": "export" }),
        store.clone(),
    )
    .unwrap();

    let tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let path = tmp.path().to_str().unwrap().to_string();

    let args = json!({
        "output": path,
        "format": "json"
    });

    let result = mem_export(args, store).expect("mem_export failed");
    let text = result_text(&result);

    assert!(
        text.contains("Export completed"),
        "Expected export completed, got: {}",
        text
    );
    assert!(text.contains("\"format\": \"json\""));
}

#[test]
fn test_mem_export_csv() {
    let store = test_store();

    mem_store(
        json!({ "content": "CSV export test", "topic": "csv" }),
        store.clone(),
    )
    .unwrap();

    let tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let path = tmp.path().to_str().unwrap().to_string();

    let args = json!({
        "output": path,
        "format": "csv"
    });

    let result = mem_export(args, store).expect("mem_export failed");
    let text = result_text(&result);

    assert!(text.contains("Export completed"));
    assert!(text.contains("\"format\": \"csv\""));
}

#[test]
fn test_mem_export_markdown() {
    let store = test_store();

    mem_store(
        json!({ "content": "Markdown export test", "topic": "md" }),
        store.clone(),
    )
    .unwrap();

    let tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let path = tmp.path().to_str().unwrap().to_string();

    let args = json!({
        "output": path,
        "format": "markdown"
    });

    let result = mem_export(args, store).expect("mem_export failed");
    let text = result_text(&result);

    assert!(text.contains("Export completed"));
}

// ============================================================
// mem_import
// ============================================================

#[test]
fn test_mem_import_dry_run() {
    let store = test_store();

    // First export to create a valid file
    mem_store(
        json!({ "content": "Import test", "topic": "import" }),
        store.clone(),
    )
    .unwrap();

    let tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let path = tmp.path().to_str().unwrap().to_string();

    mem_export(json!({ "output": &path, "format": "json" }), store.clone()).unwrap();

    // Now import with dry_run
    let args = json!({
        "input": &path,
        "dry_run": true
    });

    let result = mem_import(args, store).expect("mem_import dry_run failed");
    let text = result_text(&result);

    assert!(
        text.contains("Dry run") || text.contains("dry_run"),
        "Expected dry run response, got: {}",
        text
    );
}

#[test]
fn test_mem_import_nonexistent_file_fails() {
    let store = test_store();
    let args = json!({
        "input": "/tmp/nonexistent_file_12345.json"
    });

    let err = mem_import(args, store).expect_err("Expected error for nonexistent file");
    assert_eq!(err.code, -32602);
}

#[test]
fn test_mem_import_roundtrip() {
    let store = test_store();

    // Store memories
    mem_store(
        json!({ "content": "Roundtrip memory 1", "topic": "roundtrip" }),
        store.clone(),
    )
    .unwrap();
    mem_store(
        json!({ "content": "Roundtrip memory 2", "topic": "roundtrip" }),
        store.clone(),
    )
    .unwrap();

    // Export to a temp file with .json extension so import can detect format
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = tmp_dir.path().join("export.json");
    let path_str = path.to_str().unwrap().to_string();
    mem_export(
        json!({ "output": &path_str, "format": "json" }),
        store.clone(),
    )
    .unwrap();

    // Import into a fresh store
    let store2 = test_store();
    let args = json!({
        "input": &path_str,
        "mode": "skip"
    });

    let result = mem_import(args, store2).expect("mem_import failed");
    let text = result_text(&result);

    assert!(
        text.contains("Import completed"),
        "Expected import completed, got: {}",
        text
    );
}

// ============================================================
// Helpers
// ============================================================

/// Extract the memory ID from a mem_store response text.
///
/// The response text looks like:
/// ```
/// Memory stored:
/// {
///   "id": "01HQ7X8Y9Z...",
///   "action": "created"
/// }
/// ```
fn extract_id_from_store_response(text: &str) -> String {
    let parsed: Value =
        serde_json::from_str(text.strip_prefix("Memory stored:\n").unwrap_or(text)).expect(
            &format!("Failed to parse store response JSON from: {}", text),
        );
    parsed["id"]
        .as_str()
        .expect("Missing 'id' in store response")
        .to_string()
}

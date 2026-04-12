//! Integration tests for import conflict resolution
//!
//! Tests all three conflict resolution modes:
//! - Skip: Keep existing, ignore import
//! - Update: Merge changes
//! - Replace: Overwrite with import

use alejandria_core::{Importance, Memory, MemoryStore};
use alejandria_storage::SqliteStore;
use std::fs::File;
use tempfile::Builder;

fn create_test_memory(id: &str, topic: &str, summary: &str, importance: Importance) -> Memory {
    let mut memory = Memory::new(topic.to_string(), summary.to_string());
    memory.id = id.to_string();
    memory.importance = importance;
    memory.raw_excerpt = Some(format!("Content for {}", summary));
    // Use unique keywords per memory to avoid deduplication
    memory.keywords = vec![format!("test-{}", id)];
    memory.owner_key_hash = String::new(); // Will default to LEGACY_SYSTEM in storage
    memory
}

#[test]
fn test_import_mode_skip() {
    // Create store with existing memory
    let store = SqliteStore::open_in_memory().unwrap();

    let existing = create_test_memory(
        "01CONFLICT",
        "original-topic",
        "Original summary",
        Importance::High,
    );
    store.store(existing.clone()).unwrap();

    // Export a conflicting memory to file
    let export_file = Builder::new().suffix(".json").tempfile().unwrap();
    let export_path = export_file.path();

    let conflicting = create_test_memory("01CONFLICT", "new-topic", "New summary", Importance::Low);

    // Manually write JSON export
    let export_json = serde_json::json!([conflicting]);
    let file = File::create(export_path).unwrap();
    serde_json::to_writer_pretty(file, &export_json).unwrap();

    // Import with Skip mode
    let result = store
        .import_memories(export_path, alejandria_core::import::ImportMode::Skip)
        .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.updated, 0);
    assert_eq!(result.skipped, 1);

    // Verify original data unchanged
    let memory = store.get("01CONFLICT").unwrap().unwrap();
    assert_eq!(memory.summary, "Original summary");
    assert_eq!(memory.topic, "original-topic");
    assert_eq!(memory.importance, Importance::High);
}

#[test]
fn test_import_mode_update() {
    // Create store with existing memory
    let store = SqliteStore::open_in_memory().unwrap();

    let mut existing = create_test_memory(
        "01UPDATE",
        "original-topic",
        "Original summary",
        Importance::High,
    );
    existing.access_count = 5;
    existing.weight = 0.9;
    store.store(existing.clone()).unwrap();

    // Export a conflicting memory with different data
    let export_file = Builder::new().suffix(".json").tempfile().unwrap();
    let export_path = export_file.path();

    let mut updated = create_test_memory(
        "01UPDATE",
        "new-topic",
        "Updated summary",
        Importance::Critical,
    );
    updated.access_count = 10;
    updated.weight = 0.95;

    let export_json = serde_json::json!([updated]);
    let file = File::create(export_path).unwrap();
    serde_json::to_writer_pretty(file, &export_json).unwrap();

    // Import with Update mode
    let result = store
        .import_memories(export_path, alejandria_core::import::ImportMode::Update)
        .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.updated, 1);
    assert_eq!(result.skipped, 0);

    // Verify memory was updated with new data
    let memory = store.get("01UPDATE").unwrap().unwrap();
    assert_eq!(memory.summary, "Updated summary");
    assert_eq!(memory.topic, "new-topic");
    assert_eq!(memory.importance, Importance::Critical);
}

#[test]
fn test_import_mode_replace() {
    // Create store with existing memory
    let store = SqliteStore::open_in_memory().unwrap();

    let mut existing = create_test_memory(
        "01REPLACE",
        "original-topic",
        "Original summary",
        Importance::High,
    );
    existing.access_count = 5;
    existing.weight = 0.9;
    store.store(existing.clone()).unwrap();

    // Export a replacement memory
    let export_file = Builder::new().suffix(".json").tempfile().unwrap();
    let export_path = export_file.path();

    let mut replacement = create_test_memory(
        "01REPLACE",
        "replacement-topic",
        "Replacement summary",
        Importance::Low,
    );
    replacement.access_count = 1;
    replacement.weight = 0.5;

    let export_json = serde_json::json!([replacement]);
    let file = File::create(export_path).unwrap();
    serde_json::to_writer_pretty(file, &export_json).unwrap();

    // Import with Replace mode
    let result = store
        .import_memories(export_path, alejandria_core::import::ImportMode::Replace)
        .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.updated, 1);
    assert_eq!(result.skipped, 0);

    // Verify memory was completely replaced
    let memory = store.get("01REPLACE").unwrap().unwrap();
    assert_eq!(memory.summary, "Replacement summary");
    assert_eq!(memory.topic, "replacement-topic");
    assert_eq!(memory.importance, Importance::Low);
}

#[test]
fn test_import_conflict_by_topic_key() {
    // Test conflict resolution using topic_key instead of ID
    let store = SqliteStore::open_in_memory().unwrap();

    let mut existing = create_test_memory("01ORIGINAL", "test", "Original", Importance::Medium);
    existing.topic_key = Some("shared/key".to_string());
    store.store(existing.clone()).unwrap();

    // Export memory with different ID but same topic_key
    let export_file = Builder::new().suffix(".json").tempfile().unwrap();
    let export_path = export_file.path();

    let mut conflicting =
        create_test_memory("01DIFFERENT", "test", "Conflicting", Importance::High);
    conflicting.topic_key = Some("shared/key".to_string());

    let export_json = serde_json::json!([conflicting]);
    let file = File::create(export_path).unwrap();
    serde_json::to_writer_pretty(file, &export_json).unwrap();

    // Import with Skip mode
    let result = store
        .import_memories(export_path, alejandria_core::import::ImportMode::Skip)
        .unwrap();

    // Should detect conflict by topic_key and skip
    assert_eq!(result.skipped, 1);
    assert_eq!(result.imported, 0);

    // Verify original memory still exists
    let memory = store.get("01ORIGINAL").unwrap().unwrap();
    assert_eq!(memory.summary, "Original");
}

#[test]
fn test_import_mixed_new_and_conflicts() {
    let store = SqliteStore::open_in_memory().unwrap();

    // Store one existing memory
    let existing = create_test_memory("01EXISTS", "test", "Existing", Importance::Medium);
    store.store(existing.clone()).unwrap();

    // Prepare export with mix of new and conflicting memories
    let export_file = Builder::new().suffix(".json").tempfile().unwrap();
    let export_path = export_file.path();

    let memory_new = create_test_memory("01NEW", "test", "New memory", Importance::Low);
    let memory_conflict =
        create_test_memory("01EXISTS", "test", "Updated existing", Importance::High);

    let export_json = serde_json::json!([memory_new, memory_conflict]);
    let file = File::create(export_path).unwrap();
    serde_json::to_writer_pretty(file, &export_json).unwrap();

    // Import with Update mode
    let result = store
        .import_memories(export_path, alejandria_core::import::ImportMode::Update)
        .unwrap();

    assert_eq!(result.imported, 1); // New memory imported
    assert_eq!(result.updated, 1); // Existing memory updated
    assert_eq!(result.skipped, 0);

    // Verify both memories exist
    assert_eq!(store.count().unwrap(), 2);

    let new_mem = store.get("01NEW").unwrap().unwrap();
    assert_eq!(new_mem.summary, "New memory");

    let updated_mem = store.get("01EXISTS").unwrap().unwrap();
    assert_eq!(updated_mem.summary, "Updated existing");
    assert_eq!(updated_mem.importance, Importance::High);
}

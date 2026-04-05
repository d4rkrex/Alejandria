//! Integration tests for export/import roundtrip
//!
//! Tests the complete workflow:
//! 1. Store memories
//! 2. Export to file
//! 3. Clear database
//! 4. Import from file
//! 5. Verify all data restored correctly

use alejandria_core::{Importance, Memory, MemoryStore};
use alejandria_storage::{ExportFormat, ExportOptions, SqliteStore};
use std::fs::File;
use std::io::BufWriter;
use tempfile::NamedTempFile;

fn create_test_memory(id: &str, topic: &str, summary: &str, importance: Importance) -> Memory {
    let mut memory = Memory::new(topic.to_string(), summary.to_string());
    memory.id = id.to_string();
    memory.importance = importance;
    memory.raw_excerpt = Some(format!("Content for {}", summary));
    // Use unique keywords per memory to avoid deduplication
    memory.keywords = vec![format!("test-{}", id)];
    memory
}

#[test]
fn test_json_export_import_roundtrip() {
    let store = SqliteStore::open_in_memory().unwrap();
    let memory1 = create_test_memory("01TEST1", "test-topic", "Test memory 1", Importance::High);
    let memory2 = create_test_memory("01TEST2", "test-topic", "Test memory 2", Importance::Medium);
    store.store(memory1.clone()).unwrap();
    store.store(memory2.clone()).unwrap();
    assert_eq!(store.count().unwrap(), 2);

    let export_file = NamedTempFile::with_suffix(".json").unwrap();
    let file = File::create(export_file.path()).unwrap();
    let writer = BufWriter::new(file);
    store
        .export_memories(ExportFormat::Json, ExportOptions::default(), writer)
        .unwrap();

    let store2 = SqliteStore::open_in_memory().unwrap();
    let import_result = store2
        .import_memories(
            export_file.path(),
            alejandria_core::import::ImportMode::Skip,
        )
        .unwrap();

    assert_eq!(import_result.imported, 2);
    assert_eq!(import_result.errors.len(), 0);
    assert_eq!(store2.count().unwrap(), 2);

    let restored1 = store2.get("01TEST1").unwrap().unwrap();
    assert_eq!(restored1.summary, "Test memory 1");
    assert_eq!(restored1.importance, Importance::High);
}

#[test]
fn test_csv_export_import_roundtrip() {
    let store = SqliteStore::open_in_memory().unwrap();
    let memory = create_test_memory("01TESTCSV", "csv-test", "CSV test memory", Importance::Low);
    store.store(memory.clone()).unwrap();

    let export_file = NamedTempFile::with_suffix(".csv").unwrap();
    let file = File::create(export_file.path()).unwrap();
    let writer = BufWriter::new(file);
    store
        .export_memories(ExportFormat::Csv, ExportOptions::default(), writer)
        .unwrap();

    let store2 = SqliteStore::open_in_memory().unwrap();
    let import_result = store2
        .import_memories(
            export_file.path(),
            alejandria_core::import::ImportMode::Skip,
        )
        .unwrap();

    assert_eq!(import_result.imported, 1);
    assert_eq!(import_result.errors.len(), 0);

    let restored = store2.get("01TESTCSV").unwrap().unwrap();
    assert_eq!(restored.summary, "CSV test memory");
}

#[test]
fn test_export_with_filters() {
    let store = SqliteStore::open_in_memory().unwrap();
    for i in 0..5 {
        let importance = if i < 2 {
            Importance::Critical
        } else {
            Importance::Low
        };
        let memory = create_test_memory(
            &format!("01TEST{}", i),
            "test",
            &format!("Memory {}", i),
            importance,
        );
        store.store(memory).unwrap();
    }

    let export_file = NamedTempFile::with_suffix(".json").unwrap();
    let file = File::create(export_file.path()).unwrap();
    let writer = BufWriter::new(file);
    let options = ExportOptions {
        importance_threshold: Some("critical".to_string()),
        ..Default::default()
    };
    let metadata = store
        .export_memories(ExportFormat::Json, options, writer)
        .unwrap();
    assert_eq!(metadata.total_count, 2);

    let store2 = SqliteStore::open_in_memory().unwrap();
    let import_result = store2
        .import_memories(
            export_file.path(),
            alejandria_core::import::ImportMode::Skip,
        )
        .unwrap();
    assert_eq!(import_result.imported, 2);
}

#[test]
fn test_export_include_deleted() {
    let store = SqliteStore::open_in_memory().unwrap();
    let memory = create_test_memory("01DELETED", "test", "Deleted memory", Importance::Medium);
    store.store(memory).unwrap();
    store.delete("01DELETED").unwrap();

    let export_file1 = NamedTempFile::with_suffix(".json").unwrap();
    let file = File::create(export_file1.path()).unwrap();
    let writer = BufWriter::new(file);
    let metadata1 = store
        .export_memories(ExportFormat::Json, ExportOptions::default(), writer)
        .unwrap();
    assert_eq!(metadata1.total_count, 0);

    let export_file2 = NamedTempFile::with_suffix(".json").unwrap();
    let file = File::create(export_file2.path()).unwrap();
    let writer = BufWriter::new(file);
    let options = ExportOptions {
        include_deleted: true,
        ..Default::default()
    };
    let metadata2 = store
        .export_memories(ExportFormat::Json, options, writer)
        .unwrap();
    assert_eq!(metadata2.total_count, 1);
}

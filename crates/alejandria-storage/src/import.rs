//! Import implementation for SqliteStore.
//!
//! Provides functionality to import memories from JSON and CSV formats,
//! with conflict resolution and validation.

use alejandria_core::{
    error::{IcmError, IcmResult},
    import::{conflict, validate, ImportMode, ImportResult},
    memory::Memory,
    store::MemoryStore,
};
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::SqliteStore;

/// Import format detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Json,
    Csv,
}

impl ImportFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> IcmResult<Self> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("json") => Ok(ImportFormat::Json),
            Some("csv") => Ok(ImportFormat::Csv),
            Some(ext) => Err(IcmError::InvalidInput(format!(
                "Unsupported import format: .{}",
                ext
            ))),
            None => Err(IcmError::InvalidInput(
                "Cannot detect format: no file extension".to_string(),
            )),
        }
    }
}

impl SqliteStore {
    /// Import memories from a file
    pub fn import_memories(&self, input_path: &Path, mode: ImportMode) -> IcmResult<ImportResult> {
        let format = ImportFormat::from_path(input_path)?;

        match format {
            ImportFormat::Json => self.import_json(input_path, mode),
            ImportFormat::Csv => self.import_csv(input_path, mode),
        }
    }

    /// Import from JSON format
    fn import_json(&self, input_path: &Path, mode: ImportMode) -> IcmResult<ImportResult> {
        let file = File::open(input_path)
            .map_err(|e| IcmError::InvalidInput(format!("Failed to open file: {}", e)))?;

        let reader = BufReader::new(file);
        let json_value: Value = serde_json::from_reader(reader)
            .map_err(|e| IcmError::InvalidInput(format!("Failed to parse JSON: {}", e)))?;

        let mut result = ImportResult::new();

        // Handle array, single object, or metadata-envelope format
        let memories: Vec<Value> = match json_value {
            Value::Array(arr) => arr,
            Value::Object(ref obj) if obj.contains_key("memories") => {
                // Metadata-envelope format: { "metadata": {...}, "memories": [...] }
                match obj.get("memories") {
                    Some(Value::Array(arr)) => arr.clone(),
                    _ => {
                        result.add_error("'memories' field must be an array".to_string());
                        return Ok(result);
                    }
                }
            }
            Value::Object(_) => vec![json_value],
            _ => {
                result.add_error("JSON must be an array or object".to_string());
                return Ok(result);
            }
        };

        for (idx, memory_json) in memories.into_iter().enumerate() {
            // Validate JSON schema
            if let Err(e) = validate::validate_json_schema(&memory_json) {
                result.add_error(format!("Record {}: {}", idx + 1, e));
                continue;
            }

            // Deserialize to Memory
            let memory: Memory = match serde_json::from_value(memory_json) {
                Ok(m) => m,
                Err(e) => {
                    result.add_error(format!("Record {}: Failed to deserialize: {}", idx + 1, e));
                    continue;
                }
            };

            // Validate memory fields
            if let Err(e) = validate::validate_memory(&memory) {
                result.add_error(format!("Record {}: {}", idx + 1, e));
                continue;
            }

            // Process memory with conflict resolution
            if let Err(e) = self.process_import_memory(memory, mode, &mut result) {
                result.add_error(format!("Record {}: {}", idx + 1, e));
            }
        }

        Ok(result)
    }

    /// Import from CSV format
    fn import_csv(&self, input_path: &Path, mode: ImportMode) -> IcmResult<ImportResult> {
        let file = File::open(input_path)
            .map_err(|e| IcmError::InvalidInput(format!("Failed to open file: {}", e)))?;

        let mut reader = csv::Reader::from_reader(file);
        let mut result = ImportResult::new();

        // Get headers to map columns
        let headers = reader
            .headers()
            .map_err(|e| IcmError::InvalidInput(format!("Failed to read CSV headers: {}", e)))?
            .clone();

        for (idx, record) in reader.records().enumerate() {
            match record {
                Ok(row) => {
                    let memory = match parse_csv_row(&headers, &row) {
                        Ok(m) => m,
                        Err(e) => {
                            result.add_error(format!("Row {}: {}", idx + 2, e));
                            continue;
                        }
                    };

                    // Validate memory fields
                    if let Err(e) = validate::validate_memory(&memory) {
                        result.add_error(format!("Row {}: {}", idx + 2, e));
                        continue;
                    }

                    // Process memory with conflict resolution
                    if let Err(e) = self.process_import_memory(memory, mode, &mut result) {
                        result.add_error(format!("Row {}: {}", idx + 2, e));
                    }
                }
                Err(e) => {
                    result.add_error(format!("Row {}: Failed to parse CSV: {}", idx + 2, e));
                }
            }
        }

        Ok(result)
    }

    /// Process a single memory with conflict resolution
    fn process_import_memory(
        &self,
        memory: Memory,
        mode: ImportMode,
        result: &mut ImportResult,
    ) -> IcmResult<()> {
        // Check for existing memory by ID
        let existing_by_id = self.get(&memory.id)?;

        // Check for existing memory by topic_key
        let existing_by_topic = if let Some(topic_key) = &memory.topic_key {
            self.get_by_topic_key(topic_key)?
        } else {
            None
        };

        // Determine if there's a conflict
        let existing = existing_by_id.or(existing_by_topic);

        match existing {
            None => {
                // No conflict, insert new memory
                self.store(memory)?;
                result.add_imported();
            }
            Some(existing_memory) => {
                // Conflict detected, resolve based on mode
                let resolved = conflict::resolve_conflict(memory, existing_memory, mode)?;

                match resolved {
                    conflict::ResolvedMemory::Skipped(_) => {
                        result.add_skipped();
                    }
                    conflict::ResolvedMemory::Updated(updated) => {
                        self.update(updated)?;
                        result.add_updated();
                    }
                    conflict::ResolvedMemory::Replaced(replaced) => {
                        self.update(replaced)?;
                        result.add_updated();
                    }
                }
            }
        }

        Ok(())
    }
}

/// Parse a CSV row into a Memory using column headers for mapping.
///
/// Handles the export format columns:
/// id, topic, summary, importance, source, keywords, created_at, updated_at,
/// last_accessed, access_count, weight, decay_profile, topic_key, revision_count
fn parse_csv_row(headers: &csv::StringRecord, row: &csv::StringRecord) -> IcmResult<Memory> {
    // Helper to get a column value by header name
    let get_col = |name: &str| -> Option<&str> {
        headers
            .iter()
            .position(|h| h == name)
            .and_then(|i| row.get(i))
    };

    let id = get_col("id")
        .ok_or_else(|| IcmError::InvalidInput("Missing 'id' column".to_string()))?
        .to_string();
    let topic = get_col("topic")
        .ok_or_else(|| IcmError::InvalidInput("Missing 'topic' column".to_string()))?
        .to_string();
    let summary = get_col("summary")
        .ok_or_else(|| IcmError::InvalidInput("Missing 'summary' column".to_string()))?
        .to_string();

    let importance = get_col("importance")
        .unwrap_or("medium")
        .parse()
        .unwrap_or(alejandria_core::memory::Importance::Medium);

    let keywords: Vec<String> = get_col("keywords")
        .unwrap_or("")
        .split("; ")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let access_count: u32 = get_col("access_count").unwrap_or("0").parse().unwrap_or(0);

    let weight: f32 = get_col("weight").unwrap_or("1.0").parse().unwrap_or(1.0);

    let revision_count: u32 = get_col("revision_count")
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);

    let created_at = get_col("created_at")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    let updated_at = get_col("updated_at")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    let last_accessed = get_col("last_accessed")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    let decay_profile = get_col("decay_profile")
        .filter(|s| !s.is_empty() && *s != "none")
        .map(|s| s.to_string());

    let topic_key = get_col("topic_key")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let mut memory = Memory::new(topic, summary);
    memory.id = id;
    memory.importance = importance;
    memory.keywords = keywords;
    memory.access_count = access_count;
    memory.weight = weight;
    memory.revision_count = revision_count;
    memory.created_at = created_at;
    memory.updated_at = updated_at;
    memory.last_accessed = last_accessed;
    memory.decay_profile = decay_profile;
    memory.topic_key = topic_key;

    Ok(memory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    #[test]
    fn test_import_format_from_path() {
        let json_path = Path::new("test.json");
        assert_eq!(
            ImportFormat::from_path(json_path).unwrap(),
            ImportFormat::Json
        );

        let csv_path = Path::new("test.csv");
        assert_eq!(
            ImportFormat::from_path(csv_path).unwrap(),
            ImportFormat::Csv
        );

        let invalid_path = Path::new("test.txt");
        assert!(ImportFormat::from_path(invalid_path).is_err());

        let no_ext_path = Path::new("test");
        assert!(ImportFormat::from_path(no_ext_path).is_err());
    }

    #[test]
    fn test_import_json_empty_array() {
        let store = SqliteStore::open_in_memory().unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"[]").unwrap();
        temp_file.flush().unwrap();

        let result = store
            .import_json(temp_file.path(), ImportMode::Skip)
            .unwrap();
        assert_eq!(result.total, 0);
        assert_eq!(result.imported, 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_import_json_invalid_json() {
        let store = SqliteStore::open_in_memory().unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"not valid json").unwrap();
        temp_file.flush().unwrap();

        let result = store.import_json(temp_file.path(), ImportMode::Skip);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_json_single_valid_memory() {
        let store = SqliteStore::open_in_memory().unwrap();

        let memory = Memory::new("test topic".to_string(), "test summary".to_string());
        let json = serde_json::to_string(&memory).unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = store
            .import_json(temp_file.path(), ImportMode::Skip)
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.imported, 1);
        assert_eq!(result.errors.len(), 0);

        // Verify memory was stored
        let stored = store.get(&memory.id).unwrap().unwrap();
        assert_eq!(stored.topic, "test topic");
    }

    #[test]
    fn test_import_json_array_valid_memories() {
        let store = SqliteStore::open_in_memory().unwrap();

        let memory1 = Memory::new("topic1".to_string(), "summary1".to_string());
        let memory2 = Memory::new("topic2".to_string(), "summary2".to_string());
        let json = serde_json::to_string(&vec![&memory1, &memory2]).unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = store
            .import_json(temp_file.path(), ImportMode::Skip)
            .unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.imported, 2);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_import_json_conflict_skip() {
        let store = SqliteStore::open_in_memory().unwrap();

        // Insert initial memory
        let memory = Memory::new("original".to_string(), "original summary".to_string());
        store.store(memory.clone()).unwrap();

        // Try to import same ID with different content
        let mut updated = memory.clone();
        updated.topic = "updated".to_string();
        let json = serde_json::to_string(&updated).unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = store
            .import_json(temp_file.path(), ImportMode::Skip)
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.imported, 0);

        // Verify original was preserved
        let stored = store.get(&memory.id).unwrap().unwrap();
        assert_eq!(stored.topic, "original");
    }

    #[test]
    fn test_import_json_conflict_update() {
        let store = SqliteStore::open_in_memory().unwrap();

        // Insert initial memory
        let memory = Memory::new("original".to_string(), "original summary".to_string());
        let original_created_at = memory.created_at;
        store.store(memory.clone()).unwrap();

        // Import same ID with updated content
        let mut updated = memory.clone();
        updated.topic = "updated".to_string();
        let json = serde_json::to_string(&updated).unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = store
            .import_json(temp_file.path(), ImportMode::Update)
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.updated, 1);
        assert_eq!(result.imported, 0);

        // Verify memory was updated
        let stored = store.get(&memory.id).unwrap().unwrap();
        assert_eq!(stored.topic, "updated");
        // Check timestamp is close (within 1 second due to SQLite precision)
        let time_diff =
            (stored.created_at.timestamp_millis() - original_created_at.timestamp_millis()).abs();
        assert!(
            time_diff < 1000,
            "created_at should be preserved (diff: {}ms)",
            time_diff
        );
    }

    #[test]
    fn test_import_json_conflict_replace() {
        let store = SqliteStore::open_in_memory().unwrap();

        // Insert initial memory
        let memory = Memory::new("original".to_string(), "original summary".to_string());
        store.store(memory.clone()).unwrap();

        // Import same ID with completely new data
        let mut replaced = memory.clone();
        replaced.topic = "replaced".to_string();
        replaced.created_at = chrono::Utc::now(); // Different timestamp
        let json = serde_json::to_string(&replaced).unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = store
            .import_json(temp_file.path(), ImportMode::Replace)
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.updated, 1);
        assert_eq!(result.imported, 0);

        // Verify memory was replaced (created_at should be from imported)
        let stored = store.get(&memory.id).unwrap().unwrap();
        assert_eq!(stored.topic, "replaced");
        // Check timestamp is close (within 1 second due to SQLite precision)
        let time_diff =
            (stored.created_at.timestamp_millis() - replaced.created_at.timestamp_millis()).abs();
        assert!(
            time_diff < 1000,
            "created_at should match replaced (diff: {}ms)",
            time_diff
        );
    }
}

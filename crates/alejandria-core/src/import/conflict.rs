//! Conflict resolution logic for importing memories.
//!
//! Handles conflicts when imported data matches existing observations
//! by ID or topic_key, applying different resolution strategies.

use crate::error::IcmResult;
use crate::import::ImportMode;
use crate::memory::Memory;

/// Conflict detected during import
#[derive(Debug, Clone)]
pub struct Conflict {
    /// The imported memory causing the conflict
    pub imported: Memory,
    /// The existing memory in the database
    pub existing: Memory,
    /// Type of conflict
    pub conflict_type: ConflictType,
}

/// Type of conflict detected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// ID matches existing observation
    IdConflict,
    /// topic_key matches existing observation
    TopicKeyConflict,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictType::IdConflict => write!(f, "ID conflict"),
            ConflictType::TopicKeyConflict => write!(f, "topic_key conflict"),
        }
    }
}

/// Resolve a conflict between imported and existing memory
pub fn resolve_conflict(
    imported: Memory,
    existing: Memory,
    mode: ImportMode,
) -> IcmResult<ResolvedMemory> {
    match mode {
        ImportMode::Skip => Ok(ResolvedMemory::Skipped(existing)),
        ImportMode::Update => merge_memories(imported, existing),
        ImportMode::Replace => replace_memory(imported, existing),
    }
}

/// Result of conflict resolution
#[derive(Debug, Clone)]
pub enum ResolvedMemory {
    /// Memory was skipped (kept existing)
    Skipped(Memory),
    /// Memory was updated (merged)
    Updated(Memory),
    /// Memory was replaced
    Replaced(Memory),
}

/// Merge imported memory with existing, preserving ID and creation timestamp
fn merge_memories(imported: Memory, mut existing: Memory) -> IcmResult<ResolvedMemory> {
    // Update fields from imported, but preserve:
    // - id (database primary key)
    // - created_at (original creation time)
    // - revision_count (will be incremented by upsert logic)

    existing.topic = imported.topic;
    existing.summary = imported.summary;
    existing.raw_excerpt = imported.raw_excerpt;
    existing.keywords = imported.keywords;
    existing.embedding = imported.embedding;
    existing.importance = imported.importance;
    existing.source = imported.source;
    existing.related_ids = imported.related_ids;
    existing.topic_key = imported.topic_key;
    existing.weight = imported.weight;
    existing.decay_profile = imported.decay_profile;
    existing.decay_params = imported.decay_params;

    // Update timestamps
    existing.updated_at = chrono::Utc::now();
    existing.last_accessed = imported.last_accessed;
    existing.last_seen_at = imported.last_seen_at;
    existing.deleted_at = imported.deleted_at;

    // Update counters
    existing.access_count = imported.access_count;
    existing.duplicate_count = imported.duplicate_count;

    Ok(ResolvedMemory::Updated(existing))
}

/// Replace existing memory with imported, preserving only ID
fn replace_memory(mut imported: Memory, existing: Memory) -> IcmResult<ResolvedMemory> {
    // Keep the existing ID (database primary key)
    imported.id = existing.id;

    // Everything else comes from imported memory
    Ok(ResolvedMemory::Replaced(imported))
}

/// Check if two memories conflict by ID
pub fn has_id_conflict(imported: &Memory, existing: &Memory) -> bool {
    imported.id == existing.id
}

/// Check if two memories conflict by topic_key
pub fn has_topic_key_conflict(imported: &Memory, existing: &Memory) -> bool {
    if let (Some(imported_key), Some(existing_key)) = (&imported.topic_key, &existing.topic_key) {
        imported_key == existing_key
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Importance;

    fn create_test_memory(id: &str, topic: &str, summary: &str) -> Memory {
        let mut memory = Memory::new(topic.to_string(), summary.to_string());
        memory.id = id.to_string();
        memory
    }

    #[test]
    fn test_conflict_type_display() {
        assert_eq!(ConflictType::IdConflict.to_string(), "ID conflict");
        assert_eq!(
            ConflictType::TopicKeyConflict.to_string(),
            "topic_key conflict"
        );
    }

    #[test]
    fn test_resolve_conflict_skip() {
        let imported = create_test_memory("1", "new topic", "new summary");
        let existing = create_test_memory("1", "old topic", "old summary");

        let result = resolve_conflict(imported, existing.clone(), ImportMode::Skip).unwrap();
        match result {
            ResolvedMemory::Skipped(memory) => {
                assert_eq!(memory.id, "1");
                assert_eq!(memory.topic, "old topic");
            }
            _ => panic!("Expected Skipped variant"),
        }
    }

    #[test]
    fn test_resolve_conflict_update() {
        let imported = create_test_memory("1", "new topic", "new summary");
        let existing = create_test_memory("1", "old topic", "old summary");
        let original_created_at = existing.created_at;

        let result = resolve_conflict(imported, existing, ImportMode::Update).unwrap();
        match result {
            ResolvedMemory::Updated(memory) => {
                assert_eq!(memory.id, "1");
                assert_eq!(memory.topic, "new topic");
                assert_eq!(memory.summary, "new summary");
                assert_eq!(memory.created_at, original_created_at); // Preserved
            }
            _ => panic!("Expected Updated variant"),
        }
    }

    #[test]
    fn test_resolve_conflict_replace() {
        let imported = create_test_memory("99", "new topic", "new summary");
        let existing = create_test_memory("1", "old topic", "old summary");

        let result = resolve_conflict(imported.clone(), existing, ImportMode::Replace).unwrap();
        match result {
            ResolvedMemory::Replaced(memory) => {
                assert_eq!(memory.id, "1"); // Preserved from existing
                assert_eq!(memory.topic, "new topic"); // From imported
                assert_eq!(memory.summary, "new summary"); // From imported
            }
            _ => panic!("Expected Replaced variant"),
        }
    }

    #[test]
    fn test_merge_memories_preserves_id_and_created_at() {
        let imported = create_test_memory("new-id", "new topic", "new summary");
        let mut existing = create_test_memory("existing-id", "old topic", "old summary");
        existing.importance = Importance::Medium;

        let result = merge_memories(imported, existing.clone()).unwrap();
        match result {
            ResolvedMemory::Updated(memory) => {
                assert_eq!(memory.id, "existing-id");
                assert_eq!(memory.created_at, existing.created_at);
                assert_eq!(memory.topic, "new topic");
                assert_eq!(memory.summary, "new summary");
            }
            _ => panic!("Expected Updated variant"),
        }
    }

    #[test]
    fn test_replace_memory_preserves_only_id() {
        let mut imported = create_test_memory("imported-id", "new topic", "new summary");
        imported.importance = Importance::High;
        let mut existing = create_test_memory("existing-id", "old topic", "old summary");
        existing.importance = Importance::Medium;

        let result = replace_memory(imported, existing.clone()).unwrap();
        match result {
            ResolvedMemory::Replaced(memory) => {
                assert_eq!(memory.id, "existing-id"); // Preserved
                assert_eq!(memory.topic, "new topic"); // Replaced
                assert_eq!(memory.importance, Importance::High); // Replaced
            }
            _ => panic!("Expected Replaced variant"),
        }
    }

    #[test]
    fn test_has_id_conflict_true() {
        let mem1 = create_test_memory("same-id", "topic1", "summary1");
        let mem2 = create_test_memory("same-id", "topic2", "summary2");
        assert!(has_id_conflict(&mem1, &mem2));
    }

    #[test]
    fn test_has_id_conflict_false() {
        let mem1 = create_test_memory("id1", "topic1", "summary1");
        let mem2 = create_test_memory("id2", "topic2", "summary2");
        assert!(!has_id_conflict(&mem1, &mem2));
    }

    #[test]
    fn test_has_topic_key_conflict_true() {
        let mut mem1 = create_test_memory("id1", "topic1", "summary1");
        let mut mem2 = create_test_memory("id2", "topic2", "summary2");
        mem1.topic_key = Some("shared-key".to_string());
        mem2.topic_key = Some("shared-key".to_string());
        assert!(has_topic_key_conflict(&mem1, &mem2));
    }

    #[test]
    fn test_has_topic_key_conflict_false_different_keys() {
        let mut mem1 = create_test_memory("id1", "topic1", "summary1");
        let mut mem2 = create_test_memory("id2", "topic2", "summary2");
        mem1.topic_key = Some("key1".to_string());
        mem2.topic_key = Some("key2".to_string());
        assert!(!has_topic_key_conflict(&mem1, &mem2));
    }

    #[test]
    fn test_has_topic_key_conflict_false_no_keys() {
        let mem1 = create_test_memory("id1", "topic1", "summary1");
        let mem2 = create_test_memory("id2", "topic2", "summary2");
        assert!(!has_topic_key_conflict(&mem1, &mem2));
    }

    #[test]
    fn test_has_topic_key_conflict_false_one_key() {
        let mut mem1 = create_test_memory("id1", "topic1", "summary1");
        let mem2 = create_test_memory("id2", "topic2", "summary2");
        mem1.topic_key = Some("key1".to_string());
        assert!(!has_topic_key_conflict(&mem1, &mem2));
    }
}

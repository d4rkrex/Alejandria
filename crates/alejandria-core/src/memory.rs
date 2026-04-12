//! Core memory types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an episodic memory entry with temporal decay and deduplication support.
///
/// Memory entries are the core unit of storage in Alejandria's episodic memory system.
/// Each memory has a unique ULID identifier, content fields (topic, summary), lifecycle
/// tracking (created/updated/accessed timestamps), and support for semantic search via embeddings.
///
/// # Temporal Decay
///
/// Memories have a `weight` field (0.0-1.0) that decays over time based on their importance level:
/// - **Critical**: Never decays (0.0x rate)
/// - **High**: Slow decay (0.5x rate), never pruned  
/// - **Medium**: Normal decay (1.0x rate), prunable at weight < 0.1
/// - **Low**: Fast decay (2.0x rate), prunable at weight < 0.3
///
/// Access counts dampen decay to preserve frequently-used memories.
///
/// # Deduplication
///
/// The `topic_key` field enables semantic deduplication via upsert workflows.
/// When storing a memory with an existing topic_key, the system updates the existing
/// memory instead of creating a duplicate, incrementing `revision_count` and updating
/// `last_seen_at`.
///
/// # Examples
///
/// ```
/// use alejandria_core::{Memory, Importance};
///
/// // Create a new memory
/// let mut memory = Memory::new(
///     "rust-error-handling".to_string(),
///     "Use Result<T, E> for recoverable errors".to_string(),
/// );
/// memory.importance = Importance::High;
/// memory.topic_key = Some("rust/error-handling/result-type".to_string());
///
/// // Mark as accessed
/// memory.mark_accessed();
/// assert_eq!(memory.access_count, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique identifier (ULID format)
    pub id: String,

    /// Timestamps for lifecycle tracking
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,

    /// Access tracking for decay calculations
    pub access_count: u32,

    /// Current weight (0.0 - 1.0) for decay-based pruning
    pub weight: f32,

    /// Content fields
    pub topic: String,
    pub summary: String,
    pub raw_excerpt: Option<String>,
    pub keywords: Vec<String>,

    /// Optional embedding vector (768 dimensions for multilingual-e5-base)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    /// Classification
    pub importance: Importance,
    pub source: MemorySource,

    /// Relations
    pub related_ids: Vec<String>,

    /// Engram-inspired additions for upsert workflow
    pub topic_key: Option<String>,
    pub revision_count: u32,
    pub duplicate_count: u32,
    pub last_seen_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,

    /// Advanced decay strategy fields
    pub decay_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decay_params: Option<serde_json::Value>,

    /// Owner identification (SHA-256 hash of API key that created this memory)
    /// Special values:
    /// - 'shared': Accessible by all users (system-wide knowledge)
    /// - 'LEGACY_SYSTEM': Pre-migration memories (for backward compatibility)
    /// - Otherwise: 64-char hex SHA-256 hash of the creating API key
    pub owner_key_hash: String,
}

/// Memory importance levels affecting decay rates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    /// Never decays, never pruned (decay rate: 0.0)
    Critical,
    /// 0.5x decay rate, never pruned
    High,
    /// 1.0x decay rate (default), prunable at weight < 0.1
    #[default]
    Medium,
    /// 2.0x decay rate, prunable at weight < 0.3
    Low,
}

impl Importance {
    /// Get the decay rate multiplier for this importance level
    pub fn decay_multiplier(&self) -> f32 {
        match self {
            Importance::Critical => 0.0,
            Importance::High => 0.5,
            Importance::Medium => 1.0,
            Importance::Low => 2.0,
        }
    }

    /// Check if memories of this importance can be pruned
    pub fn is_prunable(&self) -> bool {
        matches!(self, Importance::Medium | Importance::Low)
    }
}

impl std::fmt::Display for Importance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Importance::Critical => write!(f, "critical"),
            Importance::High => write!(f, "high"),
            Importance::Medium => write!(f, "medium"),
            Importance::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for Importance {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Importance::Critical),
            "high" => Ok(Importance::High),
            "medium" => Ok(Importance::Medium),
            "low" => Ok(Importance::Low),
            _ => Err(format!("Invalid importance level: {}", s)),
        }
    }
}

/// Source of a memory entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemorySource {
    /// Explicitly added by user
    #[default]
    User,
    /// Created by AI agent (with optional tool name)
    Agent,
    /// System-generated (e.g., consolidation)
    System,
    /// Imported from external source
    External,
}

impl Memory {
    /// Create a new memory with default values.
    ///
    /// Generates a new ULID identifier and initializes timestamps to the current time.
    /// Default values:
    /// - `weight`: 1.0 (maximum)
    /// - `importance`: Medium
    /// - `source`: User
    /// - `access_count`: 0
    /// - `revision_count`: 1
    ///
    /// # Arguments
    ///
    /// * `topic` - High-level category or theme (e.g., "architecture", "bug-fixes")
    /// * `summary` - Main content of the memory
    ///
    /// # Examples
    ///
    /// ```
    /// use alejandria_core::Memory;
    ///
    /// let memory = Memory::new(
    ///     "authentication".to_string(),
    ///     "Implemented JWT-based auth with refresh tokens".to_string(),
    /// );
    ///
    /// assert_eq!(memory.topic, "authentication");
    /// assert_eq!(memory.weight, 1.0);
    /// assert_eq!(memory.access_count, 0);
    /// assert!(!memory.is_deleted());
    /// ```
    pub fn new(topic: String, summary: String) -> Self {
        let now = Utc::now();
        Self {
            id: ulid::Ulid::new().to_string(),
            created_at: now,
            updated_at: now,
            last_accessed: now,
            access_count: 0,
            weight: 1.0,
            topic,
            summary,
            raw_excerpt: None,
            keywords: Vec::new(),
            embedding: None,
            importance: Importance::default(),
            source: MemorySource::default(),
            related_ids: Vec::new(),
            topic_key: None,
            revision_count: 1,
            duplicate_count: 0,
            last_seen_at: now,
            deleted_at: None,
            decay_profile: None,
            decay_params: None,
            owner_key_hash: String::new(), // Will be set by storage layer or handler
        }
    }

    /// Check if this memory is soft-deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Mark this memory as accessed (increments count, updates timestamp)
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Check if this memory is shared (accessible by all users)
    pub fn is_shared(&self) -> bool {
        self.owner_key_hash == "shared"
    }

    /// Check if this memory is a legacy memory (pre-migration, accessible by all)
    pub fn is_legacy(&self) -> bool {
        self.owner_key_hash == "LEGACY_SYSTEM"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let memory = Memory::new("test".to_string(), "Test summary".to_string());

        assert_eq!(memory.topic, "test");
        assert_eq!(memory.summary, "Test summary");
        assert_eq!(memory.weight, 1.0);
        assert_eq!(memory.access_count, 0);
        assert_eq!(memory.importance, Importance::Medium);
        assert!(!memory.is_deleted());
    }

    #[test]
    fn test_importance_decay_multipliers() {
        assert_eq!(Importance::Critical.decay_multiplier(), 0.0);
        assert_eq!(Importance::High.decay_multiplier(), 0.5);
        assert_eq!(Importance::Medium.decay_multiplier(), 1.0);
        assert_eq!(Importance::Low.decay_multiplier(), 2.0);
    }

    #[test]
    fn test_importance_prunable() {
        assert!(!Importance::Critical.is_prunable());
        assert!(!Importance::High.is_prunable());
        assert!(Importance::Medium.is_prunable());
        assert!(Importance::Low.is_prunable());
    }

    #[test]
    fn test_memory_serialization() {
        let memory = Memory::new("test".to_string(), "summary".to_string());

        // Test serde round-trip
        let json = serde_json::to_string(&memory).unwrap();
        let deserialized: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(memory.id, deserialized.id);
        assert_eq!(memory.topic, deserialized.topic);
        assert_eq!(memory.summary, deserialized.summary);
    }

    #[test]
    fn test_importance_serialization() {
        let high = Importance::High;
        let json = serde_json::to_string(&high).unwrap();
        assert_eq!(json, r#""high""#);

        let deserialized: Importance = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Importance::High);
    }

    #[test]
    fn test_memory_source_serialization() {
        let source = MemorySource::Agent;
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: MemorySource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, deserialized);
    }

    #[test]
    fn test_mark_accessed() {
        let mut memory = Memory::new("test".to_string(), "summary".to_string());
        let initial_count = memory.access_count;
        let initial_time = memory.last_accessed;

        std::thread::sleep(std::time::Duration::from_millis(10));
        memory.mark_accessed();

        assert_eq!(memory.access_count, initial_count + 1);
        assert!(memory.last_accessed > initial_time);
    }
}

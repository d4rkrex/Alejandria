//! Memoir store trait and supporting types for knowledge graph operations.
//!
//! This module defines the `MemoirStore` trait for managing memoirs (knowledge graph containers),
//! concepts (graph nodes), and concept links (typed relations between concepts).
//!
//! # Example
//!
//! ```no_run
//! use alejandria_core::{MemoirStore, NewMemoir, NewConcept, NewConceptLink, RelationType};
//! use alejandria_core::error::IcmResult;
//!
//! # fn example(store: &impl MemoirStore) -> IcmResult<()> {
//! // Create a memoir
//! let memoir = store.create_memoir(NewMemoir {
//!     name: "rust-patterns".to_string(),
//!     description: "Common Rust design patterns".to_string(),
//! })?;
//!
//! // Add concepts
//! let builder = store.add_concept(NewConcept {
//!     memoir_name: "rust-patterns".to_string(),
//!     name: "Builder Pattern".to_string(),
//!     definition: "A creational pattern for constructing complex objects".to_string(),
//!     labels: vec!["design-pattern".to_string(), "creational".to_string()],
//! })?;
//!
//! let creational = store.add_concept(NewConcept {
//!     memoir_name: "rust-patterns".to_string(),
//!     name: "Creational Pattern".to_string(),
//!     definition: "Patterns for object creation".to_string(),
//!     labels: vec!["category".to_string()],
//! })?;
//!
//! // Link concepts with typed relation
//! store.link_concepts(NewConceptLink {
//!     memoir_name: "rust-patterns".to_string(),
//!     source_name: "Builder Pattern".to_string(),
//!     target_name: "Creational Pattern".to_string(),
//!     relation: RelationType::IsA,
//!     weight: 1.0,
//! })?;
//!
//! // Inspect concept neighborhood
//! let neighborhood = store.inspect_concept("rust-patterns", "Builder Pattern", 1)?;
//! println!("Found {} neighbors", neighborhood.neighbors.len());
//! # Ok(())
//! # }
//! ```

use crate::{Concept, ConceptLink, IcmResult, Memoir, RelationType};
use serde::{Deserialize, Serialize};

/// Trait for memoir (knowledge graph) storage operations.
///
/// This trait provides methods for creating and managing memoirs, concepts,
/// and the typed relations between them. All methods are thread-safe.
pub trait MemoirStore: Send + Sync {
    /// Create a new memoir.
    ///
    /// # Arguments
    ///
    /// * `memoir` - New memoir data (name must be unique)
    ///
    /// # Returns
    ///
    /// The created `Memoir` with generated ULID and timestamps.
    ///
    /// # Errors
    ///
    /// Returns `IcmError::AlreadyExists` if a memoir with the same name exists.
    fn create_memoir(&self, memoir: NewMemoir) -> IcmResult<Memoir>;

    /// List all memoirs with summary information.
    ///
    /// # Returns
    ///
    /// A vector of `MemoirSummary` objects containing memoir metadata
    /// and aggregated concept/link counts.
    fn list_memoirs(&self) -> IcmResult<Vec<MemoirSummary>>;

    /// Get detailed memoir information including all concepts and links.
    ///
    /// # Arguments
    ///
    /// * `name` - The memoir name to retrieve
    ///
    /// # Returns
    ///
    /// `Some(MemoirDetail)` if the memoir exists, `None` otherwise.
    fn get_memoir(&self, name: &str) -> IcmResult<Option<MemoirDetail>>;

    /// Add a concept to a memoir.
    ///
    /// # Arguments
    ///
    /// * `concept` - New concept data (name must be unique within memoir)
    ///
    /// # Returns
    ///
    /// The created `Concept` with generated ULID and timestamps.
    ///
    /// # Errors
    ///
    /// Returns `IcmError::NotFound` if the memoir doesn't exist.
    /// Returns `IcmError::AlreadyExists` if a concept with the same name exists in the memoir.
    fn add_concept(&self, concept: NewConcept) -> IcmResult<Concept>;

    /// Update a concept's definition or labels.
    ///
    /// # Arguments
    ///
    /// * `memoir` - The memoir name
    /// * `concept` - The concept name
    /// * `updates` - Fields to update (only provided fields are changed)
    ///
    /// # Errors
    ///
    /// Returns `IcmError::NotFound` if the memoir or concept doesn't exist.
    fn update_concept(&self, memoir: &str, concept: &str, updates: ConceptUpdate) -> IcmResult<()>;

    /// Search concepts within a specific memoir using FTS5.
    ///
    /// # Arguments
    ///
    /// * `memoir` - The memoir name to search within
    /// * `query` - Search query (FTS5 syntax supported)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of matching `Concept` objects ranked by BM25 score.
    fn search_concepts(&self, memoir: &str, query: &str, limit: u32) -> IcmResult<Vec<Concept>>;

    /// Search concepts across all memoirs using FTS5.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query (FTS5 syntax supported)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of `ConceptMatch` objects with memoir context, ranked by BM25 score.
    fn search_concepts_all(&self, query: &str, limit: u32) -> IcmResult<Vec<ConceptMatch>>;

    /// Create a typed relation between two concepts.
    ///
    /// # Arguments
    ///
    /// * `link` - New concept link data (source and target must exist in memoir)
    ///
    /// # Returns
    ///
    /// The created `ConceptLink` with generated ULID and timestamp.
    ///
    /// # Errors
    ///
    /// Returns `IcmError::NotFound` if the memoir or concepts don't exist.
    /// Returns `IcmError::InvalidInput` if source equals target (no self-loops).
    /// Returns `IcmError::AlreadyExists` if the exact link already exists.
    fn link_concepts(&self, link: NewConceptLink) -> IcmResult<ConceptLink>;

    /// Get a concept's neighborhood via BFS graph traversal.
    ///
    /// # Arguments
    ///
    /// * `memoir` - The memoir name
    /// * `concept` - The concept name to start from
    /// * `depth` - Maximum traversal depth (1 or 2)
    ///
    /// # Returns
    ///
    /// A `ConceptNeighborhood` containing the concept and its neighbors
    /// up to the specified depth, with relation and direction information.
    ///
    /// # Errors
    ///
    /// Returns `IcmError::NotFound` if the memoir or concept doesn't exist.
    fn inspect_concept(
        &self,
        memoir: &str,
        concept: &str,
        depth: u8,
    ) -> IcmResult<ConceptNeighborhood>;
}

/// Input data for creating a new memoir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMemoir {
    /// Unique name for the memoir (human-readable identifier).
    pub name: String,
    /// Description of the memoir's purpose and scope.
    pub description: String,
}

/// Summary information about a memoir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoirSummary {
    /// Memoir ULID.
    pub id: String,
    /// Memoir name.
    pub name: String,
    /// Memoir description.
    pub description: String,
    /// Number of concepts in the memoir.
    pub concept_count: u32,
    /// Number of links between concepts in the memoir.
    pub link_count: u32,
    /// Timestamp when the memoir was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when the memoir was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Detailed memoir information including all concepts and links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoirDetail {
    /// Memoir ULID.
    pub id: String,
    /// Memoir name.
    pub name: String,
    /// Memoir description.
    pub description: String,
    /// All concepts in the memoir.
    pub concepts: Vec<Concept>,
    /// All links between concepts in the memoir.
    pub links: Vec<LinkInfo>,
    /// Timestamp when the memoir was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when the memoir was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Link information with concept names for human readability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    /// Link ULID.
    pub id: String,
    /// Source concept name.
    pub source: String,
    /// Relation type.
    pub relation: RelationType,
    /// Target concept name.
    pub target: String,
    /// Relationship strength (0.0 - 1.0).
    pub weight: f32,
}

/// Input data for creating a new concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConcept {
    /// Memoir name to add the concept to.
    pub memoir_name: String,
    /// Concept name (must be unique within memoir).
    pub name: String,
    /// Concept definition.
    pub definition: String,
    /// Classification labels/tags.
    pub labels: Vec<String>,
}

/// Fields to update for a concept.
///
/// Only provided fields will be updated. Use `None` to leave a field unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptUpdate {
    /// New definition (optional).
    pub definition: Option<String>,
    /// New labels (optional, replaces existing).
    pub labels: Option<Vec<String>>,
}

/// Concept match result for cross-memoir search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptMatch {
    /// Memoir name containing the concept.
    pub memoir_name: String,
    /// Concept information.
    pub concept: Concept,
    /// BM25 relevance score.
    pub score: f32,
}

/// Input data for creating a new concept link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConceptLink {
    /// Memoir name containing both concepts.
    pub memoir_name: String,
    /// Source concept name.
    pub source_name: String,
    /// Target concept name.
    pub target_name: String,
    /// Relation type (one of 9 supported types).
    pub relation: RelationType,
    /// Relationship strength (0.0 - 1.0, default: 1.0).
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

/// Concept neighborhood information from BFS traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptNeighborhood {
    /// The concept being inspected.
    pub concept: Concept,
    /// Neighboring concepts with relation information.
    pub neighbors: Vec<NeighborInfo>,
    /// Traversal depth used.
    pub depth: u8,
}

/// Information about a neighboring concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborInfo {
    /// Relation type.
    pub relation: RelationType,
    /// Direction of the relation.
    pub direction: LinkDirection,
    /// The neighboring concept.
    pub concept: Concept,
    /// Relationship strength (0.0 - 1.0).
    pub weight: f32,
    /// BFS depth at which this neighbor was found (0 = direct neighbor).
    pub level: u8,
}

/// Direction of a concept link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkDirection {
    /// Link where the inspected concept is the source.
    Outgoing,
    /// Link where the inspected concept is the target.
    Incoming,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memoir_serialization() {
        let memoir = NewMemoir {
            name: "test-memoir".to_string(),
            description: "Test description".to_string(),
        };

        let json = serde_json::to_string(&memoir).unwrap();
        let deserialized: NewMemoir = serde_json::from_str(&json).unwrap();

        assert_eq!(memoir.name, deserialized.name);
        assert_eq!(memoir.description, deserialized.description);
    }

    #[test]
    fn test_new_concept_serialization() {
        let concept = NewConcept {
            memoir_name: "rust-patterns".to_string(),
            name: "Builder".to_string(),
            definition: "A creational pattern".to_string(),
            labels: vec!["design-pattern".to_string()],
        };

        let json = serde_json::to_string(&concept).unwrap();
        let deserialized: NewConcept = serde_json::from_str(&json).unwrap();

        assert_eq!(concept.name, deserialized.name);
        assert_eq!(concept.labels, deserialized.labels);
    }

    #[test]
    fn test_new_concept_link_serialization() {
        let link = NewConceptLink {
            memoir_name: "rust".to_string(),
            source_name: "Tokio".to_string(),
            target_name: "Async Runtime".to_string(),
            relation: RelationType::IsA,
            weight: 0.8,
        };

        let json = serde_json::to_string(&link).unwrap();
        let deserialized: NewConceptLink = serde_json::from_str(&json).unwrap();

        assert_eq!(link.source_name, deserialized.source_name);
        assert_eq!(link.relation, deserialized.relation);
        assert_eq!(link.weight, deserialized.weight);
    }

    #[test]
    fn test_new_concept_link_default_weight() {
        let json =
            r#"{"memoir_name":"test","source_name":"A","target_name":"B","relation":"is_a"}"#;
        let link: NewConceptLink = serde_json::from_str(json).unwrap();

        assert_eq!(link.weight, 1.0);
    }

    #[test]
    fn test_concept_update_partial() {
        let update = ConceptUpdate {
            definition: Some("New definition".to_string()),
            labels: None,
        };

        let json = serde_json::to_string(&update).unwrap();
        let deserialized: ConceptUpdate = serde_json::from_str(&json).unwrap();

        assert_eq!(update.definition, deserialized.definition);
        assert!(deserialized.labels.is_none());
    }

    #[test]
    fn test_link_direction_serialization() {
        let outgoing = LinkDirection::Outgoing;
        let incoming = LinkDirection::Incoming;

        let out_json = serde_json::to_string(&outgoing).unwrap();
        let in_json = serde_json::to_string(&incoming).unwrap();

        assert_eq!(out_json, r#""outgoing""#);
        assert_eq!(in_json, r#""incoming""#);

        let out_deser: LinkDirection = serde_json::from_str(&out_json).unwrap();
        let in_deser: LinkDirection = serde_json::from_str(&in_json).unwrap();

        assert_eq!(outgoing, out_deser);
        assert_eq!(incoming, in_deser);
    }

    #[test]
    fn test_neighbor_info_construction() {
        use chrono::Utc;

        let concept = Concept {
            id: "01J0EXAMPLE".to_string(),
            memoir_id: "01J0MEMOIR".to_string(),
            name: "Test Concept".to_string(),
            definition: "A test".to_string(),
            labels: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        let neighbor = NeighborInfo {
            relation: RelationType::RelatedTo,
            direction: LinkDirection::Outgoing,
            concept: concept.clone(),
            weight: 0.9,
            level: 1,
        };

        assert_eq!(neighbor.concept.id, concept.id);
        assert_eq!(neighbor.level, 1);
        assert_eq!(neighbor.direction, LinkDirection::Outgoing);
    }
}

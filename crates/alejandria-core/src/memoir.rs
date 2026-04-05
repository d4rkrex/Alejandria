//! Memoir types for knowledge graph (semantic memory) storage.
//!
//! Memoirs provide permanent, structured knowledge containers with typed relations
//! between concepts. Unlike episodic memories that decay over time, memoirs represent
//! stable knowledge that should persist indefinitely.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A named knowledge graph container (semantic memory).
///
/// Memoirs organize concepts and their relations into coherent knowledge domains.
/// Each memoir has a unique name and contains a graph of concepts linked by typed relations.
///
/// # Examples
///
/// ```
/// use alejandria_core::memoir::Memoir;
/// use chrono::Utc;
///
/// let memoir = Memoir {
///     id: ulid::Ulid::new().to_string(),
///     name: "rust-patterns".to_string(),
///     description: "Common Rust design patterns and idioms".to_string(),
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
///     metadata: serde_json::json!({}),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memoir {
    /// Unique identifier (ULID)
    pub id: String,

    /// Human-readable unique name
    pub name: String,

    /// Description of the memoir's purpose and scope
    pub description: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Extensible metadata (JSON object)
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// A knowledge graph node within a memoir.
///
/// Concepts represent individual pieces of knowledge with definitions and
/// classification labels. Concepts are linked via typed relations to form
/// a knowledge graph.
///
/// # Examples
///
/// ```
/// use alejandria_core::memoir::Concept;
/// use chrono::Utc;
///
/// let concept = Concept {
///     id: ulid::Ulid::new().to_string(),
///     memoir_id: "01HN123456789ABCDEFGHIJK".to_string(),
///     name: "Builder Pattern".to_string(),
///     definition: "A creational design pattern for constructing complex objects".to_string(),
///     labels: vec!["design-pattern".to_string(), "creational".to_string()],
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
///     metadata: serde_json::json!({}),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// Unique identifier (ULID)
    pub id: String,

    /// Parent memoir ID
    pub memoir_id: String,

    /// Concept name (unique within memoir)
    pub name: String,

    /// Detailed definition or explanation
    pub definition: String,

    /// Classification tags for organizing concepts
    pub labels: Vec<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Extensible metadata (JSON object)
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// A typed relation between two concepts in a knowledge graph.
///
/// Links represent directed edges with semantic relation types. The relation
/// type defines the nature of the connection between source and target concepts.
///
/// # Examples
///
/// ```
/// use alejandria_core::memoir::{ConceptLink, RelationType};
/// use chrono::Utc;
///
/// let link = ConceptLink {
///     id: ulid::Ulid::new().to_string(),
///     memoir_id: "01HN123456789ABCDEFGHIJK".to_string(),
///     source_id: "01HN1SOURCE00000000000".to_string(),
///     target_id: "01HN1TARGET00000000000".to_string(),
///     relation: RelationType::IsA,
///     weight: 1.0,
///     created_at: Utc::now(),
///     metadata: serde_json::json!({}),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptLink {
    /// Unique identifier (ULID)
    pub id: String,

    /// Parent memoir ID
    pub memoir_id: String,

    /// Source concept ID
    pub source_id: String,

    /// Target concept ID
    pub target_id: String,

    /// Relation type defining the semantic connection
    pub relation: RelationType,

    /// Strength of the relationship (0.0 - 1.0)
    #[serde(default = "default_weight")]
    pub weight: f32,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Extensible metadata (JSON object)
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Typed relation types for concept links.
///
/// Defines the semantic nature of connections between concepts in the knowledge graph.
/// All relations are directed from source to target.
///
/// # Examples
///
/// ```
/// use alejandria_core::memoir::RelationType;
///
/// // Taxonomy relation: "Rust is_a Programming Language"
/// let relation = RelationType::IsA;
///
/// // Causal relation: "Memory leak causes Performance degradation"
/// let relation = RelationType::Causes;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// Taxonomy/inheritance (e.g., "Rust is_a Programming Language")
    IsA,

    /// Attribute relation (e.g., "JWT has_property Stateless")
    HasProperty,

    /// Generic association (e.g., "REST related_to HTTP")
    RelatedTo,

    /// Causal relationship (e.g., "Memory leak causes Performance degradation")
    Causes,

    /// Dependency (e.g., "OAuth prerequisite_of API Access")
    PrerequisiteOf,

    /// Instantiation (e.g., "Builder example_of Creational Pattern")
    ExampleOf,

    /// Conflict (e.g., "SQL contradicts NoSQL")
    Contradicts,

    /// Similarity (e.g., "Rust similar_to C++")
    SimilarTo,

    /// Composition (e.g., "Module part_of System")
    PartOf,
}

impl RelationType {
    /// Returns all valid relation types.
    ///
    /// # Examples
    ///
    /// ```
    /// use alejandria_core::memoir::RelationType;
    ///
    /// let all_relations = RelationType::all();
    /// assert_eq!(all_relations.len(), 9);
    /// ```
    pub fn all() -> &'static [RelationType] {
        &[
            RelationType::IsA,
            RelationType::HasProperty,
            RelationType::RelatedTo,
            RelationType::Causes,
            RelationType::PrerequisiteOf,
            RelationType::ExampleOf,
            RelationType::Contradicts,
            RelationType::SimilarTo,
            RelationType::PartOf,
        ]
    }

    /// Returns the string representation of the relation type.
    ///
    /// # Examples
    ///
    /// ```
    /// use alejandria_core::memoir::RelationType;
    ///
    /// assert_eq!(RelationType::IsA.as_str(), "is_a");
    /// assert_eq!(RelationType::Causes.as_str(), "causes");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::IsA => "is_a",
            RelationType::HasProperty => "has_property",
            RelationType::RelatedTo => "related_to",
            RelationType::Causes => "causes",
            RelationType::PrerequisiteOf => "prerequisite_of",
            RelationType::ExampleOf => "example_of",
            RelationType::Contradicts => "contradicts",
            RelationType::SimilarTo => "similar_to",
            RelationType::PartOf => "part_of",
        }
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for RelationType {
    type Err = String;

    /// Parses a relation type from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use alejandria_core::memoir::RelationType;
    /// use std::str::FromStr;
    ///
    /// assert_eq!(RelationType::from_str("is_a").unwrap(), RelationType::IsA);
    /// assert!(RelationType::from_str("invalid").is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "is_a" => Ok(RelationType::IsA),
            "has_property" => Ok(RelationType::HasProperty),
            "related_to" => Ok(RelationType::RelatedTo),
            "causes" => Ok(RelationType::Causes),
            "prerequisite_of" => Ok(RelationType::PrerequisiteOf),
            "example_of" => Ok(RelationType::ExampleOf),
            "contradicts" => Ok(RelationType::Contradicts),
            "similar_to" => Ok(RelationType::SimilarTo),
            "part_of" => Ok(RelationType::PartOf),
            _ => Err(format!("Invalid relation type: {}", s)),
        }
    }
}

/// Default weight for concept links.
fn default_weight() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memoir_serialization() {
        let memoir = Memoir {
            id: "01HN123456789ABCDEFGHIJK".to_string(),
            name: "test-memoir".to_string(),
            description: "Test description".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: serde_json::json!({"version": 1}),
        };

        let json = serde_json::to_string(&memoir).unwrap();
        let deserialized: Memoir = serde_json::from_str(&json).unwrap();
        assert_eq!(memoir.id, deserialized.id);
        assert_eq!(memoir.name, deserialized.name);
    }

    #[test]
    fn test_concept_serialization() {
        let concept = Concept {
            id: "01HN123456789ABCDEFGHIJK".to_string(),
            memoir_id: "01HN1MEMOIR0000000000".to_string(),
            name: "Test Concept".to_string(),
            definition: "A test concept".to_string(),
            labels: vec!["test".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        let json = serde_json::to_string(&concept).unwrap();
        let deserialized: Concept = serde_json::from_str(&json).unwrap();
        assert_eq!(concept.id, deserialized.id);
        assert_eq!(concept.name, deserialized.name);
    }

    #[test]
    fn test_concept_link_serialization() {
        let link = ConceptLink {
            id: "01HN123456789ABCDEFGHIJK".to_string(),
            memoir_id: "01HN1MEMOIR0000000000".to_string(),
            source_id: "01HN1SOURCE00000000000".to_string(),
            target_id: "01HN1TARGET00000000000".to_string(),
            relation: RelationType::IsA,
            weight: 0.8,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        let json = serde_json::to_string(&link).unwrap();
        let deserialized: ConceptLink = serde_json::from_str(&json).unwrap();
        assert_eq!(link.id, deserialized.id);
        assert_eq!(link.relation, deserialized.relation);
        assert_eq!(link.weight, deserialized.weight);
    }

    #[test]
    fn test_relation_type_all() {
        let relations = RelationType::all();
        assert_eq!(relations.len(), 9);
        assert!(relations.contains(&RelationType::IsA));
        assert!(relations.contains(&RelationType::Causes));
    }

    #[test]
    fn test_relation_type_as_str() {
        assert_eq!(RelationType::IsA.as_str(), "is_a");
        assert_eq!(RelationType::HasProperty.as_str(), "has_property");
        assert_eq!(RelationType::RelatedTo.as_str(), "related_to");
        assert_eq!(RelationType::Causes.as_str(), "causes");
        assert_eq!(RelationType::PrerequisiteOf.as_str(), "prerequisite_of");
        assert_eq!(RelationType::ExampleOf.as_str(), "example_of");
        assert_eq!(RelationType::Contradicts.as_str(), "contradicts");
        assert_eq!(RelationType::SimilarTo.as_str(), "similar_to");
        assert_eq!(RelationType::PartOf.as_str(), "part_of");
    }

    #[test]
    fn test_relation_type_from_str() {
        use std::str::FromStr;
        assert_eq!(RelationType::from_str("is_a").unwrap(), RelationType::IsA);
        assert_eq!(
            RelationType::from_str("causes").unwrap(),
            RelationType::Causes
        );
        assert_eq!(
            RelationType::from_str("part_of").unwrap(),
            RelationType::PartOf
        );
        assert!(RelationType::from_str("invalid").is_err());
    }

    #[test]
    fn test_relation_type_display() {
        assert_eq!(RelationType::IsA.to_string(), "is_a");
        assert_eq!(RelationType::Contradicts.to_string(), "contradicts");
    }

    #[test]
    fn test_relation_type_serde() {
        let relation = RelationType::IsA;
        let json = serde_json::to_string(&relation).unwrap();
        assert_eq!(json, "\"is_a\"");

        let deserialized: RelationType = serde_json::from_str(&json).unwrap();
        assert_eq!(relation, deserialized);
    }

    #[test]
    fn test_default_weight() {
        let link = ConceptLink {
            id: "01HN123456789ABCDEFGHIJK".to_string(),
            memoir_id: "01HN1MEMOIR0000000000".to_string(),
            source_id: "01HN1SOURCE00000000000".to_string(),
            target_id: "01HN1TARGET00000000000".to_string(),
            relation: RelationType::IsA,
            weight: default_weight(),
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        assert_eq!(link.weight, 1.0);
    }
}

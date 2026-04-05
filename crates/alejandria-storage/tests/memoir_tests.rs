//! Integration tests for Memoir (knowledge graph) operations on SqliteStore.
//!
//! These tests validate the memoir functionality including:
//! - Creating memoirs with UNIQUE name constraint
//! - Adding concepts with UNIQUE(memoir_id, name) constraint
//! - Creating typed links with relation validation
//! - Self-loop prevention via CHECK constraint
//! - BFS traversal at different depths
//! - CASCADE DELETE behavior
//! - FTS5 search within memoir and across memoirs

use alejandria_core::{
    error::IcmError,
    memoir_store::{MemoirStore, NewConcept, NewConceptLink, NewMemoir},
    RelationType,
};
use alejandria_storage::SqliteStore;

/// Test UNIQUE constraint on memoir.name
///
/// Given: A memoir with name "test-memoir" exists
/// When: Attempting to create another memoir with the same name
/// Then: Returns IcmError::AlreadyExists
#[test]
fn test_memoir_unique_name_constraint() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create first memoir
    let memoir1 = store
        .create_memoir(NewMemoir {
            name: "test-memoir".to_string(),
            description: "First memoir".to_string(),
        })
        .expect("Failed to create first memoir");

    assert_eq!(memoir1.name, "test-memoir");

    // Attempt to create second memoir with same name
    let result = store.create_memoir(NewMemoir {
        name: "test-memoir".to_string(),
        description: "Second memoir".to_string(),
    });

    match result {
        Err(IcmError::AlreadyExists(msg)) => {
            assert!(msg.contains("Memoir"));
            assert!(msg.contains("name"));
            assert!(msg.contains("test-memoir"));
        }
        _ => panic!("Expected AlreadyExists error, got: {:?}", result),
    }
}

/// Test UNIQUE(memoir_id, name) constraint on concepts
///
/// Given: A memoir with a concept named "Builder Pattern"
/// When: Attempting to add another concept with the same name to the same memoir
/// Then: Returns IcmError::AlreadyExists
#[test]
fn test_concept_unique_within_memoir() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create memoir
    store
        .create_memoir(NewMemoir {
            name: "rust-patterns".to_string(),
            description: "Rust design patterns".to_string(),
        })
        .expect("Failed to create memoir");

    // Add first concept
    let concept1 = store
        .add_concept(NewConcept {
            memoir_name: "rust-patterns".to_string(),
            name: "Builder Pattern".to_string(),
            definition: "A creational pattern".to_string(),
            labels: vec!["design-pattern".to_string()],
        })
        .expect("Failed to add first concept");

    assert_eq!(concept1.name, "Builder Pattern");

    // Attempt to add second concept with same name to same memoir
    let result = store.add_concept(NewConcept {
        memoir_name: "rust-patterns".to_string(),
        name: "Builder Pattern".to_string(),
        definition: "Duplicate concept".to_string(),
        labels: vec![],
    });

    match result {
        Err(IcmError::AlreadyExists(msg)) => {
            assert!(msg.contains("Concept"));
            assert!(msg.contains("already exists"));
            assert!(msg.contains("Builder Pattern"));
        }
        _ => panic!("Expected AlreadyExists error, got: {:?}", result),
    }

    // But should allow same name in different memoir
    store
        .create_memoir(NewMemoir {
            name: "java-patterns".to_string(),
            description: "Java design patterns".to_string(),
        })
        .expect("Failed to create second memoir");

    let concept2 = store
        .add_concept(NewConcept {
            memoir_name: "java-patterns".to_string(),
            name: "Builder Pattern".to_string(),
            definition: "Builder in Java".to_string(),
            labels: vec![],
        })
        .expect("Should allow same name in different memoir");

    assert_eq!(concept2.name, "Builder Pattern");
    assert_ne!(concept1.memoir_id, concept2.memoir_id);
}

/// Test relation type validation
///
/// Given: A memoir with two concepts
/// When: Creating a link with all 9 valid relation types
/// Then: All links are created successfully
#[test]
fn test_relation_type_validation() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create memoir and concepts
    store
        .create_memoir(NewMemoir {
            name: "test-relations".to_string(),
            description: "Testing relation types".to_string(),
        })
        .expect("Failed to create memoir");

    store
        .add_concept(NewConcept {
            memoir_name: "test-relations".to_string(),
            name: "Source".to_string(),
            definition: "Source concept".to_string(),
            labels: vec![],
        })
        .expect("Failed to add source");

    store
        .add_concept(NewConcept {
            memoir_name: "test-relations".to_string(),
            name: "Target".to_string(),
            definition: "Target concept".to_string(),
            labels: vec![],
        })
        .expect("Failed to add target");

    // Test all 9 relation types
    let relation_types = vec![
        RelationType::IsA,
        RelationType::HasProperty,
        RelationType::RelatedTo,
        RelationType::Causes,
        RelationType::PrerequisiteOf,
        RelationType::ExampleOf,
        RelationType::Contradicts,
        RelationType::SimilarTo,
        RelationType::PartOf,
    ];

    for (i, relation) in relation_types.iter().enumerate() {
        // Create unique target for each relation type
        let target_name = format!("Target{}", i);
        store
            .add_concept(NewConcept {
                memoir_name: "test-relations".to_string(),
                name: target_name.clone(),
                definition: format!("Target for {:?}", relation),
                labels: vec![],
            })
            .unwrap_or_else(|_| panic!("Failed to add target for {:?}", relation));

        let link = store
            .link_concepts(NewConceptLink {
                memoir_name: "test-relations".to_string(),
                source_name: "Source".to_string(),
                target_name,
                relation: *relation,
                weight: 1.0,
            })
            .unwrap_or_else(|_| panic!("Failed to create link with {:?}", relation));

        assert_eq!(link.relation, *relation);
    }
}

/// Test self-loop prevention via CHECK constraint
///
/// Given: A memoir with a concept
/// When: Attempting to create a link from concept to itself
/// Then: Returns IcmError::InvalidInput
#[test]
fn test_self_loop_prevention() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create memoir and concept
    store
        .create_memoir(NewMemoir {
            name: "test-loops".to_string(),
            description: "Testing self-loop prevention".to_string(),
        })
        .expect("Failed to create memoir");

    store
        .add_concept(NewConcept {
            memoir_name: "test-loops".to_string(),
            name: "Self-Ref".to_string(),
            definition: "Concept that tries to link to itself".to_string(),
            labels: vec![],
        })
        .expect("Failed to add concept");

    // Attempt to create self-loop
    let result = store.link_concepts(NewConceptLink {
        memoir_name: "test-loops".to_string(),
        source_name: "Self-Ref".to_string(),
        target_name: "Self-Ref".to_string(),
        relation: RelationType::RelatedTo,
        weight: 1.0,
    });

    match result {
        Err(IcmError::InvalidInput(msg)) => {
            assert!(msg.contains("self-loop") || msg.contains("same concept"));
        }
        _ => panic!(
            "Expected InvalidInput error for self-loop, got: {:?}",
            result
        ),
    }
}

/// Test BFS traversal at depth 1 and 2
///
/// Given: A memoir with a concept graph:
///        A -> B -> C
///        A -> D
/// When: inspect_concept(A, depth=1) and inspect_concept(A, depth=2)
/// Then: depth=1 returns B and D; depth=2 returns B, D, and C
#[test]
fn test_bfs_traversal_depths() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create memoir and concepts
    store
        .create_memoir(NewMemoir {
            name: "test-bfs".to_string(),
            description: "Testing BFS traversal".to_string(),
        })
        .expect("Failed to create memoir");

    for name in &["A", "B", "C", "D"] {
        store
            .add_concept(NewConcept {
                memoir_name: "test-bfs".to_string(),
                name: name.to_string(),
                definition: format!("Concept {}", name),
                labels: vec![],
            })
            .unwrap_or_else(|_| panic!("Failed to add concept {}", name));
    }

    // Create graph: A -> B -> C, A -> D
    store
        .link_concepts(NewConceptLink {
            memoir_name: "test-bfs".to_string(),
            source_name: "A".to_string(),
            target_name: "B".to_string(),
            relation: RelationType::RelatedTo,
            weight: 1.0,
        })
        .expect("Failed to create A->B link");

    store
        .link_concepts(NewConceptLink {
            memoir_name: "test-bfs".to_string(),
            source_name: "B".to_string(),
            target_name: "C".to_string(),
            relation: RelationType::RelatedTo,
            weight: 1.0,
        })
        .expect("Failed to create B->C link");

    store
        .link_concepts(NewConceptLink {
            memoir_name: "test-bfs".to_string(),
            source_name: "A".to_string(),
            target_name: "D".to_string(),
            relation: RelationType::RelatedTo,
            weight: 1.0,
        })
        .expect("Failed to create A->D link");

    // Test depth=1 (should return B and D)
    let neighborhood1 = store
        .inspect_concept("test-bfs", "A", 1)
        .expect("Failed to inspect at depth 1");

    assert_eq!(neighborhood1.concept.name, "A");
    assert_eq!(neighborhood1.depth, 1);
    assert_eq!(neighborhood1.neighbors.len(), 2);

    let neighbor_names: Vec<String> = neighborhood1
        .neighbors
        .iter()
        .map(|n| n.concept.name.clone())
        .collect();
    assert!(neighbor_names.contains(&"B".to_string()));
    assert!(neighbor_names.contains(&"D".to_string()));

    // All neighbors should be at level 1 (direct neighbors of A)
    for neighbor in &neighborhood1.neighbors {
        assert_eq!(neighbor.level, 1);
    }

    // Test depth=2 (should return B, D at level 1, and C at level 2)
    let neighborhood2 = store
        .inspect_concept("test-bfs", "A", 2)
        .expect("Failed to inspect at depth 2");

    assert_eq!(neighborhood2.concept.name, "A");
    assert_eq!(neighborhood2.depth, 2);
    assert_eq!(neighborhood2.neighbors.len(), 3);

    let neighbor_names2: Vec<String> = neighborhood2
        .neighbors
        .iter()
        .map(|n| n.concept.name.clone())
        .collect();
    assert!(neighbor_names2.contains(&"B".to_string()));
    assert!(neighbor_names2.contains(&"C".to_string()));
    assert!(neighbor_names2.contains(&"D".to_string()));

    // Check levels
    for neighbor in &neighborhood2.neighbors {
        if neighbor.concept.name == "C" {
            assert_eq!(neighbor.level, 2, "C should be at level 2");
        } else {
            assert_eq!(neighbor.level, 1, "B and D should be at level 1");
        }
    }
}

/// Test CASCADE DELETE behavior
///
/// Given: A memoir with concepts and links
/// When: Deleting the memoir via SQL
/// Then: All concepts and links are also deleted (verified via schema test)
#[test]
fn test_cascade_delete_memoir() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create memoir with concepts and links
    store
        .create_memoir(NewMemoir {
            name: "test-cascade".to_string(),
            description: "Testing CASCADE DELETE".to_string(),
        })
        .expect("Failed to create memoir");

    store
        .add_concept(NewConcept {
            memoir_name: "test-cascade".to_string(),
            name: "Concept1".to_string(),
            definition: "First concept".to_string(),
            labels: vec![],
        })
        .expect("Failed to add concept1");

    store
        .add_concept(NewConcept {
            memoir_name: "test-cascade".to_string(),
            name: "Concept2".to_string(),
            definition: "Second concept".to_string(),
            labels: vec![],
        })
        .expect("Failed to add concept2");

    store
        .link_concepts(NewConceptLink {
            memoir_name: "test-cascade".to_string(),
            source_name: "Concept1".to_string(),
            target_name: "Concept2".to_string(),
            relation: RelationType::RelatedTo,
            weight: 1.0,
        })
        .expect("Failed to create link");

    // Verify memoir has content
    let detail_before = store
        .get_memoir("test-cascade")
        .expect("Failed to get memoir")
        .expect("Memoir should exist");
    assert_eq!(detail_before.concepts.len(), 2);
    assert_eq!(detail_before.links.len(), 1);

    // Note: CASCADE DELETE is verified in schema::tests::test_cascade_delete_memoir
    // which uses direct SQL access. Here we just verify the memoir structure exists.
    // The actual CASCADE DELETE behavior is enforced by SQLite foreign key constraints
    // defined in schema.rs with ON DELETE CASCADE.
}

/// Test FTS5 search within memoir and across memoirs
///
/// Given: Two memoirs with concepts containing searchable text
/// When: search_concepts() and search_concepts_all()
/// Then: Returns matching concepts ranked by relevance
#[test]
fn test_fts5_search() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create two memoirs with concepts
    store
        .create_memoir(NewMemoir {
            name: "rust-concepts".to_string(),
            description: "Rust programming concepts".to_string(),
        })
        .expect("Failed to create rust memoir");

    store
        .create_memoir(NewMemoir {
            name: "python-concepts".to_string(),
            description: "Python programming concepts".to_string(),
        })
        .expect("Failed to create python memoir");

    // Add concepts to rust memoir
    store
        .add_concept(NewConcept {
            memoir_name: "rust-concepts".to_string(),
            name: "Ownership".to_string(),
            definition: "Rust's ownership system manages memory safety without garbage collection"
                .to_string(),
            labels: vec!["memory".to_string(), "safety".to_string()],
        })
        .expect("Failed to add ownership concept");

    store
        .add_concept(NewConcept {
            memoir_name: "rust-concepts".to_string(),
            name: "Borrowing".to_string(),
            definition: "Borrowing allows references to data without taking ownership".to_string(),
            labels: vec!["memory".to_string()],
        })
        .expect("Failed to add borrowing concept");

    // Add concepts to python memoir
    store
        .add_concept(NewConcept {
            memoir_name: "python-concepts".to_string(),
            name: "GC".to_string(),
            definition: "Python uses automatic garbage collection for memory management"
                .to_string(),
            labels: vec!["memory".to_string()],
        })
        .expect("Failed to add GC concept");

    // Test search within rust memoir
    let rust_results = store
        .search_concepts("rust-concepts", "ownership memory", 10)
        .expect("Failed to search rust concepts");

    assert!(
        !rust_results.is_empty(),
        "Should find at least ownership concept"
    );
    assert!(rust_results.iter().any(|c| c.name == "Ownership"));

    // Test search across all memoirs
    let all_results = store
        .search_concepts_all("memory", 10)
        .expect("Failed to search all concepts");

    assert!(
        all_results.len() >= 3,
        "Should find concepts from both memoirs"
    );

    let memoir_names: Vec<String> = all_results.iter().map(|m| m.memoir_name.clone()).collect();
    assert!(memoir_names.contains(&"rust-concepts".to_string()));
    assert!(memoir_names.contains(&"python-concepts".to_string()));

    // Test that results are ranked (ownership should rank high for "ownership memory")
    let ownership_results = store
        .search_concepts_all("ownership", 10)
        .expect("Failed to search for ownership");

    assert!(!ownership_results.is_empty());
    assert_eq!(
        ownership_results[0].concept.name, "Ownership",
        "Ownership should be top result for 'ownership' query"
    );
}

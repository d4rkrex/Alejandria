//! Integration tests for memoir (knowledge graph) tool handlers.
//!
//! Tests all 9 memoir tools using SqliteStore::open_in_memory():
//! - memoir_create: create memoirs, duplicate detection
//! - memoir_list: list with counts
//! - memoir_show: full graph retrieval
//! - memoir_add_concept: add concepts, validate required fields
//! - memoir_refine: update concept definition/labels
//! - memoir_search: FTS5 search within memoir
//! - memoir_search_all: cross-memoir search
//! - memoir_link: typed links between concepts
//! - memoir_inspect: BFS neighborhood traversal

use alejandria_mcp::tools::memoir::*;
use alejandria_storage::SqliteStore;
use serde_json::{json, Value};

/// Helper: create an in-memory store
fn test_store() -> SqliteStore {
    SqliteStore::open_in_memory().expect("Failed to create in-memory store")
}

/// Helper: check that a response is successful (has result, no error)
fn assert_success(resp: &alejandria_mcp::JsonRpcResponse) {
    assert!(
        resp.error.is_none(),
        "Expected success, got error: {:?}",
        resp.error
    );
    assert!(resp.result.is_some(), "Missing result in success response");
}

/// Helper: check that a response is an error with expected code
fn assert_error(resp: &alejandria_mcp::JsonRpcResponse, expected_code: i32) {
    assert!(
        resp.error.is_some(),
        "Expected error with code {}, got success: {:?}",
        expected_code,
        resp.result
    );
    let err = resp.error.as_ref().unwrap();
    assert_eq!(
        err.code, expected_code,
        "Expected error code {}, got {}: {}",
        expected_code, err.code, err.message
    );
}

// ============================================================
// memoir_create
// ============================================================

#[test]
fn test_memoir_create_basic() {
    let store = test_store();
    let resp = handle_memoir_create(
        json!(1),
        Some(json!({ "name": "test-memoir", "description": "A test memoir" })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    assert_eq!(result["name"], "test-memoir");
    assert_eq!(result["description"], "A test memoir");
    assert!(result["id"].is_string());
    assert!(result["created_at"].is_string());
}

#[test]
fn test_memoir_create_without_description() {
    let store = test_store();
    let resp = handle_memoir_create(json!(1), Some(json!({ "name": "minimal-memoir" })), &store);

    assert_success(&resp);
    let result = resp.result.unwrap();
    assert_eq!(result["name"], "minimal-memoir");
}

#[test]
fn test_memoir_create_missing_name_fails() {
    let store = test_store();
    let resp = handle_memoir_create(
        json!(1),
        Some(json!({ "description": "No name provided" })),
        &store,
    );

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_create_missing_params_fails() {
    let store = test_store();
    let resp = handle_memoir_create(json!(1), None, &store);

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_create_duplicate_name_fails() {
    let store = test_store();

    // Create first
    let resp1 = handle_memoir_create(json!(1), Some(json!({ "name": "dup-memoir" })), &store);
    assert_success(&resp1);

    // Try to create duplicate
    let resp2 = handle_memoir_create(json!(2), Some(json!({ "name": "dup-memoir" })), &store);
    assert!(
        resp2.error.is_some(),
        "Expected error for duplicate memoir name"
    );
}

// ============================================================
// memoir_list
// ============================================================

#[test]
fn test_memoir_list_empty() {
    let store = test_store();
    let resp = handle_memoir_list(json!(1), None, &store);

    assert_success(&resp);
    let result = resp.result.unwrap();
    let memoirs = result["memoirs"].as_array().unwrap();
    assert_eq!(memoirs.len(), 0);
}

#[test]
fn test_memoir_list_with_memoirs() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "memoir-1" })), &store);
    handle_memoir_create(json!(2), Some(json!({ "name": "memoir-2" })), &store);

    let resp = handle_memoir_list(json!(3), None, &store);
    assert_success(&resp);
    let result = resp.result.unwrap();
    let memoirs = result["memoirs"].as_array().unwrap();
    assert_eq!(memoirs.len(), 2);
}

#[test]
fn test_memoir_list_includes_counts() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "counted-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "counted-memoir", "name": "Concept1", "definition": "Def" })),
        &store,
    );

    let resp = handle_memoir_list(json!(3), None, &store);
    assert_success(&resp);
    let result = resp.result.unwrap();
    let memoirs = result["memoirs"].as_array().unwrap();
    assert_eq!(memoirs.len(), 1);

    let memoir = &memoirs[0];
    assert!(memoir["concept_count"].is_number());
    assert!(memoir["link_count"].is_number());
}

// ============================================================
// memoir_show
// ============================================================

#[test]
fn test_memoir_show_existing() {
    let store = test_store();

    handle_memoir_create(
        json!(1),
        Some(json!({ "name": "show-memoir", "description": "For show test" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(2),
        Some(
            json!({ "memoir": "show-memoir", "name": "Ownership", "definition": "Rust ownership" }),
        ),
        &store,
    );

    let resp = handle_memoir_show(json!(3), Some(json!({ "name": "show-memoir" })), &store);
    assert_success(&resp);
    let result = resp.result.unwrap();

    assert_eq!(result["memoir"]["name"], "show-memoir");
    let concepts = result["concepts"].as_array().unwrap();
    assert_eq!(concepts.len(), 1);
    assert_eq!(concepts[0]["name"], "Ownership");
}

#[test]
fn test_memoir_show_nonexistent() {
    let store = test_store();
    let resp = handle_memoir_show(json!(1), Some(json!({ "name": "nonexistent" })), &store);

    assert!(
        resp.error.is_some(),
        "Expected error for nonexistent memoir"
    );
}

#[test]
fn test_memoir_show_missing_name_fails() {
    let store = test_store();
    let resp = handle_memoir_show(json!(1), Some(json!({})), &store);

    assert_error(&resp, -32602);
}

// ============================================================
// memoir_add_concept
// ============================================================

#[test]
fn test_memoir_add_concept_basic() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "concept-memoir" })), &store);

    let resp = handle_memoir_add_concept(
        json!(2),
        Some(json!({
            "memoir": "concept-memoir",
            "name": "Borrowing",
            "definition": "Temporary access to data",
            "labels": ["rust", "memory"]
        })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    assert_eq!(result["name"], "Borrowing");
    assert_eq!(result["definition"], "Temporary access to data");
    assert!(result["labels"]
        .as_array()
        .unwrap()
        .contains(&json!("rust")));
}

#[test]
fn test_memoir_add_concept_minimal() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "minimal-memoir" })), &store);

    let resp = handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "minimal-memoir", "name": "SimpleConcept" })),
        &store,
    );

    assert_success(&resp);
}

#[test]
fn test_memoir_add_concept_missing_memoir_fails() {
    let store = test_store();
    let resp = handle_memoir_add_concept(json!(1), Some(json!({ "name": "Concept" })), &store);

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_add_concept_missing_name_fails() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "memoir-x" })), &store);

    let resp = handle_memoir_add_concept(json!(2), Some(json!({ "memoir": "memoir-x" })), &store);

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_add_concept_to_nonexistent_memoir_fails() {
    let store = test_store();
    let resp = handle_memoir_add_concept(
        json!(1),
        Some(json!({ "memoir": "ghost-memoir", "name": "Concept" })),
        &store,
    );

    assert!(
        resp.error.is_some(),
        "Expected error for nonexistent memoir"
    );
}

// ============================================================
// memoir_refine
// ============================================================

#[test]
fn test_memoir_refine_definition() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "refine-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "refine-memoir", "name": "Lifetimes", "definition": "Old def" })),
        &store,
    );

    let resp = handle_memoir_refine(
        json!(3),
        Some(json!({
            "memoir": "refine-memoir",
            "concept": "Lifetimes",
            "definition": "Updated definition of lifetimes"
        })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    assert_eq!(result["success"], true);
}

#[test]
fn test_memoir_refine_labels() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "labels-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "labels-memoir", "name": "Traits" })),
        &store,
    );

    let resp = handle_memoir_refine(
        json!(3),
        Some(json!({
            "memoir": "labels-memoir",
            "concept": "Traits",
            "labels": ["interface", "polymorphism"]
        })),
        &store,
    );

    assert_success(&resp);
}

#[test]
fn test_memoir_refine_missing_memoir_fails() {
    let store = test_store();
    let resp = handle_memoir_refine(json!(1), Some(json!({ "concept": "X" })), &store);

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_refine_missing_concept_fails() {
    let store = test_store();
    let resp = handle_memoir_refine(json!(1), Some(json!({ "memoir": "some-memoir" })), &store);

    assert_error(&resp, -32602);
}

// ============================================================
// memoir_search
// ============================================================

#[test]
fn test_memoir_search_finds_concept() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "search-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({
            "memoir": "search-memoir",
            "name": "Pattern Matching",
            "definition": "Rust pattern matching with match keyword"
        })),
        &store,
    );

    let resp = handle_memoir_search(
        json!(3),
        Some(json!({ "memoir": "search-memoir", "query": "pattern" })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    let concepts = result["concepts"].as_array().unwrap();
    assert!(
        !concepts.is_empty(),
        "Expected to find concept matching 'pattern'"
    );
    assert_eq!(concepts[0]["name"], "Pattern Matching");
}

#[test]
fn test_memoir_search_empty_results() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "empty-search" })), &store);

    let resp = handle_memoir_search(
        json!(2),
        Some(json!({ "memoir": "empty-search", "query": "nonexistent" })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    let concepts = result["concepts"].as_array().unwrap();
    assert_eq!(concepts.len(), 0);
}

#[test]
fn test_memoir_search_missing_query_fails() {
    let store = test_store();
    let resp = handle_memoir_search(json!(1), Some(json!({ "memoir": "test" })), &store);

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_search_missing_memoir_fails() {
    let store = test_store();
    let resp = handle_memoir_search(json!(1), Some(json!({ "query": "test" })), &store);

    assert_error(&resp, -32602);
}

// ============================================================
// memoir_search_all
// ============================================================

#[test]
fn test_memoir_search_all_across_memoirs() {
    let store = test_store();

    // Create two memoirs with concepts
    handle_memoir_create(json!(1), Some(json!({ "name": "memoir-a" })), &store);
    handle_memoir_create(json!(2), Some(json!({ "name": "memoir-b" })), &store);

    handle_memoir_add_concept(
        json!(3),
        Some(
            json!({ "memoir": "memoir-a", "name": "Generics", "definition": "Generic type parameters" }),
        ),
        &store,
    );
    handle_memoir_add_concept(
        json!(4),
        Some(
            json!({ "memoir": "memoir-b", "name": "Generic Programming", "definition": "Writing code with type parameters" }),
        ),
        &store,
    );

    let resp = handle_memoir_search_all(json!(5), Some(json!({ "query": "generic" })), &store);

    assert_success(&resp);
    let result = resp.result.unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert!(matches.len() >= 1, "Expected cross-memoir results");
}

#[test]
fn test_memoir_search_all_missing_query_fails() {
    let store = test_store();
    let resp = handle_memoir_search_all(json!(1), Some(json!({})), &store);

    assert_error(&resp, -32602);
}

// ============================================================
// memoir_link
// ============================================================

#[test]
fn test_memoir_link_basic() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "link-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "link-memoir", "name": "Ownership" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(3),
        Some(json!({ "memoir": "link-memoir", "name": "Borrowing" })),
        &store,
    );

    let resp = handle_memoir_link(
        json!(4),
        Some(json!({
            "memoir": "link-memoir",
            "source": "Borrowing",
            "target": "Ownership",
            "relation": "prerequisite_of"
        })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    assert_eq!(result["relation"], "prerequisite_of");
    assert!(result["id"].is_string());
}

#[test]
fn test_memoir_link_with_weight() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "weight-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "weight-memoir", "name": "A" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(3),
        Some(json!({ "memoir": "weight-memoir", "name": "B" })),
        &store,
    );

    let resp = handle_memoir_link(
        json!(4),
        Some(json!({
            "memoir": "weight-memoir",
            "source": "A",
            "target": "B",
            "relation": "related_to",
            "weight": 0.75
        })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    assert_eq!(result["weight"], 0.75);
}

#[test]
fn test_memoir_link_all_relation_types() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "rel-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "rel-memoir", "name": "Source" })),
        &store,
    );

    let relations = [
        "is_a",
        "has_property",
        "related_to",
        "causes",
        "prerequisite_of",
        "example_of",
        "contradicts",
        "similar_to",
        "part_of",
    ];

    for (i, rel) in relations.iter().enumerate() {
        let target_name = format!("Target{}", i);
        handle_memoir_add_concept(
            json!(10 + i),
            Some(json!({ "memoir": "rel-memoir", "name": target_name })),
            &store,
        );

        let resp = handle_memoir_link(
            json!(100 + i),
            Some(json!({
                "memoir": "rel-memoir",
                "source": "Source",
                "target": target_name,
                "relation": rel
            })),
            &store,
        );

        assert_success(&resp);
        let result = resp.result.unwrap();
        assert_eq!(
            result["relation"].as_str().unwrap(),
            *rel,
            "Relation type mismatch for {}",
            rel
        );
    }
}

#[test]
fn test_memoir_link_invalid_relation_fails() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "bad-rel" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "bad-rel", "name": "A" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(3),
        Some(json!({ "memoir": "bad-rel", "name": "B" })),
        &store,
    );

    let resp = handle_memoir_link(
        json!(4),
        Some(json!({
            "memoir": "bad-rel",
            "source": "A",
            "target": "B",
            "relation": "InvalidRelation"
        })),
        &store,
    );

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_link_missing_source_fails() {
    let store = test_store();
    let resp = handle_memoir_link(
        json!(1),
        Some(json!({
            "memoir": "m",
            "target": "B",
            "relation": "related_to"
        })),
        &store,
    );

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_link_missing_relation_fails() {
    let store = test_store();
    let resp = handle_memoir_link(
        json!(1),
        Some(json!({
            "memoir": "m",
            "source": "A",
            "target": "B"
        })),
        &store,
    );

    assert_error(&resp, -32602);
}

// ============================================================
// memoir_inspect
// ============================================================

#[test]
fn test_memoir_inspect_single_depth() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "inspect-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "inspect-memoir", "name": "Center" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(3),
        Some(json!({ "memoir": "inspect-memoir", "name": "Neighbor1" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(4),
        Some(json!({ "memoir": "inspect-memoir", "name": "Neighbor2" })),
        &store,
    );

    handle_memoir_link(
        json!(5),
        Some(json!({
            "memoir": "inspect-memoir",
            "source": "Center",
            "target": "Neighbor1",
            "relation": "related_to"
        })),
        &store,
    );
    handle_memoir_link(
        json!(6),
        Some(json!({
            "memoir": "inspect-memoir",
            "source": "Center",
            "target": "Neighbor2",
            "relation": "has_property"
        })),
        &store,
    );

    let resp = handle_memoir_inspect(
        json!(7),
        Some(json!({ "memoir": "inspect-memoir", "concept": "Center", "depth": 1 })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    assert_eq!(result["concept"]["name"], "Center");

    let neighbors = result["neighbors"].as_array().unwrap();
    assert_eq!(neighbors.len(), 2, "Expected 2 neighbors at depth 1");
}

#[test]
fn test_memoir_inspect_deeper_traversal() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "deep-memoir" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "deep-memoir", "name": "A" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(3),
        Some(json!({ "memoir": "deep-memoir", "name": "B" })),
        &store,
    );
    handle_memoir_add_concept(
        json!(4),
        Some(json!({ "memoir": "deep-memoir", "name": "C" })),
        &store,
    );

    // A -> B -> C
    handle_memoir_link(
        json!(5),
        Some(
            json!({ "memoir": "deep-memoir", "source": "A", "target": "B", "relation": "related_to" }),
        ),
        &store,
    );
    handle_memoir_link(
        json!(6),
        Some(
            json!({ "memoir": "deep-memoir", "source": "B", "target": "C", "relation": "related_to" }),
        ),
        &store,
    );

    // Depth 1 from A: should see B but not C
    let resp1 = handle_memoir_inspect(
        json!(7),
        Some(json!({ "memoir": "deep-memoir", "concept": "A", "depth": 1 })),
        &store,
    );
    assert_success(&resp1);
    let neighbors1 = resp1.result.unwrap()["neighbors"]
        .as_array()
        .unwrap()
        .clone();
    let names1: Vec<&str> = neighbors1
        .iter()
        .filter_map(|n| n["concept"]["name"].as_str())
        .collect();
    assert!(names1.contains(&"B"), "Depth 1 should include B");

    // Depth 2 from A: should see B and C
    let resp2 = handle_memoir_inspect(
        json!(8),
        Some(json!({ "memoir": "deep-memoir", "concept": "A", "depth": 2 })),
        &store,
    );
    assert_success(&resp2);
    let neighbors2 = resp2.result.unwrap()["neighbors"]
        .as_array()
        .unwrap()
        .clone();
    let names2: Vec<&str> = neighbors2
        .iter()
        .filter_map(|n| n["concept"]["name"].as_str())
        .collect();
    assert!(names2.contains(&"B"), "Depth 2 should include B");
    assert!(names2.contains(&"C"), "Depth 2 should include C");
}

#[test]
fn test_memoir_inspect_isolated_concept() {
    let store = test_store();

    handle_memoir_create(json!(1), Some(json!({ "name": "isolated" })), &store);
    handle_memoir_add_concept(
        json!(2),
        Some(json!({ "memoir": "isolated", "name": "Lonely" })),
        &store,
    );

    let resp = handle_memoir_inspect(
        json!(3),
        Some(json!({ "memoir": "isolated", "concept": "Lonely" })),
        &store,
    );

    assert_success(&resp);
    let result = resp.result.unwrap();
    let neighbors = result["neighbors"].as_array().unwrap();
    assert_eq!(
        neighbors.len(),
        0,
        "Isolated concept should have no neighbors"
    );
}

#[test]
fn test_memoir_inspect_missing_memoir_fails() {
    let store = test_store();
    let resp = handle_memoir_inspect(json!(1), Some(json!({ "concept": "X" })), &store);

    assert_error(&resp, -32602);
}

#[test]
fn test_memoir_inspect_missing_concept_fails() {
    let store = test_store();
    let resp = handle_memoir_inspect(json!(1), Some(json!({ "memoir": "m" })), &store);

    assert_error(&resp, -32602);
}

// ============================================================
// Edge cases: missing params
// ============================================================

#[test]
fn test_all_memoir_tools_reject_none_params() {
    let store = test_store();

    // All tools that require params should fail with None
    let tools_needing_params: Vec<(
        &str,
        fn(Value, Option<Value>, &SqliteStore) -> alejandria_mcp::JsonRpcResponse,
    )> = vec![
        ("memoir_create", handle_memoir_create),
        ("memoir_show", handle_memoir_show),
        ("memoir_add_concept", handle_memoir_add_concept),
        ("memoir_refine", handle_memoir_refine),
        ("memoir_search", handle_memoir_search),
        ("memoir_search_all", handle_memoir_search_all),
        ("memoir_link", handle_memoir_link),
        ("memoir_inspect", handle_memoir_inspect),
    ];

    for (name, handler) in tools_needing_params {
        let resp = handler(json!(1), None, &store);
        assert!(resp.error.is_some(), "{} should reject None params", name);
    }
}

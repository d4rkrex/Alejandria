//! Memoir tool handlers for knowledge graph operations.
//!
//! This module provides MCP tool handlers for memoir (knowledge graph) operations including:
//! - Creating and listing memoirs
//! - Adding and updating concepts
//! - Creating typed links between concepts
//! - Searching concepts (within memoir or across all memoirs)
//! - Inspecting concept neighborhoods with BFS traversal

use alejandria_core::{
    memoir_store::{ConceptUpdate, MemoirStore, NewConcept, NewConceptLink, NewMemoir},
    RelationType,
};
use serde_json::{json, Value};
use std::str::FromStr;

use crate::protocol::{JsonRpcError, JsonRpcResponse};

/// Handle memoir_create tool - create a new memoir
pub fn handle_memoir_create<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let name = match params["name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: name"),
            )
        }
    };

    let description = params["description"].as_str().unwrap_or("").to_string();

    match store.create_memoir(NewMemoir { name, description }) {
        Ok(memoir) => JsonRpcResponse::success(
            id,
            json!({
                "id": memoir.id,
                "name": memoir.name,
                "description": memoir.description,
                "created_at": memoir.created_at.to_rfc3339(),
                "updated_at": memoir.updated_at.to_rfc3339(),
                "metadata": memoir.metadata,
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to create memoir: {}", e)),
        ),
    }
}

/// Handle memoir_list tool - list all memoirs with summary stats
pub fn handle_memoir_list<S: MemoirStore>(
    id: Value,
    _params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    match store.list_memoirs() {
        Ok(memoirs) => JsonRpcResponse::success(
            id,
            json!({
                "memoirs": memoirs.iter().map(|m| json!({
                    "id": m.id,
                    "name": m.name,
                    "description": m.description,
                    "concept_count": m.concept_count,
                    "link_count": m.link_count,
                    "created_at": m.created_at.to_rfc3339(),
                    "updated_at": m.updated_at.to_rfc3339(),
                })).collect::<Vec<_>>()
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to list memoirs: {}", e)),
        ),
    }
}

/// Handle memoir_show tool - get full memoir graph with concepts and links
pub fn handle_memoir_show<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let name = match params["name"].as_str() {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: name"),
            )
        }
    };

    match store.get_memoir(name) {
        Ok(Some(detail)) => JsonRpcResponse::success(
            id,
            json!({
                "memoir": {
                    "id": detail.id,
                    "name": detail.name,
                    "description": detail.description,
                    "created_at": detail.created_at.to_rfc3339(),
                    "updated_at": detail.updated_at.to_rfc3339(),
                },
                "concepts": detail.concepts.iter().map(|c| json!({
                    "id": c.id,
                    "name": c.name,
                    "definition": c.definition,
                    "labels": c.labels,
                    "created_at": c.created_at.to_rfc3339(),
                    "updated_at": c.updated_at.to_rfc3339(),
                    "metadata": c.metadata,
                })).collect::<Vec<_>>(),
                "links": detail.links.iter().map(|l| json!({
                    "id": l.id,
                    "source": l.source,
                    "relation": l.relation.as_str(),
                    "target": l.target,
                    "weight": l.weight,
                })).collect::<Vec<_>>(),
            }),
        ),
        Ok(None) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Memoir '{}' not found", name)),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to get memoir: {}", e)),
        ),
    }
}

/// Handle memoir_add_concept tool - add a concept to a memoir
pub fn handle_memoir_add_concept<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let memoir_name = match params["memoir"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: memoir"),
            )
        }
    };

    let name = match params["name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: name"),
            )
        }
    };

    let definition = params["definition"].as_str().unwrap_or("").to_string();

    let labels = params["labels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    match store.add_concept(NewConcept {
        memoir_name,
        name,
        definition,
        labels,
    }) {
        Ok(concept) => JsonRpcResponse::success(
            id,
            json!({
                "id": concept.id,
                "memoir_id": concept.memoir_id,
                "name": concept.name,
                "definition": concept.definition,
                "labels": concept.labels,
                "created_at": concept.created_at.to_rfc3339(),
                "updated_at": concept.updated_at.to_rfc3339(),
                "metadata": concept.metadata,
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to add concept: {}", e)),
        ),
    }
}

/// Handle memoir_refine tool - update concept definition and/or labels
pub fn handle_memoir_refine<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let memoir = match params["memoir"].as_str() {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: memoir"),
            )
        }
    };

    let concept = match params["concept"].as_str() {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: concept"),
            )
        }
    };

    let definition = params["definition"].as_str().map(|s| s.to_string());

    let labels = params["labels"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    });

    match store.update_concept(memoir, concept, ConceptUpdate { definition, labels }) {
        Ok(()) => JsonRpcResponse::success(id, json!({"success": true})),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to update concept: {}", e)),
        ),
    }
}

/// Handle memoir_search tool - search concepts within a memoir
pub fn handle_memoir_search<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let memoir = match params["memoir"].as_str() {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: memoir"),
            )
        }
    };

    let query = match params["query"].as_str() {
        Some(q) => q,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: query"),
            )
        }
    };

    let limit = params["limit"].as_u64().unwrap_or(10) as u32;

    match store.search_concepts(memoir, query, limit) {
        Ok(concepts) => JsonRpcResponse::success(
            id,
            json!({
                "concepts": concepts.iter().map(|c| json!({
                    "id": c.id,
                    "name": c.name,
                    "definition": c.definition,
                    "labels": c.labels,
                    "created_at": c.created_at.to_rfc3339(),
                    "updated_at": c.updated_at.to_rfc3339(),
                    "metadata": c.metadata,
                })).collect::<Vec<_>>()
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to search concepts: {}", e)),
        ),
    }
}

/// Handle memoir_search_all tool - search concepts across all memoirs
pub fn handle_memoir_search_all<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let query = match params["query"].as_str() {
        Some(q) => q,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: query"),
            )
        }
    };

    let limit = params["limit"].as_u64().unwrap_or(10) as u32;

    match store.search_concepts_all(query, limit) {
        Ok(matches) => JsonRpcResponse::success(
            id,
            json!({
                "matches": matches.iter().map(|m| json!({
                    "memoir_name": m.memoir_name,
                    "concept": {
                        "id": m.concept.id,
                        "name": m.concept.name,
                        "definition": m.concept.definition,
                        "labels": m.concept.labels,
                        "created_at": m.concept.created_at.to_rfc3339(),
                        "updated_at": m.concept.updated_at.to_rfc3339(),
                        "metadata": m.concept.metadata,
                    }
                })).collect::<Vec<_>>()
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to search all concepts: {}", e)),
        ),
    }
}

/// Handle memoir_link tool - create typed link between concepts
pub fn handle_memoir_link<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let memoir_name = match params["memoir"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: memoir"),
            )
        }
    };

    let source_name = match params["source"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: source"),
            )
        }
    };

    let target_name = match params["target"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: target"),
            )
        }
    };

    let relation_str = match params["relation"].as_str() {
        Some(r) => r,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: relation"),
            )
        }
    };

    let relation = match RelationType::from_str(relation_str) {
        Ok(r) => r,
        Err(_) => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params(format!(
                    "Invalid relation type '{}'. Must be one of: IsA, HasProperty, RelatedTo, Causes, PrerequisiteOf, ExampleOf, Contradicts, SimilarTo, PartOf",
                    relation_str
                ))
            )
        }
    };

    let weight = params["weight"].as_f64().unwrap_or(1.0) as f32;

    match store.link_concepts(NewConceptLink {
        memoir_name,
        source_name,
        target_name,
        relation,
        weight,
    }) {
        Ok(link) => JsonRpcResponse::success(
            id,
            json!({
                "id": link.id,
                "source_id": link.source_id,
                "target_id": link.target_id,
                "relation": link.relation.as_str(),
                "weight": link.weight,
                "created_at": link.created_at.to_rfc3339(),
                "metadata": link.metadata,
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to create link: {}", e)),
        ),
    }
}

/// Handle memoir_inspect tool - inspect concept neighborhood with BFS traversal
pub fn handle_memoir_inspect<S: MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: &S,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing parameters"));
    };

    let memoir = match params["memoir"].as_str() {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: memoir"),
            )
        }
    };

    let concept = match params["concept"].as_str() {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing required field: concept"),
            )
        }
    };

    let depth = params["depth"].as_u64().unwrap_or(1) as u8;

    match store.inspect_concept(memoir, concept, depth) {
        Ok(neighborhood) => JsonRpcResponse::success(
            id,
            json!({
                "concept": {
                    "id": neighborhood.concept.id,
                    "name": neighborhood.concept.name,
                    "definition": neighborhood.concept.definition,
                    "labels": neighborhood.concept.labels,
                    "created_at": neighborhood.concept.created_at.to_rfc3339(),
                    "updated_at": neighborhood.concept.updated_at.to_rfc3339(),
                    "metadata": neighborhood.concept.metadata,
                },
                "neighbors": neighborhood.neighbors.iter().map(|n| json!({
                    "concept": {
                        "id": n.concept.id,
                        "name": n.concept.name,
                        "definition": n.concept.definition,
                        "labels": n.concept.labels,
                    },
                    "direction": match n.direction {
                        alejandria_core::memoir_store::LinkDirection::Outgoing => "outgoing",
                        alejandria_core::memoir_store::LinkDirection::Incoming => "incoming",
                    },
                    "relation": n.relation.as_str(),
                    "weight": n.weight,
                    "level": n.level,
                })).collect::<Vec<_>>(),
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            JsonRpcError::internal_error(format!("Failed to inspect concept: {}", e)),
        ),
    }
}

use alejandria_core::{
    ConceptUpdate, MemoirStore, NewConcept, NewConceptLink, NewMemoir, RelationType,
};
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn create(name: String, description: String, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let new_memoir = NewMemoir {
        name: name.clone(),
        description,
    };

    let memoir = store
        .create_memoir(new_memoir)
        .context("Failed to create memoir")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": memoir.id,
                "name": name,
                "success": true
            }))?
        );
    } else {
        println!("Created memoir: {} (ID: {})", name, memoir.id);
    }

    Ok(())
}

pub fn list(json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let memoirs = store.list_memoirs().context("Failed to list memoirs")?;

    if json_output {
        let results: Vec<_> = memoirs
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "name": m.name,
                    "description": m.description,
                    "concept_count": m.concept_count,
                    "link_count": m.link_count,
                    "created_at": m.created_at.to_rfc3339(),
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "memoirs": results,
                "total": memoirs.len()
            }))?
        );
    } else {
        if memoirs.is_empty() {
            println!("No memoirs found");
        } else {
            println!("Memoirs:\n");
            for memoir in memoirs {
                println!("  {} - {}", memoir.name, memoir.description);
                println!("    ID: {}", memoir.id);
                println!(
                    "    Concepts: {}, Links: {}",
                    memoir.concept_count, memoir.link_count
                );
                println!(
                    "    Created: {}",
                    memoir.created_at.format("%Y-%m-%d %H:%M")
                );
                println!();
            }
        }
    }

    Ok(())
}

pub fn show(name: String, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let memoir = store
        .get_memoir(&name)
        .context("Failed to get memoir")?
        .ok_or_else(|| anyhow::anyhow!("Memoir not found: {}", name))?;

    if json_output {
        let concepts_json: Vec<_> = memoir
            .concepts
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "name": c.name,
                    "definition": c.definition,
                    "labels": c.labels,
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "memoir": {
                    "id": memoir.id,
                    "name": memoir.name,
                    "description": memoir.description,
                },
                "concepts": concepts_json,
                "total_concepts": memoir.concepts.len()
            }))?
        );
    } else {
        println!("Memoir: {}", memoir.name);
        println!("Description: {}", memoir.description);
        println!();
        println!("Concepts ({}):", memoir.concepts.len());
        for concept in memoir.concepts {
            println!();
            println!("  {} - {}", concept.name, concept.definition);
            println!("    ID: {}", concept.id);
            if !concept.labels.is_empty() {
                println!("    Labels: {}", concept.labels.join(", "));
            }
        }
    }

    Ok(())
}

pub fn add_concept(
    memoir: String,
    name: String,
    definition: String,
    labels: Option<String>,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let labels_vec = labels
        .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let new_concept = NewConcept {
        memoir_name: memoir.clone(),
        name: name.clone(),
        definition,
        labels: labels_vec,
    };

    let concept = store
        .add_concept(new_concept)
        .context("Failed to add concept")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": concept.id,
                "name": name,
                "memoir": memoir,
                "success": true
            }))?
        );
    } else {
        println!(
            "Added concept '{}' to memoir '{}' (ID: {})",
            name, memoir, concept.id
        );
    }

    Ok(())
}

pub fn refine(
    memoir: String,
    concept: String,
    definition: Option<String>,
    labels: Option<String>,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let updates = ConceptUpdate {
        definition,
        labels: labels.map(|l| l.split(',').map(|s| s.trim().to_string()).collect()),
    };

    store
        .update_concept(&memoir, &concept, updates)
        .context("Failed to update concept")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "memoir": memoir,
                "concept": concept,
                "success": true
            }))?
        );
    } else {
        println!("Refined concept '{}' in memoir '{}'", concept, memoir);
    }

    Ok(())
}

pub fn search(memoir: String, query: String, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let results = store
        .search_concepts(&memoir, &query, 20)
        .context("Failed to search concepts")?;

    if json_output {
        let concepts_json: Vec<_> = results
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "name": c.name,
                    "definition": c.definition,
                    "labels": c.labels,
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "memoir": memoir,
                "results": concepts_json,
                "total": results.len()
            }))?
        );
    } else {
        if results.is_empty() {
            println!("No concepts found matching '{}'", query);
        } else {
            println!("Found {} concepts in '{}':\n", results.len(), memoir);
            for concept in results {
                println!("  {} - {}", concept.name, concept.definition);
                println!("    ID: {}", concept.id);
                println!();
            }
        }
    }

    Ok(())
}

pub fn search_all(query: String, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let results = store
        .search_concepts_all(&query, 20)
        .context("Failed to search all concepts")?;

    if json_output {
        let concepts_json: Vec<_> = results
            .iter()
            .map(|m| {
                json!({
                    "id": m.concept.id,
                    "name": m.concept.name,
                    "definition": m.concept.definition,
                    "memoir_name": m.memoir_name,
                    "labels": m.concept.labels,
                    "score": m.score,
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "results": concepts_json,
                "total": results.len()
            }))?
        );
    } else {
        if results.is_empty() {
            println!("No concepts found matching '{}'", query);
        } else {
            println!("Found {} concepts across all memoirs:\n", results.len());
            for m in results {
                println!("  {} - {}", m.concept.name, m.concept.definition);
                println!("    ID: {}", m.concept.id);
                println!("    Memoir: {}", m.memoir_name);
                println!("    Score: {:.2}", m.score);
                println!();
            }
        }
    }

    Ok(())
}

pub fn link(
    memoir: String,
    source: String,
    target: String,
    relation: String,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let relation_enum = parse_relation(&relation)?;

    let new_link = NewConceptLink {
        memoir_name: memoir.clone(),
        source_name: source.clone(),
        target_name: target.clone(),
        relation: relation_enum,
        weight: 1.0,
    };

    let link = store
        .link_concepts(new_link)
        .context("Failed to link concepts")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": link.id,
                "memoir": memoir,
                "source": source,
                "target": target,
                "relation": relation,
                "success": true
            }))?
        );
    } else {
        println!(
            "Linked concepts in '{}': {} --[{}]--> {}",
            memoir, source, relation, target
        );
        println!("Link ID: {}", link.id);
    }

    Ok(())
}

pub fn inspect(memoir: String, concept: String, depth: usize, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    let neighborhood = store
        .inspect_concept(&memoir, &concept, depth as u8)
        .context("Failed to get concept neighborhood")?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&neighborhood)?);
    } else {
        println!(
            "Concept '{}' in memoir '{}' (depth {}):\n",
            concept, memoir, depth
        );
        println!("Total neighbors: {}", neighborhood.neighbors.len());

        for neighbor in neighborhood.neighbors {
            println!();
            println!(
                "  {} - {}",
                neighbor.concept.name, neighbor.concept.definition
            );
            println!("    Relation: {:?}", neighbor.relation);
            println!("    Direction: {:?}", neighbor.direction);
            println!("    Weight: {:.2}", neighbor.weight);
            println!("    Level: {}", neighbor.level);
        }
    }

    Ok(())
}

fn parse_relation(s: &str) -> Result<RelationType> {
    use std::str::FromStr;
    RelationType::from_str(s).map_err(|e| anyhow::anyhow!("{}", e))
}

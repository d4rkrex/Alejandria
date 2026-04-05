use alejandria_core::MemoryStore;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(
    query: String,
    limit: usize,
    topic: Option<String>,
    min_score: f32,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // For now, hybrid_search_with_fallback requires embedding and HybridConfig
    // Since we don't have embeddings generated yet, we'll use FTS search
    // This will be enhanced once embedding generation is implemented

    let mut memories = if let Some(topic_filter) = &topic {
        store
            .get_by_topic(topic_filter, Some(limit), None)
            .context("Failed to search by topic")?
    } else {
        // Use FTS search as fallback
        store
            .search_by_keywords(&query, limit)
            .context("Failed to search memories")?
    };

    // Filter by minimum score (using weight as proxy for score)
    memories.retain(|m| m.weight >= min_score);

    if json_output {
        let results: Vec<_> = memories
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "topic": m.topic,
                    "summary": m.summary,
                    "importance": format!("{:?}", m.importance).to_lowercase(),
                    "weight": m.weight,
                    "created_at": m.created_at.to_rfc3339(),
                    "revision_count": m.revision_count,
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "memories": results,
                "total_found": memories.len()
            }))?
        );
    } else {
        if memories.is_empty() {
            println!("No memories found matching '{}'", query);
        } else {
            println!("Found {} memories:\n", memories.len());
            for (i, memory) in memories.iter().enumerate() {
                println!("{}. [{:?}] {}", i + 1, memory.importance, memory.summary);
                println!("   ID: {}", memory.id);
                println!("   Topic: {}", memory.topic);
                println!("   Weight: {:.2}", memory.weight);
                println!("   Created: {}", memory.created_at.format("%Y-%m-%d %H:%M"));
                if memory.revision_count > 1 {
                    println!("   Revisions: {}", memory.revision_count);
                }
                println!();
            }
        }
    }

    Ok(())
}

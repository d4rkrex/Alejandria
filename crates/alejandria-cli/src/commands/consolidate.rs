use alejandria_core::MemoryStore;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(topic: String, min_memories: usize, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Get memories for the topic
    let memories = store
        .get_by_topic(&topic, None, None)
        .context(format!("Failed to get memories for topic: {}", topic))?;

    if memories.len() < min_memories {
        anyhow::bail!(
            "Topic '{}' has {} memories, but at least {} are required for consolidation",
            topic,
            memories.len(),
            min_memories
        );
    }

    // For MVP, we just report that consolidation would happen
    // In Phase 2, this would call an LLM to actually consolidate
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "topic": topic,
                "memory_count": memories.len(),
                "consolidation_ready": true,
                "message": "Consolidation feature requires LLM integration (Phase 2)"
            }))?
        );
    } else {
        println!(
            "Topic '{}' has {} memories ready for consolidation",
            topic,
            memories.len()
        );
        println!();
        println!("Note: Consolidation feature requires LLM integration (Phase 2)");
        println!("In the future, this will:");
        println!("  1. Summarize all memories in the topic");
        println!("  2. Create a consolidated memory");
        println!("  3. Soft-delete the original memories");
    }

    Ok(())
}

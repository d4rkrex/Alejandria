use alejandria_core::{Importance, MemoryStore};
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(
    id: String,
    summary: Option<String>,
    importance: Option<String>,
    topic: Option<String>,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Get existing memory
    let mut memory = store
        .get(&id)
        .context("Failed to retrieve memory")?
        .ok_or_else(|| anyhow::anyhow!("Memory not found: {}", id))?;

    // Apply updates
    if let Some(new_summary) = summary {
        memory.summary = new_summary;
    }

    if let Some(new_importance) = importance {
        memory.importance = parse_importance(&new_importance)?;
    }

    if let Some(new_topic) = topic {
        memory.topic = new_topic;
    }

    // Update the memory
    store
        .update(memory.clone())
        .context("Failed to update memory")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "updated": true
            }))?
        );
    } else {
        println!("Updated memory: {}", id);
    }

    Ok(())
}

fn parse_importance(s: &str) -> Result<Importance> {
    match s.to_lowercase().as_str() {
        "critical" => Ok(Importance::Critical),
        "high" => Ok(Importance::High),
        "medium" => Ok(Importance::Medium),
        "low" => Ok(Importance::Low),
        _ => anyhow::bail!(
            "Invalid importance level: {}. Must be one of: critical, high, medium, low",
            s
        ),
    }
}

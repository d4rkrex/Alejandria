use alejandria_core::{Importance, Memory, MemoryStore};
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(
    content: String,
    summary: Option<String>,
    topic: String,
    importance: String,
    topic_key: Option<String>,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Parse importance
    let importance = parse_importance(&importance)?;

    // Generate summary if not provided
    let summary = summary.unwrap_or_else(|| {
        if content.len() > 100 {
            format!("{}...", &content[..97])
        } else {
            content.clone()
        }
    });

    // Create memory - Memory::new takes (topic, summary)
    let mut memory = Memory::new(topic.clone(), summary);

    // Set additional fields
    memory.raw_excerpt = Some(content.clone());
    memory.importance = importance;
    memory.topic_key = topic_key.clone();

    // Store the memory
    let id = store.store(memory).context("Failed to store memory")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "topic": topic,
                "importance": importance.to_string().to_lowercase(),
                "topic_key": topic_key,
                "success": true
            }))?
        );
    } else {
        println!("Stored memory: {}", id);
        if let Some(key) = topic_key {
            println!("Topic key: {}", key);
        }
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

use alejandria_core::MemoryStore;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;

use crate::config::Config;

pub fn run(min_count: usize, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Get all topics - list_topics takes Option<usize> for limit and offset
    let topics_info = store
        .list_topics(None, None)
        .context("Failed to list topics")?;

    // Filter by min_count and collect
    let mut topic_counts: HashMap<String, usize> = HashMap::new();
    for topic_info in topics_info {
        if topic_info.count >= min_count {
            topic_counts.insert(topic_info.topic, topic_info.count);
        }
    }

    if json_output {
        let mut topics_list: Vec<_> = topic_counts
            .iter()
            .map(|(topic, count)| {
                json!({
                    "topic": topic,
                    "count": count
                })
            })
            .collect();
        topics_list.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap()
                .cmp(&a["count"].as_u64().unwrap())
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "topics": topics_list,
                "total": topics_list.len()
            }))?
        );
    } else {
        if topic_counts.is_empty() {
            println!("No topics found with at least {} memories", min_count);
        } else {
            println!("Topics (with at least {} memories):\n", min_count);
            let mut sorted: Vec<_> = topic_counts.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));

            for (topic, count) in sorted {
                println!("  {} ({} memories)", topic, count);
            }
        }
    }

    Ok(())
}

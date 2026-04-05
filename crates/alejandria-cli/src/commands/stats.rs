use alejandria_core::MemoryStore;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Get statistics
    let stats = store.stats().context("Failed to get statistics")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "total_memories": stats.total_memories,
                "active_memories": stats.active_memories,
                "deleted_memories": stats.deleted_memories,
                "total_size_mb": stats.total_size_mb,
                "avg_weight": stats.avg_weight,
                "embeddings_enabled": stats.embeddings_enabled,
                "last_decay_at": stats.last_decay_at.map(|d| d.to_rfc3339()),
                "by_importance": {
                    "critical": stats.by_importance.critical,
                    "high": stats.by_importance.high,
                    "medium": stats.by_importance.medium,
                    "low": stats.by_importance.low,
                },
                "by_source": {
                    "user": stats.by_source.user,
                    "agent": stats.by_source.agent,
                    "system": stats.by_source.system,
                    "external": stats.by_source.external,
                }
            }))?
        );
    } else {
        println!("Memory Statistics:");
        println!();
        println!("  Total memories: {}", stats.total_memories);
        println!("  Active memories: {}", stats.active_memories);
        println!("  Deleted memories: {}", stats.deleted_memories);
        println!("  Database size: {:.2} MB", stats.total_size_mb);
        println!("  Average weight: {:.2}", stats.avg_weight);
        println!(
            "  Embeddings: {}",
            if stats.embeddings_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );

        if let Some(last_decay) = stats.last_decay_at {
            println!("  Last decay: {}", last_decay.format("%Y-%m-%d %H:%M"));
        }

        println!();
        println!("  By Importance:");
        println!("    Critical: {}", stats.by_importance.critical);
        println!("    High: {}", stats.by_importance.high);
        println!("    Medium: {}", stats.by_importance.medium);
        println!("    Low: {}", stats.by_importance.low);

        println!();
        println!("  By Source:");
        println!("    User: {}", stats.by_source.user);
        println!("    Agent: {}", stats.by_source.agent);
        println!("    System: {}", stats.by_source.system);
        println!("    External: {}", stats.by_source.external);
    }

    Ok(())
}

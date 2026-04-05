use alejandria_core::MemoryStore;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(force: bool, json_output: bool) -> Result<()> {
    let config = Config::load()?;

    if !config.decay.auto_decay && !force {
        anyhow::bail!("Auto decay is disabled. Use --force to apply decay anyway.");
    }

    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Apply decay with default rate (0.01 per day)
    let updated_count = store.apply_decay(0.01).context("Failed to apply decay")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "memories_updated": updated_count,
                "success": true
            }))?
        );
    } else {
        println!("Applied decay to {} memories", updated_count);

        // Show some statistics after decay
        let stats = store.stats().context("Failed to get statistics")?;
        println!();
        println!("New average weight: {:.2}", stats.avg_weight);

        // Optionally prune low-weight memories
        if stats.avg_weight < config.decay.prune_threshold {
            println!();
            println!(
                "Tip: Average weight is below {:.2}. Consider running:",
                config.decay.prune_threshold
            );
            println!(
                "  alejandria prune --threshold {}",
                config.decay.prune_threshold
            );
        }
    }

    Ok(())
}

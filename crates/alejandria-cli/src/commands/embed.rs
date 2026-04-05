use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(batch_size: usize, skip_existing: bool, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Call embed_all with parameters
    let embedded_count = store
        .embed_all(batch_size, skip_existing)
        .context("Failed to embed memories")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "memories_embedded": embedded_count,
                "batch_size": batch_size,
                "success": true
            }))?
        );
    } else {
        println!("Embedded {} memories", embedded_count);
        println!("Batch size: {}", batch_size);
        if skip_existing {
            println!("(Skipped memories with existing embeddings)");
        }
    }

    Ok(())
}

use alejandria_core::MemoryStore;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;

use crate::config::Config;

pub fn run(id: String, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Soft-delete the memory
    store.delete(&id).context("Failed to delete memory")?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "deleted": true
            }))?
        );
    } else {
        println!("Deleted memory: {}", id);
    }

    Ok(())
}

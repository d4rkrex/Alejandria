use alejandria_mcp::server::run_stdio_server;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};

use crate::config::Config;

pub fn run() -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;

    eprintln!("Starting Alejandria MCP server...");
    eprintln!("Database: {}", db_path.display());
    eprintln!();

    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    run_stdio_server(store).context("MCP server error")?;

    Ok(())
}

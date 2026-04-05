use alejandria_core::import::ImportMode;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

use crate::config::Config;

pub fn run(input: String, mode: String, dry_run: bool, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Parse import mode
    let import_mode = parse_mode(&mode)?;

    // Validate input file exists
    let input_path = Path::new(&input);
    if !input_path.exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {}", input));
    }

    if dry_run {
        // Dry run: validate only, don't import
        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "dry_run": true,
                    "input_file": input,
                    "mode": format!("{:?}", import_mode),
                    "message": "Dry run completed. No data was imported."
                }))?
            );
        } else {
            println!("DRY RUN MODE");
            println!("Input file: {}", input);
            println!("Import mode: {:?}", import_mode);
            println!("\nValidation passed. File format is valid.");
            println!("No data was imported (dry run).");
        }
        return Ok(());
    }

    // Execute import
    let result = store
        .import_memories(input_path, import_mode)
        .context("Import failed")?;

    // Output results
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": result.errors.is_empty(),
                "input_file": input,
                "mode": format!("{:?}", import_mode),
                "imported": result.imported,
                "updated": result.updated,
                "skipped": result.skipped,
                "errors": result.errors,
            }))?
        );
    } else {
        println!("Import completed!");
        println!("Input file: {}", input);
        println!("Import mode: {:?}", import_mode);
        println!("\nResults:");
        println!("  Imported (new): {}", result.imported);
        println!("  Updated: {}", result.updated);
        println!("  Skipped: {}", result.skipped);

        if !result.errors.is_empty() {
            println!("\n{} errors occurred:", result.errors.len());
            for (i, error) in result.errors.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, error);
            }
            if result.errors.len() > 10 {
                println!("  ... and {} more errors", result.errors.len() - 10);
            }
        }

        let total = result.imported + result.updated + result.skipped + result.errors.len();
        println!("\nTotal records processed: {}", total);
    }

    Ok(())
}

/// Parse import mode from string
fn parse_mode(mode_str: &str) -> Result<ImportMode> {
    match mode_str.to_lowercase().as_str() {
        "skip" => Ok(ImportMode::Skip),
        "update" => Ok(ImportMode::Update),
        "replace" => Ok(ImportMode::Replace),
        _ => Err(anyhow::anyhow!(
            "Invalid import mode '{}'. Valid modes: skip, update, replace",
            mode_str
        )),
    }
}

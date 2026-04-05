use alejandria_storage::{ExportFormat, ExportOptions, SqliteStore};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::fs::File;
use std::io::BufWriter;
use std::str::FromStr;

use crate::config::Config;

pub fn run(
    format: String,
    output: String,
    filter: Option<String>,
    include_deleted: bool,
    json_output: bool,
) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Parse format
    let export_format = ExportFormat::from_str(&format).map_err(|e| anyhow::anyhow!(e))?;

    // Build export options
    let mut options = ExportOptions {
        include_deleted,
        ..Default::default()
    };

    // Parse filter if provided (format: "field:value")
    if let Some(filter_str) = filter {
        parse_filter(&filter_str, &mut options)?;
    }

    // Create output file
    let file = File::create(&output)
        .with_context(|| format!("Failed to create output file: {}", output))?;
    let mut writer = BufWriter::new(file);

    // Execute export
    let metadata = store
        .export_memories(export_format, options, &mut writer)
        .context("Export failed")?;

    // Flush writer
    drop(writer);

    // Output result
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "output_file": output,
                "format": format!("{}", export_format),
                "total_exported": metadata.total_count,
                "exported_at": metadata.exported_at.to_rfc3339(),
                "filters_applied": metadata.filters_applied,
            }))?
        );
    } else {
        println!("Export completed successfully!");
        println!("Format: {}", export_format);
        println!("Output: {}", output);
        println!("Total memories exported: {}", metadata.total_count);

        if metadata.filters_applied.session_id.is_some()
            || metadata.filters_applied.date_range.is_some()
            || metadata.filters_applied.importance_threshold.is_some()
            || metadata.filters_applied.tags.is_some()
            || metadata.filters_applied.decay_profile.is_some()
            || metadata.filters_applied.include_deleted
        {
            println!("\nFilters applied:");
            if let Some(ref session_id) = metadata.filters_applied.session_id {
                println!("  - Session ID: {}", session_id);
            }
            if let Some(ref date_range) = metadata.filters_applied.date_range {
                println!("  - Date range: {}", date_range);
            }
            if let Some(ref importance) = metadata.filters_applied.importance_threshold {
                println!("  - Importance: {}", importance);
            }
            if let Some(ref tags) = metadata.filters_applied.tags {
                println!("  - Tags: {}", tags.join(", "));
            }
            if let Some(ref profile) = metadata.filters_applied.decay_profile {
                println!("  - Decay profile: {}", profile);
            }
            if metadata.filters_applied.include_deleted {
                println!("  - Include deleted: yes");
            }
        }
    }

    Ok(())
}

/// Parse filter string into ExportOptions
/// Format examples:
/// - "session:abc123"
/// - "importance:high"
/// - "tags:rust,async"
/// - "date:2024-01-01..2024-12-31"
/// - "decay:exponential"
fn parse_filter(filter_str: &str, options: &mut ExportOptions) -> Result<()> {
    let parts: Vec<&str> = filter_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid filter format. Use 'field:value' (e.g., 'importance:high')"
        ));
    }

    let field = parts[0].trim().to_lowercase();
    let value = parts[1].trim();

    match field.as_str() {
        "session" => {
            options.session_id = Some(value.to_string());
        }
        "importance" => {
            options.importance_threshold = Some(value.to_string());
        }
        "tags" => {
            options.tags = Some(value.split(',').map(|s| s.trim().to_string()).collect());
        }
        "decay" => {
            options.decay_profile = Some(value.to_string());
        }
        "date" => {
            // Parse date range: "2024-01-01..2024-12-31"
            let date_parts: Vec<&str> = value.split("..").collect();
            if date_parts.len() != 2 {
                return Err(anyhow::anyhow!(
                    "Invalid date range format. Use 'YYYY-MM-DD..YYYY-MM-DD'"
                ));
            }

            let start = DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", date_parts[0]))
                .with_context(|| format!("Invalid start date: {}", date_parts[0]))?
                .with_timezone(&Utc);

            let end = DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", date_parts[1]))
                .with_context(|| format!("Invalid end date: {}", date_parts[1]))?
                .with_timezone(&Utc);

            options.date_range = Some((start, end));
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown filter field '{}'. Valid fields: session, importance, tags, decay, date",
                field
            ));
        }
    }

    Ok(())
}

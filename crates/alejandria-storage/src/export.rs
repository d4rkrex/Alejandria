//! Export system for memory data portability.
//!
//! Provides functionality to export memories to multiple formats:
//! - JSON: Full fidelity export with all metadata
//! - CSV: Tabular format for spreadsheet analysis
//! - Markdown: Human-readable documentation format
//!
//! Supports streaming for large datasets and flexible filtering.

use alejandria_core::{
    error::{IcmError, IcmResult},
    memory::Memory,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// JSON format with full fidelity (all fields)
    Json,
    /// CSV format for tabular analysis (selected fields)
    Csv,
    /// Markdown format for human reading
    Markdown,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "json"),
            ExportFormat::Csv => write!(f, "csv"),
            ExportFormat::Markdown => write!(f, "markdown"),
        }
    }
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "csv" => Ok(ExportFormat::Csv),
            "markdown" | "md" => Ok(ExportFormat::Markdown),
            _ => Err(format!("Invalid export format: {}", s)),
        }
    }
}

/// Export options with filtering and configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportOptions {
    /// Filter by session ID
    pub session_id: Option<String>,

    /// Filter by date range (start, end)
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,

    /// Filter by minimum importance threshold
    pub importance_threshold: Option<String>,

    /// Filter by tags (keywords)
    pub tags: Option<Vec<String>>,

    /// Filter by decay profile
    pub decay_profile: Option<String>,

    /// Custom field selection (if empty, export all fields)
    pub selected_fields: Vec<String>,

    /// Include soft-deleted memories
    pub include_deleted: bool,
}

/// Metadata about the export operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Export format version
    pub version: String,

    /// Timestamp when export was created
    pub exported_at: DateTime<Utc>,

    /// Total number of memories exported
    pub total_count: usize,

    /// Filters applied during export
    pub filters_applied: ExportFiltersApplied,

    /// Format used for export
    pub format: ExportFormat,
}

/// Record of filters applied during export
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportFiltersApplied {
    pub session_id: Option<String>,
    pub date_range: Option<String>,
    pub importance_threshold: Option<String>,
    pub tags: Option<Vec<String>>,
    pub decay_profile: Option<String>,
    pub include_deleted: bool,
}

impl From<&ExportOptions> for ExportFiltersApplied {
    fn from(options: &ExportOptions) -> Self {
        Self {
            session_id: options.session_id.clone(),
            date_range: options.date_range.map(|(start, end)| {
                format!("{} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"))
            }),
            importance_threshold: options.importance_threshold.clone(),
            tags: options.tags.clone(),
            decay_profile: options.decay_profile.clone(),
            include_deleted: options.include_deleted,
        }
    }
}

/// Export memories to JSON format with full fidelity
pub fn export_json<W: Write>(
    memories: &[Memory],
    options: &ExportOptions,
    writer: &mut W,
    is_first_batch: bool,
    is_last_batch: bool,
) -> IcmResult<()> {
    if is_first_batch {
        // Don't write opening bracket here - it's handled by the caller with metadata
    }

    for (i, memory) in memories.iter().enumerate() {
        // Add comma before each item except the very first one in the export
        if !is_first_batch || i > 0 {
            write!(writer, ",\n")?;
        }

        let json = if options.selected_fields.is_empty() {
            // Export all fields
            serde_json::to_string_pretty(memory).map_err(|e| IcmError::Serialization(e))?
        } else {
            // Export selected fields only
            let mut map = serde_json::Map::new();
            let full = serde_json::to_value(memory).map_err(|e| IcmError::Serialization(e))?;

            if let serde_json::Value::Object(obj) = full {
                for field in &options.selected_fields {
                    if let Some(value) = obj.get(field) {
                        map.insert(field.clone(), value.clone());
                    }
                }
            }

            serde_json::to_string_pretty(&map).map_err(|e| IcmError::Serialization(e))?
        };

        write!(writer, "{}", json)?;
    }

    if is_last_batch {
        // Don't write closing bracket here - it's handled by the caller
    }

    Ok(())
}

/// Export memories to CSV format (selected fields for tabular analysis)
pub fn export_csv<W: Write>(
    memories: &[Memory],
    _options: &ExportOptions,
    writer: &mut W,
    is_first_batch: bool,
) -> IcmResult<()> {
    use std::io::BufWriter;

    // CSV writer needs BufWriter for performance
    let mut csv_writer = csv::Writer::from_writer(BufWriter::new(writer));

    // Write headers only on first batch
    if is_first_batch {
        csv_writer
            .write_record(&[
                "id",
                "topic",
                "summary",
                "importance",
                "source",
                "keywords",
                "created_at",
                "updated_at",
                "last_accessed",
                "access_count",
                "weight",
                "decay_profile",
                "topic_key",
                "revision_count",
            ])
            .map_err(|e| {
                IcmError::Serialization(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })?;
    }

    // Write memory records
    for memory in memories {
        csv_writer
            .write_record(&[
                &memory.id,
                &memory.topic,
                &memory.summary,
                &memory.importance.to_string(),
                &format!("{:?}", memory.source),
                &memory.keywords.join("; "),
                &memory.created_at.to_rfc3339(),
                &memory.updated_at.to_rfc3339(),
                &memory.last_accessed.to_rfc3339(),
                &memory.access_count.to_string(),
                &format!("{:.3}", memory.weight),
                &memory.decay_profile.as_ref().unwrap_or(&"none".to_string()),
                &memory.topic_key.as_ref().unwrap_or(&"".to_string()),
                &memory.revision_count.to_string(),
            ])
            .map_err(|e| {
                IcmError::Serialization(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })?;
    }

    csv_writer
        .flush()
        .map_err(|e| IcmError::Serialization(serde_json::Error::io(e)))?;
    Ok(())
}

/// Export memories to Markdown format (human-readable)
pub fn export_markdown<W: Write>(
    memories: &[Memory],
    _options: &ExportOptions,
    writer: &mut W,
) -> IcmResult<()> {
    for memory in memories {
        // Memory header
        writeln!(writer, "## {}", memory.topic)?;
        writeln!(writer)?;

        // Metadata table
        writeln!(writer, "| Field | Value |")?;
        writeln!(writer, "|-------|-------|")?;
        writeln!(writer, "| **ID** | {} |", memory.id)?;
        writeln!(writer, "| **Importance** | {} |", memory.importance)?;
        writeln!(writer, "| **Source** | {:?} |", memory.source)?;
        writeln!(writer, "| **Weight** | {:.3} |", memory.weight)?;

        if let Some(ref profile) = memory.decay_profile {
            writeln!(writer, "| **Decay Profile** | {} |", profile)?;
        }

        if let Some(ref topic_key) = memory.topic_key {
            writeln!(writer, "| **Topic Key** | {} |", topic_key)?;
        }

        writeln!(
            writer,
            "| **Created** | {} |",
            memory.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        writeln!(
            writer,
            "| **Last Accessed** | {} |",
            memory.last_accessed.format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        writeln!(writer, "| **Access Count** | {} |", memory.access_count)?;
        writeln!(writer)?;

        // Keywords
        if !memory.keywords.is_empty() {
            writeln!(writer, "**Keywords**: {}", memory.keywords.join(", "))?;
            writeln!(writer)?;
        }

        // Summary
        writeln!(writer, "### Summary")?;
        writeln!(writer)?;
        writeln!(writer, "{}", memory.summary)?;
        writeln!(writer)?;

        // Raw excerpt if present
        if let Some(ref excerpt) = memory.raw_excerpt {
            writeln!(writer, "### Raw Excerpt")?;
            writeln!(writer)?;
            writeln!(writer, "```")?;
            writeln!(writer, "{}", excerpt)?;
            writeln!(writer, "```")?;
            writeln!(writer)?;
        }

        // Related memories
        if !memory.related_ids.is_empty() {
            writeln!(writer, "**Related**: {}", memory.related_ids.join(", "))?;
            writeln!(writer)?;
        }

        // Separator between memories
        writeln!(writer, "---")?;
        writeln!(writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alejandria_core::memory::{Importance, MemorySource};
    use chrono::Utc;

    fn create_test_memory(id: &str, topic: &str) -> Memory {
        Memory {
            id: id.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 5,
            weight: 0.85,
            topic: topic.to_string(),
            summary: format!("Summary for {}", topic),
            raw_excerpt: Some("Raw content".to_string()),
            keywords: vec!["test".to_string(), "export".to_string()],
            embedding: None,
            importance: Importance::Medium,
            source: MemorySource::User,
            related_ids: vec![],
            topic_key: Some(format!("test/{}", topic)),
            revision_count: 1,
            duplicate_count: 0,
            last_seen_at: Utc::now(),
            deleted_at: None,
            decay_profile: Some("exponential".to_string()),
            decay_params: None,
        }
    }

    #[test]
    fn test_export_format_from_str() {
        assert_eq!("json".parse::<ExportFormat>().unwrap(), ExportFormat::Json);
        assert_eq!("csv".parse::<ExportFormat>().unwrap(), ExportFormat::Csv);
        assert_eq!(
            "markdown".parse::<ExportFormat>().unwrap(),
            ExportFormat::Markdown
        );
        assert_eq!(
            "md".parse::<ExportFormat>().unwrap(),
            ExportFormat::Markdown
        );
        assert!("invalid".parse::<ExportFormat>().is_err());
    }

    #[test]
    fn test_export_json_single_memory() {
        let memory = create_test_memory("test-1", "Test Memory");
        let options = ExportOptions::default();
        let mut buffer = Vec::new();

        export_json(&[memory], &options, &mut buffer, true, true).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("\"id\": \"test-1\""));
        assert!(output.contains("\"topic\": \"Test Memory\""));
    }

    #[test]
    fn test_export_json_multiple_memories() {
        let memories = vec![
            create_test_memory("test-1", "Memory 1"),
            create_test_memory("test-2", "Memory 2"),
            create_test_memory("test-3", "Memory 3"),
        ];
        let options = ExportOptions::default();
        let mut buffer = Vec::new();

        export_json(&memories, &options, &mut buffer, true, true).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("\"id\": \"test-1\""));
        assert!(output.contains("\"id\": \"test-2\""));
        assert!(output.contains("\"id\": \"test-3\""));
    }

    #[test]
    fn test_export_json_selected_fields() {
        let memory = create_test_memory("test-1", "Test Memory");
        let options = ExportOptions {
            selected_fields: vec!["id".to_string(), "topic".to_string(), "weight".to_string()],
            ..Default::default()
        };
        let mut buffer = Vec::new();

        export_json(&[memory], &options, &mut buffer, true, true).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("\"id\": \"test-1\""));
        assert!(output.contains("\"topic\": \"Test Memory\""));
        assert!(output.contains("\"weight\""));
        // Should not contain other fields
        assert!(!output.contains("\"summary\""));
        assert!(!output.contains("\"keywords\""));
    }

    #[test]
    fn test_export_csv_headers() {
        let memory = create_test_memory("test-1", "Test Memory");
        let options = ExportOptions::default();
        let mut buffer = Vec::new();

        export_csv(&[memory], &options, &mut buffer, true).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        // Check header
        assert!(lines[0].contains("id"));
        assert!(lines[0].contains("topic"));
        assert!(lines[0].contains("importance"));

        // Check data row
        assert!(lines[1].contains("test-1"));
        assert!(lines[1].contains("Test Memory"));
    }

    #[test]
    fn test_export_csv_multiple_memories() {
        let memories = vec![
            create_test_memory("test-1", "Memory 1"),
            create_test_memory("test-2", "Memory 2"),
        ];
        let options = ExportOptions::default();
        let mut buffer = Vec::new();

        export_csv(&memories, &options, &mut buffer, true).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 3); // Header + 2 data rows
        assert!(lines[1].contains("test-1"));
        assert!(lines[2].contains("test-2"));
    }

    #[test]
    fn test_export_markdown_format() {
        let memory = create_test_memory("test-1", "Test Memory");
        let options = ExportOptions::default();
        let mut buffer = Vec::new();

        export_markdown(&[memory], &options, &mut buffer).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("## Test Memory"));
        assert!(output.contains("| **ID** | test-1 |"));
        assert!(output.contains("### Summary"));
        assert!(output.contains("Summary for Test Memory"));
        assert!(output.contains("**Keywords**: test, export"));
    }

    #[test]
    fn test_export_metadata_from_options() {
        let options = ExportOptions {
            session_id: Some("session-123".to_string()),
            importance_threshold: Some("high".to_string()),
            tags: Some(vec!["rust".to_string(), "memory".to_string()]),
            decay_profile: Some("exponential".to_string()),
            include_deleted: false,
            ..Default::default()
        };

        let filters: ExportFiltersApplied = (&options).into();

        assert_eq!(filters.session_id, Some("session-123".to_string()));
        assert_eq!(filters.importance_threshold, Some("high".to_string()));
        assert_eq!(
            filters.tags,
            Some(vec!["rust".to_string(), "memory".to_string()])
        );
        assert_eq!(filters.decay_profile, Some("exponential".to_string()));
        assert!(!filters.include_deleted);
    }
}

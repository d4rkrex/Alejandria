//! Import system for memory data portability.
//!
//! Provides functionality to import memories from multiple formats:
//! - JSON: Full fidelity import with complete metadata
//! - CSV: Basic import from tabular data
//!
//! Supports conflict resolution strategies and validation.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod conflict;
pub mod validate;

use crate::error::IcmResult;

/// Import mode for handling conflicts with existing observations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// Skip observations that conflict with existing IDs/topic_keys
    Skip,
    /// Update existing observations with new values, preserve ID and timestamps
    Update,
    /// Replace existing observations completely (preserve ID only)
    Replace,
}

impl std::fmt::Display for ImportMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportMode::Skip => write!(f, "skip"),
            ImportMode::Update => write!(f, "update"),
            ImportMode::Replace => write!(f, "replace"),
        }
    }
}

impl std::str::FromStr for ImportMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "skip" => Ok(ImportMode::Skip),
            "update" => Ok(ImportMode::Update),
            "replace" => Ok(ImportMode::Replace),
            _ => Err(format!("Invalid import mode: {}", s)),
        }
    }
}

/// Result of an import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// Total number of observations processed
    pub total: usize,
    /// Number of new observations imported
    pub imported: usize,
    /// Number of observations skipped due to conflicts
    pub skipped: usize,
    /// Number of observations updated (merge/replace)
    pub updated: usize,
    /// Errors encountered during import
    pub errors: Vec<String>,
}

impl ImportResult {
    /// Create a new empty import result
    pub fn new() -> Self {
        Self {
            total: 0,
            imported: 0,
            skipped: 0,
            updated: 0,
            errors: Vec::new(),
        }
    }

    /// Add an imported observation
    pub fn add_imported(&mut self) {
        self.total += 1;
        self.imported += 1;
    }

    /// Add a skipped observation
    pub fn add_skipped(&mut self) {
        self.total += 1;
        self.skipped += 1;
    }

    /// Add an updated observation
    pub fn add_updated(&mut self) {
        self.total += 1;
        self.updated += 1;
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Import memories from a file path
///
/// This is a convenience function that detects the format from the file extension
/// and delegates to the appropriate import implementation.
pub fn import_memories(
    _input_path: &Path,
    _mode: ImportMode,
) -> IcmResult<ImportResult> {
    // This will be implemented by the storage layer
    // Core only defines the types and interfaces
    Ok(ImportResult::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_mode_display() {
        assert_eq!(ImportMode::Skip.to_string(), "skip");
        assert_eq!(ImportMode::Update.to_string(), "update");
        assert_eq!(ImportMode::Replace.to_string(), "replace");
    }

    #[test]
    fn test_import_mode_from_str() {
        assert_eq!("skip".parse::<ImportMode>().unwrap(), ImportMode::Skip);
        assert_eq!("update".parse::<ImportMode>().unwrap(), ImportMode::Update);
        assert_eq!("replace".parse::<ImportMode>().unwrap(), ImportMode::Replace);
        assert!("invalid".parse::<ImportMode>().is_err());
    }

    #[test]
    fn test_import_result_new() {
        let result = ImportResult::new();
        assert_eq!(result.total, 0);
        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_import_result_add_imported() {
        let mut result = ImportResult::new();
        result.add_imported();
        assert_eq!(result.total, 1);
        assert_eq!(result.imported, 1);
    }

    #[test]
    fn test_import_result_add_skipped() {
        let mut result = ImportResult::new();
        result.add_skipped();
        assert_eq!(result.total, 1);
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn test_import_result_add_updated() {
        let mut result = ImportResult::new();
        result.add_updated();
        assert_eq!(result.total, 1);
        assert_eq!(result.updated, 1);
    }

    #[test]
    fn test_import_result_add_error() {
        let mut result = ImportResult::new();
        result.add_error("test error".to_string());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "test error");
    }
}

//! Storage layer for Alejandria memory system.
//!
//! This crate provides SQLite-based storage implementation for both episodic memories
//! and semantic memoirs (knowledge graphs). It includes:
//!
//! - Full-text search (FTS5) for keyword-based retrieval
//! - Vector search support via sqlite-vec for embeddings
//! - Schema migration and validation
//! - Efficient indexing for temporal queries
//! - Multi-key API authentication with rotation support (P0-2)
//!
//! # Examples
//!
//! ```no_run
//! use alejandria_storage::SqliteStore;
//!
//! # fn main() -> alejandria_core::error::IcmResult<()> {
//! let store = SqliteStore::open("alejandria.db")?;
//! # Ok(())
//! # }
//! ```

pub mod api_keys;
pub mod export;
pub mod import;
pub mod memoir_store;
pub mod migrations;
pub mod schema;
pub mod search;
mod store;

#[cfg(feature = "encryption")]
pub mod encryption;

pub use store::SqliteStore;

// Re-export export types for convenience
pub use export::{
    export_csv, export_json, export_markdown, ExportFiltersApplied, ExportFormat, ExportMetadata,
    ExportOptions,
};

// Re-export import types for convenience
pub use import::ImportFormat;

// Re-export core types for convenience
pub use alejandria_core::{
    error::{IcmError, IcmResult},
    memoir::{Concept, ConceptLink, Memoir, RelationType},
    memoir_store::MemoirStore,
    memory::{Importance, Memory, MemorySource},
    store::MemoryStore,
};

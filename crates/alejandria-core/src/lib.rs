//! Alejandria Core - Foundational types and traits for the Alejandria memory system
//!
//! This crate provides:
//! - Core memory types (`Memory`, `Memoir`, `Concept`, `ConceptLink`)
//! - Trait definitions for storage (`MemoryStore`, `MemoirStore`)
//! - Error types (`IcmError`)
//! - Embedder abstractions
//! - Decay strategies for temporal relevance

pub mod decay;
pub mod embedder;
pub mod error;
pub mod import;
pub mod memoir;
pub mod memoir_store;
pub mod memory;
pub mod store;

#[cfg(feature = "embeddings")]
pub mod fastembed_embedder;

// Re-export core types
pub use decay::{config::DecayProfileConfig, strategy::DecayStrategy};
// Decay strategy implementations will be added in Phase 2:
// pub use decay::{ContextSensitiveDecay, ExponentialDecay, ImportanceWeightedDecay, SpacedRepetitionDecay};
pub use embedder::Embedder;
pub use error::{IcmError, IcmResult};
pub use import::{ImportMode, ImportResult};
pub use memoir::{Concept, ConceptLink, Memoir, RelationType};
pub use memoir_store::{
    ConceptMatch, ConceptNeighborhood, ConceptUpdate, LinkDirection, LinkInfo, MemoirDetail,
    MemoirStore, MemoirSummary, NeighborInfo, NewConcept, NewConceptLink, NewMemoir,
};
pub use memory::{Importance, Memory, MemorySource};
pub use store::{DecayStats, MemoryStore};

#[cfg(feature = "embeddings")]
pub use fastembed_embedder::FastembedEmbedder;

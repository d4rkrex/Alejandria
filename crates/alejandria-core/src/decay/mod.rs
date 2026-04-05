//! Decay strategies for temporal relevance scoring.
//!
//! This module provides pluggable decay algorithms that determine how memory weights
//! decay over time based on access patterns and content importance.

pub mod config;
pub mod strategy;

// Decay strategy implementations
pub mod exponential;
pub mod spaced_repetition;
pub mod importance_weighted;
pub mod context_sensitive;

// Re-export public types
pub use config::{load_decay_profiles, DecayProfileConfig, ProfileSettings};
pub use strategy::DecayStrategy;

// Re-export strategy implementations
pub use exponential::ExponentialDecay;
pub use spaced_repetition::SpacedRepetitionDecay;
pub use importance_weighted::ImportanceWeightedDecay;
pub use context_sensitive::ContextSensitiveDecay;

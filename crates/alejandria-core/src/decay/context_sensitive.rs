//! Context-sensitive decay strategy.
//!
//! This strategy adjusts decay rates based on the memory's topic/type.
//! Different types of memories have different useful lifespans - architectural
//! decisions remain relevant longer than temporary bug fixes.
//!
//! ## Algorithm
//!
//! Uses exponential decay with topic-specific half-life multipliers:
//!
//! ```text
//! half_life = base_half_life * topic_multiplier
//! weight_new = weight_old * e^(-λ * days_since_access)
//! λ = ln(2) / half_life
//! ```
//!
//! ## Topic Categories (using Memory.topic field)
//!
//! Default multipliers (configurable via parameters):
//! - **architecture**: 2.0x slower decay (longer-lived decisions)
//! - **decision**: 1.5x slower decay (important choices)
//! - **bugfix**: 1.0x base decay (medium-term relevance)
//! - **discovery**: 1.5x slower decay (valuable insights)
//! - **experiment**: 0.5x faster decay (temporary notes)
//! - **manual**: 1.0x base decay (user-created memories)
//! - **default**: 1.0x (fallback for unknown topics)
//!
//! ## Use Cases
//!
//! - Mixed knowledge bases with different memory types
//! - Codebases with architecture + bug tracking + experiments
//! - Automatic decay tuning based on content classification
//! - Topic-aware memory management
//!
//! ## Parameters
//!
//! - `base_half_life_days`: Base half-life for default topics (default: 60 days)
//! - `topic_multipliers`: JSON map of topic → multiplier (optional overrides)

use super::strategy::DecayStrategy;
use crate::{IcmError, IcmResult, Memory};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Context-sensitive decay strategy.
///
/// This strategy parses the memory's topic field and applies topic-specific
/// decay rates. Topics related to architecture and decisions decay slower,
/// while experiments and temporary notes decay faster.
///
/// ## Example
///
/// ```ignore
/// use alejandria_core::decay::ContextSensitiveDecay;
///
/// let strategy = ContextSensitiveDecay;
/// let params = json!({
///     "base_half_life_days": 60.0,
///     "topic_multipliers": {
///         "architecture": 2.0,
///         "experiment": 0.5
///     }
/// });
///
/// // Memory with topic "architecture": half-life = 60 * 2.0 = 120 days
/// // Memory with topic "experiment": half-life = 60 * 0.5 = 30 days
/// ```
#[derive(Debug, Clone)]
pub struct ContextSensitiveDecay;

impl ContextSensitiveDecay {
    /// Default base half-life (2 months)
    pub const DEFAULT_BASE_HALF_LIFE_DAYS: f64 = 60.0;

    /// Minimum half-life (1 day)
    const MIN_HALF_LIFE_DAYS: f64 = 1.0;

    /// Maximum half-life (10 years)
    const MAX_HALF_LIFE_DAYS: f64 = 3650.0;

    /// Get default topic multipliers.
    ///
    /// These can be overridden via parameters.
    fn default_topic_multipliers() -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert("architecture".to_string(), 2.0);
        map.insert("decision".to_string(), 1.5);
        map.insert("bugfix".to_string(), 1.0);
        map.insert("discovery".to_string(), 1.5);
        map.insert("experiment".to_string(), 0.5);
        map.insert("manual".to_string(), 1.0);
        map.insert("learning".to_string(), 1.5);
        map.insert("reference".to_string(), 1.8);
        map.insert("temporary".to_string(), 0.3);
        map
    }

    /// Extract base_half_life_days from parameters or use default.
    fn get_base_half_life(params: &Value) -> f64 {
        params
            .get("base_half_life_days")
            .and_then(|v| v.as_f64())
            .unwrap_or(Self::DEFAULT_BASE_HALF_LIFE_DAYS)
            .clamp(Self::MIN_HALF_LIFE_DAYS, Self::MAX_HALF_LIFE_DAYS)
    }

    /// Extract topic_multipliers from parameters or use defaults.
    fn get_topic_multipliers(params: &Value) -> HashMap<String, f64> {
        let defaults = Self::default_topic_multipliers();

        if let Some(multipliers) = params.get("topic_multipliers").and_then(|v| v.as_object()) {
            let mut map = defaults;
            for (topic, value) in multipliers {
                if let Some(multiplier) = value.as_f64() {
                    if multiplier > 0.0 && multiplier <= 10.0 {
                        map.insert(topic.clone(), multiplier);
                    }
                }
            }
            map
        } else {
            defaults
        }
    }

    /// Determine multiplier for a memory based on its topic.
    ///
    /// Checks topic field against known patterns:
    /// 1. Exact match in multipliers map
    /// 2. Partial match (e.g., "bugfix" in "bugfix/auth") - prefers longest match
    /// 3. Default multiplier (1.0)
    fn get_multiplier_for_memory(memory: &Memory, multipliers: &HashMap<String, f64>) -> f64 {
        let topic_lower = memory.topic.to_lowercase();

        // Exact match
        if let Some(&multiplier) = multipliers.get(&topic_lower) {
            return multiplier;
        }

        // Partial match - find the longest matching known topic
        let mut best_match: Option<(usize, f64)> = None;
        for (known_topic, &multiplier) in multipliers {
            if topic_lower.contains(known_topic) {
                let length = known_topic.len();
                if best_match.is_none() || length > best_match.unwrap().0 {
                    best_match = Some((length, multiplier));
                }
            }
        }

        if let Some((_, multiplier)) = best_match {
            return multiplier;
        }

        // Default multiplier
        1.0
    }

    /// Calculate effective half-life based on topic.
    fn effective_half_life(base_half_life: f64, multiplier: f64) -> f64 {
        (base_half_life * multiplier).clamp(Self::MIN_HALF_LIFE_DAYS, Self::MAX_HALF_LIFE_DAYS)
    }

    /// Calculate decay lambda from half-life.
    fn calculate_lambda(half_life: f64) -> f64 {
        std::f64::consts::LN_2 / half_life
    }

    /// Calculate days elapsed since last access.
    fn days_since_access(memory: &Memory) -> f64 {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(memory.last_accessed);
        duration.num_days() as f64 + (duration.num_seconds() % 86400) as f64 / 86400.0
    }
}

impl DecayStrategy for ContextSensitiveDecay {
    fn name(&self) -> &'static str {
        "context-sensitive"
    }

    fn calculate_decay(
        &self,
        memory: &Memory,
        params: &Value,
        _base_rate: f32,
    ) -> IcmResult<(f32, Value)> {
        let base_half_life = Self::get_base_half_life(params);
        let topic_multipliers = Self::get_topic_multipliers(params);

        let multiplier = Self::get_multiplier_for_memory(memory, &topic_multipliers);
        let effective_half_life = Self::effective_half_life(base_half_life, multiplier);
        let lambda = Self::calculate_lambda(effective_half_life);
        let days = Self::days_since_access(memory);

        // Exponential decay formula: weight_new = weight_old * e^(-λ * days)
        let decay_factor = (-lambda * days).exp();
        let new_weight = (memory.weight * decay_factor as f32).max(0.0);

        // Store parameters for next decay cycle
        let updated_params = json!({
            "base_half_life_days": base_half_life,
            "effective_half_life_days": effective_half_life,
            "topic_multiplier": multiplier,
            "matched_topic": memory.topic.clone(),
            "topic_multipliers": topic_multipliers,
        });

        Ok((new_weight, updated_params))
    }

    fn calculate_temporal_score(&self, memory: &Memory, params: &Value) -> IcmResult<f32> {
        let base_half_life = Self::get_base_half_life(params);
        let topic_multipliers = Self::get_topic_multipliers(params);

        let multiplier = Self::get_multiplier_for_memory(memory, &topic_multipliers);
        let effective_half_life = Self::effective_half_life(base_half_life, multiplier);
        let lambda = Self::calculate_lambda(effective_half_life);
        let days = Self::days_since_access(memory);

        // Temporal score is the decay factor (0.0 to 1.0)
        let score = (-lambda * days).exp() as f32;
        Ok(score.clamp(0.0, 1.0))
    }

    fn default_params(&self) -> Value {
        json!({
            "base_half_life_days": Self::DEFAULT_BASE_HALF_LIFE_DAYS,
            "topic_multipliers": Self::default_topic_multipliers(),
        })
    }

    fn validate_params(&self, params: &Value) -> IcmResult<()> {
        if !params.is_object() {
            return Err(IcmError::InvalidInput(
                "Context-sensitive decay parameters must be a JSON object".to_string(),
            ));
        }

        // Validate base_half_life_days
        if let Some(half_life) = params.get("base_half_life_days") {
            match half_life.as_f64() {
                Some(value)
                    if (Self::MIN_HALF_LIFE_DAYS..=Self::MAX_HALF_LIFE_DAYS).contains(&value) => {}
                Some(value) => {
                    return Err(IcmError::InvalidInput(format!(
                        "base_half_life_days must be between {} and {}, got {}",
                        Self::MIN_HALF_LIFE_DAYS,
                        Self::MAX_HALF_LIFE_DAYS,
                        value
                    )))
                }
                None => {
                    return Err(IcmError::InvalidInput(
                        "base_half_life_days must be a number".to_string(),
                    ))
                }
            }
        }

        // Validate topic_multipliers (if present)
        if let Some(multipliers) = params.get("topic_multipliers") {
            if let Some(obj) = multipliers.as_object() {
                for (topic, value) in obj {
                    match value.as_f64() {
                        Some(v) if v > 0.0 && v <= 10.0 => {}
                        Some(v) => {
                            return Err(IcmError::InvalidInput(format!(
                                "Multiplier for topic '{}' must be between 0.0 and 10.0, got {}",
                                topic, v
                            )))
                        }
                        None => {
                            return Err(IcmError::InvalidInput(format!(
                                "Multiplier for topic '{}' must be a number",
                                topic
                            )))
                        }
                    }
                }
            } else {
                return Err(IcmError::InvalidInput(
                    "topic_multipliers must be a JSON object".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Importance, MemorySource};

    fn create_test_memory(topic: &str, weight: f32, days_ago: i64) -> Memory {
        let now = chrono::Utc::now();
        let last_accessed = now - chrono::Duration::days(days_ago);

        Memory {
            id: "test-context".to_string(),
            created_at: now,
            updated_at: now,
            last_accessed,
            access_count: 0,
            weight,
            topic: topic.to_string(),
            summary: format!("Test memory for topic: {}", topic),
            raw_excerpt: None,
            keywords: vec![],
            embedding: None,
            importance: Importance::Medium,
            source: MemorySource::User,
            related_ids: vec![],
            topic_key: None,
            revision_count: 1,
            duplicate_count: 0,
            last_seen_at: now,
            deleted_at: None,
            decay_profile: Some("context-sensitive".to_string()),
            decay_params: None,
        }
    }

    #[test]
    fn test_strategy_name() {
        let strategy = ContextSensitiveDecay;
        assert_eq!(strategy.name(), "context-sensitive");
    }

    #[test]
    fn test_default_params() {
        let strategy = ContextSensitiveDecay;
        let params = strategy.default_params();

        assert_eq!(params["base_half_life_days"].as_f64().unwrap(), 60.0);
        assert!(params["topic_multipliers"].is_object());

        let multipliers = params["topic_multipliers"].as_object().unwrap();
        assert_eq!(multipliers["architecture"].as_f64().unwrap(), 2.0);
        assert_eq!(multipliers["experiment"].as_f64().unwrap(), 0.5);
    }

    #[test]
    fn test_validate_params_valid() {
        let strategy = ContextSensitiveDecay;
        let params = json!({
            "base_half_life_days": 60.0,
            "topic_multipliers": {
                "custom": 1.5
            }
        });

        assert!(strategy.validate_params(&params).is_ok());
    }

    #[test]
    fn test_validate_params_invalid_half_life() {
        let strategy = ContextSensitiveDecay;
        let params = json!({"base_half_life_days": -10.0});

        assert!(strategy.validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_invalid_multiplier() {
        let strategy = ContextSensitiveDecay;
        let params = json!({
            "topic_multipliers": {
                "invalid": -1.0
            }
        });

        assert!(strategy.validate_params(&params).is_err());
    }

    #[test]
    fn test_architecture_decays_slower() {
        let strategy = ContextSensitiveDecay;
        let params = strategy.default_params();

        let architecture = create_test_memory("architecture", 1.0, 60);
        let bugfix = create_test_memory("bugfix", 1.0, 60);

        let (weight_arch, _) = strategy
            .calculate_decay(&architecture, &params, 0.01)
            .unwrap();
        let (weight_bug, _) = strategy.calculate_decay(&bugfix, &params, 0.01).unwrap();

        // Architecture (2.0x) should decay slower than bugfix (1.0x)
        assert!(weight_arch > weight_bug);
    }

    #[test]
    fn test_experiment_decays_faster() {
        let strategy = ContextSensitiveDecay;
        let params = strategy.default_params();

        let experiment = create_test_memory("experiment", 1.0, 30);
        let bugfix = create_test_memory("bugfix", 1.0, 30);

        let (weight_exp, _) = strategy
            .calculate_decay(&experiment, &params, 0.01)
            .unwrap();
        let (weight_bug, _) = strategy.calculate_decay(&bugfix, &params, 0.01).unwrap();

        // Experiment (0.5x) should decay faster than bugfix (1.0x)
        assert!(weight_exp < weight_bug);
    }

    #[test]
    fn test_partial_topic_matching() {
        let strategy = ContextSensitiveDecay;
        let multipliers = ContextSensitiveDecay::default_topic_multipliers();

        // Should match "bugfix" in "bugfix/authentication"
        let memory = create_test_memory("bugfix/authentication", 1.0, 0);
        let multiplier = ContextSensitiveDecay::get_multiplier_for_memory(&memory, &multipliers);
        assert_eq!(multiplier, 1.0);

        // Should match "architecture" in "architecture-decisions"
        let memory = create_test_memory("architecture-decisions", 1.0, 0);
        let multiplier = ContextSensitiveDecay::get_multiplier_for_memory(&memory, &multipliers);
        assert_eq!(multiplier, 2.0);
    }

    #[test]
    fn test_unknown_topic_uses_default() {
        let strategy = ContextSensitiveDecay;
        let multipliers = ContextSensitiveDecay::default_topic_multipliers();

        let memory = create_test_memory("unknown-topic", 1.0, 0);
        let multiplier = ContextSensitiveDecay::get_multiplier_for_memory(&memory, &multipliers);

        // Should use default multiplier (1.0)
        assert_eq!(multiplier, 1.0);
    }

    #[test]
    fn test_custom_topic_multipliers() {
        let strategy = ContextSensitiveDecay;
        let params = json!({
            "base_half_life_days": 60.0,
            "topic_multipliers": {
                "custom-topic": 3.0
            }
        });

        let memory = create_test_memory("custom-topic", 1.0, 60);
        let (_, updated_params) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Should use custom multiplier
        assert_eq!(updated_params["topic_multiplier"].as_f64().unwrap(), 3.0);

        // Effective half-life should be 60 * 3.0 = 180 days
        assert_eq!(
            updated_params["effective_half_life_days"].as_f64().unwrap(),
            180.0
        );
    }

    #[test]
    fn test_temporal_score_reflects_topic() {
        let strategy = ContextSensitiveDecay;
        let params = strategy.default_params();

        let architecture = create_test_memory("architecture", 1.0, 60);
        let experiment = create_test_memory("experiment", 1.0, 60);

        let score_arch = strategy
            .calculate_temporal_score(&architecture, &params)
            .unwrap();
        let score_exp = strategy
            .calculate_temporal_score(&experiment, &params)
            .unwrap();

        // Architecture should have higher temporal score
        assert!(score_arch > score_exp);
    }

    #[test]
    fn test_effective_half_life_clamping() {
        // Test minimum clamping
        let effective = ContextSensitiveDecay::effective_half_life(0.5, 1.0);
        assert_eq!(effective, ContextSensitiveDecay::MIN_HALF_LIFE_DAYS);

        // Test maximum clamping
        let effective = ContextSensitiveDecay::effective_half_life(5000.0, 1.0);
        assert_eq!(effective, ContextSensitiveDecay::MAX_HALF_LIFE_DAYS);
    }

    #[test]
    fn test_stores_matched_topic_in_params() {
        let strategy = ContextSensitiveDecay;
        let params = strategy.default_params();

        let memory = create_test_memory("architecture", 1.0, 30);
        let (_, updated_params) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Should store matched topic for debugging
        assert_eq!(
            updated_params["matched_topic"].as_str().unwrap(),
            "architecture"
        );
        assert_eq!(updated_params["topic_multiplier"].as_f64().unwrap(), 2.0);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let strategy = ContextSensitiveDecay;
        let multipliers = ContextSensitiveDecay::default_topic_multipliers();

        // Should match regardless of case
        let memory = create_test_memory("ARCHITECTURE", 1.0, 0);
        let multiplier = ContextSensitiveDecay::get_multiplier_for_memory(&memory, &multipliers);
        assert_eq!(multiplier, 2.0);

        let memory = create_test_memory("BugFix", 1.0, 0);
        let multiplier = ContextSensitiveDecay::get_multiplier_for_memory(&memory, &multipliers);
        assert_eq!(multiplier, 1.0);
    }

    #[test]
    fn test_zero_days_no_decay() {
        let strategy = ContextSensitiveDecay;
        let memory = create_test_memory("architecture", 1.0, 0);
        let params = strategy.default_params();

        let (new_weight, _) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // No time passed, weight should be unchanged
        assert!((new_weight - 1.0).abs() < 0.01);
    }
}

//! Exponential decay strategy implementation.
//!
//! This is the default decay algorithm using exponential decay with a configurable half-life.
//! Weight decreases exponentially over time based on days since last access.
//!
//! ## Formula
//!
//! ```text
//! weight_new = weight_old * e^(-λ * days_since_access)
//! λ = ln(2) / half_life_days
//! ```
//!
//! ## Parameters
//!
//! - `half_life_days`: Number of days for weight to decay to 50% (default: 90 days)

use super::strategy::DecayStrategy;
use crate::{IcmError, IcmResult, Memory};
use serde_json::{json, Value};

/// Exponential decay strategy with configurable half-life.
///
/// This strategy applies simple exponential decay where the weight decreases
/// continuously over time. The half-life parameter controls how quickly memories
/// decay - shorter half-life means faster decay.
///
/// ## Use Cases
///
/// - General-purpose temporal decay for most memories
/// - Backward-compatible with existing Alejandria decay behavior
/// - Good default when no specific decay pattern is needed
///
/// ## Example
///
/// ```ignore
/// use alejandria_core::decay::ExponentialDecay;
///
/// let strategy = ExponentialDecay;
/// let params = json!({"half_life_days": 90.0});
///
/// // After 90 days, weight will be ~50% of original
/// // After 180 days, weight will be ~25% of original
/// ```
#[derive(Debug, Clone)]
pub struct ExponentialDecay;

impl ExponentialDecay {
    /// Default half-life in days (3 months).
    pub const DEFAULT_HALF_LIFE_DAYS: f64 = 90.0;

    /// Calculate decay lambda from half-life.
    ///
    /// λ = ln(2) / half_life_days
    fn calculate_lambda(half_life_days: f64) -> f64 {
        std::f64::consts::LN_2 / half_life_days
    }

    /// Extract half_life_days from parameters or use default.
    fn get_half_life(params: &Value) -> f64 {
        params
            .get("half_life_days")
            .and_then(|v| v.as_f64())
            .unwrap_or(Self::DEFAULT_HALF_LIFE_DAYS)
    }

    /// Calculate days elapsed since last access.
    fn days_since_access(memory: &Memory) -> f64 {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(memory.last_accessed);
        duration.num_days() as f64 + (duration.num_seconds() % 86400) as f64 / 86400.0
    }
}

impl DecayStrategy for ExponentialDecay {
    fn name(&self) -> &'static str {
        "exponential"
    }

    fn calculate_decay(
        &self,
        memory: &Memory,
        params: &Value,
        _base_rate: f32,
    ) -> IcmResult<(f32, Value)> {
        let half_life = Self::get_half_life(params);
        let lambda = Self::calculate_lambda(half_life);
        let days = Self::days_since_access(memory);

        // Exponential decay formula: weight_new = weight_old * e^(-λ * days)
        let decay_factor = (-lambda * days).exp();
        let new_weight = (memory.weight * decay_factor as f32).max(0.0);

        // Parameters don't change for exponential decay
        let updated_params = json!({
            "half_life_days": half_life,
        });

        Ok((new_weight, updated_params))
    }

    fn calculate_temporal_score(&self, memory: &Memory, params: &Value) -> IcmResult<f32> {
        let half_life = Self::get_half_life(params);
        let lambda = Self::calculate_lambda(half_life);
        let days = Self::days_since_access(memory);

        // Temporal score is the decay factor (0.0 to 1.0)
        let score = (-lambda * days).exp() as f32;
        Ok(score.clamp(0.0, 1.0))
    }

    fn default_params(&self) -> Value {
        json!({
            "half_life_days": Self::DEFAULT_HALF_LIFE_DAYS,
        })
    }

    fn validate_params(&self, params: &Value) -> IcmResult<()> {
        if !params.is_object() {
            return Err(IcmError::InvalidInput(
                "Exponential decay parameters must be a JSON object".to_string(),
            ));
        }

        if let Some(half_life) = params.get("half_life_days") {
            match half_life.as_f64() {
                Some(value) if value > 0.0 => Ok(()),
                Some(_) => Err(IcmError::InvalidInput(
                    "half_life_days must be positive".to_string(),
                )),
                None => Err(IcmError::InvalidInput(
                    "half_life_days must be a number".to_string(),
                )),
            }
        } else {
            // half_life_days is optional, will use default
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Importance, MemorySource};

    fn create_test_memory(weight: f32, days_ago: i64) -> Memory {
        let now = chrono::Utc::now();
        let last_accessed = now - chrono::Duration::days(days_ago);

        Memory {
            id: "test-id".to_string(),
            created_at: now,
            updated_at: now,
            last_accessed,
            access_count: 1,
            weight,
            topic: "test-topic".to_string(),
            summary: "test summary".to_string(),
            raw_excerpt: None,
            keywords: vec!["test".to_string()],
            embedding: None,
            importance: Importance::Medium,
            source: MemorySource::User,
            related_ids: vec![],
            topic_key: None,
            revision_count: 1,
            duplicate_count: 0,
            last_seen_at: now,
            deleted_at: None,
            decay_profile: None,
            decay_params: None,
        }
    }

    #[test]
    fn test_name() {
        let strategy = ExponentialDecay;
        assert_eq!(strategy.name(), "exponential");
    }

    #[test]
    fn test_default_params() {
        let strategy = ExponentialDecay;
        let params = strategy.default_params();

        assert_eq!(
            params.get("half_life_days").and_then(|v| v.as_f64()),
            Some(90.0)
        );
    }

    #[test]
    fn test_validate_params_valid() {
        let strategy = ExponentialDecay;

        // Valid with explicit half_life
        let params = json!({"half_life_days": 30.0});
        assert!(strategy.validate_params(&params).is_ok());

        // Valid with empty object (uses default)
        let params = json!({});
        assert!(strategy.validate_params(&params).is_ok());
    }

    #[test]
    fn test_validate_params_invalid() {
        let strategy = ExponentialDecay;

        // Not an object
        let params = json!("invalid");
        assert!(strategy.validate_params(&params).is_err());

        // Negative half_life
        let params = json!({"half_life_days": -10.0});
        assert!(strategy.validate_params(&params).is_err());

        // Zero half_life
        let params = json!({"half_life_days": 0.0});
        assert!(strategy.validate_params(&params).is_err());

        // Non-numeric half_life
        let params = json!({"half_life_days": "thirty"});
        assert!(strategy.validate_params(&params).is_err());
    }

    #[test]
    fn test_calculate_decay_recent_access() {
        let strategy = ExponentialDecay;
        let memory = create_test_memory(1.0, 0); // Just accessed
        let params = json!({"half_life_days": 90.0});

        let (new_weight, _) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Weight should be close to 1.0 for recent access
        assert!(new_weight > 0.99);
    }

    #[test]
    fn test_calculate_decay_half_life() {
        let strategy = ExponentialDecay;
        let memory = create_test_memory(1.0, 90); // 90 days ago
        let params = json!({"half_life_days": 90.0});

        let (new_weight, _) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // After one half-life, weight should be ~0.5
        assert!((new_weight - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_decay_two_half_lives() {
        let strategy = ExponentialDecay;
        let memory = create_test_memory(1.0, 180); // 180 days ago
        let params = json!({"half_life_days": 90.0});

        let (new_weight, _) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // After two half-lives, weight should be ~0.25
        assert!((new_weight - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_calculate_decay_never_negative() {
        let strategy = ExponentialDecay;
        let memory = create_test_memory(1.0, 1000); // Very old
        let params = json!({"half_life_days": 30.0});

        let (new_weight, _) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Weight should never be negative
        assert!(new_weight >= 0.0);
    }

    #[test]
    fn test_calculate_decay_monotonic() {
        let strategy = ExponentialDecay;
        let params = json!({"half_life_days": 90.0});

        let mem_1day = create_test_memory(1.0, 1);
        let mem_30days = create_test_memory(1.0, 30);
        let mem_90days = create_test_memory(1.0, 90);

        let (weight_1, _) = strategy.calculate_decay(&mem_1day, &params, 0.01).unwrap();
        let (weight_30, _) = strategy
            .calculate_decay(&mem_30days, &params, 0.01)
            .unwrap();
        let (weight_90, _) = strategy
            .calculate_decay(&mem_90days, &params, 0.01)
            .unwrap();

        // Weight should decrease monotonically
        assert!(weight_1 > weight_30);
        assert!(weight_30 > weight_90);
    }

    #[test]
    fn test_calculate_temporal_score_range() {
        let strategy = ExponentialDecay;
        let params = json!({"half_life_days": 90.0});

        // Test various time points
        for days in [0, 1, 7, 30, 90, 180, 365] {
            let memory = create_test_memory(1.0, days);
            let score = strategy.calculate_temporal_score(&memory, &params).unwrap();

            // Score must be in [0.0, 1.0] range
            assert!(score >= 0.0 && score <= 1.0);
        }
    }

    #[test]
    fn test_calculate_temporal_score_recent_is_high() {
        let strategy = ExponentialDecay;
        let memory = create_test_memory(1.0, 0); // Just accessed
        let params = json!({"half_life_days": 90.0});

        let score = strategy.calculate_temporal_score(&memory, &params).unwrap();

        // Recent access should have high temporal score
        assert!(score > 0.99);
    }

    #[test]
    fn test_different_half_lives() {
        let strategy = ExponentialDecay;
        let memory = create_test_memory(1.0, 30); // 30 days ago

        // Short half-life (fast decay)
        let params_short = json!({"half_life_days": 15.0});
        let (weight_short, _) = strategy
            .calculate_decay(&memory, &params_short, 0.01)
            .unwrap();

        // Long half-life (slow decay)
        let params_long = json!({"half_life_days": 180.0});
        let (weight_long, _) = strategy
            .calculate_decay(&memory, &params_long, 0.01)
            .unwrap();

        // Shorter half-life should result in more decay
        assert!(weight_short < weight_long);
    }
}

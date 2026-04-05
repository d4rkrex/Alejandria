//! Spaced repetition decay strategy using SM-2 algorithm.
//!
//! This strategy implements the SuperMemo-2 (SM-2) spaced repetition algorithm,
//! commonly used in flashcard apps and learning systems. Memories are reinforced
//! through access patterns, with intervals between reviews increasing based on
//! successful recalls.
//!
//! ## Algorithm
//!
//! The SM-2 algorithm maintains three parameters:
//! - **Easiness Factor (EF)**: Determines how quickly intervals grow (range: 1.3+, default: 2.5)
//! - **Interval**: Days until next review (starts at 1 day)
//! - **Repetitions**: Count of consecutive successful reviews
//!
//! Each access (review) updates these parameters:
//! 1. If accessed recently (within interval): increase interval, increment repetitions
//! 2. If accessed late: reset to shorter interval, reset repetitions
//! 3. EF adjusts based on access patterns (simplified: stays constant in MVP)
//!
//! ## Formula
//!
//! ```text
//! interval_new = interval_old * easiness_factor
//! weight = 1.0 if within interval, otherwise exponential decay
//! ```
//!
//! ## Use Cases
//!
//! - Learning-oriented observations (documentation, tutorials)
//! - Memories that benefit from periodic reinforcement
//! - Active recall workflows (agent regularly queries topics)
//!
//! ## Parameters
//!
//! - `easiness_factor`: Growth rate of intervals (default: 2.5, range: 1.3-3.0)
//! - `interval_days`: Current interval between reviews (default: 1.0)
//! - `repetitions`: Count of successful reviews (default: 0)
//! - `last_review`: Timestamp of last review (Unix millis)

use super::strategy::DecayStrategy;
use crate::{IcmError, IcmResult, Memory};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};

/// Spaced repetition decay strategy using SM-2 algorithm.
///
/// This strategy treats memory access as "review events" and adjusts intervals
/// accordingly. Memories accessed within their interval maintain full weight;
/// memories accessed late suffer exponential decay.
///
/// ## Example
///
/// ```ignore
/// use alejandria_core::decay::SpacedRepetitionDecay;
///
/// let strategy = SpacedRepetitionDecay;
/// let params = json!({
///     "easiness_factor": 2.5,
///     "interval_days": 1.0,
///     "repetitions": 0
/// });
///
/// // First review: interval = 1 day
/// // Second review (after 1 day): interval = 2.5 days
/// // Third review (after 2.5 days): interval = 6.25 days
/// ```
#[derive(Debug, Clone)]
pub struct SpacedRepetitionDecay;

impl SpacedRepetitionDecay {
    /// Default easiness factor (2.5 is standard SM-2 default)
    pub const DEFAULT_EASINESS_FACTOR: f64 = 2.5;

    /// Minimum easiness factor (SM-2 constraint)
    pub const MIN_EASINESS_FACTOR: f64 = 1.3;

    /// Maximum easiness factor (reasonable upper bound)
    pub const MAX_EASINESS_FACTOR: f64 = 3.0;

    /// Default initial interval in days
    pub const DEFAULT_INTERVAL_DAYS: f64 = 1.0;

    /// Default repetition count
    pub const DEFAULT_REPETITIONS: u32 = 0;

    /// Grace period multiplier (allow 20% lateness before penalizing)
    const GRACE_PERIOD_MULTIPLIER: f64 = 1.2;

    /// Extract easiness_factor from parameters or use default.
    fn get_easiness_factor(params: &Value) -> f64 {
        params
            .get("easiness_factor")
            .and_then(|v| v.as_f64())
            .unwrap_or(Self::DEFAULT_EASINESS_FACTOR)
            .clamp(Self::MIN_EASINESS_FACTOR, Self::MAX_EASINESS_FACTOR)
    }

    /// Extract interval_days from parameters or use default.
    fn get_interval_days(params: &Value) -> f64 {
        params
            .get("interval_days")
            .and_then(|v| v.as_f64())
            .unwrap_or(Self::DEFAULT_INTERVAL_DAYS)
            .max(0.1) // Minimum 0.1 days
    }

    /// Extract repetitions from parameters or use default.
    fn get_repetitions(params: &Value) -> u32 {
        params
            .get("repetitions")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(Self::DEFAULT_REPETITIONS)
    }

    /// Extract last_review timestamp from parameters.
    fn get_last_review(params: &Value, memory: &Memory) -> DateTime<Utc> {
        params
            .get("last_review")
            .and_then(|v| v.as_i64())
            .and_then(DateTime::from_timestamp_millis)
            .unwrap_or(memory.last_accessed)
    }

    /// Calculate days elapsed since last review.
    fn days_since_review(last_review: DateTime<Utc>) -> f64 {
        let now = Utc::now();
        let duration = now.signed_duration_since(last_review);
        duration.num_days() as f64 + (duration.num_seconds() % 86400) as f64 / 86400.0
    }

    /// Update SM-2 parameters based on review timing.
    ///
    /// Returns (new_interval, new_repetitions, new_easiness_factor)
    fn update_sm2_params(
        interval: f64,
        repetitions: u32,
        easiness_factor: f64,
        days_since_review: f64,
    ) -> (f64, u32, f64) {
        let grace_interval = interval * Self::GRACE_PERIOD_MULTIPLIER;

        if days_since_review <= grace_interval {
            // Successful review within grace period
            let new_repetitions = repetitions + 1;
            let new_interval = if new_repetitions == 1 {
                1.0
            } else if new_repetitions == 2 {
                6.0
            } else {
                interval * easiness_factor
            };

            (new_interval, new_repetitions, easiness_factor)
        } else {
            // Late review - reset to shorter interval
            let reset_interval = (interval * 0.5).max(1.0);
            (reset_interval, 0, easiness_factor)
        }
    }

    /// Calculate weight based on review timing.
    ///
    /// Within interval + grace period: weight = 1.0
    /// Beyond grace period: exponential decay
    fn calculate_weight_from_interval(
        current_weight: f32,
        interval: f64,
        days_since_review: f64,
    ) -> f32 {
        let grace_interval = interval * Self::GRACE_PERIOD_MULTIPLIER;

        if days_since_review <= grace_interval {
            // Within interval - maintain weight
            1.0
        } else {
            // Beyond interval - apply exponential decay
            let overdue_days = days_since_review - grace_interval;
            let decay_lambda = std::f64::consts::LN_2 / (interval * 2.0); // Half-life = 2x interval
            let decay_factor = (-decay_lambda * overdue_days).exp();
            (current_weight * decay_factor as f32).clamp(0.0, 1.0)
        }
    }
}

impl DecayStrategy for SpacedRepetitionDecay {
    fn name(&self) -> &'static str {
        "spaced-repetition"
    }

    fn calculate_decay(
        &self,
        memory: &Memory,
        params: &Value,
        _base_rate: f32,
    ) -> IcmResult<(f32, Value)> {
        let easiness_factor = Self::get_easiness_factor(params);
        let interval = Self::get_interval_days(params);
        let repetitions = Self::get_repetitions(params);
        let last_review = Self::get_last_review(params, memory);

        let days_since_review = Self::days_since_review(last_review);

        // Update SM-2 parameters
        let (new_interval, new_repetitions, new_easiness_factor) =
            Self::update_sm2_params(interval, repetitions, easiness_factor, days_since_review);

        // Calculate new weight
        let new_weight =
            Self::calculate_weight_from_interval(memory.weight, interval, days_since_review);

        // Store updated parameters
        let updated_params = json!({
            "easiness_factor": new_easiness_factor,
            "interval_days": new_interval,
            "repetitions": new_repetitions,
            "last_review": Utc::now().timestamp_millis(),
        });

        Ok((new_weight, updated_params))
    }

    fn calculate_temporal_score(&self, memory: &Memory, params: &Value) -> IcmResult<f32> {
        let interval = Self::get_interval_days(params);
        let last_review = Self::get_last_review(params, memory);
        let days_since_review = Self::days_since_review(last_review);

        let grace_interval = interval * Self::GRACE_PERIOD_MULTIPLIER;

        if days_since_review <= grace_interval {
            // Within interval - maximum relevance
            Ok(1.0)
        } else {
            // Beyond interval - decay based on how overdue
            let overdue_days = days_since_review - grace_interval;
            let decay_lambda = std::f64::consts::LN_2 / (interval * 2.0);
            let score = (-decay_lambda * overdue_days).exp() as f32;
            Ok(score.clamp(0.0, 1.0))
        }
    }

    fn default_params(&self) -> Value {
        json!({
            "easiness_factor": Self::DEFAULT_EASINESS_FACTOR,
            "interval_days": Self::DEFAULT_INTERVAL_DAYS,
            "repetitions": Self::DEFAULT_REPETITIONS,
            "last_review": Utc::now().timestamp_millis(),
        })
    }

    fn validate_params(&self, params: &Value) -> IcmResult<()> {
        if !params.is_object() {
            return Err(IcmError::InvalidInput(
                "Spaced repetition parameters must be a JSON object".to_string(),
            ));
        }

        // Validate easiness_factor
        if let Some(ef) = params.get("easiness_factor") {
            match ef.as_f64() {
                Some(value)
                    if (Self::MIN_EASINESS_FACTOR..=Self::MAX_EASINESS_FACTOR).contains(&value) => {
                }
                Some(value) => {
                    return Err(IcmError::InvalidInput(format!(
                        "easiness_factor must be between {} and {}, got {}",
                        Self::MIN_EASINESS_FACTOR,
                        Self::MAX_EASINESS_FACTOR,
                        value
                    )))
                }
                None => {
                    return Err(IcmError::InvalidInput(
                        "easiness_factor must be a number".to_string(),
                    ))
                }
            }
        }

        // Validate interval_days
        if let Some(interval) = params.get("interval_days") {
            match interval.as_f64() {
                Some(value) if value > 0.0 => {}
                Some(_) => {
                    return Err(IcmError::InvalidInput(
                        "interval_days must be positive".to_string(),
                    ))
                }
                None => {
                    return Err(IcmError::InvalidInput(
                        "interval_days must be a number".to_string(),
                    ))
                }
            }
        }

        // Validate repetitions
        if let Some(reps) = params.get("repetitions") {
            if !reps.is_u64() {
                return Err(IcmError::InvalidInput(
                    "repetitions must be a non-negative integer".to_string(),
                ));
            }
        }

        // Validate last_review (if present)
        if let Some(lr) = params.get("last_review") {
            if !lr.is_i64() {
                return Err(IcmError::InvalidInput(
                    "last_review must be a Unix timestamp in milliseconds".to_string(),
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

    fn create_test_memory(weight: f32, days_ago: i64) -> Memory {
        let now = Utc::now();
        let last_accessed = now - chrono::Duration::days(days_ago);

        Memory {
            id: "test-sm2".to_string(),
            created_at: now,
            updated_at: now,
            last_accessed,
            access_count: 0,
            weight,
            topic: "learning".to_string(),
            summary: "Test memory for SM-2".to_string(),
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
            decay_profile: Some("spaced-repetition".to_string()),
            decay_params: None,
        }
    }

    #[test]
    fn test_strategy_name() {
        let strategy = SpacedRepetitionDecay;
        assert_eq!(strategy.name(), "spaced-repetition");
    }

    #[test]
    fn test_default_params() {
        let strategy = SpacedRepetitionDecay;
        let params = strategy.default_params();

        assert_eq!(params["easiness_factor"].as_f64().unwrap(), 2.5);
        assert_eq!(params["interval_days"].as_f64().unwrap(), 1.0);
        assert_eq!(params["repetitions"].as_u64().unwrap(), 0);
        assert!(params["last_review"].is_i64());
    }

    #[test]
    fn test_validate_params_valid() {
        let strategy = SpacedRepetitionDecay;
        let params = json!({
            "easiness_factor": 2.5,
            "interval_days": 1.0,
            "repetitions": 0
        });

        assert!(strategy.validate_params(&params).is_ok());
    }

    #[test]
    fn test_validate_params_invalid_easiness() {
        let strategy = SpacedRepetitionDecay;

        // Too low
        let params = json!({"easiness_factor": 1.0});
        assert!(strategy.validate_params(&params).is_err());

        // Too high
        let params = json!({"easiness_factor": 5.0});
        assert!(strategy.validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_invalid_interval() {
        let strategy = SpacedRepetitionDecay;
        let params = json!({"interval_days": -1.0});

        assert!(strategy.validate_params(&params).is_err());
    }

    #[test]
    fn test_first_review_within_interval() {
        let strategy = SpacedRepetitionDecay;
        let memory = create_test_memory(1.0, 0); // Just accessed
        let params = strategy.default_params();

        let (new_weight, updated_params) =
            strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Weight should be 1.0 (within interval)
        assert!((new_weight - 1.0).abs() < 0.01);

        // Repetitions should increment
        assert_eq!(updated_params["repetitions"].as_u64().unwrap(), 1);

        // Interval should update (first review)
        assert_eq!(updated_params["interval_days"].as_f64().unwrap(), 1.0);
    }

    #[test]
    fn test_second_review_interval_increase() {
        let strategy = SpacedRepetitionDecay;
        let memory = create_test_memory(1.0, 1); // 1 day ago

        // Simulate first review already done
        let params = json!({
            "easiness_factor": 2.5,
            "interval_days": 1.0,
            "repetitions": 1,
            "last_review": (Utc::now() - chrono::Duration::days(1)).timestamp_millis()
        });

        let (new_weight, updated_params) =
            strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Weight should be 1.0 (within interval)
        assert!((new_weight - 1.0).abs() < 0.01);

        // Repetitions should increment
        assert_eq!(updated_params["repetitions"].as_u64().unwrap(), 2);

        // Interval should be 6 days (SM-2 second interval)
        assert_eq!(updated_params["interval_days"].as_f64().unwrap(), 6.0);
    }

    #[test]
    fn test_late_review_resets_interval() {
        let strategy = SpacedRepetitionDecay;
        let memory = create_test_memory(1.0, 10); // 10 days ago

        let params = json!({
            "easiness_factor": 2.5,
            "interval_days": 1.0,
            "repetitions": 2,
            "last_review": (Utc::now() - chrono::Duration::days(10)).timestamp_millis()
        });

        let (new_weight, updated_params) =
            strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Weight should have decayed
        assert!(new_weight < 1.0);

        // Repetitions should reset
        assert_eq!(updated_params["repetitions"].as_u64().unwrap(), 0);

        // Interval should reset to shorter value
        let new_interval = updated_params["interval_days"].as_f64().unwrap();
        assert!(new_interval < 1.0 || new_interval == 1.0);
    }

    #[test]
    fn test_temporal_score_within_interval() {
        let strategy = SpacedRepetitionDecay;
        let memory = create_test_memory(1.0, 0);

        let params = json!({
            "interval_days": 10.0,
            "last_review": Utc::now().timestamp_millis()
        });

        let score = strategy.calculate_temporal_score(&memory, &params).unwrap();

        // Should be maximum (within interval)
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_temporal_score_beyond_interval() {
        let strategy = SpacedRepetitionDecay;
        let memory = create_test_memory(1.0, 20); // 20 days ago

        let params = json!({
            "interval_days": 5.0,
            "last_review": (Utc::now() - chrono::Duration::days(20)).timestamp_millis()
        });

        let score = strategy.calculate_temporal_score(&memory, &params).unwrap();

        // Should be decayed (beyond interval + grace period)
        assert!(score < 1.0);
        assert!(score > 0.0);
    }

    #[test]
    fn test_progressive_interval_growth() {
        let strategy = SpacedRepetitionDecay;
        let mut memory = create_test_memory(1.0, 0);
        let mut params = strategy.default_params();

        // First review
        let (_, params1) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();
        assert_eq!(params1["repetitions"].as_u64().unwrap(), 1);
        assert_eq!(params1["interval_days"].as_f64().unwrap(), 1.0);

        // Second review (after 1 day)
        memory.last_accessed = Utc::now() - chrono::Duration::days(1);
        let (_, params2) = strategy.calculate_decay(&memory, &params1, 0.01).unwrap();
        assert_eq!(params2["repetitions"].as_u64().unwrap(), 2);
        assert_eq!(params2["interval_days"].as_f64().unwrap(), 6.0);

        // Third review (after 6 days)
        memory.last_accessed = Utc::now() - chrono::Duration::days(6);
        let (_, params3) = strategy.calculate_decay(&memory, &params2, 0.01).unwrap();
        assert_eq!(params3["repetitions"].as_u64().unwrap(), 3);
        // Should be interval * easiness_factor = 6.0 * 2.5 = 15.0
        assert_eq!(params3["interval_days"].as_f64().unwrap(), 15.0);
    }

    #[test]
    fn test_easiness_factor_clamping() {
        let strategy = SpacedRepetitionDecay;

        // Test lower bound
        let ef_low = SpacedRepetitionDecay::get_easiness_factor(&json!({"easiness_factor": 1.0}));
        assert_eq!(ef_low, SpacedRepetitionDecay::MIN_EASINESS_FACTOR);

        // Test upper bound
        let ef_high = SpacedRepetitionDecay::get_easiness_factor(&json!({"easiness_factor": 10.0}));
        assert_eq!(ef_high, SpacedRepetitionDecay::MAX_EASINESS_FACTOR);
    }
}

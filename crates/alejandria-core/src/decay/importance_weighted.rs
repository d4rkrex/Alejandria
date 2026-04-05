//! Importance-weighted decay strategy.
//!
//! This strategy adjusts decay rates based on the memory's importance level.
//! More important memories decay more slowly, while less important memories
//! decay faster. This ensures critical information remains accessible longer.
//!
//! ## Algorithm
//!
//! Uses exponential decay with importance-based multipliers:
//!
//! ```text
//! decay_rate = base_half_life / importance_multiplier
//! weight_new = weight_old * e^(-λ * days_since_access)
//! λ = ln(2) / (base_half_life * importance_multiplier)
//! ```
//!
//! ## Importance Multipliers
//!
//! - **Critical**: 4.0x slower decay (effectively no decay for practical purposes)
//! - **High**: 2.0x slower decay
//! - **Medium**: 1.0x (base rate, default)
//! - **Low**: 0.5x (2x faster decay)
//!
//! ## Use Cases
//!
//! - Security findings (CVSS-based importance)
//! - Architecture decisions (high importance)
//! - Temporary notes or experiments (low importance)
//! - Mixed-criticality knowledge bases
//!
//! ## Parameters
//!
//! - `base_half_life_days`: Base half-life for medium importance (default: 90 days)
//! - `importance_multiplier`: Manual multiplier override (optional)

use super::strategy::DecayStrategy;
use crate::{IcmError, IcmResult, Importance, Memory};
use serde_json::{json, Value};

/// Importance-weighted decay strategy.
///
/// This strategy uses the memory's importance level to adjust decay rates.
/// The importance field is parsed from the Memory struct's importance enum.
///
/// ## Example
///
/// ```ignore
/// use alejandria_core::decay::ImportanceWeightedDecay;
/// use alejandria_core::Importance;
///
/// let strategy = ImportanceWeightedDecay;
/// let params = json!({"base_half_life_days": 90.0});
///
/// // Critical importance: effective half-life = 90 * 4.0 = 360 days
/// // High importance: effective half-life = 90 * 2.0 = 180 days
/// // Medium importance: effective half-life = 90 * 1.0 = 90 days
/// // Low importance: effective half-life = 90 * 0.5 = 45 days
/// ```
#[derive(Debug, Clone)]
pub struct ImportanceWeightedDecay;

impl ImportanceWeightedDecay {
    /// Default base half-life for medium importance (3 months)
    pub const DEFAULT_BASE_HALF_LIFE_DAYS: f64 = 90.0;

    /// Minimum half-life (1 day)
    const MIN_HALF_LIFE_DAYS: f64 = 1.0;

    /// Maximum half-life (10 years)
    const MAX_HALF_LIFE_DAYS: f64 = 3650.0;

    /// Get importance multiplier from Memory's importance field.
    ///
    /// Maps Importance enum to decay multipliers:
    /// - Critical: 4.0x slower
    /// - High: 2.0x slower
    /// - Medium: 1.0x (base rate)
    /// - Low: 0.5x (faster decay)
    fn get_importance_multiplier(memory: &Memory) -> f64 {
        match memory.importance {
            Importance::Critical => 4.0,
            Importance::High => 2.0,
            Importance::Medium => 1.0,
            Importance::Low => 0.5,
        }
    }

    /// Extract base_half_life_days from parameters or use default.
    fn get_base_half_life(params: &Value) -> f64 {
        params
            .get("base_half_life_days")
            .and_then(|v| v.as_f64())
            .unwrap_or(Self::DEFAULT_BASE_HALF_LIFE_DAYS)
            .clamp(Self::MIN_HALF_LIFE_DAYS, Self::MAX_HALF_LIFE_DAYS)
    }

    /// Extract manual importance_multiplier override from parameters.
    fn get_manual_multiplier(params: &Value) -> Option<f64> {
        params
            .get("importance_multiplier")
            .and_then(|v| v.as_f64())
            .map(|v| v.clamp(0.1, 10.0))
    }

    /// Calculate effective half-life based on importance.
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

impl DecayStrategy for ImportanceWeightedDecay {
    fn name(&self) -> &'static str {
        "importance-weighted"
    }

    fn calculate_decay(
        &self,
        memory: &Memory,
        params: &Value,
        _base_rate: f32,
    ) -> IcmResult<(f32, Value)> {
        let base_half_life = Self::get_base_half_life(params);

        // Use manual multiplier if provided, otherwise use importance level
        let multiplier = Self::get_manual_multiplier(params)
            .unwrap_or_else(|| Self::get_importance_multiplier(memory));

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
            "importance_multiplier": multiplier,
            "importance_level": memory.importance.to_string(),
        });

        Ok((new_weight, updated_params))
    }

    fn calculate_temporal_score(&self, memory: &Memory, params: &Value) -> IcmResult<f32> {
        let base_half_life = Self::get_base_half_life(params);

        // Use manual multiplier if provided, otherwise use importance level
        let multiplier = Self::get_manual_multiplier(params)
            .unwrap_or_else(|| Self::get_importance_multiplier(memory));

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
        })
    }

    fn validate_params(&self, params: &Value) -> IcmResult<()> {
        if !params.is_object() {
            return Err(IcmError::InvalidInput(
                "Importance-weighted decay parameters must be a JSON object".to_string(),
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

        // Validate importance_multiplier (optional)
        if let Some(multiplier) = params.get("importance_multiplier") {
            match multiplier.as_f64() {
                Some(value) if value > 0.0 => {}
                Some(_) => {
                    return Err(IcmError::InvalidInput(
                        "importance_multiplier must be positive".to_string(),
                    ))
                }
                None => {
                    return Err(IcmError::InvalidInput(
                        "importance_multiplier must be a number".to_string(),
                    ))
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemorySource;

    fn create_test_memory(importance: Importance, weight: f32, days_ago: i64) -> Memory {
        let now = chrono::Utc::now();
        let last_accessed = now - chrono::Duration::days(days_ago);

        Memory {
            id: "test-importance".to_string(),
            created_at: now,
            updated_at: now,
            last_accessed,
            access_count: 0,
            weight,
            topic: "test".to_string(),
            summary: "Test memory for importance-weighted decay".to_string(),
            raw_excerpt: None,
            keywords: vec![],
            embedding: None,
            importance,
            source: MemorySource::User,
            related_ids: vec![],
            topic_key: None,
            revision_count: 1,
            duplicate_count: 0,
            last_seen_at: now,
            deleted_at: None,
            decay_profile: Some("importance-weighted".to_string()),
            decay_params: None,
        }
    }

    #[test]
    fn test_strategy_name() {
        let strategy = ImportanceWeightedDecay;
        assert_eq!(strategy.name(), "importance-weighted");
    }

    #[test]
    fn test_default_params() {
        let strategy = ImportanceWeightedDecay;
        let params = strategy.default_params();

        assert_eq!(params["base_half_life_days"].as_f64().unwrap(), 90.0);
    }

    #[test]
    fn test_validate_params_valid() {
        let strategy = ImportanceWeightedDecay;
        let params = json!({
            "base_half_life_days": 90.0,
            "importance_multiplier": 2.0
        });

        assert!(strategy.validate_params(&params).is_ok());
    }

    #[test]
    fn test_validate_params_invalid_half_life() {
        let strategy = ImportanceWeightedDecay;

        // Too low
        let params = json!({"base_half_life_days": 0.5});
        assert!(strategy.validate_params(&params).is_err());

        // Too high
        let params = json!({"base_half_life_days": 5000.0});
        assert!(strategy.validate_params(&params).is_err());
    }

    #[test]
    fn test_importance_multiplier_critical() {
        let memory = create_test_memory(Importance::Critical, 1.0, 0);
        let multiplier = ImportanceWeightedDecay::get_importance_multiplier(&memory);
        assert_eq!(multiplier, 4.0);
    }

    #[test]
    fn test_importance_multiplier_high() {
        let memory = create_test_memory(Importance::High, 1.0, 0);
        let multiplier = ImportanceWeightedDecay::get_importance_multiplier(&memory);
        assert_eq!(multiplier, 2.0);
    }

    #[test]
    fn test_importance_multiplier_medium() {
        let memory = create_test_memory(Importance::Medium, 1.0, 0);
        let multiplier = ImportanceWeightedDecay::get_importance_multiplier(&memory);
        assert_eq!(multiplier, 1.0);
    }

    #[test]
    fn test_importance_multiplier_low() {
        let memory = create_test_memory(Importance::Low, 1.0, 0);
        let multiplier = ImportanceWeightedDecay::get_importance_multiplier(&memory);
        assert_eq!(multiplier, 0.5);
    }

    #[test]
    fn test_critical_decays_slower_than_medium() {
        let strategy = ImportanceWeightedDecay;
        let params = json!({"base_half_life_days": 90.0});

        let critical = create_test_memory(Importance::Critical, 1.0, 90);
        let medium = create_test_memory(Importance::Medium, 1.0, 90);

        let (weight_critical, _) = strategy.calculate_decay(&critical, &params, 0.01).unwrap();
        let (weight_medium, _) = strategy.calculate_decay(&medium, &params, 0.01).unwrap();

        // After 90 days:
        // Critical: half-life = 360 days, should be ~85% weight
        // Medium: half-life = 90 days, should be ~50% weight
        assert!(weight_critical > weight_medium);
        assert!(weight_critical > 0.8);
        assert!(weight_medium > 0.4 && weight_medium < 0.6);
    }

    #[test]
    fn test_low_decays_faster_than_medium() {
        let strategy = ImportanceWeightedDecay;
        let params = json!({"base_half_life_days": 90.0});

        let low = create_test_memory(Importance::Low, 1.0, 45);
        let medium = create_test_memory(Importance::Medium, 1.0, 45);

        let (weight_low, _) = strategy.calculate_decay(&low, &params, 0.01).unwrap();
        let (weight_medium, _) = strategy.calculate_decay(&medium, &params, 0.01).unwrap();

        // After 45 days:
        // Low: half-life = 45 days, should be ~50% weight
        // Medium: half-life = 90 days, should be ~70% weight
        assert!(weight_low < weight_medium);
        assert!(weight_low > 0.4 && weight_low < 0.6);
        assert!(weight_medium > 0.6);
    }

    #[test]
    fn test_manual_multiplier_override() {
        let strategy = ImportanceWeightedDecay;
        let memory = create_test_memory(Importance::Medium, 1.0, 90);

        // Override with custom multiplier
        let params = json!({
            "base_half_life_days": 90.0,
            "importance_multiplier": 3.0
        });

        let (weight, updated_params) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Should use manual multiplier (3.0), not medium (1.0)
        assert_eq!(
            updated_params["importance_multiplier"].as_f64().unwrap(),
            3.0
        );

        // Effective half-life should be 90 * 3.0 = 270 days
        assert_eq!(
            updated_params["effective_half_life_days"].as_f64().unwrap(),
            270.0
        );

        // After 90 days with 270-day half-life, weight should be ~75%
        assert!(weight > 0.7);
    }

    #[test]
    fn test_temporal_score_reflects_importance() {
        let strategy = ImportanceWeightedDecay;
        let params = json!({"base_half_life_days": 90.0});

        let critical = create_test_memory(Importance::Critical, 1.0, 90);
        let medium = create_test_memory(Importance::Medium, 1.0, 90);

        let score_critical = strategy
            .calculate_temporal_score(&critical, &params)
            .unwrap();
        let score_medium = strategy.calculate_temporal_score(&medium, &params).unwrap();

        // Critical should have higher temporal score
        assert!(score_critical > score_medium);
    }

    #[test]
    fn test_effective_half_life_clamping() {
        // Test minimum clamping
        let effective = ImportanceWeightedDecay::effective_half_life(0.5, 1.0);
        assert_eq!(effective, ImportanceWeightedDecay::MIN_HALF_LIFE_DAYS);

        // Test maximum clamping
        let effective = ImportanceWeightedDecay::effective_half_life(5000.0, 1.0);
        assert_eq!(effective, ImportanceWeightedDecay::MAX_HALF_LIFE_DAYS);
    }

    #[test]
    fn test_zero_days_no_decay() {
        let strategy = ImportanceWeightedDecay;
        let memory = create_test_memory(Importance::Medium, 1.0, 0);
        let params = strategy.default_params();

        let (new_weight, _) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // No time passed, weight should be unchanged
        assert!((new_weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_stores_importance_level_in_params() {
        let strategy = ImportanceWeightedDecay;
        let memory = create_test_memory(Importance::High, 1.0, 30);
        let params = strategy.default_params();

        let (_, updated_params) = strategy.calculate_decay(&memory, &params, 0.01).unwrap();

        // Should store importance level for debugging/auditing
        assert_eq!(updated_params["importance_level"].as_str().unwrap(), "high");
        assert_eq!(
            updated_params["importance_multiplier"].as_f64().unwrap(),
            2.0
        );
    }
}

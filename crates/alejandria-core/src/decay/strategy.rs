//! DecayStrategy trait definition for pluggable decay algorithms.

use serde_json::Value;
use crate::{Memory, IcmResult};

/// Trait for pluggable decay algorithms.
///
/// Implementations compute temporal relevance scores and weight decay
/// for observations based on algorithm-specific logic and parameters.
///
/// # Examples
///
/// ```ignore
/// use alejandria_core::decay::{DecayStrategy, ExponentialDecay};
///
/// let strategy = ExponentialDecay::new(30.0); // 30-day half-life
/// let (new_weight, updated_params) = strategy.calculate_decay(
///     &observation,
///     &params,
///     0.01  // base rate
/// )?;
/// ```
pub trait DecayStrategy: Send + Sync {
    /// Returns the unique identifier for this strategy (e.g., "exponential", "spaced-repetition").
    fn name(&self) -> &'static str;
    
    /// Calculate new weight after decay for an observation.
    ///
    /// # Arguments
    /// * `observation` - The observation to decay
    /// * `params` - Algorithm-specific parameters (from decay_params JSONB)
    /// * `base_rate` - Base decay rate per day (e.g., 0.01 for 1%/day)
    ///
    /// # Returns
    /// * `new_weight` - Updated weight (0.0-1.0)
    /// * `updated_params` - Updated parameters (e.g., SM-2 increments repetition count)
    fn calculate_decay(
        &self,
        observation: &Memory,
        params: &Value,
        base_rate: f32,
    ) -> IcmResult<(f32, Value)>;
    
    /// Calculate temporal relevance score for search ranking.
    ///
    /// Returns a multiplier (0.0-1.0) to apply to hybrid search scores.
    /// Higher values = observation is more temporally relevant.
    ///
    /// # Arguments
    /// * `observation` - The observation to score
    /// * `params` - Algorithm-specific parameters
    ///
    /// # Returns
    /// Temporal relevance factor (0.0-1.0)
    fn calculate_temporal_score(
        &self,
        observation: &Memory,
        params: &Value,
    ) -> IcmResult<f32>;
    
    /// Initialize default parameters for this strategy.
    ///
    /// Called when setting a decay profile on an observation without existing params.
    fn default_params(&self) -> Value;
    
    /// Validate parameters for this strategy.
    ///
    /// Returns Err if params are malformed or missing required fields.
    fn validate_params(&self, params: &Value) -> IcmResult<()>;
}

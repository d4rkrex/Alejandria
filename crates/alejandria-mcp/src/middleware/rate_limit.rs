//! Rate limiting middleware
//!
//! Implements token bucket algorithm per API key to prevent DoS attacks.
//! Configuration from design.md: 100 req/min, burst 20
//! Returns 429 Too Many Requests when limit exceeded.

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per minute per API key
    pub requests_per_minute: u32,
    
    /// Maximum burst size (tokens available at once)
    pub burst_size: u32,
    
    /// Window duration for rate limiting
    pub window_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            burst_size: 20,
            window_duration: Duration::from_secs(60),
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Available tokens
    tokens: f64,
    
    /// Maximum tokens (burst capacity)
    capacity: f64,
    
    /// Token refill rate (tokens per second)
    refill_rate: f64,
    
    /// Last refill timestamp
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }
    
    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        // Add tokens based on elapsed time
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }
    
    /// Try to consume one token
    fn try_consume(&mut self) -> bool {
        self.refill();
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Rate limiter state
pub struct RateLimiter {
    /// Token buckets per API key hash
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    
    /// Configuration
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Check if request is allowed for given API key
    pub async fn check(&self, api_key_hash: &str) -> bool {
        let mut buckets = self.buckets.write().await;
        
        // Get or create bucket for this API key
        let bucket = buckets.entry(api_key_hash.to_string()).or_insert_with(|| {
            let refill_rate = self.config.requests_per_minute as f64 / 60.0; // tokens per second
            TokenBucket::new(self.config.burst_size, refill_rate)
        });
        
        bucket.try_consume()
    }
}

/// Rate limit layer for axum
#[derive(Clone)]
pub struct RateLimitLayer {
    #[allow(dead_code)]
    limiter: Arc<RateLimiter>,
}

impl RateLimitLayer {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(config)),
        }
    }
    
    /// Middleware function
    pub async fn middleware(
        limiter: Arc<RateLimiter>,
        req: Request<Body>,
        next: Next,
    ) -> Response {
        // Extract API key hash from request extensions (set by auth middleware)
        let api_key_hash = req
            .extensions()
            .get::<crate::transport::http::auth::AuthContext>()
            .map(|ctx| ctx.api_key_hash.clone());
        
        if let Some(key_hash) = api_key_hash {
            // Check rate limit
            if !limiter.check(&key_hash).await {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    "Rate limit exceeded. Maximum 100 requests per minute.",
                )
                    .into_response();
            }
        }
        
        // Continue to next middleware/handler
        next.run(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_initial_capacity() {
        let bucket = TokenBucket::new(20, 1.0);
        assert_eq!(bucket.tokens, 20.0);
        assert_eq!(bucket.capacity, 20.0);
    }

    #[test]
    fn test_token_bucket_consumption() {
        let mut bucket = TokenBucket::new(5, 1.0);
        
        // Should succeed for first 5 requests
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        
        // Should fail when tokens exhausted
        assert!(!bucket.try_consume());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 10,
            window_duration: Duration::from_secs(60),
        };
        
        let limiter = RateLimiter::new(config);
        let key = "test_key_hash";
        
        // Should allow burst requests
        for _ in 0..10 {
            assert!(limiter.check(key).await);
        }
        
        // Should deny when burst exhausted
        assert!(!limiter.check(key).await);
    }
}

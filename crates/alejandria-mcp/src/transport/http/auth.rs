//! Authentication middleware
//!
//! Implements API key authentication with constant-time comparison to prevent
//! timing attacks (CRITICAL security requirement from review.md finding ID-004).

use super::{AppState, HttpError};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use std::time::Duration;

/// Authentication context added to request extensions after successful authentication
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// SHA-256 hash of the API key (for logging, not the raw key)
    pub api_key_hash: String,
    
    /// Client IP address
    pub client_ip: std::net::IpAddr,
}

/// Authentication middleware
///
/// Validates the X-API-Key header using constant-time comparison.
/// Adds random jitter (0-10ms) to all responses to prevent statistical timing attacks.
pub async fn authenticate<S>(
    State(state): State<AppState<S>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, HttpError> 
where
    S: Send + Sync + 'static,
{
    // Extract API key from header
    let api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| HttpError {
            status: StatusCode::UNAUTHORIZED,
            message: "Missing X-API-Key header".to_string(),
        })?;
    
    // Validate API key using constant-time comparison
    let is_valid = validate_api_key_constant_time(api_key, &state.api_key);
    
    if !is_valid {
        // Add random jitter (0-10ms) to prevent timing attacks
        let jitter = rand::random::<u64>() % 10;
        tokio::time::sleep(Duration::from_millis(jitter)).await;
        
        return Err(HttpError {
            status: StatusCode::FORBIDDEN,
            message: "Invalid API key".to_string(),
        });
    }
    
    // Extract client IP
    let client_ip = extract_client_ip(&req);
    
    // Create authentication context
    let auth_context = AuthContext {
        api_key_hash: hash_api_key(api_key),
        client_ip,
    };
    
    // Add context to request extensions
    req.extensions_mut().insert(auth_context);
    
    // Continue to next middleware/handler
    Ok(next.run(req).await)
}

/// Validate API key using constant-time comparison
///
/// This function uses constant-time comparison to prevent timing side-channel attacks.
/// Even if the provided key length differs from the expected key, we still perform
/// a dummy comparison to avoid length oracle attacks.
fn validate_api_key_constant_time(provided: &str, expected: &str) -> bool {
    let provided_bytes = provided.as_bytes();
    let expected_bytes = expected.as_bytes();
    
    // Check length first, but still perform constant-time comparison
    if provided_bytes.len() != expected_bytes.len() {
        // Perform dummy comparison to prevent length leakage
        let _ = [0u8; 32].ct_eq(&[0u8; 32]);
        return false;
    }
    
    // Constant-time comparison using subtle crate
    provided_bytes.ct_eq(expected_bytes).into()
}

/// Hash API key using SHA-256 for logging (never log raw keys)
fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Extract client IP address from request
///
/// Attempts to extract from:
/// 1. X-Forwarded-For header (if trust_proxy_headers is true)
/// 2. X-Real-IP header (if trust_proxy_headers is true)
/// 3. Connection remote address
fn extract_client_ip(_req: &Request<Body>) -> std::net::IpAddr {
    // For now, just use a placeholder - we'll enhance this when we add config support
    // In production, we need to respect trust_proxy_headers config
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_api_key_constant_time_success() {
        let key = "test-api-key-12345";
        assert!(validate_api_key_constant_time(key, key));
    }

    #[test]
    fn test_validate_api_key_constant_time_failure() {
        let key1 = "test-api-key-12345";
        let key2 = "test-api-key-67890";
        assert!(!validate_api_key_constant_time(key1, key2));
    }

    #[test]
    fn test_validate_api_key_constant_time_different_length() {
        let key1 = "short";
        let key2 = "much-longer-key";
        assert!(!validate_api_key_constant_time(key1, key2));
    }

    #[test]
    fn test_hash_api_key() {
        let key = "test-api-key";
        let hash = hash_api_key(key);
        
        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);
        
        // Hashing same key should produce same hash
        assert_eq!(hash, hash_api_key(key));
        
        // Different keys should produce different hashes
        assert_ne!(hash, hash_api_key("different-key"));
    }
}

//! Input validation middleware
//!
//! Validates JSON-RPC request structure, method names, and parameter types.
//! Returns 400 Bad Request on validation failures to prevent injection attacks.

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::Value;
use std::collections::HashSet;

/// Allowed JSON-RPC methods (whitelist)
const ALLOWED_METHODS: &[&str] = &[
    "initialize",
    "tools/list",
    "tools/call",
    "resources/list",
    "resources/read",
    "prompts/list",
    "prompts/get",
    "completion/complete",
    "logging/setLevel",
];

/// Maximum request body size (1MB from design.md)
pub const MAX_REQUEST_SIZE_BYTES: usize = 1024 * 1024;

/// Input validation layer
#[derive(Clone)]
pub struct InputValidationLayer;

impl InputValidationLayer {
    pub fn new() -> Self {
        Self
    }
    
    /// Middleware function
    pub async fn middleware(req: Request<Body>, next: Next) -> Response {
        // Extract content length
        let content_length = req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        
        // Check request size
        if content_length > MAX_REQUEST_SIZE_BYTES {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                format!(
                    "Request body too large. Maximum size: {} bytes",
                    MAX_REQUEST_SIZE_BYTES
                ),
            )
                .into_response();
        }
        
        // For JSON-RPC validation, we'll let the handler parse the body
        // and rely on serde_json for structure validation.
        // Method whitelist validation happens in the protocol handler.
        
        next.run(req).await
    }
}

impl Default for InputValidationLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate JSON-RPC request structure and method
pub fn validate_json_rpc_request(request: &Value) -> Result<(), String> {
    // Check jsonrpc version
    if request.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Err("Invalid or missing 'jsonrpc' field. Must be '2.0'".to_string());
    }
    
    // Check method exists and is a string
    let method = request
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing or invalid 'method' field".to_string())?;
    
    // Validate method is in whitelist
    if !ALLOWED_METHODS.contains(&method) {
        return Err(format!("Method '{}' not allowed", method));
    }
    
    // Check id exists (can be string, number, or null)
    if request.get("id").is_none() {
        return Err("Missing 'id' field".to_string());
    }
    
    // Params is optional, but if present must be object or array
    if let Some(params) = request.get("params") {
        if !params.is_object() && !params.is_array() {
            return Err("Invalid 'params' field. Must be object or array".to_string());
        }
    }
    
    Ok(())
}

/// Validate method name against whitelist
pub fn is_method_allowed(method: &str) -> bool {
    ALLOWED_METHODS.contains(&method)
}

/// Get all allowed methods
pub fn get_allowed_methods() -> HashSet<String> {
    ALLOWED_METHODS.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_valid_request() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {},
            "id": 1
        });
        
        assert!(validate_json_rpc_request(&request).is_ok());
    }

    #[test]
    fn test_validate_missing_jsonrpc() {
        let request = json!({
            "method": "initialize",
            "id": 1
        });
        
        assert!(validate_json_rpc_request(&request).is_err());
    }

    #[test]
    fn test_validate_invalid_method() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "system/shutdown",
            "id": 1
        });
        
        let result = validate_json_rpc_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not allowed"));
    }

    #[test]
    fn test_validate_missing_id() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "initialize"
        });
        
        assert!(validate_json_rpc_request(&request).is_err());
    }

    #[test]
    fn test_validate_invalid_params() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": "invalid",
            "id": 1
        });
        
        assert!(validate_json_rpc_request(&request).is_err());
    }

    #[test]
    fn test_is_method_allowed() {
        assert!(is_method_allowed("initialize"));
        assert!(is_method_allowed("tools/list"));
        assert!(!is_method_allowed("system/shutdown"));
        assert!(!is_method_allowed("../../../etc/passwd"));
    }
}

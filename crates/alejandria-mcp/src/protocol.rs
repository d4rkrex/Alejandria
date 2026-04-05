//! JSON-RPC 2.0 protocol types for MCP server
//!
//! Implements the core JSON-RPC 2.0 request/response/error structures
//! as specified by the Model Context Protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (must be "2.0")
    pub jsonrpc: String,

    /// Request identifier (can be string, number, or null for notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// Method name to invoke
    pub method: String,

    /// Method parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version (must be "2.0")
    pub jsonrpc: String,

    /// Request identifier (echoed from request)
    pub id: Value,

    /// Result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Parse error (-32700)
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    /// Invalid request (-32600)
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    /// Method not found (-32601)
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }

    /// Invalid params (-32602)
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {}", message.into()),
            data: None,
        }
    }

    /// Internal error (-32603)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }

    /// Custom error: Not found (-32001)
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: -32001,
            message: format!("Not found: {}", message.into()),
            data: None,
        }
    }

    /// Custom error: Already exists (-32002)
    pub fn already_exists(message: impl Into<String>) -> Self {
        Self {
            code: -32002,
            message: format!("Already exists: {}", message.into()),
            data: None,
        }
    }
}

/// MCP Tool Call Parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    /// Tool name
    pub name: String,

    /// Tool arguments
    #[serde(default)]
    pub arguments: Value,
}

/// MCP Tool Result Content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    /// Content type (always "text" for now)
    #[serde(rename = "type")]
    pub content_type: String,

    /// Text content
    pub text: String,
}

impl ToolContent {
    /// Create a text content response
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".to_string(),
            text: text.into(),
        }
    }
}

/// MCP Tool Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Content array (usually single text element)
    pub content: Vec<ToolContent>,

    /// Whether the tool call was successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    /// Create a success result
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: None,
        }
    }

    /// Create an error result
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({})),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_response_success() {
        let resp = JsonRpcResponse::success(json!(1), json!({"status": "ok"}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_response_error() {
        let error = JsonRpcError::invalid_params("Missing field: content");
        let resp = JsonRpcResponse::error(json!(1), error);
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32602);
    }

    #[test]
    fn test_tool_content() {
        let content = ToolContent::text("Memory stored");
        assert_eq!(content.content_type, "text");
        assert_eq!(content.text, "Memory stored");
    }

    #[test]
    fn test_tool_result() {
        let result = ToolResult::success("Operation completed");
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].text, "Operation completed");
        assert!(result.is_error.is_none());

        let error = ToolResult::error("Operation failed");
        assert_eq!(error.is_error, Some(true));
    }
}

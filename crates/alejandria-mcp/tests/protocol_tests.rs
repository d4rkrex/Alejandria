//! Integration tests for the JSON-RPC 2.0 protocol dispatch layer.
//!
//! Tests the `handle_request` function which is the main dispatch point for:
//! - `initialize` → server capabilities and protocol version
//! - `tools/list` → returns all 20 tool schemas
//! - `tools/call` → dispatches to individual tool handlers
//! - Error handling for invalid requests, unknown methods, parse errors

use alejandria_mcp::{handle_request, JsonRpcRequest, JsonRpcResponse};
use alejandria_storage::SqliteStore;
use serde_json::{json, Value};
use std::sync::Arc;

/// Helper: create an in-memory store wrapped in Arc
fn test_store() -> Arc<SqliteStore> {
    Arc::new(SqliteStore::open_in_memory().expect("Failed to create in-memory store"))
}

/// Helper: build a valid JSON-RPC 2.0 request
fn rpc_request(id: Value, method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(id),
        method: method.to_string(),
        params,
    }
}

// ============================================================
// Initialize
// ============================================================

#[test]
fn test_initialize_returns_capabilities() {
    let store = test_store();
    let req = rpc_request(json!(1), "initialize", Some(json!({})));

    let resp = handle_request(req, store);

    assert!(
        resp.error.is_none(),
        "Expected success, got error: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "alejandria");
    assert!(result["capabilities"]["tools"].is_object());
}

#[test]
fn test_initialize_without_params() {
    let store = test_store();
    let req = rpc_request(json!(2), "initialize", None);

    let resp = handle_request(req, store);

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(result["protocolVersion"], "2024-11-05");
}

#[test]
fn test_initialize_echoes_request_id() {
    let store = test_store();
    let req = rpc_request(json!("abc-123"), "initialize", None);

    let resp = handle_request(req, store);

    assert_eq!(resp.id, json!("abc-123"));
}

// ============================================================
// Tools/List
// ============================================================

#[test]
fn test_tools_list_returns_20_tools() {
    let store = test_store();
    let req = rpc_request(json!(1), "tools/list", None);

    let resp = handle_request(req, store);

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let tools = result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 20, "Expected 20 tools (11 memory + 9 memoir)");
}

#[test]
fn test_tools_list_contains_all_memory_tools() {
    let store = test_store();
    let req = rpc_request(json!(1), "tools/list", None);

    let resp = handle_request(req, store);
    let result = resp.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    let expected_memory_tools = [
        "mem_store",
        "mem_recall",
        "mem_update",
        "mem_forget",
        "mem_consolidate",
        "mem_list_topics",
        "mem_stats",
        "mem_health",
        "mem_embed_all",
        "mem_export",
        "mem_import",
    ];

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    for expected in &expected_memory_tools {
        assert!(
            tool_names.contains(expected),
            "Missing memory tool: {}",
            expected
        );
    }
}

#[test]
fn test_tools_list_contains_all_memoir_tools() {
    let store = test_store();
    let req = rpc_request(json!(1), "tools/list", None);

    let resp = handle_request(req, store);
    let result = resp.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    let expected_memoir_tools = [
        "memoir_create",
        "memoir_list",
        "memoir_show",
        "memoir_add_concept",
        "memoir_refine",
        "memoir_search",
        "memoir_search_all",
        "memoir_link",
        "memoir_inspect",
    ];

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    for expected in &expected_memoir_tools {
        assert!(
            tool_names.contains(expected),
            "Missing memoir tool: {}",
            expected
        );
    }
}

#[test]
fn test_tools_list_schemas_have_required_fields() {
    let store = test_store();
    let req = rpc_request(json!(1), "tools/list", None);

    let resp = handle_request(req, store);
    let result = resp.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    for tool in tools {
        assert!(tool["name"].is_string(), "Tool missing 'name'");
        assert!(
            tool["description"].is_string(),
            "Tool '{}' missing 'description'",
            tool["name"]
        );
        assert!(
            tool["inputSchema"].is_object(),
            "Tool '{}' missing 'inputSchema'",
            tool["name"]
        );
    }
}

// ============================================================
// Invalid JSON-RPC version
// ============================================================

#[test]
fn test_invalid_jsonrpc_version() {
    let store = test_store();
    let req = JsonRpcRequest {
        jsonrpc: "1.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: None,
    };

    let resp = handle_request(req, store);

    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32600, "Expected invalid request error code");
    assert!(err.message.contains("2.0"));
}

#[test]
fn test_empty_jsonrpc_version() {
    let store = test_store();
    let req = JsonRpcRequest {
        jsonrpc: "".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: None,
    };

    let resp = handle_request(req, store);

    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32600);
}

// ============================================================
// Method not found
// ============================================================

#[test]
fn test_unknown_method() {
    let store = test_store();
    let req = rpc_request(json!(1), "nonexistent/method", None);

    let resp = handle_request(req, store);

    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601, "Expected method not found error code");
    assert!(err.message.contains("nonexistent/method"));
}

#[test]
fn test_empty_method() {
    let store = test_store();
    let req = rpc_request(json!(1), "", None);

    let resp = handle_request(req, store);

    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32601);
}

// ============================================================
// tools/call dispatch errors
// ============================================================

#[test]
fn test_tools_call_missing_params() {
    let store = test_store();
    let req = rpc_request(json!(1), "tools/call", None);

    let resp = handle_request(req, store);

    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32602, "Expected invalid params error code");
}

#[test]
fn test_tools_call_invalid_params_structure() {
    let store = test_store();
    // ToolCallParams expects { name, arguments } — send garbage instead
    let req = rpc_request(json!(1), "tools/call", Some(json!("not an object")));

    let resp = handle_request(req, store);

    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[test]
fn test_tools_call_unknown_tool() {
    let store = test_store();
    let req = rpc_request(
        json!(1),
        "tools/call",
        Some(json!({
            "name": "nonexistent_tool",
            "arguments": {}
        })),
    );

    let resp = handle_request(req, store);

    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(
        err.code, -32601,
        "Expected method not found for unknown tool"
    );
    assert!(err.message.contains("nonexistent_tool"));
}

// ============================================================
// tools/call → mem_store (quick smoke via dispatch)
// ============================================================

#[test]
fn test_tools_call_dispatches_to_mem_store() {
    let store = test_store();
    let req = rpc_request(
        json!(1),
        "tools/call",
        Some(json!({
            "name": "mem_store",
            "arguments": {
                "content": "Integration test memory via dispatch",
                "topic": "testing"
            }
        })),
    );

    let resp = handle_request(req, store);

    assert!(
        resp.error.is_none(),
        "mem_store via dispatch failed: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    // ToolResult has content array with text
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("Memory stored"),
        "Expected 'Memory stored' in response, got: {}",
        text
    );
}

#[test]
fn test_tools_call_dispatches_to_memoir_create() {
    let store = test_store();
    let req = rpc_request(
        json!(1),
        "tools/call",
        Some(json!({
            "name": "memoir_create",
            "arguments": {
                "name": "test-memoir-dispatch",
                "description": "Created via protocol dispatch"
            }
        })),
    );

    let resp = handle_request(req, store);

    assert!(
        resp.error.is_none(),
        "memoir_create via dispatch failed: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    assert_eq!(result["name"], "test-memoir-dispatch");
}

// ============================================================
// Request ID handling
// ============================================================

#[test]
fn test_null_request_id() {
    let store = test_store();
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialize".to_string(),
        params: None,
    };

    let resp = handle_request(req, store);

    // When id is None, server should use null
    assert_eq!(resp.id, Value::Null);
    assert!(resp.error.is_none());
}

#[test]
fn test_numeric_request_id() {
    let store = test_store();
    let req = rpc_request(json!(42), "initialize", None);

    let resp = handle_request(req, store);

    assert_eq!(resp.id, json!(42));
}

#[test]
fn test_string_request_id() {
    let store = test_store();
    let req = rpc_request(json!("request-abc"), "initialize", None);

    let resp = handle_request(req, store);

    assert_eq!(resp.id, json!("request-abc"));
}

// ============================================================
// Response structure validation
// ============================================================

#[test]
fn test_success_response_has_no_error() {
    let store = test_store();
    let req = rpc_request(json!(1), "initialize", None);

    let resp = handle_request(req, store);

    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn test_error_response_has_no_result() {
    let store = test_store();
    let req = rpc_request(json!(1), "nonexistent", None);

    let resp = handle_request(req, store);

    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
}

#[test]
fn test_response_serialization_roundtrip() {
    let store = test_store();
    let req = rpc_request(json!(1), "initialize", None);

    let resp = handle_request(req, store);

    // Serialize to JSON and back
    let json_str = serde_json::to_string(&resp).expect("Failed to serialize response");
    let deserialized: JsonRpcResponse =
        serde_json::from_str(&json_str).expect("Failed to deserialize response");

    assert_eq!(deserialized.jsonrpc, "2.0");
    assert_eq!(deserialized.id, json!(1));
    assert!(deserialized.result.is_some());
}

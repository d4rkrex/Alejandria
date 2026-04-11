//! MCP Server protocol handler
//!
//! This module implements the transport-agnostic JSON-RPC 2.0 protocol handler
//! that dispatches requests to appropriate tool implementations.

use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolCallParams};
use crate::tools;
use crate::transport::{StdioTransport, Transport};
use alejandria_core::{MemoirStore, MemoryStore};
use serde_json::{json, Value};
use std::io;

/// Run the MCP server with stdio transport (legacy entry point)
///
/// This is a convenience wrapper around `StdioTransport::run()` for backward compatibility.
///
/// # Arguments
///
/// * `store` - Memory store implementation
///
/// # Errors
///
/// Returns error if transport encounters unrecoverable error.
pub fn run_stdio_server<S: MemoryStore + MemoirStore + Clone + 'static>(
    store: S,
) -> io::Result<()> {
    StdioTransport
        .run(store)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Handle a single JSON-RPC request.
///
/// This is the main dispatch function for incoming JSON-RPC requests.
/// Exposed publicly for integration testing.
pub fn handle_request<S: MemoryStore + MemoirStore>(
    request: JsonRpcRequest,
    store: std::sync::Arc<S>,
) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        return JsonRpcResponse::error(id, JsonRpcError::invalid_request("jsonrpc must be '2.0'"));
    }

    // Dispatch based on method
    match request.method.as_str() {
        "initialize" => handle_initialize(id, request.params),
        "tools/list" => handle_list_tools(id, request.params),
        "tools/call" => handle_tool_call(id, request.params, store),
        _ => JsonRpcResponse::error(id, JsonRpcError::method_not_found(&request.method)),
    }
}

/// Handle MCP initialization
fn handle_initialize(id: Value, _params: Option<Value>) -> JsonRpcResponse {
    let result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "alejandria",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    JsonRpcResponse::success(id, result)
}

/// Handle tools/list request - returns all available tool schemas
fn handle_list_tools(id: Value, _params: Option<Value>) -> JsonRpcResponse {
    let tools = vec![
        // Memory Tools
        json!({
            "name": "mem_store",
            "description": "Store a new memory or update existing via topic_key upsert",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Memory content (required)"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Brief summary of the memory"
                    },
                    "importance": {
                        "type": "string",
                        "enum": ["critical", "high", "medium", "low"],
                        "description": "Importance level affecting decay rate"
                    },
                    "topic": {
                        "type": "string",
                        "description": "Topic for organization"
                    },
                    "topic_key": {
                        "type": "string",
                        "description": "Unique key for upsert workflow"
                    },
                    "source": {
                        "type": "string",
                        "description": "Source of the memory"
                    },
                    "related_ids": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Related memory IDs"
                    }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "mem_recall",
            "description": "Search and recall memories using hybrid search (BM25 + vector similarity)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (required)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 10)",
                        "default": 10
                    },
                    "min_score": {
                        "type": "number",
                        "description": "Minimum similarity score (0.0-1.0)",
                        "default": 0.0
                    },
                    "topic": {
                        "type": "string",
                        "description": "Filter by topic"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "mem_update",
            "description": "Update an existing memory by ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Memory ID (ULID format, required)"
                    },
                    "content": {
                        "type": "string",
                        "description": "New content"
                    },
                    "summary": {
                        "type": "string",
                        "description": "New summary"
                    },
                    "importance": {
                        "type": "string",
                        "enum": ["critical", "high", "medium", "low"],
                        "description": "New importance level"
                    },
                    "topic": {
                        "type": "string",
                        "description": "New topic"
                    }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "mem_forget",
            "description": "Soft-delete a memory by ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Memory ID to delete (required)"
                    }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "mem_consolidate",
            "description": "Consolidate memories in a topic into a high-level summary",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Topic to consolidate (required)"
                    },
                    "min_weight": {
                        "type": "number",
                        "description": "Minimum weight threshold (default: 0.5)",
                        "default": 0.5
                    },
                    "min_memories": {
                        "type": "integer",
                        "description": "Minimum number of memories required (default: 3)",
                        "default": 3
                    }
                },
                "required": ["topic"]
            }
        }),
        json!({
            "name": "mem_list_topics",
            "description": "List all topics with counts and statistics",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of topics (default: 100)",
                        "default": 100
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Offset for pagination (default: 0)",
                        "default": 0
                    },
                    "min_count": {
                        "type": "integer",
                        "description": "Minimum memory count (default: 1)",
                        "default": 1
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "mem_stats",
            "description": "Get memory statistics",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "mem_health",
            "description": "Check system health (database, FTS, vector search, embeddings)",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "mem_embed_all",
            "description": "Batch embed existing memories that lack embeddings",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "batch_size": {
                        "type": "integer",
                        "description": "Batch size for processing (default: 100)",
                        "default": 100
                    },
                    "skip_existing": {
                        "type": "boolean",
                        "description": "Skip memories that already have embeddings (default: true)",
                        "default": true
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "mem_export",
            "description": "Export memories to file (JSON, CSV, or Markdown format)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "output": {
                        "type": "string",
                        "description": "Output file path (required)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["json", "csv", "markdown"],
                        "description": "Export format (default: json)",
                        "default": "json"
                    },
                    "include_deleted": {
                        "type": "boolean",
                        "description": "Include soft-deleted memories (default: false)",
                        "default": false
                    },
                    "filters": {
                        "type": "object",
                        "description": "Optional filters",
                        "properties": {
                            "session_id": {
                                "type": "string",
                                "description": "Filter by session ID"
                            },
                            "importance": {
                                "type": "string",
                                "description": "Filter by importance level"
                            },
                            "tags": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Filter by tags"
                            },
                            "decay_profile": {
                                "type": "string",
                                "description": "Filter by decay profile"
                            }
                        }
                    }
                },
                "required": ["output"]
            }
        }),
        json!({
            "name": "mem_import",
            "description": "Import memories from file (JSON or CSV format)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input file path (required)"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["skip", "update", "replace"],
                        "description": "Import mode for conflicts: skip (default), update, replace",
                        "default": "skip"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Dry run: validate without importing (default: false)",
                        "default": false
                    }
                },
                "required": ["input"]
            }
        }),
        // Memoir Tools (Knowledge Graph)
        json!({
            "name": "memoir_create",
            "description": "Create a new memoir (knowledge graph)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Memoir name (unique, required)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Memoir description"
                    }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "memoir_list",
            "description": "List all memoirs with concept and link counts",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "memoir_show",
            "description": "Get full memoir graph with all concepts and links",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Memoir name (required)"
                    }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "memoir_add_concept",
            "description": "Add a concept to a memoir",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "memoir": {
                        "type": "string",
                        "description": "Memoir name (required)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Concept name (required)"
                    },
                    "definition": {
                        "type": "string",
                        "description": "Concept definition"
                    },
                    "labels": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Labels for categorization"
                    }
                },
                "required": ["memoir", "name"]
            }
        }),
        json!({
            "name": "memoir_refine",
            "description": "Update concept definition and/or labels",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "memoir": {
                        "type": "string",
                        "description": "Memoir name (required)"
                    },
                    "concept": {
                        "type": "string",
                        "description": "Concept name (required)"
                    },
                    "definition": {
                        "type": "string",
                        "description": "New definition"
                    },
                    "labels": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "New labels"
                    }
                },
                "required": ["memoir", "concept"]
            }
        }),
        json!({
            "name": "memoir_search",
            "description": "Search concepts within a memoir using FTS5",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "memoir": {
                        "type": "string",
                        "description": "Memoir name (required)"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (required)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 10)",
                        "default": 10
                    }
                },
                "required": ["memoir", "query"]
            }
        }),
        json!({
            "name": "memoir_search_all",
            "description": "Search concepts across all memoirs using FTS5",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (required)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 10)",
                        "default": 10
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "memoir_link",
            "description": "Create typed link between two concepts",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "memoir": {
                        "type": "string",
                        "description": "Memoir name (required)"
                    },
                    "source": {
                        "type": "string",
                        "description": "Source concept name (required)"
                    },
                    "target": {
                        "type": "string",
                        "description": "Target concept name (required)"
                    },
                    "relation": {
                        "type": "string",
                        "enum": ["IsA", "HasProperty", "RelatedTo", "Causes", "PrerequisiteOf", "ExampleOf", "Contradicts", "SimilarTo", "PartOf"],
                        "description": "Relation type (required)"
                    },
                    "weight": {
                        "type": "number",
                        "description": "Link weight (default: 1.0)",
                        "default": 1.0
                    }
                },
                "required": ["memoir", "source", "target", "relation"]
            }
        }),
        json!({
            "name": "memoir_inspect",
            "description": "Inspect concept neighborhood using BFS traversal",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "memoir": {
                        "type": "string",
                        "description": "Memoir name (required)"
                    },
                    "concept": {
                        "type": "string",
                        "description": "Concept name (required)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Traversal depth (default: 1)",
                        "default": 1
                    }
                },
                "required": ["memoir", "concept"]
            }
        }),
    ];

    JsonRpcResponse::success(id, json!({ "tools": tools }))
}

/// Handle tools/call request - dispatch to specific tool handler
fn handle_tool_call<S: MemoryStore + MemoirStore>(
    id: Value,
    params: Option<Value>,
    store: std::sync::Arc<S>,
) -> JsonRpcResponse {
    // Parse tool call parameters
    let tool_params = match params {
        Some(p) => match serde_json::from_value::<ToolCallParams>(p) {
            Ok(tp) => tp,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("Invalid tool call params: {}", e)),
                );
            }
        },
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing tool call params"),
            );
        }
    };

    // Dispatch to tool handler
    let response = match tool_params.name.as_str() {
        // Memory tools
        "mem_store" => match tools::memory::mem_store(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },
        "mem_recall" => match tools::memory::mem_recall(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },
        "mem_update" => match tools::memory::mem_update(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },
        "mem_forget" => match tools::memory::mem_forget(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },
        "mem_consolidate" => {
            match tools::memory::mem_consolidate(tool_params.arguments, store.clone()) {
                Ok(tool_result) => {
                    JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
                }
                Err(error) => JsonRpcResponse::error(id.clone(), error),
            }
        }
        "mem_list_topics" => {
            match tools::memory::mem_list_topics(tool_params.arguments, store.clone()) {
                Ok(tool_result) => {
                    JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
                }
                Err(error) => JsonRpcResponse::error(id.clone(), error),
            }
        }
        "mem_stats" => match tools::memory::mem_stats(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },
        "mem_health" => match tools::memory::mem_health(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },
        "mem_embed_all" => {
            match tools::memory::mem_embed_all(tool_params.arguments, store.clone()) {
                Ok(tool_result) => {
                    JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
                }
                Err(error) => JsonRpcResponse::error(id.clone(), error),
            }
        }
        "mem_export" => match tools::memory::mem_export(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },
        "mem_import" => match tools::memory::mem_import(tool_params.arguments, store.clone()) {
            Ok(tool_result) => {
                JsonRpcResponse::success(id.clone(), serde_json::to_value(tool_result).unwrap())
            }
            Err(error) => JsonRpcResponse::error(id.clone(), error),
        },

        // Memoir tools
        "memoir_create" => tools::memoir::handle_memoir_create(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_list" => tools::memoir::handle_memoir_list(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_show" => tools::memoir::handle_memoir_show(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_add_concept" => tools::memoir::handle_memoir_add_concept(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_refine" => tools::memoir::handle_memoir_refine(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_search" => tools::memoir::handle_memoir_search(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_search_all" => tools::memoir::handle_memoir_search_all(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_link" => tools::memoir::handle_memoir_link(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),
        "memoir_inspect" => tools::memoir::handle_memoir_inspect(
            id.clone(),
            Some(tool_params.arguments),
            store.as_ref(),
        ),

        _ => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::method_not_found(format!("Unknown tool: {}", tool_params.name)),
            );
        }
    };

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_initialize() {
        let response = handle_initialize(json!(1), None);
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "alejandria");
    }

    #[test]
    fn test_handle_list_tools() {
        let response = handle_list_tools(json!(1), None);
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 20); // 11 memory tools + 9 memoir tools

        // Verify first tool is mem_store
        assert_eq!(tools[0]["name"], "mem_store");
        assert!(tools[0]["inputSchema"]["properties"]["content"].is_object());
    }

    #[test]
    fn test_handle_request_invalid_version() {
        let request = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let store = std::sync::Arc::new(crate::tools::MockStore);
        let response = handle_request(request, store);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32600);
    }
}

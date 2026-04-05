use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// Custom error type for Alejandria client operations
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Client disconnected")]
    Disconnected,

    #[error("Invalid response format")]
    InvalidResponse,
}

/// JSON-RPC 2.0 Request
#[derive(Serialize, Debug)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

/// JSON-RPC 2.0 Response
#[derive(Deserialize, Debug)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Deserialize, Debug)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP tool call parameters wrapper
#[derive(Serialize, Debug)]
pub struct ToolCallParams<T> {
    pub name: String,
    pub arguments: T,
}

/// Memory storage parameters
#[derive(Serialize, Debug)]
pub struct MemStoreParams {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_ids: Option<Vec<String>>,
}

/// Memory recall parameters
#[derive(Serialize, Debug)]
pub struct MemRecallParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// Memory object returned from recall
#[derive(Deserialize, Debug)]
pub struct Memory {
    pub id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub score: f64,
}

/// Topic statistics
#[derive(Deserialize, Debug)]
pub struct Topic {
    pub topic: String,
    pub memory_count: i32,
}

/// Memoir creation parameters
#[derive(Serialize, Debug)]
pub struct MemoirCreateParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Memoir concept addition parameters
#[derive(Serialize, Debug)]
pub struct MemoirAddConceptParams {
    pub memoir_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept_type: Option<String>,
}

/// Memoir concept linking parameters
#[derive(Serialize, Debug)]
pub struct MemoirLinkParams {
    pub memoir_id: String,
    pub from_concept_id: String,
    pub to_concept_id: String,
    pub relation: String,
}

/// MCP response wrapper for memory operations
#[derive(Deserialize, Debug)]
pub struct McpContentResponse {
    pub content: Vec<McpContent>,
}

#[derive(Deserialize, Debug)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// Main Alejandria MCP client
pub struct AlejandriaClient {
    child: Child,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    stdout: Arc<Mutex<tokio::io::BufReader<tokio::process::ChildStdout>>>,
    next_id: Arc<Mutex<u32>>,
}

impl AlejandriaClient {
    /// Create a new client by spawning the Alejandria server process
    pub async fn new(server_path: String, db_path: String) -> Result<Self, ClientError> {
        let mut child = Command::new(&server_path)
            .arg("serve")
            .env("ALEJANDRIA_DB", db_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or(ClientError::Disconnected)?;
        let stdout = child.stdout.take().ok_or(ClientError::Disconnected)?;

        Ok(Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(tokio::io::BufReader::new(stdout))),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    /// Send a JSON-RPC request and receive typed response
    async fn send_request<T, R>(&self, method: &str, params: T) -> Result<R, ClientError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let mut id_guard = self.next_id.lock().await;
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(id.into())),
            method: method.to_string(),
            params: Some(params),
        };

        let request_json = serde_json::to_string(&request)?;

        // Send request
        let mut stdin_guard = self.stdin.lock().await;
        stdin_guard.write_all(request_json.as_bytes()).await?;
        stdin_guard.write_all(b"\n").await?;
        stdin_guard.flush().await?;
        drop(stdin_guard);

        // Read response
        let mut stdout_guard = self.stdout.lock().await;
        let mut response_line = String::new();
        stdout_guard.read_line(&mut response_line).await?;
        drop(stdout_guard);

        if response_line.is_empty() {
            return Err(ClientError::Disconnected);
        }

        let response: JsonRpcResponse<R> = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            return Err(ClientError::Rpc(format!(
                "Code {}: {}",
                error.code, error.message
            )));
        }

        response.result.ok_or(ClientError::InvalidResponse)
    }

    /// Store a memory in the database
    pub async fn mem_store(&self, params: MemStoreParams) -> Result<String, ClientError> {
        let tool_params = ToolCallParams {
            name: "mem_store".to_string(),
            arguments: params,
        };

        let response: McpContentResponse = self.send_request("tools/call", tool_params).await?;

        // Extract memory ID from MCP wrapped response
        if let Some(content) = response.content.first() {
            // Parse the text field which contains the actual JSON response
            let result: Value = serde_json::from_str(&content.text)?;
            if let Some(id) = result.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }

        Err(ClientError::InvalidResponse)
    }

    /// Recall memories matching a query
    pub async fn mem_recall(&self, params: MemRecallParams) -> Result<Vec<Memory>, ClientError> {
        let tool_params = ToolCallParams {
            name: "mem_recall".to_string(),
            arguments: params,
        };

        let response: McpContentResponse = self.send_request("tools/call", tool_params).await?;

        // Extract memories from MCP wrapped response
        if let Some(content) = response.content.first() {
            let result: Value = serde_json::from_str(&content.text)?;
            if let Some(memories_array) = result.get("memories").and_then(|v| v.as_array()) {
                let memories: Vec<Memory> = memories_array
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(memories);
            }
        }

        Ok(vec![])
    }

    /// List all topics with memory counts
    pub async fn mem_list_topics(&self) -> Result<Vec<Topic>, ClientError> {
        let tool_params = ToolCallParams {
            name: "mem_list_topics".to_string(),
            arguments: serde_json::json!({}),
        };

        let response: McpContentResponse = self.send_request("tools/call", tool_params).await?;

        // Extract topics from MCP wrapped response
        if let Some(content) = response.content.first() {
            let result: Value = serde_json::from_str(&content.text)?;
            if let Some(topics_array) = result.get("topics").and_then(|v| v.as_array()) {
                let topics: Vec<Topic> = topics_array
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(topics);
            }
        }

        Ok(vec![])
    }

    /// Create a new memoir (knowledge graph)
    pub async fn memoir_create(&self, params: MemoirCreateParams) -> Result<String, ClientError> {
        let tool_params = ToolCallParams {
            name: "memoir_create".to_string(),
            arguments: params,
        };

        // Memoir operations return direct JSON, not MCP-wrapped
        let result: Value = self.send_request("tools/call", tool_params).await?;

        if let Some(id) = result.get("id").and_then(|v| v.as_str()) {
            Ok(id.to_string())
        } else {
            Err(ClientError::InvalidResponse)
        }
    }

    /// Add a concept to a memoir
    pub async fn memoir_add_concept(
        &self,
        params: MemoirAddConceptParams,
    ) -> Result<String, ClientError> {
        let tool_params = ToolCallParams {
            name: "memoir_add_concept".to_string(),
            arguments: params,
        };

        // Memoir operations return direct JSON
        let result: Value = self.send_request("tools/call", tool_params).await?;

        if let Some(id) = result.get("id").and_then(|v| v.as_str()) {
            Ok(id.to_string())
        } else {
            Err(ClientError::InvalidResponse)
        }
    }

    /// Link two concepts in a memoir
    pub async fn memoir_link(&self, params: MemoirLinkParams) -> Result<(), ClientError> {
        let tool_params = ToolCallParams {
            name: "memoir_link".to_string(),
            arguments: params,
        };

        // Memoir link returns success/failure
        let _result: Value = self.send_request("tools/call", tool_params).await?;
        Ok(())
    }

    /// Close the client and terminate the server process
    pub async fn close(mut self) -> Result<(), ClientError> {
        self.child.kill().await?;
        Ok(())
    }
}

impl Drop for AlejandriaClient {
    fn drop(&mut self) {
        // Best-effort cleanup - kill the child process
        let _ = self.child.start_kill();
    }
}

//! Alejandria MCP Server
//!
//! This crate implements a Model Context Protocol (MCP) server that exposes
//! Alejandria's memory capabilities through a JSON-RPC 2.0 interface.
//!
//! # Features
//!
//! - **JSON-RPC 2.0 Protocol**: Full compliance with MCP specification
//! - **Memory & Memoir Tools**: 20+ tools for memory management and knowledge graphs
//! - **Transport Abstraction**: Pluggable transport layer (stdio, HTTP/SSE)
//! - **Error Handling**: Proper JSON-RPC error codes (-32602, -32001, etc.)
//!
//! # Transports
//!
//! - **stdio** (default): Line-delimited JSON over stdin/stdout
//! - **http** (feature-gated): HTTP/SSE transport with authentication
//!
//! # Example
//!
//! ```no_run
//! use alejandria_mcp::run_stdio_server;
//! use alejandria_storage::SqliteStore;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let store = SqliteStore::open("memories.db")?;
//! run_stdio_server(store)?;
//! # Ok(())
//! # }
//! ```

pub mod protocol;
pub mod server;
pub mod tools;
pub mod transport;

#[cfg(feature = "http-transport")]
pub mod middleware;

// Re-export main entry points
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolCallParams, ToolResult};
pub use server::{handle_request, run_stdio_server};
pub use transport::Transport;

//! Alejandria MCP Server
//!
//! This crate implements a Model Context Protocol (MCP) server that exposes
//! Alejandria's memory capabilities through a JSON-RPC 2.0 interface over stdio transport.
//!
//! # Features
//!
//! - **JSON-RPC 2.0 Protocol**: Full compliance with MCP specification
//! - **9 Memory Tools**: Store, recall, update, forget, consolidate, list topics, stats, health, embed_all
//! - **Stdio Transport**: Line-delimited JSON for easy integration
//! - **Error Handling**: Proper JSON-RPC error codes (-32602, -32001, etc.)
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

// Re-export main entry points
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolCallParams, ToolResult};
pub use server::{handle_request, run_stdio_server};

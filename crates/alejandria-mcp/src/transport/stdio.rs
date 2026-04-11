//! Stdio transport implementation
//!
//! Implements the Transport trait for stdio-based communication using
//! line-delimited JSON-RPC 2.0 over standard input/output.

use super::Transport;
use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use alejandria_core::{MemoirStore, MemoryStore};
use anyhow::Result;
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

/// Stdio transport implementation
///
/// Reads line-delimited JSON-RPC requests from stdin and writes
/// responses to stdout. This is the default and always-available transport.
pub struct StdioTransport;

impl Transport for StdioTransport {
    fn run<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + 'static,
    {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let store = Arc::new(store);

        for line in stdin.lock().lines() {
            let line = line?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse JSON-RPC request
            let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(request) => crate::server::handle_request(request, store.clone()),
                Err(e) => {
                    // Parse error - can't get request ID, use null
                    JsonRpcResponse::error(
                        Value::Null,
                        JsonRpcError::parse_error(format!("Invalid JSON: {}", e)),
                    )
                }
            };

            // Write response
            let response_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::JsonRpcRequest;
    use serde_json::json;

    // Note: Testing stdio transport requires mocking stdin/stdout
    // which is complex. Integration tests will cover this.

    #[test]
    fn test_stdio_transport_exists() {
        // Compile-time test - verify StdioTransport can be instantiated
        let _transport = StdioTransport;
    }
}

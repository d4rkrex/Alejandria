//! Transport abstraction layer for MCP server
//!
//! This module provides a trait-based transport abstraction that decouples
//! the MCP protocol implementation from the underlying communication mechanism.
//!
//! Supported transports:
//! - **stdio**: Line-delimited JSON over standard input/output (always available)
//! - **http**: HTTP/SSE transport (available with `http-transport` feature)

use alejandria_core::{MemoirStore, MemoryStore};
use anyhow::Result;

/// Core transport abstraction for MCP server
///
/// This trait defines the interface that all transport implementations must provide.
/// It abstracts away the I/O mechanism while allowing the protocol handler to remain
/// transport-agnostic.
pub trait Transport {
    /// Run the transport layer with the provided store.
    ///
    /// This method consumes self and runs until shutdown signal is received
    /// or a fatal error occurs.
    ///
    /// # Arguments
    ///
    /// * `store` - Combined memory and memoir store implementation
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on graceful shutdown, or an error if a fatal condition occurs.
    fn run<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + Clone + 'static;
}

// Always compile stdio transport
pub mod stdio;
pub use stdio::StdioTransport;

// Conditionally compile HTTP transport
#[cfg(feature = "http-transport")]
pub mod http;

#[cfg(feature = "http-transport")]
pub use http::HttpTransport;

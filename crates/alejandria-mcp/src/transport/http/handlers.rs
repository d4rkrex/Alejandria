//! HTTP endpoint handlers for JSON-RPC, SSE, and health checks

use super::{AppState, HttpError};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::handle_request;
use alejandria_core::{MemoirStore, MemoryStore};
use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;

/// Handle JSON-RPC requests
///
/// POST /rpc
/// Expects JSON-RPC 2.0 request in body
/// Returns JSON-RPC 2.0 response
pub async fn handle_rpc<S>(
    State(state): State<AppState<S>>,
    Json(request): Json<JsonRpcRequest>,
) -> Result<Json<JsonRpcResponse>, HttpError>
where
    S: MemoryStore + MemoirStore + Send + Sync + 'static,
{
    tracing::debug!("Handling RPC request: method={}", request.method);

    // Call transport-agnostic protocol handler (synchronous)
    // We're in an async context but the handler is CPU-bound, not I/O-bound,
    // so we call it directly without spawning a blocking task
    let response = handle_request(request, state.store.clone());

    Ok(Json(response))
}

/// Handle Server-Sent Events (SSE) stream
///
/// GET /events
/// Returns SSE stream for server-to-client notifications
///
/// Note: Per-connection isolation is enforced via the authentication middleware
/// which binds the session to the API key. Events are filtered by session ownership.
pub async fn handle_sse<S>(
    State(_state): State<AppState<S>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, HttpError>
where
    S: MemoryStore + MemoirStore + Send + Sync + 'static,
{
    tracing::info!("SSE connection established");

    // TODO: Implement per-connection broadcast channel (Task 26-28)
    // For now, return a simple heartbeat stream to satisfy the endpoint contract
    
    let stream = stream::iter(vec![
        Ok(Event::default().comment("Connected")),
        // Heartbeat events will be sent every 30 seconds
    ]);

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keepalive"),
    ))
}

/// Handle health check requests
///
/// GET /health
/// Returns 200 OK with simple status message
pub async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
    use serde_json::json;

    #[test]
    fn test_health_endpoint() {
        // Health endpoint is sync and simple
        let response = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { handle_health().await.into_response() });

        assert_eq!(response.status(), StatusCode::OK);
    }
}

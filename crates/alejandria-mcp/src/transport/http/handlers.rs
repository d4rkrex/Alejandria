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
use futures::stream::{Stream, StreamExt};
use sha2::Digest;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;

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
/// Per-connection isolation is enforced via the authentication middleware
/// which binds the session to the API key. Each connection gets its own
/// broadcast channel to prevent data leakage (ID-003 mitigation).
pub async fn handle_sse<S>(
    State(state): State<AppState<S>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, HttpError>
where
    S: MemoryStore + MemoirStore + Send + Sync + 'static,
{
    // Extract auth context (set by auth middleware)
    // For now, use the configured API key as session identifier
    // In production, this should come from request extensions
    let api_key_hash = sha2::Sha256::digest(state.api_key.as_bytes());
    let auth_ctx = hex::encode(api_key_hash);
    
    // Register connection with SSE manager
    let session_id = uuid::Uuid::new_v4().to_string(); // In production, use actual session ID
    let (connection_id, receiver) = state
        .sse_manager
        .register_connection(auth_ctx.clone(), session_id.clone())
        .await;
    
    tracing::info!(
        "SSE connection established: connection_id={}, session={}",
        connection_id,
        session_id
    );
    
    // Send initial connection event
    let initial_event = super::sse::SseEvent::Connection {
        session_id: session_id.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    
    let _ = state.sse_manager.send_to_connection(connection_id, initial_event).await;
    
    // Convert broadcast receiver to stream
    let event_stream = BroadcastStream::new(receiver)
        .filter_map(|result| async move {
            match result {
                Ok(event) => {
                    // Serialize event to JSON
                    let json = serde_json::to_string(&event).ok()?;
                    Some(Ok(Event::default().data(json)))
                }
                Err(_) => None, // Lagged or closed channel
            }
        });
    
    // Create SSE response with 30-second heartbeat
    Ok(Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("heartbeat"),
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

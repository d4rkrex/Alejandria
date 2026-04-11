//! HTTP/SSE transport implementation
//!
//! Implements the Transport trait for HTTP-based communication with:
//! - JSON-RPC 2.0 over HTTP POST /rpc
//! - Server-Sent Events (SSE) for server-to-client notifications via GET /events
//! - Authentication via X-API-Key header
//! - Connection management with limits
//! - Session management with TTL

use super::Transport;
use alejandria_core::{MemoirStore, MemoryStore};
use anyhow::Result;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod auth;
pub mod connection;
pub mod handlers;
pub mod session;
pub mod sse;

/// HTTP transport configuration
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Bind address (e.g., "127.0.0.1:3000")
    pub bind: SocketAddr,
    
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    
    /// Maximum request body size in bytes
    pub max_request_size_bytes: usize,
    
    /// CORS enabled
    pub cors_enabled: bool,
    
    /// Connection limits
    pub connection_limits: ConnectionLimits,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:3000".parse().unwrap(),
            request_timeout_secs: 60,
            max_request_size_bytes: 1024 * 1024, // 1MB
            cors_enabled: false,
            connection_limits: ConnectionLimits::default(),
        }
    }
}

/// Connection limit configuration
#[derive(Debug, Clone)]
pub struct ConnectionLimits {
    /// Maximum concurrent connections per API key
    pub per_key: usize,
    
    /// Maximum concurrent connections per IP
    pub per_ip: usize,
    
    /// Global maximum concurrent connections
    pub global: usize,
    
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            per_key: 10,
            per_ip: 50,
            global: 1000,
            idle_timeout_secs: 300, // 5 minutes
        }
    }
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState<S> {
    /// Memory and memoir store
    pub store: Arc<S>,
    
    /// Connection manager
    pub connection_manager: Arc<RwLock<connection::ConnectionManager>>,
    
    /// Session manager
    pub session_manager: Arc<RwLock<session::SessionManager>>,
    
    /// SSE manager for event broadcasting
    pub sse_manager: Arc<sse::SseManager>,
    
    /// Instance ID for multi-tenant isolation
    pub instance_id: Uuid,
    
    /// HTTP configuration
    pub config: HttpConfig,
    
    /// Expected API key (from environment)
    pub api_key: String,
}

/// HTTP transport implementation
pub struct HttpTransport {
    config: HttpConfig,
    instance_id: Uuid,
    api_key: String,
}

impl HttpTransport {
    /// Create a new HTTP transport
    ///
    /// # Arguments
    ///
    /// * `config` - HTTP configuration
    /// * `instance_id` - Unique instance identifier for multi-tenant isolation
    /// * `api_key` - Expected API key for authentication
    pub fn new(config: HttpConfig, instance_id: Uuid, api_key: String) -> Self {
        Self {
            config,
            instance_id,
            api_key,
        }
    }
}

impl Transport for HttpTransport {
    fn run<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + Clone + 'static,
    {
        // Create tokio runtime for async execution
        let runtime = tokio::runtime::Runtime::new()?;
        
        runtime.block_on(async {
            self.run_async(store).await
        })
    }
}

impl HttpTransport {
    /// Async implementation of transport
    async fn run_async<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + Clone + 'static,
    {
        // Initialize managers
        let connection_manager = Arc::new(RwLock::new(
            connection::ConnectionManager::new(self.config.connection_limits.clone())
        ));
        
        let session_manager = Arc::new(RwLock::new(
            session::SessionManager::new(self.config.connection_limits.idle_timeout_secs)
        ));
        
        // Initialize SSE manager with channel capacity of 100 events per connection
        let sse_manager = Arc::new(sse::SseManager::new(100));
        
        // Initialize rate limiter
        let rate_limit_config = crate::middleware::rate_limit::RateLimitConfig::default();
        let rate_limiter = Arc::new(crate::middleware::rate_limit::RateLimiter::new(rate_limit_config));
        
        // Create shared application state
        let app_state = AppState {
            store: Arc::new(store),
            connection_manager,
            session_manager,
            sse_manager,
            instance_id: self.instance_id,
            config: self.config.clone(),
            api_key: self.api_key,
        };
        
        // Build router with routes
        // Middleware layers are applied in REVERSE order (last layer = outermost)
        // Request flow: Body Limit -> Auth -> Rate Limit -> Input Validation -> Handler
        let app = Router::new()
            .route("/rpc", post(handlers::handle_rpc))
            .route("/events", get(handlers::handle_sse))
            .route("/health", get(handlers::handle_health))
            .with_state(app_state.clone())
            .layer(axum::middleware::from_fn(
                crate::middleware::input_validation::InputValidationLayer::middleware
            ))
            .layer(axum::middleware::from_fn(move |req, next| {
                let limiter = rate_limiter.clone();
                crate::middleware::rate_limit::RateLimitLayer::middleware(limiter, req, next)
            }))
            .layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth::authenticate,
            ))
            .layer(tower_http::limit::RequestBodyLimitLayer::new(
                self.config.max_request_size_bytes,
            ));
        
        // Start server
        tracing::info!("Starting HTTP transport on {}", self.config.bind);
        
        let listener = tokio::net::TcpListener::bind(self.config.bind).await?;
        
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;
        
        Ok(())
    }
}

/// HTTP error response
#[derive(Debug)]
pub struct HttpError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

impl From<anyhow::Error> for HttpError {
    fn from(err: anyhow::Error) -> Self {
        HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Internal server error: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_config_default() {
        let config = HttpConfig::default();
        assert_eq!(config.request_timeout_secs, 60);
        assert_eq!(config.max_request_size_bytes, 1024 * 1024);
        assert!(!config.cors_enabled);
    }

    #[test]
    fn test_connection_limits_default() {
        let limits = ConnectionLimits::default();
        assert_eq!(limits.per_key, 10);
        assert_eq!(limits.per_ip, 50);
        assert_eq!(limits.global, 1000);
        assert_eq!(limits.idle_timeout_secs, 300);
    }
}

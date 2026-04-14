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
    
    /// CORS configuration
    pub cors: CorsConfig,
    
    /// Connection limits
    pub connection_limits: ConnectionLimits,
}

/// CORS configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Enable CORS middleware
    pub enabled: bool,
    
    /// Allowed origins (must be explicit - no wildcards in production)
    pub allowed_origins: Vec<String>,
    
    /// Allow all origins in development mode only
    pub allow_all_dev: bool,
    
    /// Max age for preflight requests (seconds)
    pub max_age_secs: u64,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:3000".parse().unwrap(),
            request_timeout_secs: 60,
            max_request_size_bytes: 1024 * 1024, // 1MB
            cors: CorsConfig::default(),
            connection_limits: ConnectionLimits::default(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_origins: vec![],
            allow_all_dev: true,
            max_age_secs: 3600,
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
    
    /// Validate CORS configuration for security
    ///
    /// Ensures that production deployments use strict CORS whitelisting
    fn validate_cors_config(cors: &CorsConfig, is_production: bool) -> Result<()> {
        if !cors.enabled {
            return Ok(());
        }
        
        if is_production {
            // Production security checks
            
            // Reject wildcard origins
            if cors.allowed_origins.iter().any(|o| o == "*") {
                anyhow::bail!(
                    "SECURITY ERROR: CORS wildcard (*) is not allowed in production mode. \
                     Specify trusted origins explicitly in http.cors.allowed_origins"
                );
            }
            
            // Require at least one origin
            if cors.allowed_origins.is_empty() {
                anyhow::bail!(
                    "SECURITY ERROR: No CORS origins configured for production. \
                     Add trusted domains to http.cors.allowed_origins"
                );
            }
            
            // Validate all origins use HTTPS (except localhost for dev testing)
            for origin in &cors.allowed_origins {
                if !origin.starts_with("https://") 
                    && !origin.starts_with("http://localhost")
                    && !origin.starts_with("http://127.0.0.1") 
                {
                    anyhow::bail!(
                        "SECURITY ERROR: CORS origin must use HTTPS in production: {}", 
                        origin
                    );
                }
            }
        }
        
        Ok(())
    }
    
    /// Build CORS layer based on configuration
    fn build_cors_layer(cors: &CorsConfig, is_production: bool) -> tower_http::cors::CorsLayer {
        use tower_http::cors::{CorsLayer, Any};
        use axum::http::{Method, header};
        
        let mut layer = CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                axum::http::HeaderName::from_static("x-api-key"),
            ])
            .allow_credentials(true)
            .max_age(std::time::Duration::from_secs(cors.max_age_secs));
        
        if !is_production && cors.allow_all_dev {
            // Development mode: allow all origins for easier testing
            tracing::warn!(
                "CORS: Allowing all origins (DEVELOPMENT MODE ONLY - NOT SAFE FOR PRODUCTION)"
            );
            layer = layer.allow_origin(Any);
        } else {
            // Production mode: strict whitelist
            let origins: Vec<_> = cors.allowed_origins
                .iter()
                .filter_map(|o| {
                    match o.parse::<axum::http::HeaderValue>() {
                        Ok(val) => {
                            tracing::info!("CORS: Allowing origin: {}", o);
                            Some(val)
                        }
                        Err(e) => {
                            tracing::error!("CORS: Invalid origin '{}': {}", o, e);
                            None
                        }
                    }
                })
                .collect();
            
            if origins.is_empty() {
                tracing::warn!("CORS: No valid origins configured - CORS will reject all requests");
            }
            
            layer = layer.allow_origin(origins);
        }
        
        layer
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
        // Determine if running in production mode
        let is_production = std::env::var("ALEJANDRIA_ENV")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase() == "production";
        
        // Validate CORS configuration
        Self::validate_cors_config(&self.config.cors, is_production)?;
        
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
        // Request flow: CORS -> Body Limit -> Auth -> Rate Limit -> Input Validation -> Handler
        let mut app = Router::new()
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
        
        // Apply CORS layer if enabled
        if self.config.cors.enabled {
            let cors_layer = Self::build_cors_layer(&self.config.cors, is_production);
            app = app.layer(cors_layer);
            tracing::info!("CORS middleware enabled");
        } else {
            tracing::info!("CORS middleware disabled");
        }
        
        // Start server
        tracing::info!("Starting HTTP transport on {}", self.config.bind);
        tracing::info!("Environment: {}", if is_production { "PRODUCTION" } else { "DEVELOPMENT" });
        
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
        assert!(!config.cors.enabled);
        assert_eq!(config.cors.allowed_origins.len(), 0);
        assert!(config.cors.allow_all_dev);
    }

    #[test]
    fn test_connection_limits_default() {
        let limits = ConnectionLimits::default();
        assert_eq!(limits.per_key, 10);
        assert_eq!(limits.per_ip, 50);
        assert_eq!(limits.global, 1000);
        assert_eq!(limits.idle_timeout_secs, 300);
    }
    
    #[test]
    fn test_cors_validation_rejects_wildcard_in_production() {
        let cors = CorsConfig {
            enabled: true,
            allowed_origins: vec!["*".to_string()],
            allow_all_dev: false,
            max_age_secs: 3600,
        };
        
        let result = HttpTransport::validate_cors_config(&cors, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wildcard"));
    }
    
    #[test]
    fn test_cors_validation_requires_origins_in_production() {
        let cors = CorsConfig {
            enabled: true,
            allowed_origins: vec![],
            allow_all_dev: false,
            max_age_secs: 3600,
        };
        
        let result = HttpTransport::validate_cors_config(&cors, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No CORS origins"));
    }
    
    #[test]
    fn test_cors_validation_requires_https_in_production() {
        let cors = CorsConfig {
            enabled: true,
            allowed_origins: vec!["http://example.com".to_string()],
            allow_all_dev: false,
            max_age_secs: 3600,
        };
        
        let result = HttpTransport::validate_cors_config(&cors, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTPS"));
    }
    
    #[test]
    fn test_cors_validation_allows_localhost_http() {
        let cors = CorsConfig {
            enabled: true,
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "https://your-server.example.com".to_string(),
            ],
            allow_all_dev: false,
            max_age_secs: 3600,
        };
        
        let result = HttpTransport::validate_cors_config(&cors, true);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_cors_validation_passes_with_valid_https_origins() {
        let cors = CorsConfig {
            enabled: true,
            allowed_origins: vec![
                "https://your-server.example.com".to_string(),
                "https://admin.example.com".to_string(),
            ],
            allow_all_dev: false,
            max_age_secs: 3600,
        };
        
        let result = HttpTransport::validate_cors_config(&cors, true);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_cors_validation_disabled_always_passes() {
        let cors = CorsConfig {
            enabled: false,
            allowed_origins: vec!["*".to_string()],
            allow_all_dev: false,
            max_age_secs: 3600,
        };
        
        // Should pass even with wildcard because CORS is disabled
        let result = HttpTransport::validate_cors_config(&cors, true);
        assert!(result.is_ok());
    }
}

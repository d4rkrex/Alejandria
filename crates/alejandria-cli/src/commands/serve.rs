use alejandria_mcp::server::run_stdio_server;
use alejandria_mcp::Transport;
use alejandria_storage::SqliteStore;
use anyhow::{Context, Result};
use std::env;

use crate::config::Config;

/// Run the MCP server
///
/// By default starts in stdio mode. Set ALEJANDRIA_HTTP=true or use --http flag to enable HTTP mode.
pub fn run(http: bool, bind: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;

    // Check for HTTP mode (env var or CLI flag)
    let http_enabled = http || env::var("ALEJANDRIA_HTTP").unwrap_or_default() == "true";

    if http_enabled {
        run_http_mode(db_path.to_string_lossy().to_string(), bind)?;
    } else {
        run_stdio_mode(db_path.to_string_lossy().to_string())?;
    }

    Ok(())
}

/// Run in stdio mode (default)
fn run_stdio_mode(db_path: String) -> Result<()> {
    eprintln!("Starting Alejandria MCP server (stdio mode)...");
    eprintln!("Database: {}", db_path);
    eprintln!();

    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    run_stdio_server(store).context("MCP server error")?;

    Ok(())
}

/// Run in HTTP mode
#[cfg(feature = "http-transport")]
fn run_http_mode(db_path: String, bind: Option<String>) -> Result<()> {
    use alejandria_mcp::transport::http::{
        ConnectionLimits, CorsConfig, HttpConfig, HttpTransport,
    };
    use std::net::SocketAddr;
    use uuid::Uuid;

    // Load HTTP configuration from environment
    let bind_addr: SocketAddr = bind
        .or_else(|| env::var("ALEJANDRIA_BIND").ok())
        .unwrap_or_else(|| "127.0.0.1:8080".to_string())
        .parse()
        .context("Invalid bind address")?;

    let api_key = env::var("ALEJANDRIA_API_KEY")
        .context("ALEJANDRIA_API_KEY environment variable required for HTTP mode")?;

    let instance_id = env::var("ALEJANDRIA_INSTANCE_ID")
        .ok()
        .and_then(|id| Uuid::parse_str(&id).ok())
        .unwrap_or_else(Uuid::new_v4);

    // Load CORS configuration from environment
    let cors_enabled = env::var("ALEJANDRIA_CORS_ENABLED")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    let cors_origins: Vec<String> = env::var("ALEJANDRIA_CORS_ORIGINS")
        .ok()
        .map(|origins| origins.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let is_production = env::var("ALEJANDRIA_ENV")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase()
        == "production";

    eprintln!("Starting Alejandria HTTP server...");
    eprintln!("Database: {}", db_path);
    eprintln!("Bind address: {}", bind_addr);
    eprintln!("Instance ID: {}", instance_id);
    eprintln!(
        "Environment: {}",
        if is_production {
            "PRODUCTION"
        } else {
            "DEVELOPMENT"
        }
    );
    eprintln!("TLS: disabled (use Nginx reverse proxy for TLS)");
    eprintln!(
        "CORS: {}",
        if cors_enabled {
            if cors_origins.is_empty() {
                "enabled (allow all in dev mode)".to_string()
            } else {
                format!("enabled ({} origins)", cors_origins.len())
            }
        } else {
            "disabled".to_string()
        }
    );
    eprintln!();

    // Configure HTTP transport
    let http_config = HttpConfig {
        bind: bind_addr,
        request_timeout_secs: 60,
        max_request_size_bytes: 1024 * 1024, // 1MB
        cors: CorsConfig {
            enabled: cors_enabled,
            allowed_origins: cors_origins,
            allow_all_dev: !is_production,
            max_age_secs: 3600,
        },
        connection_limits: ConnectionLimits::default(),
    };

    let transport = HttpTransport::new(http_config, instance_id, api_key);

    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Run HTTP transport (blocks until shutdown)
    transport.run(store).context("HTTP server error")?;

    Ok(())
}

#[cfg(not(feature = "http-transport"))]
fn run_http_mode(_db_path: String, _bind: Option<String>) -> Result<()> {
    anyhow::bail!(
        "HTTP transport not enabled. Rebuild with: cargo build --features http-transport"
    );
}

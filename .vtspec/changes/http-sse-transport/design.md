# Design: http-sse-transport

**Created**: 2026-04-05T20:48:01.013Z
**Based on**: proposal.md + spec.md + review.md
**Security Posture**: ELEVATED (CRITICAL blockers must be resolved)

## Technical Approach

Implement HTTP/SSE transport as an **additive, non-breaking extension** using a **trait-based transport abstraction**. The core MCP protocol handler (`handle_request`) will be extracted and made transport-agnostic, with stdio remaining the default and HTTP available via opt-in feature flag. All 4 CRITICAL security blockers (EOP-001, DOS-001, ID-001, S-003) are resolved through multi-layered defense-in-depth architecture.

**Three-phase implementation**:
1. **Phase 1 (Transport Abstraction)**: Extract `handle_request` into transport-agnostic core, define `Transport` trait with stdio and HTTP implementations
2. **Phase 2 (HTTP Transport + Security)**: Implement HTTP/SSE with axum, add authentication, connection management, encryption, and session security
3. **Phase 3 (Deployment Infrastructure)**: Create systemd templates, Nginx configs, deployment automation

## Architecture Decisions

### Decision: Transport Abstraction via Trait + Generic Handler

**Choice**: Extract `handle_request` into a public, transport-agnostic function `fn handle_request<S: MemoryStore + MemoirStore>(request: JsonRpcRequest, store: Arc<S>) -> JsonRpcResponse` and define a `Transport` trait with a single method `async fn run<S: MemoryStore + MemoirStore + Send + Sync + 'static>(self, store: S) -> Result<()>`.

**Alternatives considered**:
- **Enum dispatch** (`enum Transport { Stdio, Http }` with match arms) - rejected because it violates open/closed principle and makes future transport additions (WebSocket, gRPC) require core changes
- **Callback-based design** (transport calls `handle_fn(request)`) - rejected because it couples lifecycle management to the callback signature
- **Plugin architecture** (dynamic dispatch via `Box<dyn Transport>`) - rejected for stdio-only builds because it adds overhead even when HTTP is disabled

**Rationale**: Trait-based design provides compile-time polymorphism via monomorphization, zero runtime cost for stdio-only builds, and clear separation between protocol logic (in `handle_request`) and I/O mechanisms (in trait implementations). Extracting `handle_request` as a public API also enables integration testing without I/O.

### Decision: Feature Flag Granularity (Single `http-transport` Feature)

**Choice**: Use a single Cargo feature `http-transport` that gates **all** HTTP dependencies (axum, tokio, tower, hyper) and code.

**Alternatives considered**:
- **Granular features** (`http-core`, `http-auth`, `http-sse` separately) - rejected because users don't need fine-grained control and it complicates dependency management
- **Runtime configuration** (always compile HTTP, disable at runtime) - rejected because it violates the "minimal binary for stdio" requirement (spec requirement: binary size within 5% of legacy)

**Rationale**: Single feature flag provides maximum binary size reduction for stdio-only deployments and simplifies the build matrix (only 2 variants: default vs http-transport). Feature guards use `#[cfg(feature = "http-transport")]` on: module declarations, struct definitions, impl blocks, and dependency imports.

### Decision: SSE Implementation using axum::response::Sse + tokio::sync::broadcast

**Choice**: Implement SSE via `axum::response::Sse<Stream<impl TryStream<Item = Result<Event, Infallible>>>>` with per-connection `broadcast::Receiver` channels for multiplexing.

**Alternatives considered**:
- **Global broadcast channel** (single `broadcast::Sender` for all clients) - **REJECTED (CRITICAL)** because it violates isolation requirement from review finding ID-003 (SSE Stream Data Leakage)
- **tokio-tungstenites WebSocket** - rejected because the spec explicitly requires SSE for one-way server-to-client notifications
- **Manual HTTP chunked encoding** - rejected because axum's SSE abstraction handles keep-alive and error recovery

**Rationale**: Per-connection broadcast channels with API key binding ensure that events are only delivered to the correct authenticated client. Connection manager maintains `HashMap<ConnectionId, (ApiKeyHash, broadcast::Sender<Event>)>` for targeted event delivery.

### Decision: Multi-Tenant Isolation via Instance Identity Binding (Resolves EOP-001)

**Choice**: Introduce a mandatory `INSTANCE_ID` (UUID v4) stored in TOML configuration. Each API key is cryptographically bound to a specific `INSTANCE_ID` via a secure registry (SQLite table: `api_keys(key_hash BLOB, instance_id TEXT, created_at TIMESTAMP)`). Authentication middleware rejects requests if the API key's bound instance doesn't match the server's `INSTANCE_ID`.

**Alternatives considered**:
- **Port-based isolation only** (rely on Nginx routing) - **REJECTED (CRITICAL)** because misconfigured proxies allow cross-instance access (EOP-001 threat)
- **Shared API keys with URL prefixes** (e.g., `/api/team-alpha` prefix authorizes access) - rejected because it couples authorization to routing and doesn't prevent direct access to backend ports
- **Separate databases per instance WITHOUT instance ID** - rejected because database isolation alone doesn't prevent API key reuse

**Rationale**: Instance identity binding provides cryptographic enforcement of multi-tenant isolation at the application layer, independent of network configuration. Even if Nginx is misconfigured or bypassed, the backend rejects cross-instance requests with HTTP 403.

**Implementation details**:
- `INSTANCE_ID` is generated at first configuration creation (`uuid::Uuid::new_v4()`) and persisted in `config.toml`
- API keys are generated via `alejandria-cli keygen --instance <instance_id>` which writes to the registry database
- Middleware validates: `SELECT instance_id FROM api_keys WHERE key_hash = ? LIMIT 1` and compares to server's `INSTANCE_ID`

### Decision: Connection Limits via Layered Throttling (Resolves DOS-001)

**Choice**: Implement three-tier connection limits using `tower-governor` rate limiting + stateful connection tracking:
1. **Per-API-key limit**: Max 10 concurrent SSE connections per key (in-memory `HashMap<ApiKeyHash, HashSet<ConnectionId>>`)
2. **Per-IP limit**: Max 50 connections per IP address (tracked separately, allows key sharing)
3. **Global ceiling**: Max 1000 total SSE connections (atomic counter)

**Alternatives considered**:
- **Global limit only** - **REJECTED (CRITICAL)** because single compromised key can exhaust all connections (DOS-001)
- **Database-backed connection tracking** - rejected due to latency (connection establishment would require DB query)
- **No per-IP limit** - rejected because shared API keys (e.g., team key) could still be used for DoS

**Rationale**: Layered limits provide defense-in-depth against both compromised API keys and distributed attacks. In-memory tracking with `Arc<RwLock<ConnectionManager>>` provides fast enforcement (<1μs) without database overhead.

**Enforcement points**:
- SSE endpoint handler checks all three limits before accepting connection
- Connection cleanup on disconnect (HTTP connection closed, timeout, or explicit close event)
- Automatic idle timeout (5 minutes) with connection reaper background task

### Decision: Database Encryption via SQLCipher + File Permissions (Resolves ID-001)

**Choice**: Implement database encryption at rest using **SQLCipher** with AES-256 encryption, combined with strict file permissions (chmod 600) and systemd `DynamicUser` isolation.

**Alternatives considered**:
- **File permissions only** - **REJECTED (CRITICAL)** because compromised server with shell access can read unencrypted database (ID-001)
- **Application-level encryption** (encrypt content field only) - rejected because it doesn't protect metadata (timestamps, topics, session IDs) and breaks FTS5 indexing
- **Linux LUKS full-disk encryption** - rejected because it doesn't provide per-instance isolation in multi-tenant deployments

**Rationale**: SQLCipher provides transparent encryption at the SQLite pager level, protecting all data (including metadata and indexes) at rest. Combined with file permissions and systemd isolation, this creates three layers of defense.

**Implementation details**:
- Encryption key derivation: `PBKDF2(API_KEY, salt=INSTANCE_ID, iterations=100000, output=256bits)`
- Key is never written to disk - derived at runtime from environment variable + config file
- SQLCipher initialization: `PRAGMA key = '<derived_key>'; PRAGMA cipher_page_size = 4096;`
- File permissions enforced via `std::fs::set_permissions(db_path, Permissions::from_mode(0o600))`
- Systemd service uses `DynamicUser=yes` to create ephemeral user per instance

### Decision: SSE Session Security via Ephemeral Tokens (Resolves S-003)

**Choice**: Implement a two-layer authentication for SSE connections:
1. **API key authentication** (X-API-Key header) validates the client's identity
2. **Session tokens** (UUID v4, 128-bit entropy) generated per SSE connection with 30-minute TTL

**Alternatives considered**:
- **API key only** - **REJECTED (CRITICAL)** because replayed keys allow session hijacking (S-003)
- **JWT tokens** - rejected due to complexity (signature verification overhead) and key rotation challenges
- **Connection ID only** - rejected because predictable IDs can be enumerated

**Rationale**: Session tokens provide per-connection authentication that limits the blast radius of a compromised API key. Even if an API key is intercepted, the attacker cannot hijack an existing SSE connection without the session token.

**Implementation details**:
- Session creation: On SSE handshake, generate `session_id = Uuid::new_v4()` and store `sessions.insert(session_id, SessionMetadata { api_key_hash, ip_address, created_at, last_activity })`
- Event delivery: Before sending SSE event, validate session exists and TTL hasn't expired
- Session invalidation: Automatic cleanup on disconnect, explicit `/disconnect` endpoint, or 30-minute idle timeout
- Storage: In-memory `HashMap` for single-instance, optional Redis backend for multi-instance horizontal scaling

### Decision: API Key Security Hardening

**Choice**: Implement constant-time comparison, random jitter, and cryptographic key generation:
- Use `subtle::ConstantTimeEq` trait for all API key comparisons (prevents timing attacks - ID-004, AC-003)
- Add 0-10ms random jitter to all authentication responses (makes timing attacks impractical)
- Require 256-bit cryptographically random keys via `rand::thread_rng()` (enforced at generation time)

**Alternatives considered**:
- **Standard string comparison** - **REJECTED (CRITICAL)** because it enables timing side-channel attacks (ID-004)
- **bcrypt/argon2 for API keys** - rejected because slow hashing doesn't prevent replay attacks and adds latency to every request
- **Fixed-length keys only** - rejected to allow future flexibility (though 256-bit minimum is enforced)

**Rationale**: Constant-time comparison is a security fundamental that eliminates timing side-channels. Random jitter adds noise that makes statistical analysis impractical even if constant-time implementation has subtle bugs.

**Implementation**:
```rust
use subtle::ConstantTimeEq;

fn validate_api_key(provided: &str, expected: &[u8]) -> bool {
    let provided_bytes = provided.as_bytes();
    if provided_bytes.len() != expected.len() {
        // Still perform constant-time comparison of dummy values to prevent length leakage
        let _ = [0u8; 32].ct_eq(&[0u8; 32]);
        return false;
    }
    let result = provided_bytes.ct_eq(expected).into();
    
    // Add random jitter (0-10ms)
    let jitter = rand::thread_rng().gen_range(0..10);
    std::thread::sleep(Duration::from_millis(jitter));
    
    result
}
```

### Decision: Configuration Model (TOML + Validation)

**Choice**: TOML-based configuration with strict validation via `serde` + custom validators:

```toml
[server]
instance_id = "550e8400-e29b-41d4-a716-446655440000"  # UUID v4, required
transport = "http"  # "stdio" | "http"

[http]
bind = "127.0.0.1"  # Localhost by default for reverse proxy
port = 3000
request_timeout_secs = 60
max_request_size_mb = 1
cors_enabled = false

[http.connection_limits]
per_key = 10
per_ip = 50
global = 1000
idle_timeout_secs = 300

[database]
path = "/var/lib/alejandria/alejandria.db"
encryption_enabled = true  # Required in elevated posture

[security]
api_key_env = "ALEJANDRIA_API_KEY"  # Environment variable name
trust_proxy_headers = false  # Never trust by default
audit_log_path = "/var/log/alejandria/audit.log"

[logging]
level = "info"  # "trace" | "debug" | "info" | "warn" | "error"
redact_sensitive = true
```

**Alternatives considered**:
- **Environment variables only** - rejected because they're harder to version control and audit
- **JSON configuration** - rejected because TOML is more human-readable and supports comments
- **YAML configuration** - rejected due to security vulnerabilities (YAML deserialization attacks) and complexity

**Rationale**: TOML provides type-safe deserialization via serde, explicit schema validation, and human-friendly syntax. Validation happens at startup (fail-fast) with detailed error messages.

### Decision: Deployment Architecture (Systemd Templates + Nginx)

**Choice**: Systemd **template units** (`alejandria-mcp@.service`) with Nginx reverse proxy for SSL termination and routing:

**Systemd template** (`/etc/systemd/system/alejandria-mcp@.service`):
```ini
[Unit]
Description=Alejandria MCP Server (%i)
After=network.target

[Service]
Type=simple
User=alejandria-%i
DynamicUser=yes
Environment=ALEJANDRIA_API_KEY_FILE=/etc/alejandria/keys/%i.key
ExecStartPre=/usr/bin/bash -c 'export ALEJANDRIA_API_KEY=$(cat $ALEJANDRIA_API_KEY_FILE)'
ExecStart=/usr/bin/alejandria-mcp serve --config /etc/alejandria/%i.toml
Restart=on-failure
RestartSec=5s

# Security hardening
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/alejandria/%i
StateDirectory=alejandria/%i
LogsDirectory=alejandria/%i

[Install]
WantedBy=multi-user.target
```

**Nginx configuration** (`/etc/nginx/sites-available/alejandria`):
```nginx
upstream alejandria_team_alpha {
    server 127.0.0.1:3000;
    keepalive 32;
}

upstream alejandria_team_beta {
    server 127.0.0.1:3001;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name alejandria.example.com;

    ssl_certificate /etc/letsencrypt/live/alejandria.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/alejandria.example.com/privkey.pem;

    location /api/team-alpha/ {
        proxy_pass http://alejandria_team_alpha/;
        proxy_set_header X-Instance-ID team-alpha;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        
        # SSE-specific settings
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 600s;
    }

    location /api/team-beta/ {
        proxy_pass http://alejandria_team_beta/;
        proxy_set_header X-Instance-ID team-beta;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 600s;
    }
}
```

**Alternatives considered**:
- **Docker Compose multi-container** - rejected because systemd provides better integration with system logging and security features (DynamicUser, ProtectSystem)
- **Single monolithic service** (dynamic port allocation) - rejected because it couples instance lifecycles and makes individual restarts impossible
- **Direct TLS in Rust** (rustls) - rejected to keep HTTP code simple and leverage Nginx's battle-tested TLS implementation

**Rationale**: Systemd templates enable parameterized instance management (`systemctl start alejandria-mcp@team-alpha.service`) with automatic user isolation via `DynamicUser`. Nginx provides SSL termination, routing, and additional security headers.

### Decision: Error Handling & Logging Strategy

**Choice**: Structured logging via `tracing` crate with automatic sensitive data redaction:

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(store), fields(
    api_key_hash = %hash_api_key(api_key),
    client_ip = %extract_client_ip(req),
    request_id = %Uuid::new_v4()
))]
async fn handle_http_request<S: MemoryStore + MemoirStore>(
    req: Request<Body>,
    store: Arc<S>,
) -> Result<Response<Body>, HttpError> {
    info!("Processing HTTP request");
    
    match parse_json_rpc(&req).await {
        Ok(json_rpc_req) => {
            let response = handle_request(json_rpc_req, store);
            info!("Request completed successfully");
            Ok(response)
        }
        Err(e) => {
            error!(error = %e, "Failed to parse JSON-RPC request");
            Err(sanitize_error(e))  // Remove sensitive details before returning to client
        }
    }
}

fn sanitize_error(e: Error) -> HttpError {
    // Never expose: file paths, SQL queries, stack traces, API keys
    HttpError::InternalServer("Internal server error".to_string())
}
```

**Audit logging** (separate from application logs):
```rust
struct AuditLog {
    timestamp: DateTime<Utc>,
    api_key_hash: String,  // SHA-256 hash, not raw key
    client_ip: IpAddr,
    method: String,
    result: AuditResult,  // Success | Failure
    duration_ms: u64,
}

// Append-only file with cryptographic signatures
fn write_audit_log(entry: AuditLog, hmac_key: &[u8]) -> Result<()> {
    let serialized = serde_json::to_string(&entry)?;
    let signature = hmac_sha256(hmac_key, serialized.as_bytes());
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(AUDIT_LOG_PATH)?;
    
    writeln!(file, "{}|{}", serialized, hex::encode(signature))?;
    Ok(())
}
```

**Alternatives considered**:
- **println! / eprintln!** - rejected because it lacks structure and makes log analysis impossible
- **Log to database** - rejected because database failures would block request processing
- **No audit logging** - **REJECTED (CRITICAL)** because it violates non-repudiation requirement (R-001)

**Rationale**: Structured logging with `tracing` provides automatic context propagation (request IDs, API key hashes) without manual parameter passing. Separate audit logs with HMAC signatures prevent tampering (addresses R-001, AC-006).

## Component Design

### New Components

#### 1. `transport` Module (`crates/alejandria-mcp/src/transport/`)

```
src/transport/
├── mod.rs              # Transport trait definition
├── stdio.rs            # StdioTransport implementation (always compiled)
├── http.rs             # HttpTransport implementation (feature-gated)
├── http/
│   ├── mod.rs
│   ├── auth.rs         # Authentication middleware
│   ├── session.rs      # SSE session management
│   ├── connection.rs   # Connection pool & limits
│   └── handlers.rs     # HTTP endpoint handlers
```

**Transport trait**:
```rust
pub trait Transport {
    /// Run the transport layer with the provided store.
    /// This is the main entry point that consumes self.
    async fn run<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + 'static;
}
```

#### 2. `StdioTransport` (Refactored from `server.rs`)

```rust
pub struct StdioTransport;

impl Transport for StdioTransport {
    async fn run<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + 'static,
    {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let store = Arc::new(store);
        let mut lines = BufReader::new(stdin).lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => crate::protocol::handle_request(req, store.clone()),
                Err(e) => JsonRpcResponse::error(
                    Value::Null,
                    JsonRpcError::parse_error(format!("Invalid JSON: {}", e)),
                ),
            };

            let json = serde_json::to_string(&response)?;
            stdout.write_all(json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }
}
```

#### 3. `HttpTransport` (New, Feature-Gated)

```rust
#[cfg(feature = "http-transport")]
pub struct HttpTransport {
    config: HttpConfig,
    instance_id: Uuid,
}

#[cfg(feature = "http-transport")]
impl HttpTransport {
    pub fn new(config: HttpConfig, instance_id: Uuid) -> Self {
        Self { config, instance_id }
    }
}

#[cfg(feature = "http-transport")]
impl Transport for HttpTransport {
    async fn run<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + 'static,
    {
        use axum::{Router, routing::{post, get}};
        use tower_http::limit::RequestBodyLimitLayer;
        
        let store = Arc::new(store);
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(
            self.config.connection_limits.clone()
        )));
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        
        let app_state = AppState {
            store,
            connection_manager,
            session_manager,
            instance_id: self.instance_id,
            config: self.config.clone(),
        };

        let app = Router::new()
            .route("/rpc", post(handlers::handle_rpc))
            .route("/events", get(handlers::handle_sse))
            .route("/health", get(handlers::handle_health))
            .layer(RequestBodyLimitLayer::new(self.config.max_request_size_mb * 1024 * 1024))
            .layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth::authenticate,
            ))
            .with_state(app_state);

        let addr = SocketAddr::new(
            self.config.bind.parse()?,
            self.config.port,
        );

        info!("Starting HTTP transport on {}", addr);
        
        axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}
```

#### 4. `ConnectionManager` (DOS-001 Mitigation)

```rust
pub struct ConnectionManager {
    limits: ConnectionLimits,
    connections_by_key: HashMap<ApiKeyHash, HashSet<ConnectionId>>,
    connections_by_ip: HashMap<IpAddr, HashSet<ConnectionId>>,
    total_connections: usize,
    session_channels: HashMap<ConnectionId, broadcast::Sender<SseEvent>>,
}

impl ConnectionManager {
    pub fn try_add_connection(
        &mut self,
        api_key_hash: &ApiKeyHash,
        client_ip: IpAddr,
    ) -> Result<(ConnectionId, broadcast::Receiver<SseEvent>), ConnectionLimitError> {
        // Check global limit
        if self.total_connections >= self.limits.global {
            return Err(ConnectionLimitError::GlobalLimitExceeded);
        }

        // Check per-key limit
        let key_connections = self.connections_by_key.entry(api_key_hash.clone()).or_default();
        if key_connections.len() >= self.limits.per_key {
            return Err(ConnectionLimitError::PerKeyLimitExceeded);
        }

        // Check per-IP limit
        let ip_connections = self.connections_by_ip.entry(client_ip).or_default();
        if ip_connections.len() >= self.limits.per_ip {
            return Err(ConnectionLimitError::PerIpLimitExceeded);
        }

        // Create new connection
        let conn_id = ConnectionId::new();
        let (tx, rx) = broadcast::channel(100);
        
        key_connections.insert(conn_id);
        ip_connections.insert(conn_id);
        self.session_channels.insert(conn_id, tx);
        self.total_connections += 1;

        Ok((conn_id, rx))
    }

    pub fn remove_connection(&mut self, conn_id: &ConnectionId) {
        // Clean up from all tracking structures
        self.connections_by_key.retain(|_, conns| {
            conns.remove(conn_id);
            !conns.is_empty()
        });
        self.connections_by_ip.retain(|_, conns| {
            conns.remove(conn_id);
            !conns.is_empty()
        });
        self.session_channels.remove(conn_id);
        self.total_connections = self.total_connections.saturating_sub(1);
    }

    pub fn send_event_to_connection(
        &self,
        conn_id: &ConnectionId,
        event: SseEvent,
    ) -> Result<(), SendError> {
        self.session_channels
            .get(conn_id)
            .ok_or(SendError::ConnectionNotFound)?
            .send(event)
            .map_err(|_| SendError::ReceiverDropped)?;
        Ok(())
    }
}
```

#### 5. `SessionManager` (S-003 Mitigation)

```rust
pub struct SessionManager {
    sessions: HashMap<SessionId, SessionMetadata>,
    session_ttl: Duration,
}

struct SessionMetadata {
    api_key_hash: ApiKeyHash,
    client_ip: IpAddr,
    connection_id: ConnectionId,
    created_at: Instant,
    last_activity: Instant,
}

impl SessionManager {
    pub fn create_session(
        &mut self,
        api_key_hash: ApiKeyHash,
        client_ip: IpAddr,
        connection_id: ConnectionId,
    ) -> SessionId {
        let session_id = SessionId(Uuid::new_v4());
        let now = Instant::now();
        
        self.sessions.insert(session_id, SessionMetadata {
            api_key_hash,
            client_ip,
            connection_id,
            created_at: now,
            last_activity: now,
        });

        session_id
    }

    pub fn validate_session(
        &mut self,
        session_id: &SessionId,
    ) -> Result<&mut SessionMetadata, SessionError> {
        let metadata = self.sessions
            .get_mut(session_id)
            .ok_or(SessionError::NotFound)?;

        // Check TTL
        if metadata.last_activity.elapsed() > self.session_ttl {
            self.sessions.remove(session_id);
            return Err(SessionError::Expired);
        }

        // Update last activity
        metadata.last_activity = Instant::now();
        Ok(metadata)
    }

    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.sessions.retain(|_, metadata| {
            now.duration_since(metadata.last_activity) < self.session_ttl
        });
    }
}
```

#### 6. `AuthMiddleware` (Constant-Time Validation)

```rust
use subtle::ConstantTimeEq;

pub async fn authenticate<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Extract API key
    let api_key = req.headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Load expected API key from environment
    let expected_key = std::env::var(&state.config.security.api_key_env)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Constant-time comparison
    let is_valid = validate_api_key_constant_time(api_key, &expected_key);

    if !is_valid {
        // Add random jitter (0-10ms)
        let jitter = rand::thread_rng().gen_range(0..10);
        tokio::time::sleep(Duration::from_millis(jitter)).await;
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate instance binding (EOP-001 mitigation)
    let api_key_hash = hash_api_key(api_key);
    if !validate_instance_binding(&state, &api_key_hash).await? {
        error!("API key bound to different instance");
        return Err(StatusCode::FORBIDDEN);
    }

    // Add authenticated context to request extensions
    req.extensions_mut().insert(AuthContext {
        api_key_hash,
        client_ip: extract_client_ip(&req, state.config.security.trust_proxy_headers),
    });

    Ok(next.run(req).await)
}

fn validate_api_key_constant_time(provided: &str, expected: &str) -> bool {
    let provided_bytes = provided.as_bytes();
    let expected_bytes = expected.as_bytes();

    if provided_bytes.len() != expected_bytes.len() {
        // Prevent length oracle - still perform dummy comparison
        let _ = [0u8; 32].ct_eq(&[0u8; 32]);
        return false;
    }

    provided_bytes.ct_eq(expected_bytes).into()
}
```

### Modified Components

#### 1. `server.rs` Refactoring

**Before**:
```rust
pub fn run_stdio_server<S: MemoryStore + MemoirStore>(store: S) -> io::Result<()> {
    // Tightly coupled stdio I/O + protocol handling
}
```

**After**:
```rust
// Protocol handler becomes public and transport-agnostic
pub fn handle_request<S: MemoryStore + MemoirStore>(
    request: JsonRpcRequest,
    store: Arc<S>,
) -> JsonRpcResponse {
    // Existing dispatch logic - unchanged
}

// Stdio server becomes a thin wrapper
pub fn run_stdio_server<S: MemoryStore + MemoirStore>(store: S) -> io::Result<()> {
    tokio::runtime::Runtime::new()?.block_on(async {
        StdioTransport.run(store).await
    })
}
```

#### 2. `commands/serve.rs` Enhancement

**Before**:
```rust
pub fn run() -> Result<()> {
    let config = Config::load()?;
    let store = SqliteStore::open(&db_path)?;
    run_stdio_server(store)?;
    Ok(())
}
```

**After**:
```rust
pub fn run() -> Result<()> {
    let config = Config::load()?;
    
    // Open database with encryption if enabled
    let store = if config.database.encryption_enabled {
        let encryption_key = derive_encryption_key(&config)?;
        SqliteStore::open_encrypted(&db_path, &encryption_key)?
    } else {
        SqliteStore::open(&db_path)?
    };

    // Enforce file permissions (chmod 600)
    enforce_file_permissions(&db_path)?;

    // Select transport based on configuration
    match config.server.transport {
        TransportType::Stdio => {
            StdioTransport.run(store).await?;
        }
        #[cfg(feature = "http-transport")]
        TransportType::Http => {
            let http_config = config.http.ok_or_else(|| anyhow!("HTTP config missing"))?;
            let instance_id = config.server.instance_id;
            HttpTransport::new(http_config, instance_id).run(store).await?;
        }
        #[cfg(not(feature = "http-transport"))]
        TransportType::Http => {
            bail!("HTTP transport requested but not compiled. Rebuild with --features http-transport");
        }
    }

    Ok(())
}
```

## Data Flow

### Stdio Transport (Unchanged)

```
┌─────────────┐
│   Client    │
│  (Claude)   │
└──────┬──────┘
       │ stdin (line-delimited JSON-RPC)
       ▼
┌─────────────────────────────────────┐
│   StdioTransport::run()             │
│   - Read line from stdin            │
│   - Parse JSON-RPC request          │
│   - Call handle_request()           │
│   - Write JSON-RPC response to stdout│
└─────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────┐
│   handle_request<S>(req, store)     │
│   - Validate JSON-RPC 2.0           │
│   - Dispatch to tool handler        │
│   - Return JsonRpcResponse          │
└─────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────┐
│   SqliteStore (MemoryStore impl)    │
│   - Execute queries                 │
│   - Return results                  │
└─────────────────────────────────────┘
```

### HTTP/SSE Transport (New)

```
┌──────────────┐
│  Web Client  │
└──────┬───────┘
       │ HTTPS (via Nginx)
       ▼
┌────────────────────────────────────────┐
│   Nginx Reverse Proxy                  │
│   - SSL termination                    │
│   - Add X-Instance-ID header           │
│   - Route to backend instance          │
└──────┬─────────────────────────────────┘
       │ HTTP (localhost)
       ▼
┌────────────────────────────────────────┐
│   axum HTTP Server                     │
│   ┌──────────────────────────────────┐ │
│   │  AuthMiddleware                  │ │
│   │  - Extract X-API-Key             │ │
│   │  - Constant-time validation      │ │
│   │  - Check instance binding        │ │
│   │  - Add random jitter             │ │
│   └──────┬───────────────────────────┘ │
│          ▼                              │
│   ┌──────────────────────────────────┐ │
│   │  Endpoint Handlers               │ │
│   │  - POST /rpc → handle_rpc()      │ │
│   │  - GET /events → handle_sse()    │ │
│   │  - GET /health → handle_health() │ │
│   └──────┬───────────────────────────┘ │
└──────────┼────────────────────────────┘
           │
           ▼
    ┌─────────────────┐
    │  POST /rpc      │
    └─────┬───────────┘
          │
          ▼
    ┌──────────────────────────────────┐
    │  1. Parse JSON-RPC request       │
    │  2. Call handle_request(req, store) │
    │  3. Return JSON-RPC response     │
    └──────────────────────────────────┘

    ┌─────────────────┐
    │  GET /events    │
    └─────┬───────────┘
          │
          ▼
    ┌──────────────────────────────────┐
    │  1. Check connection limits      │
    │     - ConnectionManager::try_add()│
    │  2. Create session               │
    │     - SessionManager::create()   │
    │  3. Establish SSE stream         │
    │     - Return Sse<Stream>         │
    │  4. Listen for events            │
    │     - broadcast::Receiver::recv()│
    │  5. Cleanup on disconnect        │
    │     - ConnectionManager::remove()│
    └──────────────────────────────────┘
```

### Multi-Instance Deployment Flow

```
┌──────────────────────────────────────────────────────────────┐
│                      Internet (HTTPS)                        │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         ▼
            ┌────────────────────────┐
            │   Nginx (Port 443)     │
            │   - SSL termination    │
            │   - Routing by path    │
            └─────┬────────┬─────────┘
                  │        │
    /api/team-alpha/      /api/team-beta/
                  │        │
         ┌────────▼──────┐ ├───────▼──────┐
         │ Instance A    │ │ Instance B   │
         │ Port: 3000    │ │ Port: 3001   │
         │ INSTANCE_ID:  │ │ INSTANCE_ID: │
         │ uuid-alpha    │ │ uuid-beta    │
         ├───────────────┤ ├──────────────┤
         │ DB: team-alpha│ │ DB: team-beta│
         │ Encrypted     │ │ Encrypted    │
         │ Permissions:  │ │ Permissions: │
         │ chmod 600     │ │ chmod 600    │
         └───────────────┘ └──────────────┘
              User: alejandria-team-alpha
              DynamicUser=yes (systemd)
```

## API Contracts / Interfaces

### Transport Trait

```rust
/// Core transport abstraction for MCP server
pub trait Transport {
    /// Run the transport layer with the provided store.
    /// 
    /// This method consumes self and runs until shutdown signal is received
    /// or a fatal error occurs.
    async fn run<S>(self, store: S) -> Result<()>
    where
        S: MemoryStore + MemoirStore + Send + Sync + 'static;
}
```

### HTTP Configuration Structures

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub instance_id: Uuid,  // Required, generated on init
    pub transport: TransportType,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Stdio,
    Http,
}

#[cfg(feature = "http-transport")]
#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    pub bind: String,  // Default: "127.0.0.1"
    pub port: u16,     // Default: 3000
    pub request_timeout_secs: u64,  // Default: 60
    pub max_request_size_mb: usize,  // Default: 1
    pub cors_enabled: bool,  // Default: false
    pub connection_limits: ConnectionLimits,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionLimits {
    pub per_key: usize,   // Default: 10
    pub per_ip: usize,    // Default: 50
    pub global: usize,    // Default: 1000
    pub idle_timeout_secs: u64,  // Default: 300 (5 minutes)
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub encryption_enabled: bool,  // Required in elevated posture
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub api_key_env: String,  // Default: "ALEJANDRIA_API_KEY"
    pub trust_proxy_headers: bool,  // Default: false (never trust by default)
    pub audit_log_path: PathBuf,
}
```

### SSE Event Format

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SseEvent {
    pub id: Option<String>,     // Event ID for client-side deduplication
    pub event: Option<String>,  // Event type (e.g., "progress", "log")
    pub data: String,           // JSON-RPC 2.0 notification
}

// Example SSE event for progress notification
{
    "id": "01HN3Q4Y5Z6X7W8V9U0T1S2R3Q",
    "event": "progress",
    "data": "{\"jsonrpc\":\"2.0\",\"method\":\"notification/progress\",\"params\":{\"operation\":\"mem_embed_all\",\"progress\":0.75,\"message\":\"Embedded 750/1000 memories\"}}"
}
```

### Authentication Context

```rust
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub api_key_hash: ApiKeyHash,  // SHA-256 hash for logging
    pub client_ip: IpAddr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiKeyHash(String);  // Hex-encoded SHA-256 hash

impl ApiKeyHash {
    pub fn from_key(key: &str) -> Self {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(key.as_bytes());
        Self(hex::encode(hash))
    }
}
```

### Instance Identity Registry (Database Schema)

```sql
-- API key registry for instance binding (EOP-001 mitigation)
CREATE TABLE IF NOT EXISTS api_key_registry (
    key_hash TEXT PRIMARY KEY,  -- SHA-256 hash of API key
    instance_id TEXT NOT NULL,  -- UUID of bound instance
    created_at INTEGER NOT NULL,  -- Unix timestamp
    description TEXT,  -- Optional human-readable description (e.g., "Team Alpha production key")
    last_used_at INTEGER  -- Automatic tracking for key rotation
);

CREATE INDEX idx_api_key_instance ON api_key_registry(instance_id);
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/alejandria-mcp/Cargo.toml` | Modify | Add `http-transport` feature with dependencies: axum, tokio, tower, hyper, tower-http, tower-governor, subtle, rand |
| `crates/alejandria-mcp/src/lib.rs` | Modify | Export `transport` module, re-export `handle_request` as public API |
| `crates/alejandria-mcp/src/server.rs` | Modify | Refactor to extract `handle_request` as public function, make `run_stdio_server` a thin wrapper around `StdioTransport::run` |
| `crates/alejandria-mcp/src/transport/mod.rs` | Create | Define `Transport` trait, export stdio and http modules |
| `crates/alejandria-mcp/src/transport/stdio.rs` | Create | Implement `StdioTransport` struct with `Transport` trait |
| `crates/alejandria-mcp/src/transport/http.rs` | Create | Module declaration with feature guard `#[cfg(feature = "http-transport")]` |
| `crates/alejandria-mcp/src/transport/http/mod.rs` | Create | Define `HttpTransport` struct and `Transport` impl, export auth/session/connection modules |
| `crates/alejandria-mcp/src/transport/http/auth.rs` | Create | Implement `authenticate` middleware with constant-time comparison and instance binding validation |
| `crates/alejandria-mcp/src/transport/http/session.rs` | Create | Implement `SessionManager` for SSE session tracking with TTL enforcement |
| `crates/alejandria-mcp/src/transport/http/connection.rs` | Create | Implement `ConnectionManager` with layered limits (per-key, per-IP, global) |
| `crates/alejandria-mcp/src/transport/http/handlers.rs` | Create | Implement HTTP endpoint handlers: `handle_rpc`, `handle_sse`, `handle_health` |
| `crates/alejandria-cli/src/config.rs` | Modify | Add `ServerConfig`, `HttpConfig`, `DatabaseConfig`, `SecurityConfig` with serde deserialization |
| `crates/alejandria-cli/src/commands/serve.rs` | Modify | Add transport selection logic, database encryption, file permission enforcement |
| `crates/alejandria-cli/src/commands/keygen.rs` | Create | Implement API key generation CLI: `alejandria keygen --instance <uuid> --output <file>` |
| `crates/alejandria-storage/src/lib.rs` | Modify | Add `open_encrypted` method using SQLCipher, add instance identity registry schema |
| `config/alejandria.toml.example` | Create | Example TOML configuration with all sections documented |
| `config/systemd/alejandria-mcp@.service` | Create | Systemd template unit with DynamicUser, security hardening, environment variable loading |
| `config/nginx/alejandria.conf.example` | Create | Nginx reverse proxy configuration with SSL, routing, SSE support |
| `docs/DEPLOYMENT.md` | Create | Comprehensive deployment guide for multi-instance HTTP deployments |
| `docs/SECURITY.md` | Create | Security architecture documentation covering all mitigations for CRITICAL findings |
| `.github/workflows/ci.yml` | Modify | Add matrix build for `--features http-transport` and stdio-only |
| `tests/integration/http_transport_test.rs` | Create | Integration tests for HTTP endpoints, authentication, connection limits |
| `tests/integration/multi_instance_test.rs` | Create | Integration tests for cross-instance isolation (EOP-001 validation) |
| `tests/security/timing_attack_test.rs` | Create | Security tests for constant-time comparison (ID-004 validation) |

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| **Unit** | Constant-time API key comparison | Property-based testing: verify timing is independent of difference position (QuickCheck with 10,000 samples) |
| **Unit** | Connection limit enforcement | Test ConnectionManager: per-key (10), per-IP (50), global (1000) limits with concurrent simulated connections |
| **Unit** | Session TTL expiration | Mock time using tokio::time::pause, create session, advance 31 minutes, verify validation fails |
| **Unit** | Instance ID binding validation | Create registry with key→instance bindings, verify cross-instance requests are rejected |
| **Unit** | Database encryption | Create encrypted database with SQLCipher, verify file is not readable as plaintext SQLite, verify queries work correctly |
| **Integration** | HTTP endpoint authentication | Send requests with: (1) no API key → 401, (2) invalid key → 403, (3) valid key → 200, measure response time variance |
| **Integration** | SSE connection lifecycle | Establish SSE connection, send events, verify reception, close connection, verify cleanup |
| **Integration** | Multi-instance isolation | Start 2 instances with different INSTANCE_IDs and API keys, verify cross-instance requests fail with 403 |
| **Integration** | Configuration validation | Test invalid configs: negative port, missing instance_id, invalid UUID format → verify startup fails with error |
| **Integration** | Graceful shutdown | Send SIGTERM during active SSE connections, verify connections drain before shutdown (max 5 seconds) |
| **Security** | Timing attack resistance | Statistical analysis: send 1000 requests with correct vs incorrect API keys, t-test for timing difference (p-value > 0.05 required) |
| **Security** | Connection flood DoS | Simulate connection flood: 1000 concurrent SSE connections, verify limits enforced, legitimate clients not affected |
| **Security** | Database file permissions | After database creation, verify permissions are 600, attempt read as different user, verify denied |
| **Security** | Audit log integrity | Write audit logs, modify a log entry, verify signature validation detects tampering |
| **Load** | SSE throughput | 100 concurrent SSE connections, send 1000 events/sec, measure latency (p99 < 100ms), memory usage (< 500MB) |
| **Load** | HTTP request throughput | 1000 concurrent POST /rpc requests, measure throughput (> 1000 req/sec on 4-core machine) |
| **Regression** | Stdio transport unchanged | Run existing stdio integration tests against refactored code, verify 100% pass rate |
| **Regression** | Binary size comparison | Compare binary size of stdio-only build before/after refactor, verify within 5% (spec requirement) |

## Migration / Rollout

### Phase 1: Transport Abstraction (Non-Breaking)

**Timeline**: Week 1-2

1. **Refactor `server.rs`**: Extract `handle_request` as public API
2. **Create `transport` module**: Define trait, implement `StdioTransport`
3. **Update `serve.rs`**: Use `StdioTransport::run` instead of `run_stdio_server`
4. **Regression testing**: Run full stdio test suite
5. **No user-visible changes** - stdio behavior identical

**Rollout**: Standard release, no migration required

### Phase 2: HTTP Transport + Security (Feature-Gated)

**Timeline**: Week 3-4

1. **Implement HTTP transport**: axum server, SSE handlers
2. **Add security layers**: Authentication, connection limits, session management
3. **Database encryption**: SQLCipher integration
4. **Configuration support**: TOML parsing, validation
5. **Testing**: Unit, integration, security tests

**Rollout**:
- Release with `--features http-transport` flag
- Default builds remain stdio-only (no binary size impact)
- Users opt-in via feature flag during build

### Phase 3: Deployment Tooling (Optional)

**Timeline**: Week 5-6

1. **Systemd templates**: Parameterized unit files
2. **Nginx configuration**: SSL, routing, SSE support
3. **CLI enhancements**: `keygen` command for API key generation
4. **Documentation**: DEPLOYMENT.md, SECURITY.md

**Rollout**:
- Release deployment artifacts (systemd, nginx configs) separately
- Users manually deploy (not automated)

### No Data Migration Required

This is an **additive-only change** - no existing data structures are modified:
- SQLite schema unchanged (only new `api_key_registry` table for HTTP deployments)
- Existing stdio deployments continue to work without modification
- Database files remain compatible across versions

## Open Questions

- [x] **Resolved**: Should database encryption be mandatory or optional? **Answer**: Optional by default, mandatory in elevated security posture (configurable via `database.encryption_enabled`)
- [x] **Resolved**: Should we use Redis for session storage in multi-instance deployments? **Answer**: Start with in-memory HashMap, document Redis as future enhancement for horizontal scaling
- [ ] **Unresolved**: Should we implement OAuth 2.0 / OIDC in addition to API keys? **Decision needed**: Adds complexity but enables SSO integration for enterprise deployments
- [ ] **Unresolved**: Should we support WebSocket transport in addition to SSE? **Decision needed**: SSE is simpler and meets requirements, but WebSocket allows bidirectional communication
- [x] **Resolved**: How to handle API key rotation without downtime? **Answer**: Support multiple API keys per instance (primary + secondary), allow overlap during rotation period

## Security Review Compliance Matrix

| Finding ID | Severity | Mitigation Status | Design Decision Reference |
|------------|----------|------------------|---------------------------|
| EOP-001 | 🔴 Critical | ✅ RESOLVED | Instance Identity Binding (Decision 4) |
| DOS-001 | 🔴 Critical | ✅ RESOLVED | Connection Limits via Layered Throttling (Decision 5) |
| ID-001 | 🔴 Critical | ✅ RESOLVED | Database Encryption via SQLCipher (Decision 6) |
| S-003 | 🔴 Critical | ✅ RESOLVED | SSE Session Security via Ephemeral Tokens (Decision 7) |
| S-001 | High | ✅ RESOLVED | API Key Security Hardening (Decision 8) |
| ID-004 | Medium | ✅ RESOLVED | Constant-time comparison (Decision 8) |
| R-001 | High | ✅ RESOLVED | Audit Logging with HMAC signatures (Decision 9) |
| T-001 | High | ✅ RESOLVED | JSON-RPC strict validation in handlers |
| DOS-002 | High | ✅ RESOLVED | Request timeouts in HttpConfig |
| DOS-003 | High | ✅ RESOLVED | Response pagination + size limits |
| S-002 | Medium | ✅ RESOLVED | Unset env vars after load (Decision 8) |
| T-003 | Medium | ✅ RESOLVED | X-Forwarded-For validation (Decision 9) |
| ID-002 | Medium | ✅ RESOLVED | Error sanitization layer (Decision 9) |
| ID-003 | High | ✅ RESOLVED | Per-connection broadcast channels (Decision 3) |

**All 4 CRITICAL blockers are architecturally resolved** - implementation will validate via security testing.
# Specification: http-sse-transport

**Created**: 2026-04-05T20:33:19.373Z

## Requirements

### 1. Transport Abstraction Layer

The system MUST provide a trait-based transport abstraction that decouples the MCP protocol implementation from the underlying communication mechanism. This enables support for multiple transports (stdio, HTTP/SSE) without duplicating protocol logic.

#### Scenarios

##### Stdio transport processes JSON-RPC requests

- **GIVEN** The server is running with stdio transport
- **WHEN** A JSON-RPC 2.0 request arrives via stdin
- **THEN** The transport layer reads the request and delivers it to the protocol handler

##### HTTP transport processes JSON-RPC requests

- **GIVEN** The server is running with HTTP transport and http-transport feature enabled
- **WHEN** A JSON-RPC 2.0 request arrives via HTTP POST to /rpc
- **THEN** The transport layer extracts the request body and delivers it to the protocol handler

##### Transport trait defines required capabilities

- **GIVEN** A new transport implementation is being created
- **WHEN** The transport trait is implemented
- **THEN** It MUST provide methods for: receiving requests, sending responses, sending notifications, and connection lifecycle management

##### Protocol handler is transport-agnostic

- **GIVEN** The MCP protocol handler processes a request
- **WHEN** The request is received via any transport implementation
- **THEN** The handler MUST process the request identically regardless of transport type

##### Transport abstraction handles errors gracefully

- **GIVEN** A transport layer encounters a connection error
- **WHEN** The error occurs during request processing
- **THEN** The transport MUST return a transport-specific error to the server

### 2. HTTP Transport Feature Flag

The system MUST support conditional compilation of HTTP transport via a Cargo feature flag 'http-transport'. When the feature is disabled, no HTTP-related dependencies SHALL be included in the binary, ensuring minimal binary size for stdio-only deployments.

#### Scenarios

##### Binary without http-transport feature excludes HTTP dependencies

- **GIVEN** The project is built without --features http-transport
- **WHEN** The binary is inspected for dependencies
- **THEN** No axum, hyper, or tower dependencies SHALL be present in the binary

##### CLI with http-transport feature enables HTTP options

- **GIVEN** The project is built with --features http-transport
- **WHEN** The serve command is invoked with --help
- **THEN** HTTP-specific flags (--http, --port, --bind) MUST be visible

##### CLI without http-transport feature rejects HTTP flags

- **GIVEN** The project is built without --features http-transport
- **WHEN** The serve command is invoked with --http flag
- **THEN** The CLI MUST return an error stating HTTP transport is not compiled in

##### Feature flag guards HTTP module compilation

- **GIVEN** The codebase includes HTTP transport module
- **WHEN** The code is compiled without http-transport feature
- **THEN** All HTTP transport code MUST be excluded via #[cfg(feature = "http-transport")]

### 3. HTTP/SSE Protocol Implementation

The system MUST implement HTTP endpoints for client-to-server requests and Server-Sent Events (SSE) for server-to-client streaming notifications, both conforming to JSON-RPC 2.0 specification.

#### Scenarios

##### HTTP POST endpoint accepts JSON-RPC requests

- **GIVEN** The HTTP transport is active and authenticated
- **WHEN** A client sends POST /rpc with valid JSON-RPC 2.0 request body
- **THEN** The server MUST parse the request and invoke the corresponding MCP tool

##### SSE endpoint streams server notifications

- **GIVEN** A client has established an SSE connection to /events
- **WHEN** The server generates a notification (e.g., progress update, log message)
- **THEN** The notification MUST be serialized as JSON-RPC 2.0

##### Invalid JSON-RPC request returns error response

- **GIVEN** The HTTP transport receives a malformed request
- **WHEN** The request body is not valid JSON or lacks required JSON-RPC fields
- **THEN** The server MUST return HTTP 400 with JSON-RPC 2.0 error response

##### SSE connection lifecycle is managed properly

- **GIVEN** A client establishes an SSE connection
- **WHEN** The connection is interrupted or closed by the client
- **THEN** The server MUST detect the disconnection within 30 seconds

##### HTTP transport supports CORS for browser clients

- **GIVEN** The HTTP transport is configured with CORS enabled
- **WHEN** A browser sends a preflight OPTIONS request
- **THEN** The server MUST respond with appropriate CORS headers (Access-Control-Allow-Origin, etc.)

##### Concurrent HTTP requests are handled independently

- **GIVEN** Multiple clients send HTTP POST requests simultaneously
- **WHEN** Requests arrive within the same time window
- **THEN** Each request MUST be processed in its own task/thread

### 4. API Key Authentication

The system MUST enforce API key authentication for all HTTP/SSE endpoints via the X-API-Key header. The expected API key SHALL be loaded from an environment variable. Requests without a valid API key MUST be rejected with HTTP 401 or 403.

#### Scenarios

##### Valid API key grants access

- **GIVEN** The server is configured with API key from ALEJANDRIA_API_KEY environment variable
- **WHEN** A client sends a request with X-API-Key header matching the configured key
- **THEN** The request MUST be processed normally

##### Missing API key returns 401 Unauthorized

- **GIVEN** The HTTP transport requires authentication
- **WHEN** A client sends a request without the X-API-Key header
- **THEN** The server MUST return HTTP 401 Unauthorized

##### Invalid API key returns 403 Forbidden

- **GIVEN** The HTTP transport requires authentication
- **WHEN** A client sends a request with an incorrect X-API-Key value
- **THEN** The server MUST return HTTP 403 Forbidden

##### API key is loaded from environment variable

- **GIVEN** The server starts with HTTP transport enabled
- **WHEN** The ALEJANDRIA_API_KEY environment variable is set
- **THEN** The server MUST load the API key from that variable

##### Server fails to start if API key is missing

- **GIVEN** The HTTP transport is enabled in configuration
- **WHEN** The ALEJANDRIA_API_KEY environment variable is not set or empty
- **THEN** The server MUST exit with an error during startup

##### API key authentication applies to all HTTP endpoints

- **GIVEN** The HTTP transport is active
- **WHEN** A client accesses any endpoint (/rpc, /events, /health)
- **THEN** The API key MUST be validated before processing the request

### 5. TOML Configuration File Support

The system MUST support TOML configuration files for transport selection and HTTP-specific settings. The configuration SHALL specify transport type (stdio or http), HTTP bind address, port, and API key environment variable name.

#### Scenarios

##### TOML config selects stdio transport

- **GIVEN** A config file exists with transport = 'stdio'
- **WHEN** The server is started with --config <path>
- **THEN** The server MUST initialize the stdio transport layer

##### TOML config selects HTTP transport with settings

- **GIVEN** A config file specifies transport = 'http', bind = '0.0.0.0', port = 3000
- **WHEN** The server is started with --config <path>
- **THEN** The HTTP transport MUST bind to 0.0.0.0:3000

##### Configuration validation rejects invalid values

- **GIVEN** A config file specifies an invalid port (e.g., -1 or 70000)
- **WHEN** The server attempts to load the configuration
- **THEN** The server MUST exit with a validation error

##### Configuration supports API key environment variable override

- **GIVEN** A config file specifies api_key_env = 'CUSTOM_API_KEY'
- **WHEN** The server starts with HTTP transport
- **THEN** The server MUST load the API key from the CUSTOM_API_KEY environment variable

##### Default configuration is used when file is absent

- **GIVEN** No configuration file is provided
- **WHEN** The server is started without --config flag
- **THEN** The server MUST default to stdio transport

##### Configuration file path can be relative or absolute

- **GIVEN** A configuration file exists at ./config/alejandria.toml
- **WHEN** The server is started with --config ./config/alejandria.toml
- **THEN** The server MUST resolve the path relative to the current working directory

### 6. Multi-Instance Deployment Support

The system MUST support running multiple independent server instances simultaneously, each with isolated database files, separate configurations, and distinct HTTP ports (when applicable). This enables multi-team deployments where each team has their own isolated Alejandria instance.

#### Scenarios

##### Multiple HTTP instances run on different ports

- **GIVEN** Two configuration files specify HTTP transport with ports 3000 and 3001
- **WHEN** Both server instances are started concurrently
- **THEN** Each instance MUST bind to its configured port without conflict

##### Each instance uses isolated database file

- **GIVEN** Configuration files specify different database paths
- **WHEN** Multiple server instances are running
- **THEN** Each instance MUST read/write only to its configured database file

##### Instance-specific API keys prevent cross-instance access

- **GIVEN** Two HTTP instances are configured with different API keys
- **WHEN** A client attempts to access instance A with instance B's API key
- **THEN** The request MUST be rejected with HTTP 403

##### Systemd service template supports parameterized instances

- **GIVEN** A systemd service template alejandria@.service exists
- **WHEN** The template is instantiated as alejandria@team-alpha.service
- **THEN** The service MUST load configuration from /etc/alejandria/team-alpha.toml

##### Nginx reverse proxy routes requests to correct instance

- **GIVEN** Nginx is configured with location blocks for /api/team-alpha and /api/team-beta
- **WHEN** A client sends a request to /api/team-alpha/rpc
- **THEN** Nginx MUST proxy the request to the team-alpha instance

### 7. Backward Compatibility with Stdio Transport

The system MUST maintain 100% backward compatibility with existing stdio-based deployments. The stdio transport SHALL remain the default, and existing CLI usage patterns MUST continue to work without modification. No breaking changes to the MCP protocol API are permitted.

#### Scenarios

##### Default behavior is stdio transport

- **GIVEN** The server is started without any transport configuration
- **WHEN** The serve command is invoked with no flags
- **THEN** The server MUST use stdio transport

##### Existing MCP tools continue to work

- **GIVEN** The server is running with stdio transport
- **WHEN** A client invokes any existing MCP tool (e.g., mem_save, mem_search)
- **THEN** The tool MUST function identically to pre-HTTP versions

##### Binary built without http-transport feature is identical to legacy

- **GIVEN** The project is built without --features http-transport
- **WHEN** The binary is compared to the previous stdio-only version
- **THEN** The binary size MUST be within 5% of the legacy version

##### Configuration file is optional for stdio transport

- **GIVEN** The server is started without --config flag
- **WHEN** No configuration file exists in default locations
- **THEN** The server MUST start successfully with stdio transport

##### Protocol version remains unchanged

- **GIVEN** The MCP server advertises a protocol version
- **WHEN** A client queries the server capabilities
- **THEN** The protocol version MUST NOT change due to HTTP transport addition

### 8. HTTP Server Lifecycle Management

The system MUST provide proper lifecycle management for HTTP servers including graceful startup, health checks, signal handling for shutdown, and connection draining. This ensures reliable operation in production environments.

#### Scenarios

##### Server starts and binds to configured address

- **GIVEN** The HTTP transport is configured to bind to 0.0.0.0:3000
- **WHEN** The server starts
- **THEN** The server MUST successfully bind to the address

##### Health check endpoint reports server status

- **GIVEN** The HTTP server is running
- **WHEN** A GET request is sent to /health
- **THEN** The server MUST return HTTP 200 with JSON body containing status: 'healthy'

##### SIGTERM triggers graceful shutdown

- **GIVEN** The HTTP server is running with active connections
- **WHEN** The process receives SIGTERM signal
- **THEN** The server MUST stop accepting new connections immediately

##### SIGINT triggers immediate shutdown

- **GIVEN** The HTTP server is running
- **WHEN** The process receives SIGINT signal (Ctrl+C)
- **THEN** The server MUST stop accepting new connections immediately

##### Connection timeouts are enforced

- **GIVEN** The HTTP server has a configured request timeout of 60 seconds
- **WHEN** A client sends a request but does not complete it within the timeout
- **THEN** The server MUST close the connection after 60 seconds

##### Server logs startup configuration

- **GIVEN** The HTTP server is starting
- **WHEN** The server completes initialization
- **THEN** The server MUST log: bind address, port, authentication status (enabled/disabled), CORS status

### 9. Error Handling and Logging

The system MUST provide comprehensive error handling and structured logging for all transport operations. Errors SHALL be categorized (transport, protocol, application) and logged with appropriate severity levels. Sensitive information MUST NOT be logged.

#### Scenarios

##### Transport errors are logged with context

- **GIVEN** An HTTP request fails due to network error
- **WHEN** The transport layer encounters the error
- **THEN** The server MUST log the error with: timestamp, error type, client IP (if available), request ID

##### Protocol errors are returned to client

- **GIVEN** A JSON-RPC request has an invalid method name
- **WHEN** The protocol handler processes the request
- **THEN** The server MUST return JSON-RPC error response with code -32601 (Method not found)

##### Application errors include request context

- **GIVEN** An MCP tool execution fails (e.g., database error)
- **WHEN** The error is propagated to the transport layer
- **THEN** The server MUST log: tool name, error message, request ID, timestamp

##### Sensitive data is redacted from logs

- **GIVEN** A request includes API key or authentication token
- **WHEN** The request is logged for debugging
- **THEN** The API key value MUST be redacted or replaced with '***'

##### Structured logging enables filtering

- **GIVEN** The server uses structured logging (e.g., tracing crate)
- **WHEN** Logs are written to output
- **THEN** Each log entry MUST include: level, timestamp, module path, structured fields

##### Rate limiting errors are handled gracefully

- **GIVEN** The HTTP transport implements rate limiting
- **WHEN** A client exceeds the rate limit
- **THEN** The server MUST return HTTP 429 Too Many Requests

### 10. Security Hardening

The system MUST implement security best practices including request size limits, timeout enforcement, TLS support readiness, and input validation. The HTTP transport SHALL be designed to prevent common web vulnerabilities (injection, DoS, etc.).

#### Scenarios

##### Request body size is limited

- **GIVEN** The HTTP transport has a configured max request size (default: 1MB)
- **WHEN** A client sends a POST request with body larger than the limit
- **THEN** The server MUST reject the request with HTTP 413 Payload Too Large

##### Request headers are validated

- **GIVEN** The HTTP transport receives a request
- **WHEN** The request includes invalid or malicious headers
- **THEN** The server MUST reject headers exceeding size limits (default: 8KB per header)

##### TLS termination is supported via reverse proxy

- **GIVEN** Nginx is configured with TLS termination for Alejandria backend
- **WHEN** The server binds to 127.0.0.1:3000 (localhost only)
- **THEN** The server MUST accept plaintext HTTP from Nginx

##### Path traversal attacks are prevented

- **GIVEN** The HTTP server serves static endpoints (/rpc, /events, /health)
- **WHEN** A client sends a request with path traversal sequences (../, encoded versions)
- **THEN** The server MUST normalize paths before routing

##### JSON-RPC method names are validated

- **GIVEN** A client sends a JSON-RPC request with method name
- **WHEN** The method name contains unexpected characters (e.g., shell metacharacters)
- **THEN** The server MUST validate method name against allowlist of known MCP tools

##### Connection limits prevent resource exhaustion

- **GIVEN** The HTTP server has a configured max connection limit (default: 1000)
- **WHEN** The number of active connections reaches the limit
- **THEN** The server MUST reject new connections with HTTP 503 Service Unavailable

---
spec_hash: "sha256:cd4d69b30a6b570730a99e47279a99aee56a20d957328e0192520f5c0c58727b"
posture: "elevated"
findings_count: 21
critical_count: 4
risk_level: "high"
timestamp: "2026-04-05T20:39:53.371Z"
change: "http-sse-transport"
---

# Security Review: http-sse-transport

**Posture**: elevated | **Risk Level**: high | **Findings**: 21 (4 critical)

## STRIDE Analysis

### Spoofing

#### S-001: API Key Brute Force Attack

- **Severity**: high
- **Description**: **Attacker Goal**: Gain unauthorized access to Alejandria instance by guessing valid API keys. **Attack Vector**: Automated tools send thousands of HTTP requests with different X-API-Key values to /rpc endpoint. If no rate limiting is enforced, attackers can systematically enumerate keys using dictionary attacks or leaked key lists from other breaches. Timing side-channels in constant-time comparison may leak key length information.
- **Mitigation**: Implement constant-time comparison for API key validation using subtle::ConstantTimeEq trait. Add exponential backoff rate limiting (e.g., 5 failed attempts = 1 minute lockout per IP). Enforce minimum API key entropy (256 bits) via startup validation. Log all failed authentication attempts with client IP to detect brute force patterns. Consider implementing account lockout after N failed attempts.
- **Affected Components**: HTTP transport middleware, API key validation, Authentication layer

#### S-002: Environment Variable Leakage via Process Inspection

- **Severity**: medium
- **Description**: **Attacker Goal**: Steal API keys by inspecting process memory or environment variables on multi-tenant systems. **Attack Vector**: On shared hosting or compromised systems, attackers use /proc/<pid>/environ on Linux or Process Explorer on Windows to read ALEJANDRIA_API_KEY from running processes. If the API key is stored in environment variables, it remains in plaintext in process memory for the lifetime of the server.
- **Mitigation**: After reading API key from environment variable at startup, immediately unset it using std::env::remove_var. Store the key in a secured memory structure (consider using mlock/VirtualLock to prevent swapping to disk). Document that API keys should use secrets management systems (HashiCorp Vault, AWS Secrets Manager) in production rather than environment variables. Add warning in logs if ALEJANDRIA_API_KEY is still set after server initialization.
- **Affected Components**: Server initialization, Configuration loader, Environment variable handling

#### S-003: Session Hijacking via SSE Connection Takeover

- **Severity**: critical
- **Description**: **Attacker Goal**: Intercept or replay SSE event streams to gain access to another user's notifications and data. **Attack Vector**: If SSE connections are not properly authenticated per-connection or use predictable connection IDs, attackers can guess or enumerate connection identifiers. Without TLS, network-level attackers (same WiFi, compromised router) can sniff X-API-Key headers and replay them to establish parallel SSE connections.
- **Mitigation**: Require X-API-Key header for EVERY SSE connection, not just initial handshake. Generate cryptographically random connection IDs using rand::thread_rng() with at least 128 bits of entropy. Implement connection binding: tie each SSE stream to the originating IP address and reject if IP changes (with configurable tolerance for mobile clients). Enforce TLS-only mode in production configurations with clear documentation. Add connection timeout for idle SSE streams (max 5 minutes without keepalive).
- **Affected Components**: SSE handler, Connection manager, Authentication middleware

### Tampering

#### T-001: JSON-RPC Request Injection

- **Severity**: high
- **Description**: **Attacker Goal**: Manipulate server behavior by injecting malicious JSON-RPC payloads that bypass validation. **Attack Vector**: Attackers craft JSON-RPC requests with oversized arrays, deeply nested objects, or duplicate 'method' fields to cause deserialization issues. They exploit differences between serde_json parsing and the application's validation logic. For example, {'method': 'mem_save', 'method': 'mem_delete'} might confuse parsers that don't reject duplicates.
- **Mitigation**: Use strict JSON schema validation with maximum depth limits (e.g., max 10 levels) and array size limits (max 1000 elements). Configure serde_json to reject duplicate keys via serde(deny_unknown_fields). Implement allowlist validation for 'method' field using an enum-based dispatcher. Add request size limit (1MB default) at HTTP layer before JSON parsing to prevent memory exhaustion. Validate all JSON-RPC required fields (jsonrpc, method, id) before processing.
- **Affected Components**: JSON-RPC parser, Request validator, HTTP POST handler

#### T-002: Path Traversal in Configuration File Loading

- **Severity**: medium
- **Description**: **Attacker Goal**: Read arbitrary files or inject malicious configuration by exploiting --config file path handling. **Attack Vector**: Attackers invoke the server with --config ../../../../etc/passwd or --config /path/to/malicious.toml containing attacker-controlled bind addresses or database paths. If path normalization is insufficient, they can read sensitive files via error messages or trick the server into binding to malicious addresses.
- **Mitigation**: Canonicalize all config file paths using std::fs::canonicalize() before opening. Restrict config files to specific directories (e.g., /etc/alejandria/, ./config/) via allowlist. Validate TOML structure before parsing: reject files with unexpected top-level keys. Never include file contents in error messages - only report 'invalid configuration file' without details. Ensure all file I/O errors are sanitized to prevent path disclosure.
- **Affected Components**: Configuration loader, CLI argument parser, File I/O layer

#### T-003: HTTP Header Injection via Reverse Proxy

- **Severity**: medium
- **Description**: **Attacker Goal**: Bypass authentication or gain elevated privileges by injecting spoofed headers through misconfigured reverse proxies. **Attack Vector**: If the server trusts X-Forwarded-For or X-Real-IP headers without validation, attackers send requests with forged headers to bypass IP-based rate limiting or logging. For example, sending 'X-Forwarded-For: 127.0.0.1' to appear as localhost and potentially bypass firewall rules.
- **Mitigation**: Do NOT trust X-Forwarded-For headers by default - make this opt-in via configuration flag 'trust_proxy_headers'. When enabled, only accept these headers from configured trusted proxy IP ranges. Use the rightmost IP in X-Forwarded-For chain as the actual client IP (leftmost IPs can be forged). Document reverse proxy configuration requirements clearly: Nginx must use proxy_set_header X-Real-IP $remote_addr. Validate all header values against expected formats (IP regex) before use.
- **Affected Components**: Reverse proxy integration, IP address extraction, Rate limiting

### Repudiation

#### R-001: Insufficient Audit Logging for MCP Operations

- **Severity**: high
- **Description**: **Attacker Goal**: Perform malicious actions (data deletion, exfiltration) without leaving traceable evidence. **Attack Vector**: If the system only logs transport-level events but not application-level MCP tool invocations, an attacker with valid API key can invoke mem_delete or mem_search extensively without attribution. They can claim 'it wasn't me' or 'the system malfunctioned' because there's no proof of who invoked what operation.
- **Mitigation**: Implement comprehensive audit logging for ALL MCP tool invocations with these fields: timestamp, client_ip, api_key_id (hashed identifier, not the key itself), method_name, parameters (sanitized - no sensitive data), result_status (success/failure), session_id, request_id. Store audit logs separately from application logs using append-only storage or forward to external SIEM. Include cryptographic signatures on log entries to prevent tampering. Add log rotation with retention policy (minimum 90 days for compliance).
- **Affected Components**: MCP protocol handler, Logging layer, Audit subsystem

#### R-002: Missing Correlation Between Multi-Instance Operations

- **Severity**: medium
- **Description**: **Attacker Goal**: Exploit the lack of cross-instance audit trails in multi-team deployments to hide coordinated attacks. **Attack Vector**: In multi-instance deployments, each instance logs independently without correlation IDs. An attacker with access to multiple team API keys can perform distributed attacks (e.g., scanning for specific data across all instances) and the separate log files won't reveal the coordinated pattern.
- **Mitigation**: Generate a global request ID at ingress (e.g., UUID v4) and propagate it through all log entries. For multi-instance deployments, add instance_id to all log entries to enable correlation. Consider implementing centralized logging infrastructure (ELK stack, Loki) where all instances forward structured logs. Document log aggregation requirements in deployment guide. Add optional distributed tracing integration (OpenTelemetry) for advanced deployments.
- **Affected Components**: Logging infrastructure, Request context, Multi-instance deployment

### Information Disclosure

#### ID-001: Database File Leakage in Multi-Tenant Deployments

- **Severity**: critical
- **Description**: **Attacker Goal**: Access another team's SQLite database file to exfiltrate all stored memories and knowledge graphs. **Attack Vector**: If database file paths follow predictable patterns (e.g., /var/lib/alejandria/team-{name}.db) and file permissions are misconfigured, attackers can read other teams' database files directly from the filesystem. Even with proper API key isolation, a compromised server allows direct file access.
- **Mitigation**: Enforce strict file permissions: database files must be readable/writable only by the service user (chmod 600). Use separate Linux user accounts for each instance in multi-tenant deployments (systemd DynamicUser=yes). Store database files in instance-specific directories with parent directory permissions 700. Add database encryption at rest using SQLCipher with per-instance encryption keys derived from API key or separate secrets. Document file permission requirements in deployment guide with automated validation script.
- **Affected Components**: Database initialization, Multi-instance deployment, File system security

#### ID-002: Sensitive Data in Error Messages

- **Severity**: medium
- **Description**: **Attacker Goal**: Extract database schema, file paths, API keys, or internal state by triggering verbose error messages. **Attack Vector**: Attackers send malformed requests designed to trigger exceptions. If error messages include full stack traces, database query details, or configuration values, they leak implementation details. For example, 'Failed to connect to database at /home/user/.alejandria/secret_project.db' reveals internal paths.
- **Mitigation**: Implement error sanitization layer that returns generic error messages to clients ('Internal server error') while logging full details internally. Use custom error types with explicit to_client() methods that never include: file paths, SQL queries, environment variable names, stack traces, memory addresses. In development mode, allow verbose errors via ALEJANDRIA_DEBUG_ERRORS env var (never in production). Add automated tests that verify error responses contain no sensitive patterns.
- **Affected Components**: Error handling, HTTP response formatter, Logging

#### ID-003: SSE Stream Data Leakage to Unauthorized Clients

- **Severity**: high
- **Description**: **Attacker Goal**: Receive real-time notifications and progress updates intended for other users by exploiting weak SSE stream isolation. **Attack Vector**: If SSE stream multiplexing is implemented incorrectly, messages intended for one authenticated client might be broadcast to all connected clients. An attacker establishes a legitimate SSE connection and receives events from other users' operations.
- **Mitigation**: Implement per-client stream isolation: each SSE connection gets a unique channel tied to its authenticated API key. Never use a global broadcast mechanism - always target specific connection IDs. Add session validation: before sending any SSE event, verify the connection's API key still matches. Implement connection metadata tracking: store {connection_id -> (api_key_hash, ip_address, created_at)} mapping. Add automated tests that verify multi-client SSE isolation by establishing connections with different API keys.
- **Affected Components**: SSE event broadcaster, Connection manager, Session isolation

#### ID-004: Timing Side-Channel in API Key Comparison

- **Severity**: medium
- **Description**: **Attacker Goal**: Determine valid API key characters one-by-one using timing attack analysis. **Attack Vector**: If API key comparison uses standard string comparison (==), it returns early on the first mismatched character. By measuring response times with high precision (nanosecond resolution), attackers can statistically determine correct characters. For example, 'Axxxxx' takes 1μs, 'Bxxxxx' takes 1μs, but correct first char takes 1.1μs.
- **Mitigation**: Use constant-time comparison for API key validation: subtle::ConstantTimeEq or ring::constant_time::verify_slices_are_equal. This ensures comparison time is independent of where keys differ. Add random jitter (0-10ms) to authentication responses to make timing measurements noisier. Implement rate limiting that makes timing attacks impractical (max 1 auth attempt per second per IP). Document the importance of cryptographically random API key generation (use rand::thread_rng() with at least 32 bytes).
- **Affected Components**: API key validator, Authentication middleware

### Denial of Service

#### DOS-001: SSE Connection Exhaustion Attack

- **Severity**: critical
- **Description**: **Attacker Goal**: Exhaust server resources by opening thousands of persistent SSE connections. **Attack Vector**: Attackers with valid API keys (or multiple leaked keys) establish thousands of SSE connections to /events endpoint. Each connection consumes server memory (buffers, state) and file descriptors. With default limits, 65k connections can exhaust file descriptor limits and prevent legitimate users from connecting.
- **Mitigation**: Implement per-API-key connection limits (max 10 concurrent SSE connections per key). Add global connection ceiling (default: 1000 total connections) configurable via ALEJANDRIA_MAX_CONNECTIONS. Use tower-governor or similar rate limiting middleware to throttle connection attempts (max 5/minute per IP). Implement connection keepalive timeout: close idle connections after 5 minutes of inactivity. Add monitoring/alerting when connection count exceeds 80% of limit. Document connection limits in deployment guide.
- **Affected Components**: SSE endpoint, Connection manager, Resource limits

#### DOS-002: Slowloris-Style Slow Request Attack

- **Severity**: high
- **Description**: **Attacker Goal**: Tie up server resources by sending deliberately slow HTTP requests. **Attack Vector**: Attackers open many connections and send partial HTTP requests (headers or body) at extremely slow rates (1 byte per 10 seconds). The server holds these connections open, waiting for complete requests, eventually exhausting connection pools and preventing legitimate requests.
- **Mitigation**: Configure aggressive request timeouts in axum: request header timeout (10 seconds), request body timeout (30 seconds), total request timeout (60 seconds). Use hyper server configuration with http1_header_read_timeout and http2_keep_alive_timeout. Implement read buffer limits to prevent memory exhaustion from partial requests. Add connection-level timeouts at OS level using SO_RCVTIMEO socket option. Monitor slow request metrics and alert when >10% of requests exceed thresholds.
- **Affected Components**: HTTP server configuration, Timeout management, Connection handling

#### DOS-003: Amplification Attack via Large Response Generation

- **Severity**: high
- **Description**: **Attacker Goal**: Cause resource exhaustion by requesting operations that generate disproportionately large responses. **Attack Vector**: Attackers invoke MCP tools like mem_search with broad queries that return thousands of results. If response size is unlimited, a single request can consume gigabytes of memory during serialization and transmission. Repeated requests cause memory exhaustion.
- **Mitigation**: Implement response pagination for mem_search and similar tools (max 100 results per request by default). Add response size limits at serialization layer (max 10MB per response). Use streaming serialization for large responses instead of buffering entire response in memory. Implement query complexity analysis: reject queries that would scan >10000 database rows. Add request cost accounting: track resource consumption per API key and throttle expensive operations.
- **Affected Components**: MCP tool handlers, Response serialization, Query executor

#### DOS-004: Database Lock Contention via Concurrent Writes

- **Severity**: medium
- **Description**: **Attacker Goal**: Degrade database performance and cause request timeouts by flooding the system with write operations. **Attack Vector**: SQLite uses a global write lock - only one writer can proceed at a time. Attackers send bursts of mem_save requests to create lock contention. Legitimate read queries wait for write locks to release, causing cascading timeouts and degraded service.
- **Mitigation**: Enable WAL (Write-Ahead Logging) mode in SQLite to allow concurrent reads during writes: PRAGMA journal_mode=WAL. Implement write request queuing with bounded queue size (max 1000 pending writes). Add database connection pooling with separate read/write connection pools. Configure SQLite busy timeout (PRAGMA busy_timeout=5000) to reduce immediate lock failures. Implement write rate limiting per API key (max 100 writes/minute). Monitor database lock wait metrics.
- **Affected Components**: SQLite configuration, Database connection pool, Write path

### Elevation of Privilege

#### EOP-001: Cross-Instance Data Access via Port Confusion

- **Severity**: critical
- **Description**: **Attacker Goal**: Access data from a different team's Alejandria instance by exploiting port-based routing weaknesses. **Attack Vector**: In multi-instance deployments, if Nginx routing is misconfigured or API key validation is bypassed, attackers can send requests to /api/team-alpha/rpc but include team-beta's API key. If the backend doesn't validate that the API key matches the instance, they gain unauthorized access to team-beta's database.
- **Mitigation**: Implement instance identity validation: each server instance must have a unique INSTANCE_ID (separate from API key) set at startup. Add instance_id to configuration file and validate that all requests are intended for this specific instance. At authentication layer, bind API keys to specific instance IDs in a mapping file or database. Reject requests if X-Instance-ID header (optional) doesn't match server's INSTANCE_ID. Add automated tests for multi-instance isolation. Document Nginx configuration requirements: proxy_set_header X-Instance-ID team-alpha.
- **Affected Components**: Multi-instance routing, Authentication, Configuration management

#### EOP-002: Feature Flag Bypass via Build Manipulation

- **Severity**: low
- **Description**: **Attacker Goal**: Enable HTTP transport features in a supposedly stdio-only binary to gain remote access. **Attack Vector**: If feature flag guards are inconsistent or incomplete, attackers might exploit partial HTTP code that remains in the binary. They manipulate environment variables or configuration files to enable HTTP endpoints even when http-transport feature was not compiled in.
- **Mitigation**: Ensure comprehensive feature gating: all HTTP-related code must be behind #[cfg(feature = "http-transport")], including: struct definitions, imports, function implementations. Add compile-time assertion that fails if HTTP code is accessible without feature flag. Implement runtime validation: check cfg!(feature = "http-transport") at startup and panic if HTTP config is provided but feature is disabled. Add automated CI tests that verify binary size and symbol table of non-HTTP build contains no axum/hyper symbols.
- **Affected Components**: Feature flag guards, Compilation, Runtime validation

#### EOP-003: Privilege Escalation via TOML Config Injection

- **Severity**: medium
- **Description**: **Attacker Goal**: Gain elevated system privileges by injecting malicious configuration that changes server behavior. **Attack Vector**: If config file parsing is vulnerable to TOML injection or directory traversal, attackers could modify: bind address to 0.0.0.0 (exposing server publicly), database path to /etc/shadow (reading sensitive files via error messages), or log paths to overwrite system files.
- **Mitigation**: Validate all configuration values against strict schemas using serde validation. For bind addresses: allowlist only valid IP formats and warn if 0.0.0.0 is used in production. For file paths: canonicalize and restrict to specific base directories. Never allow absolute paths in config files unless explicitly enabled via --allow-absolute-paths flag (disabled by default). Implement configuration signing: optionally verify config file signature using ed25519 before loading. Run server process with minimal privileges: use systemd DynamicUser and PrivateTmp.
- **Affected Components**: Configuration parser, Path validation, Privilege management

## OWASP Top 10 Mapping

| OWASP ID | Category | Related Threats | Applicable |
|----------|----------|-----------------|------------|
| A01:2021 | Broken Access Control | EOP-001, S-003, ID-003 | Yes |
| A02:2021 | Cryptographic Failures | ID-001, S-002, ID-004 | Yes |
| A03:2021 | Injection | T-001, T-002, EOP-003 | Yes |
| A04:2021 | Insecure Design | DOS-003, R-002 | Yes |
| A05:2021 | Security Misconfiguration | T-003, ID-001, EOP-002 | Yes |
| A06:2021 | Vulnerable and Outdated Components | — | No |
| A07:2021 | Identification and Authentication Failures | S-001, S-003, ID-004 | Yes |
| A08:2021 | Software and Data Integrity Failures | R-001 | Yes |
| A09:2021 | Security Logging and Monitoring Failures | R-001, R-002 | Yes |
| A10:2021 | Server-Side Request Forgery | — | No |

## Abuse Cases

| ID | Severity | As an attacker, I want to... | STRIDE |
|----|----------|------------------------------|--------|
| AC-001 | 🔴 critical | As an attacker, I want to access confidential memory data from other teams' Alejandria instances to steal proprietary information and trade secrets | Elevation of Privilege |
| AC-002 | 🔴 critical | As an attacker, I want to exhaust server resources and deny service to legitimate users by flooding the system with persistent SSE connections | Denial of Service |
| AC-003 | 🟠 high | As an attacker, I want to enumerate valid API key characters using timing side-channel attacks to gain unauthorized access without brute forcing the entire keyspace | Spoofing |
| AC-004 | 🟠 high | As an attacker with compromised server access, I want to bypass API authentication entirely by directly reading SQLite database files to extract all stored data | Information Disclosure |
| AC-005 | 🟠 high | As an attacker, I want to invoke restricted or dangerous MCP tools by exploiting JSON-RPC parsing vulnerabilities to gain unauthorized capabilities | Tampering |
| AC-006 | 🟡 medium | As an insider threat with valid access, I want to delete or modify audit logs to cover my tracks after exfiltrating sensitive data | Repudiation |
| AC-007 | 🟡 medium | As an attacker with limited process inspection privileges, I want to steal API keys from environment variables to gain persistent authenticated access | Spoofing |
| AC-008 | 🟡 medium | As an external attacker, I want to bypass IP-based access controls by spoofing X-Forwarded-For headers to appear as a trusted internal IP address | Tampering |

### AC-001: Unauthorized Multi-Instance Data Exfiltration

- **Severity**: 🔴 Critical
- **Goal**: As an attacker, I want to access confidential memory data from other teams' Alejandria instances to steal proprietary information and trade secrets
- **Technique**: The attacker compromises one team's API key (via phishing, insider threat, or leaked credentials). They analyze the Nginx reverse proxy configuration to identify other team endpoints (/api/team-beta, /api/team-gamma). They craft HTTP requests to these endpoints using the compromised API key. If instance identity validation is missing (threat EOP-001), the backend accepts the valid API key regardless of which instance it's intended for. The attacker enumerates all team endpoints and systematically calls mem_search with broad queries to exfiltrate all stored memories. Tools used: curl, Burp Suite for request manipulation, custom Python scripts for bulk enumeration.
- **Preconditions**: Multi-instance deployment is active with multiple teams; Attacker has obtained at least one valid API key; Instance identity validation is not implemented; Nginx routing allows requests to reach backend regardless of API key binding
- **Impact**: Complete compromise of data isolation in multi-tenant deployment. Attacker gains access to all teams' episodic memories, semantic knowledge graphs, and private notes. This violates confidentiality guarantees and could lead to: IP theft, competitive intelligence leakage, regulatory compliance violations (GDPR if PII is stored), reputational damage, and potential legal liability.
- **Mitigation**: Implement instance identity binding: each API key must be cryptographically bound to a specific INSTANCE_ID. Add instance_id field to server configuration and API key validation logic that rejects requests if the API key's bound instance doesn't match the server's INSTANCE_ID. Store API key->instance mappings in a secure registry (database or config file with file permissions 600). Add integration tests that verify cross-instance requests are rejected with HTTP 403. Document Nginx configuration to include X-Instance-ID header for defense-in-depth.
- **STRIDE**: Elevation of Privilege
- **Testable**: Yes
- **Test Hint**: Integration test: Start two server instances with different INSTANCE_IDs and API keys. Send request to instance A with instance B's API key. Verify HTTP 403 response and audit log entry.

### AC-002: SSE Connection Flooding Resource Exhaustion

- **Severity**: 🔴 Critical
- **Goal**: As an attacker, I want to exhaust server resources and deny service to legitimate users by flooding the system with persistent SSE connections
- **Technique**: The attacker obtains a valid API key (or multiple keys from compromised accounts). They develop a Python script using requests library with stream=True to establish persistent SSE connections. The script spawns 1000+ threads, each establishing a /events connection with valid authentication. Due to missing per-key connection limits (threat DOS-001), the server accepts all connections. Each connection consumes ~4KB of memory plus file descriptor. With 10,000 connections, the server exhausts file descriptors (ulimit -n typically 65536) and available memory. Legitimate users receive 'connection refused' errors.
- **Preconditions**: Attacker has access to at least one valid API key; Per-API-key connection limits are not enforced; Global connection ceiling is absent or too high; Server is not behind a rate-limiting proxy
- **Impact**: Complete denial of service for all legitimate users. The server becomes unresponsive to new connections and existing SSE streams may degrade. Database operations may timeout due to resource starvation. The server may require manual restart, causing downtime. In multi-instance deployments, attackers can target all instances simultaneously to create organization-wide outage.
- **Mitigation**: Implement layered connection limits: (1) Per-API-key limit: max 10 concurrent SSE connections per key. (2) Per-IP limit: max 50 connections per IP address (for shared API keys). (3) Global ceiling: max 1000 total SSE connections. Use tower-governor middleware for rate limiting connection attempts (max 5/minute per IP). Implement connection keepalive with automatic timeout: close idle connections after 5 minutes. Add connection telemetry: track connection count by API key and IP, expose metrics to monitoring. Document connection limits in API documentation and return HTTP 429 with Retry-After header when limits are exceeded.
- **STRIDE**: Denial of Service
- **Testable**: Yes
- **Test Hint**: Load test: Use vegeta or similar tool to establish 1000 SSE connections with valid API key. Verify that request 11 is rejected with HTTP 429 when per-key limit is 10. Verify server remains responsive to other operations.

### AC-003: API Key Enumeration via Timing Analysis

- **Severity**: 🟠 High
- **Goal**: As an attacker, I want to enumerate valid API key characters using timing side-channel attacks to gain unauthorized access without brute forcing the entire keyspace
- **Technique**: The attacker develops a custom timing attack tool that sends authentication requests with crafted API keys and measures response times with nanosecond precision (using rdtsc on x86 or similar). If the server uses standard string comparison, it returns early on the first mismatched character. The attacker sends requests like 'Axxxxxxx...', 'Bxxxxxxx...', 'Cxxxxxxx...' and identifies the correct first character by detecting the longest comparison time. They repeat this process for each character position, reducing keyspace from 256^32 to 32*256 attempts for a 32-byte key. Total attacks: ~8,000 instead of 2^256.
- **Preconditions**: API key validation uses non-constant-time comparison; Network latency variance is low enough to detect microsecond differences; Rate limiting is absent or insufficient to prevent timing analysis; Server is not behind a proxy that adds unpredictable latency
- **Impact**: Successful recovery of valid API keys provides full authenticated access to the Alejandria instance. Attacker can read, modify, or delete all stored memories and knowledge. The compromised API key can be used for long-term persistent access until manually rotated. This bypasses brute force protections and allows targeted attacks.
- **Mitigation**: Replace all API key comparisons with constant-time implementations: use subtle::ConstantTimeEq trait from subtle crate or ring::constant_time::verify_slices_are_equal. Add random jitter to authentication responses: sleep for random 0-10ms before returning 401/403 to make timing measurements impractical. Implement aggressive rate limiting for authentication: max 1 failed attempt per second per IP, exponential backoff after 5 failures. Use cryptographically strong random API keys with minimum 256 bits of entropy (32 bytes from rand::thread_rng()). Add monitoring alerts for repeated authentication failures from same IP.
- **STRIDE**: Spoofing
- **Testable**: Yes
- **Test Hint**: Timing test: Send 1000 authentication requests with incorrect first character vs correct first character. Measure response time distributions. Verify that distributions are statistically indistinguishable (p-value > 0.05 in t-test).

### AC-004: Database File Exfiltration via Filesystem Access

- **Severity**: 🟠 High
- **Goal**: As an attacker with compromised server access, I want to bypass API authentication entirely by directly reading SQLite database files to extract all stored data
- **Technique**: The attacker gains shell access to the server (via SSH compromise, container escape, or exploited RCE in another service). They enumerate the filesystem for SQLite database files using find /var/lib/alejandria -name '*.db'. If file permissions are misconfigured (world-readable or readable by www-data group), they use sqlite3 CLI or Python sqlite3 module to directly query the database files. They dump all tables (observations, sessions, topics) to JSON or CSV and exfiltrate via network. If databases are not encrypted at rest, all data is immediately accessible.
- **Preconditions**: Attacker has shell access to the server (any user account); Database files have overly permissive file permissions (not 600); Database encryption at rest is not enabled; Multi-instance deployments share the same Linux user account
- **Impact**: Complete data breach of all stored episodic memories and semantic knowledge without leaving audit logs (API layer is bypassed). In multi-instance deployments, single server compromise exposes all teams' data. Regulatory compliance violations if PII is stored (GDPR, HIPAA). Data can be modified directly in database, corrupting integrity. No intrusion detection at application layer.
- **Mitigation**: Enforce strict file permissions: all database files must be chmod 600 (readable/writable only by owner). Use systemd DynamicUser=yes to create ephemeral per-instance user accounts that are isolated. Store database files in instance-specific directories with parent permissions 700. Implement database encryption at rest using SQLCipher with per-instance encryption keys derived from API key or separate key management system. Add file integrity monitoring (AIDE, Tripwire) to detect unauthorized database access. Document secure deployment requirements with automated validation script that checks file permissions at startup.
- **STRIDE**: Information Disclosure
- **Testable**: Yes
- **Test Hint**: Security audit: Use ls -l to verify database file permissions are 600. Attempt to read database file as different user account. Verify access is denied with permission error.

### AC-005: JSON-RPC Injection for Unauthorized Tool Invocation

- **Severity**: 🟠 High
- **Goal**: As an attacker, I want to invoke restricted or dangerous MCP tools by exploiting JSON-RPC parsing vulnerabilities to gain unauthorized capabilities
- **Technique**: The attacker crafts malicious JSON-RPC requests with duplicate 'method' fields or deeply nested parameters to exploit parser inconsistencies. For example: {'jsonrpc':'2.0','method':'mem_search','method':'mem_delete_all','id':1}. If serde_json accepts the last duplicate and validation checks the first, the parser sees mem_search (benign) but execution receives mem_delete_all (destructive). They also test oversized array parameters: {'jsonrpc':'2.0','method':'mem_save','params':{'observations':[...10000 items...]},'id':1} to cause memory exhaustion. Tools: Burp Suite for request manipulation, custom fuzzing scripts.
- **Preconditions**: JSON-RPC parser accepts malformed requests (duplicate keys, excessive nesting); Method name validation is insufficient or uses allowlist bypass; Request size limits are not enforced before parsing; Deserialization happens before validation
- **Impact**: Unauthorized invocation of destructive operations (bulk deletes, data corruption). Memory exhaustion causing denial of service. Bypass of intended API restrictions if certain tools are supposed to be internal-only. Data integrity compromise through malformed parameter injection. Server crashes due to stack overflow from deeply nested objects.
- **Mitigation**: Configure serde_json to reject duplicate keys: use #[serde(deny_unknown_fields)] on all JSON-RPC request structs. Implement strict JSON schema validation before deserialization: max depth 10 levels, max array size 1000 elements. Use enum-based method dispatcher with exhaustive pattern matching to ensure only known tools are invoked. Add request size limit (1MB default) at HTTP layer before JSON parsing. Validate all required JSON-RPC fields (jsonrpc='2.0', method, id) and reject if missing or malformed. Add fuzzing tests to CI pipeline using cargo-fuzz with JSON-RPC payloads.
- **STRIDE**: Tampering
- **Testable**: Yes
- **Test Hint**: Fuzz test: Send JSON-RPC requests with duplicate 'method' keys. Verify server rejects with HTTP 400 and specific error code. Send deeply nested JSON (50 levels). Verify rejection before memory allocation.

### AC-006: Audit Log Manipulation to Hide Malicious Activity

- **Severity**: 🟡 Medium
- **Goal**: As an insider threat with valid access, I want to delete or modify audit logs to cover my tracks after exfiltrating sensitive data
- **Technique**: The attacker uses their legitimate API key to access and exfiltrate valuable data via mem_search queries. After exfiltration, they exploit missing audit log integrity protections. If logs are stored in plain text files or database tables without append-only constraints, they use shell access or SQL injection to modify or delete log entries corresponding to their malicious activity. They might also exploit log rotation without integrity checks to remove evidence. For example: 'DELETE FROM audit_logs WHERE method="mem_search" AND timestamp > "2026-04-01"'.
- **Preconditions**: Attacker has valid API key for legitimate access; Attacker gains shell or database access (via escalated privileges or separate vulnerability); Audit logs lack cryptographic integrity protection (signatures, append-only storage); Log storage allows modification or deletion
- **Impact**: Successful evidence destruction makes forensic investigation impossible. The organization cannot determine what data was accessed or when. Attackers can claim plausible deniability ('I never accessed that data'). Compliance violations if audit trails are required by regulations (SOX, PCI-DSS). Loss of accountability and non-repudiation guarantees.
- **Mitigation**: Implement cryptographic log signing: each audit log entry must include HMAC-SHA256 signature using a secret key. Verify signature chain on log reads. Use append-only log storage: immutable data structures, Write-Once-Read-Many filesystems, or forward logs to external SIEM that prevents deletion. Store audit logs separately from application database using different access controls. Implement log forwarding in real-time to external systems (syslog, Elasticsearch) before local storage. Add tamper detection: periodic verification of log signature chains with alerting on anomalies. Document log retention policy and implement automated compliance checks.
- **STRIDE**: Repudiation
- **Testable**: Yes
- **Test Hint**: Integrity test: Generate audit log entries, compute signature chain. Attempt to modify a log entry. Verify that signature verification detects tampering and alerts.

### AC-007: Environment Variable API Key Harvesting

- **Severity**: 🟡 Medium
- **Goal**: As an attacker with limited process inspection privileges, I want to steal API keys from environment variables to gain persistent authenticated access
- **Technique**: The attacker exploits shared hosting or container environments where process inspection is possible. On Linux, they read /proc/<pid>/environ to dump all environment variables of the Alejandria process. If ALEJANDRIA_API_KEY remains set after server initialization, it's available in plaintext. On Windows, they use Process Explorer or PowerShell Get-Process | Select-Object -ExpandProperty Environment. In container environments, they exploit Docker socket access (docker inspect) or Kubernetes ConfigMaps/Secrets misconfiguration. Tools: procfs reading, Process Explorer, kubectl get secrets.
- **Preconditions**: API key is stored in environment variable throughout server lifetime; Attacker has permission to inspect process memory/environment (same user, container escape, or privilege escalation); Server does not unset environment variable after reading; Secrets management system is not used
- **Impact**: Complete compromise of API key provides full authenticated access to the Alejandria instance. Unlike network-based attacks, this requires only local access and leaves no authentication logs. Stolen keys can be used from different IP addresses, bypassing IP-based restrictions. In container orchestration, a single vulnerability can expose all team API keys.
- **Mitigation**: Immediately unset environment variable after reading at startup: call std::env::remove_var("ALEJANDRIA_API_KEY") after loading. Store API key in secured memory structure with explicit zeroing on drop. Consider using mlock()/VirtualLock() to prevent key swapping to disk. Document migration to secrets management systems (HashiCorp Vault, AWS Secrets Manager, Kubernetes Secrets with encryption) for production. Add startup warning log if ALEJANDRIA_API_KEY is still set after initialization. Implement process memory protection flags (MADV_DONTDUMP on Linux) for key storage pages.
- **STRIDE**: Spoofing
- **Testable**: Yes
- **Test Hint**: Process inspection test: Start server with ALEJANDRIA_API_KEY set. After initialization, check /proc/<pid>/environ. Verify the variable is no longer present. Use strings command on process memory, verify API key is not in plaintext.

### AC-008: Reverse Proxy Header Spoofing for IP Whitelist Bypass

- **Severity**: 🟡 Medium
- **Goal**: As an external attacker, I want to bypass IP-based access controls by spoofing X-Forwarded-For headers to appear as a trusted internal IP address
- **Technique**: The attacker identifies that the server trusts X-Forwarded-For headers for client IP extraction (threat T-003). They send HTTP requests with spoofed headers: 'X-Forwarded-For: 127.0.0.1' or 'X-Forwarded-For: 10.0.0.1' to appear as originating from localhost or internal network. If rate limiting or IP allowlists trust these headers without validation, the attacker bypasses these controls. They can also use this to hide their true IP in audit logs, making forensics impossible. Tools: curl, Burp Suite for header manipulation.
- **Preconditions**: Server trusts X-Forwarded-For headers by default without validation; No reverse proxy configuration or misconfigured Nginx/Apache; IP-based rate limiting or allowlists rely on X-Forwarded-For; No validation of proxy IP addresses
- **Impact**: Complete bypass of IP-based security controls including rate limiting, geofencing, and IP allowlists. Attacker can perform unlimited authentication attempts by spoofing different IPs. Audit logs contain forged IP addresses, preventing accurate attribution and forensics. In multi-tenant scenarios, attackers can masquerade as different clients.
- **Mitigation**: Never trust X-Forwarded-For headers by default - make this strictly opt-in via configuration flag 'trust_proxy_headers=false' by default. When enabled, only accept these headers from configured trusted proxy IP ranges (e.g., Nginx server's IP). Use the rightmost IP in X-Forwarded-For chain as the actual client IP (leftmost IPs can be forged by clients). Validate all header values against expected IP address formats (regex, ipaddress crate). Document proper Nginx configuration: proxy_set_header X-Real-IP $remote_addr; proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for. Add integration tests that verify header spoofing is rejected when trust_proxy_headers is disabled.
- **STRIDE**: Tampering
- **Testable**: Yes
- **Test Hint**: Header spoofing test: Send request with X-Forwarded-For: 127.0.0.1 directly to server (bypassing proxy). Verify server uses actual client IP, not spoofed header. Test with trust_proxy_headers enabled/disabled.

## Mitigations Required

- [ ] Implement constant-time API key comparison (subtle::ConstantTimeEq)
- [ ] Add per-API-key and global connection limits for SSE endpoints
- [ ] Enforce strict file permissions (600) and database encryption at rest (SQLCiper)
- [ ] Implement instance identity validation for multi-tenant isolation
- [ ] Add comprehensive audit logging with cryptographic signatures
- [ ] Implement request size limits and JSON-RPC validation before parsing
- [ ] Use WAL mode for SQLite and implement write rate limiting
- [ ] Unset API key environment variables after server initialization
- [ ] Validate X-Forwarded-For headers and make trust opt-in
- [ ] Add automated security testing for SSE isolation and cross-instance access

# Proposal: http-sse-transport

**Created**: 2026-04-05T20:30:46.815Z

## Intent

Add optional HTTP/SSE transport to Alejandria MCP server for remote multi-team deployment while maintaining 100% backward compatibility with stdio transport

## Scope

- crates/alejandria-mcp/src/transport/ (new module)
- crates/alejandria-mcp/src/server.rs (refactor)
- crates/alejandria-mcp/Cargo.toml (feature flag)
- crates/alejandria-cli/src/commands/serve.rs (config support)
- config/ (new HTTP config templates)
- docs/DEPLOYMENT.md (HTTP deployment guide)

## Approach

Three-phase implementation: (1) Refactor to transport abstraction with trait-based design, (2) Implement HTTP/SSE transport behind feature flag using axum + tokio, (3) Add multi-instance deployment tooling (systemd templates, nginx configs, API key auth). Stdio remains default, HTTP is opt-in via --features http-transport.

# Project Context: Alejandria

## Description

Alejandria: Persistent memory system for AI agents via MCP. Rust workspace with SQLite backend, hybrid search (BM25+vector), and dual-memory architecture (episodic memories + semantic knowledge graphs).

## Stack

Rust 2021, SQLite, MCP Protocol

## Conventions

- Cargo workspace with 4 crates (core, storage, mcp, cli)
- Use tracing for logging
- WAL mode for SQLite
- Trait-based abstraction for extensibility
- Feature flags for optional functionality
- Comprehensive test coverage (>70%)
- Error handling with anyhow/thiserror

## Security Posture

**elevated**

Full STRIDE + OWASP Top 10 mapping — threat scenarios with attack vectors.

## Memory Backend

alejandria

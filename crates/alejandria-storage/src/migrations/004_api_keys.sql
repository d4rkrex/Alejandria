-- Migration 004: API Keys Multi-Key Support & Rotation (P0-2)
-- 
-- Purpose: Replace single API key limitation with database-backed multi-key support
-- Features: expiration, revocation, per-user tracking, audit trail
-- 
-- Created: 2026-04-12
-- Security: P0-2 (DREAD 8.2 → 2.0)

-- API Keys table for multi-key support
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY NOT NULL,            -- ULID identifier
    key_hash TEXT NOT NULL UNIQUE,           -- SHA-256 hash of API key
    username TEXT NOT NULL,                  -- User identifier (e.g., "juan.perez")
    description TEXT,                        -- Human-readable description (e.g., "Juan Pérez - Mobile")
    created_at INTEGER NOT NULL,             -- Timestamp in milliseconds since Unix epoch
    expires_at INTEGER,                      -- Expiration timestamp (NULL = never expires)
    revoked_at INTEGER,                      -- Revocation timestamp (NULL = active)
    last_used_at INTEGER,                    -- Last successful authentication timestamp
    usage_count INTEGER NOT NULL DEFAULT 0,  -- Number of successful requests
    created_by TEXT NOT NULL,                -- Admin/system that created this key
    CHECK(expires_at IS NULL OR expires_at > created_at),
    CHECK(revoked_at IS NULL OR revoked_at >= created_at)
) STRICT;

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_username ON api_keys(username);
CREATE INDEX IF NOT EXISTS idx_api_keys_active ON api_keys(revoked_at) WHERE revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_api_keys_expires ON api_keys(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_api_keys_created_at ON api_keys(created_at);

-- Optional: Migrate existing API key from environment variable
-- This creates a legacy marker that can be used during transition period
-- Actual migration happens in Rust code that reads ALEJANDRIA_API_KEY env var
INSERT OR IGNORE INTO api_keys (
    id, 
    key_hash, 
    username, 
    description, 
    created_at, 
    expires_at, 
    revoked_at, 
    last_used_at, 
    usage_count, 
    created_by
)
SELECT 
    '01H0000000000000000000LEGACY' as id,
    'MIGRATE_FROM_ENV' as key_hash,  -- Special marker - will be replaced by init code
    'system' as username,
    'Migrated from ALEJANDRIA_API_KEY environment variable' as description,
    (strftime('%s', 'now') * 1000) as created_at,
    NULL as expires_at,
    NULL as revoked_at,
    NULL as last_used_at,
    0 as usage_count,
    'system' as created_by
WHERE NOT EXISTS (SELECT 1 FROM api_keys WHERE id = '01H0000000000000000000LEGACY');

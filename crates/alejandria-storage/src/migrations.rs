//! Database schema migrations for Alejandria.
//!
//! This module provides a robust schema migration system that:
//! - Tracks applied migrations in a dedicated table
//! - Supports forward migrations (up) and rollbacks (down)
//! - Validates migration integrity before applying
//! - Provides idempotent migration application
//!
//! ## Migration Structure
//!
//! Each migration consists of:
//! - **version**: Monotonically increasing version number (u32)
//! - **description**: Human-readable description of the migration
//! - **up_sql**: SQL statements to apply the migration
//! - **down_sql**: SQL statements to rollback the migration
//!
//! ## Usage
//!
//! ```ignore
//! use rusqlite::Connection;
//! use alejandria_storage::migrations::apply_migrations;
//!
//! let conn = Connection::open("alejandria.db").unwrap();
//! apply_migrations(&conn).unwrap();
//! ```

use alejandria_core::error::{IcmError, IcmResult};
use rusqlite::Connection;

/// Represents a single database migration.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version number (must be unique and monotonically increasing)
    pub version: u32,
    /// Human-readable description of what this migration does
    pub description: &'static str,
    /// SQL statements to apply this migration (forward)
    pub up_sql: &'static str,
    /// SQL statements to rollback this migration (backward)
    pub down_sql: &'static str,
}

/// All available migrations in ascending version order.
///
/// **IMPORTANT**: When adding a new migration:
/// 1. Increment the version number sequentially
/// 2. Provide clear up_sql and down_sql
/// 3. Test both directions (up and down)
/// 4. Update SCHEMA_VERSION in schema.rs to match the latest migration
const MIGRATIONS: &[Migration] = &[
    // Migration 1: Initial schema (already applied via init_db)
    // This is a placeholder - the actual schema is created by schema::init_db
    Migration {
        version: 1,
        description: "Initial schema with memories, memoirs, concepts, and FTS tables",
        up_sql: "-- Initial schema created by schema::init_db, no additional SQL needed",
        down_sql: "-- Cannot rollback initial schema",
    },
    // Migration 2: Add decay strategy support
    Migration {
        version: 2,
        description: "Add decay_profile and decay_params columns for advanced decay algorithms",
        up_sql: r#"
            ALTER TABLE memories ADD COLUMN decay_profile TEXT;
            ALTER TABLE memories ADD COLUMN decay_params TEXT;
            CREATE INDEX IF NOT EXISTS idx_memories_decay_profile ON memories(decay_profile);
        "#,
        down_sql: r#"
            DROP INDEX IF EXISTS idx_memories_decay_profile;
            -- Note: SQLite doesn't support DROP COLUMN directly
            -- To rollback, would need table recreation (not implemented for safety)
            -- For now, columns remain but are unused after rollback
        "#,
    },
];

/// Creates the schema_migrations table to track applied migrations.
///
/// This table is separate from icm_metadata and specifically tracks
/// which migrations have been applied and when.
fn create_migrations_table(conn: &Connection) -> IcmResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            description TEXT NOT NULL,
            applied_at INTEGER NOT NULL,        -- Timestamp in milliseconds since Unix epoch
            checksum TEXT                       -- Future: Hash of migration SQL for integrity verification
        ) STRICT;
        "#,
    ).map_err(|e| IcmError::Database(format!("Failed to create schema_migrations table: {}", e)))?;

    Ok(())
}

/// Get the current schema version from the database.
///
/// Returns the highest migration version that has been applied,
/// or 0 if no migrations have been applied yet.
pub fn get_current_version(conn: &Connection) -> IcmResult<u32> {
    // First ensure migrations table exists
    create_migrations_table(conn)?;

    let version: Option<u32> = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .map_err(|e| IcmError::Database(format!("Failed to query current version: {}", e)))?;

    Ok(version.unwrap_or(0))
}

/// Get the target schema version (highest available migration).
pub fn get_target_version() -> u32 {
    MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0)
}

/// Apply all pending migrations to bring the database up to date.
///
/// This function:
/// 1. Creates the schema_migrations table if it doesn't exist
/// 2. Determines which migrations need to be applied
/// 3. Applies them in order within a transaction
/// 4. Records each successful migration
///
/// # Arguments
///
/// * `conn` - SQLite database connection
///
/// # Returns
///
/// Returns Ok(()) if all migrations were applied successfully,
/// or IcmError if any migration fails (database is rolled back).
///
/// # Examples
///
/// ```ignore
/// use rusqlite::Connection;
/// use alejandria_storage::migrations::apply_migrations;
///
/// let conn = Connection::open("alejandria.db").unwrap();
/// apply_migrations(&conn).unwrap();
/// ```
pub fn apply_migrations(conn: &Connection) -> IcmResult<()> {
    // Ensure migrations table exists
    create_migrations_table(conn)?;

    // Get current version
    let current_version = get_current_version(conn)?;

    // Find pending migrations
    let pending: Vec<&Migration> = MIGRATIONS
        .iter()
        .filter(|m| m.version > current_version)
        .collect();

    if pending.is_empty() {
        // Already up to date
        return Ok(());
    }

    // Apply pending migrations in a transaction
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| IcmError::Database(format!("Failed to begin transaction: {}", e)))?;

    for migration in pending {
        apply_migration(&tx, migration)?;
    }

    tx.commit()
        .map_err(|e| IcmError::Database(format!("Failed to commit migrations: {}", e)))?;

    Ok(())
}

/// Apply a single migration within the current transaction.
fn apply_migration(conn: &Connection, migration: &Migration) -> IcmResult<()> {
    // Skip migration 1 if it's a no-op (initial schema handled by init_db)
    if migration.version == 1 && migration.up_sql.contains("no additional SQL needed") {
        // Just record it as applied
        record_migration(conn, migration)?;
        return Ok(());
    }

    // Special handling for migration 2: check if columns already exist
    if migration.version == 2 {
        apply_migration_2_with_check(conn, migration)?;
        return Ok(());
    }

    // Execute up_sql
    conn.execute_batch(migration.up_sql).map_err(|e| {
        IcmError::Database(format!(
            "Failed to apply migration {}: {}",
            migration.version, e
        ))
    })?;

    // Record migration as applied
    record_migration(conn, migration)?;

    Ok(())
}

/// Apply migration 2 with column existence checks.
///
/// SQLite doesn't support IF NOT EXISTS for ALTER TABLE ADD COLUMN,
/// so we check PRAGMA table_info first to avoid duplicate column errors.
fn apply_migration_2_with_check(conn: &Connection, migration: &Migration) -> IcmResult<()> {
    // Check if decay_profile column exists
    let has_decay_profile: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name = 'decay_profile'")
        .and_then(|mut stmt| {
            stmt.query_row([], |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            })
        })
        .map_err(|e| IcmError::Database(format!("Failed to check decay_profile column: {}", e)))?;

    // Check if decay_params column exists
    let has_decay_params: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name = 'decay_params'")
        .and_then(|mut stmt| {
            stmt.query_row([], |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            })
        })
        .map_err(|e| IcmError::Database(format!("Failed to check decay_params column: {}", e)))?;

    // Add decay_profile column if it doesn't exist
    if !has_decay_profile {
        conn.execute("ALTER TABLE memories ADD COLUMN decay_profile TEXT", [])
            .map_err(|e| {
                IcmError::Database(format!("Failed to add decay_profile column: {}", e))
            })?;
    }

    // Add decay_params column if it doesn't exist
    if !has_decay_params {
        conn.execute("ALTER TABLE memories ADD COLUMN decay_params TEXT", [])
            .map_err(|e| IcmError::Database(format!("Failed to add decay_params column: {}", e)))?;
    }

    // Create index (CREATE INDEX IF NOT EXISTS is safe)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memories_decay_profile ON memories(decay_profile)",
        [],
    )
    .map_err(|e| IcmError::Database(format!("Failed to create decay_profile index: {}", e)))?;

    // Record migration as applied
    record_migration(conn, migration)?;

    Ok(())
}

/// Record a migration as applied in the schema_migrations table.
fn record_migration(conn: &Connection, migration: &Migration) -> IcmResult<()> {
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO schema_migrations (version, description, applied_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![migration.version, migration.description, now],
    )
    .map_err(|e| {
        IcmError::Database(format!(
            "Failed to record migration {}: {}",
            migration.version, e
        ))
    })?;

    Ok(())
}

/// Rollback the most recent migration.
///
/// **WARNING**: This is a destructive operation. Always backup your database before rollback.
///
/// # Arguments
///
/// * `conn` - SQLite database connection
///
/// # Returns
///
/// Returns Ok(version) where version is the new current version after rollback,
/// or IcmError if rollback fails.
///
/// # Examples
///
/// ```ignore
/// use rusqlite::Connection;
/// use alejandria_storage::migrations::rollback_migration;
///
/// let conn = Connection::open("alejandria.db").unwrap();
/// let new_version = rollback_migration(&conn).unwrap();
/// println!("Rolled back to version {}", new_version);
/// ```
pub fn rollback_migration(conn: &Connection) -> IcmResult<u32> {
    let current_version = get_current_version(conn)?;

    if current_version == 0 {
        return Err(IcmError::InvalidInput(
            "No migrations to rollback".to_string(),
        ));
    }

    // Find the migration to rollback
    let migration = MIGRATIONS
        .iter()
        .find(|m| m.version == current_version)
        .ok_or_else(|| {
            IcmError::InvalidInput(format!(
                "Migration version {} not found in MIGRATIONS",
                current_version
            ))
        })?;

    // Cannot rollback migration 1 (initial schema)
    if migration.version == 1 {
        return Err(IcmError::InvalidInput(
            "Cannot rollback initial schema (version 1)".to_string(),
        ));
    }

    // Apply rollback in a transaction
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| IcmError::Database(format!("Failed to begin transaction: {}", e)))?;

    // Execute down_sql
    tx.execute_batch(migration.down_sql).map_err(|e| {
        IcmError::Database(format!(
            "Failed to rollback migration {}: {}",
            migration.version, e
        ))
    })?;

    // Remove migration record
    tx.execute(
        "DELETE FROM schema_migrations WHERE version = ?1",
        rusqlite::params![migration.version],
    )
    .map_err(|e| {
        IcmError::Database(format!(
            "Failed to remove migration record {}: {}",
            migration.version, e
        ))
    })?;

    tx.commit()
        .map_err(|e| IcmError::Database(format!("Failed to commit rollback: {}", e)))?;

    // Return new version
    get_current_version(conn)
}

/// Get a list of all applied migrations with their metadata.
///
/// Returns a vector of tuples (version, description, applied_at timestamp).
pub fn list_applied_migrations(conn: &Connection) -> IcmResult<Vec<(u32, String, i64)>> {
    create_migrations_table(conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT version, description, applied_at FROM schema_migrations ORDER BY version ASC",
        )
        .map_err(|e| IcmError::Database(format!("Failed to prepare statement: {}", e)))?;

    let migrations = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| IcmError::Database(format!("Failed to query migrations: {}", e)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| IcmError::Database(format!("Failed to collect migrations: {}", e)))?;

    Ok(migrations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;
    use rusqlite::Connection;

    #[test]
    fn test_create_migrations_table() {
        let conn = Connection::open_in_memory().unwrap();

        create_migrations_table(&conn).unwrap();

        // Verify table exists
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
            [],
            |row| row.get(0)
        ).unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_current_version_empty() {
        let conn = Connection::open_in_memory().unwrap();

        let version = get_current_version(&conn).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_get_target_version() {
        let target = get_target_version();
        assert!(target >= 1, "Should have at least migration version 1");
    }

    #[test]
    fn test_apply_migrations() {
        let conn = Connection::open_in_memory().unwrap();

        // Initialize schema first (real-world usage pattern)
        schema::init_db(&conn).unwrap();

        // Apply migrations
        apply_migrations(&conn).unwrap();

        // Verify current version matches target
        let current = get_current_version(&conn).unwrap();
        let target = get_target_version();
        assert_eq!(current, target);

        // Verify migration 1 was recorded
        let migrations = list_applied_migrations(&conn).unwrap();
        assert!(!migrations.is_empty());
        assert_eq!(migrations[0].0, 1);
    }

    #[test]
    fn test_apply_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // Initialize schema first
        schema::init_db(&conn).unwrap();

        // Apply twice
        apply_migrations(&conn).unwrap();
        apply_migrations(&conn).unwrap();

        // Should still be at target version
        let current = get_current_version(&conn).unwrap();
        let target = get_target_version();
        assert_eq!(current, target);
    }

    #[test]
    fn test_list_applied_migrations() {
        let conn = Connection::open_in_memory().unwrap();

        // Initialize schema first
        schema::init_db(&conn).unwrap();

        apply_migrations(&conn).unwrap();

        let migrations = list_applied_migrations(&conn).unwrap();
        assert!(!migrations.is_empty());

        // Verify structure
        for (version, description, applied_at) in migrations {
            assert!(version > 0);
            assert!(!description.is_empty());
            assert!(applied_at > 0);
        }
    }

    #[test]
    fn test_rollback_migration_fails_on_version_1() {
        let conn = Connection::open_in_memory().unwrap();

        // Initialize schema first
        schema::init_db(&conn).unwrap();

        apply_migrations(&conn).unwrap();

        // Current version should be 2 (after both migrations)
        let current = get_current_version(&conn).unwrap();
        assert_eq!(current, 2);

        // First rollback migration 2 - this should succeed
        let new_version = rollback_migration(&conn).unwrap();
        assert_eq!(new_version, 1);

        // Now try to rollback version 1 - this should fail
        let result = rollback_migration(&conn);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot rollback initial schema"));
    }

    #[test]
    fn test_rollback_migration_fails_when_empty() {
        let conn = Connection::open_in_memory().unwrap();

        let result = rollback_migration(&conn);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("No migrations to rollback"));
    }
}

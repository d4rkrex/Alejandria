# Schema Migrations Guide

This document explains how to create and apply database schema migrations in Alejandria.

## Overview

Alejandria uses a robust schema migration system that:
- Tracks applied migrations in the `schema_migrations` table
- Supports forward migrations (up) and rollbacks (down)
- Validates migration integrity before applying
- Provides idempotent migration application

## Migration Structure

Each migration consists of:
- **version**: Monotonically increasing version number (u32)
- **description**: Human-readable description of the migration
- **up_sql**: SQL statements to apply the migration
- **down_sql**: SQL statements to rollback the migration

## How Migrations Work

1. When you open a `SqliteStore`, the system:
   - Creates the `schema_migrations` table if it doesn't exist
   - Checks which migrations have been applied
   - Applies any pending migrations in order
   - Records each successful migration

2. Migrations are applied within a transaction, so if any migration fails, the database is rolled back to its previous state.

## Creating a New Migration

To add a new migration:

1. Open `crates/alejandria-storage/src/migrations.rs`
2. Add your migration to the `MIGRATIONS` array:

```rust
Migration {
    version: 2, // Increment version sequentially
    description: "Add user_tags column to memories table",
    up_sql: r#"
        ALTER TABLE memories ADD COLUMN user_tags TEXT NOT NULL DEFAULT '[]';
        CREATE INDEX idx_memories_user_tags ON memories(user_tags);
    "#,
    down_sql: r#"
        DROP INDEX IF EXISTS idx_memories_user_tags;
        -- Note: SQLite doesn't support DROP COLUMN directly
        -- For production, implement proper column removal via table recreation
    "#,
},
```

3. Update `SCHEMA_VERSION` in `crates/alejandria-storage/src/schema.rs` to match the new version
4. Test both directions (up and down) thoroughly
5. Run tests: `cargo test --lib --tests --all-features`

## Important Notes

### SQLite Limitations

SQLite has limited ALTER TABLE support:
- You CAN add columns with ALTER TABLE
- You CANNOT drop columns directly
- To remove a column, you must recreate the table

Example of proper column removal:
```sql
-- Create new table without the column
CREATE TABLE memories_new AS SELECT id, topic, summary, ... FROM memories;

-- Drop old table
DROP TABLE memories;

-- Rename new table
ALTER TABLE memories_new RENAME TO memories;

-- Recreate indexes
CREATE INDEX ...
```

### Version Numbers

- Version numbers must be unique and monotonically increasing
- Never reuse a version number
- Never modify an already-applied migration
- If you need to fix a migration, create a new one

### Testing Migrations

Always test your migrations:
1. Test the up migration on a fresh database
2. Test the down migration (rollback)
3. Test applying migrations multiple times (idempotency)
4. Test with production-like data

### Rollback Considerations

- Rollbacks are destructive operations
- Always backup your database before rolling back
- Cannot rollback migration version 1 (initial schema)
- Test rollback SQL thoroughly before deploying

## Using the Migration API

### Apply All Pending Migrations

```rust
use alejandria_storage::migrations::apply_migrations;
use rusqlite::Connection;

let conn = Connection::open("alejandria.db")?;
apply_migrations(&conn)?;
```

This is automatically called when you open a `SqliteStore`:
```rust
use alejandria_storage::SqliteStore;

let store = SqliteStore::open("alejandria.db")?;
// Migrations are automatically applied
```

### Check Current Version

```rust
use alejandria_storage::migrations::{get_current_version, get_target_version};

let current = get_current_version(&conn)?;
let target = get_target_version();
println!("Database is at version {}, latest is {}", current, target);
```

### List Applied Migrations

```rust
use alejandria_storage::migrations::list_applied_migrations;

let migrations = list_applied_migrations(&conn)?;
for (version, description, applied_at) in migrations {
    println!("Migration {} ({}): applied at {}", version, description, applied_at);
}
```

### Rollback a Migration

⚠️ **WARNING**: This is destructive. Backup your database first!

```rust
use alejandria_storage::migrations::rollback_migration;

let new_version = rollback_migration(&conn)?;
println!("Rolled back to version {}", new_version);
```

## Migration Best Practices

1. **Keep migrations small**: One logical change per migration
2. **Test thoroughly**: Test both up and down directions
3. **Document breaking changes**: Add comments explaining impact
4. **Preserve data**: Never drop columns with user data without migration path
5. **Use transactions**: Migrations are already wrapped in transactions
6. **Version control**: Commit migrations with the code that uses them
7. **Backward compatibility**: Consider adding columns as nullable or with defaults

## Troubleshooting

### Migration Failed During Apply

If a migration fails:
1. Check the error message for SQL syntax errors
2. Fix the migration SQL
3. The database was rolled back automatically (no partial state)
4. Re-run the migration

### Migration Version Mismatch

If `SCHEMA_VERSION` doesn't match the latest migration version:
1. Update `SCHEMA_VERSION` in `schema.rs`
2. Rebuild: `cargo build`

### Cannot Rollback Migration

If rollback fails:
1. Check the `down_sql` for errors
2. Ensure you're not trying to rollback version 1
3. Backup and manually fix the database if necessary

## Future Enhancements

Planned improvements to the migration system:
- [ ] Migration checksum validation
- [ ] Dry-run mode for testing migrations
- [ ] SQL script file support (instead of inline strings)
- [ ] Migration dependency tracking
- [ ] Automatic backup before migration
- [ ] Migration history export/import

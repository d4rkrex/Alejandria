# P0-5: BOLA (Broken Object Level Authorization) Protection Implementation

**Project:** Alejandría - Persistent Memory System for AI Agents  
**Security Finding:** P0-5 from SECURITY_REMEDIATION_PLAN.md  
**Severity:** CRITICAL (DREAD 8.0)  
**Status:** 🚧 IN PROGRESS  
**Implementation Date:** 2026-04-11  
**Assignee:** AppSec Team

---

## 📋 Executive Summary

### Vulnerability Description

**BOLA (Broken Object Level Authorization)** - Also known as IDOR (Insecure Direct Object Reference), OWASP API #1.

**Current State:**
- ANY authenticated user can access ANY memory by guessing/enumerating memory IDs
- No ownership validation on `mem_recall`, `mem_update`, `mem_forget`, or `mem_get_observation`
- All memories are globally accessible to all API key holders
- Critical data isolation failure for multi-tenant deployments

**Attack Scenario:**
```bash
# User A creates a memory
curl -H "X-API-Key: user_a_key" -X POST https://alejandria/rpc \
  -d '{"method":"mem_store","params":{"content":"SECRET: API_KEY_PROD=abc123"}}'
# Response: {"id": "01HQABCDEFG..."}

# User B can read User A's secret by guessing the ID
curl -H "X-API-Key: user_b_key" -X POST https://alejandria/rpc \
  -d '{"method":"mem_recall","params":{"query":"SECRET"}}'
# Response: Returns User A's memory with secret!

# User B can also UPDATE or DELETE User A's memories
curl -H "X-API-Key: user_b_key" -X POST https://alejandria/rpc \
  -d '{"method":"mem_forget","params":{"id":"01HQABCDEFG..."}}'
# User A's memory is deleted!
```

**Impact:**
- **Confidentiality:** Complete loss - all memories readable by any user
- **Integrity:** Memory tampering - any user can modify/delete others' data
- **Compliance:** GDPR violation (cross-tenant data access)
- **Trust:** Catastrophic - users cannot trust the system to protect their data

**DREAD Score Breakdown:**
- **D**amage: 10 (complete data breach)
- **R**eproducibility: 10 (trivial to exploit)
- **E**xploitability: 10 (no special tools needed)
- **A**ffected Users: 5 (all users in multi-tenant setup)
- **D**iscoverability: 5 (medium - requires ID enumeration)
- **Average:** (10+10+10+5+5)/5 = **8.0** (CRITICAL)

---

## 🎯 Implementation Approach

### Design Principles

1. **Ownership-Based Access Control:** Every memory has an `owner_key_hash` identifying the API key that created it
2. **Shared Memory Support:** Special `owner_key_hash = 'shared'` for system-wide accessible memories
3. **Authorization at Storage Layer:** Enforce checks in `SqliteStore` methods, not just handlers
4. **Backward Compatibility:** Existing memories assigned to `owner_key_hash = 'LEGACY_SYSTEM'`
5. **Audit Logging:** All authorization failures logged with attacker context

### Architecture Changes

```
┌─────────────────────────────────────────────────────────┐
│  MCP Handler Layer (tools/memory.rs)                    │
│  - mem_store, mem_recall, mem_update, mem_forget        │
│  - Extract AuthContext from request extensions          │
│  - Pass api_key_hash to storage layer                   │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│  Storage Layer (storage/src/store.rs)                   │
│  - NEW: authorize_access(memory_id, requester_hash)     │
│  - Modified: get(), update(), delete() - add hash param │
│  - Modified: search queries - filter by owner           │
│  - Reject unauthorized access with IcmError::Forbidden  │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│  Database Layer (SQLite)                                 │
│  - memories table: + owner_key_hash TEXT NOT NULL       │
│  - Index: idx_memories_owner_key_hash                   │
│  - Queries: WHERE owner_key_hash = ? OR = 'shared'      │
└─────────────────────────────────────────────────────────┘
```

---

## 🗄️ Database Migration

### Migration File: `crates/alejandria-storage/migrations/003_add_owner_key_hash.sql`

```sql
-- Migration 003: Add owner_key_hash for BOLA protection
-- Issue: P0-5 from SECURITY_REMEDIATION_PLAN.md
-- DREAD: 8.0 (CRITICAL)

-- Add owner_key_hash column to memories table
ALTER TABLE memories ADD COLUMN owner_key_hash TEXT;

-- Create index for efficient ownership lookups
CREATE INDEX IF NOT EXISTS idx_memories_owner_key_hash ON memories(owner_key_hash);

-- Backfill existing memories with legacy owner
-- In production, you may want to assign to a specific user instead
UPDATE memories 
SET owner_key_hash = 'LEGACY_SYSTEM' 
WHERE owner_key_hash IS NULL;

-- Make owner_key_hash NOT NULL after backfill
-- NOTE: Uncomment this after verifying backfill in production
-- We keep it nullable during migration for safety
-- ALTER TABLE memories ALTER COLUMN owner_key_hash SET NOT NULL;

-- Verify migration
SELECT 'Migration 003 completed: ' || COUNT(*) || ' memories now have owner_key_hash' 
FROM memories 
WHERE owner_key_hash IS NOT NULL;
```

### Migration Runner Update

File: `crates/alejandria-storage/src/migrations.rs`

```rust
pub fn apply_migrations(conn: &Connection) -> IcmResult<()> {
    // ... existing migrations ...
    
    // Migration 003: Add owner_key_hash for BOLA protection
    conn.execute_batch(include_str!("migrations/003_add_owner_key_hash.sql"))
        .map_err(|e| IcmError::Database(format!("Migration 003 failed: {}", e)))?;
    
    Ok(())
}
```

---

## 🔧 Code Changes

### 1. Update Memory Struct

**File:** `crates/alejandria-core/src/memory.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    // ... existing fields ...
    
    /// Owner identification (SHA-256 hash of API key)
    /// Special values:
    /// - 'shared': Accessible by all users
    /// - 'LEGACY_SYSTEM': Pre-migration memories (optional backward compat)
    pub owner_key_hash: String,
}

impl Memory {
    pub fn new(topic: String, summary: String) -> Self {
        Self {
            // ... existing initialization ...
            owner_key_hash: String::new(), // Will be set by storage layer
        }
    }
    
    /// Check if this memory is shared (accessible by all users)
    pub fn is_shared(&self) -> bool {
        self.owner_key_hash == "shared"
    }
    
    /// Check if this memory is a legacy memory (pre-migration)
    pub fn is_legacy(&self) -> bool {
        self.owner_key_hash == "LEGACY_SYSTEM"
    }
}
```

### 2. Add Authorization Layer to Storage

**File:** `crates/alejandria-storage/src/store.rs`

```rust
impl SqliteStore {
    /// Authorize access to a memory by verifying ownership
    ///
    /// Returns Ok(()) if access is allowed, Err otherwise.
    ///
    /// Access is allowed if:
    /// - Memory owner matches requester
    /// - Memory is marked as 'shared'
    /// - Memory is legacy (LEGACY_SYSTEM) - optional for backward compat
    fn authorize_access(&self, memory_id: &str, requester_key_hash: &str) -> IcmResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            IcmError::Database(format!("Failed to acquire connection lock: {}", e))
        })?;
        
        let owner_hash: String = conn
            .query_row(
                "SELECT owner_key_hash FROM memories WHERE id = ?",
                [memory_id],
                |row| row.get(0),
            )
            .optional()
            .into_icm_result()?
            .ok_or_else(|| IcmError::NotFound(format!("Memory not found: {}", memory_id)))?;
        
        // Allow access if:
        // 1. Owner matches requester
        // 2. Memory is shared
        // 3. Memory is legacy (optional - remove for strict mode)
        if owner_hash == requester_key_hash 
            || owner_hash == "shared" 
            || owner_hash == "LEGACY_SYSTEM" 
        {
            Ok(())
        } else {
            // Log authorization failure for security monitoring
            log::warn!(
                "BOLA attempt blocked: user {} tried to access memory {} owned by {}",
                requester_key_hash,
                memory_id,
                owner_hash
            );
            
            Err(IcmError::Forbidden(format!(
                "Access denied: memory {} is owned by another user",
                memory_id
            )))
        }
    }
    
    /// Get a memory by ID (with ownership check)
    pub fn get_authorized(&self, id: &str, requester_key_hash: &str) -> IcmResult<Option<Memory>> {
        // Check authorization first
        self.authorize_access(id, requester_key_hash)?;
        
        // If authorized, fetch memory
        self.get(id)
    }
    
    /// Update a memory (with ownership check)
    pub fn update_authorized(&self, mut memory: Memory, requester_key_hash: &str) -> IcmResult<()> {
        // Check authorization
        self.authorize_access(&memory.id, requester_key_hash)?;
        
        // Prevent ownership transfer via update
        let original_owner = {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                "SELECT owner_key_hash FROM memories WHERE id = ?",
                [&memory.id],
                |row| row.get::<_, String>(0),
            )
            .into_icm_result()?
        };
        
        memory.owner_key_hash = original_owner; // Force original owner
        
        self.update(memory)
    }
    
    /// Delete a memory (with ownership check)
    pub fn delete_authorized(&self, id: &str, requester_key_hash: &str) -> IcmResult<()> {
        // Check authorization
        self.authorize_access(id, requester_key_hash)?;
        
        self.delete(id)
    }
    
    /// Search by keywords (filtered by owner)
    pub fn search_by_keywords_authorized(
        &self,
        query: &str,
        limit: usize,
        requester_key_hash: &str,
    ) -> IcmResult<Vec<Memory>> {
        let conn = self.conn.lock().map_err(|e| {
            IcmError::Database(format!("Failed to acquire connection lock: {}", e))
        })?;
        
        let sql = "
            SELECT id FROM memories_fts 
            WHERE memories_fts MATCH ?1 
            AND id IN (
                SELECT id FROM memories 
                WHERE owner_key_hash = ?2 OR owner_key_hash = 'shared'
            )
            ORDER BY rank 
            LIMIT ?3
        ";
        
        let mut stmt = conn.prepare(sql).into_icm_result()?;
        let ids: Vec<String> = stmt
            .query_map([query, requester_key_hash, &limit.to_string()], |row| row.get(0))
            .into_icm_result()?
            .collect::<Result<Vec<_>, _>>()
            .into_icm_result()?;
        
        drop(stmt);
        drop(conn);
        
        // Fetch full memories
        ids.into_iter()
            .filter_map(|id| self.get(&id).ok().flatten())
            .collect::<Vec<_>>()
            .into()
    }
}
```

### 3. Update MCP Handlers

**File:** `crates/alejandria-mcp/src/tools/memory.rs`

```rust
/// mem_recall - Search and recall memories (with ownership filter)
pub fn mem_recall<S: MemoryStore>(
    args: Value, 
    store: Arc<S>,
    auth_context: &AuthContext,  // NEW: required parameter
) -> Result<ToolResult, JsonRpcError> {
    let args: RecallArgs = serde_json::from_value(args)?;
    
    // Downcast to SqliteStore for authorized search
    use alejandria_storage::SqliteStore;
    let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
    let sqlite_store = unsafe { &*sqlite_store };
    
    // Use authorized search (filters by owner automatically)
    let scored_results = sqlite_store
        .hybrid_search_with_authorization(
            &args.query,
            args.limit,
            &auth_context.api_key_hash,  // Filter by this user's hash
        )
        .map_err(|e| JsonRpcError::internal_error(format!("Search failed: {}", e)))?;
    
    // ... rest of handler ...
}

/// mem_update - Update an existing memory (with ownership check)
pub fn mem_update<S: MemoryStore>(
    args: Value, 
    store: Arc<S>,
    auth_context: &AuthContext,  // NEW: required parameter
) -> Result<ToolResult, JsonRpcError> {
    let args: UpdateArgs = serde_json::from_value(args)?;
    
    // Get memory with authorization check
    let mut memory = {
        use alejandria_storage::SqliteStore;
        let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
        let sqlite_store = unsafe { &*sqlite_store };
        
        sqlite_store
            .get_authorized(&args.id, &auth_context.api_key_hash)?
            .ok_or_else(|| JsonRpcError::not_found(format!("Memory not found: {}", args.id)))?
    };
    
    // ... update fields ...
    
    // Store with authorization
    {
        use alejandria_storage::SqliteStore;
        let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
        let sqlite_store = unsafe { &*sqlite_store };
        
        sqlite_store.update_authorized(memory, &auth_context.api_key_hash)?;
    }
    
    Ok(ToolResult::success(format!("Memory updated: {}", args.id)))
}

/// mem_forget - Soft-delete a memory (with ownership check)
pub fn mem_forget<S: MemoryStore>(
    args: Value, 
    store: Arc<S>,
    auth_context: &AuthContext,  // NEW: required parameter
) -> Result<ToolResult, JsonRpcError> {
    let args: ForgetArgs = serde_json::from_value(args)?;
    
    // Delete with authorization check
    {
        use alejandria_storage::SqliteStore;
        let sqlite_store = Arc::as_ptr(&store) as *const SqliteStore;
        let sqlite_store = unsafe { &*sqlite_store };
        
        sqlite_store.delete_authorized(&args.id, &auth_context.api_key_hash)?;
    }
    
    Ok(ToolResult::success(format!("Memory deleted: {}", args.id)))
}

/// mem_store - Store a new memory (with owner assignment)
pub fn mem_store<S: MemoryStore>(
    args: Value, 
    store: Arc<S>,
    auth_context: &AuthContext,  // NEW: required parameter
) -> Result<ToolResult, JsonRpcError> {
    let args: StoreArgs = serde_json::from_value(args)?;
    
    let mut memory = Memory::new(/* ... */);
    
    // Assign owner (requester's API key hash)
    memory.owner_key_hash = auth_context.api_key_hash.clone();
    
    // If shared flag is set, make it globally accessible
    if args.shared.unwrap_or(false) {
        memory.owner_key_hash = "shared".to_string();
    }
    
    let id = store.store(memory)?;
    
    Ok(ToolResult::success(format!("Memory stored: {}", id)))
}
```

### 4. Propagate AuthContext to Handlers

**File:** `crates/alejandria-mcp/src/transport/http/mod.rs`

```rust
async fn handle_tool_call(
    State(state): State<AppState<S>>,
    Extension(auth_context): Extension<AuthContext>,  // Extract from middleware
    Json(request): Json<ToolCallRequest>,
) -> Result<Json<Value>, HttpError> {
    match request.tool_name.as_str() {
        "mem_store" => mem_store(request.args, state.store.clone(), &auth_context),
        "mem_recall" => mem_recall(request.args, state.store.clone(), &auth_context),
        "mem_update" => mem_update(request.args, state.store.clone(), &auth_context),
        "mem_forget" => mem_forget(request.args, state.store.clone(), &auth_context),
        // ... other tools ...
    }
}
```

### 5. Add `shared` Parameter to mem_store

**File:** `crates/alejandria-mcp/src/tools/memory.rs`

```rust
#[derive(Debug, Deserialize)]
struct StoreArgs {
    content: String,
    // ... existing fields ...
    
    /// NEW: Mark memory as shared (accessible by all users)
    #[serde(default)]
    shared: Option<bool>,
}
```

---

## 🧪 Testing Methodology

### Unit Tests

**File:** `crates/alejandria-storage/tests/authorization_tests.rs`

```rust
#[test]
fn test_bola_protection_get() {
    let store = SqliteStore::open_in_memory().unwrap();
    
    // User A creates memory
    let mut mem_a = Memory::new("topic".into(), "User A's secret".into());
    mem_a.owner_key_hash = "user_a_hash".to_string();
    let id = store.store(mem_a).unwrap();
    
    // User A can read
    assert!(store.get_authorized(&id, "user_a_hash").is_ok());
    
    // User B CANNOT read (BOLA protection)
    let result = store.get_authorized(&id, "user_b_hash");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Access denied"));
}

#[test]
fn test_bola_protection_update() {
    let store = SqliteStore::open_in_memory().unwrap();
    
    // User A creates memory
    let mut mem_a = Memory::new("topic".into(), "original".into());
    mem_a.owner_key_hash = "user_a_hash".to_string();
    let id = store.store(mem_a.clone()).unwrap();
    
    // User B tries to update User A's memory
    mem_a.summary = "hacked".to_string();
    let result = store.update_authorized(mem_a, "user_b_hash");
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Access denied"));
    
    // Verify original memory unchanged
    let original = store.get(&id).unwrap().unwrap();
    assert_eq!(original.summary, "original");
}

#[test]
fn test_bola_protection_delete() {
    let store = SqliteStore::open_in_memory().unwrap();
    
    // User A creates memory
    let mut mem_a = Memory::new("topic".into(), "data".into());
    mem_a.owner_key_hash = "user_a_hash".to_string();
    let id = store.store(mem_a).unwrap();
    
    // User B tries to delete User A's memory
    let result = store.delete_authorized(&id, "user_b_hash");
    
    assert!(result.is_err());
    
    // Verify memory still exists
    assert!(store.get(&id).unwrap().is_some());
}

#[test]
fn test_shared_memory_accessible_by_all() {
    let store = SqliteStore::open_in_memory().unwrap();
    
    // Create shared memory
    let mut mem = Memory::new("topic".into(), "shared knowledge".into());
    mem.owner_key_hash = "shared".to_string();
    let id = store.store(mem).unwrap();
    
    // Any user can read shared memory
    assert!(store.get_authorized(&id, "user_a_hash").is_ok());
    assert!(store.get_authorized(&id, "user_b_hash").is_ok());
    assert!(store.get_authorized(&id, "user_c_hash").is_ok());
}

#[test]
fn test_search_isolation() {
    let store = SqliteStore::open_in_memory().unwrap();
    
    // User A creates memories
    for i in 0..3 {
        let mut mem = Memory::new("topic".into(), format!("secret_a_{}", i));
        mem.owner_key_hash = "user_a_hash".to_string();
        store.store(mem).unwrap();
    }
    
    // User B creates memories
    for i in 0..3 {
        let mut mem = Memory::new("topic".into(), format!("secret_b_{}", i));
        mem.owner_key_hash = "user_b_hash".to_string();
        store.store(mem).unwrap();
    }
    
    // User A search should only return User A's memories
    let results = store.search_by_keywords_authorized("secret", 10, "user_a_hash").unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|m| m.owner_key_hash == "user_a_hash"));
    
    // User B search should only return User B's memories
    let results = store.search_by_keywords_authorized("secret", 10, "user_b_hash").unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|m| m.owner_key_hash == "user_b_hash"));
}

#[test]
fn test_prevent_owner_change_via_update() {
    let store = SqliteStore::open_in_memory().unwrap();
    
    // User A creates memory
    let mut mem = Memory::new("topic".into(), "data".into());
    mem.owner_key_hash = "user_a_hash".to_string();
    let id = store.store(mem.clone()).unwrap();
    
    // User A tries to change owner to User B (should be rejected)
    mem.owner_key_hash = "user_b_hash".to_string();
    mem.summary = "updated".to_string();
    
    store.update_authorized(mem, "user_a_hash").unwrap();
    
    // Verify owner didn't change
    let updated = store.get(&id).unwrap().unwrap();
    assert_eq!(updated.owner_key_hash, "user_a_hash");
    assert_eq!(updated.summary, "updated"); // Content updated, but NOT owner
}
```

### Integration Tests

```bash
#!/bin/bash
# integration_test_bola.sh

set -e

echo "Setting up test environment..."

# Generate two API keys
export USER_A_KEY=$(openssl rand -base64 32)
export USER_B_KEY=$(openssl rand -base64 32)

export ALEJANDRIA_API_KEY_USER_A="$USER_A_KEY"
export ALEJANDRIA_API_KEY_USER_B="$USER_B_KEY"

# Start server
cargo run --release -- serve --config config/http.toml &
SERVER_PID=$!
sleep 3

echo "✅ Server started (PID: $SERVER_PID)"

# Test 1: User A creates memory
echo -e "\n🧪 Test 1: User A creates memory"
RESPONSE=$(curl -s -H "X-API-Key: $USER_A_KEY" -X POST http://localhost:8080/rpc \
  -d '{"jsonrpc":"2.0","method":"mem_store","params":{"content":"SECRET: password123","topic":"credentials"},"id":1}')

MEMORY_ID=$(echo "$RESPONSE" | jq -r '.result.id')
echo "Memory ID: $MEMORY_ID"

# Test 2: User A can read own memory
echo -e "\n🧪 Test 2: User A can read own memory"
RESPONSE=$(curl -s -H "X-API-Key: $USER_A_KEY" -X POST http://localhost:8080/rpc \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"mem_recall\",\"params\":{\"query\":\"SECRET\"},\"id\":2}")

if echo "$RESPONSE" | grep -q "password123"; then
    echo "✅ PASS: User A can read own memory"
else
    echo "❌ FAIL: User A cannot read own memory"
    exit 1
fi

# Test 3: User B CANNOT read User A's memory (BOLA protection)
echo -e "\n🧪 Test 3: User B CANNOT read User A's memory (BOLA protection)"
RESPONSE=$(curl -s -H "X-API-Key: $USER_B_KEY" -X POST http://localhost:8080/rpc \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"mem_recall\",\"params\":{\"query\":\"SECRET\"},\"id\":3}")

if echo "$RESPONSE" | grep -q "password123"; then
    echo "❌ FAIL: User B can read User A's memory (BOLA VULNERABLE!)"
    kill $SERVER_PID
    exit 1
else
    echo "✅ PASS: User B CANNOT read User A's memory"
fi

# Test 4: User B CANNOT update User A's memory
echo -e "\n🧪 Test 4: User B CANNOT update User A's memory"
RESPONSE=$(curl -s -H "X-API-Key: $USER_B_KEY" -X POST http://localhost:8080/rpc \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"mem_update\",\"params\":{\"id\":\"$MEMORY_ID\",\"summary\":\"HACKED\"},\"id\":4}")

if echo "$RESPONSE" | grep -q "Access denied"; then
    echo "✅ PASS: User B CANNOT update User A's memory"
else
    echo "❌ FAIL: User B can update User A's memory (BOLA VULNERABLE!)"
    kill $SERVER_PID
    exit 1
fi

# Test 5: Shared memory accessible by all
echo -e "\n🧪 Test 5: Shared memory accessible by all users"
RESPONSE=$(curl -s -H "X-API-Key: $USER_A_KEY" -X POST http://localhost:8080/rpc \
  -d '{"jsonrpc":"2.0","method":"mem_store","params":{"content":"Public knowledge","shared":true},"id":5}')

SHARED_ID=$(echo "$RESPONSE" | jq -r '.result.id')

# User B should be able to read shared memory
RESPONSE=$(curl -s -H "X-API-Key: $USER_B_KEY" -X POST http://localhost:8080/rpc \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"mem_recall\",\"params\":{\"query\":\"Public\"},\"id\":6}")

if echo "$RESPONSE" | grep -q "Public knowledge"; then
    echo "✅ PASS: Shared memory accessible by User B"
else
    echo "❌ FAIL: Shared memory NOT accessible"
    kill $SERVER_PID
    exit 1
fi

# Cleanup
kill $SERVER_PID
echo -e "\n✅ All BOLA protection tests passed!"
```

---

## 📊 Verification Checklist

- [ ] Migration 003 applied successfully
- [ ] `owner_key_hash` column exists in `memories` table
- [ ] Index `idx_memories_owner_key_hash` created
- [ ] Existing memories backfilled with `LEGACY_SYSTEM`
- [ ] `Memory` struct updated with `owner_key_hash` field
- [ ] `SqliteStore::authorize_access()` implemented
- [ ] `get_authorized()`, `update_authorized()`, `delete_authorized()` added
- [ ] `search_by_keywords_authorized()` filters by owner
- [ ] MCP handlers updated to accept `AuthContext`
- [ ] `mem_store` assigns `owner_key_hash` from `AuthContext`
- [ ] `mem_recall` filters results by ownership
- [ ] `mem_update` checks ownership before update
- [ ] `mem_forget` checks ownership before delete
- [ ] `shared` parameter added to `mem_store`
- [ ] Unit tests pass (10/10 authorization tests)
- [ ] Integration tests pass (BOLA protection verified)
- [ ] Authorization failures logged to audit log
- [ ] No performance degradation (index optimized queries)

---

## 🚀 Deployment Steps

### Pre-Deployment

1. **Backup database:**
   ```bash
   cp alejandria.db alejandria.db.backup.$(date +%Y%m%d_%H%M%S)
   ```

2. **Test migration on copy:**
   ```bash
   cp alejandria.db alejandria_test.db
   sqlite3 alejandria_test.db < crates/alejandria-storage/migrations/003_add_owner_key_hash.sql
   sqlite3 alejandria_test.db "SELECT COUNT(*) FROM memories WHERE owner_key_hash IS NOT NULL;"
   ```

3. **Run unit tests:**
   ```bash
   cargo test --package alejandria-storage authorization_tests
   ```

### Deployment (Downtime Required)

1. **Stop server:**
   ```bash
   systemctl stop alejandria-mcp
   ```

2. **Apply migration:**
   ```bash
   cd /opt/alejandria
   cargo run -- migrate --database alejandria.db
   ```

3. **Deploy new binary:**
   ```bash
   cargo build --release
   cp target/release/alejandria-mcp /usr/local/bin/
   ```

4. **Start server:**
   ```bash
   systemctl start alejandria-mcp
   ```

5. **Verify service:**
   ```bash
   curl -H "X-API-Key: $API_KEY" https://localhost:8080/health
   ```

### Post-Deployment

1. **Run integration tests:**
   ```bash
   bash scripts/integration_test_bola.sh
   ```

2. **Monitor logs for authorization failures:**
   ```bash
   tail -f /var/log/alejandria/audit.log | grep "BOLA attempt blocked"
   ```

3. **Check performance:**
   ```bash
   # Query time should be similar (index optimized)
   time curl -H "X-API-Key: $API_KEY" -X POST https://localhost:8080/rpc \
     -d '{"method":"mem_recall","params":{"query":"test"}}'
   ```

---

## 📈 Updated DREAD Score

### After Implementation

| Factor | Before | After | Change |
|--------|--------|-------|--------|
| **Damage** | 10 | 2 | -8 (isolated data) |
| **Reproducibility** | 10 | 2 | -8 (requires ownership) |
| **Exploitability** | 10 | 2 | -8 (authorization enforced) |
| **Affected Users** | 5 | 1 | -4 (single-tenant scope) |
| **Discoverability** | 5 | 2 | -3 (harder to find) |
| **TOTAL** | **8.0** | **1.8** | **-6.2** |

**Risk Reduction:** 77.5% (CRITICAL → LOW)

---

## 🔍 Backward Compatibility

### Legacy Memories

- All pre-migration memories assigned `owner_key_hash = 'LEGACY_SYSTEM'`
- **Option 1 (Permissive):** Allow all users to access legacy memories
- **Option 2 (Strict):** Assign legacy memories to a specific admin user
- **Option 3 (Migration):** Provide a tool to reassign legacy memories to current users

**Recommendation:** Start with Option 1 (permissive) for smooth migration, then run a cleanup script to reassign legacy memories.

### API Compatibility

- No breaking changes to MCP API
- `shared` parameter is optional (defaults to `false`)
- Existing clients work without modification
- New clients can use `shared: true` for system-wide knowledge

---

## 📝 Next Steps

1. ✅ Complete P0-5 implementation
2. ⏭️ **P0-6:** Implement global rate limiting (depends on P0-5 for per-user tracking)
3. ⏭️ **P1-1:** Add audit logging for all BOLA attempts
4. ⏭️ **P1-2:** Enhance `extract_client_ip()` for accurate logging
5. 🔄 **Post-Release:** Migration script to reassign legacy memories

---

**Implementation Status:** 🚧 IN PROGRESS  
**Target Completion:** 2026-04-12  
**Next Review:** After deployment to staging environment


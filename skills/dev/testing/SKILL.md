---
name: alejandria-testing
description: >
  TDD and coverage standards for Alejandría.
  Trigger: Adding features, fixing bugs, refactoring, or any behavior change.
license: MIT
metadata:
  author: appsec-team
  version: "1.0"
  project: alejandria
---

## When to Use

Use this skill when:
- Implementing new features
- Fixing bugs
- Refactoring existing code
- Adding security mitigations
- Modifying business logic
- Changing data structures

---

## TDD Loop (MANDATORY)

Follow this loop for ALL behavior changes:

```
1. RED:    Write a failing test for the target behavior
2. GREEN:  Implement the smallest code to make test pass
3. REFACTOR: Clean up while keeping tests green
4. EDGE:   Add error path and edge case tests
```

### Example Flow
```rust
// 1. RED - Write failing test
#[test]
fn test_memory_search_with_special_chars() {
    let store = create_test_store();
    let result = store.search("test@example.com");
    assert!(result.is_ok());
}

// 2. GREEN - Make it pass (minimal)
pub fn search(&self, query: &str) -> Result<Vec<Memory>> {
    // Handle special chars
    let sanitized = query.replace("@", "");
    // ... rest of implementation
}

// 3. REFACTOR - Clean up
pub fn search(&self, query: &str) -> Result<Vec<Memory>> {
    let sanitized = sanitize_fts5_query(query);
    self.execute_search(&sanitized)
}

// 4. EDGE - Add error cases
#[test]
fn test_search_with_sql_injection() {
    // Test SQL injection prevention
}
```

---

## Coverage Rules (MANDATORY)

### Minimum Requirements
- **70% coverage** for new code (enforced in CI)
- **90% coverage** for critical paths (auth, data storage, security)
- **100% coverage** for security fixes

### What to Cover
1. **Happy path** - Normal successful execution
2. **Error paths** - Expected failures (not found, invalid input, etc.)
3. **Edge cases** - Empty inputs, null values, boundary conditions
4. **Security cases** - Path traversal, SQL injection, XSS attempts

### What NOT to Test
- External library behavior (trust rusqlite, ratatui, etc.)
- Trivial getters/setters (unless they have logic)
- Generated code
- CLI argument parsing (covered by integration tests)

---

## Test Organization

### File Structure
```
crates/
  alejandria-cli/
    src/
      commands/
        tui.rs
        tui_tests.rs          ← TUI unit tests
    tests/
      integration/
        tui_integration.rs    ← Integration tests
        
  alejandria-storage/
    src/
      memory.rs
      memory_tests.rs         ← Storage tests
```

### Test Modules
```rust
// Unit tests (same file or _tests.rs)
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature() { }
}

// Integration tests (tests/ directory)
#[test]
fn test_end_to_end_flow() { }
```

---

## Test Types

### 1. Unit Tests
Test individual functions/methods in isolation.

```rust
#[test]
fn test_sanitize_path() {
    let result = sanitize_path("../etc/passwd");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("traversal"));
}
```

### 2. Integration Tests
Test multiple components working together.

```rust
#[test]
fn test_memory_crud_flow() {
    let store = SqliteStore::new_in_memory().unwrap();
    
    // Create
    let id = store.save(Memory { ... }).unwrap();
    
    // Read
    let memory = store.get(&id).unwrap();
    assert_eq!(memory.content, "test");
    
    // Update
    store.update(&id, updated_memory).unwrap();
    
    // Delete
    store.delete(&id).unwrap();
    assert!(store.get(&id).is_err());
}
```

### 3. Property-Based Tests (when applicable)
Test invariants across many random inputs.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_search_never_panics(query in "\\PC*") {
        let store = create_test_store();
        let _ = store.search(&query); // Should not panic
    }
}
```

### 4. Security Tests
Test attack scenarios explicitly.

```rust
#[test]
fn test_path_traversal_blocked() {
    let attempts = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32",
        "/etc/shadow",
        "~/.ssh/id_rsa",
    ];
    
    for attempt in attempts {
        let result = validate_export_path(Path::new(attempt));
        assert!(result.is_err(), "Should block: {}", attempt);
    }
}
```

---

## Test Quality Standards

### Good Test Characteristics
- ✅ **Fast** - Runs in milliseconds
- ✅ **Isolated** - No dependencies on other tests
- ✅ **Deterministic** - Same input = same output
- ✅ **Clear** - Obvious what's being tested
- ✅ **Maintainable** - Easy to update when code changes

### Bad Test Smells
- ❌ Depends on external services (use mocks)
- ❌ Requires manual setup (automate in test)
- ❌ Flaky (sometimes passes, sometimes fails)
- ❌ Tests implementation details (test behavior, not internals)
- ❌ Too many assertions (one concept per test)

---

## Mocking and Test Doubles

### When to Mock
- External APIs/services
- File system operations (when not testing I/O)
- Time-dependent behavior
- Expensive operations

### How to Mock in Rust

```rust
// Trait-based mocking
trait MemoryStore {
    fn get(&self, id: &str) -> Result<Memory>;
    fn save(&self, memory: Memory) -> Result<String>;
}

// Test implementation
struct MockStore {
    memories: HashMap<String, Memory>,
}

impl MemoryStore for MockStore {
    fn get(&self, id: &str) -> Result<Memory> {
        self.memories.get(id)
            .cloned()
            .ok_or(anyhow!("not found"))
    }
}

#[test]
fn test_with_mock() {
    let mock = MockStore::new();
    // ... test using mock
}
```

---

## Coverage Tools

### Measure Coverage
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run with HTML report
cargo tarpaulin --out Html --exclude-files "crates/alejandria-mcp/*"

# Open report
open tarpaulin-report.html
```

### CI Integration
```yaml
# .gitlab-ci.yml
test:
  script:
    - cargo test --all-features
    - cargo tarpaulin --out Xml
  coverage: '/\d+\.\d+% coverage/'
  artifacts:
    reports:
      coverage_report:
        coverage_format: cobertura
        path: cobertura.xml
```

---

## Test Naming Conventions

### Pattern
`test_<unit>_<scenario>_<expected_result>`

### Examples
```rust
#[test]
fn test_memory_save_with_valid_data_succeeds() { }

#[test]
fn test_memory_save_with_empty_content_fails() { }

#[test]
fn test_search_with_special_chars_sanitizes_input() { }

#[test]
fn test_export_with_path_traversal_blocked() { }
```

---

## Test Data Management

### Create Test Fixtures
```rust
fn create_test_memory() -> Memory {
    Memory {
        id: "test-123".to_string(),
        title: "Test Memory".to_string(),
        content: "Test content".to_string(),
        type_: MemoryType::Decision,
        importance: "high".to_string(),
        timestamp: Utc::now(),
        // ... other fields with sensible defaults
    }
}

#[test]
fn test_with_fixture() {
    let memory = create_test_memory();
    // ... use in test
}
```

### Use Builders for Complex Data
```rust
struct MemoryBuilder {
    memory: Memory,
}

impl MemoryBuilder {
    fn new() -> Self {
        Self { memory: create_test_memory() }
    }
    
    fn with_title(mut self, title: &str) -> Self {
        self.memory.title = title.to_string();
        self
    }
    
    fn build(self) -> Memory {
        self.memory
    }
}

#[test]
fn test_with_builder() {
    let memory = MemoryBuilder::new()
        .with_title("Custom Title")
        .build();
}
```

---

## Pre-Commit Checklist

Before committing ANY code change:

- [ ] All tests pass: `cargo test`
- [ ] New code has tests (happy + error + edge)
- [ ] Coverage ≥70% for new code
- [ ] Security-relevant code has explicit security tests
- [ ] No flaky tests (run 3x to verify)
- [ ] Test names follow convention
- [ ] No `println!` or debug code in tests
- [ ] CI will pass (lint + test + coverage)

---

## CI Commands

```bash
# Local testing (before push)
cargo test --all-features
cargo clippy -- -D warnings
cargo fmt --check

# Coverage check
cargo tarpaulin --out Html
# Verify coverage ≥70%

# Security audit
cargo audit

# Full pre-push check
./scripts/pre-push.sh  # if exists
```

---

## Common Testing Mistakes

### 1. Testing Implementation Instead of Behavior
```rust
// BAD - tests implementation detail
#[test]
fn test_memory_stored_in_hashmap() {
    assert!(store.memories.contains_key("id"));
}

// GOOD - tests behavior
#[test]
fn test_memory_can_be_retrieved_after_save() {
    let id = store.save(memory).unwrap();
    let retrieved = store.get(&id).unwrap();
    assert_eq!(retrieved.title, memory.title);
}
```

### 2. Too Many Assertions
```rust
// BAD - testing multiple concerns
#[test]
fn test_memory_operations() {
    // tests save, get, update, delete all in one
}

// GOOD - one concept per test
#[test]
fn test_memory_save_returns_valid_id() { }

#[test]
fn test_memory_get_retrieves_correct_data() { }
```

### 3. No Error Testing
```rust
// BAD - only happy path
#[test]
fn test_memory_save() {
    let result = store.save(memory);
    assert!(result.is_ok());
}

// GOOD - also test errors
#[test]
fn test_memory_save_with_empty_content_fails() {
    let invalid = Memory { content: "".into(), ..default() };
    let result = store.save(invalid);
    assert!(result.is_err());
}
```

---

## Security Testing Requirements

For security-sensitive code (auth, file I/O, database, API):

### Required Security Tests
1. **Input validation** - Reject malicious inputs
2. **Path traversal** - Block directory escapes
3. **SQL injection** - Parameterized queries safe
4. **XSS prevention** - Output sanitized
5. **Secret redaction** - API keys masked in logs

### Example Security Test Suite
```rust
#[cfg(test)]
mod security_tests {
    #[test]
    fn test_path_traversal_prevention() { }
    
    #[test]
    fn test_sql_injection_blocked() { }
    
    #[test]
    fn test_secret_redaction_in_exports() { }
    
    #[test]
    fn test_file_permissions_restrictive() { }
    
    #[test]
    fn test_xss_sanitization() { }
}
```

---

## Enforcement

- CI MUST run all tests and block merge if any fail
- Coverage reports MUST be generated on every PR
- PRs with <70% coverage for new code MUST be rejected
- Security fixes MUST include explicit security tests
- Flaky tests MUST be fixed or removed immediately

**Target: 80%+ overall coverage, 0 flaky tests**

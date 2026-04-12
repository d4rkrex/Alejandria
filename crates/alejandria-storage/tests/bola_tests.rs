//! BOLA (Broken Object Level Authorization) protection tests
//!
//! These tests verify that:
//! 1. Users can only access their own memories
//! 2. Shared memories (SHARED) are accessible by all users
//! 3. Legacy memories (LEGACY_SYSTEM) are accessible by all users
//! 4. Authorization failures are properly logged and blocked

use alejandria_core::{Importance, Memory, MemoryStore};
use alejandria_storage::SqliteStore;

/// Test that BOLA protection blocks unauthorized access to GET operations
#[test]
fn test_bola_protection_get() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // User 1 creates a memory
    let mut memory1 = Memory::new("test-topic".to_string(), "User 1's secret".to_string());
    memory1.importance = Importance::High;
    memory1.raw_excerpt = Some("Confidential data for user 1".to_string());

    let user1_hash = "user1_api_key_hash_64_chars_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let memory1_id = store
        .store_with_owner(memory1, user1_hash)
        .expect("Failed to store memory for user1");

    // User 1 can access their own memory
    let result = store.get_authorized(&memory1_id, user1_hash);
    assert!(
        result.is_ok(),
        "User 1 should be able to access their own memory"
    );
    assert!(result.unwrap().is_some());

    // User 2 tries to access User 1's memory (should FAIL)
    let user2_hash = "user2_api_key_hash_64_chars_YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";
    let result = store.get_authorized(&memory1_id, user2_hash);
    assert!(
        result.is_err(),
        "User 2 should NOT be able to access User 1's memory"
    );

    // Verify the error is Forbidden
    match result {
        Err(alejandria_core::error::IcmError::Forbidden(_)) => {
            // Expected error type
        }
        _ => panic!("Expected Forbidden error, got: {:?}", result),
    }
}

/// Test that BOLA protection blocks unauthorized UPDATE operations
#[test]
fn test_bola_protection_update() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // User 1 creates a memory
    let mut memory1 = Memory::new("test-topic".to_string(), "User 1's data".to_string());
    memory1.importance = Importance::Medium;

    let user1_hash = "user1_hash_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let memory1_id = store
        .store_with_owner(memory1.clone(), user1_hash)
        .expect("Failed to store memory");

    // User 2 tries to update User 1's memory (should FAIL)
    let user2_hash = "user2_hash_YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";
    let mut updated_memory = store
        .get(&memory1_id)
        .expect("Failed to get memory")
        .expect("Memory not found");
    updated_memory.summary = "Hacked by user 2".to_string();

    let result = store.update_authorized(&updated_memory, user2_hash);
    assert!(
        result.is_err(),
        "User 2 should NOT be able to update User 1's memory"
    );

    // Verify the error is Forbidden
    match result {
        Err(alejandria_core::error::IcmError::Forbidden(_)) => {
            // Expected
        }
        _ => panic!("Expected Forbidden error"),
    }

    // User 1 CAN update their own memory
    updated_memory.summary = "Updated by user 1".to_string();
    let result = store.update_authorized(&updated_memory, user1_hash);
    assert!(
        result.is_ok(),
        "User 1 should be able to update their own memory"
    );
}

/// Test that BOLA protection blocks unauthorized DELETE operations
#[test]
fn test_bola_protection_delete() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // User 1 creates a memory
    let memory1 = Memory::new("test-topic".to_string(), "User 1's data".to_string());
    let user1_hash = "user1_hash_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let memory1_id = store
        .store_with_owner(memory1, user1_hash)
        .expect("Failed to store memory");

    // User 2 tries to delete User 1's memory (should FAIL)
    let user2_hash = "user2_hash_YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";
    let result = store.delete_authorized(&memory1_id, user2_hash);
    assert!(
        result.is_err(),
        "User 2 should NOT be able to delete User 1's memory"
    );

    // Verify memory still exists
    let existing = store.get(&memory1_id).expect("Failed to get memory");
    assert!(
        existing.is_some(),
        "Memory should still exist after failed delete"
    );

    // User 1 CAN delete their own memory
    let result = store.delete_authorized(&memory1_id, user1_hash);
    assert!(
        result.is_ok(),
        "User 1 should be able to delete their own memory"
    );

    // Verify memory is deleted
    let existing = store.get(&memory1_id).expect("Failed to get memory");
    assert!(existing.is_none(), "Memory should be deleted");
}

/// Test that SHARED memories are accessible by all users
#[test]
fn test_shared_memory_accessible_by_all() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create a shared memory (system-wide knowledge)
    let mut shared_memory = Memory::new(
        "shared-knowledge".to_string(),
        "Public system information".to_string(),
    );
    shared_memory.importance = Importance::Critical;
    shared_memory.raw_excerpt = Some("This is accessible to everyone".to_string());

    let shared_id = store
        .store_with_owner(shared_memory, "SHARED")
        .expect("Failed to store shared memory");

    // User 1 can access shared memory
    let user1_hash = "user1_hash_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let result = store.get_authorized(&shared_id, user1_hash);
    assert!(
        result.is_ok(),
        "User 1 should be able to access shared memory"
    );
    assert!(result.unwrap().is_some());

    // User 2 can also access shared memory
    let user2_hash = "user2_hash_YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";
    let result = store.get_authorized(&shared_id, user2_hash);
    assert!(
        result.is_ok(),
        "User 2 should be able to access shared memory"
    );
    assert!(result.unwrap().is_some());

    // User 3 can also access shared memory
    let user3_hash = "user3_hash_ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ";
    let result = store.get_authorized(&shared_id, user3_hash);
    assert!(
        result.is_ok(),
        "User 3 should be able to access shared memory"
    );
    assert!(result.unwrap().is_some());
}

/// Test that LEGACY_SYSTEM memories are accessible by all users (backward compatibility)
#[test]
fn test_legacy_memory_accessible_by_all() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // Create a legacy memory (pre-migration data)
    let legacy_memory = Memory::new(
        "legacy-data".to_string(),
        "Pre-migration memory".to_string(),
    );

    let legacy_id = store
        .store_with_owner(legacy_memory, "LEGACY_SYSTEM")
        .expect("Failed to store legacy memory");

    // User 1 can access legacy memory
    let user1_hash = "user1_hash_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let result = store.get_authorized(&legacy_id, user1_hash);
    assert!(
        result.is_ok(),
        "User 1 should be able to access legacy memory"
    );
    assert!(result.unwrap().is_some());

    // User 2 can also access legacy memory
    let user2_hash = "user2_hash_YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";
    let result = store.get_authorized(&legacy_id, user2_hash);
    assert!(
        result.is_ok(),
        "User 2 should be able to access legacy memory"
    );
    assert!(result.unwrap().is_some());
}

/// Test that search operations respect ownership isolation
#[test]
fn test_search_isolation() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // User 1 creates memories
    let user1_hash = "user1_hash_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let mut mem1 = Memory::new("rust".to_string(), "User 1: Rust programming".to_string());
    mem1.keywords = vec!["rust".to_string(), "programming".to_string()];
    store
        .store_with_owner(mem1, user1_hash)
        .expect("Failed to store user1 memory");

    // User 2 creates memories
    let user2_hash = "user2_hash_YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";
    let mut mem2 = Memory::new("rust".to_string(), "User 2: Rust secrets".to_string());
    mem2.keywords = vec!["rust".to_string(), "secrets".to_string()];
    store
        .store_with_owner(mem2, user2_hash)
        .expect("Failed to store user2 memory");

    // Create a shared memory
    let mut shared = Memory::new("rust".to_string(), "Shared: Rust documentation".to_string());
    shared.keywords = vec!["rust".to_string(), "docs".to_string()];
    store
        .store_with_owner(shared, "SHARED")
        .expect("Failed to store shared memory");

    // User 1 searches for "rust" - should see their own + shared (NOT user2's)
    let results = store
        .search_by_keywords_authorized("rust", 10, user1_hash)
        .expect("Search failed");

    // User 1 should see 2 results: their own + shared
    assert_eq!(results.len(), 2, "User 1 should see exactly 2 results");

    // Verify User 1 does NOT see User 2's secret
    for result in &results {
        assert!(
            !result.summary.contains("User 2"),
            "User 1 should NOT see User 2's memory"
        );
    }

    // User 2 searches for "rust" - should see their own + shared (NOT user1's)
    let results = store
        .search_by_keywords_authorized("rust", 10, user2_hash)
        .expect("Search failed");

    // User 2 should see 2 results: their own + shared
    assert_eq!(results.len(), 2, "User 2 should see exactly 2 results");

    // Verify User 2 does NOT see User 1's memory
    for result in &results {
        assert!(
            !result.summary.contains("User 1"),
            "User 2 should NOT see User 1's memory"
        );
    }
}

/// Test that users cannot change ownership via update
#[test]
fn test_prevent_owner_change_via_update() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    // User 1 creates a memory
    let memory1 = Memory::new("test".to_string(), "User 1's data".to_string());
    let user1_hash = "user1_hash_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let memory1_id = store
        .store_with_owner(memory1, user1_hash)
        .expect("Failed to store memory");

    // User 1 tries to change ownership to User 2 via update
    let mut updated_memory = store
        .get(&memory1_id)
        .expect("Failed to get memory")
        .expect("Memory not found");

    let user2_hash = "user2_hash_YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";
    updated_memory.owner_key_hash = user2_hash.to_string(); // Attempt to change owner
    updated_memory.summary = "Trying to reassign".to_string();

    // Update should succeed but owner should NOT change
    store
        .update_authorized(&updated_memory, user1_hash)
        .expect("Update should succeed");

    // Verify owner is still User 1 (NOT User 2)
    let final_memory = store
        .get(&memory1_id)
        .expect("Failed to get memory")
        .expect("Memory not found");

    assert_eq!(
        final_memory.owner_key_hash, user1_hash,
        "Owner should NOT have changed"
    );

    // User 2 should still NOT be able to access this memory
    let result = store.get_authorized(&memory1_id, user2_hash);
    assert!(
        result.is_err(),
        "User 2 should still NOT have access after attempted owner change"
    );
}

/// Test that access to non-existent memory returns NotFound (not Forbidden)
#[test]
fn test_nonexistent_memory_returns_not_found() {
    let store = SqliteStore::open_in_memory().expect("Failed to create in-memory store");

    let user1_hash = "user1_hash_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let fake_id = "01HQXXXXXXXXXXXXXXXXXXXXXXXXXXX"; // Non-existent ULID

    let result = store.get_authorized(fake_id, user1_hash);
    assert!(
        result.is_err(),
        "Should return error for non-existent memory"
    );

    // Verify it's NotFoundSimple, not Forbidden
    match result {
        Err(alejandria_core::error::IcmError::NotFoundSimple(_)) => {
            // Expected
        }
        _ => panic!("Expected NotFoundSimple error, got: {:?}", result),
    }
}

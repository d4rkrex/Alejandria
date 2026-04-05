//! Integration tests for CLI commands
//!
//! These tests verify the CLI binary works correctly end-to-end.
//! They test:
//! - Task 7.40: `alejandria store` returns ULID
//! - Task 7.41: `alejandria recall --json` outputs JSON array
//! - Task 7.42: Invalid command returns non-zero exit code

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

/// Helper to create a temporary database and get its path
fn setup_test_db() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir
        .path()
        .join("test.db")
        .to_str()
        .unwrap()
        .to_string();
    (temp_dir, db_path)
}

/// Task 7.40: Test that `alejandria store` returns ULID
#[test]
fn test_store_returns_ulid() {
    let (_temp_dir, db_path) = setup_test_db();

    let mut cmd = Command::cargo_bin("alejandria").unwrap();
    cmd.env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("store")
        .arg("Test memory content")
        .arg("--topic")
        .arg("test")
        .arg("--importance")
        .arg("high");

    let output = cmd.output().unwrap();

    // Verify success
    assert!(output.status.success(), "Command should succeed");

    // Verify output contains ULID format
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Stored memory:"),
        "Should show success message"
    );
    assert!(
        stdout.contains("01"),
        "Should contain ULID starting with 01"
    );

    // Verify ULID format (26 character identifier)
    let lines: Vec<&str> = stdout.lines().collect();
    let memory_line = lines
        .iter()
        .find(|line| line.contains("Stored memory:"))
        .expect("Should have memory line");

    // Extract the ULID part (after "Stored memory: ")
    let parts: Vec<&str> = memory_line.split(": ").collect();
    assert_eq!(parts.len(), 2, "Should have ID after colon");
    let ulid = parts[1].trim();
    assert_eq!(ulid.len(), 26, "ULID should be 26 characters");
    assert!(ulid.starts_with("01"), "ULID should start with 01");
}

/// Task 7.41: Test that `alejandria recall --json` outputs valid JSON array
#[test]
fn test_recall_json_output() {
    let (_temp_dir, db_path) = setup_test_db();

    // First, store a memory
    let mut store_cmd = Command::cargo_bin("alejandria").unwrap();
    store_cmd
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("store")
        .arg("Test memory for recall")
        .arg("--topic")
        .arg("test")
        .assert()
        .success();

    // Then recall with JSON output
    let mut recall_cmd = Command::cargo_bin("alejandria").unwrap();
    let output = recall_cmd
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("--json")
        .arg("recall")
        .arg("test")
        .arg("--limit")
        .arg("5")
        .output()
        .unwrap();

    assert!(output.status.success());

    // Parse JSON output
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify structure
    assert!(
        json.get("memories").is_some(),
        "JSON should have 'memories' field"
    );
    assert!(
        json.get("total_found").is_some(),
        "JSON should have 'total_found' field"
    );

    let memories = json["memories"]
        .as_array()
        .expect("memories should be an array");
    assert_eq!(memories.len(), 1, "Should find 1 memory");

    // Verify memory structure
    let memory = &memories[0];
    assert!(memory.get("id").is_some(), "Memory should have 'id' field");
    assert!(
        memory.get("topic").is_some(),
        "Memory should have 'topic' field"
    );
    assert!(
        memory.get("summary").is_some(),
        "Memory should have 'summary' field"
    );
    assert!(
        memory.get("importance").is_some(),
        "Memory should have 'importance' field"
    );
}

/// Task 7.42: Test that invalid command returns non-zero exit code
#[test]
fn test_invalid_command_fails() {
    let mut cmd = Command::cargo_bin("alejandria").unwrap();
    cmd.arg("invalid-command").arg("some-arg");

    cmd.assert().failure().code(2); // clap returns exit code 2 for usage errors
}

/// Additional test: Verify stderr for errors and stdout for output (task 7.39)
#[test]
fn test_error_output_to_stderr() {
    let mut cmd = Command::cargo_bin("alejandria").unwrap();
    cmd.env("ALEJANDRIA_DB_PATH", "/nonexistent/path/db.sqlite")
        .arg("store")
        .arg("Test content");

    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error:"));
}

/// Test JSON output for topics command
#[test]
fn test_topics_json_output() {
    let (_temp_dir, db_path) = setup_test_db();

    // Store a memory first
    Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("store")
        .arg("Test content")
        .arg("--topic")
        .arg("testing")
        .assert()
        .success();

    // Get topics with JSON output
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("--json")
        .arg("topics")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(
        json.get("topics").is_some(),
        "JSON should have 'topics' field"
    );
    let topics = json["topics"]
        .as_array()
        .expect("topics should be an array");
    assert!(!topics.is_empty(), "Should have at least 1 topic");
}

/// Test stats command JSON output
#[test]
fn test_stats_json_output() {
    let (_temp_dir, db_path) = setup_test_db();

    // Store a memory first
    Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("store")
        .arg("Test content")
        .assert()
        .success();

    // Get stats with JSON output
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("--json")
        .arg("stats")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Verify required fields
    assert!(json.get("total_memories").is_some());
    assert!(json.get("active_memories").is_some());
    assert!(json.get("by_importance").is_some());
    assert!(json.get("by_source").is_some());
}

/// Test memoir commands
#[test]
fn test_memoir_commands() {
    let (_temp_dir, db_path) = setup_test_db();

    // Create a memoir
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("memoir")
        .arg("create")
        .arg("Test Memoir")
        .arg("A test knowledge graph")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created memoir:"));
    assert!(stdout.contains("01")); // ULID prefix

    // List memoirs
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("memoir")
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Test Memoir"));

    // Add a concept
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("memoir")
        .arg("add-concept")
        .arg("Test Memoir")
        .arg("AI")
        .arg("--definition")
        .arg("Artificial Intelligence")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Added concept"));
    assert!(stdout.contains("AI"));
}

/// Test update command
#[test]
fn test_update_command() {
    let (_temp_dir, db_path) = setup_test_db();

    // Store a memory and capture the ID
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("store")
        .arg("Original content")
        .arg("--topic")
        .arg("test")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Extract ULID from output
    let id_start = stdout.find("01").expect("Should contain ULID");
    let id = &stdout[id_start..id_start + 26];

    // Update the memory
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("update")
        .arg(id)
        .arg("--summary")
        .arg("Updated summary")
        .arg("--importance")
        .arg("high")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Updated memory"));
}

/// Test forget command
#[test]
fn test_forget_command() {
    let (_temp_dir, db_path) = setup_test_db();

    // Store a memory
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("store")
        .arg("To be deleted")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let id_start = stdout.find("01").expect("Should contain ULID");
    let id = &stdout[id_start..id_start + 26];

    // Forget the memory
    let output = Command::cargo_bin("alejandria")
        .unwrap()
        .env("ALEJANDRIA_DB_PATH", &db_path)
        .arg("forget")
        .arg(id)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Deleted memory"));
}

//! Validation logic for imported memory data.
//!
//! Ensures imported data matches the expected schema and contains
//! all required fields before attempting database insertion.

use crate::error::{IcmError, IcmResult};
use crate::memory::Memory;
use serde_json::Value;

/// Validate a memory object has all required fields
pub fn validate_memory(memory: &Memory) -> IcmResult<()> {
    // Check required fields
    if memory.id.is_empty() {
        return Err(IcmError::InvalidInput(
            "Memory missing required field: id".to_string(),
        ));
    }

    if memory.topic.is_empty() {
        return Err(IcmError::InvalidInput(
            "Memory missing required field: topic".to_string(),
        ));
    }

    // Validate weight is in valid range [0, 1]
    if !(0.0..=1.0).contains(&memory.weight) {
        return Err(IcmError::InvalidInput(format!(
            "Memory weight must be between 0.0 and 1.0, got: {}",
            memory.weight
        )));
    }

    Ok(())
}

/// Validate JSON schema matches Memory structure
pub fn validate_json_schema(value: &Value) -> IcmResult<()> {
    let obj = value
        .as_object()
        .ok_or_else(|| IcmError::InvalidInput("JSON value must be an object".to_string()))?;

    // Check required fields exist
    let required_fields = ["id", "created_at", "topic"];
    for field in &required_fields {
        if !obj.contains_key(*field) {
            return Err(IcmError::InvalidInput(format!(
                "JSON object missing required field: {}",
                field
            )));
        }
    }

    // Validate data types
    if !obj["id"].is_string() {
        return Err(IcmError::InvalidInput(
            "Field 'id' must be a string".to_string(),
        ));
    }

    if !obj["topic"].is_string() {
        return Err(IcmError::InvalidInput(
            "Field 'topic' must be a string".to_string(),
        ));
    }

    if !obj["created_at"].is_number() && !obj["created_at"].is_string() {
        return Err(IcmError::InvalidInput(
            "Field 'created_at' must be a number or string".to_string(),
        ));
    }

    Ok(())
}

/// Validate YAML schema matches Memory structure
pub fn validate_yaml_schema(value: &Value) -> IcmResult<()> {
    // YAML is parsed to JSON Value, so use same validation
    validate_json_schema(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;

    #[test]
    fn test_validate_memory_valid() {
        let memory = Memory::new("test topic".to_string(), "test summary".to_string());
        assert!(validate_memory(&memory).is_ok());
    }

    #[test]
    fn test_validate_memory_missing_id() {
        let mut memory = Memory::new("test topic".to_string(), "test summary".to_string());
        memory.id = String::new();
        let result = validate_memory(&memory);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing required field: id"));
    }

    #[test]
    fn test_validate_memory_missing_topic() {
        let mut memory = Memory::new("test topic".to_string(), "test summary".to_string());
        memory.topic = String::new();
        let result = validate_memory(&memory);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing required field: topic"));
    }

    #[test]
    fn test_validate_memory_invalid_weight() {
        let mut memory = Memory::new("test topic".to_string(), "test summary".to_string());
        memory.weight = 1.5;
        let result = validate_memory(&memory);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("weight must be"));
    }

    #[test]
    fn test_validate_json_schema_valid() {
        let json = serde_json::json!({
            "id": "test-id",
            "created_at": "2024-01-01T00:00:00Z",
            "topic": "test topic",
            "summary": "test summary"
        });
        assert!(validate_json_schema(&json).is_ok());
    }

    #[test]
    fn test_validate_json_schema_not_object() {
        let json = serde_json::json!("not an object");
        let result = validate_json_schema(&json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be an object"));
    }

    #[test]
    fn test_validate_json_schema_missing_id() {
        let json = serde_json::json!({
            "created_at": "2024-01-01T00:00:00Z",
            "topic": "test topic"
        });
        let result = validate_json_schema(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing required field: id"));
    }

    #[test]
    fn test_validate_json_schema_missing_topic() {
        let json = serde_json::json!({
            "id": "test-id",
            "created_at": "2024-01-01T00:00:00Z"
        });
        let result = validate_json_schema(&json);
        assert!(result.unwrap_err().to_string().contains("missing required field: topic"));
    }

    #[test]
    fn test_validate_json_schema_invalid_id_type() {
        let json = serde_json::json!({
            "id": 123,
            "created_at": "2024-01-01T00:00:00Z",
            "topic": "test topic"
        });
        let result = validate_json_schema(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("'id' must be a string"));
    }

    #[test]
    fn test_validate_json_schema_invalid_topic_type() {
        let json = serde_json::json!({
            "id": "test-id",
            "created_at": "2024-01-01T00:00:00Z",
            "topic": 123
        });
        let result = validate_json_schema(&json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("'topic' must be a string"));
    }
}

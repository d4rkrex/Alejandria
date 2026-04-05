//! Error types for Alejandria

use thiserror::Error;

/// Main error type for all Alejandria operations
#[derive(Error, Debug)]
pub enum IcmError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {entity} with {field}={value}")]
    NotFound {
        entity: String,
        field: String,
        value: String,
    },

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias using IcmError
pub type IcmResult<T> = Result<T, IcmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = IcmError::NotFound {
            entity: "Memory".to_string(),
            field: "id".to_string(),
            value: "123".to_string(),
        };
        assert_eq!(err.to_string(), "Not found: Memory with id=123");
    }

    #[test]
    fn test_error_conversions() {
        let json_err = serde_json::from_str::<i32>("invalid").unwrap_err();
        let icm_err: IcmError = json_err.into();
        assert!(matches!(icm_err, IcmError::Serialization(_)));
    }
}

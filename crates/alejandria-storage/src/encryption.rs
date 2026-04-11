//! Database encryption support using SQLCipher
//!
//! Provides AES-256 encryption at rest with PBKDF2 key derivation.
//! This module is only compiled when the `encryption` feature is enabled.

use alejandria_core::error::{IcmError, IcmResult};
use std::path::Path;

/// Encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Encryption key (32 bytes for AES-256)
    pub key: Vec<u8>,

    /// KDF iterations (100,000 recommended for PBKDF2)
    pub kdf_iter: u32,

    /// Cipher page size (default: 4096)
    pub page_size: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            key: vec![0u8; 32], // Should be replaced with actual key
            kdf_iter: 100_000,
            page_size: 4096,
        }
    }
}

impl EncryptionConfig {
    /// Create encryption config from password
    ///
    /// Uses PBKDF2 with SHA-256 to derive a 256-bit key from the password.
    pub fn from_password(password: &str, salt: &[u8]) -> IcmResult<Self> {
        use pbkdf2::pbkdf2_hmac;
        use sha2::Sha256;

        let kdf_iter = 100_000;
        let mut key = vec![0u8; 32];

        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, kdf_iter, &mut key);

        Ok(Self {
            key,
            kdf_iter,
            page_size: 4096,
        })
    }

    /// Create encryption config from raw key bytes
    pub fn from_key(key: Vec<u8>) -> IcmResult<Self> {
        if key.len() != 32 {
            return Err(IcmError::Config(
                "Encryption key must be exactly 32 bytes for AES-256".to_string(),
            ));
        }

        Ok(Self {
            key,
            kdf_iter: 100_000,
            page_size: 4096,
        })
    }
}

/// Open encrypted SQLite connection using SQLCipher
#[cfg(feature = "encryption")]
pub fn open_encrypted(path: &Path, config: &EncryptionConfig) -> IcmResult<rusqlite::Connection> {
    use rusqlite::Connection;

    let conn = Connection::open(path)
        .map_err(|e| IcmError::Database(format!("Failed to open database: {}", e)))?;

    // Set encryption key (SQLCipher PRAGMA)
    let key_hex = hex::encode(&config.key);
    conn.execute(&format!("PRAGMA key = \"x'{}'\";", key_hex), [])
        .map_err(|e| IcmError::Database(format!("Failed to set encryption key: {}", e)))?;

    // Configure KDF iterations
    conn.execute(&format!("PRAGMA kdf_iter = {};", config.kdf_iter), [])
        .map_err(|e| IcmError::Database(format!("Failed to set KDF iterations: {}", e)))?;

    // Configure cipher page size
    conn.execute(
        &format!("PRAGMA cipher_page_size = {};", config.page_size),
        [],
    )
    .map_err(|e| IcmError::Database(format!("Failed to set cipher page size: {}", e)))?;

    // Verify encryption is working by querying SQLite master table
    conn.query_row("SELECT COUNT(*) FROM sqlite_master;", [], |_| Ok(()))
        .map_err(|e| IcmError::Database(format!("Failed to verify encryption: {}", e)))?;

    Ok(conn)
}

/// Encrypt an existing unencrypted database
#[cfg(feature = "encryption")]
pub fn encrypt_database(
    source_path: &Path,
    dest_path: &Path,
    config: &EncryptionConfig,
) -> IcmResult<()> {
    use rusqlite::Connection;

    // Open source (unencrypted)
    let source = Connection::open(source_path)
        .map_err(|e| IcmError::Database(format!("Failed to open source database: {}", e)))?;

    // Attach encrypted destination
    let key_hex = hex::encode(&config.key);
    source
        .execute(
            &format!(
                "ATTACH DATABASE '{}' AS encrypted KEY \"x'{}'\";",
                dest_path.display(),
                key_hex
            ),
            [],
        )
        .map_err(|e| IcmError::Database(format!("Failed to attach encrypted database: {}", e)))?;

    // Configure encryption for attached database
    source
        .execute(
            &format!("PRAGMA encrypted.kdf_iter = {};", config.kdf_iter),
            [],
        )
        .map_err(|e| IcmError::Database(format!("Failed to set KDF iterations: {}", e)))?;
    source
        .execute(
            &format!("PRAGMA encrypted.cipher_page_size = {};", config.page_size),
            [],
        )
        .map_err(|e| IcmError::Database(format!("Failed to set cipher page size: {}", e)))?;

    // Copy schema and data
    source
        .execute("SELECT sqlcipher_export('encrypted');", [])
        .map_err(|e| IcmError::Database(format!("Failed to export data: {}", e)))?;

    // Detach
    source
        .execute("DETACH DATABASE encrypted;", [])
        .map_err(|e| IcmError::Database(format!("Failed to detach database: {}", e)))?;

    Ok(())
}

/// Decrypt an encrypted database to unencrypted format
#[cfg(feature = "encryption")]
pub fn decrypt_database(
    source_path: &Path,
    dest_path: &Path,
    config: &EncryptionConfig,
) -> IcmResult<()> {
    // Open encrypted source
    let source = open_encrypted(source_path, config)?;

    // Attach unencrypted destination
    source
        .execute(
            &format!(
                "ATTACH DATABASE '{}' AS plaintext KEY '';",
                dest_path.display()
            ),
            [],
        )
        .map_err(|e| IcmError::Database(format!("Failed to attach plaintext database: {}", e)))?;

    // Copy schema and data
    source
        .execute("SELECT sqlcipher_export('plaintext');", [])
        .map_err(|e| IcmError::Database(format!("Failed to export data: {}", e)))?;

    // Detach
    source
        .execute("DETACH DATABASE plaintext;", [])
        .map_err(|e| IcmError::Database(format!("Failed to detach database: {}", e)))?;

    Ok(())
}

#[cfg(test)]
#[cfg(feature = "encryption")]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_encryption_config_from_password() {
        let salt = b"test_salt_12345";
        let config = EncryptionConfig::from_password("test_password", salt).unwrap();

        assert_eq!(config.key.len(), 32);
        assert_eq!(config.kdf_iter, 100_000);
    }

    #[test]
    fn test_encryption_config_from_key() {
        let key = vec![42u8; 32];
        let config = EncryptionConfig::from_key(key.clone()).unwrap();

        assert_eq!(config.key, key);
    }

    #[test]
    fn test_encryption_config_invalid_key_length() {
        let key = vec![42u8; 16]; // Wrong size
        assert!(EncryptionConfig::from_key(key).is_err());
    }
}

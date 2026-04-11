//! Session manager
//!
//! Manages SSE session lifecycle with TTL enforcement
//! (CRITICAL security requirement from review.md finding S-003).

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use uuid::Uuid;

use super::connection::ConnectionId;

/// Session identifier (UUID v4 for 128-bit entropy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Session metadata
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    /// SHA-256 hash of the API key
    pub api_key_hash: String,

    /// Client IP address
    pub client_ip: IpAddr,

    /// Associated connection ID
    pub connection_id: ConnectionId,

    /// Session creation timestamp
    pub created_at: Instant,

    /// Last activity timestamp
    pub last_activity: Instant,
}

/// Session errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found")]
    NotFound,

    #[error("Session expired (TTL: {0} seconds)")]
    Expired(u64),
}

/// Session manager
///
/// Manages SSE session lifecycle with automatic expiration.
/// Sessions are identified by UUID v4 tokens with 30-minute TTL.
pub struct SessionManager {
    /// Active sessions
    sessions: HashMap<SessionId, SessionMetadata>,

    /// Session TTL (time-to-live)
    session_ttl: Duration,
}

impl SessionManager {
    /// Create a new session manager
    ///
    /// # Arguments
    ///
    /// * `ttl_secs` - Session TTL in seconds (default: 1800 = 30 minutes)
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            session_ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Create a new session
    ///
    /// Returns a unique session ID that can be used to validate future requests.
    pub fn create_session(
        &mut self,
        api_key_hash: String,
        client_ip: IpAddr,
        connection_id: ConnectionId,
    ) -> SessionId {
        let session_id = SessionId::new();
        let now = Instant::now();

        let metadata = SessionMetadata {
            api_key_hash,
            client_ip,
            connection_id,
            created_at: now,
            last_activity: now,
        };

        self.sessions.insert(session_id, metadata);
        session_id
    }

    /// Validate a session
    ///
    /// Checks if the session exists and hasn't expired.
    /// Updates the last_activity timestamp on successful validation.
    pub fn validate_session(
        &mut self,
        session_id: SessionId,
    ) -> Result<&SessionMetadata, SessionError> {
        let metadata = self
            .sessions
            .get_mut(&session_id)
            .ok_or(SessionError::NotFound)?;

        // Check TTL
        if metadata.last_activity.elapsed() > self.session_ttl {
            self.sessions.remove(&session_id);
            return Err(SessionError::Expired(self.session_ttl.as_secs()));
        }

        // Update last activity
        metadata.last_activity = Instant::now();

        // Return immutable reference (safe because we updated above)
        Ok(self.sessions.get(&session_id).unwrap())
    }

    /// Remove a session
    pub fn remove_session(&mut self, session_id: SessionId) {
        self.sessions.remove(&session_id);
    }

    /// Cleanup expired sessions
    ///
    /// Should be called periodically to prevent memory leaks.
    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.sessions
            .retain(|_, metadata| now.duration_since(metadata.last_activity) < self.session_ttl);
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::super::connection::ConnectionId;
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_create_session() {
        let mut manager = SessionManager::new(1800);
        let api_key_hash = "test-hash".to_string();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let conn_id = ConnectionId::new();

        let session_id = manager.create_session(api_key_hash.clone(), ip, conn_id);
        assert_eq!(manager.session_count(), 1);

        let metadata = manager.validate_session(session_id).unwrap();
        assert_eq!(metadata.api_key_hash, api_key_hash);
        assert_eq!(metadata.client_ip, ip);
    }

    #[test]
    fn test_session_expiration() {
        let mut manager = SessionManager::new(0); // 0 second TTL for immediate expiration
        let api_key_hash = "test-hash".to_string();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let conn_id = ConnectionId::new();

        let session_id = manager.create_session(api_key_hash, ip, conn_id);

        // Sleep to ensure expiration
        std::thread::sleep(Duration::from_millis(10));

        let result = manager.validate_session(session_id);
        assert!(matches!(result, Err(SessionError::Expired(_))));
    }

    #[test]
    fn test_remove_session() {
        let mut manager = SessionManager::new(1800);
        let api_key_hash = "test-hash".to_string();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let conn_id = ConnectionId::new();

        let session_id = manager.create_session(api_key_hash, ip, conn_id);
        assert_eq!(manager.session_count(), 1);

        manager.remove_session(session_id);
        assert_eq!(manager.session_count(), 0);

        let result = manager.validate_session(session_id);
        assert!(matches!(result, Err(SessionError::NotFound)));
    }

    #[test]
    fn test_cleanup_expired() {
        let mut manager = SessionManager::new(0); // 0 second TTL
        let api_key_hash = "test-hash".to_string();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Create multiple sessions
        for _ in 0..5 {
            let conn_id = ConnectionId::new();
            manager.create_session(api_key_hash.clone(), ip, conn_id);
        }

        assert_eq!(manager.session_count(), 5);

        // Sleep to ensure expiration
        std::thread::sleep(Duration::from_millis(10));

        // Cleanup should remove all expired sessions
        manager.cleanup_expired();
        assert_eq!(manager.session_count(), 0);
    }
}

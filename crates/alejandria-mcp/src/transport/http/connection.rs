//! Connection manager
//!
//! Tracks active connections and enforces limits to prevent DoS attacks
//! (CRITICAL security requirement from review.md finding DOS-001).

use super::ConnectionLimits;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use uuid::Uuid;

/// Unique connection identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// API key hash (SHA-256) for tracking connections per key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiKeyHash(String);

impl ApiKeyHash {
    pub fn new(hash: String) -> Self {
        Self(hash)
    }
}

/// Connection limit errors
#[derive(Debug, thiserror::Error)]
pub enum ConnectionLimitError {
    #[error("Global connection limit exceeded (max: {0})")]
    GlobalLimitExceeded(usize),

    #[error("Per-API-key connection limit exceeded (max: {0})")]
    PerKeyLimitExceeded(usize),

    #[error("Per-IP connection limit exceeded (max: {0})")]
    PerIpLimitExceeded(usize),
}

/// Connection manager
///
/// Implements three-tier connection limits:
/// 1. Per-API-key limit (default: 10 connections)
/// 2. Per-IP limit (default: 50 connections)
/// 3. Global limit (default: 1000 connections)
pub struct ConnectionManager {
    limits: ConnectionLimits,

    /// Connections grouped by API key hash
    connections_by_key: HashMap<ApiKeyHash, HashSet<ConnectionId>>,

    /// Connections grouped by IP address
    connections_by_ip: HashMap<IpAddr, HashSet<ConnectionId>>,

    /// Total active connections
    total_connections: usize,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(limits: ConnectionLimits) -> Self {
        Self {
            limits,
            connections_by_key: HashMap::new(),
            connections_by_ip: HashMap::new(),
            total_connections: 0,
        }
    }

    /// Attempt to add a new connection
    ///
    /// Checks all three limits before allowing the connection.
    ///
    /// # Returns
    ///
    /// Returns a unique `ConnectionId` if the connection is allowed,
    /// or an error if any limit is exceeded.
    pub fn try_add_connection(
        &mut self,
        api_key_hash: &str,
        client_ip: IpAddr,
    ) -> Result<ConnectionId, ConnectionLimitError> {
        // Check global limit
        if self.total_connections >= self.limits.global {
            return Err(ConnectionLimitError::GlobalLimitExceeded(
                self.limits.global,
            ));
        }

        let api_key = ApiKeyHash::new(api_key_hash.to_string());

        // Check per-key limit
        let key_connections = self.connections_by_key.entry(api_key.clone()).or_default();
        if key_connections.len() >= self.limits.per_key {
            return Err(ConnectionLimitError::PerKeyLimitExceeded(
                self.limits.per_key,
            ));
        }

        // Check per-IP limit
        let ip_connections = self.connections_by_ip.entry(client_ip).or_default();
        if ip_connections.len() >= self.limits.per_ip {
            return Err(ConnectionLimitError::PerIpLimitExceeded(self.limits.per_ip));
        }

        // Create new connection
        let conn_id = ConnectionId::new();

        // Add to tracking structures
        key_connections.insert(conn_id);
        ip_connections.insert(conn_id);
        self.total_connections += 1;

        Ok(conn_id)
    }

    /// Remove a connection
    ///
    /// Cleans up the connection from all tracking structures.
    pub fn remove_connection(&mut self, conn_id: ConnectionId) {
        // Remove from per-key tracking
        self.connections_by_key.retain(|_, conns| {
            conns.remove(&conn_id);
            !conns.is_empty()
        });

        // Remove from per-IP tracking
        self.connections_by_ip.retain(|_, conns| {
            conns.remove(&conn_id);
            !conns.is_empty()
        });

        // Decrement total
        self.total_connections = self.total_connections.saturating_sub(1);
    }

    /// Get current connection count
    pub fn connection_count(&self) -> usize {
        self.total_connections
    }

    /// Get connection count for a specific API key
    pub fn connection_count_for_key(&self, api_key_hash: &str) -> usize {
        let api_key = ApiKeyHash::new(api_key_hash.to_string());
        self.connections_by_key
            .get(&api_key)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }

    /// Get connection count for a specific IP
    pub fn connection_count_for_ip(&self, ip: IpAddr) -> usize {
        self.connections_by_ip
            .get(&ip)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn test_limits() -> ConnectionLimits {
        ConnectionLimits {
            per_key: 3,
            per_ip: 5,
            global: 10,
            idle_timeout_secs: 300,
        }
    }

    #[test]
    fn test_add_connection_within_limits() {
        let mut manager = ConnectionManager::new(test_limits());
        let api_key = "test-key-hash";
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let conn1 = manager.try_add_connection(api_key, ip).unwrap();
        assert_eq!(manager.connection_count(), 1);
        assert_eq!(manager.connection_count_for_key(api_key), 1);
        assert_eq!(manager.connection_count_for_ip(ip), 1);
    }

    #[test]
    fn test_per_key_limit() {
        let mut manager = ConnectionManager::new(test_limits());
        let api_key = "test-key-hash";
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Add up to limit
        for _ in 0..3 {
            manager.try_add_connection(api_key, ip).unwrap();
        }

        // Next one should fail
        let result = manager.try_add_connection(api_key, ip);
        assert!(matches!(
            result,
            Err(ConnectionLimitError::PerKeyLimitExceeded(3))
        ));
    }

    #[test]
    fn test_per_ip_limit() {
        let mut manager = ConnectionManager::new(test_limits());
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Add up to limit with different keys
        for i in 0..5 {
            let api_key = format!("key-{}", i);
            manager.try_add_connection(&api_key, ip).unwrap();
        }

        // Next one should fail
        let result = manager.try_add_connection("key-6", ip);
        assert!(matches!(
            result,
            Err(ConnectionLimitError::PerIpLimitExceeded(5))
        ));
    }

    #[test]
    fn test_global_limit() {
        let mut manager = ConnectionManager::new(test_limits());

        // Add up to global limit with different keys and IPs
        for i in 0..10 {
            let api_key = format!("key-{}", i);
            let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8));
            manager.try_add_connection(&api_key, ip).unwrap();
        }

        // Next one should fail
        let result = manager.try_add_connection("key-11", IpAddr::V4(Ipv4Addr::new(127, 0, 0, 11)));
        assert!(matches!(
            result,
            Err(ConnectionLimitError::GlobalLimitExceeded(10))
        ));
    }

    #[test]
    fn test_remove_connection() {
        let mut manager = ConnectionManager::new(test_limits());
        let api_key = "test-key-hash";
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let conn_id = manager.try_add_connection(api_key, ip).unwrap();
        assert_eq!(manager.connection_count(), 1);

        manager.remove_connection(conn_id);
        assert_eq!(manager.connection_count(), 0);
        assert_eq!(manager.connection_count_for_key(api_key), 0);
        assert_eq!(manager.connection_count_for_ip(ip), 0);
    }
}

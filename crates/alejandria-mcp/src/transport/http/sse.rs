//! Server-Sent Events (SSE) manager
//!
//! Implements per-connection broadcast channels for isolated SSE streams.
//! Each connection gets its own channel to prevent data leakage (ID-003 mitigation).

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// SSE event type
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SseEvent {
    /// Connection established
    #[serde(rename = "connection")]
    Connection {
        session_id: String,
        timestamp: String,
    },
    
    /// JSON-RPC notification (method call result)
    #[serde(rename = "notification")]
    Notification {
        method: String,
        result: serde_json::Value,
        timestamp: String,
    },
    
    /// Heartbeat ping (keep-alive)
    #[serde(rename = "heartbeat")]
    Heartbeat {
        timestamp: String,
    },
    
    /// Server shutdown
    #[serde(rename = "shutdown")]
    Shutdown {
        reason: String,
        timestamp: String,
    },
}

/// Connection metadata
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Unique connection ID
    pub connection_id: Uuid,
    
    /// API key hash (for filtering events)
    pub api_key_hash: String,
    
    /// Session ID (for multi-tenant isolation)
    pub session_id: String,
    
    /// Connection timestamp
    pub connected_at: chrono::DateTime<chrono::Utc>,
    
    /// Broadcast sender for this connection
    pub sender: broadcast::Sender<SseEvent>,
}

/// SSE manager - maintains per-connection broadcast channels
pub struct SseManager {
    /// Active connections indexed by connection ID
    connections: Arc<RwLock<HashMap<Uuid, ConnectionInfo>>>,
    
    /// Channel capacity (max buffered events per connection)
    channel_capacity: usize,
}

impl SseManager {
    /// Create a new SSE manager
    pub fn new(channel_capacity: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            channel_capacity,
        }
    }
    
    /// Register a new SSE connection
    ///
    /// Returns (connection_id, receiver) for the SSE stream
    pub async fn register_connection(
        &self,
        api_key_hash: String,
        session_id: String,
    ) -> (Uuid, broadcast::Receiver<SseEvent>) {
        let connection_id = Uuid::new_v4();
        let (sender, receiver) = broadcast::channel(self.channel_capacity);
        
        let info = ConnectionInfo {
            connection_id,
            api_key_hash,
            session_id,
            connected_at: chrono::Utc::now(),
            sender,
        };
        
        self.connections.write().await.insert(connection_id, info);
        
        (connection_id, receiver)
    }
    
    /// Unregister a connection
    pub async fn unregister_connection(&self, connection_id: Uuid) {
        self.connections.write().await.remove(&connection_id);
    }
    
    /// Send event to a specific connection
    pub async fn send_to_connection(&self, connection_id: Uuid, event: SseEvent) -> Result<(), String> {
        let connections = self.connections.read().await;
        
        if let Some(conn) = connections.get(&connection_id) {
            conn.sender.send(event)
                .map(|_| ())
                .map_err(|e| format!("Failed to send event: {}", e))
        } else {
            Err(format!("Connection {} not found", connection_id))
        }
    }
    
    /// Broadcast event to all connections with matching session_id
    ///
    /// This provides multi-tenant isolation - events only go to clients
    /// authenticated with the same session/API key.
    pub async fn broadcast_to_session(&self, session_id: &str, event: SseEvent) {
        let connections = self.connections.read().await;
        
        for conn in connections.values() {
            if conn.session_id == session_id {
                // Ignore send errors (connection may have closed)
                let _ = conn.sender.send(event.clone());
            }
        }
    }
    
    /// Broadcast event to all active connections
    ///
    /// WARNING: Only use for global events (shutdown, server status).
    /// For user-specific events, use broadcast_to_session() instead.
    pub async fn broadcast_to_all(&self, event: SseEvent) {
        let connections = self.connections.read().await;
        
        for conn in connections.values() {
            let _ = conn.sender.send(event.clone());
        }
    }
    
    /// Get number of active connections
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
    
    /// Get number of connections for a specific session
    pub async fn session_connection_count(&self, session_id: &str) -> usize {
        self.connections.read().await
            .values()
            .filter(|conn| conn.session_id == session_id)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_unregister() {
        let manager = SseManager::new(10);
        
        let (conn_id, _rx) = manager.register_connection(
            "key_hash".to_string(),
            "session_1".to_string(),
        ).await;
        
        assert_eq!(manager.connection_count().await, 1);
        
        manager.unregister_connection(conn_id).await;
        
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let manager = SseManager::new(10);
        
        let (_conn1, mut rx1) = manager.register_connection(
            "key_hash_1".to_string(),
            "session_1".to_string(),
        ).await;
        
        let (_conn2, mut rx2) = manager.register_connection(
            "key_hash_2".to_string(),
            "session_2".to_string(),
        ).await;
        
        // Broadcast to session_1 only
        let event = SseEvent::Heartbeat {
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        
        manager.broadcast_to_session("session_1", event).await;
        
        // rx1 should receive the event
        assert!(rx1.try_recv().is_ok());
        
        // rx2 should NOT receive the event (different session)
        assert!(rx2.try_recv().is_err());
    }
}

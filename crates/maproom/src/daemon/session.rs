use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use thiserror::Error;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::daemon::protocol::JsonRpcMessage;

/// Error types for session operations
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Session channel closed: {0}")]
    ChannelClosed(Uuid),
}

/// Represents a connected client session
pub struct Session {
    pub id: Uuid,
    pub connected_at: Instant,
    response_tx: mpsc::UnboundedSender<JsonRpcMessage>,
}

impl Session {
    pub fn new(response_tx: mpsc::UnboundedSender<JsonRpcMessage>) -> Self {
        Self {
            id: Uuid::new_v4(),
            connected_at: Instant::now(),
            response_tx,
        }
    }

    pub fn send_response(&self, response: JsonRpcMessage) -> Result<(), SessionError> {
        self.response_tx
            .send(response)
            .map_err(|_| SessionError::ChannelClosed(self.id))
    }

    pub fn duration(&self) -> std::time::Duration {
        self.connected_at.elapsed()
    }
}

/// Thread-safe registry of active client sessions
pub struct SessionRegistry {
    sessions: DashMap<Uuid, Session>,
    active_count: AtomicUsize,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            active_count: AtomicUsize::new(0),
        }
    }

    /// Register a new client session
    pub fn register(&self, response_tx: mpsc::UnboundedSender<JsonRpcMessage>) -> Uuid {
        let session = Session::new(response_tx);
        let session_id = session.id;

        tracing::info!(
            session_id = %session_id,
            "Client connected"
        );

        self.sessions.insert(session_id, session);
        self.active_count.fetch_add(1, Ordering::SeqCst);

        session_id
    }

    /// Unregister a client session
    pub fn unregister(&self, session_id: &Uuid) {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            let duration = session.duration();
            tracing::info!(
                session_id = %session_id,
                duration_secs = duration.as_secs(),
                "Client disconnected"
            );

            self.active_count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Get number of active sessions (for idle timeout logic)
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Send response to specific session
    pub fn send_to_session(
        &self,
        session_id: &Uuid,
        response: JsonRpcMessage,
    ) -> Result<(), SessionError> {
        self.sessions
            .get(session_id)
            .ok_or(SessionError::SessionNotFound(*session_id))?
            .send_response(response)
    }

    /// Get session by ID (for inspection, not routine use)
    pub fn get_session(
        &self,
        session_id: &Uuid,
    ) -> Option<dashmap::mapref::one::Ref<Uuid, Session>> {
        self.sessions.get(session_id)
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::types::JsonRpcResponse;
    use std::sync::Arc;

    // Helper to create a test response
    fn test_response(id: i32) -> JsonRpcMessage {
        JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
            id: serde_json::json!(id),
        })
    }

    #[test]
    fn test_session_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let session = Session::new(tx);

        assert!(session.duration() < std::time::Duration::from_millis(100));
        assert_ne!(session.id, Uuid::nil());
    }

    #[test]
    fn test_session_send_response() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let session = Session::new(tx);

        let response = test_response(1);

        session.send_response(response.clone()).unwrap();

        let received = rx.try_recv().unwrap();
        // Verify response matches
        match (response, received) {
            (JsonRpcMessage::Response(r1), JsonRpcMessage::Response(r2)) => {
                assert_eq!(r1.id, r2.id);
                assert_eq!(r1.jsonrpc, r2.jsonrpc);
            }
            _ => panic!("Response type mismatch"),
        }
    }

    #[test]
    fn test_registry_register_unregister() {
        let registry = SessionRegistry::new();
        assert_eq!(registry.active_count(), 0);

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let session_id1 = registry.register(tx1);
        assert_eq!(registry.active_count(), 1);

        let (tx2, _rx2) = mpsc::unbounded_channel();
        let session_id2 = registry.register(tx2);
        assert_eq!(registry.active_count(), 2);

        registry.unregister(&session_id1);
        assert_eq!(registry.active_count(), 1);

        registry.unregister(&session_id2);
        assert_eq!(registry.active_count(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_registration() {
        let registry = Arc::new(SessionRegistry::new());
        let mut handles = vec![];

        // Spawn 10 concurrent threads registering and unregistering
        for _ in 0..10 {
            let registry = registry.clone();
            let handle = tokio::spawn(async move {
                let (tx, _rx) = mpsc::unbounded_channel();
                let session_id = registry.register(tx);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                registry.unregister(&session_id);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // All sessions should be unregistered
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_send_to_nonexistent_session() {
        let registry = SessionRegistry::new();
        let fake_id = Uuid::new_v4();

        let response = test_response(1);

        let result = registry.send_to_session(&fake_id, response);
        assert!(matches!(result, Err(SessionError::SessionNotFound(_))));
    }

    #[test]
    fn test_session_duration_tracking() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let session = Session::new(tx);

        std::thread::sleep(std::time::Duration::from_millis(100));
        let duration = session.duration();
        assert!(duration >= std::time::Duration::from_millis(100));
        assert!(duration < std::time::Duration::from_millis(200));
    }

    #[test]
    fn test_session_channel_closed_error() {
        let (tx, rx) = mpsc::unbounded_channel();
        let session = Session::new(tx);

        // Drop the receiver to close the channel
        drop(rx);

        let response = test_response(1);
        let result = session.send_response(response);
        assert!(matches!(result, Err(SessionError::ChannelClosed(_))));
    }

    #[test]
    fn test_get_session() {
        let registry = SessionRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let session_id = registry.register(tx);

        // Should be able to get the session
        let session_ref = registry.get_session(&session_id);
        assert!(session_ref.is_some());
        assert_eq!(session_ref.unwrap().id, session_id);

        // After unregister, should not be found
        registry.unregister(&session_id);
        assert!(registry.get_session(&session_id).is_none());
    }

    #[test]
    fn test_send_to_session_success() {
        let registry = SessionRegistry::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let session_id = registry.register(tx);

        let response = test_response(42);
        registry
            .send_to_session(&session_id, response.clone())
            .unwrap();

        let received = rx.try_recv().unwrap();
        match (response, received) {
            (JsonRpcMessage::Response(r1), JsonRpcMessage::Response(r2)) => {
                assert_eq!(r1.id, r2.id);
            }
            _ => panic!("Response type mismatch"),
        }
    }
}

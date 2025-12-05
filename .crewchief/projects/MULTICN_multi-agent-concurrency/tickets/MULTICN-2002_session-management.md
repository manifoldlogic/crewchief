# Ticket: MULTICN-2002: Session Management

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create session tracking infrastructure for managing connected clients. Implements Session struct with UUID tracking, SessionRegistry using DashMap for concurrent access, and atomic counter for idle timeout detection.

## Background

The socket server (MULTICN-2003) needs to track multiple connected clients and route responses back to the correct client. Session management provides the foundation for multiplexing requests/responses and implementing idle timeout (daemon shuts down when no clients connected for 5 minutes).

This is a focused MVP implementation - no per-session metrics or broadcast capabilities (deferred to post-MVP).

Reference: [architecture.md](../planning/architecture.md) - Session Management section

## Acceptance Criteria

- [ ] Session struct has essential fields: id (UUID), connected_at (Instant), response_tx (channel)
- [ ] SessionRegistry uses DashMap for lock-free concurrent access
- [ ] active_count() method returns AtomicUsize value
- [ ] register() logs connection and increments counter
- [ ] unregister() logs disconnection and decrements counter
- [ ] NO broadcast capability (explicitly deferred)
- [ ] NO per-session metrics like request_count (explicitly deferred)
- [ ] Test: Concurrent register/unregister from 10 threads maintains accurate count

## Technical Requirements

Create `crates/maproom/src/daemon/session.rs` implementing session tracking.

### Session Structure

```rust
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::daemon::protocol::JsonRpcMessage;

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
        self.response_tx.send(response)
            .map_err(|_| SessionError::ChannelClosed(self.id))
    }

    pub fn duration(&self) -> std::time::Duration {
        self.connected_at.elapsed()
    }
}
```

### SessionRegistry Structure

```rust
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

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
        response: JsonRpcMessage
    ) -> Result<(), SessionError> {
        self.sessions
            .get(session_id)
            .ok_or(SessionError::SessionNotFound(*session_id))?
            .send_response(response)
    }

    /// Get session by ID (for inspection, not routine use)
    pub fn get_session(&self, session_id: &Uuid) -> Option<dashmap::mapref::one::Ref<Uuid, Session>> {
        self.sessions.get(session_id)
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

### Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Session channel closed: {0}")]
    ChannelClosed(Uuid),
}
```

## Implementation Notes

### Why DashMap?

DashMap provides lock-free concurrent HashMap access:
- **No RwLock needed**: Internal sharding reduces contention
- **Better concurrency**: Multiple readers/writers without global lock
- **Mature library**: Battle-tested in production Rust systems

Alternative considered: `Arc<RwLock<HashMap>>` - rejected due to lock contention under concurrent access.

### Why AtomicUsize for active_count?

While DashMap.len() exists, using AtomicUsize provides:
- **Fast reads**: No HashMap traversal for idle timeout checks
- **Explicit lifecycle**: Increment/decrement makes session lifecycle clear
- **Future-proof**: Supports metrics without HashMap iteration

### Deferred Features (Explicitly Out of Scope)

**Not implementing in MVP:**
- **Per-session metrics**: request_count, bytes_sent, etc.
  - *Rationale*: Not needed for core functionality. Add if observability demands.

- **Broadcast capability**: send_to_all() method
  - *Rationale*: No current use case. Could be added for cache invalidation later.

- **Session timeout per client**: Individual client timeout tracking
  - *Rationale*: Global idle timeout (MULTICN-2004) is sufficient for MVP.

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

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

        let response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
            id: RequestId::Number(1),
        });

        session.send_response(response.clone()).unwrap();

        let received = rx.try_recv().unwrap();
        // Verify response matches
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

        let response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: None,
            id: RequestId::Number(1),
        });

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
}
```

## Dependencies

- MULTICN-2001 (JSON-RPC Codec) - uses JsonRpcMessage type
- dashmap crate (add to Cargo.toml if not present)

## Risk Assessment

- **Risk**: DashMap memory overhead vs standard HashMap
  - **Mitigation**: Overhead is small (sharding metadata). Acceptable for concurrent access benefits.

- **Risk**: UnboundedSender could cause memory issues if client stops reading
  - **Mitigation**: Sessions unregister on disconnect. Bounded channels add complexity without clear benefit for MVP.

- **Risk**: Active count could drift out of sync
  - **Mitigation**: Atomic operations ensure consistency. Unit tests verify correctness.

## Files/Packages Affected

- `crates/maproom/src/daemon/session.rs` (NEW)
- `crates/maproom/src/daemon/mod.rs` (MODIFY - add module export)
- `Cargo.toml` (ADD dashmap dependency if needed)

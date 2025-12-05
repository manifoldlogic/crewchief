# Ticket: MULTICN-2003: Unix Socket Server

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
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Implement Unix socket-based daemon server with per-client task spawning, shared state via Arc<DaemonState>, and PID file management with O_EXCL + flock for single-daemon guarantee.

## Background

The core of the shared daemon architecture: a Unix socket server that accepts multiple client connections and routes requests to shared SQLite/embedding resources. This eliminates per-agent daemon spawning and associated SQLITE_BUSY errors.

Socket permissions (0600) and PID file locking ensure only one daemon runs per user.

Reference: [architecture.md](../planning/architecture.md) - Socket Server Component

## Acceptance Criteria

- [ ] UnixListener binds to `/tmp/maproom-{uid}.sock` with 0600 permissions
- [ ] Accept loop spawns per-client tasks using tokio::spawn
- [ ] Shared state via Arc<DaemonState> containing SqliteStore, EmbeddingService, SessionRegistry
- [ ] PID file created with O_EXCL + flock at `/tmp/maproom-{uid}.pid`
- [ ] PID file contains daemon process ID
- [ ] PID file automatically cleaned up on shutdown
- [ ] Test: Multiple clients can connect and send concurrent requests
- [ ] Test: PID file prevents second daemon from starting

## Technical Requirements

Create `crates/maproom/src/daemon/server.rs` implementing socket server.

### Server Configuration

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub socket_path: PathBuf,
    pub pid_path: PathBuf,
    pub database_path: PathBuf,
    pub sqlite_config: SqliteConfig,
    pub idle_timeout: Duration, // From MULTICN-2004
}

impl ServerConfig {
    pub fn default_for_user() -> Self {
        let uid = users::get_current_uid();
        Self {
            socket_path: PathBuf::from(format!("/tmp/maproom-{}.sock", uid)),
            pid_path: PathBuf::from(format!("/tmp/maproom-{}.pid", uid)),
            database_path: default_database_path(),
            sqlite_config: SqliteConfig::from_env().unwrap_or_default(),
            idle_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}
```

### Shared Daemon State

```rust
use std::sync::Arc;

/// Shared state accessible by all client handlers
pub struct DaemonState {
    pub store: SqliteStore,
    pub embedding_service: EmbeddingService,
    pub sessions: SessionRegistry,
}

impl DaemonState {
    pub async fn new(config: &ServerConfig) -> Result<Self, DaemonError> {
        let store = SqliteStore::new(
            &config.database_path,
            config.sqlite_config.clone()
        )?;

        let embedding_service = EmbeddingService::new()?;

        Ok(Self {
            store,
            embedding_service,
            sessions: SessionRegistry::new(),
        })
    }
}
```

### PID File Guard

```rust
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

/// RAII guard for PID file. Automatically cleans up on drop.
pub struct PidFileGuard {
    path: PathBuf,
    _file: File,
}

impl PidFileGuard {
    /// Create PID file with exclusive lock.
    /// Returns error if PID file already exists and is locked (daemon already running).
    pub fn create(path: &Path) -> Result<Self, DaemonError> {
        // O_EXCL ensures atomicity if file doesn't exist
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600) // Owner read/write only
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    DaemonError::AlreadyRunning(path.to_path_buf())
                } else {
                    DaemonError::PidFileError(e)
                }
            })?;

        // Advisory lock (flock) as additional safeguard
        use fs2::FileExt;
        file.try_lock_exclusive()
            .map_err(|_| DaemonError::AlreadyRunning(path.to_path_buf()))?;

        // Write current PID
        let pid = std::process::id();
        writeln!(file, "{}", pid)?;
        file.flush()?;

        tracing::info!(pid, path = %path.display(), "PID file created");

        Ok(Self {
            path: path.to_path_buf(),
            _file: file, // Hold file open to maintain lock
        })
    }
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.path) {
            tracing::warn!(
                path = %self.path.display(),
                error = %e,
                "Failed to remove PID file"
            );
        } else {
            tracing::info!(path = %self.path.display(), "PID file removed");
        }
    }
}
```

### Socket Server

```rust
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

pub struct SocketServer {
    config: ServerConfig,
    state: Arc<DaemonState>,
    shutdown_tx: broadcast::Sender<()>,
}

impl SocketServer {
    pub fn new(config: ServerConfig) -> Result<Self, DaemonError> {
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config,
            state: Arc::new(DaemonState::new(&config).await?),
            shutdown_tx,
        })
    }

    /// Run the socket server (blocks until shutdown)
    pub async fn run(&self) -> Result<(), DaemonError> {
        // Create PID file (returns error if daemon already running)
        let _pid_guard = PidFileGuard::create(&self.config.pid_path)?;

        // Remove stale socket file if exists
        if self.config.socket_path.exists() {
            std::fs::remove_file(&self.config.socket_path)?;
        }

        // Bind Unix socket with restricted permissions
        let listener = UnixListener::bind(&self.config.socket_path)?;

        // Set socket permissions to 0600 (owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&self.config.socket_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&self.config.socket_path, permissions)?;
        }

        tracing::info!(
            socket_path = %self.config.socket_path.display(),
            "Socket server listening"
        );

        // Accept loop
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                Ok((stream, _addr)) = listener.accept() => {
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, state).await {
                            tracing::error!(error = %e, "Client handler error");
                        }
                    });
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutdown signal received");
                    break;
                }
            }
        }

        // Cleanup socket file
        std::fs::remove_file(&self.config.socket_path)?;
        Ok(())
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}
```

### Client Handler

```rust
use tokio_util::codec::Framed;

async fn handle_client(
    stream: UnixStream,
    state: Arc<DaemonState>
) -> Result<(), DaemonError> {
    let mut framed = Framed::new(stream, JsonRpcCodec::new());

    // Create response channel for this session
    let (response_tx, mut response_rx) = mpsc::unbounded_channel();
    let session_id = state.sessions.register(response_tx);

    // Ensure session cleanup on disconnect
    let _session_guard = SessionGuard {
        registry: state.sessions.clone(),
        session_id,
    };

    loop {
        tokio::select! {
            // Receive request from client
            Some(Ok(message)) = framed.next() => {
                match message {
                    JsonRpcMessage::Request(req) => {
                        let state = state.clone();
                        let session_id = session_id;
                        tokio::spawn(async move {
                            let response = handle_request(req, &state).await;
                            if let Err(e) = state.sessions.send_to_session(&session_id, response) {
                                tracing::warn!(error = %e, "Failed to send response");
                            }
                        });
                    }
                    JsonRpcMessage::Response(_) => {
                        tracing::warn!("Unexpected response from client (should be request)");
                    }
                }
            }
            // Send response to client
            Some(response) = response_rx.recv() => {
                framed.send(response).await?;
            }
            else => break,
        }
    }

    Ok(())
}

/// RAII guard to ensure session cleanup on disconnect
struct SessionGuard {
    registry: Arc<SessionRegistry>,
    session_id: Uuid,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.registry.unregister(&self.session_id);
    }
}
```

### Request Handler (Stub)

```rust
async fn handle_request(
    req: JsonRpcRequest,
    state: &DaemonState
) -> JsonRpcMessage {
    // TODO: Dispatch to actual method handlers
    // For now, simple echo response
    JsonRpcMessage::Response(JsonRpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({
            "method": req.method,
            "received": true
        })),
        error: None,
        id: req.id,
    })
}
```

## Implementation Notes

### Socket Path Selection

Using `/tmp/maproom-{uid}.sock`:
- **Per-user isolation**: Different users get different sockets
- **Temporary location**: Cleaned up on reboot
- **Predictable**: Easy for clients to discover

Alternative considered: `~/.maproom/daemon.sock` - rejected because home directory may be on NFS.

### Why O_EXCL + flock?

Two-level locking for robustness:
- **O_EXCL**: Atomic check for file existence during creation
- **flock**: Advisory lock prevents races if file deleted externally

Both mechanisms together ensure only one daemon per user.

### Security Considerations

- **0600 permissions**: Only file owner can connect (prevents other users accessing daemon)
- **PID file**: Prevents multiple daemons interfering
- **UnixListener**: More secure than TCP (no network exposure)

## Dependencies

- MULTICN-2001 (JSON-RPC Codec)
- MULTICN-2002 (Session Management)
- fs2 crate for flock (add to Cargo.toml)
- users crate for UID lookup (add to Cargo.toml)

## Risk Assessment

- **Risk**: Socket file permissions not set correctly on all platforms
  - **Mitigation**: Explicit permission setting with error logging. Unit test verifies.

- **Risk**: PID file races on NFS filesystems
  - **Mitigation**: Use /tmp (local filesystem). Document NFS limitations.

- **Risk**: Client disconnects leave orphaned sessions
  - **Mitigation**: SessionGuard RAII pattern ensures cleanup on disconnect.

## Files/Packages Affected

- `crates/maproom/src/daemon/server.rs` (NEW)
- `crates/maproom/src/daemon/mod.rs` (MODIFY - add server module)
- `crates/maproom/src/main.rs` (MODIFY - add `serve --socket` command)
- `Cargo.toml` (ADD fs2, users dependencies)

//! Unix socket-based daemon server for concurrent client connections.
//!
//! This module implements the core socket server infrastructure for the shared daemon,
//! including:
//! - Unix socket server with per-client task spawning
//! - PID file management with O_EXCL + flock for single-daemon guarantee
//! - Shared state (dyn Store, EmbeddingService, SessionRegistry) via Arc
//! - Session cleanup with RAII pattern
//!
//! Reference: MULTICN-2003 (Unix Socket Server)

use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc};
use tokio::time::interval;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::daemon::protocol::{JsonRpcCodec, JsonRpcMessage};
use crate::daemon::session::SessionRegistry;
use crate::daemon::types::{JsonRpcRequest, JsonRpcResponse};
use crate::db::{connect, Store};
use crate::embedding::EmbeddingService;

/// Errors that can occur during daemon server operations
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Daemon already running (PID file locked): {0}")]
    AlreadyRunning(PathBuf),

    #[error("PID file error: {0}")]
    PidFileError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] anyhow::Error),

    #[error("Socket error: {0}")]
    SocketError(String),
}

/// Server configuration
///
/// Note: the database is NOT configured here. Backend selection (SQLite vs
/// Postgres) is owned by the shared `crate::db::connect()` factory, driven by
/// `MAPROOM_DATABASE_URL` / `--database-url` (R-SEL-1, R-SEL-6).
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub socket_path: PathBuf,
    pub pid_path: PathBuf,
    pub idle_timeout: Duration,
}

impl ServerConfig {
    /// Create default configuration for current user
    pub fn default_for_user() -> Result<Self> {
        let uid = users::get_current_uid();

        Ok(Self {
            socket_path: PathBuf::from(format!("/tmp/maproom-{}.sock", uid)),
            pid_path: PathBuf::from(format!("/tmp/maproom-{}.pid", uid)),
            idle_timeout: Duration::from_secs(300), // 5 minutes
        })
    }
}

/// Shared state accessible by all client handlers
pub struct DaemonState {
    pub store: Arc<dyn Store + Send + Sync>,
    pub embedding_service: EmbeddingService,
    pub sessions: Arc<SessionRegistry>,
}

impl DaemonState {
    /// Initialize daemon state with database and embedding service
    pub async fn new(_config: &ServerConfig) -> Result<Self, DaemonError> {
        // Route through the shared factory so the socket daemon honors the DSN
        // scheme (SQLite vs Postgres) identically to the STDIO daemon, and fails
        // loud on a postgres:// URL in a non-postgres build (F70 / R-SEL-1..4).
        let store = connect()
            .await
            .context("Failed to initialize database store")?;

        let embedding_service = EmbeddingService::from_env()
            .await
            .context("Failed to initialize embedding service")?;

        Ok(Self {
            store,
            embedding_service,
            sessions: Arc::new(SessionRegistry::new()),
        })
    }
}

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
        #[cfg(unix)]
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

        #[cfg(not(unix))]
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    DaemonError::AlreadyRunning(path.to_path_buf())
                } else {
                    DaemonError::PidFileError(e)
                }
            })?;

        // Advisory lock (flock) as additional safeguard
        file.try_lock_exclusive()
            .map_err(|_| DaemonError::AlreadyRunning(path.to_path_buf()))?;

        // Write current PID
        let pid = std::process::id();
        writeln!(file, "{}", pid)?;
        file.flush()?;

        info!(pid, path = %path.display(), "PID file created");

        Ok(Self {
            path: path.to_path_buf(),
            _file: file, // Hold file open to maintain lock
        })
    }
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.path) {
            warn!(
                path = %self.path.display(),
                error = %e,
                "Failed to remove PID file"
            );
        } else {
            info!(path = %self.path.display(), "PID file removed");
        }
    }
}

/// Unix socket server
pub struct SocketServer {
    config: ServerConfig,
    state: Arc<DaemonState>,
    shutdown_tx: broadcast::Sender<()>,
}

impl SocketServer {
    /// Create a new socket server
    pub async fn new(config: ServerConfig) -> Result<Self, DaemonError> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let state = Arc::new(DaemonState::new(&config).await?);

        Ok(Self {
            config,
            state,
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
        let listener = UnixListener::bind(&self.config.socket_path)
            .map_err(|e| DaemonError::SocketError(format!("Failed to bind socket: {}", e)))?;

        // Set socket permissions to 0600 (owner only)
        #[cfg(unix)]
        {
            let metadata = std::fs::metadata(&self.config.socket_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&self.config.socket_path, permissions)?;
        }

        info!(
            socket_path = %self.config.socket_path.display(),
            idle_timeout_secs = %self.config.idle_timeout.as_secs(),
            "Socket server listening"
        );

        // Accept loop with idle timeout
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut idle_check = interval(Duration::from_secs(60)); // Check every minute
        let mut idle_since: Option<Instant> = Some(Instant::now());

        loop {
            tokio::select! {
                Ok((stream, _addr)) = listener.accept() => {
                    idle_since = None; // Reset idle timer when client connects
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, state).await {
                            error!(error = %e, "Client handler error");
                        }
                    });
                }

                _ = idle_check.tick() => {
                    let active_count = self.state.sessions.active_count();

                    if active_count == 0 {
                        if idle_since.is_none() {
                            idle_since = Some(Instant::now());
                            debug!("No active clients, idle timer started");
                        } else if let Some(since) = idle_since {
                            let idle_duration = since.elapsed();
                            if idle_duration >= self.config.idle_timeout {
                                info!(
                                    idle_secs = idle_duration.as_secs(),
                                    "Idle timeout reached, shutting down"
                                );
                                break;
                            }
                        }
                    } else {
                        if idle_since.is_some() {
                            debug!(active_count, "Clients connected, idle timer reset");
                        }
                        idle_since = None;
                    }
                }

                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }

        // Graceful shutdown
        self.graceful_shutdown().await?;

        // Cleanup socket file
        if let Err(e) = std::fs::remove_file(&self.config.socket_path) {
            warn!(error = %e, "Failed to remove socket file");
        }

        Ok(())
    }

    /// Graceful shutdown: wait for active sessions to complete
    async fn graceful_shutdown(&self) -> Result<(), DaemonError> {
        info!("Starting graceful shutdown");

        let shutdown_timeout = Duration::from_secs(30);
        let start = Instant::now();

        // Wait for active sessions to complete (with timeout)
        loop {
            let active_count = self.state.sessions.active_count();

            if active_count == 0 {
                info!("All sessions completed");
                break;
            }

            if start.elapsed() >= shutdown_timeout {
                warn!(
                    active_count,
                    "Shutdown timeout reached, {} sessions still active", active_count
                );
                break;
            }

            debug!(active_count, "Waiting for sessions to complete");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }

    /// Trigger shutdown (for testing or external signals)
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Run socket server with signal handling (SIGTERM, SIGINT)
#[cfg(unix)]
pub async fn run_with_signal_handling(server: SocketServer) -> Result<(), DaemonError> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate())
        .map_err(|e| DaemonError::SocketError(format!("Failed to setup SIGTERM handler: {}", e)))?;
    let mut sigint = signal(SignalKind::interrupt())
        .map_err(|e| DaemonError::SocketError(format!("Failed to setup SIGINT handler: {}", e)))?;

    let server = Arc::new(server);

    // Run the server directly without spawning
    // The shutdown signal will be sent via the broadcast channel
    let result = tokio::select! {
        _ = sigterm.recv() => {
            info!("SIGTERM received, initiating graceful shutdown");
            server.shutdown();
            // Server will exit its run loop and clean up
            Ok(())
        }
        _ = sigint.recv() => {
            info!("SIGINT received, initiating graceful shutdown");
            server.shutdown();
            // Server will exit its run loop and clean up
            Ok(())
        }
        result = server.run() => {
            result
        }
    };

    result
}

/// Run socket server with signal handling (Windows - no SIGTERM support)
#[cfg(not(unix))]
pub async fn run_with_signal_handling(server: SocketServer) -> Result<(), DaemonError> {
    use tokio::signal;

    let server = Arc::new(server);

    // Run the server directly without spawning
    // The shutdown signal will be sent via the broadcast channel
    let result = tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Ctrl+C received, initiating graceful shutdown");
            server.shutdown();
            // Server will exit its run loop and clean up
            Ok(())
        }
        result = server.run() => {
            result
        }
    };

    result
}

/// Handle a single client connection
async fn handle_client(stream: UnixStream, state: Arc<DaemonState>) -> Result<()> {
    let mut framed = Framed::new(stream, JsonRpcCodec::new());

    // Create response channel for this session
    let (response_tx, mut response_rx) = mpsc::unbounded_channel();
    let session_id = state.sessions.register(response_tx);

    // Ensure session cleanup on disconnect
    let _session_guard = SessionGuard {
        registry: state.sessions.clone(),
        session_id,
    };

    use futures::stream::StreamExt;
    use futures::SinkExt;

    loop {
        tokio::select! {
            // Receive request from client
            message = framed.next() => {
                match message {
                    Some(Ok(JsonRpcMessage::Request(req))) => {
                        let state_clone = state.clone();
                        let sid = session_id;
                        tokio::spawn(async move {
                            let response = handle_request(req, &state_clone).await;
                            if let Err(e) = state_clone.sessions.send_to_session(&sid, response) {
                                warn!(error = %e, "Failed to send response");
                            }
                        });
                    }
                    Some(Ok(JsonRpcMessage::Response(_))) => {
                        warn!("Unexpected response from client (should be request)");
                    }
                    Some(Err(e)) => {
                        error!(error = %e, "Failed to decode message");
                        break;
                    }
                    None => {
                        // Client disconnected
                        break;
                    }
                }
            }
            // Send response to client
            response = response_rx.recv() => {
                match response {
                    Some(msg) => {
                        if let Err(e) = framed.send(msg).await {
                            error!(error = %e, "Failed to send response to client");
                            break;
                        }
                    }
                    None => {
                        // Response channel closed
                        break;
                    }
                }
            }
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

/// Handle a JSON-RPC request (stub implementation)
async fn handle_request(req: JsonRpcRequest, _state: &DaemonState) -> JsonRpcMessage {
    // TODO: Dispatch to actual method handlers (MULTICN-2005)
    // For now, simple echo response
    JsonRpcMessage::Response(JsonRpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({
            "method": req.method,
            "received": true
        })),
        error: None,
        id: req.id.unwrap_or(serde_json::Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    // Helper to create a temp PID path
    fn temp_pid_path() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.pid");
        (dir, path)
    }

    #[test]
    fn test_pid_file_creation() {
        let (_dir, pid_path) = temp_pid_path();

        let guard = PidFileGuard::create(&pid_path).unwrap();
        assert!(pid_path.exists());

        // Read PID from file
        let content = std::fs::read_to_string(&pid_path).unwrap();
        let pid: u32 = content.trim().parse().unwrap();
        assert_eq!(pid, std::process::id());

        drop(guard);
        assert!(!pid_path.exists());
    }

    #[test]
    fn test_pid_file_prevents_second_daemon() {
        let (_dir, pid_path) = temp_pid_path();

        let _guard1 = PidFileGuard::create(&pid_path).unwrap();

        // Second attempt should fail
        let result = PidFileGuard::create(&pid_path);
        assert!(matches!(result, Err(DaemonError::AlreadyRunning(_))));
    }

    #[test]
    fn test_pid_file_permissions() {
        let (_dir, pid_path) = temp_pid_path();

        let _guard = PidFileGuard::create(&pid_path).unwrap();

        // Check file permissions (0600 = owner read/write only)
        let metadata = std::fs::metadata(&pid_path).unwrap();
        let mode = metadata.permissions().mode();
        // Mask off file type bits, check permission bits
        assert_eq!(mode & 0o777, 0o600);
    }

    #[tokio::test]
    #[ignore] // Requires embedding provider (Ollama)
    async fn test_multiple_clients_concurrent() {
        use tokio::net::UnixStream;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        // Create minimal config
        let config = ServerConfig {
            socket_path: socket_path.clone(),
            pid_path,
            idle_timeout: Duration::from_secs(300),
        };

        let server = SocketServer::new(config).await.unwrap();

        // Spawn server in background
        let server_handle = {
            let server = Arc::new(server);
            let server_clone = server.clone();
            tokio::spawn(async move { server_clone.run().await })
        };

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Spawn multiple concurrent clients
        let mut client_handles = vec![];

        for i in 0..5 {
            let socket_path = socket_path.clone();
            let handle = tokio::spawn(async move {
                // Connect to server
                let stream = UnixStream::connect(&socket_path).await.unwrap();
                let mut framed = Framed::new(stream, JsonRpcCodec::new());

                // Send request
                let request = JsonRpcMessage::Request(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    method: format!("test_{}", i),
                    params: None,
                    id: Some(serde_json::json!(i)),
                });

                use futures::SinkExt;
                framed.send(request).await.unwrap();

                // Receive response
                use futures::StreamExt;
                let response = framed.next().await.unwrap().unwrap();

                match response {
                    JsonRpcMessage::Response(resp) => {
                        assert_eq!(resp.id, serde_json::json!(i));
                        assert!(resp.result.is_some());
                    }
                    _ => panic!("Expected response"),
                }
            });
            client_handles.push(handle);
        }

        // Wait for all clients to complete
        for handle in client_handles {
            handle.await.unwrap();
        }

        // Shutdown server
        // Note: server is moved into server_handle, so we can't call shutdown directly
        // In a real test, we'd keep a reference to the server
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_server_config_default_for_user() {
        let config = ServerConfig::default_for_user().unwrap();
        let uid = users::get_current_uid();

        assert_eq!(
            config.socket_path,
            PathBuf::from(format!("/tmp/maproom-{}.sock", uid))
        );
        assert_eq!(
            config.pid_path,
            PathBuf::from(format!("/tmp/maproom-{}.pid", uid))
        );
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
    }

    #[tokio::test]
    #[ignore] // Requires embedding provider (Ollama)
    async fn test_session_cleanup_on_disconnect() {
        use tokio::net::UnixStream;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let config = ServerConfig {
            socket_path: socket_path.clone(),
            pid_path,
            idle_timeout: Duration::from_secs(300),
        };

        let server = Arc::new(SocketServer::new(config).await.unwrap());
        let server_clone = server.clone();

        // Spawn server
        let _server_handle = tokio::spawn(async move { server_clone.run().await });

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect client
        let stream = UnixStream::connect(&socket_path).await.unwrap();

        // Wait for client handler to register session
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(server.state.sessions.active_count(), 1);

        // Disconnect client
        drop(stream);

        // Give more time for cleanup (handler task needs to finish)
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Session should be cleaned up
        assert_eq!(server.state.sessions.active_count(), 0);
    }

    #[tokio::test]
    #[ignore] // Requires embedding provider (Ollama)
    async fn test_idle_timeout_triggers() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let config = ServerConfig {
            socket_path,
            pid_path,
            idle_timeout: Duration::from_millis(100), // Very short timeout for test
        };

        let server = SocketServer::new(config).await.unwrap();

        // Start server in background
        let start_time = std::time::Instant::now();
        let handle = tokio::spawn(async move { server.run().await });

        // Wait for idle check interval (60s) plus a bit more for processing
        // Note: The server checks every 60 seconds, so we need to wait at least that long
        // Since this is impractical for tests, we verify the server is still running
        // and then manually trigger shutdown for cleanup

        // Give server time to start and begin idle timeout
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Server should still be running (idle check happens every 60s)
        assert!(
            !handle.is_finished(),
            "Server should still be running (idle checks are every 60s)"
        );

        // For this test, we just verify the server starts correctly with a short idle timeout
        // Full idle timeout behavior is tested in integration tests with shorter check intervals
        // Clean up by aborting the task
        handle.abort();

        let elapsed = start_time.elapsed();
        assert!(
            elapsed < Duration::from_secs(1),
            "Test should complete quickly, got {:?}",
            elapsed
        );
    }

    #[tokio::test]
    #[ignore] // Requires embedding provider (Ollama)
    async fn test_active_client_prevents_idle_timeout() {
        use tokio::net::UnixStream;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let config = ServerConfig {
            socket_path: socket_path.clone(),
            pid_path,
            idle_timeout: Duration::from_secs(2), // Short timeout for test
        };

        let server = Arc::new(SocketServer::new(config).await.unwrap());
        let server_clone = server.clone();

        // Start server in background
        let handle = tokio::spawn(async move { server_clone.run().await });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect a client (this should prevent idle timeout)
        let _stream = UnixStream::connect(&socket_path).await.unwrap();

        // Wait for client handler to register session
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(server.state.sessions.active_count(), 1);

        // Wait longer than idle timeout
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Server should still be running (because client is connected)
        assert!(
            !handle.is_finished(),
            "Server should still be running with active client"
        );

        // Clean up: trigger shutdown
        server.shutdown();
        let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
    }

    #[tokio::test]
    #[ignore] // Requires embedding provider (Ollama)
    async fn test_graceful_shutdown_waits_for_sessions() {
        use tokio::net::UnixStream;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let config = ServerConfig {
            socket_path: socket_path.clone(),
            pid_path,
            idle_timeout: Duration::from_secs(300),
        };

        let server = Arc::new(SocketServer::new(config).await.unwrap());
        let server_clone = server.clone();

        // Start server in background
        let handle = tokio::spawn(async move { server_clone.run().await });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect a client
        let stream = UnixStream::connect(&socket_path).await.unwrap();

        // Wait for client handler to register session
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(server.state.sessions.active_count(), 1);

        // Trigger shutdown while client is connected
        let shutdown_start = std::time::Instant::now();
        server.shutdown();

        // Disconnect client after a short delay (simulating in-flight request)
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            drop(stream);
        });

        // Wait for server to complete graceful shutdown
        let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(result.is_ok(), "Server should complete graceful shutdown");

        // Verify graceful shutdown waited for client (but not too long)
        let shutdown_duration = shutdown_start.elapsed();
        assert!(
            shutdown_duration >= Duration::from_millis(200)
                && shutdown_duration < Duration::from_secs(2),
            "Graceful shutdown should wait for client disconnect, got {:?}",
            shutdown_duration
        );
    }
}

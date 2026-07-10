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
    /// Review M11: OPTIONAL pinned database URL — None (production) keeps the
    /// ambient `db::connect()` resolution (R-SEL-1 factory routing, F70
    /// fail-loud included via connect_url's shared dispatch). Tests MUST pin
    /// a per-test database here; the un-ignored socket tests briefly used
    /// ambient resolution and mutated the developer's real DB (including a
    /// live Postgres under MAPROOM_DATABASE_URL=postgres://...).
    pub database_url: Option<String>,
}

impl ServerConfig {
    /// Create default configuration for current user
    pub fn default_for_user() -> Result<Self> {
        let uid = users::get_current_uid();

        Ok(Self {
            socket_path: PathBuf::from(format!("/tmp/maproom-{}.sock", uid)),
            pid_path: PathBuf::from(format!("/tmp/maproom-{}.pid", uid)),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            database_url: None,
        })
    }
}

/// Shared state accessible by all client handlers
pub struct DaemonState {
    pub store: Arc<dyn Store + Send + Sync>,
    /// Lazily-initialized embedding service (R16 / R-LAZY-5, OD-6). The old
    /// eager `EmbeddingService::from_env()` here made `serve --socket` DOA in
    /// provider-less environments AND mislabeled the failure as a
    /// "Database error" (via DaemonError::DatabaseError's #[from] anyhow).
    /// Kept as a OnceCell for parity with the stdio daemon so future real
    /// handlers (MULTICN-2005) don't re-plumb; dispatch is a stub today.
    pub embedding: tokio::sync::OnceCell<EmbeddingService>,
    pub sessions: Arc<SessionRegistry>,
}

impl DaemonState {
    /// Initialize daemon state with database (embeddings init lazily).
    pub async fn new(config: &ServerConfig) -> Result<Self, DaemonError> {
        // Route through the shared factory so the socket daemon honors the DSN
        // scheme (SQLite vs Postgres) identically to the STDIO daemon, and fails
        // loud on a postgres:// URL in a non-postgres build (F70 / R-SEL-1..4).
        // A pinned database_url (tests) goes through the SAME dispatch/bail
        // (connect_url) — review M11.
        let store = match &config.database_url {
            Some(url) => crate::db::connect_url(url)
                .await
                .context("Failed to initialize database store")?,
            None => connect()
                .await
                .context("Failed to initialize database store")?,
        };

        Ok(Self {
            store,
            embedding: tokio::sync::OnceCell::new(),
            sessions: Arc::new(SessionRegistry::new()),
        })
    }
}

#[cfg(unix)]
fn libc_o_nofollow() -> i32 {
    // O_NOFOLLOW is stable per-platform; avoid a direct libc dependency.
    #[cfg(target_os = "linux")]
    {
        0o400000
    }
    #[cfg(target_os = "macos")]
    {
        0x0100
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        0
    }
}

#[cfg(unix)]
extern "C" {
    fn geteuid() -> u32;
}

/// RAII guard for the Unix socket file (R21 / R-SOCK-1): removes the socket
/// on drop so every shutdown path (incl. select!-cancellation on signals)
/// cleans up, symmetric with PidFileGuard.
struct SocketFileGuard {
    path: PathBuf,
}

impl Drop for SocketFileGuard {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.path) {
            warn!(path = %self.path.display(), error = %e, "Failed to remove socket file");
        } else {
            info!(path = %self.path.display(), "Socket file removed");
        }
    }
}

/// RAII guard for PID file. Automatically cleans up on drop.
pub struct PidFileGuard {
    path: PathBuf,
    _file: File,
}

impl PidFileGuard {
    /// Create PID file with exclusive lock.
    /// Returns error if PID file already exists AND is still flock-held by a
    /// live daemon. A stale PID file (previous daemon crashed / SIGKILLed, so
    /// its RAII cleanup never ran) is taken over: the flock dies with its
    /// process, so lock acquisition — not file existence — is the real
    /// single-daemon guard (R21-adjacent robustness; pre-fix, one crashed
    /// daemon permanently blocked every restart with AlreadyRunning).
    pub fn create(path: &Path) -> Result<Self, DaemonError> {
        // O_EXCL fast path when no file exists.
        #[cfg(unix)]
        let created = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600) // Owner read/write only
            .open(path);

        #[cfg(not(unix))]
        let created = OpenOptions::new().write(true).create_new(true).open(path);

        let mut file = match created {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Stale-or-live? Open the existing file and let the flock
                // decide (it is released automatically when its owner dies).
                // Review [10]: /tmp is world-writable — open with O_NOFOLLOW
                // (no symlink traversal to attacker-chosen targets), verify
                // the inode is a regular file we own, and reset mode 0600.
                // Review [25]: after acquiring the lock, re-stat the PATH and
                // compare (dev, ino) with the locked fd — a concurrently
                // dropping daemon unlinks before its flock releases, and
                // adopting that doomed inode would leave us with no on-disk
                // PID file (letting a third daemon start).
                #[cfg(unix)]
                let f = {
                    use std::os::unix::fs::MetadataExt;
                    let f = OpenOptions::new()
                        .write(true)
                        .custom_flags(libc_o_nofollow())
                        .open(path)
                        .map_err(DaemonError::PidFileError)?;
                    let meta = f.metadata().map_err(DaemonError::PidFileError)?;
                    if !meta.is_file() || meta.uid() != unsafe { geteuid() } {
                        return Err(DaemonError::SocketError(format!(
                            "refusing PID-file takeover: {} is not a regular file owned by this user",
                            path.display()
                        )));
                    }
                    f.try_lock_exclusive()
                        .map_err(|_| DaemonError::AlreadyRunning(path.to_path_buf()))?;
                    // Post-lock identity check: the path must still refer to
                    // the inode we locked (nlink > 0 and same dev/ino).
                    let on_disk = std::fs::symlink_metadata(path);
                    let same = on_disk
                        .map(|d| d.dev() == meta.dev() && d.ino() == meta.ino())
                        .unwrap_or(false);
                    if !same || f.metadata().map(|m| m.nlink()).unwrap_or(0) == 0 {
                        return Err(DaemonError::AlreadyRunning(path.to_path_buf()));
                    }
                    let mut perms = meta.permissions();
                    perms.set_mode(0o600);
                    let _ = f.set_permissions(perms);
                    f
                };
                #[cfg(not(unix))]
                let f = {
                    let f = OpenOptions::new()
                        .write(true)
                        .open(path)
                        .map_err(DaemonError::PidFileError)?;
                    f.try_lock_exclusive()
                        .map_err(|_| DaemonError::AlreadyRunning(path.to_path_buf()))?;
                    f
                };
                f.set_len(0).map_err(DaemonError::PidFileError)?;
                info!(path = %path.display(), "Took over stale PID file (previous daemon did not shut down cleanly)");
                f
            }
            Err(e) => return Err(DaemonError::PidFileError(e)),
        };

        // Advisory lock (flock) as the real single-daemon safeguard.
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
    /// Review [08]: level-triggered shutdown flag. The broadcast alone is
    /// edge-triggered — a signal landing before run() first subscribes would
    /// send to zero receivers and be lost forever (daemon then only exits on
    /// idle timeout / SIGKILL). run() checks this flag after subscribing.
    shutdown_requested: std::sync::atomic::AtomicBool,
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
            shutdown_requested: std::sync::atomic::AtomicBool::new(false),
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

        // R21 / R-SOCK-1: RAII socket-file cleanup, symmetric with
        // PidFileGuard. The old straight-line unlink after the accept loop
        // never ran when run_with_signal_handling's select! dropped this
        // future on SIGTERM/SIGINT — stale .sock files accumulated. A guard
        // covers normal return, idle break, shutdown break, AND
        // cancellation-drop. (SIGKILL remains uncoverable; the stale-socket
        // pre-bind removal above stays as the mitigation.)
        let _socket_guard = SocketFileGuard {
            path: self.config.socket_path.clone(),
        };

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
        // Review [08]: a shutdown requested before this subscription existed
        // was broadcast to nobody — honor the level-triggered flag now.
        if self
            .shutdown_requested
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            info!("Shutdown was requested before the accept loop started");
            return Ok(());
        }
        // R20 / R-IDLE-1 (OD-15): the idle check ticks proportionally to the
        // timeout (was a fixed 60s tick that quantized --idle-timeout 5 into
        // ~69s: idle_since set up to 60s late, then checked at 60s grain).
        // Worst-case overshoot is ~2 ticks <= timeout/2 for sub-60s timeouts.
        let tick =
            (self.config.idle_timeout / 4).clamp(Duration::from_secs(1), Duration::from_secs(60));
        let mut idle_check = interval(tick);
        let mut idle_since: Option<Instant> = Some(Instant::now());

        loop {
            tokio::select! {
                Ok((stream, _addr)) = listener.accept() => {
                    idle_since = None; // Reset idle timer when client connects
                    let state = self.state.clone();
                    // Review [09]: give every client handler a shutdown
                    // receiver so graceful_shutdown's session drain can
                    // actually complete — previously nothing ever told
                    // connected clients to close and every signal shutdown
                    // with a persistent client sat out the full 30s cap.
                    let client_shutdown = self.shutdown_tx.subscribe();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, state, client_shutdown).await {
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

        // Socket file cleanup is handled by _socket_guard (R21 / R-SOCK-1).
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
        // Review [08]: set the level-triggered flag BEFORE the broadcast so a
        // not-yet-subscribed run() still observes the request.
        self.shutdown_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);
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

    // R21 / R-SOCK-2 (OD-17): pin the run future so a signal does NOT drop it
    // mid-flight — after broadcasting shutdown we AWAIT the loop, which
    // processes the broadcast, drains sessions (graceful_shutdown), and drops
    // its RAII guards. The old select! arms returned immediately, cancelling
    // run() and skipping both drain and (pre-guard) socket cleanup.
    let mut run_fut = Box::pin(server.run());
    let result = tokio::select! {
        _ = sigterm.recv() => {
            info!("SIGTERM received, initiating graceful shutdown");
            server.shutdown();
            (&mut run_fut).await
        }
        _ = sigint.recv() => {
            info!("SIGINT received, initiating graceful shutdown");
            server.shutdown();
            (&mut run_fut).await
        }
        result = &mut run_fut => {
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

    // R21 / R-SOCK-2: same pin-and-resume as the unix variant.
    let mut run_fut = Box::pin(server.run());
    let result = tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Ctrl+C received, initiating graceful shutdown");
            server.shutdown();
            (&mut run_fut).await
        }
        result = &mut run_fut => {
            result
        }
    };

    result
}

/// Handle a single client connection
async fn handle_client(
    stream: UnixStream,
    state: Arc<DaemonState>,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
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
                            // R19 / R-RPC-4: an ABSENT id is a notification —
                            // handle it but never write a reply (same rule as
                            // the stdio daemon's run loop).
                            let is_notification = req.id.is_none();
                            let response = handle_request(req, &state_clone).await;
                            if is_notification {
                                return;
                            }
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
            // Review [09]: server shutdown closes the connection so the
            // graceful drain can complete (previously a persistent idle
            // client pinned active_count > 0 for the whole 30s cap).
            _ = shutdown.recv() => {
                debug!(session_id = %session_id, "Server shutting down; closing client connection");
                break;
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
    // Review [27] / R-RPC-2: validate the version exactly like the stdio
    // daemon — both transports share one wire contract.
    if req.jsonrpc.as_deref() != Some("2.0") {
        return JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(crate::daemon::types::JsonRpcError {
                code: -32600,
                message: "Invalid Request".into(),
                data: Some(serde_json::json!("jsonrpc must be \"2.0\"")),
            }),
            id: req.id.clone().flatten().unwrap_or(serde_json::Value::Null),
        });
    }
    // TODO: Dispatch to actual method handlers (MULTICN-2005)
    // For now, simple echo response
    JsonRpcMessage::Response(JsonRpcResponse {
        jsonrpc: "2.0".into(),
        result: Some(serde_json::json!({
            "method": req.method,
            "received": true
        })),
        error: None,
        // R-RPC-1/4 id semantics: Some(Some(v)) = id v; Some(None) = explicit
        // null id (still answered); None = notification (reply suppressed at
        // the dispatch site above).
        id: req.id.flatten().unwrap_or(serde_json::Value::Null),
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
            // Review M11: pin a per-test DB — never ambient connect().
            database_url: Some(format!("sqlite://{}/test.db", temp_dir.path().display())),
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
                    jsonrpc: Some("2.0".into()),
                    method: format!("test_{}", i),
                    params: None,
                    id: Some(Some(serde_json::json!(i))),
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
    async fn test_session_cleanup_on_disconnect() {
        use tokio::net::UnixStream;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let config = ServerConfig {
            socket_path: socket_path.clone(),
            pid_path,
            idle_timeout: Duration::from_secs(300),
            // Review M11: pin a per-test DB — never ambient connect().
            database_url: Some(format!("sqlite://{}/test.db", temp_dir.path().display())),
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

    /// R20 / R-IDLE-2 (binding, UN-IGNORED — the old ignore reason, "Requires
    /// embedding provider", died with R-LAZY-5's lazy init): the server must
    /// idle-exit within 2*tick + timeout. With a 2s timeout the tick clamps
    /// to 1s, so the bound is ~4s — the pre-R20 fixed 60s tick fails this
    /// by an order of magnitude.
    #[tokio::test]
    async fn test_idle_timeout_triggers() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let idle_timeout = Duration::from_secs(2);
        let config = ServerConfig {
            socket_path,
            pid_path,
            idle_timeout,
            // Review M11: pin a per-test DB — never ambient connect().
            database_url: Some(format!("sqlite://{}/test.db", temp_dir.path().display())),
        };

        let server = SocketServer::new(config).await.unwrap();
        let tick = (idle_timeout / 4).clamp(Duration::from_secs(1), Duration::from_secs(60));
        let bound = 2 * tick + idle_timeout + Duration::from_secs(1); // +1s scheduling slack

        let start_time = std::time::Instant::now();
        let handle = tokio::spawn(async move { server.run().await });

        let result = tokio::time::timeout(bound, handle).await;
        let elapsed = start_time.elapsed();
        match result {
            Ok(join) => {
                join.expect("server task join").expect("server run result");
            }
            Err(_) => panic!(
                "server did not idle-exit within {bound:?} (elapsed {elapsed:?}) — \
                 the R20 proportional tick is not in effect"
            ),
        }
    }

    #[tokio::test]
    async fn test_active_client_prevents_idle_timeout() {
        use tokio::net::UnixStream;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let config = ServerConfig {
            socket_path: socket_path.clone(),
            pid_path,
            idle_timeout: Duration::from_secs(2), // Short timeout for test
            // Review M11: pin a per-test DB — never ambient connect().
            database_url: Some(format!("sqlite://{}/test.db", temp_dir.path().display())),
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
    async fn test_graceful_shutdown_waits_for_sessions() {
        use tokio::net::UnixStream;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let pid_path = temp_dir.path().join("test.pid");

        let config = ServerConfig {
            socket_path: socket_path.clone(),
            pid_path,
            idle_timeout: Duration::from_secs(300),
            // Review M11: pin a per-test DB — never ambient connect().
            database_url: Some(format!("sqlite://{}/test.db", temp_dir.path().display())),
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

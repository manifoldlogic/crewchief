# Ticket: MULTICN-2004: Daemon Lifecycle Management

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- process-management-specialist
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Implement daemon lifecycle features including idle timeout (5 minutes with no clients), graceful shutdown on SIGTERM, PID file cleanup, and `--socket` flag for the serve command to enable socket mode.

## Background

The socket server (MULTICN-2003) needs lifecycle management to prevent resource leaks and enable clean operation. Idle timeout ensures daemons don't run indefinitely when unused. Graceful shutdown allows in-flight requests to complete before termination.

The `--socket` flag makes socket mode opt-in during MVP, allowing fallback to stdio mode if issues occur.

Reference: [architecture.md](../planning/architecture.md) - Lifecycle Manager section

## Acceptance Criteria

- [x] Idle timeout: daemon shuts down after 5 minutes with no active clients
- [x] Idle timeout uses SessionRegistry.active_count() to check for clients
- [x] SIGTERM handler triggers graceful shutdown
- [x] Graceful shutdown waits for in-flight requests with configurable timeout (default 30s)
- [x] PID file automatically removed on shutdown (normal or SIGTERM)
- [x] `--socket` flag added to `crewchief-maproom serve` command
- [x] Test: Daemon shuts down when idle timeout expires
- [x] Test: SIGTERM allows in-flight requests to complete

## Technical Requirements

Enhance `crates/maproom/src/daemon/server.rs` and `crates/maproom/src/main.rs`.

### Idle Timeout Implementation

```rust
use tokio::time::{interval, Duration};

impl SocketServer {
    pub async fn run(&self) -> Result<(), DaemonError> {
        let _pid_guard = PidFileGuard::create(&self.config.pid_path)?;

        // ... socket binding code from MULTICN-2003 ...

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
                            tracing::error!(error = %e, "Client handler error");
                        }
                    });
                }

                _ = idle_check.tick() => {
                    let active_count = self.state.sessions.active_count();

                    if active_count == 0 {
                        if idle_since.is_none() {
                            idle_since = Some(Instant::now());
                            tracing::debug!("No active clients, idle timer started");
                        } else if let Some(since) = idle_since {
                            let idle_duration = since.elapsed();
                            if idle_duration >= self.config.idle_timeout {
                                tracing::info!(
                                    idle_secs = idle_duration.as_secs(),
                                    "Idle timeout reached, shutting down"
                                );
                                break;
                            }
                        }
                    } else {
                        if idle_since.is_some() {
                            tracing::debug!(active_count, "Clients connected, idle timer reset");
                        }
                        idle_since = None;
                    }
                }

                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutdown signal received");
                    break;
                }
            }
        }

        // Graceful shutdown
        self.graceful_shutdown().await?;

        // Cleanup
        std::fs::remove_file(&self.config.socket_path)?;
        Ok(())
    }

    async fn graceful_shutdown(&self) -> Result<(), DaemonError> {
        tracing::info!("Starting graceful shutdown");

        let shutdown_timeout = Duration::from_secs(30);
        let start = Instant::now();

        // Wait for active sessions to complete (with timeout)
        loop {
            let active_count = self.state.sessions.active_count();

            if active_count == 0 {
                tracing::info!("All sessions completed");
                break;
            }

            if start.elapsed() >= shutdown_timeout {
                tracing::warn!(
                    active_count,
                    "Shutdown timeout reached, {} sessions still active",
                    active_count
                );
                break;
            }

            tracing::debug!(active_count, "Waiting for sessions to complete");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }
}
```

### SIGTERM Handler

```rust
use tokio::signal::unix::{signal, SignalKind};

pub async fn run_with_signal_handling(server: SocketServer) -> Result<(), DaemonError> {
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let server = Arc::new(server);
    let server_clone = server.clone();

    let server_task = tokio::spawn(async move {
        server_clone.run().await
    });

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received");
            server.shutdown();
        }
        _ = sigint.recv() => {
            tracing::info!("SIGINT received");
            server.shutdown();
        }
        result = server_task => {
            return result.unwrap_or_else(|e| {
                Err(DaemonError::ServerError(format!("Server task panicked: {}", e)))
            });
        }
    }

    Ok(())
}
```

### CLI Integration

Update `crates/maproom/src/main.rs` to add `--socket` flag:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crewchief-maproom")]
#[command(about = "Maproom semantic code search daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the maproom daemon
    Serve {
        /// Use Unix socket mode (experimental)
        #[arg(long)]
        socket: bool,

        /// Socket path (default: /tmp/maproom-{uid}.sock)
        #[arg(long)]
        socket_path: Option<PathBuf>,

        /// Idle timeout in seconds (default: 300 = 5 minutes)
        #[arg(long, default_value_t = 300)]
        idle_timeout: u64,
    },
    // ... other commands ...
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { socket, socket_path, idle_timeout } => {
            if socket {
                // Socket mode
                let mut config = ServerConfig::default_for_user();

                if let Some(path) = socket_path {
                    config.socket_path = path;
                }

                config.idle_timeout = Duration::from_secs(idle_timeout);

                tracing::info!("Starting socket server");
                let server = SocketServer::new(config)?;
                run_with_signal_handling(server).await?;
            } else {
                // Stdio mode (existing behavior)
                tracing::info!("Starting stdio daemon");
                run_stdio_daemon().await?;
            }
        }
        // ... other commands ...
    }

    Ok(())
}
```

## Implementation Notes

### Idle Timeout Design

The idle timeout prevents resource leaks when clients disconnect without cleanup:

1. **Idle timer starts**: When active_count reaches 0
2. **Timer resets**: When new client connects
3. **Shutdown triggers**: After 5 minutes with no clients

This is more robust than per-client timeouts because it handles the daemon-level lifecycle.

### Graceful Shutdown Strategy

Two-phase shutdown:
1. **Stop accepting new connections**: Close listener socket
2. **Wait for in-flight requests**: Up to 30 seconds
3. **Force shutdown**: After timeout, close remaining sessions

This ensures requests in progress complete (avoiding data loss) while preventing indefinite hangs.

### Socket Mode Opt-In

Using `--socket` flag during MVP phase:
- **Default**: Stdio mode (existing behavior, proven)
- **Opt-in**: `crewchief-maproom serve --socket` enables socket mode
- **Rollback**: Users can revert to stdio if issues occur
- **Future**: After stabilization, socket becomes default with stdio fallback

### Configuration Priority

1. Command-line flags (highest)
2. Environment variables
3. Default values (lowest)

Example:
```bash
# Custom socket path and idle timeout
crewchief-maproom serve --socket --socket-path /var/run/maproom.sock --idle-timeout 600
```

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_idle_timeout_triggers() {
        let config = ServerConfig {
            idle_timeout: Duration::from_secs(1), // Short timeout for test
            ..ServerConfig::default_for_user()
        };

        let server = SocketServer::new(config).unwrap();

        // Start server in background
        let handle = tokio::spawn(async move {
            server.run().await
        });

        // Wait for idle timeout
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Server should have shut down
        let result = tokio::time::timeout(Duration::from_secs(1), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_active_client_prevents_idle_timeout() {
        let config = ServerConfig {
            idle_timeout: Duration::from_secs(2),
            ..ServerConfig::default_for_user()
        };

        let server = SocketServer::new(config).unwrap();

        // Register a session
        let (tx, _rx) = mpsc::unbounded_channel();
        server.state.sessions.register(tx);

        // Start server in background
        let handle = tokio::spawn(async move {
            server.run().await
        });

        // Wait longer than idle timeout
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Server should still be running
        assert!(!handle.is_finished());

        // Cleanup
        server.shutdown();
    }
}
```

### Integration Tests

```rust
// tests/integration/daemon_lifecycle.rs

#[tokio::test]
async fn test_sigterm_graceful_shutdown() {
    let server = start_test_server().await;

    // Start a slow request
    let client = connect_client(&server.socket_path).await;
    let request_future = client.slow_search();

    // Send SIGTERM
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(server.pid()),
        nix::sys::signal::Signal::SIGTERM
    ).unwrap();

    // Request should complete
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        request_future
    ).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_ok());
}
```

## Dependencies

- MULTICN-2003 (Unix Socket Server)
- tokio signal handling

## Risk Assessment

- **Risk**: Idle timeout triggers while client thinks it's connected
  - **Mitigation**: 5 minutes is generous. Clients should reconnect automatically.

- **Risk**: Graceful shutdown timeout too short for long operations
  - **Mitigation**: 30s default is configurable via CLI. Document for long-running indexing operations.

- **Risk**: PID file not cleaned up on crash
  - **Mitigation**: PidFileGuard Drop impl ensures cleanup in most cases. Clients should detect stale PID files.

## Files/Packages Affected

- `crates/maproom/src/daemon/server.rs` (MODIFY - add lifecycle logic)
- `crates/maproom/src/main.rs` (MODIFY - add --socket flag and signal handling)
- `tests/integration/daemon_lifecycle.rs` (NEW - integration tests)

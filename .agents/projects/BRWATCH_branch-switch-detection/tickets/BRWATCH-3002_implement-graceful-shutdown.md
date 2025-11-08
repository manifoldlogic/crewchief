# Ticket: BRWATCH-3002: Implement graceful shutdown with Ctrl+C

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement graceful shutdown handling for the watch command using Ctrl+C signal, ensuring clean resource cleanup and proper exit.

## Background
This ticket implements Step 3.2 from the implementation plan (plan.md - Phase 3). Long-running CLI commands must handle interruption signals gracefully, cleaning up resources (file watchers, database connections) before exiting.

From architecture.md lines 228-247, we use:
- ctrlc crate for signal handling
- tokio::select! for shutdown coordination
- oneshot channel for shutdown signaling

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 3.2

## Acceptance Criteria
- [ ] Ctrl+C signal handler registered
- [ ] Shutdown signal propagated to watcher via channel
- [ ] BranchWatcher stops gracefully (exits watch_loop)
- [ ] Database pool closed cleanly
- [ ] File watcher resources released
- [ ] "Shutting down..." message logged
- [ ] Exit code 0 on graceful shutdown
- [ ] No zombie processes or resource leaks

## Technical Requirements
- Use ctrlc crate for signal handling (already added in BRWATCH-1001)
- Use tokio::sync::oneshot channel for shutdown signaling
- Implement tokio::select! to race watcher against shutdown signal
- Add Drop trait to BranchWatcher for cleanup (optional)
- Log shutdown initiation and completion
- Return Ok(()) on graceful shutdown (exit code 0)

## Implementation Notes

From architecture.md lines 228-247:

```rust
use tokio::sync::oneshot;
use ctrlc;

async fn watch_command(args: WatchArgs) -> Result<()> {
    // ... setup logging, database, etc ...

    info!("Starting branch watcher (Ctrl+C to stop)");

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Setup Ctrl+C handler
    ctrlc::set_handler(move || {
        info!("Shutting down...");
        let _ = shutdown_tx.send(());
    })?;

    // Create watcher
    let mut watcher = BranchWatcher::new(args.repo, pool)?;

    // Run watcher until shutdown signal
    tokio::select! {
        result = watcher.start() => {
            // Watcher exited (error or channel closed)
            match result {
                Ok(_) => {
                    info!("Watcher stopped normally");
                }
                Err(e) => {
                    error!("Watcher error: {}", e);
                    return Err(e);
                }
            }
        }
        _ = shutdown_rx => {
            // Shutdown signal received
            info!("Shutdown signal received");
        }
    }

    info!("Branch watcher stopped");
    Ok(())
}
```

### Watcher Cleanup

Optionally implement Drop for BranchWatcher:

```rust
impl Drop for BranchWatcher {
    fn drop(&mut self) {
        info!("Cleaning up watcher resources");
        // File watcher dropped automatically
        // Database pool cleaned up by Arc::drop
    }
}
```

### Making watch_loop Interruptible

Modify watch_loop to check for shutdown:

```rust
async fn watch_loop(&mut self, mut shutdown: oneshot::Receiver<()>) -> Result<()> {
    loop {
        tokio::select! {
            event_result = self.rx.recv() => {
                match event_result {
                    Ok(event) => {
                        // Handle event
                    }
                    Err(e) => {
                        error!("Channel error: {}", e);
                        break;
                    }
                }
            }
            _ = &mut shutdown => {
                info!("Watch loop shutting down");
                break;
            }
        }
    }

    Ok(())
}
```

Alternative: Use atomic bool for shutdown flag if simpler.

### Testing Shutdown

Manual test:
```bash
# Start watcher
$ maproom watch --repo /path/to/repo

[INFO] Starting branch watcher (Ctrl+C to stop)
[INFO] Watching for branch switches...

# Press Ctrl+C
^C[INFO] Shutting down...
[INFO] Shutdown signal received
[INFO] Branch watcher stopped

$ echo $?
0  # Exit code should be 0
```

## Dependencies
- BRWATCH-1001 complete (ctrlc dependency)
- BRWATCH-3001 complete (watch command)

## Risk Assessment
- **Risk**: Signal handler doesn't trigger on some platforms
  - **Mitigation**: Test on Linux and macOS, ctrlc crate handles platform differences
- **Risk**: Watcher doesn't stop quickly (stuck in long operation)
  - **Mitigation**: Use tokio::select! to interrupt watch_loop immediately
- **Risk**: Resources not cleaned up (database connections leak)
  - **Mitigation**: Rely on Drop trait, verify with connection pool monitoring

## Files/Packages Affected
- `/workspace/crates/maproom/src/cli.rs` (modify watch_command)
- `/workspace/crates/maproom/src/watcher.rs` (modify watch_loop for shutdown)

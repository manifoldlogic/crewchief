# Ticket: BRWATCH-2002: Add error handling and retry logic

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement robust error handling and retry logic for branch switch handling to ensure the watcher continues running despite transient failures.

## Background
This ticket implements Step 2.2 from the implementation plan (plan.md - Phase 2). The watcher must be fault-tolerant - errors should be logged but not crash the watcher. Transient errors (network timeouts, temporary database unavailability) should trigger retries with exponential backoff.

From architecture.md lines 279-327:
- Graceful degradation: log errors, continue watching
- Retry logic: 3 attempts with exponential backoff
- Error classification: distinguish transient from permanent errors

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 2.2

## Acceptance Criteria
- [x] `handle_branch_switch_with_retry()` method implemented with exponential backoff
- [x] Maximum 3 retry attempts before giving up
- [x] Backoff delays: 2s, 4s, 8s (exponential)
- [x] Errors logged at appropriate levels (warn for retry, error for final failure)
- [x] Watcher continues watching after errors (doesn't crash)
- [x] Database errors trigger retry (tokio_postgres::Error)
- [x] watch_loop() calls retry-wrapped handler instead of direct handler
- [x] should_retry() classifies transient vs permanent errors

## Technical Requirements
- Modify `watch_loop()` to call `handle_branch_switch_with_retry()`
- Implement retry logic with exponential backoff
- Use `tokio::time::sleep` for delays
- Log retry attempts with context
- Classify errors: SqlxError::PoolClosed vs other errors
- Handle reconnection for pool closed errors
- Continue watching loop even on final failure
- Return Ok(()) from watch_loop event handlers (errors absorbed)

## Implementation Notes

From architecture.md lines 279-327:

### Graceful Degradation
```rust
async fn watch_loop(&mut self) -> Result<()> {
    loop {
        match self.rx.recv() {
            Ok(event) => {
                match event {
                    DebouncedEvent::Write(_) | DebouncedEvent::Create(_) => {
                        // Use retry wrapper instead of direct handler
                        if let Err(e) = self.handle_branch_switch_with_retry().await {
                            error!("Failed to handle branch switch after retries: {}", e);
                            // Continue watching despite error
                        }
                    }
                    DebouncedEvent::Error(e, path) => {
                        error!("Watcher error for {:?}: {}", path, e);
                    }
                    _ => {}
                }
            }
            Err(e) => {
                error!("Channel error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
```

### Retry Logic
```rust
async fn handle_branch_switch_with_retry(&self) -> Result<()> {
    let max_retries = 3;
    let mut attempt = 0;

    loop {
        match self.handle_branch_switch().await {
            Ok(_) => return Ok(()),
            Err(e) if attempt < max_retries => {
                warn!("Branch switch failed (attempt {}/{}): {}",
                      attempt + 1, max_retries, e);

                // Check if we should retry
                if should_retry(&e) {
                    attempt += 1;
                    let delay = Duration::from_secs(2_u64.pow(attempt));
                    warn!("Retrying in {:?}...", delay);
                    tokio::time::sleep(delay).await;
                } else {
                    // Permanent error, don't retry
                    return Err(e);
                }
            }
            Err(e) => {
                error!("Failed after {} retries: {}", max_retries, e);
                return Err(e);
            }
        }
    }
}

fn should_retry(error: &anyhow::Error) -> bool {
    // Retry on transient errors
    if let Some(sqlx_error) = error.downcast_ref::<sqlx::Error>() {
        match sqlx_error {
            sqlx::Error::PoolClosed => true,
            sqlx::Error::PoolTimedOut => true,
            sqlx::Error::Io(_) => true,
            _ => false,
        }
    } else {
        // Retry on I/O errors (file read, network)
        error.downcast_ref::<std::io::Error>().is_some()
    }
}
```

### Database Reconnection
```rust
async fn attempt_db_reconnect(&mut self) -> Result<()> {
    error!("Database pool closed, attempting reconnect...");

    // Note: Reconnection might require recreating the pool
    // For now, just log and continue (pool might recover)
    warn!("Database reconnection not yet implemented");

    Ok(())
}
```

## Dependencies
- BRWATCH-2001 complete (handle_branch_switch implemented)

## Risk Assessment
- **Risk**: Retry logic causes cascading delays
  - **Mitigation**: Exponential backoff with max 3 retries limits total delay to ~14 seconds
- **Risk**: Permanent errors repeatedly retry
  - **Mitigation**: should_retry() classifies errors, permanent errors fail fast
- **Risk**: Database pool never recovers
  - **Mitigation**: Log error prominently, consider restart watcher or alert user

## Files/Packages Affected
- `/workspace/crates/maproom/src/watcher.rs` (add retry logic, modify watch_loop)

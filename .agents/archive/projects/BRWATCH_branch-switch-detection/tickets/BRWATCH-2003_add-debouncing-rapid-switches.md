# Ticket: BRWATCH-2003: Add debouncing for rapid branch switches

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
Implement debouncing to prevent multiple concurrent indexing operations when developers rapidly switch branches.

## Background
This ticket addresses the rapid switching scenario from analysis.md lines 98-113:

```bash
git checkout feature-1  # Trigger index
git checkout feature-2  # Trigger index (1st still running!)
git checkout feature-3  # Trigger index (2 running!)
```

**Problem**: Multiple concurrent indexing operations waste resources and may cause database contention.

**Solution**: Debounce events so rapid switches only trigger one index operation after activity settles.

From architecture.md lines 330-353, we use time-based debouncing: if another switch happens within the debounce window, ignore previous switches and only index the final branch.

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Phase 2

## Acceptance Criteria
- [x] DebouncedHandler struct implemented with Mutex-protected timestamp
- [x] Debounce duration configurable (default 2 seconds)
- [x] Events within debounce window are ignored
- [x] Only the latest branch switch triggers indexing
- [x] watch_loop() integrates with debouncer
- [x] Logs when events are debounced
- [x] No race conditions in timestamp checking

## Technical Requirements
- Create DebouncedHandler struct with fields:
  - `last_event: Mutex<Instant>` - Timestamp of last processed event
  - `debounce_duration: Duration` - Minimum time between events
- Implement `handle_event()` method that:
  - Locks timestamp mutex
  - Checks if enough time has passed since last event
  - Returns early if within debounce window (log at debug level)
  - Updates timestamp if processing event
  - Releases lock before calling handler
- Integrate with BranchWatcher
- Default debounce duration: 2 seconds

## Implementation Notes

From architecture.md lines 330-353:

```rust
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct DebouncedHandler {
    last_event: Mutex<Instant>,
    debounce_duration: Duration,
}

impl DebouncedHandler {
    fn new(debounce_duration: Duration) -> Self {
        Self {
            last_event: Mutex::new(Instant::now()),
            debounce_duration,
        }
    }

    async fn should_handle(&self) -> bool {
        let mut last = self.last_event.lock().unwrap();
        let now = Instant::now();

        if now.duration_since(*last) < self.debounce_duration {
            debug!("Debouncing event (too soon after previous)");
            false
        } else {
            *last = now;
            true
        }
    }
}
```

### Integration with BranchWatcher

```rust
pub struct BranchWatcher {
    repo_path: PathBuf,
    pool: PgPool,
    watcher: RecommendedWatcher,
    rx: Receiver<DebouncedEvent>,
    debouncer: DebouncedHandler,  // Add this field
}

impl BranchWatcher {
    pub fn new(repo_path: PathBuf, pool: PgPool) -> Result<Self> {
        let (tx, rx) = channel();
        let watcher = watcher(tx, Duration::from_secs(1))?;
        let debouncer = DebouncedHandler::new(Duration::from_secs(2));

        Ok(Self {
            repo_path,
            pool,
            watcher,
            rx,
            debouncer,
        })
    }

    async fn watch_loop(&mut self) -> Result<()> {
        loop {
            match self.rx.recv() {
                Ok(event) => {
                    match event {
                        DebouncedEvent::Write(_) | DebouncedEvent::Create(_) => {
                            // Check debouncer before processing
                            if self.debouncer.should_handle().await {
                                if let Err(e) = self.handle_branch_switch_with_retry().await {
                                    error!("Failed to handle branch switch: {}", e);
                                }
                            } // else: event debounced, logged in should_handle()
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
}
```

### Testing Rapid Switches

Manual test scenario:
```bash
# Terminal 1: Watch running
maproom watch --repo /path/to/repo

# Terminal 2: Rapid switches
git checkout feature-1
git checkout feature-2
git checkout feature-3  # Only this should trigger index

# Expected: Only feature-3 gets indexed, first two debounced
```

## Dependencies
- BRWATCH-2001 complete (handle_branch_switch)
- BRWATCH-2002 complete (retry logic)

## Risk Assessment
- **Risk**: Debouncing causes valid switches to be ignored
  - **Mitigation**: 2-second window is short enough to feel responsive, long enough to avoid waste
- **Risk**: Mutex contention slows down event handling
  - **Mitigation**: Lock is held only during timestamp check (<1ms), released before expensive indexing
- **Risk**: User expects immediate indexing, debouncing feels unresponsive
  - **Mitigation**: 2 seconds is imperceptible, incremental update already fast (<1 min)

## Files/Packages Affected
- `/workspace/crates/maproom/src/watcher.rs` (add DebouncedHandler struct, modify BranchWatcher)

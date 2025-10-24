# Ticket: INC_INDEX-3001: Update Queue Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement a priority-based update queue system for incremental indexing that handles task deduplication, batch processing, and error handling with retry logic. This queue sits between the change detection system and the incremental processor, ensuring efficient and reliable processing of file changes.

## Background
The incremental indexing pipeline needs a robust queue system to manage the flow of update tasks from the file watcher and change detector to the processor. Without proper queuing, the system would be overwhelmed by bursts of file changes (e.g., git branch switches, bulk saves), lack prioritization for user-triggered vs automatic updates, and have no mechanism for handling processing failures.

This work is part of Phase 3 (Week 3) of the INC_INDEX project, which implements real-time incremental indexing with file watching. The update queue is a critical component that enables efficient batch processing, prevents duplicate work, and ensures high-priority tasks are processed first.

## Acceptance Criteria
- [ ] Queue processes tasks correctly with priority ordering
- [ ] Task deduplication working - multiple updates to same file are merged
- [ ] Batch processing implemented - similar tasks grouped together
- [ ] Error handling with retry logic and dead letter queue
- [ ] Priority levels correctly assigned: User=High, Save=Medium, Auto=Low
- [ ] Tasks can be enqueued and dequeued efficiently
- [ ] Processing state tracked (pending, in-progress, completed, failed)
- [ ] Unit tests cover all queue operations and edge cases

## Technical Requirements
- Implement `PriorityQueue<UpdateTask>` using a suitable Rust priority queue library (e.g., `priority-queue` crate)
- Create `UpdateTask` type with fields:
  - `path: PathBuf` - file path being updated
  - `change_type: ChangeType` - New, Modified, or Deleted
  - `trigger: Trigger` - User, Save, or Auto
  - `priority: Priority` - High, Medium, or Low (calculated)
  - `created_at: DateTime<Utc>` - when task was created
  - `retry_count: u32` - number of retry attempts
- Implement task deduplication:
  - If task for same path exists, merge it with new task
  - Preserve highest priority
  - Update change_type if needed
- Implement batch processing:
  - Group similar tasks together (same directory, same file type)
  - Process batches transactionally
- Error handling:
  - Retry failed tasks up to 3 times with exponential backoff
  - Move tasks to dead letter queue after max retries
  - Log all errors with context
- Thread-safe operations using `Arc<Mutex<UpdateQueue>>` or similar
- Maintain `processing: HashSet<PathBuf>` to prevent concurrent processing of same file

## Implementation Notes

### Priority Calculation
Priority is determined by the trigger type as specified in the architecture:
```rust
fn calculate_priority(&self, task: &UpdateTask) -> Priority {
    match task.trigger {
        Trigger::User => Priority::High,
        Trigger::Save => Priority::Medium,
        Trigger::Auto => Priority::Low,
    }
}
```

Future enhancements may consider:
- Recency (recently modified files higher priority)
- Active worktrees (files in active worktree higher priority)
- File type (source files vs config files)

### Task Deduplication
When enqueuing a task for a path that's already in the queue:
```rust
pub fn enqueue(&mut self, task: UpdateTask) {
    let priority = self.calculate_priority(&task);

    // Dedup and merge
    if let Some(existing) = self.queue.get_mut(&task.path) {
        existing.merge(task);
    } else {
        self.queue.push(task, priority);
    }
}
```

The `merge` operation should:
- Keep the highest priority
- Update change_type appropriately (e.g., New + Deleted = nothing, Modified + Modified = Modified)
- Update timestamp to latest

### Batch Processing
Group tasks by:
- Directory (files in same directory processed together)
- File type (all TypeScript files, all Rust files, etc.)

This reduces database round-trips and improves cache locality.

### Error Handling
- Implement exponential backoff: 1s, 2s, 4s between retries
- Track retry count per task
- After 3 failed attempts, move to dead letter queue
- Dead letter queue persisted to database for later inspection/retry
- Log all processing errors with full context (path, change_type, error message, stack trace)

### Thread Safety
The queue will be accessed from multiple threads:
- Watcher thread enqueuing new tasks
- Processor thread dequeuing tasks
- Status reporting thread reading queue state

Use appropriate synchronization primitives (`Arc<Mutex<>>` or `Arc<RwLock<>>`) to ensure thread safety.

## Dependencies
- **INC_INDEX-2002** (File watcher events to queue) - The file watcher must emit events that feed into this queue
- `priority-queue` crate or similar for heap-based priority queue
- `tokio` for async runtime and time utilities
- `chrono` for timestamp handling

## Risk Assessment
- **Risk**: Priority queue performance degrades with large queue sizes
  - **Mitigation**: Use efficient heap-based priority queue implementation, monitor queue depth, add alerting if queue grows too large (>1000 items)

- **Risk**: Task deduplication logic is complex and may have edge cases
  - **Mitigation**: Comprehensive unit tests for all merge scenarios (New+Modified, Modified+Deleted, etc.), property-based testing with quickcheck

- **Risk**: Dead letter queue grows unbounded with persistent failures
  - **Mitigation**: Implement dead letter queue size limits, periodic cleanup of old failed tasks, alerting on dead letter queue growth

- **Risk**: Lock contention on queue access from multiple threads
  - **Mitigation**: Use `RwLock` instead of `Mutex` if reads are more frequent than writes, consider lock-free queue implementation if contention becomes an issue, profile and measure lock wait times

## Files/Packages Affected
- `crates/maproom/src/incremental/queue.rs` - UpdateQueue implementation (NEW)
- `crates/maproom/src/incremental/task.rs` - UpdateTask types and enums (NEW)
- `crates/maproom/src/incremental/priority.rs` - Priority calculation logic (NEW)
- `crates/maproom/src/incremental/mod.rs` - Module declarations (MODIFIED)
- `crates/maproom/tests/incremental/queue_test.rs` - Comprehensive unit tests (NEW)
- `crates/maproom/tests/incremental/task_test.rs` - Task merge logic tests (NEW)
- `crates/maproom/Cargo.toml` - Add priority-queue dependency (MODIFIED)

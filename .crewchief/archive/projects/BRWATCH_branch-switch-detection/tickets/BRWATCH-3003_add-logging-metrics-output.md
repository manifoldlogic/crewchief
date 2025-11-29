# Ticket: BRWATCH-3003: Add logging and metrics output

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Enhance logging throughout the watcher with informative messages and metrics output to give users visibility into watcher activity and indexing performance.

## Background
This ticket implements Step 3.3 from the implementation plan (plan.md - Phase 3). Good logging is essential for long-running background processes - users need to know:
- Is the watcher running?
- Did it detect my branch switch?
- How long did indexing take?
- How much did it cost (embeddings)?

From architecture.md lines 250-275, we log:
- Watcher lifecycle events
- Branch switch detection
- Indexing metrics (duration, files, chunks, cache hit rate, cost)
- Errors with context

**Planning Reference**: `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 3.3

## Acceptance Criteria
- [x] Watcher startup logged (repository path, status)
- [x] Current branch logged on initialization
- [x] Branch switch events logged with branch name
- [x] Indexing metrics logged (duration, files, chunks, cache hit rate)
- [x] Embedding generation logged with cost estimate
- [x] Errors logged with context and error messages
- [x] Shutdown events logged
- [x] Logs use appropriate levels (info, warn, error, debug)
- [x] Verbose mode shows additional debug information

## Technical Requirements
- Use log macros: `info!()`, `warn!()`, `error!()`, `debug!()`
- Log at strategic points throughout watcher lifecycle
- Format metrics clearly and concisely
- Include relevant context in error logs
- Use debug level for verbose implementation details
- Keep info level logs user-friendly
- No sensitive information in logs (no auth tokens, passwords)

## Implementation Notes

### Logging Points

**Startup** (cli.rs):
```rust
info!("Starting branch watcher for {}", args.repo.display());
info!("Connected to database");
```

**Watcher Initialization** (watcher.rs):
```rust
info!("Watching {} for branch switches", git_head.display());
info!("Indexing current branch...");
```

**Branch Switch Detection**:
```rust
info!("Branch switch detected: {}", current_branch);
```

**Indexing Metrics**:
```rust
info!("Index updated in {:.1}s:", duration.as_secs_f64());
info!("  Files processed: {}", stats.files_processed);
info!("  Chunks processed: {}", stats.chunks_processed);
info!("  Cache hit rate: {:.1}%", stats.cache_hit_rate() * 100.0);
info!("  Embeddings generated: {}", stats.embeddings_generated);
info!("  Estimated cost: ${:.4}", stats.cost());
```

**Waiting State**:
```rust
info!("Waiting for changes...");
```

**Errors**:
```rust
error!("Failed to handle branch switch: {}", e);
warn!("Retry {}/{}: {}", attempt + 1, max_retries, e);
error!("Database pool closed: {}", e);
```

**Debug Logging** (--verbose):
```rust
debug!("Debouncing event (too soon after previous)");
debug!("Extracting branch name from {}", head_path.display());
debug!("Creating worktree record for {}", branch_name);
debug!("Calling incremental_update with worktree_id={}", worktree_id);
```

**Shutdown**:
```rust
info!("Shutting down...");
info!("Shutdown signal received");
info!("Branch watcher stopped");
```

### Example Output

**Normal operation**:
```bash
$ maproom watch --repo /workspace/myproject

[2024-11-08 23:45:12 INFO] Starting branch watcher for /workspace/myproject
[2024-11-08 23:45:12 INFO] Connected to database
[2024-11-08 23:45:12 INFO] Watching /workspace/myproject/.git/HEAD for branch switches
[2024-11-08 23:45:12 INFO] Indexing current branch...
[2024-11-08 23:45:12 INFO] Branch switch detected: main
[2024-11-08 23:45:12 INFO] Index updated in 0.1s:
[2024-11-08 23:45:12 INFO]   Files processed: 0
[2024-11-08 23:45:12 INFO]   Chunks processed: 0
[2024-11-08 23:45:12 INFO]   Cache hit rate: 100.0%
[2024-11-08 23:45:12 INFO] Waiting for changes...

# User switches branch
[2024-11-08 23:47:03 INFO] Branch switch detected: feature-auth
[2024-11-08 23:47:48 INFO] Index updated in 45.2s:
[2024-11-08 23:47:48 INFO]   Files processed: 150
[2024-11-08 23:47:48 INFO]   Chunks processed: 7500
[2024-11-08 23:47:48 INFO]   Cache hit rate: 84.2%
[2024-11-08 23:47:48 INFO]   Embeddings generated: 1185
[2024-11-08 23:47:48 INFO]   Estimated cost: $0.0237
[2024-11-08 23:47:48 INFO] Waiting for changes...
```

**Verbose mode**:
```bash
$ maproom watch --repo /workspace/myproject --verbose

[2024-11-08 23:45:12 DEBUG] Initializing file watcher with 1s debounce
[2024-11-08 23:45:12 DEBUG] Creating database pool from DATABASE_URL
[2024-11-08 23:45:12 INFO]  Connected to database
[2024-11-08 23:45:12 DEBUG] Watching .git/HEAD with RecursiveMode::NonRecursive
[2024-11-08 23:45:12 INFO]  Watching /workspace/myproject/.git/HEAD for branch switches
[2024-11-08 23:45:12 DEBUG] Reading .git/HEAD content
[2024-11-08 23:45:12 DEBUG] Parsed branch name: main
[2024-11-08 23:45:12 INFO]  Branch switch detected: main
...
```

**Error scenario**:
```bash
[2024-11-08 23:45:12 ERROR] Failed to connect to database: connection refused
Error: Failed to connect to database: connection refused
```

### Log Configuration

In watch_command:
```rust
if args.verbose {
    env::set_var("RUST_LOG", "maproom=debug");
} else {
    env::set_var("RUST_LOG", "maproom=info");
}
env_logger::Builder::from_default_env()
    .format_timestamp_secs()
    .init();
```

## Dependencies
- BRWATCH-3001 complete (watch command exists)
- env_logger crate (should be available)

## Risk Assessment
- **Risk**: Too much logging floods terminal
  - **Mitigation**: Use info level for important events only, debug for details
- **Risk**: Logs expose sensitive information
  - **Mitigation**: Avoid logging database passwords, auth tokens, file contents
- **Risk**: Log file not specified, output lost
  - **Mitigation**: For MVP, stdout is sufficient; document how to redirect to file

## Files/Packages Affected
- `/workspace/crates/maproom/src/watcher.rs` (add logging throughout)
- `/workspace/crates/maproom/src/cli.rs` (configure logging)

## Implementation Notes

### Changes Made

Enhanced logging in `/workspace/crates/maproom/src/watcher.rs`:

1. **Added cache hit rate logging** (line 249):
   ```rust
   info!("  Cache hit rate: {:.1}%", stats.cache_hit_rate() * 100.0);
   ```

2. **Added cost estimate logging** (line 250):
   ```rust
   info!("  Estimated cost: ${:.4}", stats.cost());
   ```

3. **Added "Waiting for changes..." message** (line 251):
   ```rust
   info!("Waiting for changes...");
   ```

### Existing Logging Verified

The following logging was already present and meets requirements:

- **Startup**: main.rs lines 373, 379, 384
- **Current branch initialization**: watcher.rs line 154
- **Branch switch detection**: watcher.rs line 210
- **Indexing metrics**: watcher.rs lines 242-248
- **Error logging**: watcher.rs lines 132, 138, 143, 195
- **Retry logging**: watcher.rs lines 176, 187
- **Shutdown events**: main.rs lines 394, 407, 416, 420
- **Debug logging**: watcher.rs line 39

All acceptance criteria are now met. The logging provides comprehensive visibility into:
- Watcher lifecycle (startup, shutdown, errors)
- Branch switch detection and current branch
- Indexing performance metrics (duration, files, chunks, embeddings)
- Cache efficiency (hit rate)
- Cost tracking (estimated embedding costs)
- Error context and retry attempts

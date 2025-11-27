# Ticket: IDXABS-6003: Implement Watch Command for SQLite

## Status
- [ ] **Task completed** - watch command works with SQLite
- [ ] **Tests pass** - watch E2E tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the `watch` command that was stubbed out during the SQLite migration. The command currently prints an error message telling users it's unavailable.

## Background
The `watch` command in `main.rs` was stubbed during IDXABS:
```rust
// Line 988 in main.rs
anyhow::bail!(
    "Watch command is temporarily unavailable.\n\
    The watch_worktree function was removed during SQLite-only migration.\n\
    Use 'scan' for initial indexing and 'upsert' for incremental updates.\n\
    Watch functionality will be reimplemented in a future update."
);
```

This ticket implements the watch command using the incremental module functions from IDXABS-6002.

## Acceptance Criteria
- [ ] `crewchief-maproom watch` starts file watching successfully
- [ ] File changes are detected and indexed incrementally
- [ ] Output shows progress (files indexed, chunks created)
- [ ] Ctrl+C terminates gracefully
- [ ] Works with both `--path` and current directory
- [ ] NDJSON output mode available for VSCode extension integration

## Technical Requirements

### Watch Command Implementation
```rust
async fn run_watch(
    store: Arc<SqliteStore>,
    repo_path: &Path,
    worktree_id: i64,
    output_format: OutputFormat,
) -> Result<()> {
    // 1. Create IncrementalProcessor
    let processor = IncrementalProcessor::new(store.clone(), repo_path.to_path_buf());

    // 2. Set up file watcher using notify crate
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(repo_path, RecursiveMode::Recursive)?;

    // 3. Event loop
    loop {
        match rx.recv() {
            Ok(event) => {
                // Process file change
                // Create UpdateTask
                // Call processor.process(task)
                // Output progress
            }
            Err(_) => break,
        }
    }
}
```

### Output Formats
1. **Human-readable** (default):
   ```
   Watching /workspace for changes...
   [12:34:56] Indexed src/main.rs (3 chunks)
   [12:34:57] Indexed src/lib.rs (5 chunks)
   ```

2. **NDJSON** (for VSCode extension):
   ```json
   {"type":"started","path":"/workspace"}
   {"type":"indexed","file":"src/main.rs","chunks":3}
   {"type":"indexed","file":"src/lib.rs","chunks":5}
   ```

### Integration with Existing Code
- Use `notify` crate (already in dependencies)
- Use `IncrementalProcessor` from `crate::incremental`
- Use `ChangeDetector` for hash-based change detection
- Use existing CLI argument parsing structure

## CLI Interface
```bash
# Watch current directory
crewchief-maproom watch --repo myrepo --worktree main

# Watch specific path
crewchief-maproom watch --path /workspace --repo myrepo --worktree main

# NDJSON output for extensions
crewchief-maproom watch --repo myrepo --worktree main --format ndjson
```

## Dependencies
- IDXABS-6002 (incremental module must be implemented)

## Risk Assessment
- **Risk**: File event flooding (many rapid changes)
  - **Mitigation**: Debouncing in event handler
- **Risk**: Database locking during rapid writes
  - **Mitigation**: SQLite WAL mode (already configured)

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/main.rs` (watch command implementation)
- `crates/maproom/src/cli/watch.rs` (if separate module)

## Testing Requirements
1. **Unit test**: Mock file events, verify processor called
2. **Integration test**: Create/modify/delete files, verify database state
3. **E2E test**: `scripts/test_watch_sqlite.sh`
   - Start watch command
   - Create test file
   - Verify file indexed
   - Modify test file
   - Verify changes indexed
   - Delete test file
   - Verify chunks removed

## Estimated Effort
Medium - Command structure exists, needs integration with incremental module.

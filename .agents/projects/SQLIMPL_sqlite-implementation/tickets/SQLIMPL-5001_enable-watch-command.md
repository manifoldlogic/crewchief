# Ticket: SQLIMPL-5001: Enable Watch Command in CLI

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Enable the watch command in the CLI by removing the "temporarily unavailable" error and wiring it to the incremental module. This allows continuous file monitoring and automatic re-indexing.

## Background
The watch command is currently disabled at CLI level with an error message saying it's "temporarily unavailable". With Phase 3 (Incremental Updates) complete, the watch command can now be enabled.

This ticket implements Plan Phase 5, Ticket 5001: "Enable Watch Command in CLI".

## Acceptance Criteria
- [ ] "Temporarily unavailable" error removed from `src/main.rs`
- [ ] Watch command wired to incremental module
- [ ] File system watcher (notify crate) is configured
- [ ] `cargo run -- watch --repo test` starts without error
- [ ] Watch command shows status messages when starting

## Technical Requirements
- Remove the error guard in `src/main.rs` watch handler
- Wire watch to `incremental_update()` from Phase 3
- Use `notify` crate for file system events
- Debounce rapid changes (don't re-index on every keystroke)
- Handle graceful shutdown on Ctrl+C

## Implementation Notes

### Current State
```rust
// src/main.rs - watch command handler
Command::Watch { repo, .. } => {
    // Currently returns error
    bail!("Watch command is temporarily unavailable")
}
```

### Target Implementation
```rust
// src/main.rs - watch command handler
Command::Watch { repo, worktree, path } => {
    let store = SqliteStore::connect(&config.database_url)?;
    let repo_id = store.get_or_create_repo(&repo).await?;
    let worktree_id = store.get_or_create_worktree(repo_id, &worktree).await?;

    // Set up file watcher
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })?;

    watcher.watch(&path, RecursiveMode::Recursive)?;

    println!("Watching {} for changes...", path.display());

    // Process events with debouncing
    let incremental = IncrementalUpdater::new(store.clone());
    let mut pending_paths = HashSet::new();
    let mut last_update = Instant::now();
    let debounce_duration = Duration::from_millis(500);

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                for path in event.paths {
                    pending_paths.insert(path);
                }
            }
            Err(_timeout) => {
                // Check if we should process pending changes
                if !pending_paths.is_empty() && last_update.elapsed() > debounce_duration {
                    let paths: Vec<_> = pending_paths.drain().collect();
                    println!("Processing {} changed files...", paths.len());

                    incremental.incremental_update(repo_id, worktree_id).await?;

                    last_update = Instant::now();
                    println!("Update complete.");
                }
            }
        }
    }
}
```

### Graceful Shutdown
```rust
// Handle Ctrl+C
tokio::select! {
    _ = signal::ctrl_c() => {
        println!("\nShutting down watch...");
        break;
    }
    event = rx.recv() => {
        // Process event...
    }
}
```

## Dependencies
- Phase 3 Complete (Incremental Updates working)

## Risk Assessment
- **Risk**: File watcher may miss rapid changes
  - **Mitigation**: Debouncing handles this; full sync on startup
- **Risk**: Large changesets may slow down watch
  - **Mitigation**: Process in batches; show progress

## Files/Packages Affected
- `crates/maproom/src/main.rs` (primary)
- `crates/maproom/Cargo.toml` (verify `notify` dependency)

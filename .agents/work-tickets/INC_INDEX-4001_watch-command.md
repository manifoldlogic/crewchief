# Ticket: INC_INDEX-4001: Watch Command Implementation

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
Implement the `maproom watch` CLI command that enables continuous incremental indexing by starting the file system watcher, loading configuration, providing status reporting, and supporting graceful shutdown. This command serves as the user-facing interface to the watch subsystem developed in previous phases.

## Background
Phase 4 focuses on delivering the watch functionality to end users through a polished CLI interface. The underlying watcher (INC_INDEX-2001) and event processor (INC_INDEX-3002) have been implemented, but users need a simple command to activate continuous indexing. The watch command must provide visibility into the watching process (status reporting), load configuration appropriately, and handle shutdown gracefully to ensure data integrity.

This is the primary deliverable of Week 4 in Phase 4, making the incremental indexing feature fully accessible to users.

## Acceptance Criteria
- [ ] `maproom watch` command is functional and can be invoked from the CLI
- [ ] Configuration is loaded correctly from the config file
- [ ] Status reporting displays: files watched, events processed, queue size, and watcher state
- [ ] Graceful shutdown implemented for SIGINT/SIGTERM signals
- [ ] Background daemon mode option is available
- [ ] Command integrates seamlessly with existing `maproom` CLI structure
- [ ] Help text and documentation are clear and complete

## Technical Requirements
- Create `maproom watch` subcommand in the CLI command structure
- Load watch configuration from `config/watch.rs` or configuration file
- Implement status reporting that outputs:
  - Number of files being watched
  - Number of events processed since start
  - Current event queue size
  - Watcher state (running, stopped, error)
- Handle SIGINT (Ctrl+C) and SIGTERM signals for graceful shutdown
- Implement optional background daemon mode (`--daemon` flag)
- Ensure proper cleanup of resources on shutdown
- Use structured logging for watch command operations
- Follow existing CLI patterns from other maproom commands

## Implementation Notes

### Command Structure
```rust
// crates/maproom/src/cli/commands/watch.rs
pub struct WatchCommand {
    /// Run in background daemon mode
    #[arg(long)]
    daemon: bool,

    /// Display status updates every N seconds
    #[arg(long, short = 's', default_value = "10")]
    status_interval: u64,
}
```

### Status Reporting
- Print status updates at configurable intervals
- Include timestamp, files watched, events processed, queue size
- Use indicatif or similar crate for progress indicators
- Support JSON output format for scripting (`--json` flag)

### Daemon Mode
- Fork process or use daemon libraries for background execution
- Write PID file to allow status checks and shutdown
- Log output to file instead of stdout when in daemon mode
- Consider using `daemonize` crate or similar

### Graceful Shutdown
- Register signal handlers for SIGINT and SIGTERM
- Shutdown sequence:
  1. Stop accepting new file system events
  2. Process remaining events in queue
  3. Flush any pending database writes
  4. Close database connections
  5. Exit cleanly
- Use tokio's signal handling capabilities
- Timeout after reasonable period if shutdown stalls

### Configuration Loading
- Reuse configuration loading patterns from other commands
- Support CLI flags to override config file settings
- Validate configuration before starting watcher
- Provide helpful error messages for config issues

### Integration Points
- Wire into existing CLI command registration
- Use WatchManager from INC_INDEX-2001
- Use EventProcessor from INC_INDEX-3002
- Follow patterns from `scan`, `search`, and other commands

## Dependencies
- **INC_INDEX-2001** (File System Watcher Implementation) - Required to start and manage the watcher
- **INC_INDEX-3002** (Event Processing Pipeline) - Required to process events from the watcher
- Configuration subsystem for loading watch settings
- Signal handling utilities (tokio::signal or similar)

## Risk Assessment
- **Risk**: Daemon mode complexity may introduce platform-specific issues
  - **Mitigation**: Start with foreground mode, add daemon mode as optional enhancement. Test on Linux and macOS. Consider using well-tested daemonization libraries.

- **Risk**: Signal handling may not work correctly in all scenarios (e.g., forceful kill)
  - **Mitigation**: Implement timeout for graceful shutdown. Document that SIGKILL cannot be handled. Use write-ahead logging or transactions to prevent data corruption.

- **Risk**: Status reporting may impact performance if too frequent
  - **Mitigation**: Make status interval configurable with sensible default (10 seconds). Use async channels to avoid blocking main processing loop.

- **Risk**: Configuration loading errors may not provide clear guidance to users
  - **Mitigation**: Implement comprehensive validation with specific error messages. Provide example configuration in documentation.

## Files/Packages Affected
- `crates/maproom/src/cli/commands/watch.rs` - Main watch command implementation (NEW)
- `crates/maproom/src/cli/watch_daemon.rs` - Daemon mode functionality (NEW)
- `crates/maproom/src/config/watch.rs` - Watch configuration loading (NEW)
- `crates/maproom/src/cli/mod.rs` - Register watch command
- `crates/maproom/tests/cli/watch_test.rs` - CLI integration tests (NEW)
- `crates/maproom/Cargo.toml` - Add dependencies for signal handling, daemonization
- `crates/maproom/README.md` - Update documentation with watch command usage

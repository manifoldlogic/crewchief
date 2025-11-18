# Ticket: IDXCLEAN-2001: Add CLI Subcommand Structure for Cleanup

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
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add `maproom db cleanup-stale` subcommand to the CLI with `--confirm` and `--verbose` flags to expose cleanup functionality for stale worktree detection and deletion. This ticket establishes the user interface contract without implementing the execution logic.

## Background
Phase 1 (tickets IDXCLEAN-1001 and IDXCLEAN-1002) completed the detection and deletion modules for identifying and removing stale worktree data. Phase 2 now focuses on exposing this functionality via the maproom CLI to make it accessible to users.

This ticket implements the subcommand structure, argument parsing, and help text as defined in plan.md Phase 2 - CLI Command Interface (lines 228-258). The command will follow the existing CLI patterns established in `main.rs` and support both dry-run mode (default) and confirmed deletion mode.

**Planning Reference**: `.agents/projects/IDXCLEAN_index-stale-worktree-cleanup/planning/plan.md` (lines 228-258)

## Acceptance Criteria
- [x] New subcommand `cleanup-stale` added under `db` command in CLI
- [x] `--confirm` flag added (defaults to false for dry-run mode)
- [x] `--verbose` flag added (shows detailed output)
- [x] Command integrated into main CLI routing in `Commands::Db` match arm
- [x] Help text added: `maproom db cleanup-stale --help` shows usage
- [x] Help text includes examples of both dry-run and confirm usage
- [x] Subcommand compiles without errors
- [x] Unit test: CLI argument parsing works correctly for all flag combinations

## Technical Requirements
- Use clap derive API for argument parsing (consistent with existing CLI patterns)
- Add `CleanupStale` variant to existing `DbCommand` enum
- Default behavior must be dry-run (safe by default: `confirm: false`)
- Follow existing CLI structure in `crates/maproom/src/main.rs` (lines 32-247)
- Help text should be clear and user-friendly, explaining dry-run vs confirmed deletion
- Document that command requires database connection via `MAPROOM_DATABASE_URL`

## Implementation Notes

### Struct Definition
Based on plan.md lines 239-248, add the following to the `DbCommand` enum:

```rust
#[derive(Subcommand, Debug)]
enum DbCommand {
    /// Apply SQL migrations to the configured database
    Migrate,

    /// Clean up stale worktree data from the database
    ///
    /// By default, runs in dry-run mode showing what would be deleted.
    /// Use --confirm to actually perform deletions.
    ///
    /// Examples:
    ///   maproom db cleanup-stale              # Dry-run mode (show what would be deleted)
    ///   maproom db cleanup-stale --confirm    # Actually delete stale data
    ///   maproom db cleanup-stale --verbose    # Show detailed information
    CleanupStale {
        /// Actually delete stale data (default is dry-run)
        #[arg(long, help = "Actually delete (default is dry-run)")]
        confirm: bool,

        /// Show detailed information during cleanup
        #[arg(long, short, help = "Show detailed information")]
        verbose: bool,
    },
}
```

### Command Routing
Add to the `Commands::Db` match arm in `main()`:

```rust
Commands::Db { command } => match command {
    DbCommand::Migrate => {
        let client = db::connect().await?;
        db::migrate(&client).await?;
        tracing::info!("migrations applied");
    }
    DbCommand::CleanupStale { confirm, verbose } => {
        // TODO: Implementation in IDXCLEAN-2002
        let _client = db::connect().await?;
        if confirm {
            println!("Would run cleanup with confirmation");
        } else {
            println!("Would run cleanup in dry-run mode");
        }
        if verbose {
            println!("Verbose mode enabled");
        }
    }
}
```

### Testing
Add unit tests for argument parsing in a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_stale_defaults() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale"]);
        if let Commands::Db { command: DbCommand::CleanupStale { confirm, verbose } } = cli.command {
            assert_eq!(confirm, false, "confirm should default to false");
            assert_eq!(verbose, false, "verbose should default to false");
        } else {
            panic!("Expected cleanup-stale command");
        }
    }

    #[test]
    fn test_cleanup_stale_with_confirm() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale", "--confirm"]);
        if let Commands::Db { command: DbCommand::CleanupStale { confirm, verbose } } = cli.command {
            assert_eq!(confirm, true);
            assert_eq!(verbose, false);
        } else {
            panic!("Expected cleanup-stale command");
        }
    }

    #[test]
    fn test_cleanup_stale_with_verbose() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale", "--verbose"]);
        if let Commands::Db { command: DbCommand::CleanupStale { confirm, verbose } } = cli.command {
            assert_eq!(confirm, false);
            assert_eq!(verbose, true);
        } else {
            panic!("Expected cleanup-stale command");
        }
    }

    #[test]
    fn test_cleanup_stale_short_verbose() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale", "-v"]);
        if let Commands::Db { command: DbCommand::CleanupStale { confirm, verbose } } = cli.command {
            assert_eq!(verbose, true);
        } else {
            panic!("Expected cleanup-stale command");
        }
    }
}
```

## Dependencies
- **IDXCLEAN-1001**: Stale detection module must exist (provides detection logic for Phase 2)
- **IDXCLEAN-1002**: Safe deletion module must exist (provides deletion logic for Phase 2)
- **IDXCLEAN-1003**: Data models and error types must exist (provides supporting types)

## Risk Assessment
- **Risk**: Breaking existing CLI patterns or command structure
  - **Mitigation**: Follow established patterns in `main.rs` for `DbCommand` enum and command routing. Use existing `Migrate` subcommand as reference.

- **Risk**: Default behavior not safe (accidentally deleting data)
  - **Mitigation**: Ensure `confirm` defaults to `false`, requiring explicit `--confirm` flag for deletions. Add clear help text explaining dry-run mode.

- **Risk**: Help text unclear or missing examples
  - **Mitigation**: Include comprehensive help text with examples in doc comments. Follow patterns from existing commands like `Scan`.

## Files/Packages Affected
- `/workspace/crates/maproom/src/main.rs` (add `CleanupStale` variant to `DbCommand` enum, add routing logic)

**Estimated Effort**: 0.5-1 day

## Implementation Notes

### Changes Made

1. **Added `CleanupStale` variant to `DbCommand` enum** (lines 248-266)
   - Includes comprehensive help text with examples
   - Two flags: `--confirm` (defaults to false) and `--verbose/-v` (defaults to false)
   - Follows existing CLI patterns using clap derive API

2. **Added command routing** (lines 470-481)
   - Integrated into `Commands::Db` match arm
   - Placeholder implementation that connects to database and prints what mode would execute
   - Marked with TODO comment referencing IDXCLEAN-2002 for actual implementation

3. **Added unit tests** (lines 1054-1100)
   - `test_cleanup_stale_defaults` - verifies both flags default to false
   - `test_cleanup_stale_with_confirm` - verifies --confirm flag parsing
   - `test_cleanup_stale_with_verbose` - verifies --verbose flag parsing
   - `test_cleanup_stale_short_verbose` - verifies -v short form works

### Verification

- **Compilation**: Clean compilation with no warnings
- **Help text**: `cargo run --bin crewchief-maproom -- db cleanup-stale --help` shows complete help with examples
- **Unit tests**: All 4 tests passing (test_cleanup_stale_*)
- **Command routing**: Database connection is established before placeholder output (correct per ticket spec)

### Design Decisions

1. **Safe by default**: `confirm` defaults to false, requiring explicit `--confirm` for deletions
2. **Clear help text**: Includes 3 usage examples showing dry-run, confirm, and verbose modes
3. **Placeholder routing**: Establishes database connection to validate connection string before execution
4. **Follows patterns**: Matches existing `DbCommand::Migrate` structure for consistency

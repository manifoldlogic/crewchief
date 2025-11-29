# Ticket: MRPROG-1006: Update scan command help text and documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update the help text for the scan command to clearly document that it defaults to the current directory and explain the --verbose flag behavior. Add usage examples showing the simplified invocation.

## Background
Users currently type `maproom scan .` because they don't know the current directory is the default. Clear help text makes this discoverable, reducing friction.

This ticket updates the in-CLI help text (--help output) and adds examples to make the default behavior obvious. This is task 6 from Phase 1 (Progress Tracking Foundation) of the MRPROG project plan.

**Plan Reference**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md` - Phase 1, Task 6: "Update scan command help text"

## Acceptance Criteria
- [x] Scan command help text updated with clear description
- [x] Path parameter help text clarifies default behavior: "Path to scan (defaults to current directory)"
- [x] --verbose flag help text explains purpose
- [x] Command description mentions progress display
- [x] `maproom scan --help` shows updated text
- [x] Examples in help output demonstrate: `maproom scan` (no path needed)

## Technical Requirements

**Update Command Description:**
```rust
/// Scan and index a worktree with real-time progress display
///
/// By default, scans the current directory and shows progress updates.
/// Use --verbose for detailed output (future enhancement).
///
/// Examples:
///   maproom scan                    # Scan current directory
///   maproom scan /path/to/repo      # Scan specific path
///   maproom scan --verbose          # Scan with detailed output
#[command(name = "scan")]
Scan {
    #[arg(
        short,
        long,
        help = "Path to scan (defaults to current directory)"
    )]
    path: Option<PathBuf>,

    // ... other fields ...

    #[arg(long, help = "Show detailed output (currently same as default)")]
    verbose: bool,
}
```

**Help Text Goals:**
1. **Discoverability**: Make default directory behavior obvious
2. **Simplicity**: Show minimal invocation first
3. **Clarity**: Explain what progress display does
4. **Future-proof**: Mention verbose flag for future enhancements

**Before (current):**
```
maproom scan [OPTIONS]

Options:
  -p, --path <PATH>    Path to scan
  --verbose            Show detailed output
```

**After (improved):**
```
Scan and index a worktree with real-time progress display

By default, scans the current directory and shows progress updates.

Usage: maproom scan [OPTIONS]

Options:
  -p, --path <PATH>
          Path to scan (defaults to current directory)

  --verbose
          Show detailed output (currently same as default)

Examples:
  maproom scan                    # Scan current directory
  maproom scan /path/to/repo      # Scan specific path
  maproom scan --verbose          # Scan with detailed output
```

## Implementation Notes

1. Update `Commands::Scan` doc comment in main.rs with detailed description and examples
2. Update `path` field help attribute to mention default behavior
3. Update `verbose` field help attribute to explain current state and future use
4. Add examples section using Clap's doc comment features (triple-slash comments support examples)
5. Test help output: `cargo run -- scan --help`

**Clap Documentation Features:**
- Use `///` doc comments for command description (appears at top of help)
- Use `#[arg(help = "...")]` for individual argument descriptions
- Doc comments support examples section that Clap will format nicely

## Dependencies
- **BLOCKED BY**: MRPROG-1003 (add --verbose flag to CLI integration)
  - Needs the `verbose` flag to exist in the CLI interface before documentation can reference it

## Risk Assessment
- **Risk**: None - this is documentation-only work
  - **Mitigation**: N/A - no code logic changes, only help text

## Files/Packages Affected
- **MODIFY**: `crates/maproom/src/main.rs` (Commands::Scan enum variant and field attributes)

## Estimated Effort
30 minutes - 1 hour

## Verification Steps
Run `cargo run -- scan --help` and verify:
1. Clear description of default behavior appears at top
2. Path parameter help text mentions "defaults to current directory"
3. Examples section shows `maproom scan` without path argument
4. Verbose flag is documented with current behavior noted
5. Help text is well-formatted and easy to read

## References
- **Analysis**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/analysis.md` (Requirements Synthesis section)
- **Plan**: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Phase 1, Task 6)

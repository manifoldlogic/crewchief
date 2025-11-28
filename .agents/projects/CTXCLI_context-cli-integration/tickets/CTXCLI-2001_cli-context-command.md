# Ticket: CTXCLI-2001: Add CLI Context Command Variant

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
Add `Context` command variant to the CLI `Commands` enum with all arguments for context retrieval.

## Background
This is the first ticket of Phase 2 (CLI Context Command). It defines the CLI interface for the context command, allowing users to retrieve context bundles directly from the command line. This ticket focuses only on argument parsing - the actual handler logic is in CTXCLI-2002.

Reference: [planning/architecture.md](../planning/architecture.md) - Section 1: CLI Context Command

## Acceptance Criteria
- [ ] `Context` variant added to `Commands` enum in `main.rs`
- [ ] `crewchief-maproom context --help` shows all options with descriptions
- [ ] Required argument: `--chunk-id <ID>` (i64)
- [ ] Optional arguments with defaults: `--budget` (6000), `--max-depth` (2)
- [ ] Boolean flags: `--callers`, `--callees`, `--tests`, `--docs`, `--config`, `--json`
- [ ] Arguments parse correctly for minimal input: `context --chunk-id 12345`
- [ ] Arguments parse correctly with all flags
- [ ] Unit tests pass for argument parsing

## Technical Requirements
- Use clap derive macros for argument definition
- All boolean expand flags default to `false`
- `--json` flag for machine-readable output (default: human-readable)
- Chunk ID is required (no default)

## Implementation Notes

### Command Definition
```rust
/// Retrieve context bundle for a chunk
Context {
    /// Chunk ID to retrieve context for
    #[arg(long)]
    chunk_id: i64,

    /// Maximum tokens for the bundle (default: 6000)
    #[arg(long, default_value_t = 6000)]
    budget: usize,

    /// Include caller functions
    #[arg(long)]
    callers: bool,

    /// Include callee functions
    #[arg(long)]
    callees: bool,

    /// Include test files
    #[arg(long)]
    tests: bool,

    /// Include documentation
    #[arg(long)]
    docs: bool,

    /// Include configuration files
    #[arg(long)]
    config: bool,

    /// Maximum traversal depth (default: 2)
    #[arg(long, default_value_t = 2)]
    max_depth: i32,

    /// Output as JSON instead of human-readable
    #[arg(long)]
    json: bool,
}
```

### Unit Tests
```rust
#[test]
fn test_context_command_parsing_minimal() {
    let cli = Cli::parse_from(["maproom", "context", "--chunk-id", "12345"]);
    match cli.command {
        Commands::Context { chunk_id, budget, .. } => {
            assert_eq!(chunk_id, 12345);
            assert_eq!(budget, 6000); // default
        }
        _ => panic!("Expected Context command"),
    }
}

#[test]
fn test_context_command_parsing_with_expands() {
    let cli = Cli::parse_from([
        "maproom", "context",
        "--chunk-id", "12345",
        "--budget", "4000",
        "--callers",
        "--callees",
        "--tests",
    ]);
    // ... verify options
}
```

## Dependencies
- None (can be developed in parallel with Phase 1, but tested after CTXCLI-1002)

## Risk Assessment
- **Risk**: Inconsistent argument naming with daemon params
  - **Mitigation**: Match CLI args to `ExpandConfig` field names (callers, callees, tests, docs, config, max_depth)

## Files/Packages Affected
- `crates/maproom/src/main.rs` (modify - add Context variant to Commands enum)

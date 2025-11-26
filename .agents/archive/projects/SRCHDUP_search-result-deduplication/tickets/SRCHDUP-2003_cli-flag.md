# Ticket: SRCHDUP-2003: Add --deduplicate CLI flag to search command

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (805 lib tests pass)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Add `--deduplicate`/`--no-deduplicate` flags to the Rust CLI `search` command so users can control deduplication behavior from the command line. Default should be enabled (deduplicate=true).

## Background

The CLI is one of the interfaces to the search pipeline. Users testing or debugging may want to see all results including duplicates. The flag provides this control while maintaining the default dedup-enabled behavior.

**Reference:** plan.md Phase 2, architecture.md Section 5 "CLI Flag Support"

## Acceptance Criteria

- [ ] `SearchArgs` struct has `deduplicate: bool` field with clap attribute
- [ ] `--deduplicate` flag defaults to `true`
- [ ] `--no-deduplicate` can be used to disable
- [ ] Flag value is passed to `SearchOptions::with_deduplicate()`
- [ ] `crewchief-maproom search --help` shows the new flag
- [ ] Manual test: `crewchief-maproom search --no-deduplicate` returns duplicates

## Technical Requirements

### SearchArgs Modification
```rust
#[derive(Parser, Debug)]
pub struct SearchArgs {
    /// Search query
    query: String,

    /// Repository name
    #[arg(long)]
    repo: String,

    /// Worktree name (optional)
    #[arg(long)]
    worktree: Option<String>,

    /// Maximum results
    #[arg(long, default_value = "10")]
    limit: usize,

    /// Deduplicate results across worktrees (default: true)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    deduplicate: bool,
}
```

### Handler Update
```rust
// In search command handler
let options = SearchOptions::new(repo_id, worktree_id, args.limit)
    .with_deduplicate(args.deduplicate);

let results = pipeline.search(&args.query, &options).await?;
```

### Alternative: Negation Flag Pattern
If clap version supports it, can use the negation pattern:
```rust
#[arg(long, default_value_t = true)]
#[arg(action = clap::ArgAction::Set)]
deduplicate: bool,
```

Or explicit negative flag:
```rust
#[arg(long)]
no_deduplicate: bool,  // If present, deduplicate = false
```

## Implementation Notes

1. **Find CLI definition** - Locate where `SearchArgs` is defined (main.rs, cli.rs, or commands/)
2. **Check clap version** - Boolean flag handling varies by clap version
3. **Test both flags** - Ensure both `--deduplicate` and `--no-deduplicate` work
4. **Update help text** - Ensure flag description is clear

### Verification Commands
```bash
# Should work (default dedup)
crewchief-maproom search "validate" --repo crewchief

# Should show duplicates
crewchief-maproom search "validate" --repo crewchief --no-deduplicate

# Should show help with new flag
crewchief-maproom search --help
```

## Dependencies

- SRCHDUP-2002 (pipeline must support deduplicate option)

## Risk Assessment

- **Risk**: Clap version incompatibility with boolean flag syntax
  - **Mitigation**: Check Cargo.toml for clap version, adjust syntax accordingly
- **Risk**: CLI tests may break if they check result counts
  - **Mitigation**: Update tests to account for deduplication or use --no-deduplicate

## Files/Packages Affected

- `crates/maproom/src/main.rs` or `crates/maproom/src/cli.rs` or `crates/maproom/src/commands/search.rs`

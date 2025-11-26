# Ticket: SRCHDUP-4002: Update search documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation only)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: N/A - This is a documentation-only ticket.

## Agents
- technical-researcher
- verify-ticket
- commit-ticket

## Summary

Update search-related documentation to describe the new deduplication feature, including API usage, CLI flags, MCP parameters, and known limitations.

## Background

Documentation ensures users understand and can effectively use the deduplication feature. Key documentation updates include explaining default behavior, how to opt-out, and the identity key limitations.

**Reference:** plan.md Phase 4

## Acceptance Criteria

- [ ] Rust API documentation in code comments is complete
- [ ] CLI `--help` output describes `--deduplicate` flag clearly
- [ ] MCP tool description mentions `deduplicate` parameter
- [ ] Any existing search documentation is updated
- [ ] Known limitations (line number sensitivity) are documented

## Technical Requirements

### Code Documentation

#### dedup.rs Module Doc
```rust
//! Search result deduplication.
//!
//! This module provides deduplication of search results across worktrees.
//! When the same code exists in multiple worktrees (e.g., main and feature
//! branches), search results may contain duplicates. This module groups
//! results by their logical identity and returns only the highest-scoring
//! representative from each group.
//!
//! # Identity Key
//!
//! Results are considered duplicates if they have the same:
//! - `relpath` (relative file path)
//! - `symbol_name` (function/class name, or empty)
//! - `start_line` (line number)
//!
//! Note: Line number sensitivity means code that has shifted by even one
//! line (e.g., due to added imports) will not be considered a duplicate.
//!
//! # Usage
//!
//! Deduplication is enabled by default. To disable:
//! ```rust
//! let options = SearchOptions::new(repo_id, None, 10).without_dedup();
//! ```
```

#### SearchOptions Documentation
```rust
/// Search configuration options.
pub struct SearchOptions {
    // ...

    /// Whether to deduplicate results across worktrees.
    ///
    /// When enabled (default), results with the same identity
    /// (relpath, symbol_name, start_line) are grouped, and only
    /// the highest-scoring instance is returned.
    ///
    /// Default: `true`
    pub deduplicate: bool,
}
```

### CLI Help Text
Ensure `--deduplicate` flag help is clear:
```
--deduplicate <BOOL>
    Deduplicate results across worktrees. Results with the same
    file path, symbol name, and line number are grouped, returning
    only the highest-scoring instance.
    [default: true]
```

### MCP Tool Description
Update search tool schema description:
```typescript
deduplicate: {
  type: 'boolean',
  description: 'Deduplicate results across worktrees. When true, ' +
    'results with the same file path, symbol name, and line number ' +
    'are grouped, returning only the highest-scoring instance. ' +
    'Default: true',
},
```

### README or Docs Updates
If there's a search documentation file (e.g., `docs/search.md`):
- Add section on deduplication
- Explain when to disable (debugging, seeing all variants)
- Document known limitations

## Implementation Notes

1. **Find existing docs** - Check `docs/` folder for search documentation
2. **Update inline docs** - Add rustdoc comments to new types and functions
3. **Verify help output** - Run `crewchief-maproom search --help` to check
4. **Check MCP inspector** - Verify tool description appears in MCP clients

### Documentation Locations
- `crates/maproom/src/search/dedup.rs` - Module and type docs
- `crates/maproom/src/search/results.rs` - SearchOptions docs
- `packages/maproom-mcp/src/tools/search.ts` - MCP tool description
- `docs/` - Any user-facing documentation

## Dependencies

- SRCHDUP-3003 (all features implemented)

## Risk Assessment

- **Risk**: Documentation gets out of sync with code
  - **Mitigation**: Verify docs match actual behavior before completing
- **Risk**: Missing important use cases
  - **Mitigation**: Review common search scenarios when writing docs

## Files/Packages Affected

- `crates/maproom/src/search/dedup.rs` (module docs)
- `crates/maproom/src/search/results.rs` (SearchOptions docs)
- `packages/maproom-mcp/src/tools/search.ts` (tool description)
- `docs/` (any existing search documentation)

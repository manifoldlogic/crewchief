# Analysis: maproom ignore patterns

## Problem Definition

Maproom currently has **fragmented and inconsistent** ignore pattern handling across its two main operations:

1. **Scan operation** (initial indexing): Uses `ignore` crate's `WalkBuilder` with `.git_ignore(true)` to respect `.gitignore` patterns
2. **Watch operation** (incremental updates): Uses git status polling which automatically respects `.gitignore`, but also has a separate `IgnorePatternMatcher` with hardcoded default patterns

There is **no way to ignore files that are committed to git but should not be indexed**. Common examples include:
- Large generated files (test fixtures, baseline snapshots, demo data)
- Build artifacts that are checked in for distribution
- Documentation in other languages
- Legacy code directories

This creates indexing overhead and search noise for files users explicitly want to exclude from semantic search.

## Context

The issue surfaced in real usage:
- Users need to commit large test fixtures for reproducibility
- Generated SQL baselines are checked into git for validation
- Legacy directories exist but shouldn't pollute search results
- Lock files and other metadata are version-controlled but irrelevant to code search

Git's `.gitignore` solves "don't version this" but we need ".maproomignore" for "don't index this (even though it's versioned)".

## Existing Solutions

### Industry Patterns

1. **Ripgrep** (our `WalkBuilder` source): Supports multiple ignore files (`.gitignore`, `.ignore`, `.rgignore`) with override precedence
2. **Language servers**: Use `.{tool}ignore` pattern (e.g., `.prettierignore`, `.eslintignore`)
3. **Search tools**: Often combine `.gitignore` + tool-specific ignore files

### Current Codebase State

**Scan operation** (`crates/maproom/src/indexer/mod.rs:274-286`):
```rust
let mut walk = WalkBuilder::new(&root_abs);
walk.hidden(false)
    .ignore(true)
    .git_ignore(true)
    .git_exclude(true);
// Note: exclude parameter exists for programmatic use (not exposed via CLI in MVP)
if let Some(globs) = &exclude {
    let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);
    for g in globs {
        ob.add(&format!("!{}", g))?;
    }
    walk.overrides(ob.build()?);
}
```

**Watch operation** (`crates/maproom/src/incremental/ignore.rs:31-91`):
- Has `IgnorePatternMatcher` struct with `DEFAULT_IGNORE_PATTERNS`
- Includes hardcoded patterns: `*.log`, `.git/**`, `**/node_modules`, etc.
- Has `from_gitignore()` method that **reads .gitignore** and merges with defaults
- **NOT currently used** - GitPoller automatically respects `.gitignore` via git status

## Research Findings

### Key Insights

1. **Watch doesn't use IgnorePatternMatcher**: The existing `ignore.rs` module is tested but not integrated into the actual watch pipeline. GitPoller relies on git's native ignore handling.

2. **No shared ignore logic**: Scan and watch have completely different mechanisms despite needing identical semantics.

3. **Git-aware vs file-aware**: Git status automatically filters out gitignored files, but doesn't know about `.maproomignore`. We need post-processing.

4. **Programmatic exclude parameter**: The scan function has an `exclude` parameter for programmatic use (e.g., daemon integration) but it's not exposed via CLI in MVP.

### Pattern Precedence (MVP Scope)

Most specific wins:
1. `.maproomignore` (repository-specific, new in this project)
2. `.gitignore` (already respected by both operations)
3. Default patterns (hardcoded minimums)

**Note:** CLI overrides (via `--exclude` flag) are deferred to Phase 2. The `exclude` parameter in `scan_worktree()` exists for programmatic use (e.g., daemon integration) but is not user-facing in MVP.

## Constraints

### Technical

- **Must not break existing behavior**: `.gitignore` must continue to work
- **Git poller architecture**: Can't change git status output, must filter events post-detection
- **WalkBuilder integration**: Must work with existing `ignore` crate patterns
- **Performance**: Ignore matching happens on every file - must be fast

### User Experience

- **Zero-config**: Existing repos work without `.maproomignore`
- **Git-like syntax**: Users already know glob patterns from `.gitignore`
- **Clear precedence**: Behavior must be predictable when patterns conflict

### Implementation

- **Rust-only changes**: This is purely maproom indexer work
- **No database migration**: No schema changes needed
- **Backward compatible**: Old installs should just work

## Success Criteria

The project succeeds when:

1. **Unified ignore handling**: Both scan and watch use identical pattern matching logic
2. **`.maproomignore` support**: Users can create `.maproomignore` in repo root with gitignore-style patterns
3. **Correct precedence**: Patterns override in expected order (.maproomignore > .gitignore > defaults)
4. **Tested critical paths**:
   - Scan respects `.maproomignore` patterns
   - Watch respects `.maproomignore` patterns
   - Existing `.gitignore` behavior unchanged
   - Patterns are relative to repo root
   - Invalid patterns cause watcher startup to fail (fail-fast error handling)
5. **Documentation**: CLAUDE.md updated with `.maproomignore` usage

### Measurable Outcomes

- [ ] Can create `.maproomignore` with pattern `test-fixtures/**` and verify files not indexed
- [ ] Scan and watch produce identical ignore decisions for same file
- [ ] Test coverage for pattern precedence logic
- [ ] Zero regression in existing gitignore behavior

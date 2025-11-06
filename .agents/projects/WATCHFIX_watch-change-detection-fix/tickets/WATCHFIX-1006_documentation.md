# Ticket: WATCHFIX-1006: Documentation and Code Polish

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Add comprehensive documentation, code comments, and polish to all changed code. Update project changelog and ensure all public functions have doc comments. This makes the fix maintainable and helps future developers understand the bug and solution.

## Background
Good documentation prevents future bugs and helps onboard new contributors. Since this fix addresses a subtle path normalization issue, clear comments explaining WHY paths are normalized and WHERE the bug was are essential. This ticket ensures the code is self-documenting and the fix is properly recorded.

This implements Phase 6 (Documentation & Polish) from the project plan.

## Acceptance Criteria
- [ ] All new public functions have doc comments with examples
- [ ] Complex logic in processor_task has inline comments explaining the fix
- [ ] Code comments reference the bug being fixed
- [ ] CHANGELOG.md updated (if project uses one)
- [ ] No TODO comments left unaddressed
- [ ] All functions compile with `cargo doc` without warnings
- [ ] README or watch command docs updated (if applicable)

## Technical Requirements

### 1. Doc Comments for New Functions

Add comprehensive rustdoc comments to `normalize_to_relpath()` in `path_utils.rs`:

```rust
/// Convert absolute filesystem path to repository-relative path.
///
/// This function is critical for watch command correctness. The file watcher
/// provides absolute paths, but the database stores relative paths. Using
/// mismatched formats causes file lookups to fail.
///
/// # Arguments
/// * `absolute_path` - Full filesystem path (e.g., `/workspace/src/main.rs`)
/// * `repo_root` - Repository root directory (e.g., `/workspace`)
///
/// # Returns
/// * `Ok(PathBuf)` - Relative path (e.g., `src/main.rs`)
/// * `Err(_)` - Path is outside repo or contains `..` components
///
/// # Security
/// Rejects paths outside repository and paths with parent directory
/// components to prevent path traversal attacks.
///
/// # Example
/// ```
/// let abs = Path::new("/workspace/packages/cli/src/main.ts");
/// let root = Path::new("/workspace");
/// let rel = normalize_to_relpath(abs, root)?;
/// assert_eq!(rel.to_str().unwrap(), "packages/cli/src/main.ts");
/// ```
```

### 2. Inline Comments in processor_task

Add comment block in `indexer/mod.rs` explaining the bug and fix:

```rust
// BUG FIX: Normalize path to relative format ONCE at entry.
// Previously, we passed absolute paths to get_file_id_by_path() which
// failed to find files (DB stores relative paths). This caused existing
// files to be misclassified as NEW, leading to "File not found" errors.
// See: .agents/projects/WATCHFIX_watch-change-detection-fix/planning/analysis.md
let relpath = match normalize_to_relpath(&indexing_event.path, &root_clone) {
    // ...
};
```

### 3. CHANGELOG Entry

If `CHANGELOG.md` exists in project root, add:

```markdown
## [Unreleased]

### Fixed
- **watch**: Fixed file change detection misclassifying modified files as new files
  - Root cause: Path format mismatch between file watcher (absolute) and database (relative)
  - Impact: Watch now correctly re-indexes modified files with updated timestamps
  - Added file size limits (10MB) to prevent DoS attacks
  - Added path traversal protection in normalization utility
```

### 4. Module Documentation

Update `crates/maproom/src/incremental/mod.rs` or `lib.rs`:

```rust
//! # Path Handling Strategy
//!
//! The watch command deals with two path representations:
//! - **Absolute paths**: From file watcher (e.g., `/workspace/src/main.rs`)
//! - **Relative paths**: Stored in database (e.g., `src/main.rs`)
//!
//! Always normalize to relative paths for database queries using
//! [`normalize_to_relpath()`](path_utils::normalize_to_relpath).
```

### 5. Watch Command Documentation

Update any watch-related docs to mention:
- File size limit (10MB)
- Symlink behavior (allowed, logged)
- Path requirements (must be within repo)

## Implementation Notes

- Run `cargo doc --open` to preview documentation
- Use rustdoc conventions (`///` for doc comments, `//!` for module docs)
- Include examples in doc comments where helpful
- Reference planning docs for detailed context
- Keep comments concise - code should be self-documenting where possible
- Ensure all `warn!()` log messages are clear and actionable

## Dependencies

**Depends on:**
- WATCHFIX-1001 (path normalization utility)
- WATCHFIX-1002 (processor_task refactor)
- WATCHFIX-1003 (processor path handling)
- WATCHFIX-1004 (security/performance)
- WATCHFIX-1005 (integration tests)

All implementation must be complete before documentation can be finalized.

## Risk Assessment

**Risk**: Documentation might not match final implementation if code changes during development
- **Mitigation**: This ticket should be the last one executed, after all code is finalized

**Risk**: Missing doc comments on some functions
- **Mitigation**: Run `cargo doc` and check for warnings, grep for `pub fn` without doc comments

## Files/Packages Affected

**Files to modify:**
- `crates/maproom/src/incremental/path_utils.rs` - Add comprehensive doc comments
- `crates/maproom/src/indexer/mod.rs` - Add inline comments in processor_task
- `crates/maproom/src/incremental/processor.rs` - Add comments explaining path handling
- `crates/maproom/src/incremental/mod.rs` or `lib.rs` - Add module-level documentation
- `CHANGELOG.md` - Add entry (if file exists)
- `README.md` or watch docs - Update if documentation exists

**Quality Gates:**
- `cargo doc` runs without warnings
- All public items have doc comments
- Code comments explain non-obvious logic
- No unaddressed TODOs in changed files

## Estimated Effort

2 hours

## Priority

MEDIUM - Polish step, not blocking but important for maintainability

## References

**Planning documents:**
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/plan.md` - Phase 6 deliverables
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/analysis.md` - For accurate bug description
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/architecture.md` - For solution details
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/security-review.md` - For security considerations

**Rust documentation:**
- [Rustdoc Book](https://doc.rust-lang.org/rustdoc/) - Documentation conventions
- [API Guidelines](https://rust-lang.github.io/api-guidelines/documentation.html) - Best practices

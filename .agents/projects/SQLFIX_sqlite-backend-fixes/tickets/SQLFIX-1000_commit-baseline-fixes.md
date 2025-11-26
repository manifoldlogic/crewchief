# Ticket: SQLFIX-1000: Commit Baseline Fixes

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This ticket commits existing fixes; verification is `cargo check` passing
- "Tests pass" means `cargo check` (default postgres feature) was executed and passed

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Commit the baseline fixes that were made during the initial investigation of SQLite backend issues. These changes fix Postgres backend compilation and restore the corrupted VSCode extension.

## Background
During investigation of the SQLite backend failures, several fixes were made to the codebase that are prerequisites for the SQLFIX project but remain uncommitted. These include:
- Feature-gating the SQLite module to prevent compile errors when building with default (postgres) features
- Refactoring PostgresStore to use connection pool instead of single Client
- Fixing type mismatches in queries.rs
- Restoring the corrupted VSCode extension

**Plan Reference**: Phase 0 - Prerequisites

## Acceptance Criteria
- [ ] All listed files are staged and committed with a proper Conventional Commit message
- [ ] `cargo check` passes (default postgres feature, no regressions)
- [ ] `cargo check --features postgres` passes explicitly
- [ ] VSCode extension TypeScript compiles without errors
- [ ] Git working tree is clean after commit

## Technical Requirements
- Commit message follows Conventional Commits format: `fix(maproom): commit baseline fixes for SQLite project`
- All changes must be committed together as a single logical commit
- No new code changes - only commit existing uncommitted work

## Implementation Notes

### Files to Commit
1. **`crates/maproom/src/db/mod.rs`**
   - Added `#[cfg(feature = "sqlite")]` gate on sqlite module import
   - Prevents compile error when sqlite feature is disabled

2. **`crates/maproom/src/db/factory.rs`**
   - Feature-gated SQLite imports and usage
   - Ensures factory compiles with either backend

3. **`crates/maproom/src/db/postgres/mod.rs`**
   - Refactored from single `Client` to `PgPool` connection pool
   - Fixed `batch_upsert_embeddings` which tried to clone non-Clone `Client`

4. **`crates/maproom/src/db/queries.rs`**
   - Removed duplicate SearchHit struct definition
   - Added `use super::SearchHit;` import

5. **`packages/vscode-maproom/src/extension.ts`**
   - Restored from commit `f099757b` (was corrupted with placeholder text)
   - Fixed `ensureServicesRunning()` API mismatch (0 args, not 2)

### Verification Steps
```bash
# Verify Rust compiles
cargo check
cargo check --features postgres

# Verify TypeScript compiles
cd packages/vscode-maproom && pnpm build
```

## Dependencies
- None - this is the first ticket (Phase 0)

## Risk Assessment
- **Risk**: Uncommitted changes could be lost if working directory is reset
  - **Mitigation**: Commit early in project execution
- **Risk**: Changes might conflict with other work on the branch
  - **Mitigation**: Verify clean merge with `git status` before commit

## Files/Packages Affected
- `crates/maproom/src/db/mod.rs`
- `crates/maproom/src/db/factory.rs`
- `crates/maproom/src/db/postgres/mod.rs`
- `crates/maproom/src/db/queries.rs`
- `packages/vscode-maproom/src/extension.ts`

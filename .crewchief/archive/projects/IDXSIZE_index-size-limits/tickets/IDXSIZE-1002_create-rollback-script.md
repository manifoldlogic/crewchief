# Ticket: IDXSIZE-1002: Create Rollback Script

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (SQL file creation, no tests to execute)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create SQL rollback script that reverses migration 0013 by dropping the new multi-index strategy and attempting to restore the original covering index.

## Background
Following database migration best practices, we need a rollback procedure even though we don't recommend using it. The rollback script will drop the three new indexes and attempt to recreate the original `idx_chunks_search_covering`. However, this rollback will FAIL if any large-preview chunks (>2704 bytes) were indexed after the forward migration, making this truly a forward-only migration.

This ticket implements Step 1.2 from `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md`.

## Acceptance Criteria
- [x] Rollback SQL file created (at `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql` - numbered 0017 to match forward migration)
- [x] File includes DROP statements for the 2 new indexes (Note: only 2 indexes exist, not 3 - hash-based index was not created due to PostgreSQL limitation)
- [x] File includes CREATE statement to restore original covering index
- [x] WARNING comment documents that rollback may fail on databases with large previews
- [x] SQL syntax is valid PostgreSQL
- [x] Rollback script is idempotent (uses IF EXISTS clauses)

## Technical Requirements

### Indexes to Drop
- `idx_chunks_search_small_preview` - Partial covering index for small previews
- `idx_chunks_search_basic` - Basic non-covering index fallback

**Note**: `idx_chunks_search_hash` was NOT created in migration 0017 due to PostgreSQL not supporting expressions in INCLUDE clauses, so it does not need to be dropped in rollback.

### Index to Recreate
- `idx_chunks_search_covering` - Original covering index (file_id, kind, start_line) INCLUDE (symbol_name, preview)

### Migration Requirements
- Use `DROP INDEX IF EXISTS` for defensive programming
- Use `CREATE INDEX CONCURRENTLY` to avoid table locks
- Add WARNING comment about potential failure on large-preview data
- Use transaction block (BEGIN/COMMIT)
- Include header comments explaining the rollback limitations

## Implementation Notes

Follow the SQL structure defined in `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` Step 1.2 (lines 92-112). The rollback is provided for completeness but forward migration is strongly preferred.

Include prominent warning that rollback will fail if database contains chunks with preview > 2704 bytes. This is an intentional limitation - once large previews are indexed, the only safe path is forward (to the multi-index strategy).

### Key Technical Details
- **Defensive programming**: Use IF EXISTS to avoid errors if rollback is run multiple times
- **CONCURRENTLY**: Allows index creation without locking the table
- **Transaction safety**: Wrap in BEGIN/COMMIT for atomic rollback
- **Warning placement**: Put WARNING at top of file where it's visible

### Expected Rollback Behavior
- ✅ **Success**: On databases that only have small previews (<= 2704 bytes)
- ❌ **Failure**: On databases with any chunk having preview > 2704 bytes
- 🎯 **Recommendation**: Use forward-fix instead of rollback if issues occur

## Dependencies
- IDXSIZE-1001 (need to know exact index names from forward migration)

## Risk Assessment

- **Risk**: Rollback fails on production data
  - **Mitigation**: Document warning clearly, recommend forward-fix instead of rollback

- **Risk**: Accidentally run rollback on wrong database
  - **Mitigation**: Include confirmation prompts in script header comments

- **Risk**: Rollback leaves database in partially-rolled-back state
  - **Mitigation**: Use transaction block and IF EXISTS clauses for idempotency

- **Risk**: User expects rollback to work but doesn't read warning
  - **Mitigation**: Place WARNING at top of file in prominent format

## Files/Packages Affected
- `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql` (new file - numbered 0017 to match forward migration)

## Planning References
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` - Step 1.2 (lines 89-118)

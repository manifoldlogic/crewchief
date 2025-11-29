# Ticket: IDXSIZE-1003: Update documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update CHANGELOG.md and create/update database index documentation to reflect the migration fixing PostgreSQL B-tree index size limit errors.

## Background
Users and future developers need to understand what changed, why, and how the new multi-index strategy works. This ticket documents the migration in user-facing CHANGELOG and technical DATABASE_INDICES documentation.

This ticket implements Step 1.3 from `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md`. The migration (IDXSIZE-1001) replaced a single covering index with a multi-index strategy to handle PostgreSQL's B-tree index size limits. This fixes production blocker errors affecting 50%+ of codebases when indexing code with large preview text (e.g., minified files, large constants).

## Acceptance Criteria
- [x] CHANGELOG.md updated with migration 0017 entry under "Unreleased" → "Fixed" section
- [x] DATABASE_INDICES.md updated with two-index strategy documentation (Covering Indices section)
- [x] CHANGELOG entry explains the problem (B-tree size limit errors) and solution (two-index strategy)
- [x] Documentation links to migration file location (`crates/maproom/migrations/0017_fix_index_size_limits.sql`)
- [x] Documentation explains performance characteristics (95%+ maintain index-only scans, 5% use heap lookups)
- [x] Documentation mentions storage impact (+31% typical, ~155MB)

## Technical Requirements
- Add "Fixed" entry to CHANGELOG.md under ## [Unreleased] section
- Follow the CHANGELOG entry format specified in plan.md (lines 128-137)
- Document all 3 new indexes: purpose, when each is used, size characteristics
  - `idx_chunks_repo_worktree` (repo_id, worktree_id)
  - `idx_chunks_paths` (repo_id, worktree_id, chunk_path)
  - `idx_chunks_preview` (repo_id, worktree_id, chunk_path, preview_text)
- Link to migration file: `crates/maproom/migrations/0013_fix_index_size_limits.sql`
- Reference planning documentation for technical details
- Explain query planner behavior with multiple indexes
- Document that this fixes production blocker affecting 50%+ of codebases
- Include monitoring queries and expected index usage patterns
- Create DATABASE_INDICES.md if it doesn't exist, or update existing index documentation

## Implementation Notes
The documentation should enable future maintainers to understand the design without reading all planning docs. Key points to cover:

**CHANGELOG.md**:
- Use the format from plan.md lines 128-137
- Emphasize this fixes a production blocker
- Mention the trade-off: storage (+31%) for reliability (100% success rate)

**DATABASE_INDICES.md** (or equivalent):
- Explain why single covering index failed (PostgreSQL 8KB page limit)
- Document the multi-index strategy: narrowest-to-widest approach
- Explain when each index is used (query planner picks by selectivity)
- Include size characteristics from planning analysis
- Provide example queries showing index usage
- Document expected index-only scan rates (95%+ for typical workloads)

If DATABASE_ARCHITECTURE.md exists, consider adding a cross-reference.

## Dependencies
- IDXSIZE-1001 (create-migration-sql) - Need migration file path and final SQL
- IDXSIZE-1002 (create-rollback-script) - Need rollback procedure details for documentation

## Risk Assessment
- **Risk**: Documentation doesn't match actual implementation
  - **Mitigation**: Review migration SQL from IDXSIZE-1001 before writing docs; verify index names and structure match

- **Risk**: Missing critical information for troubleshooting
  - **Mitigation**: Include monitoring queries and expected index usage patterns; document how to check if indexes are being used correctly

- **Risk**: Technical documentation is too dense for general users
  - **Mitigation**: Keep CHANGELOG entry concise and user-focused; put detailed technical info in DATABASE_INDICES.md

## Files/Packages Affected
- `/workspace/CHANGELOG.md` (update - add migration 0013 entry)
- `/workspace/docs/DATABASE_INDICES.md` (create or update - document new index strategy)
- `/workspace/docs/architecture/DATABASE_ARCHITECTURE.md` (possibly update if exists - add cross-reference)

# Ticket: COMPFIX-2005: Implement Status Command in Rust Maproom Binary

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - manual tests executed successfully
- [ ] **Verified** - by the verify-ticket agent

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
Implement `status` subcommand in crewchief-maproom binary to query and display worktree indexing status. This command is required by the PreFlightValidator (COMPFIX-1001) but was never implemented in the Rust binary.

## Background
The TypeScript PreFlightValidator (`packages/cli/src/search-optimization/validation/pre-flight-validator.ts`) implemented in COMPFIX-1001 calls:
```bash
crewchief-maproom status --repo crewchief --worktree main --json
```

However, this command **does not exist** in the Rust binary. Available commands are: db, cache, scan, upsert, watch, branch-watch, search, generate-embeddings, migrate.

This missing implementation is a critical blocker:
- **BLOCKS COMPFIX-2002** (End-to-End Validation) - Cannot run any optimizer tests
- All competition runs fail at Phase 1: Setup
- `verifyBaseBranchIndexed()` cannot check if base branch is indexed
- Cannot validate that worktrees are properly scanned

This was identified during E2E validation testing and documented in `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`.

## Acceptance Criteria
- [ ] `crewchief-maproom status` command exists and runs without error
- [ ] Lists all repos and worktrees with chunk counts from database
- [ ] `--repo <name>` filters output to specific repository
- [ ] `--worktree <name>` filters to specific worktree (requires --repo)
- [ ] `--json` flag outputs valid machine-readable JSON
- [ ] PreFlightValidator can call status command successfully
- [ ] Returns non-zero exit code if database connection fails
- [ ] Shows meaningful error message if no data indexed yet
- [ ] Manual test demonstrates command works with actual database

## Technical Requirements

### Command Interface
```bash
# Basic usage (all repos and worktrees)
crewchief-maproom status

# Filter by repo
crewchief-maproom status --repo crewchief

# Filter by worktree (requires --repo)
crewchief-maproom status --repo crewchief --worktree main

# JSON output for machine consumption
crewchief-maproom status --repo crewchief --worktree main --json
```

### JSON Output Format
```json
{
  "repos": [
    {
      "name": "crewchief",
      "worktrees": [
        {
          "name": "main",
          "chunk_count": 353879,
          "last_updated": "2025-11-10T23:59:00Z"
        }
      ]
    }
  ]
}
```

### Text Output Format
```
Repository: crewchief
  Worktree: main
    Chunks: 353,879
    Last Updated: 2025-11-10 23:59:00 UTC
```

### Database Queries
Query the existing database schema:
- Join `repos`, `worktrees`, and `chunks` tables
- Count chunks per worktree using `worktree_ids` JSONB field
- Get last updated timestamp from worktrees table
- Support optional filtering by repo name and worktree name

### Error Handling
- Database connection failure: Exit code 1, clear error message
- No repos indexed: Exit code 0, informative message
- Invalid filter arguments: Exit code 2, usage help
- Missing required --repo when --worktree specified: Exit code 2

## Implementation Notes

### File Structure
Add new status module to `crates/maproom/src/`:
- `status.rs` - Status command implementation
- Update `main.rs` - Add status subcommand to CLI parser

### Database Schema Reference
From `packages/maproom-mcp/config/init.sql`:
- `repos` table: id, name, created_at
- `worktrees` table: id, repo_id, name, created_at, updated_at
- `chunks` table: worktree_ids (JSONB array of worktree IDs)

### Query Logic
```sql
-- Count chunks per worktree
SELECT
  r.name as repo_name,
  w.name as worktree_name,
  COUNT(*) FILTER (WHERE c.worktree_ids @> jsonb_build_array(w.id)) as chunk_count,
  w.updated_at
FROM repos r
JOIN worktrees w ON w.repo_id = r.id
LEFT JOIN chunks c ON c.worktree_ids @> jsonb_build_array(w.id)
WHERE r.name = $1  -- Optional filter
  AND w.name = $2  -- Optional filter
GROUP BY r.name, w.name, w.updated_at
ORDER BY r.name, w.name;
```

### Integration Point
The PreFlightValidator calls this command via Node.js `spawn()`:
- Path: `packages/cli/bin/crewchief-maproom`
- Expected output: Valid JSON on stdout when `--json` flag used
- Exit code 0: Success
- Exit code non-zero: Failure (with error on stderr)

## Dependencies
- **Prerequisite**: COMPFIX-1001 (PreFlightValidator implementation) - COMPLETED
- **Blocks**: COMPFIX-2002 (End-to-End Validation)
- **Database**: PostgreSQL with existing repos/worktrees/chunks schema
- **External**: None - uses existing database connection patterns from other commands

## Risk Assessment
- **Risk**: Database query performance with large chunk counts
  - **Mitigation**: Use COUNT with FILTER, avoid loading all chunk records into memory

- **Risk**: JSON serialization of timestamps (timezone handling)
  - **Mitigation**: Use UTC timestamps consistently, ISO 8601 format

- **Risk**: Breaking changes to expected JSON output format
  - **Mitigation**: Match exact format expected by PreFlightValidator (see TypeScript code lines 73-87)

- **Risk**: Command not rebuilding to packages/cli/bin/
  - **Mitigation**: Test with `pnpm build:all` and verify binary update

## Files/Packages Affected
- `crates/maproom/src/main.rs` - Add status subcommand
- `crates/maproom/src/status.rs` - New file for status implementation
- `packages/cli/bin/crewchief-maproom` - Binary will be rebuilt
- `packages/cli/src/search-optimization/validation/pre-flight-validator.ts` - Caller (no changes needed, just verification)

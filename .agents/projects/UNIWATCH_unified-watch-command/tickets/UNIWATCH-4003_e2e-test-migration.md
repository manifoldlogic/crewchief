# Ticket: UNIWATCH-4003: Migrate E2E Test Script to SQLite

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the E2E test script `test_unified_watch_workflow.sh` to use SQLite instead of PostgreSQL, and add branch switch test scenarios.

## Background
The E2E test script was written during the PostgreSQL era and uses `psql` commands for database operations. With the SQLite migration complete, this script must be updated to work with the new database backend.

**Plan Reference:** Phase 4 - Testing, Task 3

## Acceptance Criteria
- [x] Script updated to use SQLite database path (`~/.maproom/maproom.db`)
- [x] All `psql` commands replaced with `sqlite3` or maproom CLI equivalents
- [x] Branch switch scenarios added to test script
- [x] Script runs successfully: `./crates/maproom/tests/e2e/test_unified_watch_workflow.sh`
- [x] Script cleans up test artifacts properly

## Technical Requirements
**Current (PostgreSQL) pattern to replace:**
```bash
DB_URL="${MAPROOM_DATABASE_URL:-postgresql://maproom:maproom@localhost:5432/maproom}"
psql "$DB_URL" -c "DELETE FROM maproom.repos WHERE name='$REPO_NAME'"
```

**New (SQLite) pattern:**
```bash
# SQLite database location
DB_PATH="${MAPROOM_DATABASE_URL:-$HOME/.maproom/maproom.db}"

# Cleanup using sqlite3
sqlite3 "$DB_PATH" "DELETE FROM repos WHERE name='$REPO_NAME';"

# OR use maproom CLI (preferred when available):
cargo run --bin crewchief-maproom -- db cleanup-stale --confirm
```

**Branch switch scenarios to add:**
```bash
# Test 1: Basic branch switch detection
echo "Testing branch switch detection..."
git checkout -b test-feature
# Edit file
# Verify indexed to test-feature

# Test 2: Rapid switches
echo "Testing rapid switch debouncing..."
git checkout main
git checkout test-feature
git checkout main
sleep 3
# Verify only final branch is current

# Test 3: Return to original branch
echo "Testing return to original branch..."
git checkout test-feature
# Verify state updates
```

## Implementation Notes
- Prefer maproom CLI commands for database cleanup when available
- Use `sqlite3` as fallback for direct SQL operations
- Script should be idempotent (can run multiple times)
- Add timeout to prevent hanging if watch command fails
- Capture and validate NDJSON output from watch command

**Key file:** `crates/maproom/tests/e2e/test_unified_watch_workflow.sh`

## Dependencies
- UNIWATCH-3001 (full implementation must be complete)
- UNIWATCH-4002 (integration tests should pass first)

## Risk Assessment
- **Risk**: sqlite3 not installed on all systems
  - **Mitigation**: Add check at script start, suggest installation
- **Risk**: Script timing sensitive
  - **Mitigation**: Use generous sleep values and explicit waits

## Files/Packages Affected
- `crates/maproom/tests/e2e/test_unified_watch_workflow.sh` (~100 lines modified)
  - Script is ~245 lines total
  - PostgreSQL references at lines 34-35, 180-209
  - Database cleanup logic in cleanup() function
  - Database verification section lines 174-210

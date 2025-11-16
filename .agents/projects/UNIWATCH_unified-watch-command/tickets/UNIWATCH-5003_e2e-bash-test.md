# Ticket: UNIWATCH-5003: Create End-to-End Bash Test Script

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
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
Create a bash script that simulates real developer workflow with actual git operations, maproom commands, and database verification.

## Background
E2E tests validate the complete user experience from CLI invocation through final database state. Unlike integration tests (which use Rust test harness), bash scripts test the actual binary that users will run.

This is part of Phase 5 (Testing & Verification) which validates all implementation work from Phases 1-4 before final release. E2E tests complement unit and integration tests by testing the actual binary in a real workflow.

## Acceptance Criteria
- [ ] Create `crates/maproom/tests/e2e/test_unified_watch_workflow.sh`
- [ ] Script creates temporary git repo
- [ ] Starts maproom watch in background
- [ ] Simulates developer workflow:
  - Edit file on main branch
  - Switch to feature branch
  - Edit file on feature branch
  - Switch back to main
- [ ] Verifies database state with psql queries
- [ ] Cleans up background process and temp files
- [ ] Script passes (exit code 0)
- [ ] Can run in CI environment

## Technical Requirements
- Location: `crates/maproom/tests/e2e/test_unified_watch_workflow.sh` (new file, ~100 lines)
- Use bash (`#!/bin/bash`)
- Set `-e` (exit on error)
- Proper cleanup on exit (trap)
- Database queries use psql or similar
- Binary path: `cargo build --release && ./target/release/crewchief-maproom`
- Execute from: `crates/maproom/` directory
- Make executable: `chmod +x tests/e2e/test_unified_watch_workflow.sh`

## Implementation Notes

### Script Structure
```bash
#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "Starting E2E test for unified watch command..."

# Setup
REPO=$(mktemp -d)
echo "Created temp repo: $REPO"
cd "$REPO"
git init
git config user.email "test@example.com"
git config user.name "Test User"
git checkout -b main
echo "initial" > main.txt
git add . && git commit -m "initial"

# Build binary
echo "Building maproom binary..."
cd /workspace/crates/maproom
cargo build --release 2>&1 | grep -v "^   "

# Start watch in background
echo "Starting watch in background..."
./target/release/crewchief-maproom watch "$REPO" &
WATCH_PID=$!

# Trap to cleanup
trap "kill $WATCH_PID 2>/dev/null || true; rm -rf $REPO; exit" EXIT INT TERM

# Wait for startup
sleep 2

# Developer workflow
echo "Simulating developer workflow..."

cd "$REPO"

echo "1. Working on main branch..."
echo "Working on main..." > main.txt
git add main.txt
git commit -m "main work"
sleep 2

echo "2. Switching to feature branch..."
git checkout -b feature-auth
echo "Auth code" > auth.txt
git add auth.txt
git commit -m "add auth"
sleep 3  # Wait for branch switch detection + indexing

echo "3. Switching back to main..."
git checkout main
echo "More main work" >> main.txt
git add main.txt
git commit -m "more main work"
sleep 2

# Verify database state
echo "Verifying database state..."

# Database connection
DB_URL="${MAPROOM_DATABASE_URL:-postgresql://maproom:maproom@localhost:5432/maproom}"

# Query chunks for main worktree
MAIN_CHUNKS=$(psql "$DB_URL" -t -c \
  "SELECT COUNT(*) FROM maproom.chunks WHERE worktree_name='main' AND repo_name='test-repo'" 2>/dev/null || echo "0")

# Query chunks for feature worktree
FEATURE_CHUNKS=$(psql "$DB_URL" -t -c \
  "SELECT COUNT(*) FROM maproom.chunks WHERE worktree_name='feature-auth' AND repo_name='test-repo'" 2>/dev/null || echo "0")

# Trim whitespace
MAIN_CHUNKS=$(echo "$MAIN_CHUNKS" | xargs)
FEATURE_CHUNKS=$(echo "$FEATURE_CHUNKS" | xargs)

# Assertions
FAILED=0

if [ "$MAIN_CHUNKS" -lt 1 ]; then
  echo -e "${RED}✗ FAIL: No chunks indexed to main (found: $MAIN_CHUNKS)${NC}"
  FAILED=1
else
  echo -e "${GREEN}✓ PASS: Main chunks indexed ($MAIN_CHUNKS chunks)${NC}"
fi

if [ "$FEATURE_CHUNKS" -lt 1 ]; then
  echo -e "${RED}✗ FAIL: No chunks indexed to feature-auth (found: $FEATURE_CHUNKS)${NC}"
  FAILED=1
else
  echo -e "${GREEN}✓ PASS: Feature chunks indexed ($FEATURE_CHUNKS chunks)${NC}"
fi

# Cleanup (trap will handle this)
kill $WATCH_PID 2>/dev/null || true
rm -rf "$REPO"

if [ $FAILED -eq 0 ]; then
  echo -e "${GREEN}✓ E2E test PASSED: $MAIN_CHUNKS main chunks, $FEATURE_CHUNKS feature chunks${NC}"
  exit 0
else
  echo -e "${RED}✗ E2E test FAILED${NC}"
  exit 1
fi
```

### Running the Script
```bash
cd crates/maproom
chmod +x tests/e2e/test_unified_watch_workflow.sh
./tests/e2e/test_unified_watch_workflow.sh
```

### CI Integration
For GitHub Actions or CI environments:
- Ensure PostgreSQL is available (use docker-compose or service container)
- Set `MAPROOM_DATABASE_URL` environment variable
- Run script as part of test suite

## Dependencies
- UNIWATCH-5001 (Execute and Verify Unit Tests) - MUST pass
- UNIWATCH-5002 (Create and Execute Integration Tests) - MUST pass
- PostgreSQL database must be running and accessible

## Risk Assessment
- **Risk**: Timing issues (race conditions)
  - **Mitigation**: Use generous sleep intervals (2-3 seconds); make timeouts configurable

- **Risk**: Database not accessible in CI
  - **Mitigation**: Use docker-compose for CI database; document setup requirements

- **Risk**: Binary not built
  - **Mitigation**: Script builds binary first; show build output for debugging

- **Risk**: Temp directory cleanup fails
  - **Mitigation**: Use trap to ensure cleanup on exit; handle errors gracefully

## Files/Packages Affected
- `crates/maproom/tests/e2e/test_unified_watch_workflow.sh` (NEW, ~100 lines)
- `crates/maproom/Cargo.toml` (no changes, but binary must be buildable)
- Database: `maproom.chunks` table (queries only, no modifications)

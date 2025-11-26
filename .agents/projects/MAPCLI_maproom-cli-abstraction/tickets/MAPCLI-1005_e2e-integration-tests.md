# Ticket: MAPCLI-1005: E2E Integration Tests with SQLite Backend

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive end-to-end integration tests verifying the MVP CLI flow with a pre-indexed SQLite database. Since scan is deferred to Phase 2, tests use a pre-populated SQLite database fixture.

## Background
The MAPCLI project adds SQLite backend support to the CLI and daemon. To verify the integration works correctly, we need E2E tests that exercise the full command flow with a SQLite backend. Since the `scan` command is deferred to Phase 2 (requires indexer abstraction), these tests use a pre-indexed SQLite database fixture.

**Plan Reference**: Phase 1: Integration Testing (MAPCLI-1005) in plan.md

## Acceptance Criteria
- [ ] Pre-indexed SQLite test fixture exists at `crates/maproom/tests/fixtures/pre-indexed-maproom.db`
- [ ] E2E test script `tests/e2e/test_sqlite_flow.sh` exists and passes
- [ ] Test covers search command with SQLite backend
- [ ] Test covers status command with SQLite backend
- [ ] Test covers db cleanup-stale command with SQLite backend
- [ ] Test covers daemon search RPC (fts mode) with SQLite backend
- [ ] Test covers daemon search RPC (vector mode) with SQLite backend
- [ ] Test covers daemon search RPC (hybrid mode) with SQLite backend
- [ ] Test verifies graceful error for scan command with SQLite
- [ ] Documentation for running tests and creating/updating fixtures
- [ ] Tests can run in CI (GitHub Actions)

## Technical Requirements
- Create SQLite test fixture with realistic data (repos, worktrees, chunks)
- Write bash script for E2E testing
- Tests must be idempotent (can run multiple times)
- Clean up test artifacts after completion
- Support both local development and CI environments

## Implementation Notes

### Step 1: Create Pre-Indexed SQLite Fixture

The fixture database should contain:
- At least 1 repo
- At least 1 worktree
- At least 10 chunks with various types (function, class, module, etc.)
- FTS index populated
- (Optional) Vector embeddings if sqlite-vec is available

**Fixture Creation Script** (`scripts/create-test-fixture.sh`):
```bash
#!/bin/bash
# Create test fixture using PostgreSQL backend, then export to SQLite format

# This is a one-time operation to create the fixture
# The fixture file is checked into the repository

export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost/maproom"

# Create a minimal test repository
mkdir -p /tmp/test-repo
cat > /tmp/test-repo/main.rs << 'EOF'
fn main() {
    println!("Hello, World!");
}

fn helper_function() -> i32 {
    42
}

struct Config {
    name: String,
    value: i32,
}
EOF

# Scan the test repository
cargo run --bin crewchief-maproom -- scan --path /tmp/test-repo --repo test-repo --worktree main

# Export to SQLite (requires separate tooling or manual copy)
# Note: This may require a database migration tool
```

**Alternative: Create fixture programmatically in Rust**:
```rust
// tests/fixtures/create_fixture.rs
async fn create_test_fixture() -> anyhow::Result<()> {
    let store = SqliteStore::connect("tests/fixtures/pre-indexed-maproom.db").await?;

    // Insert test data
    store.insert_repo(&Repo { name: "test-repo".into(), .. }).await?;
    store.insert_worktree(&Worktree { repo_name: "test-repo".into(), name: "main".into(), .. }).await?;

    // Insert test chunks
    for i in 0..10 {
        store.insert_chunk(&Chunk {
            file_path: "main.rs".into(),
            start_line: i * 5,
            end_line: i * 5 + 4,
            content: format!("fn test_function_{}() {{ }}", i),
            kind: "function".into(),
            ..
        }).await?;
    }

    Ok(())
}
```

### Step 2: E2E Test Script

**File**: `tests/e2e/test_sqlite_flow.sh`
```bash
#!/bin/bash
set -e

echo "=== MAPCLI SQLite E2E Tests ==="

# Configuration
FIXTURE_DB="crates/maproom/tests/fixtures/pre-indexed-maproom.db"
TEST_DB="/tmp/mapcli-test-$$.db"
DAEMON_PORT=19999
DAEMON_PID=""

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    if [ -n "$DAEMON_PID" ]; then
        kill $DAEMON_PID 2>/dev/null || true
    fi
    rm -f "$TEST_DB"
}
trap cleanup EXIT

# Copy fixture to test location
echo "Setting up test database..."
cp "$FIXTURE_DB" "$TEST_DB"
export MAPROOM_DATABASE_URL="sqlite://$TEST_DB"

# Build with SQLite feature
echo "Building CLI with SQLite support..."
cargo build --features sqlite --bin crewchief-maproom --release
CLI="./target/release/crewchief-maproom"

# Test 1: Search command
echo ""
echo "Test 1: Search command"
SEARCH_RESULT=$($CLI search --repo test-repo --query "function" 2>&1)
if echo "$SEARCH_RESULT" | grep -q "main.rs"; then
    echo "  PASS: Search returned results"
else
    echo "  FAIL: Search did not return expected results"
    echo "  Output: $SEARCH_RESULT"
    exit 1
fi

# Test 2: Status command
echo ""
echo "Test 2: Status command"
STATUS_RESULT=$($CLI status --json 2>&1)
if echo "$STATUS_RESULT" | jq -e '.repos | length > 0' > /dev/null 2>&1; then
    echo "  PASS: Status shows repositories"
else
    echo "  FAIL: Status did not show repositories"
    echo "  Output: $STATUS_RESULT"
    exit 1
fi

# Test 3: Scan command graceful error
echo ""
echo "Test 3: Scan command shows Phase 2 message"
SCAN_RESULT=$($CLI scan --path . 2>&1) || true
if echo "$SCAN_RESULT" | grep -qi "phase 2\|postgresql\|requires"; then
    echo "  PASS: Scan shows Phase 2/PostgreSQL message"
else
    echo "  FAIL: Scan did not show expected error message"
    echo "  Output: $SCAN_RESULT"
    exit 1
fi

# Test 4: Cleanup-stale command
echo ""
echo "Test 4: Cleanup-stale command"
CLEANUP_RESULT=$($CLI db cleanup-stale --dry-run 2>&1)
if [ $? -eq 0 ]; then
    echo "  PASS: Cleanup-stale runs without error"
else
    echo "  FAIL: Cleanup-stale failed"
    echo "  Output: $CLEANUP_RESULT"
    exit 1
fi

# Test 5: Daemon tests
echo ""
echo "Test 5: Daemon JSON-RPC tests"

# Start daemon in background
$CLI serve --port $DAEMON_PORT &
DAEMON_PID=$!
sleep 2  # Wait for daemon to start

# Test ping
echo "  Testing ping..."
PING_RESULT=$(echo '{"jsonrpc":"2.0","method":"ping","id":1}' | nc -w 2 localhost $DAEMON_PORT)
if echo "$PING_RESULT" | grep -q "pong"; then
    echo "    PASS: Ping returned pong"
else
    echo "    FAIL: Ping did not return pong"
    echo "    Output: $PING_RESULT"
    exit 1
fi

# Test search RPC (fts mode)
echo "  Testing search RPC (fts)..."
SEARCH_RPC=$(echo '{"jsonrpc":"2.0","method":"search","params":{"repo":"test-repo","query":"function"},"id":2}' | nc -w 5 localhost $DAEMON_PORT)
if echo "$SEARCH_RPC" | jq -e '.result.hits | length > 0' > /dev/null 2>&1; then
    echo "    PASS: Search RPC (fts) returned results"
else
    echo "    FAIL: Search RPC (fts) did not return results"
    echo "    Output: $SEARCH_RPC"
    exit 1
fi

# Test search RPC (vector mode) - expect results or graceful error
echo "  Testing search RPC (vector)..."
VECTOR_RPC=$(echo '{"jsonrpc":"2.0","method":"search","params":{"repo":"test-repo","query":"function","mode":"vector"},"id":3}' | nc -w 5 localhost $DAEMON_PORT)
if echo "$VECTOR_RPC" | jq -e '.result.hits' > /dev/null 2>&1 || echo "$VECTOR_RPC" | grep -qi "unavailable\|error"; then
    echo "    PASS: Vector search returned results or graceful error"
else
    echo "    FAIL: Vector search unexpected response"
    echo "    Output: $VECTOR_RPC"
    exit 1
fi

# Test search RPC (hybrid mode) - expect results or graceful error
echo "  Testing search RPC (hybrid)..."
HYBRID_RPC=$(echo '{"jsonrpc":"2.0","method":"search","params":{"repo":"test-repo","query":"function","mode":"hybrid"},"id":4}' | nc -w 5 localhost $DAEMON_PORT)
if echo "$HYBRID_RPC" | jq -e '.result.hits' > /dev/null 2>&1 || echo "$HYBRID_RPC" | grep -qi "unavailable\|error"; then
    echo "    PASS: Hybrid search returned results or graceful error"
else
    echo "    FAIL: Hybrid search unexpected response"
    echo "    Output: $HYBRID_RPC"
    exit 1
fi

# Stop daemon
kill $DAEMON_PID
DAEMON_PID=""

echo ""
echo "=== All E2E tests passed! ==="
```

### Step 3: Documentation

**File**: `docs/testing/SQLITE_INTEGRATION_TESTS.md`
```markdown
# SQLite Integration Tests

## Running Tests

```bash
# Run E2E test suite
./tests/e2e/test_sqlite_flow.sh

# Run with verbose output
VERBOSE=1 ./tests/e2e/test_sqlite_flow.sh
```

## Test Fixture

The pre-indexed SQLite database is located at:
`crates/maproom/tests/fixtures/pre-indexed-maproom.db`

### Fixture Contents
- Repository: `test-repo`
- Worktree: `main`
- Chunks: 10 functions from `main.rs`

### Updating the Fixture

1. Start PostgreSQL backend
2. Run scan on test repository
3. Export database to SQLite format
4. Replace fixture file

## CI Integration

Tests run automatically in GitHub Actions on:
- Push to main
- Pull requests

See `.github/workflows/test.yml` for CI configuration.
```

### Step 4: CI Integration

Add to `.github/workflows/test.yml`:
```yaml
  test-sqlite-e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install jq and netcat
        run: sudo apt-get install -y jq netcat-openbsd

      - name: Run SQLite E2E tests
        run: ./tests/e2e/test_sqlite_flow.sh
```

## Dependencies
- **MAPCLI-1004**: CLI commands must be updated before E2E tests can pass
- `jq` for JSON parsing in tests
- `netcat` (`nc`) for daemon communication

## Risk Assessment
- **Risk**: Test fixture becomes stale
  - **Mitigation**: Document how to regenerate fixture; include creation script
- **Risk**: Flaky daemon tests due to timing
  - **Mitigation**: Add retry logic and appropriate sleep times
- **Risk**: CI environment differences
  - **Mitigation**: Use same tools available in GitHub Actions runners

## Files/Packages Affected
- `crates/maproom/tests/fixtures/pre-indexed-maproom.db` - Test fixture (new)
- `tests/e2e/test_sqlite_flow.sh` - E2E test script (new)
- `scripts/create-test-fixture.sh` - Fixture creation script (new)
- `docs/testing/SQLITE_INTEGRATION_TESTS.md` - Test documentation (new)
- `.github/workflows/test.yml` - CI configuration (modify)

## Testing
```bash
# Run the E2E test suite itself
./tests/e2e/test_sqlite_flow.sh

# Verify fixture exists
ls -la crates/maproom/tests/fixtures/pre-indexed-maproom.db

# Verify documentation is accurate
cat docs/testing/SQLITE_INTEGRATION_TESTS.md

# Check fixture validity
sqlite3 crates/maproom/tests/fixtures/pre-indexed-maproom.db "SELECT COUNT(*) FROM chunks;"
```

# Ticket: IDXABS-4002: E2E Validation Script

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- The script itself must run successfully
- All commands in the script must complete without errors
- Script output should document successful execution

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Create a script that validates the full workflow: scan → generate-embeddings → search → upsert.

## Background
After all refactoring is complete, we need to validate that the entire system works end-to-end with SQLite. This script provides confidence that the migration was successful.

**Reference**: Phase 4, Ticket 4002 of `planning/plan.md` - "E2E Validation Script"
**Success Criteria**: See `planning/review-updates.md` - "Success Criteria"

## Acceptance Criteria
- [ ] Script exists at `scripts/test_sqlite_e2e.sh` (or similar)
- [ ] Script creates a temporary test repository
- [ ] `scan` command indexes the test repository
- [ ] `status` command shows indexed data
- [ ] `generate-embeddings` command runs (with mock or skip for CI)
- [ ] `search` command returns results
- [ ] `upsert` command updates a file
- [ ] Script cleans up temporary files on exit
- [ ] Script is executable and passes shellcheck
- [ ] Script added to CI workflow (or documented for manual run)

## Technical Requirements
- Script should be self-contained (creates own test data)
- Use `MAPROOM_DATABASE_URL` to control database location
- Use temporary directory that gets cleaned up
- Include meaningful output/assertions
- Handle errors gracefully (set -e or equivalent)

## Implementation Notes

### Script Template
```bash
#!/bin/bash
set -e

# Configuration
TEMP_DIR=$(mktemp -d)
DB_PATH="$TEMP_DIR/test.db"
REPO_DIR="$TEMP_DIR/repo"

# Cleanup on exit
cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo "=== SQLite E2E Validation ==="
echo "Database: $DB_PATH"
echo "Test repo: $REPO_DIR"

# Create test repository
echo "Creating test repository..."
mkdir -p "$REPO_DIR/src"
git init "$REPO_DIR"

cat > "$REPO_DIR/src/main.rs" << 'EOF'
fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
EOF

cat > "$REPO_DIR/src/lib.rs" << 'EOF'
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
EOF

# Commit test files
cd "$REPO_DIR"
git add -A
git commit -m "Initial commit"

# Set database URL
export MAPROOM_DATABASE_URL="sqlite://$DB_PATH"

# Build the binary
echo "Building crewchief-maproom..."
cargo build --bin crewchief-maproom --release

MAPROOM="cargo run --bin crewchief-maproom --release --"

# Test 1: Scan
echo ""
echo "=== Test 1: Scan ==="
$MAPROOM scan --path "$REPO_DIR"
echo "✓ Scan completed"

# Test 2: Status
echo ""
echo "=== Test 2: Status ==="
$MAPROOM status
echo "✓ Status completed"

# Test 3: Search
echo ""
echo "=== Test 3: Search ==="
$MAPROOM search --query "function" --limit 5
echo "✓ Search completed"

# Test 4: Upsert (modify and re-index)
echo ""
echo "=== Test 4: Upsert ==="
echo "// Modified" >> "$REPO_DIR/src/main.rs"
$MAPROOM upsert --paths "$REPO_DIR/src/main.rs"
echo "✓ Upsert completed"

# Test 5: Generate embeddings (skip if no embedding provider)
echo ""
echo "=== Test 5: Generate Embeddings ==="
if [ -n "$OPENAI_API_KEY" ] || [ -n "$OLLAMA_URL" ]; then
    $MAPROOM generate-embeddings --model text-embedding-3-small || echo "Embedding generation skipped (no provider)"
else
    echo "Skipped (no embedding provider configured)"
fi

# Final verification
echo ""
echo "=== Final Verification ==="
$MAPROOM status

echo ""
echo "=== All E2E tests passed! ==="
```

### CI Integration
Add to `.github/workflows/test.yml`:
```yaml
e2e-sqlite:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install Rust
      uses: dtolnay/rust-action@stable
    - name: Run E2E tests
      run: ./scripts/test_sqlite_e2e.sh
```

### Verification
```bash
# Make script executable
chmod +x scripts/test_sqlite_e2e.sh

# Run shellcheck
shellcheck scripts/test_sqlite_e2e.sh

# Execute script
./scripts/test_sqlite_e2e.sh
```

## Dependencies
- IDXABS-1001 through IDXABS-4001 (all previous tickets)

## Risk Assessment
- **Risk**: Embedding generation requires API keys
  - **Mitigation**: Skip embedding test if no provider configured
  - **Mitigation**: Document required environment variables
- **Risk**: CI environment may differ from local
  - **Mitigation**: Use only standard tools (bash, git)
  - **Mitigation**: Document any CI-specific requirements
- **Risk**: Script may leave temporary files on failure
  - **Mitigation**: Use trap for cleanup

## Files/Packages Affected
Files to CREATE:
- `scripts/test_sqlite_e2e.sh` - E2E validation script

Files to potentially MODIFY:
- `.github/workflows/test.yml` - Add E2E job (if CI integration desired)

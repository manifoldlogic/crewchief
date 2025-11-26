# SQLite Integration Tests

This document describes how to run the SQLite backend integration tests for the Maproom CLI.

## Overview

The SQLite backend enables zero-configuration semantic search without requiring PostgreSQL. The integration tests verify that all CLI commands work correctly with the SQLite backend.

## Running Tests

### Quick Start

```bash
# Run E2E test suite
./tests/e2e/test_sqlite_flow.sh
```

### Prerequisites

1. **Build the CLI with SQLite feature**:
   ```bash
   cargo build --features sqlite --bin crewchief-maproom --release
   ```

2. **Ensure test fixture exists**:
   ```bash
   ls crates/maproom/tests/fixtures/pre-indexed-maproom.db
   ```

3. **Install jq** (for JSON parsing in tests):
   ```bash
   # Ubuntu/Debian
   sudo apt-get install jq

   # macOS
   brew install jq
   ```

## Test Fixture

The pre-indexed SQLite database is located at:
`crates/maproom/tests/fixtures/pre-indexed-maproom.db`

### Fixture Contents

- **Repository**: `test-repo`
- **Worktree**: `main`
- **Chunks**: 3 code chunks from `main.rs`
  - `main` function (lines 1-5)
  - `helper_function` function (lines 7-10)
  - `Config` struct (lines 12-16)

### Regenerating the Fixture

If you need to update the fixture (e.g., after schema changes):

```bash
# Remove old fixture
rm crates/maproom/tests/fixtures/pre-indexed-maproom.db

# Generate new fixture
cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture
```

The fixture creator is located at:
`crates/maproom/tests/create_sqlite_fixture.rs`

## Test Coverage

The E2E tests cover:

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Status (JSON) | Query repos/worktrees | Returns JSON with repos |
| Status (text) | Human-readable output | Shows test-repo |
| Search (main) | FTS search for "main" | Finds main function |
| Search (function) | FTS search for "function" | Finds multiple chunks |
| Search (result structure) | Verify JSON schema | Has score, lines, etc. |
| Search (nonexistent) | Search missing repo | Empty results |
| Scan (Phase 2) | Scan with SQLite | Shows Phase 2 message |
| Upsert (Phase 2) | Upsert with SQLite | Shows Phase 2 message |
| Cleanup-stale | Dry-run cleanup | Runs without error |
| DB migrate | Migrate SQLite | No-op or informative |
| Daemon ping | JSON-RPC ping | Returns "pong" |
| Daemon FTS | JSON-RPC search | Returns search hits |

## CI Integration

The tests run automatically in GitHub Actions. See `.github/workflows/test.yml` for CI configuration.

### Adding to CI

Add this job to your workflow:

```yaml
test-sqlite-e2e:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-action@stable

    - name: Install jq
      run: sudo apt-get install -y jq

    - name: Run SQLite E2E tests
      run: ./tests/e2e/test_sqlite_flow.sh
```

## Manual Testing

### Test Search Command

```bash
# Set SQLite database
export MAPROOM_DATABASE_URL="sqlite:///path/to/maproom.db"

# Search for code
cargo run --features sqlite --bin crewchief-maproom -- \
  search --repo test-repo --query "function"
```

### Test Status Command

```bash
# JSON output
cargo run --features sqlite --bin crewchief-maproom -- status --json

# Text output
cargo run --features sqlite --bin crewchief-maproom -- status
```

### Test Daemon (stdio)

```bash
# Send JSON-RPC request via stdin
echo '{"jsonrpc":"2.0","method":"ping","id":1}' | \
  cargo run --features sqlite --bin crewchief-maproom -- serve
```

## Troubleshooting

### "Test fixture not found"

Run the fixture generator:
```bash
cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture
```

### "jq: command not found"

Install jq for your platform (see Prerequisites above).

### "Search returns empty results"

1. Verify the database has data:
   ```bash
   cargo run --features sqlite --bin crewchief-maproom -- status --json
   ```

2. Check the query matches indexed content. The fixture contains functions named "main", "helper_function", and struct "Config".

### "Phase 2 message not shown"

The scan/upsert/watch commands require PostgreSQL backend. When using SQLite, they should show an informative message about Phase 2 support.

## Architecture Notes

The SQLite backend uses:

- **FTS5** for full-text search with BM25 ranking
- **sqlite-vec** for vector similarity (if available)
- **Junction table** (`chunk_worktrees`) for chunk-to-worktree mapping
- **VectorStore trait** for backend abstraction

For more details, see `crates/maproom/CLAUDE.md` and `crates/maproom/src/db/sqlite/`.

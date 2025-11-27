# Quality Strategy: Indexer VectorStore Abstraction

> **Note:** This document will be updated during Phase 4 (Testing). Current content
> references the deprecated dual-backend approach. The project is now **SQLite-only** -
> all PostgreSQL references below are legacy and will be removed.

## 1. Testing Philosophy

This project refactors internal code paths without changing external behavior. The testing strategy focuses on:

1. **No regressions** - PostgreSQL path must work identically
2. **New capability** - SQLite path must achieve feature parity
3. **Integration confidence** - End-to-end tests prove the system works

## 2. Test Categories

### 2.1 Unit Tests

**Purpose**: Verify individual function behavior with mocked dependencies.

**Scope**:
- Indexer helper functions (parsing, chunk creation)
- Backend type detection logic
- Parallel/sequential decision logic

**Example**:
```rust
#[test]
fn test_parallel_flag_ignored_for_sqlite() {
    let backend = BackendType::SQLite;
    let parallel_requested = true;
    let effective_parallel = should_use_parallel(parallel_requested, backend);
    assert!(!effective_parallel, "SQLite should not use parallel scan");
}

#[test]
fn test_parallel_flag_honored_for_postgres() {
    let backend = BackendType::PostgreSQL;
    let parallel_requested = true;
    let effective_parallel = should_use_parallel(parallel_requested, backend);
    assert!(effective_parallel, "PostgreSQL should use parallel scan when requested");
}
```

### 2.2 Integration Tests

**Purpose**: Verify indexer works with real database backends.

**Test Matrix**:

| Test Case | PostgreSQL | SQLite |
|-----------|------------|--------|
| Scan empty directory | ✓ | ✓ |
| Scan single file | ✓ | ✓ |
| Scan mixed languages | ✓ | ✓ |
| Incremental upsert | ✓ | ✓ |
| Chunk deduplication | ✓ | ✓ |
| Edge creation (imports) | ✓ | ✓ |
| Index state tracking | ✓ | ✓ |

**SQLite Integration Tests** (`tests/sqlite_indexer.rs`):
```rust
#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_scan_worktree_sqlite() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create test repo with sample files
    let repo_dir = create_test_repo(&temp_dir);

    // Initialize store
    let store = SqliteStore::connect(&db_path.to_string_lossy()).await.unwrap();
    store.migrate().await.unwrap();

    // Run scan
    let stats = indexer::scan_worktree(
        Arc::new(store),
        false, // no parallel for SQLite
        "test-repo",
        "main",
        &repo_dir,
        "abc123",
        None, None, None, // defaults
        &progress,
    ).await.unwrap();

    assert!(stats.files_indexed > 0);
    assert!(stats.chunks_created > 0);
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_upsert_files_sqlite() {
    // Similar structure, tests incremental update
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_watch_sqlite_file_changes() {
    // Tests file watching with SQLite backend
}
```

### 2.3 Regression Tests

**Purpose**: Ensure PostgreSQL path unchanged.

**Approach**: Run existing test suite without modification.

```bash
# Must pass unchanged:
cargo test
cargo test --features sqlite  # Includes new SQLite tests
```

**Critical PostgreSQL Tests**:
- `test_scan_worktree` (existing)
- `test_scan_worktree_parallel` (existing)
- `test_upsert_files` (existing)
- `test_incremental_scan` (existing)

### 2.4 E2E Tests

**Purpose**: Prove real-world usage works.

**Test Script** (`scripts/test_sqlite_indexing.sh`):
```bash
#!/bin/bash
set -e

# Setup
TEMP_DB=$(mktemp /tmp/maproom_test_XXXXXX.db)
TEMP_REPO=$(mktemp -d)

# Create sample repo
mkdir -p "$TEMP_REPO/src"
cat > "$TEMP_REPO/src/main.rs" << 'EOF'
fn main() {
    println!("Hello, world!");
}
EOF

# Test scan
echo "Testing scan with SQLite..."
MAPROOM_DATABASE_URL="sqlite://$TEMP_DB" \
cargo run --features sqlite --bin crewchief-maproom -- \
    scan --path "$TEMP_REPO"

# Verify data exists
echo "Verifying indexed data..."
MAPROOM_DATABASE_URL="sqlite://$TEMP_DB" \
cargo run --features sqlite --bin crewchief-maproom -- \
    status

# Test search
echo "Testing search..."
MAPROOM_DATABASE_URL="sqlite://$TEMP_DB" \
cargo run --features sqlite --bin crewchief-maproom -- \
    search --query "main" --repo "$(basename $TEMP_REPO)"

# Cleanup
rm -rf "$TEMP_DB" "$TEMP_REPO"

echo "All E2E tests passed!"
```

## 3. Test Fixtures

### Sample Repository Structure

```
test-fixtures/
├── rust-sample/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   └── utils/
│   │       └── helpers.rs
│   └── Cargo.toml
├── python-sample/
│   ├── main.py
│   ├── utils.py
│   └── __init__.py
└── mixed-sample/
    ├── index.ts
    ├── main.go
    └── script.py
```

### Test Data Characteristics

- **Small**: 5-10 files, fast CI execution
- **Multi-language**: Rust, Python, TypeScript, Go
- **Realistic**: Functions, classes, imports
- **Deterministic**: Fixed content for reproducible tests

## 4. CI Integration

### GitHub Actions Workflow

```yaml
# In .github/workflows/test.yml

jobs:
  test-sqlite-indexer:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Run SQLite indexer tests
        run: |
          cargo test --features sqlite indexer
          cargo test --features sqlite --test sqlite_indexer
        working-directory: crates/maproom

      - name: Run E2E SQLite test
        run: ./scripts/test_sqlite_indexing.sh
```

### Test Timing Budget

| Test Category | Target Time | Max Time |
|---------------|-------------|----------|
| Unit tests | 5s | 15s |
| Integration tests | 30s | 60s |
| E2E tests | 60s | 120s |
| **Total** | **95s** | **195s** |

## 5. Coverage Requirements

### Critical Paths (Must Test)

1. **scan_worktree with SQLite** - Core indexing
2. **upsert_files with SQLite** - Incremental updates
3. **Chunk deduplication** - blob_sha based
4. **Index state tracking** - tree SHA optimization
5. **Edge creation** - import/call relationships

### Coverage Metrics

- **Line coverage**: Not enforced (pragmatic testing)
- **Critical path coverage**: 100% of listed paths
- **Regression coverage**: All existing tests pass

## 6. Manual Verification

### Developer Checklist

Before marking complete, manually verify:

```bash
# 1. Fresh SQLite scan
rm -f ~/.maproom/maproom.db
cargo run --features sqlite --bin crewchief-maproom -- scan --path .

# 2. Check status shows indexed data
cargo run --features sqlite --bin crewchief-maproom -- status

# 3. Search returns results
cargo run --features sqlite --bin crewchief-maproom -- search --query "function"

# 4. Incremental update works
touch src/main.rs
cargo run --features sqlite --bin crewchief-maproom -- upsert --paths src/main.rs

# 5. PostgreSQL still works (no regression)
MAPROOM_DATABASE_URL="postgresql://..." cargo run --bin crewchief-maproom -- scan --path .
```

## 7. Error Scenarios

### Tests for Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| SQLite DB locked | Retry with backoff, then error |
| Invalid file path | Skip file, continue indexing |
| Unsupported language | Skip file, log warning |
| Empty repository | Return stats with 0 files |
| Missing vec extension | Graceful degradation (FTS only) |

### Error Message Quality

All error messages should be actionable:

```
✗ Bad: "Error: database locked"
✓ Good: "Error: SQLite database is locked. Another process may be writing.
         If using watch mode, ensure only one instance is running."
```

## 8. Performance Validation

### Benchmark Comparisons

```bash
# Compare scan performance
hyperfine \
  'MAPROOM_DATABASE_URL="postgresql://..." cargo run -r -- scan --path ./test-fixtures' \
  'MAPROOM_DATABASE_URL="sqlite:///tmp/bench.db" cargo run -r --features sqlite -- scan --path ./test-fixtures'
```

### Acceptable Performance

| Operation | PostgreSQL | SQLite | Acceptable Ratio |
|-----------|------------|--------|------------------|
| Scan 100 files | 5s | 7.5s | 1.5x |
| Scan 1000 files | 30s | 60s | 2x |
| Upsert 10 files | 0.5s | 1s | 2x |

SQLite being 1.5-2x slower is acceptable for the zero-config benefit.

## 9. Quality Gates

### Before Merge

1. ✓ All existing tests pass (`cargo test`)
2. ✓ All new SQLite tests pass (`cargo test --features sqlite`)
3. ✓ E2E script passes
4. ✓ No clippy warnings (`cargo clippy --features sqlite`)
5. ✓ Manual verification complete

### Definition of Done

- [ ] `scan` command works with SQLite
- [ ] `upsert` command works with SQLite
- [ ] `watch` command works with SQLite
- [ ] PostgreSQL path unchanged (regression tests pass)
- [ ] Documentation updated
- [ ] E2E tests in CI

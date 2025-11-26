# Quality Strategy: MAPCLI - Maproom CLI Abstraction

## Testing Philosophy

This project modifies existing, working code to use an abstraction layer. The primary risk is regression - breaking existing PostgreSQL functionality while adding SQLite support. Testing focuses on:

1. **Behavior Preservation**: Existing commands produce identical results
2. **Backend Parity**: SQLite produces equivalent results to PostgreSQL (for MVP commands)
3. **Error Handling**: Graceful degradation when features are unavailable

**MVP Scope Note**: Phase 1 tests only MVP commands (search, status, daemon, cleanup). Scan/upsert/watch testing deferred to Phase 2.

## Test Categories

### 1. Unit Tests

**Purpose**: Verify individual functions work correctly with mocked dependencies

**Location**: `crates/maproom/src/main.rs` (inline tests) and `crates/maproom/tests/`

**Key Tests**:

```rust
#[cfg(test)]
mod tests {
    // Test BackendType trait method (MAPCLI-1000)
    #[tokio::test]
    async fn test_postgres_store_backend_type() {
        let store = PostgresStore::connect().await.unwrap();
        assert_eq!(store.backend_type(), BackendType::PostgreSQL);
    }

    #[tokio::test]
    #[cfg(feature = "sqlite")]
    async fn test_sqlite_store_backend_type() {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        assert_eq!(store.backend_type(), BackendType::SQLite);
    }

    // Test backend detection from URL
    #[test]
    fn test_backend_type_from_postgres_url() {
        assert_eq!(
            detect_backend("postgresql://localhost/db"),
            BackendType::PostgreSQL
        );
    }

    #[test]
    fn test_backend_type_from_sqlite_url() {
        assert_eq!(
            detect_backend("sqlite:///path/to/db.sqlite"),
            BackendType::SQLite
        );
    }

    // Test CLI argument parsing (existing)
    #[test]
    fn test_scan_defaults() {
        let cli = Cli::parse_from(&["maproom", "scan"]);
        // Verify defaults...
    }
}
```

### 2. Integration Tests

**Purpose**: Verify command handlers work with real database backends

**Location**: `crates/maproom/tests/cli_integration.rs`

**Key Tests**:

```rust
#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_scan_with_sqlite_backend() {
    // Create temp SQLite database
    let db_path = tempfile::NamedTempFile::new().unwrap();
    std::env::set_var("MAPROOM_DATABASE_URL", format!("sqlite://{}", db_path.path().display()));

    // Get store and verify it's SQLite
    let store = db::factory::get_store().await.unwrap();
    assert_eq!(store.backend_type(), BackendType::SQLite);

    // Run scan equivalent operations
    // ...
}

#[tokio::test]
async fn test_search_returns_results() {
    // Setup store with test data
    // Execute search
    // Verify results match expected format
}

#[tokio::test]
async fn test_daemon_search_handler() {
    // Create mock request
    let request = JsonRpcRequest {
        method: "search".to_string(),
        params: Some(json!({"repo": "test", "query": "function"})),
        id: Some(json!(1)),
        jsonrpc: "2.0".to_string(),
    };

    // Execute with test store
    let response = handle_request(request, state).await;

    // Verify response structure
    assert!(response.result.is_some());
}
```

### 3. Contract Tests (From VECSTORE)

**Purpose**: Verify both backends implement VectorStore correctly

**Location**: `crates/maproom/tests/vectorstore_contract.rs`

**Already Implemented**: 18 tests covering all trait methods

These tests ensure the abstraction layer the CLI depends on is correct.

### 4. End-to-End Tests

**Purpose**: Verify full CLI commands work from command line

**Approach**: Shell scripts or Rust test binaries that spawn the CLI

**MVP Testing Strategy**: Since scan is deferred to Phase 2, E2E tests use a **pre-indexed SQLite database fixture**.

**Creating Pre-Indexed Test Fixture**:
```bash
# One-time fixture creation (using PostgreSQL backend to populate)
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost/maproom"
cargo run --bin crewchief-maproom -- scan --path ./test-fixtures/sample-repo
# Then export/convert to SQLite fixture (separate tooling)
```

**Example E2E Test**:
```bash
#!/bin/bash
# tests/e2e/test_sqlite_flow.sh

# Use pre-indexed fixture (scan NOT tested - deferred to Phase 2)
cp ./tests/fixtures/pre-indexed-maproom.db /tmp/test_maproom.db
export MAPROOM_DATABASE_URL="sqlite:///tmp/test_maproom.db"

# Verify search returns results from pre-indexed data
RESULTS=$(cargo run --features sqlite --bin crewchief-maproom -- search --repo sample-repo --query "function")
echo "$RESULTS" | jq -e '.hits | length > 0' || { echo "Search failed"; exit 1; }

# Verify status shows indexed data
STATUS=$(cargo run --features sqlite --bin crewchief-maproom -- status --repo sample-repo --json)
echo "$STATUS" | jq -e '.repos | length > 0' || { echo "Status failed"; exit 1; }

# Verify graceful error for scan with SQLite (Phase 2 feature)
if cargo run --features sqlite --bin crewchief-maproom -- scan --path ./test-fixtures/sample-repo 2>&1 | grep -q "Phase 2"; then
    echo "Scan correctly shows Phase 2 message"
else
    echo "Scan should show Phase 2 deferral message"; exit 1
fi

echo "E2E tests passed!"
```

## Test Matrix

### Phase 1 (MVP) Tests

| Command | Unit Test | Integration Test | E2E Test | SQLite |
|---------|-----------|------------------|----------|--------|
| `search` | Query parsing | Trait method call | JSON output | ✅ MVP |
| `vector-search` | Query parsing | Trait method call | JSON output | ✅ MVP |
| `status` | Args validation | Status retrieval | Text output | ✅ MVP |
| `serve` | Request parsing | Handler dispatch | JSON-RPC flow | ✅ MVP |
| `db migrate` | Backend detection | Skip for SQLite | N/A | ✅ MVP |
| `db cleanup-stale` | Args parsing | Cleanup ops | Dry-run mode | ✅ MVP |

### Phase 2 (Deferred) Tests

| Command | Unit Test | Integration Test | E2E Test | SQLite |
|---------|-----------|------------------|----------|--------|
| `scan` | CLI parsing | Store operations | Full flow | ⏸️ Phase 2 |
| `upsert` | CLI parsing | Store operations | Full flow | ⏸️ Phase 2 |
| `watch` | CLI parsing | Store operations | Full flow | ⏸️ Phase 2 |
| `generate-embeddings` | CLI parsing | Embedding ops | Full flow | ⏸️ Phase 2 |

## Critical Paths

### Path 1: PostgreSQL Regression Prevention

**Risk**: Breaking existing functionality for PostgreSQL users

**Tests**:
1. All existing tests continue to pass without `--features sqlite`
2. PostgreSQL-specific commands (migrate) still work
3. Parallel scan mode still works with PgPool

**Verification**:
```bash
# Must pass without sqlite feature
cargo test
cargo test --test vectorstore_contract  # Requires PostgreSQL service
```

### Path 2: SQLite Backend Functionality (MVP)

**Risk**: SQLite backend doesn't work correctly through CLI

**Tests** (using pre-indexed database):
1. ~~Scan creates database and indexes files~~ (deferred to Phase 2)
2. Search returns results from pre-indexed data
3. Status shows correct counts using trait methods
4. Daemon responds to JSON-RPC calls (all search modes)
5. `status.rs` no longer creates its own PostgreSQL connection

**Verification**:
```bash
# Must pass with sqlite feature
cargo test --features sqlite
cargo test --features sqlite --test vectorstore_contract

# E2E with pre-indexed fixture
./tests/e2e/test_sqlite_flow.sh
```

### Path 3: Backend Detection

**Risk**: Wrong backend selected based on URL

**Tests**:
1. `postgresql://` URLs select PostgreSQL
2. `sqlite://` URLs select SQLite
3. File paths select SQLite
4. Missing URL uses appropriate default

## Mocking Strategy

### VectorStore Mock

For unit testing command handlers without real databases:

```rust
#[cfg(test)]
mod tests {
    use mockall::mock;

    mock! {
        VectorStore {}

        #[async_trait]
        impl VectorStore for VectorStore {
            async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<Repo>>;
            async fn search_chunks_fts(...) -> anyhow::Result<Vec<SearchHit>>;
            // ... other methods
        }
    }

    #[tokio::test]
    async fn test_search_handler_uses_trait() {
        let mut mock = MockVectorStore::new();
        mock.expect_get_repo_by_name()
            .with(eq("test-repo"))
            .returning(|_| Ok(Some(Repo { id: 1, name: "test-repo".into(), ... })));

        // Test handler with mock
    }
}
```

### Embedded SQLite for Tests

For integration tests, use in-memory SQLite:

```rust
async fn create_test_store() -> Arc<dyn VectorStore> {
    let store = SqliteStore::connect(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}
```

## Test Data Fixtures

**Location**: `crates/maproom/tests/fixtures/`

**Contents**:
- `sample-repo/` - Minimal git repository with various file types
- `test-data.sql` - PostgreSQL test data insertion
- `test-chunks.json` - Sample chunk data for mocking

## CI Integration

### GitHub Actions Workflow

```yaml
test-cli-postgres:
  services:
    postgres:
      image: ankane/pgvector:latest
      env:
        POSTGRES_USER: maproom
        POSTGRES_PASSWORD: maproom
        POSTGRES_DB: maproom_test
  steps:
    - run: cargo test --bin crewchief-maproom
    - run: cargo test --test cli_integration

test-cli-sqlite:
  steps:
    - run: cargo test --features sqlite --bin crewchief-maproom
    - run: cargo test --features sqlite --test cli_integration
```

## Acceptance Criteria Verification

Each ticket's acceptance criteria maps to specific tests:

| Acceptance Criteria | Test Type | Test Name | MVP |
|---------------------|-----------|-----------|-----|
| `backend_type()` returns correct enum | Unit | `test_postgres/sqlite_store_backend_type` | ✅ |
| `get_store()` returns working store | Integration | `test_factory_returns_store` | ✅ |
| Daemon serves JSON-RPC (fts) with SQLite | E2E | `test_daemon_sqlite_fts` | ✅ |
| Daemon serves JSON-RPC (vector) with SQLite | E2E | `test_daemon_sqlite_vector` | ✅ |
| Daemon serves JSON-RPC (hybrid) with SQLite | E2E | `test_daemon_sqlite_hybrid` | ✅ |
| Search returns results from SQLite | Integration | `test_search_returns_results` | ✅ |
| Status works with SQLite | Integration | `test_status_with_sqlite` | ✅ |
| `status.rs` uses trait methods (no direct PG) | Code review | Manual verification | ✅ |
| `scan --sqlite` works | E2E | ~~`test_scan_with_sqlite_flag`~~ | ⏸️ Phase 2 |
| PostgreSQL unchanged | Regression | Existing test suite | ✅ |

## Test Commands Summary

```bash
# Quick validation (unit tests only)
cargo test --lib

# Full PostgreSQL testing
cargo test

# Full SQLite testing
cargo test --features sqlite

# CLI-specific tests
cargo test --bin crewchief-maproom
cargo test --test cli_integration --features sqlite

# E2E tests (requires built binary)
./tests/e2e/test_sqlite_flow.sh
```

## Definition of Done

A ticket is complete when:

1. ✅ Implementation compiles without warnings
2. ✅ All existing tests pass (`cargo test`)
3. ✅ SQLite tests pass (`cargo test --features sqlite`)
4. ✅ No clippy warnings (`cargo clippy`)
5. ✅ Manual testing confirms expected behavior
6. ✅ Acceptance criteria from ticket are met

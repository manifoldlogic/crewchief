# Quality Strategy: SQLite Backend Fixes

## 1. Test Strategy

### 1.1 Critical Paths

| Path | Risk | Test Type |
|------|------|-----------|
| SQLite module compiles | High | CI cargo check |
| Basic CRUD operations | High | Unit tests |
| FTS search returns results | High | Unit test |
| FTS5 query syntax valid | High | Unit test |
| Connection pooling/threading | Medium | Concurrent test |
| Vector search (deferred) | Low | Future |

### 1.2 Test Pyramid

```
         ╱╲
        ╱  ╲     E2E (VSCode extension)
       ╱────╲    - Deferred to separate project
      ╱      ╲
     ╱ Integ  ╲  Integration Tests
    ╱──────────╲ - Future: backend parity tests
   ╱            ╲
  ╱    Unit      ╲ Unit Tests
 ╱────────────────╲ - SqliteStore methods
╱__________________╲ - Schema creation
                     - FTS5 query syntax
```

## 2. Unit Tests

### 2.1 `tests/sqlite_store.rs`

```rust
#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use crewchief_maproom::db::sqlite::SqliteStore;

    #[tokio::test]
    async fn test_connect() {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_repo() {
        let store = setup_store().await;
        let id = store.get_or_create_repo("test", "/path").await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_idempotent_repo_creation() {
        let store = setup_store().await;
        let id1 = store.get_or_create_repo("test", "/path").await.unwrap();
        let id2 = store.get_or_create_repo("test", "/path").await.unwrap();
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn test_fts_query_syntax() {
        // Verify FTS5 queries don't produce syntax errors
        let store = setup_indexed_store().await;
        // Should not panic with "fts5: syntax error"
        let results = store.search_chunks_fts("repo", None, "test query", 10, false).await;
        assert!(results.is_ok());
    }
}
```

### 2.2 Coverage Targets

| Module | Target | Notes |
|--------|--------|-------|
| `sqlite/mod.rs` | 80% | Core CRUD paths |
| `sqlite/schema.rs` | 100% | Schema creation |
| `factory.rs` | 60% | Feature gating |

## 3. FTS5 Validation Tests

### 3.1 Query Syntax Validation

```rust
#[tokio::test]
async fn test_fts5_prefix_search() {
    let store = setup_store().await;

    // Index known content
    index_test_chunk(&store, "test_function", "fn test_function() {}").await;

    // Prefix search should work
    let results = store.search_chunks_fts("repo", None, "test", 10, false).await.unwrap();
    assert!(!results.is_empty(), "FTS prefix search should return results");
}

#[tokio::test]
async fn test_fts5_multiword_query() {
    let store = setup_store().await;

    // Index known content
    index_test_chunk(&store, "authenticate_user", "fn authenticate_user() {}").await;

    // Multi-word query should work (OR semantics)
    let results = store.search_chunks_fts("repo", None, "authenticate user", 10, false).await.unwrap();
    assert!(!results.is_empty(), "FTS multi-word search should return results");
}
```

## 4. CI Integration

### 4.1 GitHub Actions Matrix

```yaml
# .github/workflows/test.yml
jobs:
  test:
    strategy:
      matrix:
        features: [postgres, sqlite]
    steps:
      - run: cargo test --features ${{ matrix.features }}
```

### 4.2 Build Verification

```yaml
- name: Check SQLite feature
  run: cargo check --features sqlite

- name: Check Postgres feature (default)
  run: cargo check
```

## 5. Acceptance Criteria

### 5.1 Build Requirements (Must Pass)
- [ ] `cargo check --features sqlite` passes
- [ ] `cargo check --features postgres` passes (no regression)
- [ ] `cargo check` passes (default features)

### 5.2 Test Requirements (Must Pass)
- [ ] All SQLite unit tests pass
- [ ] FTS search returns valid results (no syntax errors)
- [ ] No `SQLITE_BUSY` errors in concurrent tests
- [ ] FTS5 query syntax validates against SQLite documentation

### 5.3 Performance Goals (Aspirational, Not Blocking)

These are future optimization targets, not acceptance criteria for this fix project:
- Index 100 files in < 5 seconds
- FTS search p95 < 100ms
- Binary size increase < 2MB

**Note**: Performance requirements are deferred. This project focuses on correctness.

## 6. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| SQLite lock contention | WAL mode + busy_timeout PRAGMA |
| FTS5 syntax errors | Explicit syntax validation tests |
| CI failures | Feature matrix ensures both backends tested |
| Regression in Postgres | Default feature unchanged, explicit tests |
| Schema mismatches | Test validates all columns exist |

## 7. Definition of Done

A ticket is complete when:
1. Code compiles without errors or warnings
2. All unit tests pass
3. CI workflow passes (after CI ticket is done)
4. No regressions in postgres feature
5. Code follows existing patterns (error handling, async)

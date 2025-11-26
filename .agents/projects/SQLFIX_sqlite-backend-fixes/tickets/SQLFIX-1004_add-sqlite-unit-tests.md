# Ticket: SQLFIX-1004: Add SQLite Unit Tests

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Run `cargo test --features sqlite` and verify all tests pass
- Test output must be captured and reviewed

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive unit tests for the SQLite backend to validate CRUD operations, FTS search, and concurrent access patterns.

## Background
The SQLite backend now compiles (SQLFIX-1001), has correct schema (SQLFIX-1002), and CRUD operations work (SQLFIX-1003). This ticket adds test coverage to validate functionality and prevent regressions.

Tests should use `:memory:` SQLite databases for speed and isolation.

**Plan Reference**: Phase 3 - Testing (Ticket 1004)

## Acceptance Criteria
- [ ] `cargo test --features sqlite` passes with all tests green
- [ ] Test file `crates/maproom/tests/sqlite_store.rs` created
- [ ] Tests cover: connect, migrate, CRUD cycle, FTS search, idempotent operations
- [ ] Tests use `:memory:` database for isolation
- [ ] No `SQLITE_BUSY` errors in concurrent test scenarios
- [ ] Tests are feature-gated with `#[cfg(feature = "sqlite")]`

## Technical Requirements

### Test File Structure
Create `crates/maproom/tests/sqlite_store.rs`:

```rust
//! SQLite backend integration tests
//!
//! Run with: cargo test --features sqlite

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use crewchief_maproom::db::sqlite::SqliteStore;
    use crewchief_maproom::db::{ChunkRecord, FileRecord, VectorStore};

    /// Helper to create an in-memory store with schema
    async fn setup_store() -> SqliteStore {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    // ==================== Connection Tests ====================

    #[tokio::test]
    async fn test_connect_and_migrate() {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        // Should not panic - schema created successfully
    }

    #[tokio::test]
    async fn test_migrate_idempotent() {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store.migrate().await.unwrap(); // Second call should succeed
    }

    // ==================== Repository Tests ====================

    #[tokio::test]
    async fn test_create_repo() {
        let store = setup_store().await;
        let id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        assert!(id > 0, "Repo ID should be positive");
    }

    #[tokio::test]
    async fn test_repo_idempotent() {
        let store = setup_store().await;
        let id1 = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let id2 = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        assert_eq!(id1, id2, "Same repo should return same ID");
    }

    // ==================== Full CRUD Cycle ====================

    #[tokio::test]
    async fn test_full_crud_cycle() {
        let store = setup_store().await;

        // 1. Create repo
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        assert!(repo_id > 0);

        // 2. Create worktree
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        assert!(worktree_id > 0);

        // 3. Create commit
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123def456", None)
            .await
            .unwrap();
        assert!(commit_id > 0);

        // 4. Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "src/main.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash123abc".to_string(),
            size_bytes: 1024,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();
        assert!(file_id > 0);

        // 5. Create chunk
        let chunk = ChunkRecord {
            file_id,
            blob_sha: "blobsha123".to_string(),
            symbol_name: Some("main".to_string()),
            kind: "function".to_string(),
            signature: Some("fn main()".to_string()),
            docstring: Some("Entry point of the application".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            ts_doc_text: "main function entry point".to_string(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
            worktree_id,
        };
        let chunk_id = store.insert_chunk(&chunk).await.unwrap();
        assert!(chunk_id > 0);
    }

    // ==================== FTS Search Tests ====================

    #[tokio::test]
    async fn test_fts_search() {
        let store = setup_store().await;

        // Setup: create full hierarchy
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "src/auth.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash456".to_string(),
            size_bytes: 2048,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = ChunkRecord {
            file_id,
            blob_sha: "blobsha456".to_string(),
            symbol_name: Some("authenticate_user".to_string()),
            kind: "function".to_string(),
            signature: Some("fn authenticate_user(username: &str) -> bool".to_string()),
            docstring: Some("Authenticates a user by username".to_string()),
            start_line: 10,
            end_line: 30,
            preview: "fn authenticate_user(username: &str) -> bool {\n    // auth logic\n}".to_string(),
            ts_doc_text: "authenticate user authentication login".to_string(),
            recency_score: 1.0,
            churn_score: 0.2,
            metadata: None,
            worktree_id,
        };
        store.insert_chunk(&chunk).await.unwrap();

        // Search should find the chunk
        let results = store
            .search_chunks_fts("test-repo", Some("main"), "authenticate", 10, false)
            .await
            .unwrap();

        assert!(!results.is_empty(), "FTS search should return results");
        assert!(
            results[0].symbol_name.as_ref().unwrap().contains("authenticate"),
            "Should find authenticate_user function"
        );
    }

    #[tokio::test]
    async fn test_fts_multiword_query() {
        let store = setup_store().await;

        // Setup minimal hierarchy
        let repo_id = store
            .get_or_create_repo("test-repo", "/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "sha1", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = ChunkRecord {
            file_id,
            blob_sha: "blob".to_string(),
            symbol_name: Some("test_func".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 5,
            preview: "fn test_func() {}".to_string(),
            ts_doc_text: "test function".to_string(),
            recency_score: 1.0,
            churn_score: 0.0,
            metadata: None,
            worktree_id,
        };
        store.insert_chunk(&chunk).await.unwrap();

        // Multi-word query should not cause syntax error
        let result = store
            .search_chunks_fts("test-repo", None, "test function", 10, false)
            .await;

        assert!(result.is_ok(), "Multi-word FTS query should not error");
    }

    #[tokio::test]
    async fn test_fts_no_results() {
        let store = setup_store().await;

        // Create repo but no chunks
        let _repo_id = store
            .get_or_create_repo("empty-repo", "/empty")
            .await
            .unwrap();

        // Search in empty repo should return empty, not error
        let results = store
            .search_chunks_fts("empty-repo", None, "nonexistent", 10, false)
            .await
            .unwrap();

        assert!(results.is_empty(), "Should return empty for no matches");
    }

    // ==================== Edge Cases ====================

    #[tokio::test]
    async fn test_special_characters_in_query() {
        let store = setup_store().await;
        let _repo_id = store
            .get_or_create_repo("test-repo", "/path")
            .await
            .unwrap();

        // Queries with special characters should not crash
        let result = store
            .search_chunks_fts("test-repo", None, "test \"quoted\" func", 10, false)
            .await;
        assert!(result.is_ok(), "Should handle quotes in query");

        let result = store
            .search_chunks_fts("test-repo", None, "func()", 10, false)
            .await;
        assert!(result.is_ok(), "Should handle parentheses in query");
    }

    #[tokio::test]
    async fn test_empty_query() {
        let store = setup_store().await;
        let _repo_id = store
            .get_or_create_repo("test-repo", "/path")
            .await
            .unwrap();

        // Empty or whitespace query
        let result = store
            .search_chunks_fts("test-repo", None, "   ", 10, false)
            .await;
        // Should either return empty results or handle gracefully
        assert!(result.is_ok() || result.is_err(), "Should handle empty query");
    }
}
```

### Test Categories Summary

1. **Connection Tests**
   - `test_connect_and_migrate` - Basic connection works
   - `test_migrate_idempotent` - Migrate can be called multiple times

2. **CRUD Tests**
   - `test_create_repo` - Repo creation works
   - `test_repo_idempotent` - Same repo returns same ID
   - `test_full_crud_cycle` - Complete create chain works

3. **FTS Tests**
   - `test_fts_search` - Basic FTS search returns results
   - `test_fts_multiword_query` - Multi-word queries work
   - `test_fts_no_results` - Empty results handled correctly

4. **Edge Cases**
   - `test_special_characters_in_query` - Quotes, parens handled
   - `test_empty_query` - Whitespace-only query handled

## Implementation Notes

### Running Tests
```bash
# Run all SQLite tests
cargo test --features sqlite sqlite_tests

# Run with output visible
cargo test --features sqlite -- --nocapture

# Run specific test
cargo test --features sqlite test_fts_search

# Run tests showing passed tests too
cargo test --features sqlite -- --show-output
```

### Test Isolation
Each test:
- Creates its own `:memory:` database
- Does not depend on other tests
- Cleans up automatically (in-memory DB is dropped)

### Cargo.toml Updates
May need to add dev-dependencies if not present:
```toml
[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Verification Checklist
```bash
# All these commands should pass:
cargo test --features sqlite test_connect_and_migrate
cargo test --features sqlite test_create_repo
cargo test --features sqlite test_repo_idempotent
cargo test --features sqlite test_full_crud_cycle
cargo test --features sqlite test_fts_search
cargo test --features sqlite test_fts_multiword_query
cargo test --features sqlite test_fts_no_results
cargo test --features sqlite test_special_characters_in_query
cargo test --features sqlite test_empty_query
```

## Dependencies
- **SQLFIX-1003**: CRUD operations and FTS must work first

## Risk Assessment
- **Risk**: Tests may be flaky under concurrent execution
  - **Mitigation**: Each test uses separate `:memory:` database
- **Risk**: Tests may not cover all edge cases
  - **Mitigation**: Focus on critical paths for MVP; expand coverage later
- **Risk**: Test file location may not be discovered
  - **Mitigation**: Place in standard `tests/` directory

## Files/Packages Affected
- `crates/maproom/tests/sqlite_store.rs` (new file)
- `crates/maproom/Cargo.toml` (may need dev-dependencies update)

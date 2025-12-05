# Ticket: MULTICN-1001: Enhanced PRAGMA Configuration

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

Update SQLite connection initialization with optimized PRAGMA settings to better handle concurrent access. Increase busy_timeout and configure WAL checkpointing, cache size, and memory-mapped I/O for improved performance under multi-agent load.

## Background

Current maproom daemon uses conservative SQLite settings that weren't tuned for concurrent multi-agent access. Increasing busy_timeout from 5s to 30s and optimizing other PRAGMAs will reduce SQLITE_BUSY errors before implementing the shared daemon architecture.

This is a safe, incremental improvement that benefits both current (per-agent daemon) and future (shared daemon) architectures.

Reference: [architecture.md](../planning/architecture.md) - SQLite Optimizations section

## Acceptance Criteria

- [ ] `busy_timeout = 30000` (30 seconds) configured in connection init
- [ ] `wal_autocheckpoint = 10000` configured (~40MB threshold)
- [ ] `cache_size = -65536` configured (64MB page cache)
- [ ] `mmap_size = 268435456` configured (256MB memory-mapped I/O)
- [ ] All 4 PRAGMA settings applied on every connection in pool
- [ ] Log output shows increased busy_timeout value on daemon startup
- [ ] Concurrent indexing test passes without SQLITE_BUSY errors

## Technical Requirements

Modify SQLite connection initialization in `crates/maproom/src/db/sqlite/mod.rs`.

### Current PRAGMA Configuration

```rust
const SQLITE_PRAGMAS: &str = r#"
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    PRAGMA busy_timeout = 5000;
    PRAGMA foreign_keys = ON;
"#;
```

### Enhanced PRAGMA Configuration

```rust
const SQLITE_PRAGMAS: &str = r#"
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    PRAGMA busy_timeout = 30000;          -- Increased from 5000
    PRAGMA wal_autocheckpoint = 10000;    -- NEW: ~40MB before checkpoint
    PRAGMA cache_size = -65536;           -- NEW: 64MB page cache
    PRAGMA mmap_size = 268435456;         -- NEW: 256MB memory-mapped I/O
    PRAGMA foreign_keys = ON;
"#;
```

### PRAGMA Rationale

| PRAGMA | Value | Purpose |
|--------|-------|---------|
| busy_timeout | 30000ms | Allow more time for lock contention resolution |
| wal_autocheckpoint | 10000 pages | Reduce checkpoint frequency (40MB threshold) |
| cache_size | -65536 KB | 64MB in-memory page cache for faster reads |
| mmap_size | 256MB | Memory-mapped I/O for large database files |

### Logging Enhancement

Add startup log to verify configuration:

```rust
tracing::info!(
    "SQLite PRAGMAs applied: busy_timeout=30s, wal_checkpoint=10000, cache=64MB, mmap=256MB"
);
```

## Implementation Notes

### Finding Connection Initialization Code

Look for:
- Connection pool setup using `r2d2` or similar
- PRAGMA execution in connection customizer
- Likely in `create_pool()` or `setup_connection()` function

### Applying PRAGMAs

Ensure PRAGMAs are applied to every connection in the pool:

```rust
pub fn create_pool(database_path: &Path) -> Result<Pool<SqliteConnectionManager>> {
    let manager = SqliteConnectionManager::file(database_path)
        .with_init(|conn| {
            conn.execute_batch(SQLITE_PRAGMAS)?;
            Ok(())
        });

    let pool = Pool::builder()
        .max_size(10)
        .build(manager)?;

    tracing::info!("SQLite connection pool created with enhanced PRAGMAs");
    Ok(pool)
}
```

### Testing Concurrent Access

Create integration test to verify SQLITE_BUSY handling:

```rust
#[test]
fn test_concurrent_writes_no_sqlite_busy() {
    let db = create_test_database();

    // Spawn 3 threads writing concurrently
    let handles: Vec<_> = (0..3)
        .map(|i| {
            let db = db.clone();
            std::thread::spawn(move || {
                for j in 0..100 {
                    db.execute(&format!("INSERT INTO test VALUES ({})", i * 100 + j))
                        .expect("Should not get SQLITE_BUSY");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all writes succeeded
    let count: i64 = db.query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 300);
}
```

## Dependencies

- None (first implementation ticket in Phase 1)
- MULTICN-0001 should be complete for baseline comparison

## Risk Assessment

- **Risk**: Increased busy_timeout could mask underlying issues
  - **Mitigation**: 30s is reasonable for concurrent workloads. We're addressing root cause (shared daemon) in Phase 2.

- **Risk**: Memory-mapped I/O issues on some filesystems
  - **Mitigation**: 256MB mmap_size is conservative. SQLite handles fallback gracefully.

- **Risk**: Cache size too large for low-memory systems
  - **Mitigation**: 64MB cache is reasonable for modern dev machines. Will be configurable in MULTICN-1002.

## Files/Packages Affected

- `crates/maproom/src/db/sqlite/mod.rs` (MODIFY - connection initialization)
- `crates/maproom/tests/integration/concurrent_writes.rs` (NEW - optional integration test)

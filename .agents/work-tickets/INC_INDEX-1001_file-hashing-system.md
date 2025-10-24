# Ticket: INC_INDEX-1001: File Hashing System

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement blake3-based file content hashing system to enable fast, accurate change detection for incremental indexing. This is the foundation for the incremental indexing pipeline, providing the change detection mechanism that determines which files need reindexing.

## Background
The current maproom indexer performs full scans on every index operation, which is inefficient for large codebases where only a few files change between scans. To enable incremental indexing, we need a robust change detection mechanism based on content hashing. This system will:

1. Hash file contents using blake3 for fast, collision-resistant hashing
2. Maintain an in-memory hash cache for recently processed files
3. Store content hashes in the database for persistence across runs
4. Provide comparison logic to detect actual file changes vs. metadata-only changes (like timestamp updates)

This is Phase 1, Week 1, Task 1 from the INC_INDEX implementation plan and is a prerequisite for all other incremental indexing features.

## Acceptance Criteria
- [ ] Hash generation completes in <10ms per file (blake3 performance target)
- [ ] Accurate change detection: only files with actual content changes are flagged
- [ ] Hash cache working: in-memory HashMap<PathBuf, ContentHash> stores recent hashes
- [ ] Database integration complete: content_hash column added and populated in files table
- [ ] Unit tests cover: hash generation, cache hit/miss, database storage/retrieval
- [ ] Integration tests verify: change detection across cache → database → filesystem

## Technical Requirements
- Use blake3 crate for content hashing (fast, cryptographically secure)
- Implement ContentHash type as wrapper around blake3::Hash
- Create hash cache: HashMap<PathBuf, ContentHash> with basic eviction policy
- Add content_hash column (BYTEA type) to maproom.files table via migration
- Implement three-tier comparison logic:
  1. Check in-memory cache first (fastest)
  2. Check database if cache miss
  3. Compare against filesystem hash
- Return ChangeType enum: None, New(hash), Modified(old_hash, new_hash)
- Target performance: <10ms hash generation per file

## Implementation Notes

### Hash Module (`crates/maproom/src/incremental/hash.rs`)
```rust
use blake3;
use std::path::Path;

pub type ContentHash = blake3::Hash;

pub struct FileHasher;

impl FileHasher {
    pub fn hash_file(path: &Path) -> Result<ContentHash> {
        let content = std::fs::read(path)?;
        Ok(blake3::hash(&content))
    }
}
```

### Cache Module (`crates/maproom/src/incremental/cache.rs`)
```rust
use std::collections::HashMap;
use std::path::PathBuf;
use super::hash::ContentHash;

pub struct HashCache {
    cache: HashMap<PathBuf, ContentHash>,
}

impl HashCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&ContentHash> {
        self.cache.get(path)
    }

    pub fn insert(&mut self, path: PathBuf, hash: ContentHash) {
        self.cache.insert(path, hash);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
```

### Change Detector Integration (Architecture Reference)
From INC_INDEX_ARCHITECTURE.md lines 49-82:
- Three-tier comparison: cache → database → filesystem
- Return ChangeType based on comparison results
- Update cache after detecting changes

### Database Migration
Add content_hash column to files table:
```sql
ALTER TABLE maproom.files ADD COLUMN content_hash BYTEA;
CREATE INDEX idx_files_content_hash ON maproom.files(content_hash);
```

### Performance Considerations
- blake3 is significantly faster than SHA-256 (multiple GB/s throughput)
- In-memory cache avoids database queries for recently processed files
- Database stores hashes for persistence across restarts
- Index on content_hash enables fast lookups

## Dependencies
- Prerequisite: files table must exist in database (already present)
- Blake3 crate: Add to Cargo.toml dependencies
- Database connection: Use existing sqlx connection pool
- No other work tickets are blockers

## Risk Assessment
- **Risk**: Hash cache grows unbounded in long-running watch mode
  - **Mitigation**: Implement LRU eviction policy in follow-up work (not required for Phase 1)
  - **Mitigation**: Document cache clearing strategy for operators

- **Risk**: Database migration could be slow on large existing databases
  - **Mitigation**: content_hash column is nullable, can backfill incrementally
  - **Mitigation**: Create index CONCURRENTLY to avoid locking

- **Risk**: Blake3 hash collisions (theoretical)
  - **Mitigation**: Blake3 is cryptographically secure with negligible collision probability
  - **Mitigation**: Fallback to full re-parse on unexpected index corruption

## Files/Packages Affected
- **New Files**:
  - `crates/maproom/src/incremental/hash.rs` - Hashing utilities and ContentHash type
  - `crates/maproom/src/incremental/cache.rs` - Hash cache implementation
  - `crates/maproom/src/incremental/mod.rs` - Module declarations
  - `crates/maproom/tests/incremental/hash_test.rs` - Unit tests for hashing
  - `crates/maproom/tests/incremental/cache_test.rs` - Unit tests for cache

- **Modified Files**:
  - `crates/maproom/Cargo.toml` - Add blake3 dependency
  - `crates/maproom/src/lib.rs` - Add incremental module

- **Database Migrations**:
  - `crates/maproom/migrations/XXX_add_content_hash.sql` - Add content_hash column to files table

## Planning References
- Architecture: `/workspace/crewchief_context/maproom/INC_INDEX/INC_INDEX_ARCHITECTURE.md` (lines 49-82: Change Detector)
- Implementation Plan: Phase 1, Week 1, Task 1

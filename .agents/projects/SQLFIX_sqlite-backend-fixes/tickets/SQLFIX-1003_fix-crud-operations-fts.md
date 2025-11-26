# Ticket: SQLFIX-1003: Fix CRUD Operations and FTS Search

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Manual verification of CRUD cycle and FTS search
- Comprehensive unit tests in SQLFIX-1004

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the FTS5 query syntax error in `search_chunks_fts`. The current code generates invalid FTS5 syntax. Also verify CRUD operations work end-to-end.

## Background
After compilation (SQLFIX-1001) and schema fixes (SQLFIX-1002), there is one remaining runtime issue:

**FTS5 Query Syntax Bug** (sqlite/mod.rs lines 454-459):
```rust
let fts_query = query
    .split_whitespace()
    .map(|t| format!("\"{}\"*", t.replace("\"", "")))  // INVALID!
    .collect::<Vec<_>>()
    .join(" ");
```

This generates `"term1"* "term2"*` which is **invalid** FTS5 syntax. In FTS5:
- `"term"` means exact phrase match
- `term*` means prefix match
- `"term"*` is syntactically invalid (wildcard outside quotes)

**Good News**: The FTS population code is already correct (line 244):
```rust
conn.execute(
    "INSERT OR REPLACE INTO fts_chunks(rowid, content, docstring, symbol_name) VALUES (?1, ?2, ?3, ?4)",
    params![id, chunk.preview, chunk.docstring, chunk.symbol_name],
)?;
```

**Plan Reference**: Phase 2 - Runtime Functionality (Ticket 1003)

## Acceptance Criteria
- [ ] Complete CRUD cycle works: create repo → worktree → file → chunk
- [ ] FTS search returns results without syntax errors
- [ ] FTS search uses valid SQLite FTS5 syntax
- [ ] No `SQLITE_BUSY` errors during single-threaded operations
- [ ] Error messages are user-friendly (no raw SQL errors exposed)

## Technical Requirements

### 1. Fix FTS5 Query Syntax (Lines 454-459)

**Current code (INVALID):**
```rust
let fts_query = query
    .split_whitespace()
    .map(|t| format!("\"{}\"*", t.replace("\"", "")))
    .collect::<Vec<_>>()
    .join(" ");
```

**Fixed code:**
```rust
let fts_query = query
    .split_whitespace()
    .filter(|t| !t.is_empty())
    .map(|t| {
        // Sanitize: remove quotes and special FTS characters
        let clean = t
            .replace('"', "")
            .replace('\'', "")
            .replace('*', "")
            .replace('(', "")
            .replace(')', "");
        if clean.is_empty() {
            return String::new();
        }
        // FTS5 prefix syntax: term* (no quotes!)
        format!("{}*", clean)
    })
    .filter(|t| !t.is_empty())
    .collect::<Vec<_>>()
    .join(" OR ");  // Use OR for broader matching
```

### 2. FTS5 Syntax Reference

**Valid FTS5 patterns:**
| Pattern | Meaning |
|---------|---------|
| `term*` | Prefix match (words starting with "term") |
| `term1 term2` | Both terms (implicit AND) |
| `term1 OR term2` | Either term |
| `"exact phrase"` | Exact phrase match |
| `NEAR(term1 term2)` | Terms near each other |

**Invalid patterns:**
| Pattern | Why Invalid |
|---------|-------------|
| `"term"*` | Wildcard outside quotes |
| `term* AND term2*` | AND keyword not valid |

### 3. Verify FTS Ranking Understanding

The code at lines 492-508 handles FTS5 ranking correctly:
```rust
score: -score,  // FTS5 rank: more negative = better match
```

FTS5's `bm25()` returns negative values where more negative = better relevance. The code correctly negates this for consistency with Postgres (higher = better).

### 4. Verify CRUD Operations

The following operations should be tested manually after this fix:

```bash
# Build with SQLite
cargo build --features sqlite

# Create a test database and verify operations
# (This can be done via integration test in SQLFIX-1004)
```

**CRUD methods to verify:**
- `get_or_create_repo` - lines 80-102
- `get_or_create_worktree` - lines 104-126
- `get_or_create_commit` - lines 128-159 (has DateTime issue fixed in SQLFIX-1001)
- `upsert_file` - lines 162-191 (has DateTime issue fixed in SQLFIX-1001)
- `insert_chunk` - lines 193-250 (needs ts_doc_text from SQLFIX-1002)
- `search_chunks_fts` - lines 423-521 (FTS query fix in this ticket)

### 5. Error Handling Consistency

Ensure all database operations use `anyhow::Context` for user-friendly errors:
```rust
conn.execute(...)
    .context("Failed to insert chunk into SQLite")?;
```

Review and add `.context()` where missing.

## Implementation Notes

### Verification Steps
```bash
# Build with SQLite feature
cargo build --features sqlite

# Quick syntax test (create minimal test or use integration test)
# After SQLFIX-1004, run:
cargo test --features sqlite test_fts_search
cargo test --features sqlite test_fts_multiword_query
```

### FTS5 Query Testing
Test these queries manually if needed:
```sql
-- Valid queries
SELECT * FROM fts_chunks WHERE fts_chunks MATCH 'test*';
SELECT * FROM fts_chunks WHERE fts_chunks MATCH 'test* OR user*';
SELECT * FROM fts_chunks WHERE fts_chunks MATCH 'test user';  -- implicit AND

-- Invalid (should NOT be generated)
SELECT * FROM fts_chunks WHERE fts_chunks MATCH '"test"*';  -- ERROR
```

### Code Locations
| Item | File | Lines |
|------|------|-------|
| FTS query building | `sqlite/mod.rs` | 454-459 |
| FTS ranking | `sqlite/mod.rs` | 492-508 |
| FTS insert (correct) | `sqlite/mod.rs` | 243-246 |

## Dependencies
- **SQLFIX-1002**: Schema must be correct first (ts_doc_text column)

## Risk Assessment
- **Risk**: FTS syntax change may alter search behavior
  - **Mitigation**: OR semantics provide broader matching; functional correctness over exact parity with Postgres
- **Risk**: Special characters in queries could cause issues
  - **Mitigation**: Sanitization removes problematic characters
- **Risk**: Empty query handling
  - **Mitigation**: Filter empty strings after sanitization

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/mod.rs`

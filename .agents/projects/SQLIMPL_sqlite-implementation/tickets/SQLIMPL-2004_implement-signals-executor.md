# Ticket: SQLIMPL-2004: Implement Signals Executor

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the Signals executor for recency and churn scoring. Unlike other Phase 2 tickets, this requires NEW code to query the `commits` table for timestamp data - there's no existing SqliteStore method to delegate to.

## Background
The Signals executor at `src/search/signals.rs:86,116` has placeholder implementations for recency and churn scoring. These signals use git commit metadata to boost recently-modified and frequently-changed code.

**Note:** This is the one Phase 2 ticket that requires genuine new implementation, not just wiring.

This ticket implements Plan Phase 2, Ticket 2004: "Implement Signals Executor (NEW CODE)".

## Acceptance Criteria
- [ ] Recency scoring implemented using `commits.committed_at` timestamps
- [ ] Churn scoring implemented using commit frequency per file
- [ ] Decay function applied: `score = 1.0 / (days_old + 1.0)`
- [ ] TODO comments and placeholders removed from `signals.rs:86,116`
- [ ] Signals executor returns recency-weighted scores
- [ ] Signal tests (from Phase 1) now pass

## Technical Requirements
- Query `commits` table for `committed_at` timestamps
- Join through `files` table to get commit info per chunk
- Apply decay function for recency: `1.0 / (days_since_commit + 1.0)`
- Churn score based on number of commits touching the file
- Use `SqliteStore::run()` pattern for database access

## Implementation Notes

### Current Code (to replace)
```rust
// src/search/signals.rs:86
// TODO(IDXABS-2003): This is a placeholder implementation.
// (recency scoring stub)

// src/search/signals.rs:116
// TODO(IDXABS-2003): This is a placeholder implementation.
// (churn scoring stub)
```

### Database Schema Reference
```sql
-- commits table
CREATE TABLE commits (
    id INTEGER PRIMARY KEY,
    repo_id INTEGER,
    sha TEXT,
    committed_at TEXT  -- ISO8601 timestamp
);

-- files table
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    repo_id INTEGER,
    relpath TEXT,
    blob_sha TEXT
);
```

### Target Implementation Pattern

#### Recency Scoring
```rust
pub async fn get_recency_scores(&self, chunk_ids: &[i64]) -> Result<HashMap<i64, f64>> {
    self.store.run(move |conn| {
        let mut scores = HashMap::new();
        let now = chrono::Utc::now();

        // Query most recent commit per chunk via file relationship
        let sql = r#"
            SELECT c.id, MAX(commits.committed_at) as last_commit
            FROM chunks c
            JOIN files f ON c.relpath = f.relpath AND c.repo_id = f.repo_id
            JOIN commits ON commits.repo_id = f.repo_id
            WHERE c.id IN (?)
            GROUP BY c.id
        "#;

        // Execute and calculate decay
        for (chunk_id, last_commit) in results {
            let days_old = (now - last_commit).num_days() as f64;
            let score = 1.0 / (days_old.max(0.0) + 1.0);
            scores.insert(chunk_id, score);
        }

        Ok(scores)
    }).await
}
```

#### Churn Scoring
```rust
pub async fn get_churn_scores(&self, chunk_ids: &[i64]) -> Result<HashMap<i64, f64>> {
    // Count number of commits per file, normalize to 0-1 range
    // High churn = frequently modified = potentially important
}
```

### Score Normalization
- Recency: `1.0 / (days + 1)` → Recent = 1.0, older decays
- Churn: Normalize commit count to 0-1 based on max in result set

## Dependencies
- Phase 1 Complete (tests compile)
- commits table must be populated (happens during scan)

## Risk Assessment
- **Risk**: commits table may not have data for all files
  - **Mitigation**: Return neutral score (0.5) for files without commit data
- **Risk**: Complex join may be slow
  - **Mitigation**: Batch queries, consider adding index if needed

## Files/Packages Affected
- `crates/maproom/src/search/signals.rs` (primary)

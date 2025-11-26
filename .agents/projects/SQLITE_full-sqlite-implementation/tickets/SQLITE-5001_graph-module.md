# Ticket: SQLITE-5001: Graph Module

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement graph traversal for caller/callee and import relationships using SQLite recursive CTEs.

## Background
The `chunk_edges` table stores relationships between chunks (calls, imports, extends). Graph traversal enables queries like "what functions call this function?" with transitive depth. SQLite supports recursive CTEs for this purpose.

Implements: Plan Phase 5 - Graph Traversal

## Acceptance Criteria
- [ ] `graph.rs` module created with traversal methods
- [ ] `find_callers(chunk_id, max_depth)` returns chunks that call target
- [ ] `find_callees(chunk_id, max_depth)` returns chunks called by source
- [ ] `find_imports(chunk_id, direction)` returns import relationships
- [ ] Cycles handled gracefully (no infinite loops)
- [ ] Depth limited (default 3, max 10)
- [ ] Results include path information and depth
- [ ] Tests verify traversal correctness

## Technical Requirements
Create `crates/maproom/src/db/sqlite/graph.rs`:

```rust
pub struct GraphResult {
    pub chunk_id: i64,
    pub depth: usize,
    pub path: Vec<i64>,  // Path from source to this chunk
}

const DEFAULT_MAX_DEPTH: usize = 3;
const HARD_MAX_DEPTH: usize = 10;

impl SqliteStore {
    /// Find all chunks that call the target chunk (directly or transitively)
    pub async fn find_callers(
        &self,
        target_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> Result<Vec<GraphResult>> {
        let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).min(HARD_MAX_DEPTH);

        self.run(move |conn| {
            let sql = r#"
                WITH RECURSIVE callers(chunk_id, depth, path) AS (
                    -- Base case: direct callers
                    SELECT src_chunk_id, 1, '/' || src_chunk_id
                    FROM chunk_edges
                    WHERE dst_chunk_id = ?1 AND type = 'calls'

                    UNION ALL

                    -- Recursive case: callers of callers
                    SELECT e.src_chunk_id, c.depth + 1,
                           c.path || '/' || e.src_chunk_id
                    FROM chunk_edges e
                    JOIN callers c ON e.dst_chunk_id = c.chunk_id
                    WHERE c.depth < ?2
                      AND e.type = 'calls'
                      -- Cycle detection: don't revisit chunks in path
                      AND c.path NOT LIKE '%/' || e.src_chunk_id || '%'
                )
                SELECT chunk_id, depth, path
                FROM callers
                ORDER BY depth, chunk_id
            "#;

            let mut stmt = conn.prepare(sql)?;
            let results = stmt.query_map(
                params![target_chunk_id, depth],
                |row| {
                    let chunk_id: i64 = row.get(0)?;
                    let depth: i64 = row.get(1)?;
                    let path_str: String = row.get(2)?;
                    Ok(GraphResult {
                        chunk_id,
                        depth: depth as usize,
                        path: parse_path(&path_str),
                    })
                }
            )?;

            results.collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow::anyhow!("{}", e))
        }).await
    }

    /// Find all chunks called by the source chunk (directly or transitively)
    pub async fn find_callees(
        &self,
        source_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> Result<Vec<GraphResult>> {
        let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).min(HARD_MAX_DEPTH);

        self.run(move |conn| {
            let sql = r#"
                WITH RECURSIVE callees(chunk_id, depth, path) AS (
                    -- Base case: direct callees
                    SELECT dst_chunk_id, 1, '/' || dst_chunk_id
                    FROM chunk_edges
                    WHERE src_chunk_id = ?1 AND type = 'calls'

                    UNION ALL

                    -- Recursive case: callees of callees
                    SELECT e.dst_chunk_id, c.depth + 1,
                           c.path || '/' || e.dst_chunk_id
                    FROM chunk_edges e
                    JOIN callees c ON e.src_chunk_id = c.chunk_id
                    WHERE c.depth < ?2
                      AND e.type = 'calls'
                      -- Cycle detection
                      AND c.path NOT LIKE '%/' || e.dst_chunk_id || '%'
                )
                SELECT chunk_id, depth, path
                FROM callees
                ORDER BY depth, chunk_id
            "#;

            // ... similar to find_callers ...
        }).await
    }

    /// Find import relationships
    pub async fn find_imports(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
    ) -> Result<Vec<GraphResult>> {
        // Similar pattern but for 'imports' edge type
        // direction: Incoming = who imports this, Outgoing = what this imports
    }
}

fn parse_path(path_str: &str) -> Vec<i64> {
    path_str.trim_matches('/')
        .split('/')
        .filter_map(|s| s.parse().ok())
        .collect()
}

pub enum ImportDirection {
    Incoming,  // What imports this chunk
    Outgoing,  // What this chunk imports
}
```

## Implementation Notes
- Recursive CTE handles transitive closure
- Cycle detection via path tracking (LIKE '%/id/%' check)
- Path stored as string because SQLite doesn't have array type
- Hard limit of 10 prevents runaway recursion
- Order by depth for predictable results (closest first)
- Edge types: 'calls', 'imports', 'extends' (may have others)

## Dependencies
- SQLITE-1001 (Schema Migration) - chunk_edges table must exist

## Risk Assessment
- **Risk**: Large call graphs cause slow queries
  - **Mitigation**: Hard depth limit of 10; index on dst_chunk_id
- **Risk**: Cycles in graph cause issues
  - **Mitigation**: Path-based cycle detection in CTE

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/graph.rs` (NEW)
- `crates/maproom/src/db/sqlite/mod.rs` (export graph module)

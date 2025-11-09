# IDXSIZE: PostgreSQL Index Size Limit Fix

## Problem

PostgreSQL B-tree indexes have a hard limit of ~2704 bytes per index row. The current `idx_chunks_search_covering` index includes the `preview` TEXT column via `INCLUDE`, which fails when preview text exceeds this limit during normal code indexing.

```
ERROR: index row size 2768 exceeds btree version 4 maximum 2704
HINT: Values larger than 1/3 of a buffer page cannot be indexed.
```

This is **not an edge case** - it happens with:
- Long code lines (common in modern JavaScript/TypeScript)
- Large string literals or template strings
- Minified code
- Generated code
- Normal documentation strings

## Current Impact

- **Indexing fails** on real-world codebases
- **User experience broken** - can't complete initial scan
- **No workaround** except dropping the index (loses performance)
- **Affects all users** indexing normal code

## Proposed Solution

Redesign covering indexes to handle large text fields while maintaining query performance:

1. **Remove large TEXT fields from INCLUDE** - Don't include `preview` or `symbol_name` directly
2. **Use hash-based lookup** - Include MD5 hash of text fields for equality checks
3. **Maintain separate non-covering indexes** - For queries that need full text access
4. **Implement intelligent index selection** - Query planner chooses best index based on predicate

## Success Criteria

- ✅ Index any code without size errors
- ✅ Maintain query performance (<10ms for typical searches)
- ✅ Backward compatible (no data migration required)
- ✅ Handles edge cases (minified code, large strings, generated code)

## Relevant Agents

- **database-engineer** - Schema design and migration implementation
- **rust-indexer-engineer** - Update indexing code if needed
- **general-purpose** - Testing and validation

## Planning Documents

- [analysis.md](planning/analysis.md) - PostgreSQL index internals and problem analysis
- [architecture.md](planning/architecture.md) - Index redesign and query optimization strategy
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach for schema changes
- [security-review.md](planning/security-review.md) - Security implications assessment
- [plan.md](planning/plan.md) - Phased implementation roadmap

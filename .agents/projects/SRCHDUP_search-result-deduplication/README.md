# SRCHDUP: Search Result Deduplication

**Status:** 📋 Planning Complete - Ready for Ticket Creation
**Priority:** 🟡 High - Significantly improves search quality
**Duration:** 3-5 days (8-12 tickets)
**Complexity:** Low - Focused feature addition

---

## Project Summary

Implement search result deduplication to eliminate duplicate chunks that appear when the same code exists in multiple worktrees. Currently, searching for a function returns multiple identical results (one per worktree snapshot), burying unique findings in noise.

**Current Behavior:**
```
Search "validate_provider" → 15 results (same chunk from 15 worktrees)
```

**Target Behavior:**
```
Search "validate_provider" → 1 result (highest-scoring instance)
```

---

## Problem Statement

When code is indexed across multiple worktrees (main, feature branches, stale snapshots), the same logical code chunk appears multiple times in search results. This causes:

- **Signal buried in noise:** 10 unique findings become 150 results
- **User confusion:** Which result is the "real" one?
- **Wasted context:** AI agents include duplicate information
- **Performance overhead:** More results to process and display

**Root Cause:** Each worktree creates separate chunk records. Search queries across all worktrees without deduplication.

---

## Proposed Solution

Add a post-fusion deduplication step to the search pipeline:

1. **Group results** by identity key: `(relpath, symbol_name, start_line)`
2. **Select representative** from each group (highest score)
3. **Return deduplicated set** maintaining score order

**Key Features:**
- Enabled by default (users benefit immediately)
- Configurable via `SearchOptions.deduplicate` flag
- Preserves full scoring before selection
- Minimal performance impact (<10ms for 1000 results)

---

## Scope

### In Scope
- Deduplication module (`crates/maproom/src/search/dedup.rs`)
- SearchOptions extension with `deduplicate` flag
- Pipeline integration (after RRF fusion)
- MCP `search` tool parameter exposure
- Unit tests, integration tests, benchmarks
- Documentation updates

### Out of Scope
- Content-based identity (blob_sha) - future enhancement
- Worktree priority selection (prefer "main") - future enhancement
- Fuzzy line matching for near-duplicates - future enhancement
- Database-level deduplication - too complex

---

## Technical Approach

### Architecture

```
Search Pipeline (existing)
         ↓
  Fusion (RRF)
         ↓
  ┌─────────────────┐
  │  Deduplicator   │  ← NEW
  │  (post-fusion)  │
  └─────────────────┘
         ↓
  FinalSearchResults
```

### Identity Key

```rust
struct ChunkIdentity {
    relpath: String,      // File path
    symbol_name: String,  // Function/class name
    start_line: i32,      // Starting line
}
```

### Selection Strategy

Among duplicates, select the chunk with the **highest score**. This preserves the search ranking intent while eliminating redundancy.

---

## Agents

| Agent | Role |
|-------|------|
| rust-indexer-engineer | Core Rust implementation |
| integration-tester | Integration and E2E tests |
| vscode-extension-specialist | MCP TypeScript changes |
| technical-researcher | Documentation |
| verify-ticket | Verification |
| commit-ticket | Commits |

---

## Success Criteria

- [ ] Search for known duplicate returns ≤1 result per (relpath, symbol)
- [ ] Representative selected is highest-scoring instance
- [ ] Default behavior is deduplicated (opt-out available)
- [ ] Performance impact <10ms for 1000 results
- [ ] All existing search tests continue passing
- [ ] MCP search tool exposes `deduplicate` parameter

---

## Planning Documents

| Document | Description |
|----------|-------------|
| [analysis.md](planning/analysis.md) | Problem definition, research, current state |
| [architecture.md](planning/architecture.md) | Technical design, data flow, API |
| [quality-strategy.md](planning/quality-strategy.md) | Testing approach, benchmarks |
| [security-review.md](planning/security-review.md) | Security assessment (low risk) |
| [plan.md](planning/plan.md) | Phased execution plan, ticket outline |

---

## Timeline

| Phase | Duration | Description |
|-------|----------|-------------|
| Phase 1 | 1-2 days | Core dedup module + unit tests |
| Phase 2 | 1 day | Pipeline integration + integration tests |
| Phase 3 | 1 day | MCP exposure + E2E tests |
| Phase 4 | 0.5-1 day | Documentation + verification |

**Total:** 3-5 days

---

## Related Work

- **IDXCLEAN** (In Progress): Removes stale worktrees at the source
- **SEMRANK** (Completed): Improved search ranking with exact match boost
- **OPNFIX** (Completed): Fixed open tool path resolution

This project provides immediate duplicate reduction regardless of IDXCLEAN progress.

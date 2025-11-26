# Analysis: Search Result Deduplication

## Problem Definition

### Core Issue

When searching the maproom index, users see the **same code chunk multiple times** in results when that code exists in multiple worktrees. This creates severe search quality degradation:

- Searching for `validate_provider` returns 15+ identical results from different worktree snapshots
- Users cannot identify which result is the "canonical" version
- Result lists are dominated by duplicate noise, burying unique findings
- Combined with stale worktrees (IDXCLEAN issue), the problem is compounded

### Root Cause Analysis

**How Duplicates Occur:**
1. Code is indexed per-worktree (each worktree creates separate chunks)
2. Same file exists in multiple worktrees (main, feature branches, stale snapshots)
3. Search queries across all worktrees in a repository
4. Same chunk content appears once per worktree it exists in

**Why It's Not Already Solved:**
- Maproom's `worktree_id` filter is optional - callers often search across all worktrees
- No post-processing deduplication exists in the search pipeline
- Chunks have unique IDs per worktree, so database-level dedup isn't automatic
- The `blob_sha` content-addressing (for embeddings) doesn't affect chunk result deduplication

### Impact Assessment

**Search Quality Impact:**
- **Signal buried in noise**: 10 unique results become 150 results (15x duplication)
- **User trust erosion**: "Why does search show the same thing repeatedly?"
- **Incorrect chunk_id selection**: Users may pick chunk from wrong worktree for context
- **Wasted context window**: AI agents include duplicate context when they should include diverse results

**Performance Impact:**
- More results to rank and sort
- Larger response payloads
- More chunks to fetch details for

**User Experience Impact:**
- Confusion about which worktree to use
- Manual mental deduplication required
- Search feels broken even when technically working

## Existing Solutions and Approaches

### Industry Approaches

**Search Engine Deduplication:**
- Google/Bing: Deduplicate by URL + content hash
- Use canonical URLs to identify "primary" source
- Show "similar results" collapsed under primary

**Code Search Tools:**
- Sourcegraph: Groups results by repository, shows latest version
- GitHub Code Search: Shows one result per file per repository
- OpenGrok: Shows file once, links to different branches

**Database Result Grouping:**
- SQL `GROUP BY` with aggregate functions
- DISTINCT ON (PostgreSQL) - select first row per group
- Window functions with `ROW_NUMBER()` for picking representative

### What Works / What Doesn't

**What Works:**
- Content hashing for identity (blob_sha) - identifies same content
- Deterministic selection (newest, main branch) - consistent representative
- Post-query deduplication in application - flexible, doesn't change queries
- Configurable behavior (enable/disable) - users can opt out

**What Doesn't Work:**
- Database-level dedup before scoring - loses valuable ranking info
- Hash-only identity - misses near-duplicates (whitespace changes)
- First-result-wins - non-deterministic, unpredictable
- Strict dedup - may hide legitimate variations (different test files)

## Current State Analysis

### Search Pipeline Flow

```
Query → Parser → Executors (FTS/Vector/Graph/Signals)
                     ↓
              RankedResults (per source)
                     ↓
              Fusion (RRF combines scores)
                     ↓
              FinalSearchResults (sorted by score)
                     ↓
              [NO DEDUPLICATION STEP]
                     ↓
              Returned to User
```

### Current ChunkSearchResult Structure

```rust
pub struct ChunkSearchResult {
    pub chunk_id: i64,           // Unique per worktree
    pub file_id: i64,            // Unique per worktree
    pub relpath: String,         // Same across worktrees
    pub symbol_name: Option<String>, // Same across worktrees
    pub kind: String,            // Same across worktrees
    pub start_line: i32,         // Usually same (modulo drift)
    pub end_line: i32,           // Usually same (modulo drift)
    pub preview: String,         // Usually same
    pub score: f32,              // Varies by worktree freshness
    pub source_scores: HashMap<SearchSource, f32>,
}
```

### Available Identity Fields

For deduplication, we can identify "same chunk" by:

| Field | Reliability | Notes |
|-------|-------------|-------|
| `relpath` | High | Same path = same file |
| `symbol_name` | High | Same function/class |
| `start_line` | Medium | Can drift with edits |
| `kind` | High | Function, class, etc. |
| `blob_sha` | Very High | Content hash (not in search results currently) |

**Best Identity Key:** `(relpath, symbol_name, start_line)` or `(relpath, blob_sha)`

### Worktree Information

- `worktree_id` is available in SearchOptions for filtering
- Worktree name is NOT currently in ChunkSearchResult
- To prefer "main" worktree, we'd need to add worktree metadata

## Opportunities

### Deduplication Point

The ideal insertion point is **after fusion, before returning FinalSearchResults**:

```
Fusion (RRF combines scores)
       ↓
  [DEDUPLICATE HERE]
       ↓
FinalSearchResults (deduplicated)
```

This preserves full scoring information for selecting the best representative.

### Identity Options

1. **Simple:** `(relpath, symbol_name, start_line)` - existing fields
2. **Content-aware:** Include `blob_sha` in ChunkSearchResult for exact content matching
3. **Fuzzy:** Group by `(relpath, symbol_name)` only, ignoring line drift

### Representative Selection

When duplicates exist, select representative by:
1. **Highest score** - best match wins
2. **Worktree priority** - prefer "main" > named branches > stale
3. **Recency** - prefer most recently indexed
4. **Combination** - main worktree tie-breaker for equal scores

## Constraints and Considerations

### Must Preserve
- All search modes (FTS, vector, hybrid, graph)
- Score integrity (selected representative keeps its score)
- Performance (<10% latency increase acceptable)
- Backward compatibility (no API changes)

### Configuration Needs
- Enable/disable deduplication (default: enabled)
- Selection strategy (score-first vs worktree-priority)
- Grouping granularity (strict vs fuzzy)

### Edge Cases
- Same symbol name in different files → NOT duplicates
- Same file, different line numbers → likely duplicates (minor edit)
- Same file, vastly different scores → may indicate different relevance
- Zero symbol_name → group by relpath only

## Success Criteria

1. Search for known symbol returns ≤1 result per unique (relpath, symbol)
2. Representative selected is highest-scoring or from "main" worktree
3. Deduplication is transparent (users don't need to know about it)
4. Performance impact <10% additional latency
5. Configurable via feature flag or API parameter

## Research Summary

**Key Findings:**
1. Problem is well-understood: same content indexed multiple times per worktree
2. Solution is straightforward: post-fusion grouping with representative selection
3. Identity key: `(relpath, symbol_name, start_line)` is sufficient for MVP
4. Best insertion point: after RRF fusion, before returning results
5. Configuration: enable by default, allow opt-out

**Recommended Approach:**
- Add `deduplicate_results()` function in search pipeline
- Group by `(relpath, symbol_name, start_line)`
- Select highest-scoring representative per group
- Add worktree name to ChunkSearchResult for future worktree-priority selection
- Make configurable via `SearchOptions.deduplicate: bool`

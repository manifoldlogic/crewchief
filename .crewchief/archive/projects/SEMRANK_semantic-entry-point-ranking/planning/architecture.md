# Architecture: Semantic Entry Point Ranking

## Design Principles

1. **Leverage Existing Data:** Use metadata already extracted (kind, symbol_name)
2. **Preserve FTS Baseline:** Don't replace ts_rank_cd, enhance it
3. **Multiplicative Composition:** Combine signals via multiplication for intuitive tuning
4. **Future-Proof:** Design works in current FTS-only and future hybrid modes
5. **Minimal Complexity:** No ML, no new indices, just smarter scoring

## Prerequisites

**IMPORTANT:** This architecture assumes a TypeScript MCP search tool exists at `/packages/maproom-mcp/src/tools/search.ts`.

**Current State:** This tool does NOT exist. It must be created in Phase 0 before semantic enhancements can be implemented.

**Rust Implementation Exists:** The Rust FTS implementation exists at `/crates/maproom/src/search/fts.rs` (lines 77-99). The TypeScript tool will wrap this via MCP protocol.

**Phase 0 Deliverable:** Create `search.ts` that calls Rust FTS and returns chunk results via MCP.

## Architecture Overview

### Current Flow (Rust FTS Implementation)

```
User Query
    ↓
Tokenize & Prefix Match ("validate" → "validate:*")
    ↓
PostgreSQL FTS (to_tsquery + ts_doc @@)
    ↓
ts_rank_cd() Scoring + Exact Bonus (+0.2 if symbol ILIKE '%query%')
    ↓
ORDER BY fts_score DESC
    ↓
Return Top K Results
```

**Problem:** ts_rank_cd() only sees term frequency, not code semantics. Existing exact bonus is additive (+0.2) and uses substring matching (ILIKE), not exact match.

### Proposed Flow (Semantic-Enhanced FTS)

```
User Query
    ↓
Tokenize & Prefix Match
    ↓
PostgreSQL FTS (unchanged)
    ↓
ts_rank_cd() Base Score
    ↓
+──────────────────────────────────────+
│  Semantic Enhancement Layer          │
│  1. Compute kind_multiplier          │
│  2. Compute exact_match_multiplier   │
│  3. final = base × kind × exact      │
+──────────────────────────────────────+
    ↓
ORDER BY final_score DESC
    ↓
Return Top K Results
```

**Solution:** Enhance FTS score with code-semantic multipliers.

## Scoring Formula Design

### Base Formula

```sql
final_score = ts_rank_cd(c.ts_doc, query)
            × kind_multiplier(c.kind)
            × exact_match_multiplier(c.symbol_name, query_text)
```

### Component 1: Kind Multiplier

**Purpose:** Boost implementations over tests/docs.

**Rationale:**
- Implementations are better entry points for context() traversal
- Tests/docs are valuable but shouldn't dominate when looking for concepts
- Kind metadata is already extracted via tree-sitter

**Multiplier Values:**

**Note:** Actual database enum values from `init.sql:44`: `'func','class','component','hook','module','var','type','other'` plus heading types for markdown.

| Chunk Kind (Database Value) | Multiplier | Reasoning |
|------------------------------|-----------|-----------|
| `func` | 2.5 | Primary implementation code, best entry point |
| `class` | 2.0 | Important structural elements |
| `component` | 2.0 | React/UI components, equally important as classes |
| `hook` | 1.8 | React hooks, useful starting points for UI logic |
| `module` | 1.5 | Module definitions, moderate importance |
| `type` | 1.5 | Type definitions, data structures |
| `var` | 1.0 | Variables/constants, neutral baseline |
| `heading_1` | 0.6 | Top-level markdown headings, structural but not code |
| `heading_2` | 0.5 | Sub-headings in documentation |
| `heading_3` | 0.4 | Nested headings, lowest doc priority |
| `other` | 1.0 | Unknown/unclassified chunks, neutral |
| `NULL` | 1.0 | Chunks without kind detection, neutral |

**SQL Implementation:**

```sql
-- Source: packages/maproom-mcp/config/init.sql:44 (maproom.symbol_kind enum)
kind_multiplier = CASE
  WHEN c.kind = 'func' THEN 2.5
  WHEN c.kind IN ('class', 'component') THEN 2.0
  WHEN c.kind = 'hook' THEN 1.8
  WHEN c.kind IN ('module', 'type') THEN 1.5
  WHEN c.kind = 'var' THEN 1.0
  WHEN c.kind = 'heading_1' THEN 0.6
  WHEN c.kind = 'heading_2' THEN 0.5
  WHEN c.kind = 'heading_3' THEN 0.4
  WHEN c.kind = 'other' THEN 1.0
  WHEN c.kind IS NULL THEN 1.0
  ELSE 1.0
END
```

**Tuning Strategy:**
- Start with values above (derived from research + intuition)
- Monitor search quality metrics (implementation as #1 result rate)
- A/B test multiplier adjustments if needed
- Expose as configuration for future tuning

### Component 2: Exact Match Multiplier

**Purpose:** Boost when query exactly matches symbol_name.

**Rationale:**
- User searching "validate_provider" likely wants that specific function
- Symbol name is the canonical identifier (extracted from AST)
- Case-insensitive matching handles common variations

**Multiplier Values:**

| Match Type | Multiplier | Condition |
|-----------|-----------|-----------|
| Exact match | 3.0 | `LOWER(symbol_name) = LOWER(query)` |
| No match | 1.0 | Baseline (FTS still applies) |

**SQL Implementation:**

```sql
exact_match_multiplier = CASE
  WHEN LOWER(c.symbol_name) = LOWER($normalized_query) THEN 3.0
  ELSE 1.0
END
```

**Edge Cases:**

1. **Multi-token queries:** "validate provider"
   - Normalize: Remove spaces, try variations
   - Try: "validateprovider", "validate_provider", "validateProvider"
   - If any match symbol_name, apply boost

2. **Partial matches:** Query "validate", symbol "validate_provider"
   - Do NOT boost exact match multiplier
   - FTS will still match and score appropriately
   - This preserves broad search capability

3. **Null symbol_name:** Some chunks lack symbols (comments, docs)
   - Treat as no match (multiplier = 1.0)
   - Fall back to pure FTS + kind boost

### Component 3: Combined Scoring

**Full SQL Query:**

```sql
SELECT
  c.id,
  c.symbol_name,
  c.kind,
  c.relpath,
  c.preview,

  -- Base FTS score
  ts_rank_cd(c.ts_doc, to_tsquery('simple', $fts_query)) AS base_score,

  -- Kind multiplier
  CASE
    WHEN c.kind IN ('function', 'method') THEN 2.5
    WHEN c.kind = 'class' THEN 2.0
    WHEN c.kind IN ('interface', 'trait') THEN 1.8
    WHEN c.kind IN ('type_alias', 'struct', 'enum') THEN 1.5
    WHEN c.kind IN ('test', 'test_function') THEN 0.6
    WHEN c.kind IN ('docstring', 'comment', 'documentation') THEN 0.4
    WHEN c.kind IN ('import', 'use', 'require') THEN 0.3
    ELSE 1.0
  END AS kind_mult,

  -- Exact match multiplier
  CASE
    WHEN LOWER(c.symbol_name) = LOWER($normalized_query) THEN 3.0
    ELSE 1.0
  END AS exact_mult,

  -- Final score
  ts_rank_cd(c.ts_doc, to_tsquery('simple', $fts_query))
    * CASE
        WHEN c.kind IN ('function', 'method') THEN 2.5
        WHEN c.kind = 'class' THEN 2.0
        WHEN c.kind IN ('interface', 'trait') THEN 1.8
        WHEN c.kind IN ('type_alias', 'struct', 'enum') THEN 1.5
        WHEN c.kind IN ('test', 'test_function') THEN 0.6
        WHEN c.kind IN ('docstring', 'comment', 'documentation') THEN 0.4
        WHEN c.kind IN ('import', 'use', 'require') THEN 0.3
        ELSE 1.0
      END
    * CASE
        WHEN LOWER(c.symbol_name) = LOWER($normalized_query) THEN 3.0
        ELSE 1.0
      END AS final_score

FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', $fts_query)
  AND ($repo_filter IS NULL OR c.file_id IN (SELECT id FROM maproom.files WHERE repo_id = $repo_filter))

ORDER BY final_score DESC
LIMIT $limit;
```

## TypeScript Integration

**Location:** `packages/maproom-mcp/src/tools/search.ts`

**Changes Required:**

1. **Query Normalization (Enhanced for Acronyms):**
   ```typescript
   function normalizeForExactMatch(query: string): string {
     let normalized = query;

     // Handle consecutive uppercase (acronyms): HTTP → http, XMLParser → xml_parser
     normalized = normalized.replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2'); // XMLParser → XML_Parser
     normalized = normalized.replace(/([A-Z]{2,})/g, (match) => match.toLowerCase() + '_'); // HTTP → http_

     // Handle camelCase → snake_case
     normalized = normalized.replace(/([a-z])([A-Z])/g, '$1_$2'); // validateProvider → validate_Provider

     // Handle kebab-case and spaces → snake_case
     normalized = normalized.replace(/[\s\-\.]/g, '_'); // validate-provider → validate_provider

     // Lowercase everything
     normalized = normalized.toLowerCase();

     // Clean up multiple underscores and trailing underscores
     normalized = normalized.replace(/_+/g, '_').replace(/^_|_$/g, '');

     return normalized;
   }

   // Examples:
   // XMLParser → xml_parser
   // HTTPSHandler → https_handler
   // validateHTTPRequest → validate_http_request
   // Base64Encoder → base64_encoder
   // validate-provider → validate_provider
   // validateProvider → validate_provider
   ```

2. **SQL Parameter Passing:**
   ```typescript
   const normalizedQuery = normalizeForExactMatch(params.query);

   const result = await db.query({
     text: SQL_QUERY_ABOVE,
     values: [ftsQuery, normalizedQuery, repoFilter, limit]
   });
   ```

3. **Debug Mode Enhancement:**
   ```typescript
   if (params.debug) {
     return results.map(r => ({
       ...r,
       score_breakdown: {
         base_fts: r.base_score,
         kind_multiplier: r.kind_mult,
         exact_match_multiplier: r.exact_mult,
         final: r.final_score
       }
     }));
   }
   ```

## Score Normalization Impact

**Question:** Do multiplied scores (base × 2.5 × 3.0 = 7.5× boost) require normalization?

**Answer:** No normalization needed for current use cases.

**Rationale:**

1. **RRF Fusion Uses Ranks, Not Scores:**
   - RRF formula: `score = 1.0 / (k + rank + 1.0)`
   - Only relative ordering matters, not absolute score values
   - FTS score scale changes don't affect RRF output
   - **Conclusion**: Safe for hybrid search integration

2. **Weighted Fusion Uses Normalized Scores:**
   - Each signal's scores are min-max normalized to [0,1] before weighting
   - FTS scores normalized independently: `(score - min) / (max - min)`
   - Multipliers only affect relative ordering within FTS ranking
   - **Conclusion**: Safe, normalization happens automatically

3. **SQL Ordering:**
   - `ORDER BY final_score DESC` works correctly regardless of scale
   - Higher multiplied scores rank higher (as intended)
   - **Conclusion**: No issues with SQL ranking

**Future Consideration:**
- If raw scores are ever exposed to users/UI, consider min-max normalization for display
- Implementation: `(final_score - min_score) / (max_score - min_score)` → [0,1] range
- **Not needed for MVP**: Current use cases don't expose raw scores

## Integration with Hybrid Search (Current)

### RRF Fusion (Already Implemented)

**Location:** `crates/maproom/src/search/fusion/rrf.rs`

```rust
pub struct RRFFusion {
    k: f32, // Default: 60.0 (Cormack et al.)
}

impl RRFFusion {
    pub fn fuse(&self, rankings: Vec<Vec<SearchResult>>) -> Vec<SearchResult> {
        // RRF score = sum of 1.0 / (k + rank + 1.0) across all sources
        // ...existing implementation...
    }
}
```

**With Semantic FTS:**

Rankings to fuse:
1. **FTS ranking:** Now includes kind + exact match boosts
2. **Vector ranking:** Semantic similarity (when ready)
3. **Graph ranking:** PageRank/importance (optional)

**Benefit:** Improved FTS signal contributes better lexical component to RRF fusion. If FTS returns tests at rank 1, it pollutes the hybrid score. With semantic boosting, FTS contributes correct lexical signal.

### Weighted Fusion (Also Available)

**Location:** `crates/maproom/src/search/fusion/weighted.rs`

```rust
// Default weights
fts_weight: 0.40
vector_weight: 0.35
graph_weight: 0.10
recency_weight: 0.10
churn_weight: 0.05
```

**With Semantic FTS:** The `fts_weight: 0.40` now represents a higher-quality signal (implementations > tests), improving overall hybrid quality.

## Performance Considerations

### Query Performance

**Additional Overhead:**
- CASE statements: ~0.1ms per result (negligible)
- String comparison (LOWER): ~0.05ms per result
- Total: <5ms for typical 100-result set

**Mitigation:**
- Multiplier computation happens on already-filtered results (WHERE clause first)
- No additional table scans or joins
- No new indices required

### Indexing Performance

**No Changes Required:**
- Kind and symbol_name already extracted during indexing
- No new fields to populate
- No new indices to maintain

**Zero Cost Enhancement:** Leverages existing data.

## Migration from Current Exact Bonus

**Current Implementation** (`fts.rs:92-95`):
```rust
// Existing exact bonus in Rust FTS
if symbol_name ILIKE '%query%' then score + 0.2 else score
```

**Issues with Current Approach:**
1. **Additive Bonus**: Adding +0.2 has arbitrary scale, unclear interaction with base scores
2. **Substring Match**: `ILIKE '%query%'` matches "validateProvider" for query "valid" (too broad)
3. **Fixed Value**: No differentiation between exact match vs partial match

**SEMRANK Replacement:**
```sql
-- New multiplicative exact match
CASE
  WHEN LOWER(symbol_name) = LOWER($normalized_query) THEN 3.0  -- Exact match only
  ELSE 1.0
END
```

**Migration Benefits:**
1. **Multiplicative**: Scales with base FTS score naturally (high relevance × exact match = very high score)
2. **Exact Match Only**: More precise targeting, "validateProvider" only matches "validate_provider" after normalization
3. **Tunable**: 3.0× multiplier can be adjusted based on observed behavior

**Migration Task**: Remove old exact bonus logic from `fts.rs` during SEMRANK-2004b implementation to avoid conflicts.

## Configuration & Tuning

### Multiplier Tuning Criteria

**When to Adjust Multipliers:**

**Success Metrics (Target Values):**
- **Top-1 Implementation Rate**: >85% of exact function searches return implementation as #1 result
- **Average Implementation Rank**: <3 (implementation appears in top 3 results)
- **Latency**: p95 <200ms (no significant degradation)

**Trigger for Adjustment:**
- If Top-1 implementation rate drops below 70% → Increase `func` multiplier or increase exact_match boost
- If average implementation rank exceeds 5 → Review kind multipliers, tests may still rank too high
- If user feedback indicates wrong entry points → Analyze specific queries, adjust multipliers

**Tuning Process:**
1. **Collect 2-4 weeks of metrics** post-deployment
2. **Analyze distribution** of top-1 chunk kinds (should be >85% `func`, `class`, `component`)
3. **Adjust multipliers** by ±0.2 increments (e.g., if tests still too high, reduce `heading_*` multipliers)
4. **A/B test changes** before full deployment
5. **Document adjustments** in runbook for future reference

**Example Adjustment Scenario:**
```
Observed: Top-1 implementation rate = 65% (below 70% threshold)
Analysis: 20% of searches return `heading_1` chunks (markdown docs) as #1
Action: Reduce heading_1 multiplier from 0.6 → 0.4
Expected: Implementations rise to #1, Top-1 rate improves to ~80%
```

### Expose Multipliers as Config

**Future Enhancement** (not in MVP):

```typescript
interface ScoringConfig {
  kind_multipliers: {
    func: number;
    class: number;
    component: number;
    // ... etc
  };
  exact_match_boost: number;
}
```

**Default Config:**
- Hardcode initial values based on research
- Monitor metrics for 2-4 weeks
- Adjust if needed based on usage data

**MVP Approach:**
- Hardcode multipliers in SQL
- Document rationale in comments
- Easy to adjust in future iteration

## Technology Choices

### Why PostgreSQL CASE, Not Application Layer?

**Option A (Chosen):** SQL CASE statements
- ✅ Runs close to data (no network overhead)
- ✅ Leverages PostgreSQL query optimizer
- ✅ Simple to implement and debug
- ✅ Works with existing db connection pool

**Option B (Rejected):** Post-process in TypeScript
- ❌ Additional network latency (fetch all, rescore, resort)
- ❌ Can't use LIMIT efficiently (need to fetch more than needed)
- ❌ More complex code path

**Option C (Rejected):** Rust binary
- ❌ Overkill for simple arithmetic
- ❌ Requires RPC call to Rust from MCP server
- ❌ Adds deployment complexity

### Why Multiplicative, Not Additive?

**Multiplicative:** `final = base × kind × exact`
- ✅ Intuitive: "boost by X%"
- ✅ Preserves relative ordering within same kind
- ✅ Zero base score → zero final score (correct)

**Additive:** `final = base + kind_bonus + exact_bonus`
- ❌ Can't express "50% penalty" easily
- ❌ Arbitrary scale (what's the right bonus value?)
- ❌ Can make zero-relevance items rank high

## Long-Term Maintainability

### Monitoring & Metrics

**Key Metrics to Track:**
1. **Top-1 Accuracy:** Is #1 result an implementation (not test)?
2. **Implementation Rank:** Average rank of implementation chunks
3. **User Satisfaction:** Implicit (do users click #1 result?)

**Logging:**
```typescript
if (params.debug) {
  logger.info({
    query: params.query,
    top_result: { kind, symbol_name, final_score },
    score_breakdown: { base, kind_mult, exact_mult }
  });
}
```

### Future Enhancements (Out of Scope for MVP)

1. **Learning to Rank:** Use click data to train multiplier weights
2. **Personalization:** User-specific kind preferences
3. **Query Intent Classification:** Detect "navigational" vs "exploratory" queries
4. **Graph Signal:** Incorporate PageRank for high-centrality chunks

## Architecture Decision Records

### ADR-1: Use SQL CASE for Multipliers

**Context:** Need to apply kind/exact match multipliers to FTS scores.

**Decision:** Implement as SQL CASE statements in search query.

**Rationale:**
- Minimal latency (runs close to data)
- Leverages existing PostgreSQL infrastructure
- Simple to implement and debug
- No new services or complexity

**Alternatives Considered:** Application-layer rescoring (rejected: latency), Rust binary (rejected: complexity)

### ADR-2: Multiplicative Composition of Signals

**Context:** Need to combine base FTS, kind, exact match signals.

**Decision:** Use multiplication: `final = base × kind × exact`

**Rationale:**
- Intuitive interpretation (percentage boosts/penalties)
- Preserves relative ordering within groups
- Mathematically sound (zero relevance → zero score)

**Alternatives Considered:** Additive (rejected: arbitrary scales), weighted average (rejected: less intuitive)

### ADR-3: Hardcode Initial Multipliers

**Context:** Need to choose multiplier values for MVP.

**Decision:** Hardcode based on research and first principles, document in comments.

**Rationale:**
- No usage data yet for learning-based approach
- Research literature provides reasonable baselines
- Easy to adjust later with configuration system
- MVP mindset: ship with sensible defaults, iterate

**Alternatives Considered:** Expose as config immediately (rejected: premature), use ML (rejected: no training data)

## Summary

**Architecture Pattern:** SQL-based semantic enhancement layer on top of PostgreSQL FTS.

**Core Innovation:** Leverage AST-derived metadata (kind, symbol_name) that text-search tools fundamentally lack.

**Integration Point:** Single SQL query modification in `packages/maproom-mcp/src/tools/search.ts`.

**Performance Impact:** <5ms additional latency for 100 results.

**Future Compatibility:** Works in current FTS-only mode and future hybrid mode (improves RRF lexical signal).

**Strategic Positioning:** Not competing with grep on speed, competing on correctness of entry points for graph-based code understanding.

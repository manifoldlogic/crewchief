# Baseline FTS Search Behavior (SEMRANK-0002)

**Generated**: November 19, 2025
**Repository**: crewchief (74,394 chunks indexed)
**Search Mode**: FTS (Full-Text Search using PostgreSQL `ts_rank_cd()`)
**Purpose**: Document current ranking behavior before implementing semantic ranking enhancements

## Executive Summary

**Critical Finding**: Current FTS ranking consistently returns **documentation chunks (markdown headings) as top results** instead of **implementation code (functions, classes, modules)**.

- **10/10 test queries** returned markdown headings as #1 result
- **0/10 queries** returned implementation code as #1 result
- This behavior makes it difficult to find actual entry points for code navigation

This baseline validates the need for SEMRANK Phase 2 semantic ranking enhancements.

---

## Test Methodology

- **Corpus**: crewchief repository, 74,394 chunks
- **Languages**: Rust, TypeScript, Python, Markdown, JSON, YAML
- **Search Mode**: FTS only (ts_rank_cd scoring)
- **Limit**: 5-10 results per query
- **Focus**: Single-term and multi-term concept searches

### Current FTS Formula (Rust Implementation)

Location: `/crates/maproom/src/search/fts.rs` (lines 77-99)

```sql
ts_rank_cd(c.ts_doc, to_tsquery('simple', $query)) +
CASE
    WHEN c.symbol_name ILIKE '%' || $query || '%' THEN 0.2
    ELSE 0
END AS score
```

**Components**:
1. **Base FTS Score**: `ts_rank_cd()` - PostgreSQL full-text search ranking
2. **Exact Bonus**: +0.2 if symbol name contains query (substring match via ILIKE)

**Problem**: No consideration of chunk kind (function vs test vs documentation)

---

## Test Results

### Query: "authenticate"
**Type**: Single-term search (function name)

**Results**:
1. **heading_1** | `.agents/projects/WTSRCH_worktree-scoped-search/README.md` | score: 1.4
2. **heading_2** | `.agents/projects/WTSRCH_worktree-scoped-search/planning/quality-strategy.md` | score: 1.2
3. **func** | `crates/maproom/tests/relationship_test.rs` | score: 1.2

**Issue**: Documentation heading ranked #1. First `func` result is a test, not implementation.

---

### Query: "validate"
**Type**: Single-term search

**Results**:
1. **heading_1** | `.agents/archive/projects/TESTDES_grep-impossible-task-design/tickets/TESTDES-2901_tier1-suite-validation-tests.md` | score: 2.4
2. **heading_2** | `.agents/archive/projects/COMPFIX_competition-agent-setup-validation/tickets/COMPFIX-1004_security-controls.md` | score: 2.0
3. **heading_2** | `.agents/projects/OPNFIX_open-path-fix/tickets/OPNFIX-3002_security-test-suite.md` | score: 1.8

**Issue**: Top 3 results are all markdown headings, no implementation code visible.

---

### Query: "spawn"
**Type**: Single-term search (function name)

**Results**:
1. **heading_1** | `.agents/archive/projects/MCPSTART_mcp-provider-startup-fix/tickets/MCPSTART-2001_explicit-env-parameter-spawn-calls.md` | score: 1.6
2. **heading_1** | `.agents/projects/VSMAP_vscode-maproom-extension/tickets/VSMAP-1003_implement-binary-spawner.md` | score: 1.6
3. **heading_3** | `.agents/projects/VSMAP_vscode-maproom-extension/planning/security-review.md` | score: 1.05

**Issue**: All top results are markdown headings. No implementation code in top 10.

---

### Query: "database connection"
**Type**: Concept search (multi-word)

**Results**:
1. **heading_1** | `.agents/archive/projects/DBFALLBK_database-fallback/README.md` | score: 1.29
2. **heading_2** | `.agents/archive/projects/INC_INDEX_incremental-indexing/tickets/INC_INDEX-1001_database-url-validation.md` | score: 1.03
3. **heading_2** | `.agents/projects/VSMAP_vscode-maproom-extension/planning/architecture.md` | score: 0.74

**Issue**: Concept search also returns documentation, not implementation.

---

### Query: "error handling"
**Type**: Concept search (multi-word)

**Results**:
1. **heading_2** | `.agents/archive/projects/INCRSCAN_incremental-scan-completion/PROJECT_COMPLETION_SUMMARY.md` | score: 1.15
2. **heading_3** | `.agents/archive/projects/INCRSCAN_incremental-scan-completion/PROJECT_COMPLETION_SUMMARY.md` | score: 1.04
3. **heading_2** | `.agents/projects/WTSRCH_worktree-scoped-search/planning/architecture.md` | score: 1.00

**Issue**: Planning documents ranked highest for implementation concept.

---

### Query: "config"
**Type**: Single-term search

**Results**:
1. **heading_1** | `crates/maproom/docs/configuration_guide.md` | score: 4.6
2. **heading_2** | `.agents/archive/projects/TESTDES_grep-impossible-task-design/tickets/TESTDES-4003_task-generator.md` | score: 3.4
3. **heading_1** | `.agents/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/tickets/HYBRID_SEARCH-6002_configuration-management.md` | score: 2.8
4. **heading_2** | `crates/maproom/docs/configuration_guide.md` | score: 2.8
5. **module** | `crates/maproom/src/config/mod.rs` | score: 2.7

**Issue**: Documentation headings dominate. First implementation code (`module`) ranked #5.

---

### Query: "test"
**Type**: Meta-search

**Results**:
1. **heading_2** | `crates/maproom/BRANCHX_CRITICAL_PATH_STATUS.md` | score: 8.2
2. **heading_2** | `.agents/archive/projects/LOCAL_local-deployment/archive/tickets/LOCAL-2005_ollama-integration-tests.md` | score: 8.0
3. **heading_2** | `.agents/archive/projects/CONTEXT_ASM_context-assembly-engine/tickets/CONTEXT_ASM-4002_testing-suite.md` | score: 7.4

**Issue**: All headings. Extremely high scores for markdown chunks.

---

### Query: "mcp tool"
**Type**: Concept search (multi-word)

**Results**:
1. **heading_2** | `.crewchief/claude-code-plugins/docs/hooks-reference.md` | score: 2.41
2. **heading_2** | `.agents/archive/projects/MPEMBED_multi-provider-embeddings/MPEMBED_TICKET_INDEX.md` | score: 1.96
3. **heading_1** | `.agents/archive/projects/CONTEXT_ASM_context-assembly-engine/tickets/CONTEXT_ASM-4001_mcp-tool-implementation.md` | score: 1.16

**Issue**: Documentation and planning docs ranked highest.

---

### Query: "search"
**Type**: Single-term search

**Results**:
1. **heading_2** | `.agents/archive/sessions/INDEX_BY_PROJECT.md` | score: 5.6
2. **heading_1** | `.agents/archive/sessions/INDEX_BY_PROJECT.md` | score: 4.8
3. **heading_1** | `.agents/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/tickets/HYBRID_SEARCH-2003_initial-search-integration.md` | score: 4.6

**Issue**: Meta-documents (indexes, READMEs) ranked highest, not search implementation code.

---

### Query: "chunk"
**Type**: Data structure search

**Results**:
1. **heading_2** | `.agents/archive/projects/PERF_OPT_performance-optimization/tickets/PERF_OPT-2001_index-optimization.md` | score: 3.8
2. **heading_2** | `.agents/archive/projects/CONTEXT_ASM_context-assembly-engine/tickets/CONTEXT_ASM-3001_query-optimization.md` | score: 3.4
3. **func** | `crates/maproom/tests/graph_test.rs` | score: 3.2

**Issue**: Documentation ranked #1-2. First func is a test, not implementation.

---

## Problem Analysis

### Why Documentation Ranks Higher Than Code

**Term Frequency Bias**:
- Markdown documents use natural language with high term repetition
- Code has terse identifiers with low term repetition
- `ts_rank_cd()` favors documents with more occurrences of query terms

**Example**:
- README heading "Database Connection Setup" contains "database" 3 times
- Rust function `establish_db_connection()` contains "db" once (abbreviated)
- README scores higher despite being less relevant for code navigation

**Structural Bias**:
- Markdown headings often repeat key terms for clarity
- Code symbols use concise identifiers to avoid verbosity
- FTS algorithm doesn't distinguish between these contexts

### Impact on Developer Experience

1. **Poor Entry Points**: Users searching for "validate_provider" get test documentation, not the actual implementation
2. **Extra Scrolling**: Developers must scan past 5-10 documentation results to find code
3. **Context Traversal Issues**: Starting from documentation chunks provides poor `context()` graph traversal (docs don't have callers/callees)
4. **Discoverability**: Implementation patterns are buried under planning documents

---

## MCP Protocol Validation

### Tool Registration

```bash
$ echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | node dist/index.js
```

**Result**: ✅ Search tool found in MCP server tool list

**Tool Schema**:
```json
{
  "name": "search",
  "description": "Semantic code search via Rust binary subprocess",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": { "type": "string" },
      "repo": { "type": "string" },
      "worktree": { "type": "string", "optional": true },
      "limit": { "type": "number", "default": 20 },
      "mode": { "type": "string", "enum": ["fts"], "default": "fts" },
      "debug": { "type": "boolean", "default": false }
    },
    "required": ["query", "repo"]
  }
}
```

### Parameter Validation

**Test**: Missing required parameter (`query` omitted)
```bash
$ echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search","arguments":{"repo":"test"}}}' | node dist/index.js
```

**Result**: ✅ Proper error handling
```json
{
  "error": "VALIDATION_ERROR",
  "message": "Invalid parameters",
  "details": "query is required"
}
```

### Error Handling

**Test**: Invalid repo
```bash
$ echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search","arguments":{"repo":"nonexistent","query":"test"}}}' | node dist/index.js
```

**Result**: ✅ Clear error message
```json
{
  "error": "REPO_NOT_FOUND",
  "message": "Repository 'nonexistent' not found or no data indexed.",
  "hint": "Use the status tool to see available repositories, or use scan tool to index a new repository."
}
```

### Debug Mode

**Test**: Debug mode enabled
```bash
$ echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"search","arguments":{"repo":"crewchief","query":"search","limit":2,"debug":true}}}' | node dist/index.js
```

**Result**: ✅ Debug mode functional (returns score breakdown)

**Note**: Current implementation returns scores but not detailed breakdown (base_fts, multipliers). This will be enhanced in SEMRANK-2006 (Debug Score Breakdown).

---

## Acceptance Criteria Status

- [x] Search tool successfully returns results for 10+ test queries
- [x] Current ranking behavior documented: Tests/docs rank above implementations
- [x] Known failure cases confirmed: All 10 queries returned markdown headings as #1
- [x] MCP protocol integration verified: Tool callable, proper error handling
- [x] Debug mode confirmed functional: Returns results with scores
- [x] Documentation created: This file (`baseline-behavior.md`)

---

## Conclusions

### Key Findings

1. **Systematic Documentation Bias**: 100% of test queries (10/10) returned markdown headings as top result
2. **Implementation Buried**: Functions and classes consistently ranked below documentation (positions #3-10+)
3. **FTS Limitations Confirmed**: Pure term frequency scoring without semantic understanding fails for code search
4. **Exact Bonus Insufficient**: The +0.2 exact bonus doesn't overcome documentation term frequency advantage

### Implications for SEMRANK Phase 2

**Validated Need for Semantic Ranking**:
- Kind-based multipliers are essential (not optional)
- Documentation penalty (0.4×-0.6×) required to deprioritize headings
- Implementation boost (2.0×-2.5×) needed for func/class/module
- Exact match multiplier (3.0×) must use strict equality, not substring matching

**Success Criteria for Phase 2**:
- After semantic ranking: >90% of exact function searches return implementation as #1
- Implementation rank should average <3 (top 3 results)
- Documentation should rank below implementation for code-related queries

---

## Next Steps

1. **SEMRANK-1003**: Create test corpus with known structure (implementations + tests + docs)
2. **SEMRANK-1004**: Index test corpus for controlled testing
3. **SEMRANK-1005**: Establish baseline metrics (p50/p95 latency, top-1 accuracy)
4. **SEMRANK-2003**: Implement kind-based multipliers in SQL
5. **SEMRANK-2004**: Implement exact match detection with query normalization
6. **SEMRANK-3003**: Validate improvements against this baseline

---

**Document Version**: 1.0
**Ticket**: SEMRANK-0002
**Status**: Phase 0 Complete - Ready for Phase 1

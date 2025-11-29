# VECSRCH Tickets Review Report

**Review Date:** 2025-11-21
**Project:** VECSRCH - Core Vector Search Exposure
**Total Tickets Reviewed:** 4
**Overall Assessment:** ✅ **READY** (with minor adjustments)

---

## Executive Summary

The tickets created for VECSRCH are well-structured and align with the project plan. The scope is realistic, dependencies are clear, and the approach is pragmatic. There are **NO CRITICAL ISSUES** blocking execution.

**Key Findings:**
- ✅ All plan deliverables have corresponding tickets
- ✅ Dependencies are logical and achievable  
- ✅ Scope is appropriate (2-4 hours per ticket)
- ⚠️ One technical clarification needed (CLI command design decision)
- ⚠️ Missing: repo/worktree filtering parameters in CLI spec

---

## Critical Issues

**None identified.** All tickets are executable as written.

---

## Warnings

### Warning 1: CLI Command Design Ambiguity
**Tickets Affected:** VECSRCH-2002, VECSRCH-2003

**Concern:**  
The ticket mentions uncertainty about whether to create a new `VectorSearch` subcommand or extend the existing `Search` command with a `--vector` flag. The current codebase already has a `Search` command that does FTS.

**Current State:**
- Existing `Search` command (lines 160-173 in main.rs) performs FTS
- It already has flags for `--repo`, `--worktree`, `--query`, `--k`, `--debug`

**Potential Impact:**  
Creating a separate `VectorSearch` command duplicates parameter definitions and creates UX fragmentation. Users would need to learn two different search commands.

**Suggested Remediation:**  
**Option A (Recommended):** Extend the existing `Search` command with a `--mode` parameter:
```rust
Search {
    // ... existing params ...
    #[arg(long, default_value = "fts")]
    mode: String,  // "fts", "vector", or "hybrid"
}
```

**Option B:** Create separate `VectorSearch` command but ensure consistency with `Search` parameters.

**Recommendation:** Go with Option A for consistency with the Architecture doc's suggestion: "search with a flag". This maintains a unified search UX.

---

### Warning 2: Missing CLI Parameters
**Tickets Affected:** VECSRCH-2002, VECSRCH-2003

**Concern:**  
The tickets specify `query`, `k`, and `threshold` but don't mention `repo` and `worktree` parameters, which are critical for filtering search results.

**Potential Impact:**  
The search would fail or return results from all repos if repo filtering isn't implemented.

**Suggested Remediation:**  
Add to VECSRCH-2002 acceptance criteria:
- The command accepts `--repo` (required) and `--worktree` (optional) for filtering

---

### Warning 3: JSON Output Schema Not Specified
**Tickets Affected:** VECSRCH-2003

**Concern:**  
The ticket says "output schema is consistent and documented" but doesn't specify what fields the JSON should include beyond `chunk_id`, `score`, `content`, `file_path`.

**Potential Impact:**  
The MCP client (UNISRCH project) will need to consume this output. Schema mismatch could require rework.

**Suggested Remediation:**  
Specify the exact JSON schema in VECSRCH-2003:
```json
{
  "hits": [
    {
      "chunk_id": 123,
      "score": 0.92,
      "content": "...",
      "file_path": "src/main.rs",
      "start_line": 10,
      "end_line": 20,
      "symbol_name": "authenticate",
      "kind": "function"
    }
  ],
  "total": 10,
  "mode": "vector"
}
```

---

## Recommendations

### Recommendation 1: Add Query Embedding Generation
**Affected Tickets:** VECSRCH-2003

**Suggested Enhancement:**  
The ticket doesn't mention how the query text will be converted to a vector embedding. This is a critical step.

**Expected Benefit:**  
Clarify that the handler must:
1. Accept the query text
2. Call the embedding service to convert it to a vector
3. Pass the vector to `VectorExecutor::execute()`

Add to Technical Requirements:
- "Use `EmbeddingService::from_env()` to generate query embedding from the input string"

---

### Recommendation 2: Consider Error Handling for Missing Embeddings
**Affected Tickets:** VECSRCH-2003

**Suggested Enhancement:**  
If no embeddings exist in the database (i.e., `generate-embeddings` hasn't been run), the search will return empty results. This might confuse users.

**Expected Benefit:**  
Add user-friendly error message when no embeddings are found:
```
Error: Vector search requires embeddings to be generated first.
Run: crewchief-maproom generate-embeddings
```

---

### Recommendation 3: Simplify VECSRCH-2001
**Affected Tickets:** VECSRCH-2001

**Suggested Enhancement:**  
Looking at `src/lib.rs` (line 23) and `src/search/mod.rs` (line 183), **`VectorExecutor` is already exported publicly**:

```rust
// lib.rs
pub mod search;

// search/mod.rs
pub use vector::{VectorError, VectorExecutor};
```

This means `VectorExecutor` is already accessible via `crewchief_maproom::search::VectorExecutor`.

**Expected Benefit:**  
VECSRCH-2001 might be a no-op. Consider:
- Verifying this in the ticket (add test that imports it)
- Or merge VECSRCH-2001 into VECSRCH-2002 as a verification step

---

## Ticket Actions Required

### Tickets to Rework

**None.** All tickets are workable as-is, but consider the warnings above.

### Tickets to Defer

**None.**

### Tickets to Skip

**Potentially VECSRCH-2001** - See Recommendation 3. The types are already exposed, so this might just need verification rather than implementation.

### Tickets to Split

**None.** All tickets are appropriately scoped.

### Tickets to Merge

**Consider:** Merge VECSRCH-2001 into VECSRCH-2002 as a pre-requisite verification step, since the exposure work may already be done.

---

## Integration Assessment

**Overall Integration Health:** ✅ **Excellent**

**Key Integration Points:**
1. ✅ **VectorExecutor → CLI**: Types are already public in `search` module
2. ✅ **CLI → Database**: Existing DB connection pattern in `main.rs` can be reused
3. ✅ **JSON Output → UNISRCH**: Will need schema validation in UNISRCH project

**Risks to Existing Functionality:**  
- **Low Risk:** Adding a new command variant won't break existing `Search` behavior if we create `VectorSearch` 
- **No Risk:** Extending `Search` with a `--mode` flag is backward compatible (default to `"fts"`)

**Mitigation Recommendations:**
- Use default values for new parameters to maintain backward compatibility
- Add integration test (VECSRCH-3001) to verify existing `Search` still works

---

## Dependency Analysis

**Dependency Chain Validation:** ✅ **Valid**

```
VECSRCH-2001 (Expose Types)
    ↓
VECSRCH-2002 (CLI Definition)
    ↓
VECSRCH-2003 (Handler)
    ↓
VECSRCH-3001 (Testing)
```

**Problematic Dependencies:** None

**Sequencing Recommendations:**
- Execute in order: 2001 → 2002 → 2003 → 3001
- VECSRCH-2001 and 2002 could potentially run in parallel if 2001 is just verification

**Parallel Execution Opportunities:**
- If VECSRCH-2001 is confirmed as a no-op, skip it and go straight to 2002

---

## Quality & Feasibility Check

| Ticket | Scope (hrs) | Feasibility | AC Quality | Tech Reqs | Agent Match |
|:-------|:------------|:------------|:-----------|:----------|:------------|
| VECSRCH-2001 | 1-2 | ✅ High | ✅ Clear | ✅ Concrete | ✅ Rust Dev |
| VECSRCH-2002 | 2-3 | ✅ High | ⚠️ Ambiguous (mode) | ✅ Concrete | ✅ Rust Dev |
| VECSRCH-2003 | 3-4 | ✅ High | ⚠️ Missing schema | ⚠️ Missing embedding step | ✅ Rust Dev |
| VECSRCH-3001 | 2-3 | ⚠️ Medium (DB setup) | ✅ Clear | ✅ Concrete | ✅ QA Specialist |

**Overall Scope:** 8-12 hours (realistic for a 1-2 day sprint)

---

## Architecture Alignment

✅ **Fully Aligned**

- ✅ Uses existing `VectorExecutor` (no reinvention)
- ✅ Follows existing `clap` CLI patterns
- ✅ Outputs JSON (consistent with Architecture doc)
- ✅ Accepts database connection cost (per Architecture doc's acknowledged trade-off)
- ✅ Delegates to Rust core (DRY principle)

**Divergences:** None identified.

---

## Security Considerations

✅ **Adequately Addressed**

- ✅ Uses parameterized queries (VectorExecutor already does this)
- ✅ Database credentials from environment variables (existing pattern)
- ✅ No new security surface introduced

**Additional Recommendations:** None needed.

---

## Recommendations for Execution

### Suggested Execution Order

1. **VECSRCH-2001** (or skip if types already exposed)
2. **VECSRCH-2002** (CLI definition) - **Start here** if 2001 is verified as done
3. **VECSRCH-2003** (Handler implementation)
4. **VECSRCH-3001** (Integration testing)

### Risk Mitigation Strategies

1. **Before VECSRCH-2002:** Decide on CLI UX (new command vs. flag)
2. **Before VECSRCH-2003:** Define exact JSON output schema
3. **Before VECSRCH-3001:** Ensure test database has embeddings generated

### Key Checkpoints During Execution

- [ ] After VECSRCH-2002: Run `cargo check` to verify CLI compiles
- [ ] After VECSRCH-2003: Manually test with `crewchief-maproom vector-search --query "test"`
- [ ] After VECSRCH-3001: Verify existing `Search` command still works (regression check)

### Success Criteria for Project Completion

- [ ] `crewchief-maproom vector-search --query "auth" --repo crewchief --k 5` returns JSON results
- [ ] JSON output includes all required fields for MCP consumption
- [ ] Integration test passes in CI
- [ ] Existing FTS search functionality remains unaffected

---

## Final Recommendation

**Execution Decision:** ✅ **PROCEED**

The tickets are well-structured and ready for execution. Address the warnings (CLI design decision, missing parameters, JSON schema) either before starting or as part of the ticket work.

**Confidence Level:** 95%

**Estimated Timeline:** 1-2 days for a Rust developer

**Next Step:** Begin with VECSRCH-2002 (after confirming types are already exposed) and make the CLI design decision (new command vs. mode flag).

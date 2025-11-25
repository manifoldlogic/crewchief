# Project Review: SQLVEC_sqlite-vec-backend

**Review Date:** November 25, 2025
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

The proposal to replace the heavy Docker/Postgres requirement with an embedded SQLite+sqlite-vec solution is strategically sound and aligns perfectly with the "zero-dependency" goal. However, the plan underestimates the complexity of **SQL Dialect Migration** (Postgres -> SQLite) and the **Feature Gap** between `pgvector` (mature) and `sqlite-vec` (new). The architecture relies heavily on `sqlite-vec` being a drop-in replacement, which it is not.

## Critical Issues (Blockers)

### Issue 1: Vector Dimension Limits
**Severity:** Critical
**Category:** Architecture
**Description:** `sqlite-vec` (specifically `vec0`) historically had strict dimension limits or required compile-time configuration. `pgvector` handles 1536 (OpenAI) dimensions easily. We must ensure the vendored `sqlite-vec` build supports the dimensions required by our embedding providers (e.g., 768 for Nomic, 1536 for OpenAI).
**Impact:** If dimensions don't match, vectors cannot be stored.
**Required Action:** Add a specific validation step in Phase 1 to compile and test `sqlite-vec` with 1536-dim vectors.

### Issue 2: FTS Dialect Incompatibility
**Severity:** High
**Category:** Implementation
**Description:** Postgres `tsvector` / `websearch_to_tsquery` syntax is very different from SQLite `FTS5` syntax. The current plan mentions "Wire up FTS5" but doesn't detail the query translation layer.
**Impact:** Search queries will fail or return poor results if we just copy-paste logic.
**Required Action:** Create a specific ticket for "Query Dialect Adapter" to translate user queries into the appropriate SQL dialect.

## Reinvention & Duplication Analysis

### Missed Reuse Opportunities
**SQLx**: The project plans to use `rusqlite` (SQLite) and `tokio-postgres` (Postgres).
**Could Solve:** `sqlx` supports both via compile-time feature flags and generic executors, potentially reducing code duplication.
**Recommendation:** Evaluate if `sqlx` is a better fit than maintaining two separate implementations (`rusqlite` vs `tokio-postgres`). However, `sqlite-vec` is an extension, and loading extensions in `sqlx` might be trickier than `rusqlite`. Stick with `rusqlite` if extension loading is the priority, but acknowledge the code duplication.

## High-Risk Areas (Warnings)

### Risk 1: Concurrency & Locking
**Risk Level:** High
**Category:** Technical
**Description:** The Rust indexer uses `rayon` for parallel processing. SQLite allows only one writer. If the indexer tries to write from multiple threads simultaneously without a proper connection pool / queue, it will hit `SQLITE_BUSY`.
**Probability:** High
**Impact:** Indexing crashes or extreme slowness.
**Mitigation:** Enforce **Single Writer** pattern in the `VectorStore` implementation for SQLite, or use `r2d2` with `PRAGMA journal_mode=WAL` and immediate transactions.

### Risk 2: Build Complexity (C Extension)
**Risk Level:** Medium
**Category:** Maintenance
**Description:** Compiling C code in `build.rs` introduces cross-compilation headaches (e.g., building for Linux ARM64 from macOS).
**Probability:** Medium
**Impact:** Broken CI/CD pipelines.
**Mitigation:** Use the `cc` crate carefully and ensure CI tests cross-compilation.

## Gaps & Ambiguities

### Requirements Gaps
- **Migration Path**: How do existing users migrate from Postgres to SQLite? The plan implies a fresh start. Is there a `dump/restore` tool needed? (Assuming fresh start is okay for cache/index data).

### Technical Gaps
- **Extension Loading**: The exact mechanism to load `sqlite-vec` in `rusqlite` needs to be verified. `sqlite3_auto_extension` is unsafe/global.

## Scope & Feasibility Concerns

### Scope Creep Indicators
None. The scope is well-bounded (replace backend).

### Feasibility Challenges
- **Semantic Search Quality**: `sqlite-vec` might use different distance metrics or indexing algorithms (brute force vs HNSW) compared to `pgvector`'s IVFFlat/HNSW. Performance on large repos might degrade.

## Alignment Assessment

### MVP Discipline
**Rating:** Strong.
Focuses on removing Docker, which is the #1 user friction point.

### Pragmatism Score
**Rating:** Strong.
Vendoring the C file is pragmatic to avoid complex system dep requirements.

## Execution Readiness Checklist

- [ ] **Technical**: `build.rs` strategy needs a prototype.
- [ ] **Process**: Cross-compilation targets need validation.

## Recommendations

### Immediate Actions (Before Starting)
1.  **Prototype Build**: Create a tiny Rust project that successfully compiles and links `sqlite-vec` statically using `cc` and `rusqlite`. If this fails, the whole project is blocked.
2.  **Update Architecture**: Explicitly define the "Query Builder" abstraction to handle SQL dialect differences.

### Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes, but the build system risk is non-zero.

### Recommended Path Forward
**REVISE THEN PROCEED**: Address the FTS dialect and Concurrency/WAL concerns in the architecture document before creating tickets.

### Success Probability
Given current state: 75%
After recommended changes: 90%


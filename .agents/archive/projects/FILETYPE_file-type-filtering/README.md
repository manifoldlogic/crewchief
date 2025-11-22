# FILETYPE: File Type Filtering Implementation

**Status:** 📋 Planning Complete - Ready for Execution
**Priority:** 🟢 Medium - Quality of Life Improvement
**Timeline:** 1-2 days (13 hours estimated)
**Complexity:** Low - Completion of Existing Work

---

## Project Summary

Complete the partially-implemented `file_type` filter in the Maproom MCP search tool. The API surface and basic filtering logic exist, but the feature needs multi-extension support, input validation, comprehensive testing, and documentation to be production-ready.

**Current State:** 50% complete - foundation exists but needs polish
**Target State:** 100% complete - production-ready with confidence

---

## Problem Statement

Users need to narrow search results to specific programming languages or file types for improved search precision. Currently, the `file_type` filter parameter is advertised in the MCP API but only partially functional:

- ✅ Single extension filtering works (basic)
- ❌ Multi-extension support missing
- ❌ Input validation incomplete
- ❌ Edge cases not handled
- ❌ Comprehensive tests missing
- ❌ Documentation incomplete

**User Impact:** Cannot effectively filter searches by language (e.g., "show only TypeScript/JavaScript files").

---

## Proposed Solution

**Add multi-extension support:**
```typescript
// Single extension (currently works)
filters: {file_type: "ts"}  → Returns only .ts files

// Multiple extensions (new capability)
filters: {file_type: "ts,tsx,js"}  → Returns .ts OR .tsx OR .js files
```

**Implement input validation:**
- Case normalization ("TS" → "ts")
- Dot handling (".ts" → "ts")
- Whitespace tolerance (" ts , tsx " → ["ts", "tsx"])
- Character sanitization (alphanumeric only)
- Length limits (max 20 extensions, max 20 chars each)

**Write comprehensive tests:**
- 15 unit tests (parseFileTypeFilter)
- 10 integration tests (SQL generation)
- 5 E2E tests (real database queries)

**Update documentation:**
- Tool description with examples
- Error messages with helpful hints
- TypeScript types for autocomplete

---

## Key Findings

### What Exists

1. **MCP API Parameter** (`packages/maproom-mcp/src/index.ts:193`)
   - ✅ Documented in tool schema
   - ⚠️ Incomplete description (no multi-extension syntax)

2. **Filter Building Logic** (`packages/maproom-mcp/src/index.ts:458`)
   - ✅ Basic LIKE query implemented
   - ✅ Parameterized queries (SQL injection safe)
   - ❌ No multi-extension support
   - ❌ No input validation

3. **Unit Tests** (`packages/maproom-mcp/tests/search_tool.test.ts:94`)
   - ⚠️ Minimal tests (parameter presence only)
   - ❌ No edge case coverage
   - ❌ No E2E verification

### What's Missing

1. **Multi-Extension Parser**
   - Need `parseFileTypeFilter()` function
   - Comma-separated parsing
   - Case/dot/whitespace normalization

2. **SQL OR Clause Generation**
   - Single extension: `WHERE f.relpath LIKE '%.ts'`
   - Multiple: `WHERE (f.relpath LIKE '%.ts' OR f.relpath LIKE '%.tsx')`

3. **Comprehensive Testing**
   - Edge cases (empty input, invalid chars)
   - Integration (SQL correctness)
   - E2E (actual filtering)

4. **Documentation**
   - Usage examples
   - Syntax explanation
   - Error message improvements

---

## Technical Approach

### Architecture

**No database changes needed** - uses existing `files.relpath` column.

**TypeScript-only implementation** - no Rust binary changes.

**Layers:**
```
MCP Client → MCP Server (TypeScript) → PostgreSQL
              ↑
              parseFileTypeFilter()
              buildFilterClauses()
```

### Implementation Plan

**Phase 1: Core Implementation (4.5 hours)**
- Add `parseFileTypeFilter()` function
- Update `buildFilterClauses()` for multi-extension
- Add validation in `handleSearch()`
- Update tool description

**Phase 2: Comprehensive Testing (6 hours)**
- 15 unit tests
- 10 integration tests
- 5 E2E tests with database

**Phase 3: Documentation & Polish (2.5 hours)**
- Examples in tool description
- Error message improvements
- TypeScript types

**Total: 13 hours across 3 phases**

---

## Success Criteria

### Functional Requirements

- ✅ Single extension filter returns only matching files
- ✅ Multi-extension filter (comma-separated) returns union
- ✅ Case insensitive matching works (TS = ts)
- ✅ Empty filter handled gracefully (no error)
- ✅ Filter combines with other filters (recency_threshold, worktree_id)

### Quality Requirements

- ✅ All 30 tests pass
- ✅ 80%+ code coverage
- ✅ No SQL injection possible
- ✅ Performance impact <20%
- ✅ Error messages helpful and actionable

### Documentation Requirements

- ✅ Tool description includes file_type examples
- ✅ Multi-extension syntax documented
- ✅ Common use cases shown
- ✅ TypeScript types exported

---

## Agent Assignments

### Recommended Workflow

**Implementation:**
- `general-purpose` agent OR manual coding
- Straightforward TypeScript work

**Testing:**
- `unit-test-runner` for test execution
- `integration-tester` for E2E test creation

**Verification:**
- `verify-ticket` before commit
- Ensure all acceptance criteria met

**Commit:**
- `commit-ticket` for proper Conventional Commit

---

## Planning Documents

Comprehensive planning completed. See `planning/` directory:

- **[analysis.md](planning/analysis.md)** - Deep investigation of current implementation status
  - What exists vs. what's missing
  - Root cause analysis (why incomplete)
  - User belief vs. reality investigation

- **[architecture.md](planning/architecture.md)** - MVP-focused solution design
  - Component design (parseFileTypeFilter, buildFilterClauses)
  - Data flow and SQL query generation
  - Technology choices and trade-offs
  - Performance considerations

- **[quality-strategy.md](planning/quality-strategy.md)** - Pragmatic testing approach
  - 30-test suite breakdown (unit/integration/E2E)
  - Risk-based testing priorities
  - Coverage goals (80%+, not 100%)
  - Fast feedback (<5 second test suite)

- **[security-review.md](planning/security-review.md)** - Practical security assessment
  - SQL injection prevention (parameterized queries)
  - DoS mitigation (input limits)
  - No path traversal risk
  - Low security risk overall

- **[plan.md](planning/plan.md)** - High-level execution plan
  - Phase-by-phase workflow (3 phases, 13 hours)
  - Task breakdown with time estimates
  - Testing strategy and deployment plan
  - Success metrics and risk mitigation

---

## Risk Assessment

### Security

- **SQL Injection:** ✅ Mitigated (parameterized queries)
- **DoS:** ⚠️ Minor risk, mitigated (input limits)
- **Overall:** 🟢 Low risk

### Technical

- **Breaking Changes:** 🟢 None (additive only)
- **Performance:** 🟢 <20% overhead acceptable
- **Complexity:** 🟢 Low (simple string parsing + SQL)

### Project

- **Timeline Risk:** 🟢 Low (straightforward completion)
- **Scope Creep:** 🟢 Low (well-defined boundaries)
- **Testing Risk:** 🟢 Low (clear test plan)

---

## Out of Scope

Explicitly **not** included in this project:

- ❌ Database schema changes (indexed extension column)
- ❌ Rust binary modifications
- ❌ Regex-based filtering (keep simple)
- ❌ Language name mapping ("typescript" → "ts")
- ❌ Negation filtering ("exclude .test.ts files")
- ❌ File content-based type detection
- ❌ Performance optimization via indexed column

These are potential future enhancements, but not required for MVP.

---

## Next Steps

To begin execution:

1. **Create work tickets:**
   ```bash
   /create-project-tickets FILETYPE
   ```

2. **Start with Phase 1:**
   - FILETYPE-1001: Implement parseFileTypeFilter
   - FILETYPE-1002: Update buildFilterClauses
   - FILETYPE-1003: Add validation
   - FILETYPE-1004: Update documentation

3. **Test continuously:**
   - Write tests before/during implementation (TDD)
   - Run tests frequently for fast feedback

4. **Deploy when complete:**
   - All tests pass
   - All acceptance criteria met
   - No regressions in existing functionality

---

## Project Metadata

**Slug:** FILETYPE
**Name:** file-type-filtering
**Location:** `.agents/projects/FILETYPE_file-type-filtering/`
**Created:** 2025-11-19
**Status:** Planning complete, ready for ticket creation

**Related Documents:**
- Source: `.agents/reports/2025-11-18_maproom-mcp-projects-breakdown.md` (Project 5)
- Code: `packages/maproom-mcp/src/index.ts` (lines 188-461)
- Tests: `packages/maproom-mcp/tests/search_tool.test.ts`

---

## Summary

This project completes a partially-implemented feature to production quality. The work is well-scoped, low-risk, and delivers significant user value. With clear planning documents and acceptance criteria, this is ready for execution by any developer or agent.

**Estimated effort:** 1-2 days
**Value delivered:** High (improved search precision)
**Complexity:** Low (completion, not greenfield)
**Risk:** Low (isolated, tested, secure)

**Ready to ship!** 🚀

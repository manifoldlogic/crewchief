# MCPDB Tickets Review Report

**Review Date:** 2025-11-26
**Total Tickets Reviewed:** 6
**Overall Assessment:** Ready with Minor Adjustments
**Critical Issues:** 0
**Warnings:** 4
**Recommendations:** 6

## Executive Summary

The MCPDB ticket set is well-designed and ready for execution. All 6 tickets are properly scoped, have clear acceptance criteria, and follow correct dependency ordering. The tickets successfully address the critical issues identified in the project review (PostgreSQL dependencies, test isolation, graceful degradation).

**Strengths:**
- Clear dependency chain: MCPDB-1001 → 1002 → 1006 → 1003/1004 → 1005
- Good separation of concerns (URL parsing, daemon, handlers, tests, CI)
- Accurate code references matching actual codebase structure
- Appropriate graceful degradation strategy for SQLite limitations

**Minor Issues Found:**
- MCPDB-1003 has a syntax error in suggested implementation (missing import)
- MCPDB-1004 tests may need fixture data validation
- Vitest is used in tests but not listed in devDependencies
- MCPDB-1002 scope slightly underestimated for daemon.ts changes

**Recommendation:** Proceed to execution. Address warnings during implementation.

## Critical Issues

**None identified.** All tickets are executable as written.

## Warnings

### Warning 1: Missing `path` Import in MCPDB-1003

**Ticket:** MCPDB-1003
**Concern:** The suggested implementation uses `path.dirname()` but the import statement shows `import { join, resolve } from 'node:path'` - missing the default `path` import.

**Impact:** Minor - agent will catch this during implementation

**Suggested Fix:**
```typescript
// Change from:
const __dirname = path.dirname(fileURLToPath(import.meta.url))

// To:
import path from 'node:path'
// OR
import { dirname, join, resolve } from 'node:path'
const __dirname = dirname(fileURLToPath(import.meta.url))
```

**Priority:** Low - will be caught at TypeScript compilation

---

### Warning 2: Vitest Not in devDependencies

**Ticket:** MCPDB-1004
**Concern:** The integration tests use vitest (`import { describe, test, expect, beforeAll, afterAll, vi } from 'vitest'`) but the package.json shows vitest is not in devDependencies.

**Current devDependencies:**
```json
{
  "@types/node": "^20.10.5",
  "@types/pg": "^8.10.9",
  "typescript": "^5.3.3"
}
```

**Impact:** Tests won't run until vitest is added

**Suggested Fix:** Add vitest installation step to MCPDB-1004 or add a note to install vitest:
```bash
pnpm add -D vitest
```

**Priority:** Medium - will block test execution

---

### Warning 3: Fixture Data Assumptions

**Ticket:** MCPDB-1004
**Concern:** Tests assume fixture contains repo named 'maproom' and worktree 'main', but fixture contents are from MAPCLI and may have different structure.

**Code Reference:**
```typescript
const result = await handleSearch({
  repo: 'maproom', // or whatever repo is in fixture
  query: 'function',
  ...
})
```

**Impact:** Tests may fail if fixture has different repo/worktree names

**Suggested Fix:** Add a test setup step to query fixture for actual repo/worktree names, or add acceptance criteria to verify fixture contents first.

**Priority:** Medium - could cause test failures

---

### Warning 4: MCPDB-1002 Daemon Changes More Complex Than Indicated

**Ticket:** MCPDB-1002
**Concern:** The current `daemon.ts` (lines 55-59) throws error when `MAPROOM_DATABASE_URL` is not set. The ticket needs to handle the case where `resolveDatabaseConfig()` auto-detects SQLite from `~/.maproom/maproom.db` (no env var needed).

**Current Code:**
```typescript
if (!process.env.MAPROOM_DATABASE_URL) {
  throw new Error(
    'MAPROOM_DATABASE_URL environment variable is required for daemon operation'
  )
}
```

**Impact:** The "required" error must be removed when using auto-detection

**Suggested Fix:** Update acceptance criteria to explicitly mention removing the "required" error check, since `resolveDatabaseConfig()` handles auto-detection.

**Priority:** Medium - affects zero-config SQLite experience

## Recommendations

### Recommendation 1: Add Vitest Installation to MCPDB-1003 or 1004

**Area:** Test infrastructure
**Affected Tickets:** MCPDB-1003, MCPDB-1004
**Suggestion:** Add explicit step to ensure vitest is available:
- Either add `pnpm add -D vitest` to MCPDB-1003's implementation
- Or note in MCPDB-1004 that vitest must be installed
**Expected Benefit:** Prevents test execution failures

---

### Recommendation 2: Add Fixture Content Verification Step

**Area:** Test reliability
**Affected Tickets:** MCPDB-1004
**Suggestion:** Before running tests, verify fixture contains expected structure:
```typescript
beforeAll(async () => {
  // Verify fixture has expected structure
  const { handleStatus } = await import('../../src/index.js')
  // For PostgreSQL, check what repos exist
  // For SQLite, may need direct SQLite query or skip this check
})
```
**Expected Benefit:** Early failure with clear error if fixture is incompatible

---

### Recommendation 3: Export `resolveDatabaseConfig` Type Explicitly

**Area:** Type safety
**Affected Tickets:** MCPDB-1001
**Suggestion:** Export the `DatabaseConfig` type from the module so downstream code can type their variables:
```typescript
export type { DatabaseConfig }
// OR already exported as interface
```
**Expected Benefit:** Better TypeScript type inference in MCPDB-1002 and MCPDB-1006

---

### Recommendation 4: Add Manual Test Step to MCPDB-1006

**Area:** Verification
**Affected Tickets:** MCPDB-1006
**Suggestion:** Add explicit manual verification step after implementation:
1. Set `MAPROOM_DATABASE_URL=sqlite://<fixture-path>`
2. Run MCP server
3. Call status tool → verify degraded response
4. Call search tool → verify results with chunk_id=0
5. Check logs for warning about SQLite mode
**Expected Benefit:** Catches integration issues before automated tests

---

### Recommendation 5: Add Timeout Configuration to CI Job

**Area:** CI reliability
**Affected Tickets:** MCPDB-1005
**Suggestion:** Add timeout to CI job to prevent hangs:
```yaml
test-mcp-sqlite:
  name: MCP SQLite Tests (TypeScript)
  runs-on: ubuntu-latest
  timeout-minutes: 10  # Add this
```
**Expected Benefit:** Prevents CI from hanging on daemon issues

---

### Recommendation 6: Consider Adding Error Boundary Tests

**Area:** Error handling
**Affected Tickets:** MCPDB-1004
**Suggestion:** Add tests for additional error scenarios:
- SQLite file exists but is corrupted/empty
- SQLite file has wrong permissions
- Daemon times out during SQLite operation
**Expected Benefit:** More robust error handling coverage

## Ticket Actions Required

### Tickets to Rework
**None.** All tickets are executable as-is with minor fixes during implementation.

### Tickets to Defer
**None.** All tickets are appropriately scoped for this project.

### Tickets to Skip
**None.** All tickets are necessary for project completion.

### Tickets to Split
**None.** All tickets are appropriately sized (2-8 hours).

### Tickets to Merge
**None.** Current granularity is appropriate.

## Integration Assessment

### Overall Integration Health: Good

The tickets integrate well with each other and the existing codebase:

| Integration Point | Status | Notes |
|-------------------|--------|-------|
| resolve-database.ts → daemon.ts | ✅ Good | Clear interface via `resolveDatabaseConfig()` |
| daemon.ts → search.ts | ✅ Good | Existing pattern preserved |
| search.ts/index.ts → resolve-database.ts | ✅ Good | Simple import and type check |
| Test helpers → Tests | ✅ Good | Clear separation |
| Tests → CI | ✅ Good | Standard pnpm script pattern |

### Key Integration Points

1. **URL Resolution Chain:**
   - `MAPROOM_DATABASE_URL` → `resolveDatabaseConfig()` → `daemon.ts` → Rust daemon
   - Well-defined, no ambiguity

2. **Graceful Degradation Chain:**
   - `resolveDatabaseConfig().type === 'sqlite'` → skip PostgreSQL code → return degraded response
   - Clear conditional logic

3. **Test Infrastructure:**
   - Separate SQLite helpers → SQLite tests → CI job
   - No interference with existing PostgreSQL tests

### Risks to Existing Functionality

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking PostgreSQL path | Low | High | Conditional checks explicit; else branch unchanged |
| Breaking existing tests | Low | Medium | SQLite tests are separate files |
| Breaking daemon startup | Low | High | File validation only for SQLite type |

## Dependency Analysis

### Dependency Chain Validation: Valid

```
MCPDB-1001 (no deps)
    ↓
MCPDB-1002 (deps: 1001)
    ↓
MCPDB-1006 (deps: 1001, 1002)
    ↓
MCPDB-1003 (deps: 1006) ←──┬── Can run in parallel
MCPDB-1004 (deps: 1003, 1006) ←┘
    ↓
MCPDB-1005 (deps: 1004)
```

### Problematic Dependencies
**None.** Dependency chain is correct and achievable.

### Sequencing Recommendations
Execute in order: 1001 → 1002 → 1006 → 1003 → 1004 → 1005

MCPDB-1003 and MCPDB-1004 could theoretically run in parallel, but MCPDB-1004 depends on MCPDB-1003's helpers, so sequential is safer.

### Parallel Execution Opportunities
**Limited.** The dependency chain is largely sequential. Only potential parallel opportunity:
- After MCPDB-1006 completes, could start MCPDB-1003 (helpers) while manually testing 1006 changes

## Recommendations for Execution

### Suggested Ticket Execution Order
1. **MCPDB-1001** - URL parsing (foundation)
2. **MCPDB-1002** - Daemon integration (uses 1001)
3. **MCPDB-1006** - PostgreSQL handling (uses 1001, 1002)
4. **MCPDB-1003** - Test helpers (infrastructure)
5. **MCPDB-1004** - Integration tests (uses 1003)
6. **MCPDB-1005** - CI integration (final)

### Risk Mitigation Strategies

1. **After MCPDB-1001:** Run existing PostgreSQL tests to verify no regression
2. **After MCPDB-1002:** Manual test with SQLite URL to verify daemon starts
3. **After MCPDB-1006:** Manual test status + search tools with SQLite
4. **After MCPDB-1004:** Run full test suite (both SQLite and PostgreSQL)
5. **After MCPDB-1005:** Verify CI job passes on a test PR

### Key Checkpoints During Execution

| Checkpoint | After Ticket | Verification |
|------------|--------------|--------------|
| URL parsing works | MCPDB-1001 | Unit tests pass |
| Daemon starts with SQLite | MCPDB-1002 | Manual test |
| PostgreSQL still works | MCPDB-1006 | Existing tests pass |
| SQLite tests pass | MCPDB-1004 | `pnpm test:sqlite` |
| CI green | MCPDB-1005 | GitHub Actions pass |

### Success Criteria for Project Completion

1. ✅ `MAPROOM_DATABASE_URL=sqlite:///path/to/db` works
2. ✅ Auto-detection of `~/.maproom/maproom.db` works
3. ✅ Status tool returns degraded response for SQLite
4. ✅ Search tool returns results with chunk_id=0 for SQLite
5. ✅ Open tool retrieves code via daemon for SQLite
6. ✅ All PostgreSQL tests still pass (no regression)
7. ✅ SQLite tests pass in CI without PostgreSQL
8. ✅ CI job completes successfully

## Quality Standards Verification

- ✅ All tickets examined individually
- ✅ Cross-ticket interactions analyzed
- ✅ Integration with existing code assessed
- ✅ Dependencies validated
- ✅ Scope and feasibility checked
- ✅ Architecture alignment verified
- ✅ Critical issues clearly identified (none found)
- ✅ Actionable recommendations provided

## Conclusion

The MCPDB ticket set is **ready for execution**. All 6 tickets are well-defined, properly scoped, and correctly ordered. The minor warnings identified can be addressed during implementation without blocking progress.

**Recommended Action:** Proceed with `/work-on-project MCPDB`

---

**Reviewed By:** Tickets Review Agent
**Report Generated:** 2025-11-26

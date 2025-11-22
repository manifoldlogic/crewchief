# FILETYPE Project Ticket Index

**Project:** File Type Filtering Implementation
**Total Tickets:** 11
**Total Estimated Time:** 13 hours
**Status:** Ready for Execution

---

## Phase 1: Core Implementation (5 hours)

### FILETYPE-1001: Measure Performance Baseline
- **File:** `FILETYPE-1001_measure-performance-baseline.md`
- **Agent:** database-engineer
- **Time:** 30 minutes
- **Status:** ⏳ Not Started
- **Dependencies:** None
- **Summary:** Establish baseline query performance metrics before implementation

### FILETYPE-1002: Implement parseFileTypeFilter Function
- **File:** `FILETYPE-1002_implement-parse-file-type-filter.md`
- **Agent:** typescript-engineer
- **Time:** 1 hour
- **Status:** ⏳ Not Started
- **Dependencies:** None
- **Summary:** Add helper function to parse and normalize file type filter input

### FILETYPE-1003: Update buildFilterClauses for Multi-Extension Support
- **File:** `FILETYPE-1003_update-build-filter-clauses.md`
- **Agent:** typescript-engineer
- **Time:** 2 hours
- **Status:** ⏳ Not Started
- **Dependencies:** FILETYPE-1002
- **Summary:** Modify buildFilterClauses to handle multiple file extensions via OR clause

### FILETYPE-1004: Add Input Validation in handleSearch
- **File:** `FILETYPE-1004_add-input-validation.md`
- **Agent:** typescript-engineer
- **Time:** 1 hour
- **Status:** ⏳ Not Started
- **Dependencies:** FILETYPE-1002, FILETYPE-1003
- **Summary:** Add validation layer in handleSearch with helpful error messages

### FILETYPE-1005: Update MCP Tool Description with Examples
- **File:** `FILETYPE-1005_update-tool-description.md`
- **Agent:** typescript-engineer
- **Time:** 30 minutes
- **Status:** ⏳ Not Started
- **Dependencies:** None
- **Summary:** Update file_type parameter description in MCP tool schema

---

## Phase 2: Comprehensive Testing (6.5 hours)

### FILETYPE-2001: Add Unit Tests for parseFileTypeFilter
- **File:** `FILETYPE-2001_unit-tests-parse-file-type-filter.md`
- **Agent:** typescript-test-engineer
- **Time:** 2 hours
- **Status:** ⏳ Not Started
- **Dependencies:** FILETYPE-1002
- **Summary:** Add 15 unit tests to search_tool.test.ts covering all edge cases

### FILETYPE-2002: Create Integration Tests for SQL Generation
- **File:** `FILETYPE-2002_integration-tests-sql-generation.md`
- **Agent:** typescript-test-engineer
- **Time:** 2 hours
- **Status:** ⏳ Not Started
- **Dependencies:** FILETYPE-1002, FILETYPE-1003
- **Summary:** Create 10 integration tests in new tests/filters/file-type.int.test.ts

### FILETYPE-2003: Create E2E Tests with Database
- **File:** `FILETYPE-2003_e2e-tests-database.md`
- **Agent:** typescript-test-engineer
- **Time:** 2 hours
- **Status:** ⏳ Not Started
- **Dependencies:** FILETYPE-1002, FILETYPE-1003, FILETYPE-1004
- **Summary:** Create 5 E2E tests in new tests/filters/file-type.e2e.test.ts

### FILETYPE-2004: Performance Validation Against Baseline
- **File:** `FILETYPE-2004_performance-validation.md`
- **Agent:** database-engineer
- **Time:** 30 minutes
- **Status:** ⏳ Not Started
- **Dependencies:** FILETYPE-1001, All Phase 1 tickets
- **Summary:** Verify <20% performance overhead vs baseline measurement

---

## Phase 3: Documentation & Polish (1.5 hours)

### FILETYPE-3001: Update Documentation and README
- **File:** `FILETYPE-3001_update-documentation.md`
- **Agent:** documentation-specialist
- **Time:** 1 hour
- **Status:** ⏳ Not Started
- **Dependencies:** All Phase 1 and Phase 2 tickets
- **Summary:** Update package README with usage examples and complete JSDoc

### FILETYPE-3002: Final Integration Review and Cleanup
- **File:** `FILETYPE-3002_final-integration-review.md`
- **Agent:** typescript-engineer
- **Time:** 1 hour
- **Status:** ⏳ Not Started
- **Dependencies:** ALL previous tickets
- **Summary:** Quality gate - run all tests, verify no regressions, ensure consistency

---

## Execution Order (Recommended)

### Parallel Track 1: Performance + Core Implementation
1. FILETYPE-1001 (Performance Baseline) - 30 min
2. FILETYPE-1002 (parseFileTypeFilter) - 1 hour
3. FILETYPE-1003 (buildFilterClauses) - 2 hours
4. FILETYPE-1004 (Validation) - 1 hour
5. FILETYPE-1005 (Tool Description) - 30 min

### Parallel Track 2: Testing (after Track 1 completes)
6. FILETYPE-2001 (Unit Tests) - 2 hours
7. FILETYPE-2002 (Integration Tests) - 2 hours
8. FILETYPE-2003 (E2E Tests) - 2 hours
9. FILETYPE-2004 (Performance Validation) - 30 min

### Sequential Track 3: Polish (after all testing)
10. FILETYPE-3001 (Documentation) - 1 hour
11. FILETYPE-3002 (Final Review) - 1 hour

**Total Sequential Time:** 13 hours

---

## Key Features

### Test File Organization
- **Unit tests:** Extend `packages/maproom-mcp/tests/search_tool.test.ts`
- **Integration tests:** NEW `packages/maproom-mcp/tests/filters/file-type.int.test.ts`
- **E2E tests:** NEW `packages/maproom-mcp/tests/filters/file-type.e2e.test.ts`
- **New directory:** `packages/maproom-mcp/tests/filters/`

### Implementation Locations
- **parseFileTypeFilter:** `packages/maproom-mcp/src/index.ts` line ~430
- **buildFilterClauses:** `packages/maproom-mcp/src/index.ts` lines 458-461
- **handleSearch:** `packages/maproom-mcp/src/index.ts` handleSearch function
- **Tool schema:** `packages/maproom-mcp/src/index.ts` lines 188-196

### Quality Gates
- ✅ All 30 tests pass (15 unit + 10 integration + 5 E2E)
- ✅ Performance <20% overhead vs baseline
- ✅ No TypeScript errors
- ✅ Code style consistent with existing codebase
- ✅ Documentation complete

---

## Planning Document References

Each ticket references specific sections from:
- **README.md** - Project overview and success criteria
- **planning/analysis.md** - Current state investigation
- **planning/architecture.md** - Implementation Specification (exact code)
- **planning/plan.md** - Phase breakdown and task details
- **planning/quality-strategy.md** - Test file organization
- **planning/security-review.md** - Security considerations

---

## Workflow Commands

```bash
# Review ticket quality
/review-tickets FILETYPE

# Execute all tickets sequentially
/work-on-project FILETYPE

# Execute single ticket
/single-ticket FILETYPE-1001

# Verify ticket completion
/verify-ticket FILETYPE-1001

# Commit completed work
/commit-ticket FILETYPE-1001
```

---

## Success Metrics

### Functional
- ✅ Single extension filter: 100% correct results
- ✅ Multi-extension filter: 100% correct results
- ✅ Case insensitive matching works
- ✅ Empty filter handled gracefully

### Quality
- ✅ 80%+ test coverage
- ✅ All 30 tests passing
- ✅ No regressions in existing functionality

### Performance
- ✅ <20% overhead vs baseline (e.g., 100ms → <120ms)

---

**Project Status:** ✅ Ready for Execution
**Next Step:** `/work-on-project FILETYPE` or `/single-ticket FILETYPE-1001`

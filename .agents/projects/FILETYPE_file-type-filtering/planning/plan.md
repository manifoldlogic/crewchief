# Execution Plan: File Type Filtering

**Project:** FILETYPE - File Type Filtering
**Date:** 2025-11-19
**Timeline:** 1-2 days (8-16 hours total)
**Status:** Ready for execution

---

## Project Overview

**Goal:** Complete the partially-implemented file_type filter to production quality.

**Scope:**
- Add multi-extension support (comma-separated)
- Implement input validation and sanitization
- Write comprehensive test suite
- Update documentation and error messages

**Out of Scope:**
- Database schema changes
- Rust binary modifications
- Advanced features (regex, language mappings, negation)

---

## Execution Phases

### Phase 1: Core Implementation (4-6 hours)

**Deliverable:** `parseFileTypeFilter()` and `buildFilterClauses()` fully functional

#### Task 1.0: Measure Performance Baseline (Pre-Implementation)

**Purpose:** Establish baseline query performance BEFORE implementing file_type filter to validate the "Performance impact <20%" success criterion.

**Method:**
1. Run 10 search queries without any filters on medium-sized repo (5k-10k files)
2. Measure average query time (exclude outliers)
3. Document baseline in test results
4. Calculate acceptable threshold (baseline + 20%)

**Example commands:**
```bash
# Run baseline performance test
cd packages/maproom-mcp
node bin/cli.cjs search "authentication" --repo crewchief --mode hybrid

# Measure query time (look for "Query time: Xms" in output)
# Repeat 10 times, calculate average
```

**Expected baseline:**
- Small repo (<1k files): ~50ms
- Medium repo (5k-10k files): ~100ms
- Large repo (50k+ files): ~200ms

**Performance threshold calculation:**
- If baseline = 100ms, threshold = 120ms (100ms + 20%)
- If baseline = 50ms, threshold = 60ms (50ms + 20%)
- If baseline = 200ms, threshold = 240ms (200ms + 20%)

**Acceptance:**
- ✅ Baseline measurement documented (average of 10 runs)
- ✅ Acceptable threshold calculated (baseline × 1.2)
- ✅ Test repo size documented (file count)
- ✅ Measurement method reproducible

**Deliverable:**
- Baseline metrics file: `packages/maproom-mcp/tests/performance-baseline.md`
- Include: repo size, avg query time, threshold, measurement date

**Time:** 30 minutes

**Note:** This task establishes objective performance criteria. Without it, "Performance impact <20%" is unmeasurable.

---

#### Task 1.1: Implement parseFileTypeFilter

**Location:** `packages/maproom-mcp/src/index.ts`

**Code to add:**
```typescript
/**
 * Parse and normalize file type filter input
 */
function parseFileTypeFilter(input: string): string[] {
  // Type check and length limit
  if (typeof input !== 'string' || input.length > 500) {
    return []
  }

  return input
    .split(',')
    .map(ext => ext.trim())
    .map(ext => ext.replace(/^\./, ''))
    .map(ext => ext.toLowerCase())
    .filter(ext => /^[a-z0-9]{1,20}$/.test(ext))
}
```

**Acceptance:**
- ✅ Parses comma-separated extensions
- ✅ Normalizes case to lowercase
- ✅ Strips leading dots
- ✅ Filters invalid characters
- ✅ Limits extension length

**Time:** 1 hour

---

#### Task 1.2: Update buildFilterClauses for Multi-Extension

**Location:** `packages/maproom-mcp/src/index.ts:442-474`

**Current code:**
```typescript
if (filters.file_type) {
  args.push(`%.${filters.file_type}`)
  clauses += ` AND f.relpath LIKE $${args.length}`
}
```

**New code:**
```typescript
if (filters.file_type) {
  const extensions = parseFileTypeFilter(filters.file_type)

  if (extensions.length > 20) {
    throw new Error(
      `Too many file extensions: ${extensions.length} (maximum 20 allowed). ` +
      `Use broader filter or multiple searches.`
    )
  }

  if (extensions.length === 0) {
    // Empty filter - ignore (search all files)
    // Could add to debug/hint later
  } else if (extensions.length === 1) {
    // Single extension
    args.push(`%.${extensions[0]}`)
    clauses += ` AND f.relpath LIKE $${args.length}`
  } else {
    // Multiple extensions - OR clause
    const conditions = extensions.map(ext => {
      args.push(`%.${ext}`)
      return `f.relpath LIKE $${args.length}`
    }).join(' OR ')
    clauses += ` AND (${conditions})`
  }
}
```

**Acceptance:**
- ✅ Single extension generates simple LIKE clause
- ✅ Multi-extension generates OR clause
- ✅ Empty input ignored gracefully
- ✅ >20 extensions rejected with error

**Time:** 2 hours

---

#### Task 1.3: Add Validation in handleSearch

**Location:** `packages/maproom-mcp/src/index.ts:609-860` (handleSearch function)

**Code to add:**
```typescript
// After mode validation, before search execution
if (filters.file_type) {
  try {
    const extensions = parseFileTypeFilter(filters.file_type)

    if (extensions.length === 0) {
      // Warn user but don't fail
      hint = hint || ''
      hint += `\n⚠️  file_type filter "${filters.file_type}" produced no valid extensions. Searching all files.`
    }

    if (extensions.length > 20) {
      return {
        hits: [],
        error: 'Too many file extensions',
        hint: `file_type filter has ${extensions.length} extensions (maximum 20 allowed).\n\nTry: filters: {file_type: "ts,tsx,js"} instead of listing 50+ extensions`,
        suggestion: 'Use broader filter or multiple searches'
      }
    }
  } catch (error: any) {
    return {
      hits: [],
      error: 'Invalid file_type filter',
      hint: error.message
    }
  }
}
```

**Acceptance:**
- ✅ Empty input warning added to hint
- ✅ Too many extensions returns error
- ✅ Invalid input caught and reported

**Time:** 1 hour

---

#### Task 1.4: Update Tool Description

**Location:** `packages/maproom-mcp/src/index.ts:193`

**Current:**
```typescript
file_type: {
  type: 'string',
  description: 'Filter by file extension (e.g., "ts", "rs", "md")'
}
```

**New:**
```typescript
file_type: {
  type: 'string',
  description: 'Filter by file extension(s). Single: "ts" or multiple: "ts,tsx,js" (comma-separated, max 20 extensions)'
}
```

**Also update main tool description around line 166:**
```typescript
FILTERS: Narrow by file_type, recency, repo_id, worktree_id

Examples:
  filters: {file_type: "ts"}          → Only TypeScript files
  filters: {file_type: "ts,tsx,js"}   → TypeScript or JavaScript files
  filters: {file_type: "md,mdx"}      → Markdown documentation
```

**Acceptance:**
- ✅ Description explains multi-extension syntax
- ✅ Examples show common use cases
- ✅ Limits documented

**Time:** 0.5 hours

---

**Phase 1 Total Time:** 4.5 hours

**Phase 1 Checkpoint:**
```bash
# Manual test
node bin/cli.cjs  # Start MCP server
# In client:
search({
  repo: "crewchief",
  query: "authentication",
  filters: {file_type: "ts,tsx,js"}
})
# Should return only .ts, .tsx, .js files
```

---

### Phase 2: Comprehensive Testing (4-6 hours)

**Deliverable:** 30 tests passing, 80%+ coverage

#### Task 2.1: Unit Tests for parseFileTypeFilter

**Location:** `packages/maproom-mcp/tests/search_tool.test.ts`

**Tests to add:**
```typescript
describe('parseFileTypeFilter - Unit Tests', () => {
  // 15 tests covering:
  // - Basic parsing (single, multi)
  // - Case normalization
  // - Whitespace handling
  // - Dot stripping
  // - Empty input
  // - Edge cases (trailing comma, etc.)
})
```

**Reference:** See quality-strategy.md for full test list

**Acceptance:**
- ✅ 15 unit tests pass
- ✅ 100% coverage of parseFileTypeFilter
- ✅ All edge cases handled

**Time:** 2 hours

---

#### Task 2.2: Integration Tests for buildFilterClauses

**Location:** `packages/maproom-mcp/tests/search_tool.test.ts`

**Tests to add:**
```typescript
describe('buildFilterClauses - Integration Tests', () => {
  // 10 tests covering:
  // - Single extension SQL generation
  // - Multi-extension OR clause
  // - Parameterization safety
  // - Filter combination
  // - Edge cases
})
```

**Acceptance:**
- ✅ 10 integration tests pass
- ✅ SQL generation verified
- ✅ Parameterization confirmed

**Time:** 2 hours

---

#### Task 2.3: E2E Tests with Database

**Location:** `packages/maproom-mcp/tests/search_tool.integration.test.ts` (new file)

**Setup:**
```typescript
// Create test database with known data
beforeAll(async () => {
  // Insert test files with various extensions
  await testDb.query(`
    INSERT INTO maproom.files (relpath, ...)
    VALUES
      ('src/auth.ts', ...),
      ('src/auth.tsx', ...),
      ('src/utils.js', ...),
      ('README.md', ...),
      ('package.json', ...)
  `)
})
```

**Tests:**
```typescript
describe('File Type Filter - E2E Tests', () => {
  // 5 tests covering:
  // - Single extension returns only matching files
  // - Multi-extension returns union
  // - Case insensitive matching
  // - Empty filter behavior
  // - Performance acceptable
})
```

**Acceptance:**
- ✅ 5 E2E tests pass
- ✅ Real database queries work
- ✅ Filter actually filters results

**Time:** 2 hours

---

**Phase 2 Total Time:** 6 hours

**Phase 2 Checkpoint:**
```bash
pnpm test
# All 30 tests pass
# Coverage report shows 80%+
```

---

### Phase 3: Documentation & Polish (2-3 hours)

**Deliverable:** Feature documented, error messages helpful, ready to ship

#### Task 3.1: Update MCP Tool Description

**Location:** `packages/maproom-mcp/src/index.ts:118-204`

**Add detailed examples:**
```typescript
// In search tool description around line 166
FILTER EXAMPLES:

Single file type:
  filters: {file_type: "ts"}
  → Returns only TypeScript files (.ts)

Multiple file types:
  filters: {file_type: "ts,tsx,js"}
  → Returns TypeScript or JavaScript files

Documentation search:
  filters: {file_type: "md,mdx"}
  → Returns only Markdown files

Combine with other filters:
  filters: {
    file_type: "rs",
    recency_threshold: "7 days"
  }
  → Recent Rust files only

FILTER SYNTAX:
- Comma-separated for multiple types
- Case insensitive: "TS" = "ts"
- With or without dot: ".ts" = "ts"
- Max 20 extensions per filter
```

**Acceptance:**
- ✅ Examples clear and practical
- ✅ Syntax documented
- ✅ Limits explained

**Time:** 1 hour

---

#### Task 3.2: Improve Error Messages

**Location:** Throughout `handleSearch` and `buildFilterClauses`

**Error message checklist:**

```typescript
// Too many extensions
error: 'Too many file extensions'
hint: `file_type filter has ${count} extensions (maximum 20).

Common solutions:
1. Use broader filter: filters: {file_type: "ts,tsx,js"}
2. Filter by language category: filter: "code"
3. Run multiple searches for different file types`

// Invalid extension format
error: 'Invalid file extension format'
hint: `file_type must be alphanumeric characters only.

Valid:   filters: {file_type: "ts,tsx,js"}
Invalid: filters: {file_type: "ts; DROP TABLE"}

Extensions: 1-20 characters, letters and numbers only`

// No results with file_type filter
hint: `No results found for "${query}" in ${extensions.join(', ')} files.

Suggestions:
- Try broader file types: remove some extensions
- Check if files are indexed: run status tool
- Try without filter to see all results`
```

**Acceptance:**
- ✅ Error messages actionable
- ✅ Hints guide users to solutions
- ✅ Examples show correct usage

**Time:** 1 hour

---

#### Task 3.3: Add TypeScript Types

**Location:** `packages/maproom-mcp/src/types.ts` (if exists, else in index.ts)

**Types to add:**
```typescript
export interface SearchFilters {
  repo_id?: number
  worktree_id?: number
  file_type?: string  // Comma-separated extensions
  recency_threshold?: string  // PostgreSQL interval
}

export interface SearchParams {
  repo: string
  worktree?: string | null
  query: string
  k?: number
  mode?: 'fts' | 'vector' | 'hybrid'
  filter?: 'all' | 'code' | 'docs' | 'config'
  filters?: SearchFilters
  debug?: boolean
}
```

**Acceptance:**
- ✅ TypeScript types defined
- ✅ Autocomplete works in IDE
- ✅ Type errors caught at compile time

**Time:** 0.5 hours

---

**Phase 3 Total Time:** 2.5 hours

**Phase 3 Checkpoint:**
```bash
# Type check passes
pnpm typecheck

# Documentation readable
cat src/index.ts | grep -A 20 "FILTER EXAMPLES"

# Error messages tested manually
search({filters: {file_type: Array(30).fill('ts').join(',')}})
# Should show helpful error
```

---

## Testing Strategy

### Unit Test Execution

```bash
# Run unit tests only
pnpm test -- --grep "parseFileTypeFilter"

# Expected: 15 tests pass in <1 second
```

### Integration Test Execution

```bash
# Run integration tests
pnpm test -- --grep "buildFilterClauses"

# Expected: 10 tests pass in <2 seconds
```

### E2E Test Execution

```bash
# Setup test database
export TEST_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom_test"

# Run E2E tests
pnpm test -- --grep "File Type Filter - E2E"

# Expected: 5 tests pass in <2 seconds
```

### Full Test Suite

```bash
# Run all tests
pnpm test

# Expected: 30+ tests pass in <5 seconds
# Coverage: 80%+ on index.ts
```

---

## Deployment Plan

### Pre-Deployment Checklist

**Code Quality:**
- ✅ All tests pass (unit + integration + E2E)
- ✅ TypeScript compiles without errors
- ✅ ESLint passes (no new warnings)
- ✅ Code reviewed (self or peer)

**Functionality:**
- ✅ Single extension filter works
- ✅ Multi-extension filter works
- ✅ Empty filter handled gracefully
- ✅ Error messages helpful

**Documentation:**
- ✅ Tool description updated with examples
- ✅ Error messages explain how to fix
- ✅ TypeScript types exported

**Security:**
- ✅ Parameterized queries used
- ✅ Input validation implemented
- ✅ Extension count limited (max 20)
- ✅ No SQL injection possible

---

### Deployment Steps

**Step 1: Build**
```bash
cd packages/maproom-mcp
pnpm build

# Verify build output
ls dist/index.js
```

**Step 2: Deploy MCP Server**
```bash
# Update MCP server (deployment method varies)
# Example: Docker rebuild, npm publish, etc.

# Verify MCP server starts
node dist/index.js
# Should see "server-info" log
```

**Step 3: Smoke Test**
```bash
# Test search with file_type filter
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "search",
      "arguments": {
        "repo": "crewchief",
        "query": "authentication",
        "filters": {"file_type": "ts,tsx,js"}
      }
    }
  }'

# Expected: JSON response with filtered results
```

**Step 4: Monitor**
```bash
# Check logs for errors
tail -f /var/log/maproom-mcp.log

# Watch for:
# - No crashes
# - No SQL errors
# - Filters working as expected
```

---

### Rollback Plan

**If issues detected:**

1. **Revert code changes:**
   ```bash
   git revert <commit-hash>
   pnpm build
   # Redeploy
   ```

2. **No data loss risk:**
   - No database changes to rollback
   - No data migration to undo

3. **Verify rollback:**
   ```bash
   # Test search still works
   search({repo: "crewchief", query: "test"})
   # Should work without file_type filter
   ```

---

## Agent Assignments

### Recommended Agents

**Phase 1 (Implementation):**
- **Agent:** `general-purpose` or manual implementation
- **Reason:** Straightforward TypeScript coding, no specialized knowledge needed

**Phase 2 (Testing):**
- **Agent:** `unit-test-runner` for test execution
- **Agent:** `integration-tester` for E2E test creation
- **Reason:** Specialized testing workflows

**Phase 3 (Documentation):**
- **Agent:** `general-purpose` or manual
- **Reason:** Documentation writing, no specialized agent needed

---

## Success Metrics

### Functional Metrics

**Must achieve:**
- ✅ Single extension filter: 100% correct results
- ✅ Multi-extension filter: 100% correct results
- ✅ Case insensitive: Works as expected
- ✅ Empty filter: No error, searches all files

**Should achieve:**
- ✅ Filter combination: Works with other filters
- ✅ Error handling: Clear, actionable messages
- ✅ Performance: <20% overhead vs baseline (measured in Task 1.0)
  - Example: If baseline = 100ms, file_type filter must be <120ms
  - Measured on same repo/query as baseline measurement

---

### Quality Metrics

**Test coverage:**
- ✅ Unit tests: 100% of parseFileTypeFilter
- ✅ Integration tests: 85% of buildFilterClauses
- ✅ E2E tests: 70% of handleSearch (filter path)
- ✅ Overall: 80%+ coverage

**Code quality:**
- ✅ No TypeScript errors
- ✅ No ESLint warnings
- ✅ Follows existing code style

---

### User Experience Metrics

**Post-deployment (measure after 1 week):**
- Filter usage rate (how often file_type used)
- Error rate (invalid input frequency)
- Performance impact (query time delta)

**Targets:**
- Usage: >10% of searches use file_type filter
- Errors: <5% of filter uses result in error
- Performance: <20% query time increase

---

## Risk Mitigation

### High-Risk Areas

**Risk 1: SQL OR clause too complex**
- **Mitigation:** Limit to 20 extensions
- **Test:** Performance test with 20 extensions

**Risk 2: User confusion about syntax**
- **Mitigation:** Clear examples in documentation
- **Test:** Manual UX review of tool description

**Risk 3: Breaking existing functionality**
- **Mitigation:** Run full test suite before/after
- **Test:** All existing search tests must pass

---

### Contingency Plans

**If Phase 1 takes too long (>6 hours):**
- Ship single-extension only (defer multi-extension)
- Add multi-extension in Phase 2

**If Phase 2 tests fail:**
- Fix bugs before Phase 3
- Don't skip tests to save time

**If Phase 3 runs over time:**
- Ship with basic documentation
- Improve docs in follow-up PR

---

## Timeline Summary

**Total estimated time:** 13 hours

| Phase | Tasks | Time | Status |
|-------|-------|------|--------|
| Phase 1 | Core Implementation | 4.5 hours | ⏸️ Pending |
| Phase 2 | Testing | 6 hours | ⏸️ Pending |
| Phase 3 | Documentation | 2.5 hours | ⏸️ Pending |
| **Total** | **All tasks** | **13 hours** | **Ready to start** |

**Expected completion:** 1-2 days (depending on developer availability)

---

## Next Steps

**To begin execution:**

1. Create work tickets from this plan (use `/create-project-tickets FILETYPE`)
2. Assign to implementation agent or developer
3. Follow phase-by-phase workflow
4. Run tests continuously (TDD approach)
5. Deploy when all acceptance criteria met

**First ticket to create:**
- FILETYPE-1001: Implement parseFileTypeFilter function
- FILETYPE-1002: Update buildFilterClauses for multi-extension
- FILETYPE-1003: Add validation in handleSearch
- FILETYPE-1004: Update tool description
- ... (continue for all tasks)

---

## Conclusion

This plan provides a clear, executable path from current state (50% complete) to production-ready feature (100% complete). Each phase has specific deliverables, acceptance criteria, and time estimates.

**Ready to execute:** ✅ Yes
**Blocking dependencies:** None
**Risks identified:** All mitigated
**Success probability:** High (straightforward completion task)

**Let's ship it!** 🚀

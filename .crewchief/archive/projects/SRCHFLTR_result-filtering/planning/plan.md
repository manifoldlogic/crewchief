# Plan: Result Filtering

**Project:** SRCHFLTR - Result Filtering
**Date:** 2025-12-13
**Timeline:** 2-3 days (14-18 hours)
**Phases:** 3

---

## Overview

This document outlines the phased execution plan for implementing client-side result filtering, sorting, and pagination for Maproom search results. The implementation is pure TypeScript (no Rust/database changes), focused on MVP functionality, and fully backward compatible.

**Key Strategy:** Build incrementally with continuous testing. Each phase delivers value independently.

---

## Phases

### Phase 1: Core Filtering Foundation (1 day / 6-8 hours)

**Objective:** Implement FilterableSearchResult class with basic filtering capabilities.

**Deliverables:**
- `FilterableSearchResult` class in daemon-client
- `FilterCriteria` and `SortField` type definitions
- `filter()` method implementation (kind, file_type, path substring, score range)
- Basic unit tests (filter operations)
- Type exports and package integration

**Tickets:**
- SRCHFLTR-1001: Create FilterableSearchResult class skeleton
- SRCHFLTR-1002: Implement filter() method (kind, file_type, path substring, score range, custom)
- SRCHFLTR-1003: Add filter type definitions
- SRCHFLTR-1004: Write unit tests for filtering
- SRCHFLTR-1005: Export types from daemon-client index

**Agent Assignments:**
- typescript-engineer: Implement FilterableSearchResult class
- typescript-engineer: Add type definitions
- unit-test-runner: Execute filter unit tests
- verify-ticket: Verify acceptance criteria
- commit-ticket: Create commits with conventional commit messages

**Dependencies:**
- None (foundation work, no external dependencies)

**Success Criteria:**
- ✅ All filter criteria work (kind, file_type, path substring, score)
- ✅ Immutable operations (returns new instance)
- ✅ TypeScript compiles without errors
- ✅ Unit tests pass (80%+ coverage for filter logic)
- ✅ Zero new dependencies added

**Validation:**
```typescript
import { FilterableSearchResult } from '@crewchief/daemon-client'

const result = new FilterableSearchResult(mockSearchResult)
const filtered = result.filter({kind: "function"})
expect(filtered.hits.length).toBeLessThan(result.hits.length)
expect(filtered.hits.every(h => h.kind === "function")).toBe(true)
```

---

### Phase 2: Sorting and Pagination (0.5 days / 3-4 hours)

**Objective:** Add sorting and pagination capabilities to FilterableSearchResult.

**Deliverables:**
- `sortBy()` method implementation (all fields + order)
- `slice()` method implementation (pagination)
- Unit tests for new methods
- Integration tests for chained operations

**Tickets:**
- SRCHFLTR-2001: Implement sortBy() method
- SRCHFLTR-2002: Implement slice() method
- SRCHFLTR-2003: Write integration tests for chaining

**Agent Assignments:**
- typescript-engineer: Implement sorting/pagination methods
- unit-test-runner: Execute sorting/pagination tests
- verify-ticket: Verify chaining works correctly
- commit-ticket: Create commits

**Dependencies:**
- Phase 1 complete (FilterableSearchResult class exists)

**Success Criteria:**
- ✅ All sort fields work (score, relpath, symbol_name, start_line, kind)
- ✅ Both ascending and descending order work
- ✅ Pagination works (slice with start/end)
- ✅ Aggregations count correctly
- ✅ Chained operations work (filter → sort → slice)
- ✅ Performance <2ms for chained operations on 100 results

**Validation:**
```typescript
const result = new FilterableSearchResult(mockSearchResult)
const filtered = result
  .filter({kind: "function"})
  .sortBy("relpath", "asc")
  .slice(0, 10)

expect(filtered.hits.length).toBe(10)
expect(filtered.hits[0].kind).toBe("function")
// Verify sorted ascending by relpath
expect(filtered.hits[0].file_path < filtered.hits[1].file_path).toBe(true)
```

---

### Phase 3: Documentation and Validation (0.5 day / 3-4 hours)

**Objective:** Complete documentation and validate end-to-end functionality.

**Deliverables:**
- Updated daemon-client README
- TSDoc comments complete
- E2E tests (real search + filtering)
- Adoption guide for consumers

**Tickets:**
- SRCHFLTR-3001: Update daemon-client README with examples
- SRCHFLTR-3002: Add comprehensive TSDoc comments
- SRCHFLTR-3003: E2E integration tests with real daemon

**Agent Assignments:**
- typescript-engineer: Integrate with MCP server
- documentation-agent: Update README and TSDoc
- unit-test-runner: Execute E2E and performance tests
- verify-ticket: Verify all acceptance criteria
- commit-ticket: Create final commits

**Dependencies:**
- Phase 1 complete (FilterableSearchResult exists)
- Phase 2 complete (all methods implemented)

**Success Criteria:**
- ✅ Documentation complete (README + TSDoc)
- ✅ Examples cover common use cases
- ✅ E2E tests pass (real daemon + filtering)
- ✅ No breaking changes (backward compatibility verified)
- ✅ Type sync boundaries validated (no Rust changes needed)

**Validation:**
```typescript
// E2E test with real daemon
const daemon = new DaemonClient()
const rawResult = await daemon.search({query: "auth", repo: "crewchief"})
const filterable = new FilterableSearchResult(rawResult)

const functions = filterable
  .filter({kind: "function", file_type: "ts"})
  .sortBy("relpath")
  .slice(0, 10)

expect(functions.hits.length).toBeLessThanOrEqual(10)
expect(functions.hits.every(h => h.kind === "function")).toBe(true)
expect(functions.hits.every(h => h.file_path.endsWith(".ts"))).toBe(true)
```

---

## Dependencies

### Internal

- **Phase 2 depends on Phase 1**: Cannot implement sortBy/slice without FilterableSearchResult class
- **Phase 3 depends on Phase 1 + 2**: Integration requires complete implementation

### External

- **daemon-client package**: Existing (no changes needed, pure addition)
- **TypeScript**: Already configured (^5.0.0, no version changes)
- **Zero new dependencies**: Uses only native JavaScript/TypeScript features

### Cross-Project

- **Independent of SRCHTRN**: Can proceed in parallel (different layers)
- **Compatible with FILETYPE**: Uses same result structures
- **No Rust changes**: Pure TypeScript, no daemon modifications

---

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Performance regression | Low | Medium | Benchmark all operations, <5ms budget, optimize if needed |
| Breaking existing clients | Low | High | Additive API only, no changes to existing interfaces, backward compat tests |
| Type sync issues | Low | Medium | No Rust changes needed, pure TypeScript wrapper, validation tests |
| Memory leaks from large results | Low | Low | Immutable operations, no caching in MVP, rely on GC, document size limits |
| Over-engineering | Low | Low | **MITIGATED**: Reduced to 11 tickets, 3 core methods only, deferred aggregations/helpers |

---

## Testing Strategy

### Unit Tests (80%+ coverage)

**Filter tests:**
- Each filter criteria (kind, file_type, path substring, score range, custom)
- Edge cases (empty results, null values, invalid inputs)
- Multiple criteria (AND logic)
- Error handling (graceful degradation for invalid scores)

**Sort tests:**
- Each sort field (score, relpath, symbol_name, start_line, kind)
- Both orders (asc, desc)
- Null/undefined handling

**Pagination tests:**
- Slice with start only
- Slice with start and end
- Out of bounds handling

### Integration Tests

**Chaining:**
- filter → sort → slice
- Multiple filters combined
- Sort → filter (order matters)

**Immutability:**
- Original result unchanged after operations
- Each operation returns new instance

**Performance:**
- Filter 100 results <1ms
- Sort 100 results <1ms
- Chain 3 operations <2ms

### E2E Tests

**Real daemon:**
- Search + filter + sort + slice
- Verify results match expectations
- No data corruption

**Backward compatibility:**
- Existing code continues working
- Optional FilterableSearchResult usage

---

## Success Metrics

### Functional

- ✅ Filter by kind works
- ✅ Filter by file_type works
- ✅ Filter by path substring works (string.includes)
- ✅ Filter by score range works
- ✅ Custom filter function works
- ✅ Sort by all fields works (score, relpath, symbol_name, start_line, kind)
- ✅ Ascending and descending order works
- ✅ Pagination (slice) works
- ✅ Chaining works (filter → sort → slice)
- ✅ Error handling graceful (invalid inputs don't crash)

### Performance

- ✅ Filter operation: <1ms for 100 results
- ✅ Sort operation: <1ms for 100 results
- ✅ Slice operation: <0.1ms for 100 results
- ✅ Aggregate operation: <0.2ms for 100 results
- ✅ Chained operations: <2ms for filter+sort+slice on 100 results

### Quality

- ✅ 80%+ test coverage on filtering logic
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ All E2E tests pass
- ✅ TypeScript compiles without errors
- ✅ No ESLint warnings
- ✅ TSDoc comments complete

### UX

- ✅ TypeScript autocomplete works (shows all methods)
- ✅ Immutable operations (original results preserved)
- ✅ Chainable API (fluent interface)
- ✅ Backward compatible (existing code works)
- ✅ Documentation clear with examples

---

## Timeline

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Phase 1: Core Filtering | 1 day (6-8h) | Day 1 | Day 1 |
| Phase 2: Sorting & Pagination | 0.5 days (3-4h) | Day 2 AM | Day 2 PM |
| Phase 3: Documentation & Validation | 0.5 day (3-4h) | Day 2 PM | Day 3 |

**Total:** 2 days (12-16 hours)

**Ticket Count:** 11 tickets (reduced from original 16)

**Critical Path:** Phase 1 → Phase 2 → Phase 3 (sequential)

**Parallelization:** None (each phase builds on previous)

---

## Deployment Plan

### Phase 1 Deployment

**After Phase 1 completion:**
- FilterableSearchResult available in daemon-client
- Not yet exposed in MCP (internal use only)
- Can be used by advanced consumers importing daemon-client directly

**Validation:**
```bash
cd packages/daemon-client
pnpm test
pnpm build
```

### Phase 2 Deployment

**After Phase 2 completion:**
- All methods implemented
- Chaining works
- Still internal to daemon-client

**Validation:**
```bash
pnpm test
pnpm build
# Run integration tests
pnpm test:integration
```

### Phase 3 Deployment

**After Phase 3 completion:**
- Exported from daemon-client package
- Documentation complete
- Ready for public use (consumers import and wrap results)

**Validation:**
```bash
# Build all packages
pnpm build

# Run all tests
pnpm test

# E2E validation
pnpm test:e2e

# Performance benchmarks
pnpm test:perf
```

**Release Notes:**
```markdown
## Features

- **Client-Side Result Filtering**: Filter search results by kind, file_type, path substring, and score ranges without re-querying
- **Custom Sorting**: Sort results by score, file path, symbol name, line number, or kind
- **Pagination**: Slice results for page-based navigation
- **Chainable API**: Combine filter, sort, and slice operations fluently
- **Immutable**: All operations preserve original results
- **Zero Dependencies**: No new dependencies added
- **Backward Compatible**: Existing code continues working unchanged
- **TypeScript Only**: No Rust changes required

## Usage

```typescript
import { FilterableSearchResult } from '@crewchief/daemon-client'

const result = new FilterableSearchResult(searchResult)

// Filter TypeScript functions
const filtered = result.filter({kind: "function", file_type: "ts"})

// Sort by path
const sorted = filtered.sortBy("relpath")

// Get first page
const page1 = sorted.slice(0, 10)

// Or chain it all
const results = result
  .filter({kind: "function", file_type: "ts"})
  .sortBy("relpath")
  .slice(0, 10)
```
```

---

## Rollback Plan

If issues arise after deployment:

**Phase 3 Rollback:**
- Remove documentation updates
- FilterableSearchResult still usable but undocumented

**Phase 2 Rollback:**
- Remove sortBy/slice methods
- Filter-only functionality remains

**Phase 1 Rollback:**
- Remove FilterableSearchResult class entirely
- No impact on existing consumers (additive only)

**Procedure:**
1. Identify breaking change or regression
2. Revert specific commits
3. Rebuild packages
4. Re-test
5. Redeploy

**Detection:**
- Monitor error logs for new TypeError exceptions
- Check performance metrics for regressions
- User reports of broken searches

---

## Future Enhancements (Out of Scope)

Explicitly **not** included in MVP, but potential future work:

### Deferred from MVP (High Priority)
- **Aggregation methods**: `aggregate()` - count by kind, file_type
- **Helper methods**: `isEmpty()`, `map()`, `find()`
- **Glob patterns**: Advanced path matching with minimatch (requires new dependency)
- **MCP integration**: Optional wrapping in MCP server (requires type changes)
- **Performance benchmarks**: Automated performance regression tests

### Performance Optimizations
- Lazy evaluation (don't execute until hits accessed)
- Memoization (cache filtered results)
- Index building (O(1) lookups for common filters)
- Virtual scrolling support

### Advanced Filtering
- OR logic (currently only AND)
- NOT logic (exclusion filters)
- Nested filters (complex expressions)
- Regex patterns for path matching
- Date range filtering (if timestamps added)

### Rust Parity
- Move filtering logic to Rust daemon
- Type sync between Rust and TypeScript
- Server-side filtering for large result sets

### UI Integration
- VSCode extension filter controls
- Faceted navigation UI
- Saved filter presets
- Filter history

---

## Conclusion

This phased execution plan delivers **client-side result filtering** in 2 days (12-16 hours) with:

- **Clear deliverables** per phase
- **Reduced scope** (11 tickets, 3 core methods)
- **Zero dependencies** (native JavaScript/TypeScript only)
- **Low risk** (additive changes, backward compatible, no Rust changes)
- **Comprehensive testing** (unit, integration, E2E)
- **Performance validated** (<2ms overhead)

**Key Simplifications from Review:**
- Removed minimatch dependency (use native string methods)
- Removed MCP integration from MVP (future work)
- Removed aggregations and helper methods (future work)
- Reduced from 16 to 11 tickets
- True backward compatibility (no type modifications)

**Ready for ticket generation and execution.**

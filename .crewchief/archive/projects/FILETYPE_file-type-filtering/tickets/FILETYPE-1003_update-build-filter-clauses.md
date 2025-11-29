# Ticket: FILETYPE-1003: Update buildFilterClauses for Multi-Extension Support

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (implementation only, tests in FILETYPE-2002)
- [x] **Verified** - by the verify-ticket agent

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify buildFilterClauses() to handle multiple file extensions via SQL OR clause, replacing the current single-extension implementation.

## Background
The existing file_type filter implementation (lines 458-461) only supports single extensions with a simple LIKE clause. This ticket extends it to support comma-separated multi-extension filtering by generating OR clauses in SQL while maintaining backward compatibility and preventing abuse via extension count limits.

**Reference:**
- architecture.md - "Integration with buildFilterClauses (Before/After)" section (lines 297-360)
- plan.md - Task 1.2

## Acceptance Criteria
- [ ] Single extension generates simple LIKE clause (backward compatible)
- [ ] Multiple extensions generate SQL OR clause with proper parentheses
- [ ] Empty array (from parser) skips filter gracefully
- [ ] Extension count >20 is truncated to 20 (graceful degradation)
- [ ] Parameterized queries used (SQL injection safe)
- [ ] Correct parameter numbering in args array

## Technical Requirements

**Location:** `packages/maproom-mcp/src/index.ts` lines 458-461 (existing file_type filter code)

**Current code (BEFORE):**
```typescript
// Advanced filters
if (filters.file_type) {
  args.push(`%.${filters.file_type}`)
  clauses += ` AND f.relpath LIKE $${args.length}`
}
```

**New code (AFTER):**
```typescript
// Advanced file_type filter with multi-extension support
if (filters.file_type) {
  const extensions = parseFileTypeFilter(filters.file_type)

  // Handle empty result (invalid input or all-commas string)
  if (extensions.length === 0) {
    // Silent ignore - skip this filter entirely
    // This matches existing filter pattern (no errors for bad input)
    continue
  }

  // Enforce extension count limit (prevent DoS via complex OR queries)
  if (extensions.length > 20) {
    // Graceful degradation - truncate to 20
    extensions.splice(20)
  }

  // Single extension: simple LIKE clause (backward compatible)
  if (extensions.length === 1) {
    args.push(`%.${extensions[0]}`)
    clauses += ` AND f.relpath LIKE $${args.length}`
  }
  // Multiple extensions: OR clause
  else {
    const likeConditions = extensions.map(ext => {
      args.push(`%.${ext}`)
      return `f.relpath LIKE $${args.length}`
    })
    clauses += ` AND (${likeConditions.join(' OR ')})`
  }
}
```

**SQL query examples:**
```sql
-- Single extension (file_type: "ts")
WHERE f.relpath LIKE $5  -- args[4] = "%.ts"

-- Multiple extensions (file_type: "ts,tsx,js")
WHERE (f.relpath LIKE $5 OR f.relpath LIKE $6 OR f.relpath LIKE $7)
-- args[4] = "%.ts", args[5] = "%.tsx", args[6] = "%.js"

-- Combined with other filters (recency + file_type)
WHERE f.last_modified > NOW() - INTERVAL $4
  AND (f.relpath LIKE $5 OR f.relpath LIKE $6)
```

## Implementation Notes

**Design decisions:**
1. **Backward compatible:** Single extension uses same SQL pattern as before
2. **OR logic:** Standard SQL pattern for multi-value matching
3. **Parameterized queries:** Prevents SQL injection (security.md confirms this)
4. **Graceful degradation:** Truncate >20 extensions instead of failing
5. **Silent fallback:** Empty array skips filter (search all files)

**Parameter numbering:**
- Each extension adds one parameter to args array
- LIKE clause uses correct $N placeholder
- args.length correctly reflects current parameter count

**Error handling strategy:**
- Empty extensions array: Skip filter (no WHERE clause added)
- >20 extensions: Truncate to 20 (prevent query complexity DoS)
- Invalid input already handled by parseFileTypeFilter

## Dependencies
- **FILETYPE-1002** (parseFileTypeFilter must exist)

## Risk Assessment
- **Risk**: SQL OR clause with 20 extensions degrades performance
  - **Mitigation:** FILETYPE-2004 validates performance <20% overhead; 20-extension limit prevents worse cases

- **Risk**: Parameter numbering error breaks SQL query
  - **Mitigation:** Integration tests (FILETYPE-2002) verify SQL structure and parameterization

- **Risk**: Existing single-extension usage breaks
  - **Mitigation:** Backward compatible design (single extension uses same pattern)

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` (MODIFY - replace lines 458-461)

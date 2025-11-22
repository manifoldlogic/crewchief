# Ticket: FILETYPE-1004: Add Input Validation in handleSearch

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (implementation only, tests in FILETYPE-2003)
- [x] **Verified** - by the verify-ticket agent

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add validation layer in handleSearch function to provide helpful user feedback for invalid file_type filter input and enforce extension count limits.

## Background
While parseFileTypeFilter handles input normalization and buildFilterClauses handles SQL generation, we need a validation layer in handleSearch to give users clear error messages when they provide invalid input (too many extensions, malformed input, etc.). This follows the error handling strategy defined in architecture.md.

**Reference:**
- architecture.md - "Error Handling Strategy" section (lines 155-193)
- plan.md - Task 1.3

## Acceptance Criteria
- [ ] Empty input validation with warning added to hint
- [ ] Extension count >20 returns error response with helpful message
- [ ] Invalid input caught and reported clearly
- [ ] No exceptions thrown (return error response instead)

## Technical Requirements

**Location:** `packages/maproom-mcp/src/index.ts` in handleSearch function (lines 609-860), after mode validation and before search execution

**Code to add:**
```typescript
// Validate file_type filter if provided
if (filters.file_type) {
  try {
    const extensions = parseFileTypeFilter(filters.file_type)

    // Warn user if filter produced no valid extensions
    if (extensions.length === 0) {
      // Don't fail - add warning to hint and continue
      hint = hint || ''
      hint += `\n⚠️ file_type filter "${filters.file_type}" produced no valid extensions. Searching all files.`
    }

    // Enforce hard limit on extension count
    if (extensions.length > 20) {
      return {
        hits: [],
        error: 'Too many file extensions',
        hint: `file_type filter has ${extensions.length} extensions (maximum 20 allowed).\n\nTry: filters: {file_type: "ts,tsx,js"} instead of listing 50+ extensions`,
        suggestion: 'Use broader filter or multiple searches'
      }
    }
  } catch (error: any) {
    // Catch any unexpected errors from parsing
    return {
      hits: [],
      error: 'Invalid file_type filter',
      hint: error.message || 'file_type must be a comma-separated list of extensions (e.g., "ts,tsx,js")'
    }
  }
}
```

## Implementation Notes

**Validation strategy:**
1. **Permissive on input:** parseFileTypeFilter accepts flexible formatting
2. **Strict on validation:** handleSearch enforces limits and provides feedback
3. **Helpful errors:** Clear messages guide users to correct usage

**Error response format:**
- Uses existing search result structure
- Empty hits array
- error field with brief description
- hint field with detailed explanation and examples
- suggestion field with actionable next steps

**Silent fallback behavior:**
- Empty extensions: Add warning to hint, continue search
- No error thrown - graceful degradation

**Hard limit enforcement:**
- >20 extensions: Return error immediately
- Don't proceed to search execution
- Prevents DoS via complex OR queries

**Consistency with existing filters:**
- Matches pattern used by other filters (recency_threshold, repo_id)
- No exceptions thrown - return error response
- Helpful hints guide users to correct usage

## Dependencies
- **FILETYPE-1002** (parseFileTypeFilter must exist)
- **FILETYPE-1003** (buildFilterClauses must be updated to handle extension arrays)

## Risk Assessment
- **Risk**: Validation too strict rejects valid input
  - **Mitigation:** 20 extension limit is generous (covers all realistic use cases)

- **Risk**: Error messages not helpful enough
  - **Mitigation:** Includes examples of correct syntax and actionable suggestions

- **Risk**: Validation inconsistent with other filters
  - **Mitigation:** Follows same pattern as existing filter validation

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` (MODIFY - add validation in handleSearch function)

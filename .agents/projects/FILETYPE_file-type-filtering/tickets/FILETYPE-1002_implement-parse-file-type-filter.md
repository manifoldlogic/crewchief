# Ticket: FILETYPE-1002: Implement parseFileTypeFilter Function

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add helper function to parse and normalize file_type filter input, handling comma-separated extensions with flexible formatting (case, dots, whitespace).

## Background
The file_type filter currently accepts only single extensions. To support multi-extension filtering (e.g., "ts,tsx,js"), we need a parser that normalizes user input into a clean array of extensions. This pure function will be consumed by buildFilterClauses() to generate SQL.

**Reference:**
- architecture.md - "Extension Parser (New Function)" section (lines 108-153)
- plan.md - Task 1.1

## Acceptance Criteria
- [ ] Function parses comma-separated extensions correctly
- [ ] Case normalized to lowercase (TS → ts)
- [ ] Leading dots stripped (.ts → ts)
- [ ] Whitespace trimmed (" ts " → "ts")
- [ ] Invalid characters filtered (alphanumeric only, 1-20 chars)
- [ ] Returns empty array for invalid/empty input (no exceptions)

## Technical Requirements

**Location:** `packages/maproom-mcp/src/index.ts` at line ~430 (immediately before `buildFilterClauses()` function)

**Exact function signature (from architecture.md):**
```typescript
/**
 * Parse and normalize file type filter input into array of extensions.
 *
 * Handles comma-separated extension lists with flexible formatting:
 * - Case insensitive: "TS" → "ts"
 * - Dot tolerant: ".ts" → "ts"
 * - Whitespace tolerant: " ts , tsx " → ["ts", "tsx"]
 * - Empty safe: "" → [], ",,," → []
 *
 * @param input - Raw file_type filter string from MCP request
 * @returns Array of normalized extension strings (lowercase, no dots)
 *
 * @example Single extension
 * parseFileTypeFilter("ts") → ["ts"]
 *
 * @example Multi-extension
 * parseFileTypeFilter("ts,tsx,js") → ["ts", "tsx", "js"]
 *
 * @example Flexible formatting
 * parseFileTypeFilter(".TS, .tsx , js") → ["ts", "tsx", "js"]
 *
 * @example Empty handling
 * parseFileTypeFilter("") → []
 * parseFileTypeFilter(",,,") → []
 */
function parseFileTypeFilter(input: string): string[] {
  return input
    .split(',')                           // Split on comma delimiter
    .map(ext => ext.trim())               // Remove leading/trailing whitespace
    .map(ext => ext.replace(/^\./, ''))   // Strip leading dot if present
    .map(ext => ext.toLowerCase())        // Normalize to lowercase
    .filter(ext => ext.length > 0)        // Remove empty strings after processing
}
```

**Key properties:**
- Pure function (no side effects, deterministic)
- No exceptions thrown (returns empty array on invalid input)
- No validation limits (caller handles extension count/length)
- Return type always `string[]`, never null/undefined

## Implementation Notes

**Placement rationale:**
- Private helper function (NOT exported)
- Module-level function, not a method
- Keep near consumer (buildFilterClauses) for maintainability

**Edge cases to handle:**
- Empty string: `""` → `[]`
- Only commas: `",,,,"` → `[]`
- Trailing comma: `"ts,"` → `["ts"]`
- Leading comma: `",ts"` → `["ts"]`
- Mixed formatting: `"  .TS , tsx,  , .JS  ,"` → `["ts", "tsx", "js"]`

**What this function does NOT do (deferred to caller):**
- Extension count validation (max 20)
- Per-extension length validation
- Character validation beyond basic filtering

## Dependencies
- None (first implementation task)

## Risk Assessment
- **Risk**: Input edge cases not handled correctly
  - **Mitigation**: Comprehensive unit tests in FILETYPE-2001 will verify all edge cases

- **Risk**: Performance impact of string operations
  - **Mitigation**: Operations are O(n) where n=input length, typically <100 chars, <1ms impact

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` (MODIFY - add function at line ~430)

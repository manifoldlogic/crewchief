# Ticket: FILETYPE-1005: Update MCP Tool Description with Examples

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation task, no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- typescript-engineer
- verify-ticket
- commit-ticket

## Summary
Update the MCP tool schema description for file_type parameter to document multi-extension syntax with clear examples and usage limits.

## Background
The current file_type parameter description is minimal ("Filter by file extension"). Users need clear examples showing multi-extension syntax, limits, and common use cases to discover and correctly use the feature.

**Reference:**
- architecture.md - Component Design section mentions tool description
- plan.md - Task 1.4

## Acceptance Criteria
- [x] Parameter description explains multi-extension syntax
- [x] Examples show single and multi-extension usage
- [x] Extension count limit documented (max 20)
- [x] Common use cases illustrated

## Technical Requirements

**Location 1:** `packages/maproom-mcp/src/index.ts` line ~193 (file_type parameter definition)

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

**Location 2:** `packages/maproom-mcp/src/index.ts` line ~166 (main tool description)

**Add to tool description after existing filter examples:**
```typescript
FILTERS: Narrow by file_type, recency, repo_id, worktree_id

Examples:
  filters: {file_type: "ts"}          → Only TypeScript files
  filters: {file_type: "ts,tsx,js"}   → TypeScript or JavaScript files
  filters: {file_type: "md,mdx"}      → Markdown documentation
  filters: {file_type: "rs"}          → Rust source files
  filters: {
    file_type: "ts,tsx",
    recency_threshold: "7 days"
  }                                   → Recent TypeScript files only

FILTER SYNTAX:
- Comma-separated for multiple types: "ts,tsx,js"
- Case insensitive: "TS" same as "ts"
- With or without dot: ".ts" same as "ts"
- Max 20 extensions per filter
```

## Implementation Notes

**Documentation goals:**
1. **Discoverability:** Users can find the feature in tool description
2. **Learnability:** Examples show common use cases clearly
3. **Correctness:** Syntax rules prevent user errors
4. **Limits:** Document constraints (max 20 extensions)

**Examples chosen:**
- Single extension: Most common use case
- Multi-extension: Core new feature
- Documentation search: Practical example (md,mdx)
- Language-specific: Common filter pattern (ts,tsx)
- Combined filters: Show composability

**Syntax documentation:**
- Comma-separated: Clear delimiter
- Case insensitive: Prevent confusion
- Dot optional: Flexible input
- Max 20: Prevent abuse

## Dependencies
- **FILETYPE-1002** (parseFileTypeFilter functionality)
- **FILETYPE-1003** (buildFilterClauses multi-extension support)
- **FILETYPE-1004** (validation enforces limits)

## Risk Assessment
- **Risk**: Examples not representative of real use cases
  - **Mitigation:** Based on common search patterns (ts/tsx/js, md/mdx, rs)

- **Risk**: Documentation not discoverable in MCP clients
  - **Mitigation:** Updated both parameter description and main tool description

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` (MODIFY - update tool schema lines ~166 and ~193)

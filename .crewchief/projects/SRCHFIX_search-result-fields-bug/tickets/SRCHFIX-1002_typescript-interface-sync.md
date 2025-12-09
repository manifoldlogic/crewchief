# Ticket: [SRCHFIX-1002]: Update TypeScript Daemon Client Interface

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-expert
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update daemon-client SearchResult interface to match Rust daemon JSON response structure, syncing chunk_id, symbol_name, and kind fields.

## Background
The TypeScript SearchResult interface currently uses `chunk_index` and lacks `symbol_name` and `kind` fields. The Rust daemon provides these fields, but the TypeScript types don't match, causing type safety issues and preventing access to these values.

This ticket implements Tasks 1.2 and 1.2b from the execution plan: Update both daemon-client packages (main and vendored copy in maproom-mcp).

## Acceptance Criteria
- [ ] Main daemon-client interface updated with chunk_id, symbol_name, kind fields
- [ ] Vendored maproom-mcp daemon-client interface matches main package exactly
- [ ] Sync comment added pointing to Rust struct: `crates/maproom/src/db/mod.rs SearchHit`
- [ ] TypeScript compilation succeeds with no errors across all packages
- [ ] Both interfaces have identical field structure

## Technical Requirements
- Rename `chunk_index` → `chunk_id` in SearchResult interface
- Add `symbol_name: string | null` field
- Add `kind: string` field
- Add documentation comment linking to Rust source of truth
- Update both locations:
  - `/workspace/packages/daemon-client/src/client.ts` (line 26-41)
  - `/workspace/packages/maproom-mcp/src/daemon-client/client.ts` (line 31-45)

## Implementation Notes
**Target structure**:
```typescript
/**
 * Search result from daemon
 *
 * Sync with: crates/maproom/src/db/mod.rs SearchHit
 */
export interface SearchResult {
  hits: Array<{
    chunk_id: number           // RENAMED from chunk_index
    file_path: string
    start_line: number
    end_line: number
    symbol_name: string | null // ADDED
    kind: string               // ADDED
    content: string
    score: number
  }>
  total: number
  query_embedding_time_ms?: number
  search_time_ms?: number
}
```

**Note on vendored copy**: The maproom-mcp package has a vendored copy of daemon-client. Add a comment noting this must stay in sync with the main package.

**Type sync principle**: Rust is the source of truth for daemon types (per CLAUDE.md). TypeScript interfaces must mirror Rust struct fields.

## Dependencies
- Should be completed in parallel with or after SRCHFIX-1001 (daemon serialization)
- Required before SRCHFIX-1003 (mapping code update)

## Risk Assessment
- **Risk**: Renaming chunk_index breaks existing code
  - **Mitigation**: Search for all usages before renaming (see SRCHFIX-1004)
- **Risk**: Vendored copy gets out of sync
  - **Mitigation**: Add clear comments and verify both files match during verification

## Files/Packages Affected
- `/workspace/packages/daemon-client/src/client.ts`
- `/workspace/packages/maproom-mcp/src/daemon-client/client.ts`

## Verification Notes
Verify both interfaces match exactly:
1. Compare the SearchResult interface in both files line-by-line
2. Confirm sync comment points to Rust struct
3. Run `pnpm build` in both daemon-client and maproom-mcp packages
4. Check that no TypeScript errors occur in dependent packages

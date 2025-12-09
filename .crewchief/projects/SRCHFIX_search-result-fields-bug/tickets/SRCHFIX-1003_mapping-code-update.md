# Ticket: [SRCHFIX-1003]: Update Maproom MCP Mapping Code

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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
Update maproom-mcp mapping code to use actual field values from daemon instead of hardcoded empty strings, and remove obsolete fallback code.

## Background
The MCP server mapping code currently hardcodes `symbol_name: ''` and `kind: ''` instead of using the daemon-provided values. It also includes obsolete chunkIdMap fallback logic that tried to work around missing chunk_id values. Now that the daemon properly serializes these fields, we can use the actual values and remove the workarounds.

This ticket implements Task 1.3 from the execution plan: Update Maproom MCP Mapping Code.

## Acceptance Criteria
- [ ] rustOutput mapping uses `hit.symbol_name || ''` instead of hardcoded `''`
- [ ] rustOutput mapping uses `hit.kind` instead of hardcoded `''`
- [ ] chunk_id retrieved directly from `daemonHit.chunk_id` (no chunkIdMap)
- [ ] Obsolete chunkIdMap and related code removed
- [ ] Misleading comments about "not available from daemon" removed
- [ ] TypeScript compilation succeeds with no type errors
- [ ] Warning messages updated to reflect actual issue (invalid ID vs not found)

## Technical Requirements
**File**: `/workspace/packages/maproom-mcp/src/tools/search.ts`

**Changes required**:

1. **Update rustOutput mapping** (line 307-318):
   - Change `symbol_name: ''` → `symbol_name: hit.symbol_name || ''`
   - Change `kind: ''` → `kind: hit.kind`

2. **Remove obsolete chunkIdMap** (line 323-325):
   - Delete `const chunkIdMap = new Map<string, number>()`

3. **Update hits mapping** (line 328-340):
   - Get chunk_id directly from daemon: `daemonHit.chunk_id`
   - Remove chunkIdMap.get() logic
   - Update validation to check for invalid chunk_id (not missing)

4. **Update comments**:
   - Remove "Phase 2 enhancement" notes
   - Remove "not available from daemon" statements
   - Add comment that daemon provides chunk_id directly

## Implementation Notes
**RustSearchHit interface** (line 108-118) already has correct types - no changes needed:
```typescript
interface RustSearchHit {
  score: number
  file_relpath: string
  symbol_name: string | null  // ✓ Already correct
  kind: string                // ✓ Already correct
  start_line: number
  end_line: number
  // ...
}
```

**Updated mapping pattern**:
```typescript
const hits: SearchResult[] = rustOutput.hits.map((hit, index) => {
  const daemonHit = daemonResult.hits[index]

  // Validate chunk_id is present
  if (!daemonHit.chunk_id || daemonHit.chunk_id === 0) {
    log.warn({ hit: daemonHit }, 'Invalid chunk_id in search result')
  }

  const result: SearchResult = {
    chunk_id: daemonHit.chunk_id,  // Use daemon value directly
    symbol_name: hit.symbol_name,
    kind: hit.kind,
    // ... rest of mapping
  }
  // ...
})
```

**Null handling**: `hit.symbol_name || ''` converts null to empty string for backward compatibility with consumers expecting strings.

## Dependencies
- **Requires**: SRCHFIX-1001 (Rust daemon serialization)
- **Requires**: SRCHFIX-1002 (TypeScript interface updates)
- **Parallel with**: SRCHFIX-1004 (search for old field names)

## Risk Assessment
- **Risk**: Removing chunkIdMap breaks fallback logic
  - **Mitigation**: Daemon now provides chunk_id; fallback was never working anyway (always returned 0)
- **Risk**: Null symbol_name breaks downstream code
  - **Mitigation**: Use `|| ''` fallback to convert null to empty string
- **Risk**: Missing daemon fields cause runtime errors
  - **Mitigation**: Add validation logging for invalid chunk_id values

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/src/tools/search.ts`

## Verification Notes
Verify the mapping code:
1. Check rustOutput mapping uses daemon values (not hardcoded)
2. Confirm chunkIdMap code completely removed
3. Verify chunk_id comes directly from daemonHit
4. Run TypeScript compilation and check for type errors
5. Validate warning messages are accurate and helpful

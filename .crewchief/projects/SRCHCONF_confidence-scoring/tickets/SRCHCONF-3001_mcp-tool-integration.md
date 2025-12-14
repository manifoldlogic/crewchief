# Ticket: [SRCHCONF-3001]: MCP Tool Integration and Parameter Passing

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
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the MCP search tool to accept `include_confidence` parameter, add it to SearchParams interface, update tool schema, and ensure the parameter is correctly passed to the daemon.

## Background
Phase 3 exposes confidence scoring to MCP consumers (VS Code extension, Claude Code CLI). This ticket makes the `include_confidence` parameter available in the MCP tool interface and ensures it propagates through the TypeScript layers to the Rust daemon.

This is the final integration point that makes confidence scoring user-accessible via MCP.

## Acceptance Criteria
- [x] SearchParams interface in `packages/daemon-client/src/client.ts` includes `include_confidence?: boolean`
- [x] MCP search tool schema in `packages/maproom-mcp/src/tools/search_schema.ts` includes include_confidence parameter
- [x] Schema marks parameter as optional with default false
- [x] MCP search tool implementation passes include_confidence to daemon client
- [x] End-to-end test confirms confidence returned when include_confidence=true
- [x] Backward compatibility test confirms existing calls work (parameter omitted)
- [x] All TypeScript tests pass (`pnpm test packages/maproom-mcp`)
- [x] Zero TypeScript compilation errors

## Technical Requirements
**1. Update SearchParams Interface** (`packages/daemon-client/src/client.ts`):
```typescript
export interface SearchParams {
  query: string
  repo: string
  worktree?: string
  limit?: number
  mode?: 'fts' | 'vector' | 'hybrid'
  debug?: boolean
  include_confidence?: boolean  // NEW: default false
  deduplicate?: boolean
}
```

**2. Update MCP Search Schema** (`packages/maproom-mcp/src/tools/search_schema.ts`):
```typescript
export const searchSchema = {
  // ... existing fields ...
  include_confidence: {
    type: "boolean",
    description: "Include confidence signals for result quality assessment. Adds source_count, score_gap, and is_exact_match fields to results.",
    optional: true,
    default: false
  }
}
```

**3. Update MCP Tool Implementation** (`packages/maproom-mcp/src/tools/search.ts`):
```typescript
const searchParams: SearchParams = {
  query: args.query,
  repo: args.repo,
  worktree: args.worktree,
  limit: args.limit,
  mode: args.mode,
  debug: args.debug,
  include_confidence: args.include_confidence ?? false,  // Pass to daemon
  deduplicate: args.deduplicate
};

const results = await client.search(searchParams);
```

**4. End-to-End Tests** (minimum 2 tests):
- Test with include_confidence=true returns confidence in results
- Test without include_confidence parameter works (backward compatibility)

## Implementation Notes
Follow existing parameter patterns:
- `debug?: boolean` already exists as optional parameter
- `include_confidence` follows same pattern
- Default to false for opt-in rollout (per plan.md)
- Future may flip default to true after validation period

Parameter flow:
1. MCP tool receives `include_confidence` in args
2. MCP tool passes to daemon client via SearchParams
3. Daemon client serializes to JSON-RPC
4. Rust daemon deserializes to SearchOptions
5. Search pipeline computes confidence if true
6. Response includes confidence in ChunkSearchResult
7. TypeScript deserializes using ConfidenceSignals interface
8. MCP tool returns to consumer

Documentation for parameter:
- Description should explain what confidence signals provide
- Mention 3 core fields: source_count, score_gap, is_exact_match
- Note default is false (opt-in for MVP)

## Dependencies
- **Prerequisite**: SRCHCONF-2001 (TypeScript types must exist)
- **Prerequisite**: SRCHCONF-2002 (Rust integration must be complete)
- **Prerequisite**: Phase 2 complete (daemon must support parameter)

## Risk Assessment
- **Risk**: Parameter not correctly passed through all layers
  - **Mitigation**: End-to-end test verifies full flow. Test with daemon running, actual search execution.
- **Risk**: Schema validation rejects parameter
  - **Mitigation**: Mark as optional, provide default. Follow debug parameter pattern exactly.
- **Risk**: Breaks existing MCP consumers
  - **Mitigation**: Optional parameter, backward compatibility test, no change to response structure when omitted.

## Files/Packages Affected
- `packages/daemon-client/src/client.ts` - Add include_confidence to SearchParams
- `packages/maproom-mcp/src/tools/search_schema.ts` - Add parameter to schema
- `packages/maproom-mcp/src/tools/search.ts` - Pass parameter to daemon
- `packages/maproom-mcp/tests/integration/confidence.test.ts` - NEW end-to-end tests

## Verification Notes
The verify-ticket agent should check:
- include_confidence parameter exists in all 3 locations (SearchParams, schema, tool implementation)
- Parameter is optional with correct default (false)
- Schema description clearly explains what parameter does
- End-to-end test shows confidence appears in response when enabled
- Backward compatibility test shows existing calls still work
- Test output demonstrates actual daemon execution (not mocked)
- TypeScript compilation succeeds

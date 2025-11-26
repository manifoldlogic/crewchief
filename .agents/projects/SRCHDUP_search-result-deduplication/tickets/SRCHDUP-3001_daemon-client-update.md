# Ticket: SRCHDUP-3001: Update daemon-client SearchParams interface

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - core tests pass (rpc, errors, lifecycle); performance tests require daemon binary
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Update the daemon-client TypeScript package to include a `deduplicate` parameter in the `SearchParams` interface and pass it through the JSON-RPC call to the Rust daemon.

## Background

The daemon-client package provides the JSON-RPC bridge between MCP TypeScript and the Rust daemon. For the MCP search tool to control deduplication, the daemon-client must accept and forward the `deduplicate` parameter.

**Reference:** plan.md Phase 3, architecture.md Section 6 "Daemon-Client Integration"

## Acceptance Criteria

- [x] `SearchParams` interface has `deduplicate?: boolean` field
- [x] `search()` method includes `deduplicate` in JSON-RPC call
- [x] Default behavior is `deduplicate: true` when not specified
- [x] TypeScript compiles without errors
- [x] Existing daemon-client tests pass
- [x] Type exports are correct for consumers

## Technical Requirements

### SearchParams Interface Update
```typescript
// In packages/daemon-client/src/client.ts (or types.ts)

export interface SearchParams {
  query: string;
  repo: string;
  worktree?: string;
  limit?: number;
  threshold?: number;
  debug?: boolean;
  deduplicate?: boolean;  // NEW: default true
}
```

### Search Method Update
```typescript
async search(params: SearchParams): Promise<SearchResult[]> {
  return this.call('search', {
    query: params.query,
    repo: params.repo,
    worktree: params.worktree,
    limit: params.limit ?? 10,
    threshold: params.threshold,
    debug: params.debug,
    deduplicate: params.deduplicate ?? true,  // Default enabled
  });
}
```

### Type Export
Ensure `SearchParams` is exported if consumers need it:
```typescript
export type { SearchParams } from './client';
// or wherever the interface is defined
```

## Implementation Notes

1. **Locate interface** - Find where `SearchParams` is defined (client.ts, types.ts, index.ts)
2. **Check existing patterns** - See how other optional params are handled
3. **Verify export** - Ensure type changes are visible to consumers
4. **Build test** - Run `pnpm build` in daemon-client package

### Verification Steps
```bash
cd packages/daemon-client
pnpm build
pnpm test  # if tests exist
```

## Dependencies

- SRCHDUP-2002 (Rust pipeline accepts deduplicate param)

## Risk Assessment

- **Risk**: Breaking change for existing consumers
  - **Mitigation**: New field is optional with sensible default, backward compatible
- **Risk**: Type mismatch with Rust daemon
  - **Mitigation**: Ensure field name matches Rust SearchRequest exactly

## Files/Packages Affected

- `packages/daemon-client/src/client.ts` (modify SearchParams, search method)
- `packages/daemon-client/src/types.ts` (if interfaces are separate)
- `packages/daemon-client/src/index.ts` (verify exports)

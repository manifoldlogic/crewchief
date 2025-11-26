# Ticket: SRCHDUP-3003: Add deduplicate parameter to MCP search schema

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Update the MCP search tool in maproom-mcp to accept a `deduplicate` parameter and pass it through to the daemon-client. This completes the integration chain from MCP consumer to Rust pipeline.

## Background

The MCP server exposes search functionality to Claude and other MCP clients. Adding the `deduplicate` parameter to the MCP tool schema allows clients to control deduplication behavior.

**Reference:** plan.md Phase 3, architecture.md Section 8 "MCP Tool Update"

## Acceptance Criteria

- [ ] MCP search tool schema includes `deduplicate?: boolean` parameter
- [ ] Parameter has description in schema for MCP consumers
- [ ] Tool handler extracts `deduplicate` and passes to daemon-client
- [ ] Default behavior (missing param) enables deduplication
- [ ] MCP server builds: `pnpm build` in maproom-mcp
- [ ] Tool appears correctly with parameter in MCP tool listing

## Technical Requirements

### Tool Schema Update
```typescript
// In packages/maproom-mcp/src/tools/search.ts

const searchTool: Tool = {
  name: 'search',
  description: 'Search for code in the indexed repository',
  inputSchema: {
    type: 'object',
    properties: {
      query: {
        type: 'string',
        description: 'Search query',
      },
      repo: {
        type: 'string',
        description: 'Repository name',
      },
      worktree: {
        type: 'string',
        description: 'Worktree name (optional)',
      },
      limit: {
        type: 'number',
        description: 'Maximum results (default: 10)',
      },
      deduplicate: {
        type: 'boolean',
        description: 'Deduplicate results across worktrees (default: true)',
      },
    },
    required: ['query'],
  },
};
```

### Handler Update
```typescript
async function handleSearch(params: SearchParams): Promise<ToolResult> {
  const results = await client.search({
    query: params.query,
    repo: params.repo ?? defaultRepo,
    worktree: params.worktree,
    limit: params.limit,
    deduplicate: params.deduplicate,  // Pass through to daemon-client
  });

  return formatResults(results);
}
```

### TypeScript Types
```typescript
interface SearchParams {
  query: string;
  repo?: string;
  worktree?: string;
  limit?: number;
  mode?: 'fts' | 'vector' | 'hybrid';
  deduplicate?: boolean;
}
```

## Implementation Notes

1. **Find tool definition** - Locate where search tool is registered
2. **Check existing param patterns** - Follow existing patterns for optional params
3. **Verify JSON Schema** - MCP uses JSON Schema for tool definitions
4. **Test with MCP client** - Use Claude or MCP inspector to verify

### Verification
```bash
cd packages/maproom-mcp
pnpm build
pnpm test  # if tests exist

# Test with MCP inspector or Claude
# Tool should appear with deduplicate parameter in listing
```

## Dependencies

- SRCHDUP-3001 (daemon-client SearchParams updated)
- SRCHDUP-3002 (Rust daemon accepts parameter)

## Risk Assessment

- **Risk**: MCP schema validation differs from TypeScript
  - **Mitigation**: Use JSON Schema boolean type correctly
- **Risk**: Parameter not reaching Rust daemon
  - **Mitigation**: Add logging at each layer to trace parameter flow

## Files/Packages Affected

- `packages/maproom-mcp/src/tools/search.ts` (or wherever search tool is defined)
- `packages/maproom-mcp/src/tools/index.ts` (if tool registration is separate)

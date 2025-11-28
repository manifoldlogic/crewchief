# Ticket: CTXCLI-3002: Replace PostgreSQL with Daemon Client

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the MCP context tool to use the daemon client instead of PostgreSQL, including adding the `DaemonClient.context()` method and implementing the response mapping layer.

## Background
This is the core integration ticket of Phase 3. The MCP context tool currently uses PostgreSQL directly, duplicating assembly logic in TypeScript. This ticket replaces that with a daemon client call, following the pattern established by the search tool.

Reference: [planning/architecture.md](../planning/architecture.md) - Section 3: MCP Context Tool Update, Section 4: Mapping Layer

## Acceptance Criteria
- [x] `DaemonClient.context()` method added to daemon-client package
- [x] TypeScript types defined for `ContextParams` and `RustContextBundle`
- [x] `ContextParams` and `RustContextBundle` types exported from `packages/daemon-client/src/index.ts`
- [x] MCP context tool uses daemon client (not PostgreSQL)
- [x] PostgreSQL client (`pg`) usage removed from context.ts
- [x] Response mapping layer implemented:
  - Pass through: `items`, `total_tokens`, `truncated`
  - Compute: `budget_tokens` (from request params)
  - Compute: `budget_remaining` = `budget_tokens - total_tokens`
  - Compute: `metadata.worktree` and `metadata.repo` (from chunk_id lookup via daemon OR passed in request context)
- [x] Response format matches existing MCP ContextBundle interface
- [x] Error handling follows `search.ts` pattern (DaemonStartError, DaemonTimeoutError, RpcError)
- [x] Error messages match existing format

## Technical Requirements
- Follow `search.ts` pattern exactly for daemon integration
- Handle all daemon error types with appropriate MCP error responses
- Map Rust's simpler ContextBundle to MCP's enhanced format
- Preserve backward compatibility with MCP clients

## Implementation Notes

### DaemonClient.context() Method
```typescript
// packages/daemon-client/src/index.ts

export interface ContextParams {
  chunk_id: string
  budget_tokens?: number
  expand?: {
    callers?: boolean
    callees?: boolean
    tests?: boolean
    docs?: boolean
    config?: boolean
    max_depth?: number
    routes?: boolean
    hooks?: boolean
    jsx_parents?: boolean
    jsx_children?: boolean
  }
}

export interface RustContextBundle {
  items: ContextItem[]
  total_tokens: number
  truncated: boolean
}

export class DaemonClient {
  // ... existing methods ...

  async context(params: ContextParams): Promise<RustContextBundle> {
    return this.call('context', params)
  }
}
```

### Response Mapping Layer

**Note**: The Rust `ContextItem` struct does NOT contain `worktree` or `repo` fields. The metadata must be obtained either:
1. By passing `worktree` and `repo` in the request context (derived from chunk_id lookup)
2. Or by adding these fields to the Rust `ContextBundle` struct at the bundle level

**Recommended approach**: Since the MCP server already has the search context and knows the worktree/repo from prior searches, pass these through from the request context.

```typescript
// packages/maproom-mcp/src/tools/context.ts

function mapRustToMcpBundle(
  rustBundle: RustContextBundle,
  requestParams: ContextParams,
  requestContext: { worktree: string; repo: string }  // From MCP request context
): ContextBundle {
  const budgetTokens = requestParams.budget_tokens ?? 6000
  return {
    items: rustBundle.items,
    total_tokens: rustBundle.total_tokens,
    budget_tokens: budgetTokens,
    budget_remaining: budgetTokens - rustBundle.total_tokens,
    truncated: rustBundle.truncated,
    metadata: {
      worktree: requestContext.worktree,
      repo: requestContext.repo,
    },
  }
}
```

### Error Handling Pattern (from search.ts)
```typescript
export async function handleContextTool(
  params: unknown,
  daemonClient: DaemonClient
): Promise<ContextBundle> {
  const validatedParams = validateContextParams(params)

  try {
    const result = await daemonClient.context({
      chunk_id: validatedParams.chunk_id,
      budget_tokens: validatedParams.budget_tokens,
      expand: validatedParams.expand,
    })
    return mapRustToMcpBundle(result, validatedParams)
  } catch (error) {
    if (error instanceof DaemonStartError) {
      throw new McpError('DAEMON_START_FAILED', error.message)
    }
    if (error instanceof DaemonTimeoutError) {
      throw new McpError('DAEMON_TIMEOUT', error.message)
    }
    if (error instanceof RpcError) {
      if (error.code === -32000) {
        throw new McpError('CHUNK_NOT_FOUND', error.message)
      }
      throw new McpError('RPC_ERROR', error.message)
    }
    throw error
  }
}
```

## Dependencies
- CTXCLI-3001 (MCP schema must be updated first)
- CTXCLI-1002 (Daemon must support context method)

## Risk Assessment
- **Risk**: Breaking existing MCP clients expecting specific response format
  - **Mitigation**: Mapping layer preserves all existing fields, adds computed ones
- **Risk**: Daemon not running when MCP tool called
  - **Mitigation**: Graceful error handling with DaemonStartError

## Files/Packages Affected
- `packages/daemon-client/src/index.ts` (modify - add context() method and types)
- `packages/maproom-mcp/src/tools/context.ts` (modify - replace PostgreSQL with daemon)

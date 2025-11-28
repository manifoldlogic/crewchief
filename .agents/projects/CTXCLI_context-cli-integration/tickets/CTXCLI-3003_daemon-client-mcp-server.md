# Ticket: CTXCLI-3003: Add Daemon Client to MCP Server

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
Ensure the daemon client is properly initialized and passed to the context handler in the MCP server entry point.

## Background
This completes Phase 3 (MCP Integration). The daemon client singleton pattern is already established for the search tool. This ticket ensures the context handler receives the same daemon client instance and handles connection errors gracefully.

Reference: [planning/architecture.md](../planning/architecture.md) - Architecture Diagram (MCP Server section)

## Acceptance Criteria
- [x] Daemon client imported from existing singleton setup (`getDaemonClient()`)
- [x] Daemon client used by `handleContextTool()` function (via internal `getDaemonClient()` call, same pattern as search tool)
- [x] Graceful error message if daemon not running
- [x] No duplicate daemon client instances created
- [x] Context tool registered in MCP server tool list

## Technical Requirements
- Use existing `getDaemonClient()` singleton from `daemon.ts`
- Context handler calls `getDaemonClient()` internally (same pattern as search tool)
- Handle `DaemonStartError` within the tool handler with user-friendly messages
- Ensure daemon client is shared across tools (search, context)

## Implementation Notes

### MCP Server Integration
```typescript
// packages/maproom-mcp/src/index.ts

import { getDaemonClient } from './daemon'
import { handleContextTool } from './tools/context'
import { handleSearchTool } from './tools/search'

// Tool registration
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const daemonClient = await getDaemonClient()

  switch (request.params.name) {
    case 'search':
      return handleSearchTool(request.params.arguments, daemonClient)

    case 'context':
      return handleContextTool(request.params.arguments, daemonClient)

    // ... other tools
  }
})
```

### Error Handling
```typescript
// If daemon client fails to start
try {
  const daemonClient = await getDaemonClient()
  // ... handle tool
} catch (error) {
  if (error instanceof DaemonStartError) {
    return {
      content: [{
        type: 'text',
        text: `Daemon not available: ${error.message}. Start the daemon with: crewchief-maproom serve`,
      }],
      isError: true,
    }
  }
  throw error
}
```

### Daemon Client Singleton Pattern
The existing `getDaemonClient()` in `daemon.ts` already handles:
- Lazy initialization
- Auto-restart on failure
- Connection pooling
- Shared instance across tools

No changes needed to daemon.ts - just use the existing pattern.

## Dependencies
- CTXCLI-3002 (Context tool implementation must exist)

## Risk Assessment
- **Risk**: Daemon client initialization race conditions
  - **Mitigation**: `getDaemonClient()` singleton handles this already
- **Risk**: Different daemon client behavior between tools
  - **Mitigation**: Use same singleton, same error handling pattern

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` (modify - add context tool handler with daemon client)

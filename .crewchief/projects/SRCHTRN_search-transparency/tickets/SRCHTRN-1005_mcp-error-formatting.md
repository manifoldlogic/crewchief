# SRCHTRN-1005: MCP Error Formatting

## Title
Update MCP tool to format structured errors with context and suggestions

## Status
- [ ] **Implementation Complete**
- [ ] **Tests Passing**
- [ ] **Verified**
- [ ] **Committed**

## Agents
- **Primary**: typescript-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Update `packages/maproom-mcp/src/tools/search.ts` to format structured error responses from `RpcError.details`, displaying error type, stage, context, and suggestions in MCP protocol format.

## Background
With SRCHTRN-1004 providing parsed error details in `RpcError`, the MCP tool can now display rich error information to users. This ticket updates the error formatting logic to use structured details when available, while maintaining backward compatibility with generic errors.

**User Impact**: Users will see actionable errors with context and suggestions instead of generic "RPC_ERROR" messages.

## Acceptance Criteria
- [ ] `formatSearchError()` function checks for `RpcError` with details
- [ ] Structured errors formatted with error type, stage, context, and suggestions
- [ ] Fallback to existing error handling if no details available
- [ ] Error format follows MCP protocol (JSON content in text field)
- [ ] Integration test validates end-to-end error flow (Rust → TypeScript → MCP)
- [ ] Manual test: Embedding provider offline shows actionable error
- [ ] Backward compatibility verified: Old clients still work
- [ ] All tests passing

## Technical Requirements

### Update Error Formatting: `packages/maproom-mcp/src/tools/search.ts`

Locate existing error handling (likely in `handleSearchError` or similar):

```typescript
import { RpcError } from '@crewchief/daemon-client'

export function formatSearchError(error: unknown): MCPError {
  // Check for RpcError with structured details
  if (error instanceof RpcError) {
    const details = error.getDetails()

    if (details) {
      return {
        isError: true,
        content: [
          {
            type: 'text',
            text: JSON.stringify(
              {
                error: details.error_type,
                stage: details.stage,
                message: error.message,
                context: details.context,
                suggestions: details.suggestions,
              },
              null,
              2
            ),
          },
        ],
      }
    }
  }

  // Fallback to existing error handling
  return {
    isError: true,
    content: [
      {
        type: 'text',
        text: error instanceof Error ? error.message : String(error),
      },
    ],
  }
}
```

### Example Structured Error Output
```json
{
  "error": "embedding_provider",
  "stage": "query_processing",
  "message": "Query processing failed: Embedding generation failed: request timeout",
  "context": {
    "provider_error": "request timeout"
  },
  "suggestions": [
    "Check your embedding provider credentials",
    "Verify network connectivity",
    "Try FTS mode while debugging: --mode fts"
  ]
}
```

### Integration Test: `packages/maproom-mcp/tests/search-error-handling.test.ts`

```typescript
import { handleSearchTool } from '../src/tools/search.js'
import { RpcError } from '@crewchief/daemon-client'

describe('Search error handling integration', () => {
  it('should format embedding provider error with suggestions', async () => {
    // Mock daemon client to return error with details
    const mockClient = createMockDaemonClient({
      search: () => {
        throw new RpcError(
          'Embedding generation failed',
          -32000,
          {
            error_type: 'embedding_provider',
            stage: 'query_processing',
            context: { provider_error: 'timeout' },
            suggestions: [
              'Check credentials',
              'Try FTS mode'
            ],
          }
        )
      },
    })

    const result = await handleSearchTool(
      {
        query: 'test',
        repo: 'crewchief',
        mode: 'vector',
      },
      mockClient
    )

    expect(result.isError).toBe(true)
    const errorText = JSON.parse(result.content[0].text)
    expect(errorText.error).toBe('embedding_provider')
    expect(errorText.stage).toBe('query_processing')
    expect(errorText.suggestions).toHaveLength(2)
  })

  it('should handle errors without details gracefully', async () => {
    const mockClient = createMockDaemonClient({
      search: () => {
        throw new Error('Generic error')
      },
    })

    const result = await handleSearchTool(
      { query: 'test', repo: 'crewchief' },
      mockClient
    )

    expect(result.isError).toBe(true)
    expect(result.content[0].text).toContain('Generic error')
  })
})
```

### Manual E2E Test Checklist
Test with real daemon and MCP client:

**1. Embedding Provider Offline**
```bash
# Stop Ollama or invalidate API key
export OPENAI_API_KEY=invalid

# Run MCP server and trigger vector search
npx @crewchief/maproom-mcp
# → Search with mode=vector
# Expected: Error shows "embedding_provider", suggests FTS mode
```

**2. Repository Not Found**
```bash
# Search for non-existent repo
# Expected: Error shows repo name, suggests status/scan commands
```

**3. Backward Compatibility**
```bash
# Use existing MCP client (before SRCHTRN updates)
# Expected: Generic errors still display (no crash from new fields)
```

## Implementation Notes
1. Locate error handling in `packages/maproom-mcp/src/tools/search.ts`
2. Add check for `RpcError` instance and `getDetails()` method
3. Format structured error as JSON in MCP text content
4. Preserve fallback for generic errors (backward compatibility)
5. Keep formatting clean and readable (use `JSON.stringify` with indentation)

**Design Choice**: Use JSON formatting for structured data rather than freeform text. This allows MCP clients to parse and display errors flexibly.

**Backward Compatibility**: Old clients without this update will still see error messages (may be generic), but won't crash on new `data` field.

## Dependencies
- **SRCHTRN-1004**: TypeScript error deserialization (must complete first - provides `RpcError.getDetails()`)

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Breaking existing MCP clients
- Error format not readable

**Mitigations**:
- Fallback to generic error handling
- JSON formatting with indentation (readable)
- Integration tests validate end-to-end flow
- Manual testing with real daemon

## Files/Packages Affected
- **Modified**: `packages/maproom-mcp/src/tools/search.ts` (~30 lines modified)
- **New file**: `packages/maproom-mcp/tests/search-error-handling.test.ts` (or extend existing)
- **Import**: `import { RpcError } from '@crewchief/daemon-client'`

## Estimated Effort
4-5 hours

**Breakdown**:
- Error formatting implementation: 1-2 hours
- Integration tests: 1-2 hours
- Manual E2E testing: 1-2 hours

## Planning References
- [plan.md](../planning/plan.md) - Phase 1 ticket breakdown, acceptance tests
- [architecture.md](../planning/architecture.md) - MCP error formatting design
- [quality-strategy.md](../planning/quality-strategy.md) - E2E testing approach, backward compatibility

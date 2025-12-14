# SRCHTRN-1004: TypeScript Error Deserialization

## Title
Extend RpcError with error details deserialization and user message formatting

## Status
- [x] **Implementation Complete**
- [x] **Tests Passing**
- [x] **Verified**
- [x] **Committed**

## Agents
- **Primary**: typescript-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Extend the `RpcError` class in `packages/daemon-client/src/errors.ts` to parse `SearchErrorDetails` from JSON-RPC error `data` field and provide a `getUserMessage()` helper for formatted error display.

## Background
The daemon now serializes `SearchErrorDetails` in JSON-RPC error responses (SRCHTRN-1002). TypeScript clients need to deserialize these details and format them for user display. This ticket extends the existing `RpcError` class to support structured error handling.

**Existing Infrastructure**: `RpcError` class already exists in `packages/daemon-client/src/errors.ts` and extends `DaemonError`.

## Acceptance Criteria
- [x] `RpcError` class extended with `details?: SearchErrorDetails` field
- [x] Constructor parses `data` field from JSON-RPC error response
- [x] `getDetails()` method returns parsed error details or undefined
- [x] `getUserMessage()` method formats error with context and suggestions
- [x] Fallback to simple error message if no details available
- [x] Unit tests validate deserialization for all error types
- [x] Unit tests validate `getUserMessage()` formatting
- [x] All tests passing

## Technical Requirements

### Extend RpcError Class: `packages/daemon-client/src/errors.ts`

```typescript
import type { SearchErrorDetails } from './types.js'

export class RpcError extends DaemonError {
  public readonly rpcCode: number
  public readonly details?: SearchErrorDetails

  constructor(message: string, rpcCode: number, data?: unknown) {
    super(message, 'RPC_ERROR')
    this.rpcCode = rpcCode

    // Attempt to parse structured error details
    if (data && typeof data === 'object' && this.isSearchErrorDetails(data)) {
      this.details = data as SearchErrorDetails
    }
  }

  private isSearchErrorDetails(data: unknown): boolean {
    // Type guard to validate data structure
    if (typeof data !== 'object' || data === null) return false
    const obj = data as Record<string, unknown>
    return (
      typeof obj.error_type === 'string' &&
      typeof obj.stage === 'string' &&
      typeof obj.context === 'object' &&
      Array.isArray(obj.suggestions)
    )
  }

  getDetails(): SearchErrorDetails | undefined {
    return this.details
  }

  getUserMessage(): string {
    if (!this.details) {
      return this.message // Fallback to generic error
    }

    const { error_type, stage, context, suggestions } = this.details

    let formatted = `Search failed at ${stage}: ${this.message}\n`

    // Add context if available
    if (Object.keys(context).length > 0) {
      formatted += '\nContext:\n'
      for (const [key, value] of Object.entries(context)) {
        formatted += `  ${key}: ${value}\n`
      }
    }

    // Add suggestions
    if (suggestions.length > 0) {
      formatted += '\nSuggestions:\n'
      for (const suggestion of suggestions) {
        formatted += `  - ${suggestion}\n`
      }
    }

    return formatted
  }
}
```

### Unit Tests: `packages/daemon-client/src/errors.test.ts`

```typescript
import { RpcError } from './errors.js'
import type { SearchErrorDetails } from './types.js'

describe('RpcError', () => {
  it('should parse error details from data field', () => {
    const errorData: SearchErrorDetails = {
      error_type: 'embedding_provider',
      stage: 'query_processing',
      context: { provider_error: 'timeout' },
      suggestions: ['Check credentials', 'Try FTS mode'],
    }

    const error = new RpcError('Embedding failed', -32000, errorData)

    expect(error.getDetails()).toEqual(errorData)
    expect(error.details?.error_type).toBe('embedding_provider')
  })

  it('should format user message with context and suggestions', () => {
    const errorData: SearchErrorDetails = {
      error_type: 'database',
      stage: 'search_execution',
      context: { message: 'Connection failed', repo: 'crewchief' },
      suggestions: ['Check database connectivity', 'Verify repository indexed'],
    }

    const error = new RpcError('Database error', -32000, errorData)
    const message = error.getUserMessage()

    expect(message).toContain('search_execution')
    expect(message).toContain('Connection failed')
    expect(message).toContain('Check database connectivity')
    expect(message).toContain('Verify repository indexed')
  })

  it('should handle missing error details gracefully', () => {
    const error = new RpcError('Generic error', -32000)

    expect(error.getDetails()).toBeUndefined()
    expect(error.getUserMessage()).toEqual('Generic error')
  })

  it('should handle invalid error details structure', () => {
    const invalidData = { foo: 'bar' }

    const error = new RpcError('Invalid error', -32000, invalidData)

    expect(error.getDetails()).toBeUndefined()
    expect(error.getUserMessage()).toEqual('Invalid error')
  })

  it('should deserialize all error types correctly', () => {
    const errorTypes: Array<SearchErrorDetails['error_type']> = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown'
    ]

    for (const type of errorTypes) {
      const errorData: SearchErrorDetails = {
        error_type: type,
        stage: 'query_processing',
        context: {},
        suggestions: ['Test suggestion'],
      }

      const error = new RpcError('Test error', -32000, errorData)
      expect(error.getDetails()?.error_type).toBe(type)
    }
  })
})
```

## Implementation Notes
1. Locate existing `RpcError` class in `packages/daemon-client/src/errors.ts`
2. Add `details?: SearchErrorDetails` field
3. Parse `data` parameter in constructor with type guard
4. Implement `getUserMessage()` with context and suggestions formatting
5. Maintain backward compatibility (no details = fallback to simple message)

**Type Safety**: Use type guard (`isSearchErrorDetails`) to validate structure before casting to `SearchErrorDetails`.

**Formatting**: Keep formatting simple and readable. Multi-line format with clear sections (Context, Suggestions).

## Dependencies
- **SRCHTRN-1003**: TypeScript error types (must complete first - provides `SearchErrorDetails` interface)

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Malformed error data from daemon
- Breaking existing error handling

**Mitigations**:
- Type guard validates structure before parsing
- Fallback to simple message if no details
- Unit tests cover edge cases (missing details, invalid structure)

## Files/Packages Affected
- **Modified**: `packages/daemon-client/src/errors.ts` (~40 lines added)
- **New file**: `packages/daemon-client/src/errors.test.ts` (if not exists) or extend existing
- **Import**: `import type { SearchErrorDetails } from './types.js'`

## Estimated Effort
3-4 hours

**Breakdown**:
- RpcError extension: 1-2 hours
- getUserMessage() formatting: 1 hour
- Unit tests: 1-2 hours

## Planning References
- [plan.md](../planning/plan.md) - Phase 1 ticket breakdown
- [architecture.md](../planning/architecture.md) - TypeScript error deserialization design
- [quality-strategy.md](../planning/quality-strategy.md) - Unit testing approach

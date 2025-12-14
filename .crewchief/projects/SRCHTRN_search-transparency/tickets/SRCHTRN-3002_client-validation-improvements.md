# SRCHTRN-3002: Client Validation Improvements

## Title
Enhance Zod validation error messages for better clarity

## Status
- [x] **Implementation Complete**
- [x] **Tests Passing**
- [x] **Verified**
- [ ] **Committed**

## Agents
- **Primary**: typescript-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Improve Zod validation error messages in `packages/maproom-mcp` for better clarity and actionability. Focus on message quality, not new validation rules - pre-RPC validation already exists.

## Background
Client-side Zod validation catches common errors (empty queries, invalid parameters) before they reach the daemon. Current error messages work but could be clearer. This ticket enhances message quality without changing validation coverage.

**Scope**: Improve error messages for existing validation. No new validation rules.

## Acceptance Criteria
- [ ] Empty query error message improved (was: "Required", now: "Query cannot be empty")
- [ ] Invalid mode error message improved (shows valid options)
- [ ] Invalid repo parameter error message improved
- [ ] Error messages are actionable (tell user what to do)
- [ ] Unit tests validate improved error messages
- [ ] Manual test: Empty query shows clear error
- [ ] All tests passing

## Technical Requirements

### Enhance Zod Error Messages: `packages/maproom-mcp/src/tools/search.ts`

Current validation (example):
```typescript
const SearchParamsSchema = z.object({
  query: z.string(),
  repo: z.string(),
  mode: z.enum(['code', 'text', 'auto']).optional(),
  // ...
})
```

Enhanced validation with better messages:
```typescript
const SearchParamsSchema = z.object({
  query: z.string().min(1, {
    message: "Query cannot be empty. Provide a search query to find relevant code."
  }),
  repo: z.string().min(1, {
    message: "Repository name is required. Use 'crewchief status' to list available repositories."
  }),
  mode: z.enum(['code', 'text', 'auto'], {
    errorMap: () => ({
      message: "Invalid search mode. Use 'code', 'text', or 'auto'."
    })
  }).optional(),
  limit: z.number().int().positive({
    message: "Limit must be a positive integer."
  }).max(1000, {
    message: "Limit cannot exceed 1000 results."
  }).optional(),
  // ...
})
```

### Validation Error Handler
```typescript
function formatValidationError(error: z.ZodError): string {
  const messages = error.errors.map(err => {
    const path = err.path.join('.')
    return `${path}: ${err.message}`
  })

  return `Validation failed:\n${messages.join('\n')}`
}

// Usage in search tool
export async function handleSearchTool(params: unknown): Promise<MCPResponse> {
  try {
    const validated = SearchParamsSchema.parse(params)
    // ... continue with search
  } catch (error) {
    if (error instanceof z.ZodError) {
      return {
        isError: true,
        content: [
          {
            type: 'text',
            text: formatValidationError(error)
          }
        ]
      }
    }
    throw error
  }
}
```

### Unit Tests: `packages/maproom-mcp/tests/validation-messages.test.ts`

```typescript
import { SearchParamsSchema } from '../src/tools/search.js'

describe('Validation error messages', () => {
  it('should provide clear error for empty query', () => {
    const result = SearchParamsSchema.safeParse({
      query: '',
      repo: 'crewchief'
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const queryError = result.error.errors.find(e => e.path[0] === 'query')
      expect(queryError?.message).toContain('Query cannot be empty')
      expect(queryError?.message).toContain('search query')
    }
  })

  it('should provide clear error for invalid mode', () => {
    const result = SearchParamsSchema.safeParse({
      query: 'test',
      repo: 'crewchief',
      mode: 'invalid'
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const modeError = result.error.errors.find(e => e.path[0] === 'mode')
      expect(modeError?.message).toContain('code')
      expect(modeError?.message).toContain('text')
      expect(modeError?.message).toContain('auto')
    }
  })

  it('should provide clear error for missing repo', () => {
    const result = SearchParamsSchema.safeParse({
      query: 'test'
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const repoError = result.error.errors.find(e => e.path[0] === 'repo')
      expect(repoError?.message).toContain('required')
      expect(repoError?.message).toContain('crewchief status')
    }
  })

  it('should provide clear error for limit out of range', () => {
    const result = SearchParamsSchema.safeParse({
      query: 'test',
      repo: 'crewchief',
      limit: 2000
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const limitError = result.error.errors.find(e => e.path[0] === 'limit')
      expect(limitError?.message).toContain('1000')
    }
  })
})
```

### Example Improved Error Messages

**Before**:
```
Validation error: Required
```

**After**:
```
Validation failed:
query: Query cannot be empty. Provide a search query to find relevant code.
```

**Before**:
```
Invalid enum value. Expected 'code' | 'text' | 'auto'
```

**After**:
```
Validation failed:
mode: Invalid search mode. Use 'code', 'text', or 'auto'.
```

## Implementation Notes
1. Locate Zod schemas in `packages/maproom-mcp/src/tools/search.ts`
2. Add custom error messages using Zod's `message` option
3. Use `errorMap` for enum validation messages
4. Add validation error formatter
5. Update error handler to use formatter

**Message Guidelines**:
- Start with what's wrong ("Query cannot be empty")
- Add actionable guidance ("Provide a search query...")
- Reference commands where helpful ("Use 'crewchief status'...")
- Keep concise (1-2 sentences)

**No New Validation**: Only improve messages for existing validation rules. Do not add new validation logic.

## Dependencies
**Phase 1 and Phase 2 Complete**: Error infrastructure established

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Breaking existing validation
- Error messages too verbose

**Mitigations**:
- Only modify error messages, not validation logic
- Unit tests validate messages without changing behavior
- Keep messages concise (1-2 sentences)

## Files/Packages Affected
- **Modified**: `packages/maproom-mcp/src/tools/search.ts` (~20 lines modified)
- **New file**: `packages/maproom-mcp/tests/validation-messages.test.ts`

## Estimated Effort
2-3 hours

**Breakdown**:
- Error message improvements: 1 hour
- Validation error formatter: 0.5 hour
- Unit tests: 1-1.5 hours

## Planning References
- [plan.md](../planning/plan.md) - Phase 3 ticket breakdown
- [architecture.md](../planning/architecture.md) - Client-side validation scope
- [quality-strategy.md](../planning/quality-strategy.md) - Testing approach

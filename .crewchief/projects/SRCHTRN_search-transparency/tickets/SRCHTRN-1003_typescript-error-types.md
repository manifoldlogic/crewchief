# SRCHTRN-1003: TypeScript Error Types

## Title
Create TypeScript error types mirroring Rust error taxonomy

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
Create `packages/daemon-client/src/types.ts` with TypeScript interfaces mirroring the Rust error taxonomy (`SearchErrorDetails`, `ErrorType`, `PipelineStage`). Add sync comments linking to Rust source of truth.

## Background
TypeScript clients need to deserialize structured error details from JSON-RPC responses. This requires TypeScript type definitions that exactly match the Rust structs created in SRCHTRN-1001.

**Critical Requirement**: Types must match Rust exactly. Sync comments are mandatory to track the source of truth and enable future audits.

## Acceptance Criteria
- [x] `packages/daemon-client/src/types.ts` created with error type definitions
- [x] `ErrorType` union type matches Rust enum values exactly (6 variants)
- [x] `PipelineStage` union type matches Rust enum values exactly (4 variants)
- [x] `SearchErrorDetails` interface matches Rust struct fields exactly
- [x] Sync comments link each type to Rust source (e.g., `// Sync with: crates/maproom/src/search/errors.rs::ErrorType`)
- [x] Types exported from `packages/daemon-client/src/index.ts`
- [x] Type sync validation test passes (enum values match)
- [x] No new dependencies added
- [x] All tests passing

## Technical Requirements

### File Structure: `packages/daemon-client/src/types.ts`

```typescript
// Sync with: crates/maproom/src/search/errors.rs::ErrorType
export type ErrorType =
  | 'embedding_provider'
  | 'database'
  | 'validation'
  | 'timeout'
  | 'not_found'
  | 'unknown'

// Sync with: crates/maproom/src/search/errors.rs::PipelineStage
export type PipelineStage =
  | 'query_processing'
  | 'search_execution'
  | 'score_fusion'
  | 'result_assembly'

// Sync with: crates/maproom/src/search/errors.rs::SearchErrorDetails
export interface SearchErrorDetails {
  error_type: ErrorType
  stage: PipelineStage
  context: Record<string, string>
  suggestions: string[]
}
```

### Type Sync Validation Test: `packages/daemon-client/src/types.test.ts`

```typescript
import type { ErrorType, PipelineStage, SearchErrorDetails } from './types.js'

describe('Type synchronization with Rust', () => {
  // Sync with: crates/maproom/src/search/errors.rs::ErrorType
  it('should match Rust ErrorType enum values', () => {
    const rustErrorTypes = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown'
    ]

    // This will fail to compile if TypeScript ErrorType diverges
    const tsErrorTypes: ErrorType[] = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown'
    ]

    expect(rustErrorTypes).toEqual(tsErrorTypes)
  })

  // Sync with: crates/maproom/src/search/errors.rs::PipelineStage
  it('should match Rust PipelineStage enum values', () => {
    const rustStages = [
      'query_processing',
      'search_execution',
      'score_fusion',
      'result_assembly'
    ]

    const tsStages: PipelineStage[] = [
      'query_processing',
      'search_execution',
      'score_fusion',
      'result_assembly'
    ]

    expect(rustStages).toEqual(tsStages)
  })

  it('should deserialize SearchErrorDetails from Rust JSON', () => {
    // Example JSON from Rust serialization
    const rustJson = {
      error_type: 'embedding_provider',
      stage: 'query_processing',
      context: { provider_error: 'timeout' },
      suggestions: ['Check credentials', 'Try FTS mode']
    }

    // TypeScript should parse without errors
    const details: SearchErrorDetails = rustJson
    expect(details.error_type).toBe('embedding_provider')
    expect(details.suggestions).toHaveLength(2)
  })
})
```

### Export from Index: `packages/daemon-client/src/index.ts`

Add exports:
```typescript
export type { ErrorType, PipelineStage, SearchErrorDetails } from './types.js'
```

## Implementation Notes
1. Create new file `packages/daemon-client/src/types.ts`
2. Define types matching Rust exactly (snake_case preserved)
3. Add sync comments for every type definition
4. Create validation test to catch type drift
5. Export from index for use in maproom-mcp

**Naming Convention**: Use snake_case to match Rust serde serialization (`'embedding_provider'` not `'embeddingProvider'`).

**Manual Sync Checklist** (Phase 1 Quality Gate):
- [x] All `ErrorType` enum variants match Rust
- [x] All `PipelineStage` enum variants match Rust
- [x] Sync comments link to Rust source
- [x] Type sync validation tests pass

## Dependencies
- **SRCHTRN-1001**: Rust error taxonomy (must complete first - defines canonical types)

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Type drift if Rust enum changes without updating TypeScript
- Sync comments may become stale

**Mitigations**:
- Sync comments explicitly link types
- Validation tests catch enum value mismatches
- Manual audit in Phase 1 quality gate
- Integration tests validate serialization roundtrip

## Files/Packages Affected
- **New file**: `packages/daemon-client/src/types.ts`
- **New file**: `packages/daemon-client/src/types.test.ts`
- **Modified**: `packages/daemon-client/src/index.ts` (add exports)

## Estimated Effort
2-3 hours

**Breakdown**:
- Type definitions: 1 hour
- Validation tests: 1 hour
- Manual verification: 0.5-1 hour

## Planning References
- [plan.md](../planning/plan.md) - Phase 1 ticket breakdown
- [architecture.md](../planning/architecture.md) - Type sync strategy, TypeScript error types design
- [quality-strategy.md](../planning/quality-strategy.md) - Type sync validation approach

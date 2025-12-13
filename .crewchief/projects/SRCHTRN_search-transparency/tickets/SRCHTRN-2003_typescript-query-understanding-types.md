# SRCHTRN-2003: TypeScript Query Understanding Types

## Title
Create TypeScript interfaces for query understanding metadata

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
Add `QueryUnderstanding`, `QueryFilters`, and `TimingBreakdown` interfaces to `packages/daemon-client/src/types.ts`, mirroring Rust structures from SRCHTRN-2001. Update search response type to include optional `understanding` field.

## Background
Phase 2 Rust structures (SRCHTRN-2001) define query understanding metadata. TypeScript clients need matching interfaces to deserialize metadata from search responses. This follows the same type sync pattern established in Phase 1.

**Type Sync**: TypeScript mirrors Rust exactly, with sync comments linking to source of truth.

## Acceptance Criteria
- [ ] `QueryUnderstanding` interface matches Rust struct exactly
- [ ] `QueryFilters` interface matches Rust struct exactly
- [ ] `TimingBreakdown` interface matches Rust struct exactly
- [ ] `SearchMetadata` interface extended with optional `understanding` field
- [ ] Sync comments link each interface to Rust source
- [ ] Type sync validation test passes (field names and types match)
- [ ] Types exported from `packages/daemon-client/src/index.ts`
- [ ] All tests passing

## Technical Requirements

### Extend `packages/daemon-client/src/types.ts`

```typescript
// Sync with: crates/maproom/src/search/results.rs::QueryUnderstanding
export interface QueryUnderstanding {
  mode: 'code' | 'text' | 'auto'
  tokens: string[]
  expanded_terms: string[]
  filters: QueryFilters
  fusion_strategy: string
  timing: TimingBreakdown
}

// Sync with: crates/maproom/src/search/results.rs::QueryFilters
export interface QueryFilters {
  repo_id: number
  worktree_id: number | null
  file_types: string[]
  recency_threshold: string | null
}

// Sync with: crates/maproom/src/search/results.rs::TimingBreakdown
export interface TimingBreakdown {
  query_processing_ms: number
  search_execution_ms: number
  score_fusion_ms: number
  result_assembly_ms: number
  total_ms: number
}

// Extend existing SearchMetadata interface
export interface SearchMetadata {
  // ... existing fields ...

  // Query understanding metadata (optional, added in Phase 2)
  understanding?: QueryUnderstanding
}
```

### Type Sync Validation Test: Extend `packages/daemon-client/src/types.test.ts`

```typescript
describe('Type synchronization - Query Understanding', () => {
  it('should deserialize QueryUnderstanding from Rust JSON', () => {
    // Example JSON from Rust serialization
    const rustJson = {
      mode: 'auto',
      tokens: ['authenticate', 'user'],
      expanded_terms: ['auth', 'login', 'authentication'],
      filters: {
        repo_id: 1,
        worktree_id: 2,
        file_types: [],
        recency_threshold: null
      },
      fusion_strategy: 'reciprocal_rank_fusion',
      timing: {
        query_processing_ms: 4.2,
        search_execution_ms: 35.8,
        score_fusion_ms: 2.1,
        result_assembly_ms: 6.4,
        total_ms: 48.5
      }
    }

    // TypeScript should parse without errors
    const understanding: QueryUnderstanding = rustJson
    expect(understanding.mode).toBe('auto')
    expect(understanding.tokens).toEqual(['authenticate', 'user'])
    expect(understanding.timing.total_ms).toBe(48.5)
  })

  it('should handle optional understanding field', () => {
    // Metadata without understanding (backward compatibility)
    const metadataWithout = {
      // ... other metadata fields ...
    }

    const metadata1: SearchMetadata = metadataWithout
    expect(metadata1.understanding).toBeUndefined()

    // Metadata with understanding
    const metadataWith = {
      // ... other metadata fields ...
      understanding: {
        mode: 'code',
        tokens: ['test'],
        expanded_terms: [],
        filters: {
          repo_id: 1,
          worktree_id: null,
          file_types: [],
          recency_threshold: null
        },
        fusion_strategy: 'linear',
        timing: {
          query_processing_ms: 1.0,
          search_execution_ms: 2.0,
          score_fusion_ms: 3.0,
          result_assembly_ms: 4.0,
          total_ms: 10.0
        }
      }
    }

    const metadata2: SearchMetadata = metadataWith
    expect(metadata2.understanding?.mode).toBe('code')
  })

  it('should validate timing breakdown structure', () => {
    const timing: TimingBreakdown = {
      query_processing_ms: 4.2,
      search_execution_ms: 35.8,
      score_fusion_ms: 2.1,
      result_assembly_ms: 6.4,
      total_ms: 48.5
    }

    // Verify all fields are numbers
    expect(typeof timing.query_processing_ms).toBe('number')
    expect(typeof timing.total_ms).toBe('number')

    // Verify total is sum of parts
    const sum = timing.query_processing_ms
      + timing.search_execution_ms
      + timing.score_fusion_ms
      + timing.result_assembly_ms
    expect(sum).toBeCloseTo(timing.total_ms, 1)
  })
})
```

### Export from Index: Extend `packages/daemon-client/src/index.ts`

```typescript
export type {
  // ... existing exports ...
  QueryUnderstanding,
  QueryFilters,
  TimingBreakdown,
} from './types.js'
```

## Implementation Notes
1. Add interfaces to existing `packages/daemon-client/src/types.ts`
2. Match Rust field names exactly (snake_case)
3. Use TypeScript union types for enums (`'code' | 'text' | 'auto'`)
4. Add sync comments for each interface
5. Extend validation tests from Phase 1

**Type Mapping**:
- Rust `i64` → TypeScript `number`
- Rust `f64` → TypeScript `number`
- Rust `Option<T>` → TypeScript `T | null`
- Rust `Vec<T>` → TypeScript `T[]`
- Rust `String` → TypeScript `string`

**Manual Sync Checklist** (Phase 2 Quality Gate):
- [ ] All `QueryUnderstanding` fields match Rust
- [ ] All `QueryFilters` fields match Rust
- [ ] All `TimingBreakdown` fields match Rust
- [ ] Sync comments link to Rust source
- [ ] Type sync validation tests pass

## Dependencies
- **SRCHTRN-2001**: Query understanding structures (must complete first - defines canonical types)

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Type drift if Rust structs change
- Field name mismatches (snake_case vs camelCase)

**Mitigations**:
- Sync comments link to Rust source
- Validation tests catch structure mismatches
- Use snake_case to match serde serialization
- Manual audit in Phase 2 quality gate

## Files/Packages Affected
- **Modified**: `packages/daemon-client/src/types.ts` (~30 lines added)
- **Modified**: `packages/daemon-client/src/types.test.ts` (~40 lines added)
- **Modified**: `packages/daemon-client/src/index.ts` (add exports)

## Estimated Effort
2-3 hours

**Breakdown**:
- Interface definitions: 1 hour
- Validation tests: 1-2 hours
- Manual verification: 0.5 hour

## Planning References
- [plan.md](../planning/plan.md) - Phase 2 ticket breakdown
- [architecture.md](../planning/architecture.md) - Type sync strategy, query understanding types
- [quality-strategy.md](../planning/quality-strategy.md) - Type sync validation approach

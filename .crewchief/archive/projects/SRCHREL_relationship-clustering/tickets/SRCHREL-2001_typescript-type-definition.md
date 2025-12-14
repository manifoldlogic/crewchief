# Ticket: [SRCHREL-2001]: TypeScript Type Definition and Sync

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
- typescript-expert
- test-runner
- verify-ticket
- commit-ticket

## Summary
Define the TypeScript `RelatedChunkResult` interface in daemon-client to mirror the Rust struct, update `SearchParams` interface, and create validation tests for type synchronization.

## Background
The daemon client needs TypeScript types that exactly match the Rust types for JSON serialization/deserialization to work correctly. Type synchronization is critical - divergence causes runtime errors in MCP clients. This follows the pattern established by SRCHCONF.

This implements Phase 2 deliverables: TypeScript interface, SearchParams update, type sync validation tests.

## Acceptance Criteria
- [x] `RelatedChunkResult` interface defined in `packages/daemon-client/src/types.ts`
- [x] All 10 fields match Rust struct exactly (names, types, nullability)
- [x] Sync comment references Rust struct location
- [x] `SearchParams` interface updated with `include_related?: boolean`
- [x] Type sync validation test created in `packages/daemon-client/src/types.test.ts`
- [x] Test validates field presence and types
- [x] All TypeScript tests pass with `npm test`
- [x] No TypeScript compilation errors with `tsc --noEmit`

## Technical Requirements

### RelatedChunkResult Interface
Add to `packages/daemon-client/src/types.ts`:

```typescript
/**
 * Lightweight metadata for a related chunk discovered via graph traversal.
 *
 * Sync with: crates/maproom/src/search/results.rs::RelatedChunkResult
 */
export interface RelatedChunkResult {
  /** Chunk ID for requesting full context */
  chunk_id: number
  /** File path relative to repository root */
  relpath: string
  /** Symbol name */
  symbol_name: string | null
  /** Symbol kind */
  kind: string
  /** Start line (1-based) */
  start_line: number
  /** End line (1-based) */
  end_line: number
  /** Content preview */
  preview: string
  /** Graph traversal depth (1 or 2) */
  depth: number
  /** Decay-adjusted relevance (0.0-1.0) */
  relevance: number
  /** Relationship type */
  relationship_type: string
}
```

### SearchParams Update
```typescript
export interface SearchParams {
  query: string
  repo: string
  worktree?: string
  limit?: number
  mode?: 'fts' | 'vector' | 'hybrid'
  debug?: boolean
  include_confidence?: boolean  // From SRCHCONF
  include_related?: boolean     // NEW
  deduplicate?: boolean
}
```

### ChunkSearchResult Update
```typescript
export interface ChunkSearchResult {
  // ... existing fields ...
  confidence?: ConfidenceSignals  // From SRCHCONF
  related?: RelatedChunkResult[]  // NEW
}
```

### Type Sync Validation Test
Add to `packages/daemon-client/src/types.test.ts`:

```typescript
describe('RelatedChunkResult type sync', () => {
  it('matches Rust struct fields exactly', () => {
    const sample: RelatedChunkResult = {
      chunk_id: 123,
      relpath: 'src/auth/handler.ts',
      symbol_name: 'authenticate',
      kind: 'function',
      start_line: 10,
      end_line: 25,
      preview: 'export function authenticate() {...',
      depth: 2,
      relevance: 0.7,
      relationship_type: 'call',
    };

    // Validate all fields exist and have correct types
    expect(typeof sample.chunk_id).toBe('number');
    expect(typeof sample.relpath).toBe('string');
    expect(typeof sample.symbol_name).toBe('string');
    expect(typeof sample.kind).toBe('string');
    expect(typeof sample.start_line).toBe('number');
    expect(typeof sample.end_line).toBe('number');
    expect(typeof sample.preview).toBe('string');
    expect(typeof sample.depth).toBe('number');
    expect(typeof sample.relevance).toBe('number');
    expect(typeof sample.relationship_type).toBe('string');
  });

  it('handles null symbol_name', () => {
    const sample: RelatedChunkResult = {
      chunk_id: 123,
      relpath: 'src/config.ts',
      symbol_name: null,  // Anonymous chunk
      kind: 'module',
      start_line: 1,
      end_line: 100,
      preview: 'export const config = {...',
      depth: 1,
      relevance: 0.5,
      relationship_type: 'import',
    };

    expect(sample.symbol_name).toBeNull();
  });

  it('validates optional related field on ChunkSearchResult', () => {
    const result: ChunkSearchResult = {
      chunk_id: 1,
      // ... other required fields ...
      related: [
        {
          chunk_id: 2,
          relpath: 'src/related.ts',
          symbol_name: 'helper',
          kind: 'function',
          start_line: 5,
          end_line: 10,
          preview: 'export function helper() {...',
          depth: 1,
          relevance: 0.8,
          relationship_type: 'import',
        },
      ],
    };

    expect(result.related).toBeDefined();
    expect(Array.isArray(result.related)).toBe(true);
    expect(result.related!.length).toBe(1);
  });
});
```

## Implementation Notes

Type synchronization pattern from SRCHCONF:
- Comment references exact Rust file and struct name
- Field names must match exactly (snake_case in both Rust and TypeScript)
- Rust `i64` → TypeScript `number`
- Rust `Option<String>` → TypeScript `string | null`
- Rust `f32` → TypeScript `number`

JSON serialization considerations:
- Serde serializes Rust `Option::None` as omitted field
- TypeScript `undefined` represents omitted field
- TypeScript `null` represents explicit null value (matches Rust `Option::Some(None)`)

Testing strategy:
- Create sample objects with all fields
- Validate types with `typeof`
- Test optional field handling (presence/absence)
- Test null vs undefined semantics

## Dependencies
- SRCHREL-1001 (Rust RelatedChunkResult struct must exist)

## Risk Assessment
- **Risk**: Field name typo causes runtime error (hard to debug)
  - **Mitigation**: Validation tests catch mismatches; TYPE_SYNC comment makes intent explicit
- **Risk**: Type mismatch (e.g., string vs number) not caught until runtime
  - **Mitigation**: TypeScript compiler catches obvious mismatches; tests validate sample objects

## Files/Packages Affected
- `packages/daemon-client/src/types.ts` (add RelatedChunkResult, update SearchParams and ChunkSearchResult)
- `packages/daemon-client/src/types.test.ts` (add validation tests)

## Verification Notes
The verify-ticket agent should check:
- TypeScript compiles without errors: `cd packages/daemon-client && tsc --noEmit`
- Tests pass: `cd packages/daemon-client && npm test types.test.ts`
- All 10 fields present in RelatedChunkResult interface
- Field types match Rust struct (number, string, string | null)
- Sync comment correctly references Rust file
- `include_related` parameter present in SearchParams
- `related` optional field present in ChunkSearchResult
- Validation tests cover all fields and null handling

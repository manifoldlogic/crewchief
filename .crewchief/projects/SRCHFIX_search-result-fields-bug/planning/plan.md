# Execution Plan: Search Result Fields Bug Fix

## Overview

This is a simple bug fix with minimal scope. Two phases: (1) fix the data plumbing, (2) verify it works.

## Phases

### Phase 1: Fix Data Serialization and Types

**Objective**: Add missing fields to daemon JSON response and update TypeScript interfaces.

**Deliverables**:
- Rust daemon serializes chunk_id, symbol_name, kind
- TypeScript interfaces match Rust struct fields
- Mapping code uses actual values instead of hardcoded defaults
- Obsolete fallback code removed

**Agent Assignments**:
- **rust-expert**: Update Rust daemon JSON serialization
- **typescript-expert**: Update daemon-client interface
- **typescript-expert**: Update maproom-mcp mapping code

**Tasks**:

#### Task 1.1: Update Rust Daemon Serialization

**File**: `/workspace/crates/maproom/src/daemon/mod.rs` (line 332-340)

**Changes**:
Add one line to serialize chunk_id:

```rust
.map(|hit| {
    serde_json::json!({
        "chunk_id": hit.chunk_id,        // ADD THIS LINE
        "score": hit.score,
        "start_line": hit.start_line,
        "end_line": hit.end_line,
        "symbol_name": hit.symbol_name,
        "kind": hit.kind,
        "file_path": hit.file_relpath,
    })
})
```

**Validation**: `cargo build` succeeds, no clippy warnings.

#### Task 1.2: Update TypeScript Daemon Client Interface (Main Package)

**File**: `/workspace/packages/daemon-client/src/client.ts` (line 26-41)

**Changes**:
1. Add sync comment pointing to Rust struct
2. Rename `chunk_index` → `chunk_id`
3. Add `symbol_name: string | null`
4. Add `kind: string`

```typescript
/**
 * Search result from daemon
 *
 * Sync with: crates/maproom/src/db/mod.rs SearchHit
 */
export interface SearchResult {
  hits: Array<{
    chunk_id: number           // RENAMED from chunk_index
    file_path: string
    start_line: number
    end_line: number
    symbol_name: string | null // ADDED
    kind: string               // ADDED
    content: string
    score: number
  }>
  total: number
  query_embedding_time_ms?: number
  search_time_ms?: number
}
```

**Validation**: TypeScript compilation succeeds.

#### Task 1.2b: Update Vendored Daemon Client Interface

**File**: `/workspace/packages/maproom-mcp/src/daemon-client/client.ts` (line 31-45)

**Note**: This is a vendored copy of daemon-client that exists in the MCP server package. Both copies must be kept in sync.

**Changes**: Apply the exact same changes as Task 1.2:
1. Add sync comment pointing to Rust struct
2. Rename `chunk_index` → `chunk_id`
3. Add `symbol_name: string | null`
4. Add `kind: string`

**Validation**:
- TypeScript compilation succeeds
- Interface matches daemon-client/src/client.ts exactly
- Add comment noting this is a vendored copy that must stay in sync

#### Task 1.3: Update Maproom MCP Mapping Code

**File**: `/workspace/packages/maproom-mcp/src/tools/search.ts` (line 304-340)

**Note**: The RustSearchHit interface (lines 108-118) already has symbol_name and kind fields. This task updates the mapping code to use the daemon values instead of hardcoded defaults.

**Changes**:

1. **Verify RustSearchHit interface** (line 108-118):
```typescript
// This interface should already have symbol_name and kind - verify it matches:
interface RustSearchHit {
  score: number
  file_relpath: string
  symbol_name: string | null  // ✓ Already present
  kind: string                // ✓ Already present
  start_line: number
  end_line: number
  base_score?: number
  kind_mult?: number
  exact_mult?: number
}
```

2. **Update rustOutput mapping** (line 307-318):
```typescript
const rustOutput: RustSearchOutput = {
  hits: daemonResult.hits.map((hit) => ({
    file_relpath: hit.file_path,
    start_line: hit.start_line,
    end_line: hit.end_line,
    symbol_name: hit.symbol_name || '', // Use actual value, fallback to ''
    kind: hit.kind,                     // Use actual value
    score: hit.score,
    base_score: undefined,
    kind_mult: undefined,
    exact_mult: undefined,
  })),
}
```

2. **Remove obsolete chunkIdMap** (line 323-325):
```typescript
// DELETE these lines:
// const chunkIdMap = new Map<string, number>()
```

3. **Update hits mapping** (line 328-340):
```typescript
const hits: SearchResult[] = rustOutput.hits.map((hit, index) => {
  const daemonHit = daemonResult.hits[index]

  // Validate chunk_id is present
  if (!daemonHit.chunk_id || daemonHit.chunk_id === 0) {
    log.warn({ hit: daemonHit }, 'Invalid chunk_id in search result')
  }

  // Build SearchResult with optional score_breakdown
  const result: SearchResult = {
    chunk_id: daemonHit.chunk_id,  // Use daemon value directly
    symbol_name: hit.symbol_name,
    kind: hit.kind,
    // ... rest of mapping unchanged
  }
  // ...
})
```

4. **Update comments** to remove misleading "Phase 2 enhancement" notes and "not available from daemon" statements.

**Validation**: TypeScript compilation succeeds, no type errors.

#### Task 1.4: Search for chunk_index Usage

**Search patterns**:
- `chunk_index` (exact match)
- `chunkIndex` (camelCase variant)

**Files to check**:
- `packages/maproom-mcp/src/**/*.ts`
- `packages/vscode-maproom/src/**/*.ts`
- `packages/daemon-client/src/**/*.ts`

**Action**: Replace any usage with `chunk_id`.

**Expected result**: Only find the interface definition (which we're updating).

**Dependencies**: None (Phase 1 tasks can run in parallel).

**Estimated effort**: 45 minutes (increased from 30 minutes to account for Task 1.2b).

---

### Phase 2: Validation and Testing

**Objective**: Verify all three fields are correctly populated in search results.

**Deliverables**:
- Integration test passes
- Manual test confirms fields are present
- Context retrieval works with search-provided chunk_id

**Agent Assignments**:
- **unit-test-runner**: Run existing tests
- **typescript-expert**: Create integration test
- **verify-ticket**: Validate acceptance criteria

**Tasks**:

#### Task 2.1: Run Existing Tests

**Commands**:
```bash
# Rust tests
cd /workspace/crates/maproom
cargo test

# TypeScript tests
cd /workspace/packages/daemon-client
pnpm test

cd /workspace/packages/maproom-mcp
pnpm test
```

**Expected**: All existing tests pass (no regressions).

#### Task 2.2: Create Integration Test

**File**: `/workspace/packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts` (new file)

**Prerequisites**: See quality-strategy.md "Test Environment Setup" section for database requirements. Tests should skip gracefully if database is unavailable.

**Test Environment**:
- Database: `~/.maproom/maproom.db` (or `MAPROOM_DATABASE_URL`)
- Requires: crewchief repository indexed in a worktree named "main"
- Fallback: Skip tests with warning if database or worktree not found

**Test cases**:

1. **Test: chunk_id is populated**
   - Perform search
   - Assert hit.chunk_id > 0
   - Assert hit.chunk_id is a number

2. **Test: symbol_name is populated for functions**
   - Search for known function (e.g., "search")
   - Assert at least one hit has non-empty symbol_name
   - Verify symbol_name matches expected function name

3. **Test: kind is populated**
   - Search for known code
   - Assert hit.kind is one of valid values (function, class, method, etc.)
   - Assert kind is not empty string

4. **Test: null symbol_name for anonymous chunks**
   - Search for anonymous code (if available in test fixtures)
   - Assert symbol_name is null or empty string (not undefined)

5. **Test: context retrieval works**
   - Perform search to get chunk_id
   - Call context tool with chunk_id
   - Assert context returned successfully

**Validation**: All tests pass.

#### Task 2.3: Manual Validation

**Steps**:

1. **Start MCP server**:
   ```bash
   cd /workspace/packages/maproom-mcp
   pnpm build
   npx @crewchief/maproom-mcp
   ```

2. **Perform search** (via MCP client or direct daemon call):
   ```typescript
   const result = await client.search({
     query: 'function search',
     repo: 'crewchief',
     worktree: 'main'
   })
   ```

3. **Verify fields**:
   ```typescript
   console.log(result.hits[0].chunk_id)     // Should be > 0
   console.log(result.hits[0].symbol_name)  // Should be function name or null
   console.log(result.hits[0].kind)         // Should be "function", "class", etc.
   ```

4. **Test context retrieval**:
   ```typescript
   const context = await client.context({
     chunk_id: result.hits[0].chunk_id
   })
   console.log(context.items.length) // Should be > 0
   ```

**Expected results**:
- chunk_id is a valid positive integer
- symbol_name is either null or a non-empty string
- kind is a non-empty string like "function", "class", "method"
- Context tool returns results using chunk_id from search

**Validation**: Manual test confirms all fields work.

**Dependencies**: Phase 1 must be complete.

**Estimated effort**: 1 hour.

---

## Dependencies

### Cross-Phase Dependencies

- Phase 2 depends on Phase 1 completion
- All Phase 1 tasks are independent and can run in parallel

### External Dependencies

None

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Field name change breaks consumers | Low | Medium | Search codebase for chunk_index usage before renaming |
| Null symbol_name breaks mapping | Low | Medium | Add null handling with fallback to empty string |
| Tests fail due to missing database | Low | High | Use existing test database or skip integration tests |
| TypeScript compilation errors | Low | High | Validate types incrementally as changes are made |

## Success Metrics

### Phase 1 Complete When:
- [ ] Rust daemon serializes chunk_id in JSON response
- [ ] TypeScript SearchResult interface has chunk_id, symbol_name, kind fields
- [ ] Mapping code uses actual values from daemon (not hardcoded defaults)
- [ ] All obsolete fallback code removed
- [ ] TypeScript and Rust compilation succeed with no errors

### Phase 2 Complete When:
- [ ] Integration test passes (all fields populated correctly)
- [ ] Manual test confirms fields are present and valid
- [ ] Context retrieval works using chunk_id from search results
- [ ] No regressions in existing tests

### Project Complete When:
- [ ] All Phase 1 and Phase 2 criteria met
- [ ] Code committed to main branch
- [ ] Documentation updated (if needed)

## Rollback Plan

If issues discovered:

1. **Revert Rust changes**: Remove `"chunk_id": hit.chunk_id` line
2. **Revert TypeScript interface**: Change `chunk_id` back to `chunk_index`
3. **Revert mapping code**: Restore hardcoded empty strings

**Risk**: Very low. Changes are additive and localized.

## Post-Completion

### Documentation Updates

1. **Update CLAUDE.md** (if type sync examples need updating)
2. **Update daemon-client README** (mention chunk_id availability)

### Monitoring

After deployment:
- Monitor MCP server logs for warnings about invalid chunk_id
- Check if VSCode extension displays symbol names correctly
- Verify context retrieval usage increases (now that it works)

### Future Enhancements

Out of scope for this fix, but now possible:

1. **Filter by kind**: Allow clients to filter search results by symbol type
2. **Group by symbol**: Display results grouped by function/class
3. **Symbol-aware ranking**: Use kind field for better semantic ranking (already implemented, just not working due to empty values)

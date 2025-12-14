# Quality Strategy: Search Result Fields Bug Fix

## Testing Philosophy

**Goal**: Confidence that all three fields (chunk_id, symbol_name, kind) are correctly populated in search results.

**Approach**: Minimal tests that verify the actual bug is fixed, not comprehensive coverage of all search scenarios.

**Principle**: Test for confidence, not metrics. Focus on critical paths.

## Test Types

### Unit Tests

**Scope**: Verify individual components serialize/deserialize correctly

**Tools**:
- Rust: cargo test
- TypeScript: vitest

**Coverage Target**: Critical serialization paths only (not 100%)

### Integration Tests

**Scope**: End-to-end field flow from daemon through MCP to client

**Approach**:
- Start daemon, perform search, verify fields present
- Use search result chunk_id to fetch context
- Validate null handling for symbol_name

### End-to-End Tests

**Scope**: Manual validation with real MCP server

**Approach**:
- Build and run MCP server
- Perform real searches via MCP client
- Inspect JSON responses for field presence

## Critical Paths

The following paths MUST be tested:

1. **Rust daemon serializes chunk_id**: JSON response includes chunk_id field with valid value
2. **TypeScript interface accepts all fields**: Compilation succeeds with chunk_id, symbol_name, kind
3. **Field mapping preserves values**: Daemon values flow to MCP response without hardcoded overrides
4. **Context retrieval works**: chunk_id from search enables successful context lookup
5. **Null handling**: symbol_name can be null without breaking mapping code

## Test Environment Setup

**Database Location**:
- Primary: `~/.maproom/maproom.db`
- Override: Set `MAPROOM_DATABASE_URL` environment variable
- Verification: Check database file exists before running integration tests

**Required Data**:
- Repository: `crewchief` must be indexed
- Worktree: A worktree named `main` must exist
- Minimum chunks: At least 10 indexed chunks for meaningful tests

**Setup Verification**:
```bash
# Check database exists
ls -lh ~/.maproom/maproom.db

# Verify crewchief repository indexed (via daemon or MCP)
# Should return worktrees including "main"
```

**Fallback Behavior**:
- If database doesn't exist: Skip integration tests with warning
- If crewchief repo not indexed: Skip integration tests with warning
- If worktree "main" missing: Skip integration tests with warning
- CI/CD environments: Use test fixture database or skip gracefully

**Test Isolation**:
- Tests are read-only (no database modifications)
- Multiple test runs can execute concurrently (no locking issues)
- Tests use real indexed data (not mocks)

## Test Data Strategy

**Existing database**: Use existing maproom.db with crewchief repository indexed

**Known symbols**: Search for common functions like "search", "scan", "context"

**Anonymous chunks**: Search for code patterns likely to match anonymous blocks (const, import)

**No test fixtures needed**: Real database provides sufficient test data

## Rust Unit Tests

### Test: JSON Serialization Includes chunk_id

**Location**: `/workspace/crates/maproom/src/daemon/mod.rs` (new test)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_response_includes_chunk_id() {
        let hit = SearchHit {
            chunk_id: 123,
            score: 0.95,
            file_relpath: "src/main.rs".to_string(),
            symbol_name: Some("my_function".to_string()),
            kind: "function".to_string(),
            start_line: 10,
            end_line: 20,
            base_score: None,
            kind_mult: None,
            exact_mult: None,
        };

        let json = serde_json::json!({
            "chunk_id": hit.chunk_id,
            "score": hit.score,
            "start_line": hit.start_line,
            "end_line": hit.end_line,
            "symbol_name": hit.symbol_name,
            "kind": hit.kind,
            "file_path": hit.file_relpath,
        });

        assert_eq!(json["chunk_id"], 123);
        assert_eq!(json["symbol_name"], "my_function");
        assert_eq!(json["kind"], "function");
    }
}
```

## TypeScript Unit Tests

### Test: SearchResult Interface Type Safety

**Location**: `/workspace/packages/daemon-client/src/__tests__/client.test.ts`

```typescript
import { SearchResult } from '../client.js'

describe('SearchResult interface', () => {
  it('should accept chunk_id, symbol_name, and kind', () => {
    const result: SearchResult = {
      hits: [
        {
          chunk_id: 123,
          file_path: 'src/main.ts',
          start_line: 10,
          end_line: 20,
          symbol_name: 'myFunction',
          kind: 'function',
          content: 'function myFunction() {}',
          score: 0.95,
        },
      ],
      total: 1,
    }

    expect(result.hits[0].chunk_id).toBe(123)
    expect(result.hits[0].symbol_name).toBe('myFunction')
    expect(result.hits[0].kind).toBe('function')
  })

  it('should accept null symbol_name', () => {
    const result: SearchResult = {
      hits: [
        {
          chunk_id: 456,
          file_path: 'src/main.ts',
          start_line: 30,
          end_line: 40,
          symbol_name: null,
          kind: 'block',
          content: 'const x = 1',
          score: 0.8,
        },
      ],
      total: 1,
    }

    expect(result.hits[0].symbol_name).toBeNull()
  })
})
```

## Integration Tests

### Test: End-to-End Field Flow

**Location**: `/workspace/packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts` (new file)

**Prerequisites**: Requires test environment setup (see "Test Environment Setup" section above). Tests will skip if database or required data is unavailable.

```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { DaemonClient } from '@crewchief/daemon-client'
import { existsSync } from 'fs'
import { homedir } from 'os'

describe('Search result fields', () => {
  let client: DaemonClient

  beforeAll(async () => {
    // Check if test database exists
    const dbPath = process.env.MAPROOM_DATABASE_URL || `${homedir()}/.maproom/maproom.db`
    if (!existsSync(dbPath)) {
      console.warn('Test database not found - skipping integration tests')
      return
    }

    client = new DaemonClient()
    await client.connect()
  })

  afterAll(async () => {
    if (client) {
      await client.close()
    }
  })

  it('should populate chunk_id, symbol_name, and kind', async () => {
    const result = await client.search({
      query: 'function',
      repo: 'crewchief',
      worktree: 'main',
      limit: 5,
    })

    expect(result.hits.length).toBeGreaterThan(0)

    const firstHit = result.hits[0]

    // chunk_id should be a positive integer
    expect(firstHit.chunk_id).toBeGreaterThan(0)
    expect(Number.isInteger(firstHit.chunk_id)).toBe(true)

    // symbol_name should be defined
    expect(firstHit.symbol_name).toBeDefined()

    // kind should be a non-empty string
    expect(firstHit.kind).toBeTruthy()
    expect(typeof firstHit.kind).toBe('string')
  })

  it('should enable context retrieval with chunk_id', async () => {
    const searchResult = await client.search({
      query: 'search',
      repo: 'crewchief',
      worktree: 'main',
      limit: 1,
    })

    expect(searchResult.hits.length).toBeGreaterThan(0)
    const chunkId = searchResult.hits[0].chunk_id
    expect(chunkId).toBeGreaterThan(0)

    const contextResult = await client.context({
      chunk_id: chunkId.toString(),
    })

    expect(contextResult.items.length).toBeGreaterThan(0)
  })

  it('should handle null symbol_name gracefully', async () => {
    const result = await client.search({
      query: 'const',
      repo: 'crewchief',
      worktree: 'main',
      limit: 10,
    })

    const anonymousHit = result.hits.find(hit => hit.symbol_name === null)

    if (anonymousHit) {
      expect(anonymousHit.symbol_name).toBeNull()
      expect(anonymousHit.chunk_id).toBeGreaterThan(0)
      expect(anonymousHit.kind).toBeTruthy()
    }
  })
})
```

## Manual Testing

### Test Case 1: Verify Fields in Real Search

**Steps**:

1. Build and run MCP server:
   ```bash
   cd /workspace/packages/maproom-mcp
   pnpm build
   pnpm start
   ```

2. Connect via MCP client and run search

3. Inspect the search results JSON:
   ```json
   {
     "hits": [
       {
         "chunk_id": 1234,
         "symbol_name": "authenticateUser",
         "kind": "function",
         "file_path": "src/auth.ts",
         "start_line": 45,
         "end_line": 67,
         "score": 0.92
       }
     ]
   }
   ```

4. Verify:
   - chunk_id is a positive integer (not 0)
   - symbol_name is present and meaningful (or null)
   - kind is present and descriptive

**Success criteria**: All three fields visible and populated with real data.

### Test Case 2: Context Retrieval Roundtrip

**Steps**:

1. Perform search to get chunk_id:
   ```typescript
   const searchResult = await search({ query: 'search', repo: 'crewchief' })
   const chunkId = searchResult.hits[0].chunk_id
   console.log('Got chunk_id:', chunkId)  // Should be > 0
   ```

2. Use chunk_id to get context:
   ```typescript
   const context = await getContext({ chunk_id: chunkId })
   console.log('Context items:', context.items.length)  // Should be > 0
   ```

3. Verify no errors and context is returned

**Success criteria**: chunk_id from search successfully retrieves context.

## Quality Gates

### Pre-Commit

1. **TypeScript compilation**: `pnpm build` succeeds across all packages
2. **Rust compilation**: `cargo build` succeeds
3. **Linting**: `cargo clippy` and `pnpm lint` pass
4. **Unit tests**: All existing tests pass

### Pre-Merge

1. **Integration tests**: New search-fields.test.ts passes
2. **Manual validation**: Manual test cases completed successfully
3. **No regressions**: Existing MCP tools still work (search, context, status)

### Post-Merge

1. **Smoke test**: Run search and context in production environment
2. **Monitor logs**: Check for warnings about invalid chunk_id (should decrease)

## Known Gaps

### Acceptable Gaps (Out of Scope)

1. **No test for every symbol kind**: Not testing all possible values of kind
   - **Why**: Database already stores these correctly, just exposing them

2. **No performance testing**: Not measuring serialization overhead
   - **Why**: Adding fields to JSON has negligible cost

3. **No backward compatibility test**: Not testing if old clients break
   - **Why**: No known clients depend on chunk_index=0 (it was always broken)

### Risks Accepted

1. **Null symbol_name edge cases**: May not catch all scenarios
   - **Mitigation**: Manual testing + production monitoring

2. **Field name confusion**: chunk_id vs chunk_index may confuse developers
   - **Mitigation**: Clear documentation + TypeScript types prevent misuse

## Success Metrics

After deployment, these should improve:

1. **Context retrieval errors**: Should decrease (currently fails with chunk_id=0)
2. **Search result quality**: symbol_name and kind visible in UI
3. **Warning logs**: "Chunk ID not found" warnings should disappear

No specific metrics targets - this is a bug fix, not a feature. Success = bug is fixed.

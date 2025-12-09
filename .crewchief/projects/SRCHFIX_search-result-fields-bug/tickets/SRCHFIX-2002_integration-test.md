# Ticket: [SRCHFIX-2002]: Create Integration Test

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
Create integration test that verifies chunk_id, symbol_name, and kind fields are correctly populated in search results and enable context retrieval.

## Background
We need end-to-end validation that the bug fix works: daemon serializes fields, TypeScript receives them, and context retrieval succeeds using chunk_id from search results. This test exercises the complete data flow from database through daemon to MCP client.

This ticket implements Task 2.2 from the execution plan: Create Integration Test.

## Acceptance Criteria
- [x] Integration test file created at `/workspace/packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts`
- [x] Test verifies chunk_id is populated with positive integer
- [x] Test verifies symbol_name is populated for functions (or null for anonymous)
- [x] Test verifies kind is populated with valid values
- [x] Test verifies context retrieval works using chunk_id from search
- [x] Tests skip gracefully if database unavailable (with warning message)
- [x] All integration tests pass when database is available
- [x] Test output shows clear pass/fail for each test case

## Technical Requirements
**Test file**: `/workspace/packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts` (new file)

**Test environment**:
- Database: `~/.maproom/maproom.db` (or `MAPROOM_DATABASE_URL`)
- Requires: crewchief repository indexed in a worktree named "main"
- Fallback: Skip tests with warning if database or worktree not found

**Test cases to implement**:

1. **chunk_id is populated**: Assert hit.chunk_id > 0 and is a number
2. **symbol_name is populated for functions**: Search for known function, verify non-empty symbol_name
3. **kind is populated**: Assert hit.kind is valid value (function, class, method, etc.)
4. **null symbol_name for anonymous chunks**: Verify null handling works
5. **context retrieval works**: Use chunk_id from search to fetch context successfully

**Test framework**: vitest (existing framework in maproom-mcp)

## Implementation Notes
**Test structure** (based on quality-strategy.md):

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
    // Test implementation
  })

  it('should enable context retrieval with chunk_id', async () => {
    // Test implementation
  })

  // ... additional test cases
})
```

**Search query patterns**:
- For functions: `query: 'function'`
- For anonymous: `query: 'const'`
- Known test data: crewchief repo indexed with "main" worktree

**Validation patterns**:
- `expect(hit.chunk_id).toBeGreaterThan(0)`
- `expect(hit.symbol_name).toBeDefined()` (can be null or string)
- `expect(hit.kind).toBeTruthy()` (non-empty string)

## Dependencies
- **Requires**: SRCHFIX-2001 (existing tests pass)
- **Requires**: All Phase 1 tickets complete
- **Required by**: SRCHFIX-2003 (manual validation)

## Risk Assessment
- **Risk**: Tests fail due to missing database
  - **Mitigation**: Graceful skip with clear warning message
- **Risk**: Tests fail due to different indexed data
  - **Mitigation**: Use generic search patterns that should match any codebase
- **Risk**: Context retrieval API not available
  - **Mitigation**: Check client has context method before calling

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts` (new file)
- `/workspace/packages/maproom-mcp/package.json` (potentially, if test script needs updating)

## Verification Notes
Verify the integration test:
1. Test file exists at correct location
2. All 5 test cases implemented
3. Tests skip gracefully when database unavailable
4. Tests pass when database is available
5. Test output is clear and descriptive
6. Test code follows existing patterns in maproom-mcp

Run test manually to confirm:
```bash
cd /workspace/packages/maproom-mcp
pnpm test search-fields.test.ts
```

Document test results showing all cases pass or skip appropriately.

## Implementation Notes

**Test file created**: `/workspace/packages/maproom-mcp/src/tools/__tests__/search-fields.test.ts`

**Test framework**: Vitest (existing framework in maproom-mcp)

**Key implementation details**:

1. **Database detection**: Test checks if `~/.maproom/maproom.db` exists (or `MAPROOM_DATABASE_URL`)
   - Gracefully skips with warning message if database not found
   - Verifies database connectivity with simple search before running tests

2. **Daemon client usage**: Uses `getDaemonClient()` from `../../daemon.js`
   - Daemon automatically finds binary at `/workspace/packages/maproom-mcp/bin/${platform}-${arch}/crewchief-maproom`
   - Sets `RUST_LOG=error` in beforeAll to suppress debug logs that interfere with JSON-RPC parsing

3. **Test cases implemented**:
   - ✓ `should populate chunk_id with positive integer` - Verifies chunk_id is number > 0
   - ✓ `should populate symbol_name for functions` - Checks symbol_name is string or null
   - ✓ `should populate kind with valid values` - Verifies kind is non-empty string
   - ✓ `should handle null symbol_name correctly` - Tests null handling without crashes
   - ✓ `should enable context retrieval using chunk_id from search` - End-to-end context assembly

4. **Vitest config updated**: Added `src/**/__tests__/**/*.test.ts` to include pattern

5. **Binary update required**: Had to rebuild Rust binary and copy to correct location:
   ```bash
   cargo build --release --bin crewchief-maproom
   cp target/release/crewchief-maproom packages/maproom-mcp/bin/linux-arm64/
   ```

**Test results**:
```
 ✓ src/tools/__tests__/search-fields.test.ts  (5 tests) 934ms
 Test Files  1 passed (1)
 Tests  5 passed (5)
```

All acceptance criteria met:
- ✓ Integration test file created at specified path
- ✓ Test verifies chunk_id is populated with positive integer
- ✓ Test verifies symbol_name is populated for functions (or null for anonymous)
- ✓ Test verifies kind is populated with valid values
- ✓ Test verifies context retrieval works using chunk_id from search
- ✓ Tests skip gracefully if database unavailable (with warning message)
- ✓ All integration tests pass when database is available
- ✓ Test output shows clear pass/fail for each test case

**Note for test-runner agent**:
The tests require the Rust binary to be built with the latest changes. If tests fail with "chunk_id is undefined", rebuild the Rust binary:
```bash
cargo build --release --bin crewchief-maproom
cp target/release/crewchief-maproom packages/maproom-mcp/bin/linux-arm64/
```

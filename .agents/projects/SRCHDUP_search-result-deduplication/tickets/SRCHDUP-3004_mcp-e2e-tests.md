# Ticket: SRCHDUP-3004: MCP E2E tests for deduplication

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create end-to-end tests for the MCP search tool that verify deduplication behavior works correctly through the full integration stack: MCP → daemon-client → Rust daemon → search pipeline.

## Background

E2E tests are the final verification that the deduplication feature works as users will experience it. These tests should use real indexed data with known duplicates and verify the MCP tool returns expected results.

**Reference:** plan.md Phase 3, quality-strategy.md "Level 3: End-to-End Tests"

## Acceptance Criteria

- [ ] New test file for MCP deduplication tests exists
- [ ] Test fixture creates indexed repo with duplicate chunks across worktrees
- [ ] Test verifies default behavior deduplicates results
- [ ] Test verifies `deduplicate: false` returns all duplicates
- [ ] Tests pass: `pnpm test` in maproom-mcp package
- [ ] Tests complete in <30s

## Technical Requirements

### Test File
```typescript
// In packages/maproom-mcp/tests/search-dedup.test.ts

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { createTestServer, callTool } from './helpers';

describe('search tool deduplication', () => {
  let server: TestServer;

  beforeAll(async () => {
    server = await createTestServer();
    await setupDuplicateIndex(server);
  });

  afterAll(async () => {
    await cleanupTestData(server);
  });

  it('deduplicates results by default', async () => {
    const result = await callTool(server, 'search', {
      query: 'validateToken',
      repo: 'dedup-test',
    });

    // Should return only one result despite duplicates in index
    expect(result.results.length).toBe(1);
  });

  it('respects deduplicate=false parameter', async () => {
    const result = await callTool(server, 'search', {
      query: 'validateToken',
      repo: 'dedup-test',
      deduplicate: false,
    });

    // Should return multiple results (duplicates)
    expect(result.results.length).toBeGreaterThan(1);
  });

  it('returns highest-scoring duplicate', async () => {
    const result = await callTool(server, 'search', {
      query: 'validateToken',
      repo: 'dedup-test',
    });

    // Verify the returned result has the best score
    // (implementation depends on how scores are tracked)
  });
});
```

### Test Fixture Setup
```typescript
async function setupDuplicateIndex(server: TestServer) {
  // Create test repo with two worktrees containing same code
  const testRepo = await server.createTestRepo('dedup-test');

  // Main worktree
  await server.createWorktree(testRepo, 'main', {
    'src/auth.ts': `
      export function validateToken(token: string): boolean {
        return token.startsWith('valid-');
      }
    `,
  });

  // Feature worktree with same file
  await server.createWorktree(testRepo, 'feature-auth', {
    'src/auth.ts': `
      export function validateToken(token: string): boolean {
        return token.startsWith('valid-');
      }
    `,
  });

  // Index both worktrees
  await server.indexWorktree(testRepo, 'main');
  await server.indexWorktree(testRepo, 'feature-auth');
}
```

## Implementation Notes

1. **Check existing test patterns** - See how other maproom-mcp tests are structured
2. **Use existing helpers** - Leverage any test utilities already in the package
3. **Test isolation** - Ensure tests don't interfere with each other
4. **Skip if no daemon** - Tests may need to be skipped in CI without daemon

### Test Organization
```
packages/maproom-mcp/
├── tests/
│   ├── search-dedup.test.ts  # NEW
│   └── ... existing tests ...
└── vitest.config.ts (or jest.config.js)
```

### Running Tests
```bash
cd packages/maproom-mcp
pnpm test
# or
pnpm test:e2e search-dedup
```

## Dependencies

- SRCHDUP-3003 (MCP search tool accepts deduplicate param)
- Requires running daemon for full E2E

## Risk Assessment

- **Risk**: E2E tests require full daemon running
  - **Mitigation**: May need to mock daemon or skip in certain CI environments
- **Risk**: Test fixture setup is complex
  - **Mitigation**: Simplify by using existing test utilities if available
- **Risk**: Tests are slow
  - **Mitigation**: Target <30s total, use minimal fixture data

## Files/Packages Affected

- `packages/maproom-mcp/tests/search-dedup.test.ts` (NEW)
- `packages/maproom-mcp/tests/helpers.ts` or similar (if fixture helpers needed)

# Ticket: CTXCLI-4003: Add MCP Context E2E Tests

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
Add end-to-end tests for the MCP context tool, testing the full integration path from MCP server through daemon to SQLite.

## Background
This is Phase 4 (Testing & Polish). E2E tests verify the complete integration: MCP server → daemon client → Rust daemon → SQLite. These are the highest-level tests and should focus on critical paths.

Reference: [planning/quality-strategy.md](../planning/quality-strategy.md) - E2E Tests section

## Acceptance Criteria
- [ ] Test: retrieve context bundle via MCP tool call
- [ ] Test: expand options (callers, callees, tests) work through MCP
- [ ] Test: React-specific options (hooks, jsx_parents, jsx_children) accepted
- [ ] Test: chunk not found returns proper MCP error
- [ ] Test: daemon timeout/unavailable handled gracefully
- [ ] Round-trip time < 200ms for typical requests
- [ ] Tests pass in CI

## Technical Requirements
- Start daemon and MCP server in test setup
- Use MCP client to call tools
- Verify response format matches MCP ContextBundle interface
- Clean up processes in afterAll

## Implementation Notes

### Test Setup
```typescript
// packages/maproom-mcp/tests/context.e2e.test.ts

import { spawn, ChildProcess } from 'child_process'
import { Client } from '@modelcontextprotocol/sdk/client/index.js'

describe('MCP Context Tool E2E', () => {
  let daemonProcess: ChildProcess
  let mcpClient: Client

  beforeAll(async () => {
    // Start daemon
    daemonProcess = spawn('cargo', ['run', '--bin', 'crewchief-maproom', '--', 'serve'], {
      env: { ...process.env, MAPROOM_DATABASE_URL: 'sqlite://./test.db' },
    })

    // Wait for daemon to be ready
    await waitForDaemon()

    // Create MCP client
    mcpClient = new Client({ name: 'test-client', version: '1.0.0' })
    // Connect to MCP server...
  })

  afterAll(async () => {
    await mcpClient?.close()
    daemonProcess?.kill()
  })

  // Tests...
})
```

### Test Cases
```typescript
it('should retrieve context bundle via MCP', async () => {
  const start = performance.now()

  const result = await mcpClient.callTool({
    name: 'context',
    arguments: {
      chunk_id: '1',
      budget_tokens: 6000,
      expand: { callers: true },
    },
  })

  const duration = performance.now() - start

  expect(result.isError).toBe(false)
  const content = JSON.parse(result.content[0].text)
  expect(content.items).toBeDefined()
  expect(content.items.length).toBeGreaterThan(0)
  expect(content.items[0].role).toBe('primary')
  expect(content.total_tokens).toBeLessThanOrEqual(6000)

  // Performance check
  expect(duration).toBeLessThan(200)
})

it('should include callers when expand.callers=true', async () => {
  const result = await mcpClient.callTool({
    name: 'context',
    arguments: {
      chunk_id: '1',
      expand: { callers: true },
    },
  })

  const content = JSON.parse(result.content[0].text)
  const roles = content.items.map((i: any) => i.role)
  expect(roles).toContain('caller')
})

it('should accept React-specific options', async () => {
  const result = await mcpClient.callTool({
    name: 'context',
    arguments: {
      chunk_id: '1',
      expand: {
        hooks: true,
        jsx_parents: true,
        jsx_children: true,
      },
    },
  })

  // Should not error even if no React content found
  expect(result.isError).toBe(false)
})

it('should handle chunk not found error', async () => {
  const result = await mcpClient.callTool({
    name: 'context',
    arguments: {
      chunk_id: '999999',
    },
  })

  expect(result.isError).toBe(true)
  expect(result.content[0].text).toContain('CHUNK_NOT_FOUND')
})

it('should return proper response format', async () => {
  const result = await mcpClient.callTool({
    name: 'context',
    arguments: {
      chunk_id: '1',
      budget_tokens: 6000,
    },
  })

  const content = JSON.parse(result.content[0].text)

  // Verify all expected fields
  expect(content).toHaveProperty('items')
  expect(content).toHaveProperty('total_tokens')
  expect(content).toHaveProperty('budget_tokens')
  expect(content).toHaveProperty('budget_remaining')
  expect(content).toHaveProperty('truncated')
  expect(content).toHaveProperty('metadata')

  // Computed fields should be correct
  expect(content.budget_tokens).toBe(6000)
  expect(content.budget_remaining).toBe(6000 - content.total_tokens)
})
```

## Dependencies
- CTXCLI-3003 (MCP server integration must be complete)
- CTXCLI-4001 (Test database fixture must exist)

## Risk Assessment
- **Risk**: E2E tests slow and flaky
  - **Mitigation**: Use generous timeouts, focus on critical paths only
- **Risk**: Port conflicts when running daemon
  - **Mitigation**: Use random available ports or stdio communication

## Files/Packages Affected
- `packages/maproom-mcp/tests/context.e2e.test.ts` (create)

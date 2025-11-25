# Ticket: MCPSIMP-2005: Update MCP Writer Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update tests for `MCPConfigWriter` to verify that `MAPROOM_DATABASE_URL` and `MAPROOM_EMBEDDING_PROVIDER` are included in the generated MCP configuration for all providers.

## Background
MCPSIMP-2001 updated `buildEnvironment()` to include two new required environment variables. The tests need to be updated to verify this new behavior and ensure the MCP config writer generates correct configurations for the simplified MCP server. This implements Phase 2.5 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] Tests verify `MAPROOM_DATABASE_URL` is included in generated config
- [ ] Tests verify `MAPROOM_EMBEDDING_PROVIDER` is included in generated config
- [ ] Tests verify `MAPROOM_EMBEDDING_PROVIDER` matches the selected provider
- [ ] Tests cover all three provider cases: openai, google, ollama
- [ ] All tests pass when run with `pnpm test`

## Technical Requirements
Add or update tests to cover:

```typescript
describe('MCPConfigWriter.buildEnvironment', () => {
  test('always includes MAPROOM_DATABASE_URL', () => {
    // For each provider type, verify MAPROOM_DATABASE_URL is present
    // Expected value: 'postgresql://maproom:maproom@localhost:5433/maproom'
  })

  test('always includes MAPROOM_EMBEDDING_PROVIDER', () => {
    // For each provider type, verify MAPROOM_EMBEDDING_PROVIDER matches
  })

  test('includes OPENAI_API_KEY for openai provider', () => {
    // Verify openai provider includes both base vars AND OPENAI_API_KEY
  })

  test('includes GOOGLE_APPLICATION_CREDENTIALS for google provider', () => {
    // Verify google provider includes both base vars AND credentials
  })

  test('no extra env vars for ollama provider', () => {
    // Verify ollama only has MAPROOM_DATABASE_URL and MAPROOM_EMBEDDING_PROVIDER
  })
})
```

## Implementation Notes
- Locate existing tests: likely in `packages/vscode-maproom/src/config/mcp-writer.test.ts` or similar
- If no tests exist, create the test file following the extension's testing patterns
- The `buildEnvironment` method may be private - you may need to test through public methods that call it
- Run tests with `pnpm test` from `packages/vscode-maproom`
- Ensure test output shows all assertions passing

## Dependencies
- **MCPSIMP-2001** (Update MCP Config Writer) - The implementation must be done first

## Risk Assessment
- **Risk**: Tests file doesn't exist or uses different testing framework
  - **Mitigation**: Check extension's test setup; follow existing test patterns
- **Risk**: buildEnvironment is private and not directly testable
  - **Mitigation**: Test through the public API that generates MCP configs

## Files/Packages Affected
- `packages/vscode-maproom/src/config/mcp-writer.test.ts` (create or modify)
- Or equivalent test file based on project structure

# Ticket: MCPSIMP-2001: Update MCP Config Writer

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update `MCPConfigWriter.buildEnvironment()` to include `MAPROOM_DATABASE_URL` and `MAPROOM_EMBEDDING_PROVIDER` in the generated MCP configuration, ensuring the simplified MCP server receives all required environment variables.

## Background
The simplified MCP server (v3.0.0) requires `MAPROOM_DATABASE_URL` to connect to the database and `MAPROOM_EMBEDDING_PROVIDER` to know which embedding provider to use. Currently, `buildEnvironment()` only returns provider-specific API keys (OPENAI_API_KEY, GOOGLE_APPLICATION_CREDENTIALS). This ticket implements Phase 2.1 of the MCP Server Simplification plan.

**Critical**: Without these env vars, the MCP server will use auto-detection which may not work correctly for all users.

## Acceptance Criteria
- [ ] `buildEnvironment()` always includes `MAPROOM_DATABASE_URL` in returned environment
- [ ] `buildEnvironment()` always includes `MAPROOM_EMBEDDING_PROVIDER` in returned environment
- [ ] Provider-specific API keys still work correctly (openai, google, ollama)
- [ ] Generated mcp.json includes both new env vars when inspected
- [ ] Existing tests pass (update if necessary)

## Technical Requirements
Update the `buildEnvironment()` method in `packages/vscode-maproom/src/config/mcp-writer.ts`:

```typescript
private buildEnvironment(provider: EmbeddingProvider): Record<string, string> {
  const env: Record<string, string> = {
    // Always include database URL (required for MCP server)
    MAPROOM_DATABASE_URL: 'postgresql://maproom:maproom@localhost:5433/maproom',
    // Always include provider selection
    MAPROOM_EMBEDDING_PROVIDER: provider,
  }

  // Add provider-specific credentials
  switch (provider) {
    case 'openai':
      env.OPENAI_API_KEY = '${env:OPENAI_API_KEY}'
      break
    case 'google':
      env.GOOGLE_APPLICATION_CREDENTIALS = '${env:GOOGLE_APPLICATION_CREDENTIALS}'
      break
    case 'ollama':
      // Ollama doesn't need environment variables
      break
  }

  return env
}
```

## Implementation Notes
- The hardcoded `localhost:5433` URL is correct for VSCode extension users (extension manages Docker)
- DevContainer users will rely on MCP server's auto-detection or can set `MAPROOM_DATABASE_URL` in their shell
- The `${env:VAR}` syntax is VSCode's env var interpolation - it reads from user's environment
- Test the generated config by:
  1. Using the extension to set up MCP
  2. Inspecting the generated `.vscode/mcp.json` or equivalent
  3. Verifying both new env vars are present

## Dependencies
- None (this can be done in parallel with Phase 1)

## Risk Assessment
- **Risk**: Breaking existing MCP configurations
  - **Mitigation**: Adding env vars is additive, not breaking; existing configs continue to work
- **Risk**: Hardcoded URL doesn't work for all scenarios
  - **Mitigation**: MCP server has auto-detection as fallback; documented override via env var

## Files/Packages Affected
- `packages/vscode-maproom/src/config/mcp-writer.ts` (modify `buildEnvironment()`)

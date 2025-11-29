# Ticket: MCPSIMP-2004: Update DockerManager Service Startup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update `DockerManager.ensureServicesRunning()` to only start the PostgreSQL service, removing the conditional Ollama startup logic and the provider parameter.

## Background
Currently, `ensureServicesRunning()` conditionally starts different services based on the embedding provider:
- `provider === 'ollama'` → starts postgres, ollama, maproom-mcp
- Otherwise → starts postgres, maproom-mcp

In the simplified architecture, only PostgreSQL needs to be started by the extension. The MCP server runs on the host via npx, and users manage their own Ollama installation. This implements Phase 2.4 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] `ensureServicesRunning()` only starts `['postgres']` service
- [ ] `provider` parameter removed from method signature
- [ ] All callers updated to not pass provider parameter
- [ ] Health checking simplified to PostgreSQL only
- [ ] Extension successfully starts only PostgreSQL when Docker commands invoked
- [ ] Any related tests updated to match new signature

## Technical Requirements
**Update `ensureServicesRunning()` method:**

```typescript
// BEFORE (current implementation):
const services = provider === 'ollama'
  ? ['postgres', 'ollama', 'maproom-mcp']
  : ['postgres', 'maproom-mcp']

// AFTER (simplified):
const services = ['postgres']  // Only PostgreSQL, always
```

**Method signature change:**
- Remove `provider` parameter (no longer needed)
- Update method documentation to reflect simplified purpose

**Update callers:**
- Search for all calls to `ensureServicesRunning()`
- Remove the provider argument from each call

**Health checking:**
- If there's separate health check logic for ollama/maproom-mcp, remove it
- Keep only PostgreSQL health checking

## Implementation Notes
- Search for `ensureServicesRunning` to find all usages
- The method is likely in `packages/vscode-maproom/src/docker/manager.ts`
- After removing the provider parameter, TypeScript will show compile errors for callers
- Fix all compile errors before testing
- Test the extension's Docker functionality:
  1. Use the extension to start services
  2. Verify only PostgreSQL container is started
  3. Verify no ollama or maproom-mcp containers are created

## Dependencies
- **MCPSIMP-2003** (Update Extension docker-compose.yml) - docker-compose should be updated first so the services we're removing don't exist in the compose file

## Risk Assessment
- **Risk**: Missing a caller that still passes provider
  - **Mitigation**: TypeScript compile errors will catch this; run `pnpm build` to verify
- **Risk**: Health check logic still references removed services
  - **Mitigation**: Review the entire DockerManager class for ollama/maproom-mcp references
- **Risk**: Test files reference old method signature
  - **Mitigation**: Update any tests that call ensureServicesRunning()

## Files/Packages Affected
- `packages/vscode-maproom/src/docker/manager.ts` (modify)
- Potentially other files that call `ensureServicesRunning()` (modify)
- Test files if they exist for DockerManager (modify)

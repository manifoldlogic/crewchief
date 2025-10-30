# Ticket: MCP-010: Fix missing maproom-mcp service in health check after MCP-008

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - MCP server startup test passed, all services healthy
- [x] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the MCP connection issue introduced in MCP-008 where the `getRequiredServices()` function only returns infrastructure services (postgres, ollama) but omits the maproom-mcp service itself. This causes the health check to complete before the MCP server is ready, leading to connection failures.

## Background
In ticket MCP-008, conditional service startup logic was implemented to skip Ollama when using Google or OpenAI embedding providers. However, the `getRequiredServices()` function in `packages/maproom-mcp/bin/cli.cjs` (lines 199-226) only returns infrastructure services and is missing the `maproom-mcp` service itself.

This creates a race condition where:
1. `docker compose up` starts all services including maproom-mcp
2. `waitForServicesHealthy()` only checks postgres and (conditionally) ollama
3. The health check completes before maproom-mcp is ready
4. MCP client attempts to connect and fails

The maproom-mcp service must be included in the health check array so that startup waits for the MCP server to be fully ready before returning control to the caller.

## Acceptance Criteria
- [x] `getRequiredServices()` returns an array that always includes `'maproom-mcp'` in addition to conditional infrastructure services
- [x] `waitForServicesHealthy()` waits for the maproom-mcp service to report healthy status
- [x] MCP server starts successfully and accepts connections without race conditions
- [x] Existing conditional startup logic for postgres/ollama remains intact and functional
- [x] Manual testing confirms MCP server connection succeeds after startup completes

## Technical Requirements
- Modify `getRequiredServices()` function in `packages/maproom-mcp/bin/cli.cjs` (lines 199-226)
- Add `'maproom-mcp'` to the services object with a value of `true` (always required)
- Ensure the function returns all three services in appropriate conditions:
  - postgres: always included
  - ollama: conditionally included based on EMBEDDING_PROVIDER
  - maproom-mcp: always included
- Preserve all existing conditional logic for EMBEDDING_PROVIDER detection
- Preserve all existing console logging for provider selection

## Implementation Notes

**Current Implementation** (lines 199-226):
```javascript
function getRequiredServices() {
  const provider = process.env.EMBEDDING_PROVIDER?.toLowerCase();

  const services = {
    postgres: true,  // Always required for database
    ollama: false    // Only if using Ollama provider
  };

  // Conditional Ollama logic...
  // Returns only: ['postgres'] or ['postgres', 'ollama']
}
```

**Required Change**:
Add `'maproom-mcp': true` to the services object so it's always included in the returned array.

**Expected Output**:
- With Ollama: `['postgres', 'ollama', 'maproom-mcp']`
- Without Ollama: `['postgres', 'maproom-mcp']`

**Testing Approach**:
1. Test with `EMBEDDING_PROVIDER=google` - should wait for postgres and maproom-mcp
2. Test with `EMBEDDING_PROVIDER=ollama` - should wait for all three services
3. Test with no EMBEDDING_PROVIDER - should wait for all three services (zero-config default)
4. Verify MCP connection succeeds in all cases

## Dependencies
- MCP-008 (completed) - Introduced the conditional service startup logic
- No blocking dependencies - this is a critical bug fix

## Risk Assessment
- **Risk**: The fix may reveal other timing issues if maproom-mcp service takes longer to start
  - **Mitigation**: This is actually desired behavior - we want to wait for the service to be ready

- **Risk**: Health check timeout if maproom-mcp service fails to start
  - **Mitigation**: Existing timeout and error handling in `waitForServicesHealthy()` will catch this; improves failure visibility

- **Risk**: Breaking change if downstream code depends on current behavior
  - **Mitigation**: Low risk - the current behavior is buggy and causes connection failures; fix improves reliability

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/bin/cli.cjs` (lines 199-226)
  - Function: `getRequiredServices()`
  - Change: Add `'maproom-mcp': true` to services object

## Implementation Notes (by mcp-tools-engineer)

**Changes Made:**
- Added `'maproom-mcp': true` to the services object in `getRequiredServices()` function (line 205)
- Preserved all existing conditional logic for EMBEDDING_PROVIDER detection
- Preserved all existing console logging
- Updated inline comments for consistency

**Result:**
The function now correctly returns:
- With Ollama: `['postgres', 'ollama', 'maproom-mcp']`
- Without Ollama (Google/OpenAI): `['postgres', 'maproom-mcp']`

This ensures the health check waits for the maproom-mcp service to be fully ready before allowing connections, eliminating the race condition that was causing connection failures.

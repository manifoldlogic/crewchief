# Ticket: LOCAL-5003: MCP Auto-Reconnect on Container Restart

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement automatic reconnection logic or document the manual reconnection requirement when the MCP server container is restarted.

## Background
During Maproom MCP testing, it was discovered that when the MCP server container is restarted (via `docker restart` or `docker-compose restart`), the MCP client (Claude Code) loses connection and must manually reconnect. This creates a poor user experience, especially during troubleshooting workflows where multiple restarts may be necessary.

**Current Behavior**:
1. User restarts MCP server container: `docker restart maproom-mcp-server`
2. MCP client shows "Connection lost" or similar error
3. User must manually reconnect (may require restarting Claude Code or running reconnection command)
4. Only after manual reconnection do MCP tools work again

**Impact**:
- Users must manually reconnect after any restart
- Testing/debugging workflow is disrupted
- Not truly "zero-configuration" experience
- Friction for users troubleshooting installation issues

## Acceptance Criteria
- [ ] One of the following is achieved:
  - [ ] **Option A (Preferred)**: MCP client auto-reconnects when container comes back online
  - [ ] **Option B (Fallback)**: Clear documentation of manual reconnection requirement with step-by-step instructions
- [ ] If Option A: Reconnection happens within 5 seconds of container startup
- [ ] If Option A: Reconnection is logged clearly in MCP server logs
- [ ] If Option B: Documentation includes troubleshooting steps for connection issues
- [ ] Health check improvements implemented (if applicable for Option A)

## Technical Requirements
- Investigate MCP protocol's built-in reconnection capabilities
- Research Claude Code's MCP client reconnection behavior (if accessible)
- Determine if reconnection logic should live in server or client
- Implement health checks or keep-alive pings to detect disconnections faster
- Consider Docker container restart policies and their impact on connections

## Implementation Notes
This ticket has two possible approaches depending on what's technically feasible:

### Option A: Auto-Reconnection (Preferred)
**Server-Side Approach**:
- Add health check endpoint that MCP client can poll
- Implement WebSocket keep-alive/heartbeat mechanism
- Log reconnection events clearly for debugging

**Client-Side Approach** (if Claude Code configuration is accessible):
- Configure MCP client with retry/reconnection policy
- Set reasonable backoff intervals (1s, 2s, 5s, 10s)
- Document the reconnection configuration

**Docker Improvements**:
- Add Docker health check to `docker-compose.yml`
- Ensure container signals readiness before accepting connections
- Consider graceful shutdown handling

### Option B: Documentation (Fallback)
If auto-reconnection is not feasible (e.g., MCP protocol limitation, Claude Code client restriction):
- Document the manual reconnection requirement clearly in README
- Provide step-by-step instructions for reconnecting
- Add troubleshooting section for common connection issues
- Include screenshots or CLI examples

**Investigation Areas**:
- `packages/maproom-mcp/src/index.ts` - MCP server startup logic
- Claude Code MCP client configuration (if accessible)
- MCP protocol specification for reconnection behavior
- Docker health check configuration in docker-compose.yml

## Dependencies
- None - this is a UX improvement independent of other tickets

## Risk Assessment
- **Risk**: Auto-reconnection may cause connection storms if container is flapping
  - **Mitigation**: Implement exponential backoff, max retry limits
- **Risk**: Health checks may increase resource usage
  - **Mitigation**: Use lightweight health check endpoints, reasonable check intervals
- **Risk**: Documentation-only solution may not satisfy users expecting auto-reconnect
  - **Mitigation**: Clearly set expectations in README about manual reconnection

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` - MCP server startup and connection handling
- `packages/maproom-mcp/config/docker-compose.yml` - Add health check configuration
- `packages/maproom-mcp/README.md` - Document reconnection behavior (required for both options)
- Claude Code MCP client configuration (if accessible and modifiable)

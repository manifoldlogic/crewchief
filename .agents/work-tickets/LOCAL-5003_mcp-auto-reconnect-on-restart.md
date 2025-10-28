# Ticket: LOCAL-5003: MCP Auto-Reconnect on Container Restart

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

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
- [x] One of the following is achieved:
  - [ ] **Option A (Preferred)**: MCP client auto-reconnects when container comes back online
  - [x] **Option B (Fallback)**: Clear documentation of manual reconnection requirement with step-by-step instructions
- [ ] If Option A: Reconnection happens within 5 seconds of container startup
- [ ] If Option A: Reconnection is logged clearly in MCP server logs
- [x] If Option B: Documentation includes troubleshooting steps for connection issues
- [x] Health check improvements implemented (if applicable for Option A)

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
- ~~`packages/maproom-mcp/src/index.ts`~~ - No changes needed (stdio transport limitation)
- `packages/maproom-mcp/config/docker-compose.yml` - Already has optimal health checks and restart policies
- `packages/maproom-mcp/README.md` - ✅ **UPDATED** with comprehensive reconnection documentation
- ~~Claude Code MCP client configuration~~ - Not modifiable (client-side implementation)

## Implementation Notes

**Decision: Option B (Documentation) - IMPLEMENTED**

After thorough investigation, auto-reconnection is **not feasible** for the following technical reasons:

### Why Auto-Reconnection Cannot Be Implemented

1. **MCP Protocol Limitation**:
   - The MCP stdio transport creates a persistent process pipe (stdin/stdout) between client and server
   - Unlike HTTP/SSE transports, stdio connections are tied to the server process lifecycle
   - When a container restarts, the original process exits and a new process is created
   - The stdio connection breaks at the OS level and cannot be re-established without client intervention

2. **Client-Side Architecture**:
   - The MCP client (Claude Desktop/Cursor) spawns the server process via `npx @crewchief/maproom-mcp`
   - The CLI wrapper (`bin/cli.cjs`) establishes a `docker exec -i` stdio proxy to the containerized server
   - This creates a chain: **Client → CLI wrapper → Docker exec → Container process**
   - Container restart breaks the docker exec connection, which cannot be automatically repaired

3. **Protocol Design**:
   - Per MCP specification, stdio transport is designed for local, single-lifecycle processes
   - Reconnection mechanisms exist only for network-based transports (SSE/HTTP)
   - The client controls the server lifecycle and must initiate new connections

### What Was Verified

✅ **Docker restart policies**: Already configured optimally with `restart: unless-stopped`
✅ **Health checks**: Properly configured for both postgres (10s interval) and ollama (30s interval)
✅ **Container resilience**: Services automatically restart if they crash

### Documentation Improvements Implemented

Added to `packages/maproom-mcp/README.md`:

1. **New "Container Management" section**:
   - Service status checking
   - Log viewing (without breaking connection)
   - Stopping/restarting services with clear warnings
   - Health check explanation and verification commands

2. **Enhanced "Troubleshooting" section**:
   - New "Connection lost after container restart" entry at the top
   - Clear explanation of WHY this happens (stdio transport limitation)
   - Step-by-step reconnection instructions for Claude Desktop and Cursor
   - Alternative approaches (log viewing without restart)
   - Technical background explaining the underlying cause

### User Experience Impact

**Before**: Users encountered broken connections after restart with no guidance
**After**: Users have:
- Clear understanding of why reconnection is manual
- Step-by-step instructions for both Claude Desktop and Cursor
- Alternative troubleshooting approaches that avoid disconnection
- Comprehensive container management guidance

This solution aligns with MCP protocol design and provides the best possible UX within technical constraints.

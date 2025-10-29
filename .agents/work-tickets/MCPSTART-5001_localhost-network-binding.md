# Ticket: MCPSTART-5001: Update docker-compose.yml to bind services to localhost

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Change PostgreSQL and Ollama port bindings from 0.0.0.0 (all interfaces) to 127.0.0.1 (localhost only) to prevent network exposure in the Maproom MCP docker-compose configuration.

## Background
From MCPSTART_SECURITY_REVIEW.md Section 5 (Network Exposure) - services are currently exposed on all network interfaces (0.0.0.0), making them accessible to any device on the network. This creates unnecessary security risk. Binding to localhost (127.0.0.1) prevents external access while maintaining full local development functionality.

This implements Phase 5 (Security Hardening) of the MCPSTART project plan.

## Acceptance Criteria
- [ ] PostgreSQL port binding changed to 127.0.0.1:5433:5432 in docker-compose.yml
- [ ] Ollama port binding changed to 127.0.0.1:11434:11434 in docker-compose.yml
- [ ] MCP server remains stdio-only (no port binding changes)
- [ ] Updated both docker-compose.yml files in `packages/maproom-mcp/config/`
- [ ] Services remain accessible from host machine via localhost
- [ ] Verified services cannot be accessed from external network addresses

## Technical Requirements

Update the ports section in docker-compose.yml:

```yaml
services:
  postgres:
    ports:
      - "127.0.0.1:5433:5432"  # Changed from "0.0.0.0:5433:5432"

  ollama:
    ports:
      - "127.0.0.1:11434:11434"  # Changed from "0.0.0.0:11434:11434"

  maproom-mcp:
    # No ports section - stdio transport only
```

## Implementation Notes
- The `127.0.0.1:HOST_PORT:CONTAINER_PORT` format binds the host port only to the loopback interface
- This prevents access from other machines on the network
- Local development tools (database clients, HTTP clients) can still connect via localhost or 127.0.0.1
- The MCP server uses stdio transport and should not have any port bindings
- If users need network access (e.g., for remote development), they can manually change to 0.0.0.0 with full awareness of the security implications

## Dependencies
None - this is an independent security hardening change

## Risk Assessment
- **Risk**: Low - only affects network binding, not functionality
  - **Mitigation**: Services remain fully functional on localhost; documentation will explain how to expose if needed (MCPSTART-5003)
- **Risk**: Users with remote development setups may need to adjust configuration
  - **Mitigation**: Document the change and provide clear instructions for exposing services when necessary

## Files/Packages Affected
- `packages/maproom-mcp/config/docker-compose.yml`

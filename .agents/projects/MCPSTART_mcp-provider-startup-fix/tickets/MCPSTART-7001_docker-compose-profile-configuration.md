# Ticket: MCPSTART-7001: Docker Compose Profile Configuration

## Status
- [ ] **Task completed** - DEFERRED (Phase 3 solution implemented instead)
- [ ] **Tests pass** - N/A (ticket deferred)
- [ ] **Verified** - N/A (ticket deferred)

**Note**: Phase 7 (Docker Compose Profiles) was deferred in favor of the Phase 3 solution (explicit stop/remove of unnecessary services via CLI logic). The Phase 3 implementation successfully solves the problem and has been validated in production through v1.3.1. Docker profiles remain a viable alternative for future enhancement.

## Agents
- docker-engineer (primary)
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Update `config/docker-compose.yml` to use Docker Compose profiles for optional services like Ollama, enabling native service selection without manual service name management.

## Background
This ticket implements Phase 4 (Service Profiles) from MCPSTART_ARCHITECTURE.md, providing a more robust long-term alternative to manual service selection. Docker Compose profiles allow declarative control over which services start based on provider configuration, eliminating the need for dynamic service name construction and improving maintainability.

Reference: MCPSTART_ARCHITECTURE.md Phase 4 (lines 243-305)

## Acceptance Criteria
- [ ] `config/docker-compose.yml` includes profile definitions for optional services (Ollama)
- [ ] Core services (postgres, maproom-mcp) remain in default profile (no profile key)
- [ ] Ollama service uses `profiles: ["ollama"]` to make it opt-in
- [ ] Service dependencies are updated to remove optional service dependencies from core services
- [ ] Configuration validates correctly with `docker compose config`
- [ ] Existing non-profile startup still works (backward compatibility maintained)

## Technical Requirements
- Modify `config/docker-compose.yml` to add profile annotations
- Ollama service must have `profiles: ["ollama"]` key
- Remove ollama from `depends_on` in maproom-mcp service (if present)
- Preserve all existing service configurations (ports, volumes, health checks)
- Maintain existing environment variable structure
- Ensure postgres and maproom-mcp remain in default profile

## Implementation Notes

**Profile Configuration Pattern** (from MCPSTART_ARCHITECTURE.md lines 251-266):

```yaml
services:
  postgres:
    # ... always required, no profile key ...

  ollama:
    profiles: ["ollama"]  # Only start if --profile ollama
    # ... rest of ollama config ...

  maproom-mcp:
    # ... always required, no profile key ...
    depends_on:
      postgres:
        condition: service_healthy
      # Remove ollama dependency entirely
```

**Key Concepts**:
- Services without `profiles` key are in the default profile (always start)
- Services with `profiles: ["name"]` only start when that profile is activated
- Use `--profile ollama` flag to activate optional services
- This provides native Docker Compose service selection without CLI logic

**Testing Approach**:
- Validate configuration: `docker compose -f config/docker-compose.yml config`
- Test default startup: `docker compose up -d` (should start postgres + maproom-mcp only)
- Test profile startup: `docker compose --profile ollama up -d` (should start all services)

## Dependencies
- No blocking dependencies
- This is an alternative to Phase 3 tickets (MCPSTART-3001, 3002, 3003)
- Can be implemented independently as a future improvement

## Risk Assessment
- **Risk**: Breaking existing startup behavior for users without profile-aware CLI
  - **Mitigation**: Maintain backward compatibility; profile-based CLI changes are in MCPSTART-7002
- **Risk**: Services with profile key won't start without explicit --profile flag
  - **Mitigation**: Clear documentation and integration tests validating both startup modes

## Files/Packages Affected
- `/workspace/config/docker-compose.yml`

# Ticket: MCPSTART-7003: Profile Documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer (primary)
- verify-ticket
- commit-ticket

## Summary
Create comprehensive documentation explaining Docker Compose profile usage, including how to manually start services with profiles, troubleshooting profile-related issues, and migration guidance from service-based to profile-based startup.

## Background
This ticket documents Phase 4 (Service Profiles) from MCPSTART_ARCHITECTURE.md. After implementing profile-based configuration (MCPSTART-7001) and startup logic (MCPSTART-7002), users and developers need clear documentation on how profiles work, when to use them, and how to troubleshoot issues.

Reference: MCPSTART_ARCHITECTURE.md Phase 4 (lines 243-305)

## Acceptance Criteria
- [ ] README.md includes section on Docker Compose profiles
- [ ] Documentation explains what profiles are and why they're used
- [ ] Examples show manual startup with and without profiles
- [ ] Troubleshooting section covers profile-related issues
- [ ] Migration guide explains transition from service-based to profile-based approach
- [ ] Documentation covers minimum Docker Compose version requirements
- [ ] Code comments in docker-compose.yml explain profile annotations

## Technical Requirements
- Add "Docker Compose Profiles" section to main README.md
- Include code examples for manual docker compose commands
- Document environment variables that affect profile selection
- Explain relationship between `EMBEDDING_PROVIDER` and profiles
- Add inline comments to `config/docker-compose.yml` explaining profile usage
- Link to official Docker Compose profile documentation
- Include troubleshooting steps for common profile issues

## Implementation Notes

**Documentation Structure**:

1. **What are Profiles?**
   - Brief explanation of Docker Compose profiles
   - Why CrewChief uses them (optional service management)

2. **Automatic Profile Selection**
   - How `EMBEDDING_PROVIDER` env var controls profiles
   - Default behavior (Ollama profile active)
   - External provider behavior (Ollama profile inactive)

3. **Manual Usage**
   ```bash
   # Start with Ollama (all services)
   docker compose --profile ollama up -d

   # Start without Ollama (core services only)
   docker compose up -d

   # View which services will start
   docker compose --profile ollama config --services
   ```

4. **Troubleshooting Profile Issues**
   - Service not starting: Check if profile is needed
   - Unexpected services: Check active profiles
   - Version compatibility: Minimum Docker Compose version

5. **Migration from Service-Based Approach**
   - Old approach: Explicit service names in CLI
   - New approach: Profile-based selection
   - Backward compatibility considerations

**Key Points to Document** (from MCPSTART_ARCHITECTURE.md lines 301-305):
- Docker Compose handles service selection natively
- No manual service name management needed
- Clearer intent in configuration
- More maintainable long-term

## Dependencies
- **Blocks**: Requires MCPSTART-7001 and MCPSTART-7002 to be completed first
- **Related**: Complements MCPSTART-6001 (Troubleshooting Guide)

## Risk Assessment
- **Risk**: Documentation becomes outdated as implementation evolves
  - **Mitigation**: Version-tag documentation; include "last updated" dates
- **Risk**: Users confused by multiple startup approaches (service-based vs profile-based)
  - **Mitigation**: Clear comparison table; explain why profiles are preferred

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/README.md` (or main project README)
- `/workspace/config/docker-compose.yml` (inline comments)
- Possibly `/workspace/docs/DOCKER_PROFILES.md` (if separate doc is preferred)

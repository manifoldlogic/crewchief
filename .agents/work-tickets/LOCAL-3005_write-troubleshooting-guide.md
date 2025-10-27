# Ticket: LOCAL-3005: Write troubleshooting guide

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- technical-researcher
- verify-ticket
- commit-ticket

## Summary
Create a comprehensive troubleshooting guide (TROUBLESHOOTING.md) that addresses the top 10 most common issues users will encounter when running maproom-mcp locally with Docker Compose, providing clear diagnostic steps, causes, and solutions.

## Background
With the LOCAL package nearing completion (Phase 3 - Configuration & User Experience), users will need robust self-service support documentation to diagnose and resolve common issues independently. This reduces support burden and improves user satisfaction. After integration testing (LOCAL-2006), we've identified the most likely failure scenarios that users will encounter: Docker Compose v2 issues, port conflicts, slow model downloads, permission problems, disk space, and service health checks.

## Acceptance Criteria
- [ ] TROUBLESHOOTING.md file created in packages/maproom-mcp/
- [ ] Covers top 10 most likely issues with clear organization
- [ ] Each issue includes: cause explanation, solution steps, and verification commands
- [ ] All example commands are copy-pasteable and tested
- [ ] Cross-references README.md and health-check.sh where appropriate
- [ ] Includes platform-specific notes (Linux, macOS, Windows/WSL2) where applicable
- [ ] Links to relevant Docker and Ollama documentation
- [ ] Uses clear formatting with visual cues (headings, emojis, code blocks) for easy scanning

## Technical Requirements

### Document Structure
1. **Quick Diagnostics Section**:
   - How to check service status: `docker compose ps`
   - How to view logs: `docker compose logs -f maproom`
   - How to run health check script: `./health-check.sh`

2. **Common Issues Section** (must cover these):
   - **Issue A**: "Docker Compose v2 not found"
     - Cause: Old docker-compose binary instead of plugin
     - Solution: Install Docker Desktop or Compose v2 plugin
     - Verification: `docker compose version`

   - **Issue B**: "Port already in use" (3000 or 11434)
     - Cause: Another service using the port
     - Solution: Change ports via environment variables
     - Example: `MAPROOM_PORT=3001 npx @crewchief/maproom-mcp`

   - **Issue C**: "Model download is slow/stuck"
     - Cause: Large model download (~200MB nomic-embed-text)
     - Solution: Wait, check progress with `docker compose logs -f ollama`
     - Workaround: Pre-pull model manually

   - **Issue D**: "Permission denied on volumes"
     - Cause: Docker volume ownership issues
     - Solution: Platform-specific fixes (Linux uid:gid, macOS auto-fix, Windows/WSL2 considerations)

   - **Issue E**: "Out of disk space"
     - Cause: Docker images + models + data = ~5GB
     - Solution: Clean up with `docker system prune`
     - Prevention: Ensure 10GB free space before starting

   - **Issue F**: "Services keep restarting"
     - Cause: Health checks failing, OOM, or crashes
     - Solution: Check logs, increase Docker memory limit

3. **Advanced Troubleshooting Section**:
   - How to connect to containers: `docker compose exec maproom /bin/sh`
   - How to inspect volumes: `docker volume inspect maproom-mcp_postgres-data`
   - How to reset everything: `docker compose down -v`
   - How to enable debug logging: `RUST_LOG=debug docker compose up`

4. **Performance Issues Section**:
   - Slow indexing: Check CPU usage, Ollama batching configuration
   - High memory usage: Resource limits in docker-compose.yml
   - Slow search: Database indexes, query optimization tips

5. **Getting Help Section**:
   - GitHub Issues link
   - Required information for bug reports (OS, Docker version, logs)
   - How to share logs safely (sanitize file paths and sensitive data)

### Documentation Standards
- Use Markdown formatting with clear headings (##, ###)
- Code blocks with syntax highlighting for commands
- Use emojis for visual scanning: ⚠️ (warnings), ✅ (success), 🔍 (diagnostic), 🛠️ (fix)
- Include "Expected Output" examples where helpful
- Cross-reference other documentation files
- Keep language clear and actionable (imperative mood)

### External Reference Links
- Docker troubleshooting: https://docs.docker.com/config/daemon/#troubleshoot-the-daemon
- Ollama GitHub issues: https://github.com/ollama/ollama/issues
- Docker Compose v2 installation: https://docs.docker.com/compose/install/

## Implementation Notes

### Research Approach
1. Review LOCAL-2006 test results to identify actual failure scenarios
2. Test each troubleshooting scenario in a clean Docker environment
3. Verify all diagnostic commands work as documented
4. Ensure solutions are minimal and non-destructive where possible
5. Document fallback options when primary solution doesn't work

### Content Organization
- Start with most common issues (based on typical Docker Compose adoption)
- Group related issues together (e.g., all Docker-related, all Ollama-related)
- Use consistent formatting pattern: **Issue** → **Cause** → **Solution** → **Verification**
- Provide both quick fixes and deeper explanations

### Platform Considerations
- Linux: Focus on volume permissions (uid:gid mapping)
- macOS: Note Docker Desktop VM behavior, resource limits in UI
- Windows/WSL2: WSL2 file system performance, Docker Desktop integration

### Validation
- Test all commands in a fresh environment
- Verify links are not broken
- Ensure consistency with README.md terminology
- Check that TROUBLESHOOTING.md appears in README's documentation section

## Dependencies
- **LOCAL-2006**: Test batch embedding with nomic-embed-text (must be completed to identify real issues)
- **All Phase 1 & 2 tickets**: Integration must be complete to understand full failure modes
- health-check.sh script (from earlier ticket)
- README.md (for cross-references)

## Risk Assessment
- **Risk**: Documentation becomes outdated as implementation changes
  - **Mitigation**: Link to this ticket in code comments near critical failure points; include "Last Updated" date in doc
- **Risk**: Missing platform-specific edge cases
  - **Mitigation**: Note explicitly which platforms were tested; invite community contributions for platform-specific issues
- **Risk**: Overly technical language alienates beginner users
  - **Mitigation**: Use plain language; define technical terms on first use; provide "TL;DR" quick fixes

## Files/Packages Affected
- packages/maproom-mcp/TROUBLESHOOTING.md (new file)
- packages/maproom-mcp/README.md (add link to troubleshooting guide in "Documentation" section)

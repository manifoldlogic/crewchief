# Ticket: MCPSTART-6003: Update CHANGELOG for v1.1.9 release

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Document all changes from Phases 1-5 in CHANGELOG with clear upgrade notes, explaining the critical Ollama startup bug fix and all related improvements.

## Background
Version 1.1.9 represents a significant fix addressing MCP-008 and MCP-011 issues. Users need a clear changelog to understand what changed, why the upgrade is important, and what to expect during the upgrade process. This is the final documentation step before publishing to npm.

## Acceptance Criteria
- [ ] Add v1.1.9 section to CHANGELOG.md (or packages/maproom-mcp/CHANGELOG.md)
- [ ] List all fixes from MCPSTART tickets organized by category (Fixed, Added, Security, Changed)
- [ ] Explain the Ollama startup bug and the fix in clear, user-friendly language
- [ ] Include upgrade notes explaining auto-update behavior for config files
- [ ] Link to related issues (MCP-008, MCP-011) if available on GitHub
- [ ] Mention breaking changes (none expected) explicitly
- [ ] Include release date placeholder (2025-01-XX to be updated on publish)

## Technical Requirements

Add the following changelog entry to the appropriate CHANGELOG.md file:

```markdown
## [1.1.9] - 2025-01-XX

### Fixed
- **CRITICAL**: Ollama no longer starts when using Google or OpenAI providers (#MCPSTART)
  - Fixed environment variable propagation from MCP client to Docker Compose
  - Added explicit env passing in all spawn() calls (MCPSTART-2001)
  - Implemented pre-flight container cleanup (MCPSTART-3001)
  - Added explicit stop/remove for unnecessary services (MCPSTART-3002)
  - Verify final container state after startup (MCPSTART-3003)

### Added
- Comprehensive diagnostic logging with MAPROOM_MCP_DEBUG=true (MCPSTART-1001, 1002, 1003)
  - Docker command execution logging
  - Container state verification logging
  - Credential redaction in logs (MCPSTART-1004)
  - docker-compose.yml config validation (MCPSTART-2002)
  - Provider env var validation with fail-fast behavior (MCPSTART-2003)
- Integration test suite for startup scenarios (MCPSTART-4001, 4002)
  - Test all three provider configurations (Ollama, Google, OpenAI)
  - Verify correct containers start/stop
  - Verify environment variable propagation

### Security
- Services now bound to localhost only (127.0.0.1) instead of 0.0.0.0 (MCPSTART-5001)
- Added npm audit check to prepublishOnly script (MCPSTART-5002)
- Credential redaction in diagnostic logs (MCPSTART-1004)
- Security documentation added to README (MCPSTART-5003)

### Changed
- Environment variables now explicitly passed to Docker Compose in all spawn() calls
- Config files at ~/.maproom-mcp/ auto-update if outdated (preserves user customizations)
- Improved error messages for missing provider credentials

### Upgrade Notes
- **Config files**: Files at ~/.maproom-mcp/ will auto-update on first run if the template has changed. Your customizations in docker-compose.env will be preserved.
- **No breaking changes**: Existing users can upgrade seamlessly.
- **Troubleshooting**: New MAPROOM_MCP_DEBUG=true mode available for diagnosing startup issues.
- **Security**: Services now bind to localhost only. If you were accessing from another machine, you'll need to adjust network configuration.

### Migration Guide
No migration steps required. Simply upgrade to the latest version:
```bash
npx @crewchief/maproom-mcp@latest
```

If you experience issues, see the Troubleshooting section in the README.
```

## Implementation Notes

- Use clear, user-friendly language in the changelog
- Mark the Ollama fix as **CRITICAL** since it affects core functionality
- Link each change to its ticket number for traceability
- Organize changes by category (Fixed, Added, Security, Changed) following Keep a Changelog format
- Be explicit about "no breaking changes" to give users confidence in upgrading
- Include both high-level summary and ticket-level details
- Add the upgrade notes section to help users understand what to expect
- Update the release date when actually publishing (leave as placeholder for now)

## Dependencies
- **All Phase 1-5 tickets must be complete** before this can be finalized
- Specifically documents changes from:
  - Phase 1 (Diagnostic Logging): MCPSTART-1001, 1002, 1003, 1004
  - Phase 2 (Environment Passing): MCPSTART-2001, 2002, 2003
  - Phase 3 (Container Management): MCPSTART-3001, 3002, 3003
  - Phase 4 (Testing): MCPSTART-4001, 4002
  - Phase 5 (Security): MCPSTART-5001, 5002, 5003

## Risk Assessment
- **Risk**: Low - documentation only, no code changes
  - **Mitigation**: Review changelog against actual implemented changes before publishing

## Files/Packages Affected
- `CHANGELOG.md` (root level) OR `packages/maproom-mcp/CHANGELOG.md` (package level)
  - Check which exists and update accordingly

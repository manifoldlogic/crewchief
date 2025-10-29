# Ticket: MCPSTART-2002: Implement docker-compose.yml verification on startup

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Verify docker-compose.yml uses environment variable syntax (not hardcoded values) before starting services.

## Background
MCP-008 and MCP-011 updated docker-compose.yml to use ${EMBEDDING_PROVIDER:-ollama} syntax, but users may have old hardcoded configs. This ticket adds verification that fails fast with clear error if the config file has hardcoded EMBEDDING_PROVIDER values that would override environment variables.

Implements **Phase 2.2** from MCPSTART_ARCHITECTURE.md - Docker Compose File Verification.

## Acceptance Criteria
- [ ] Function verifyDockerComposeConfig() checks for hardcoded EMBEDDING_PROVIDER
- [ ] Detects regex pattern: `EMBEDDING_PROVIDER:\s*['"]?ollama['"]?\s*$`
- [ ] Checks for env var syntax: `\$\{EMBEDDING_PROVIDER[:\-]`
- [ ] If hardcoded found WITHOUT env var syntax, exits with clear error
- [ ] Error message shows config file location
- [ ] Called after config file auto-update in setup phase

## Technical Requirements
- Read docker-compose.yml from CONFIG_DIR
- Check for patterns:
  - BAD: `EMBEDDING_PROVIDER: ollama` (hardcoded)
  - GOOD: `EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}` (env var)
- If hardcoded pattern found AND no env var syntax: fail with error
- Exit code 1 with clear message to user

## Implementation Notes
See MCPSTART_ARCHITECTURE.md lines 134-158 for detailed implementation guidance.

The verification function should:
1. Read docker-compose.yml file from CONFIG_DIR
2. Search for hardcoded EMBEDDING_PROVIDER patterns
3. Verify presence of environment variable syntax
4. Provide actionable error message if validation fails

Error message format:
```
❌ ERROR: docker-compose.yml contains hardcoded EMBEDDING_PROVIDER
   File: /path/to/docker-compose.yml

   Your config file has:
     EMBEDDING_PROVIDER: ollama

   It should be:
     EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}

   This was fixed in MCP-011. Please update your config file or run:
     npx @crewchief/maproom-mcp setup
```

## Dependencies
- MCPSTART-2001 (env propagation must exist first)

## Risk Assessment
- **Risk**: Low - fail-fast verification prevents silent failures
  - **Mitigation**: Clear error messages guide user to fix configuration

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs`

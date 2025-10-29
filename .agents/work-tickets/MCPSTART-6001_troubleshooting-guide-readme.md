# Ticket: MCPSTART-6001: Update README with troubleshooting guide

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Add comprehensive troubleshooting section to README with diagnostic steps for common issues, specifically addressing the "Ollama starts when using Google/OpenAI" problem and teaching users how to self-diagnose provider startup issues.

## Background
From MCPSTART_QUALITY_STRATEGY.md lines 419-441 - users need self-service debugging guidance. The troubleshooting section enables users to diagnose provider startup issues themselves using diagnostic mode. This is critical for Phase 6 documentation as it empowers users to identify and resolve configuration issues without requiring developer intervention.

## Acceptance Criteria
- [ ] Add "Troubleshooting" section to README after the main usage instructions
- [ ] Include "Ollama starts when using Google/OpenAI" subsection
- [ ] Document MAPROOM_MCP_DEBUG=true diagnostic mode with clear examples
- [ ] Explain what to check in diagnostic output (container states, env vars, config files)
- [ ] Provide links to common solutions (config validation, network binding, etc.)
- [ ] Include example diagnostic output showing both working and failing scenarios

## Technical Requirements

Add troubleshooting section with template from MCPSTART_QUALITY_STRATEGY.md lines 422-439:

```markdown
## Troubleshooting

### Ollama starts when using Google/OpenAI

If you see Ollama containers starting even though you set `EMBEDDING_PROVIDER=google` or `openai`:

1. **Enable diagnostic mode**:
   ```bash
   MAPROOM_MCP_DEBUG=true EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp
   ```

2. **Check the output for**:
   - "Environment variables being passed to Docker Compose" - Verify EMBEDDING_PROVIDER is listed
   - "docker-compose.yml config is up to date" - Should be ✓
   - "Starting services: maproom-postgres, maproom-embedder, maproom-mcp" - Should NOT include ollama
   - "Container states after startup" - Ollama should show "not running"

3. **Common issues**:
   - Config files outdated → Solution: Delete ~/.maproom-mcp/docker-compose.yml and restart
   - Environment variable not set → Solution: Check EMBEDDING_PROVIDER is exported
   - Previous containers running → Solution: Run `docker compose -f ~/.maproom-mcp/docker-compose.yml down`

4. **If Ollama still starts, file an issue** with the diagnostic output.
```

Position this section after the "Usage" section and before "Configuration" in the README.

## Implementation Notes

- Use clear, user-friendly language (avoid technical jargon where possible)
- Provide copy-pasteable commands
- Include expected vs. actual output examples
- Link to GitHub issues if users need further help
- Format diagnostic output examples with proper markdown code blocks
- Ensure the troubleshooting steps match the actual diagnostic output from MCPSTART-1001

## Dependencies
- All Phase 1-5 tickets must be complete (documenting what exists)
- Specifically depends on:
  - MCPSTART-1001 (diagnostic logging) - documents the MAPROOM_MCP_DEBUG mode
  - MCPSTART-2002 (config verification) - documents the config validation output
  - MCPSTART-3003 (final state verification) - documents the container state output

## Risk Assessment
- **Risk**: Low - documentation only, no code changes
  - **Mitigation**: Verify all commands and output examples match actual behavior before committing

## Files/Packages Affected
- `packages/maproom-mcp/README.md`

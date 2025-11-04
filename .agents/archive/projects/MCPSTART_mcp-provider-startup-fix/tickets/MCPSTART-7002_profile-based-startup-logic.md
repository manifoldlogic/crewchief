# Ticket: MCPSTART-7002: Profile-Based Startup Logic

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
Modify CLI startup logic in `packages/maproom-mcp/src/index.ts` to use Docker Compose profiles (`--profile ollama`) instead of explicit service names, enabling cleaner provider-based service selection.

## Background
This ticket implements Phase 4 (Service Profiles) profile-based startup from MCPSTART_ARCHITECTURE.md. After MCPSTART-7001 adds profiles to docker-compose.yml, this ticket updates the CLI to leverage them. This replaces manual service name construction with Docker's native profile system, improving maintainability and clarity.

Reference: MCPSTART_ARCHITECTURE.md Phase 4 (lines 243-305)

## Acceptance Criteria
- [ ] `startDockerCompose()` function determines which profiles to activate based on `EMBEDDING_PROVIDER`
- [ ] Profile activation uses `--profile <name>` flags in docker compose command
- [ ] Ollama profile activated when `EMBEDDING_PROVIDER` is unset or `ollama`
- [ ] Ollama profile skipped when using external provider (openai, anthropic, voyageai)
- [ ] Diagnostic logging shows which profiles are being activated
- [ ] User-friendly console messages indicate startup mode (with/without Ollama)
- [ ] Existing non-profile code path remains functional (backward compatibility)

## Technical Requirements
- Modify `startDockerCompose()` function in `packages/maproom-mcp/src/index.ts`
- Detect `EMBEDDING_PROVIDER` environment variable
- Build docker compose args array with `--profile` flags dynamically
- Add `--profile ollama` when provider is unset or `ollama`
- Skip `--profile ollama` for external providers
- Include diagnostic logging with `diagnosticLog('Docker Compose with Profiles', { args, profiles })`
- Preserve existing `docker compose up -d` behavior when no profiles needed

## Implementation Notes

**Profile-Based Startup Pattern** (from MCPSTART_ARCHITECTURE.md lines 272-299):

```javascript
function startDockerCompose() {
  const provider = process.env.EMBEDDING_PROVIDER?.toLowerCase();

  // Determine which profiles to activate
  const profiles = [];
  if (!provider || provider === 'ollama') {
    profiles.push('ollama');
    console.error('🚀 Starting with Ollama (local embeddings)...');
  } else {
    console.error(`🚀 Starting with ${provider} embeddings...`);
    console.error('   (Ollama not needed, skipping)');
  }

  const args = ['compose'];

  // Add profile flags
  profiles.forEach(profile => {
    args.push('--profile', profile);
  });

  args.push('up', '-d');

  diagnosticLog('Docker Compose with Profiles', { args, profiles });

  // ... execute command ...
}
```

**Key Benefits** (from MCPSTART_ARCHITECTURE.md lines 301-305):
- Docker Compose handles service selection natively
- No manual service name management
- Clearer intent in configuration
- More maintainable long-term

**Testing Scenarios**:
1. No `EMBEDDING_PROVIDER` → should start with `--profile ollama`
2. `EMBEDDING_PROVIDER=ollama` → should start with `--profile ollama`
3. `EMBEDDING_PROVIDER=openai` → should start without profiles
4. Verify console messages match provider selection

## Dependencies
- **Blocks**: Requires MCPSTART-7001 (Docker Compose profile configuration) to be completed first
- **Related**: Alternative to MCPSTART-3002 (explicit service selection)

## Risk Assessment
- **Risk**: Profile flags not supported in older Docker Compose versions
  - **Mitigation**: Add version check or fallback to service-based approach; document minimum Docker Compose version
- **Risk**: Users with custom docker-compose.yml missing profiles
  - **Mitigation**: Feature detection; graceful degradation to non-profile mode

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/src/index.ts`

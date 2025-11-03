# Ticket: PROVFIX-4001: Remove Docker Compose Default Endpoint

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Remove or clear the default `EMBEDDING_API_ENDPOINT=http://ollama:11434` from Docker Compose configuration. This default contributed to the endpoint resolution bug by polluting the environment for all providers. With Rust now handling provider-specific defaults, Docker should not set generic endpoint defaults.

## Background
This ticket implements Phase 4 (Cleanup) from the PROVFIX implementation plan, specifically cleaning up Docker Compose environment defaults that contributed to the endpoint resolution bug.

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md`, the root cause chain was:
1. Docker Compose sets `EMBEDDING_API_ENDPOINT=http://ollama:11434` as default
2. All containers inherit this environment variable
3. OpenAI provider sees endpoint in environment and uses it
4. Results in connection to wrong service

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md` section "4. Clean Environment Variable Contract":

**Current (buggy):**
```yaml
environment:
  EMBEDDING_API_ENDPOINT: ${EMBEDDING_API_ENDPOINT:-http://ollama:11434}
```

**Proposed (clean):**
```yaml
environment:
  EMBEDDING_API_ENDPOINT: ${EMBEDDING_API_ENDPOINT:-}  # Empty default
```

Or better: Remove entirely and let Rust defaults handle it.

**Rationale:**
- Rust now has provider-specific default endpoints (from PROVFIX-1001)
- Docker defaults should not assume all providers use Ollama endpoint
- Empty default is safer: only set when explicitly needed
- Reduces configuration complexity and potential for bugs

## Acceptance Criteria
- [ ] `EMBEDDING_API_ENDPOINT` default removed or set to empty string in docker-compose.yml
- [ ] Docker Compose documentation explains Rust handles defaults
- [ ] OpenAI provider works correctly after Docker cleanup
- [ ] Ollama provider works correctly (uses Rust default or explicit override)
- [ ] Containers start without errors
- [ ] All providers work with clean Docker environment

## Technical Requirements

**File to Modify:** `/workspace/packages/maproom-mcp/config/docker-compose.yml`

1. Locate the `EMBEDDING_API_ENDPOINT` environment variable in the environment section

2. Change the default from `http://ollama:11434` to empty string:
   ```yaml
   EMBEDDING_API_ENDPOINT: ${EMBEDDING_API_ENDPOINT:-}
   ```
   OR remove the line entirely if not needed

3. Add explanatory comment:
   ```yaml
   # EMBEDDING_API_ENDPOINT: Provider-specific defaults handled by Rust
   # Only set this if you need a custom endpoint for your provider
   ```

4. Keep other provider variables unchanged:
   - `EMBEDDING_PROVIDER`
   - `EMBEDDING_MODEL`
   - `EMBEDDING_DIMENSION`
   - Provider API keys (OPENAI_API_KEY, etc.)

## Implementation Notes

See `.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md` Phase 4 for full context.

**Decision: Empty default vs. complete removal**
- **Empty default:** Safer, allows explicit override via environment variable
- **Complete removal:** Cleaner, Rust handles everything

**Recommendation:** Use empty default (`${EMBEDDING_API_ENDPOINT:-}`) for backward compatibility. This allows users to still override if needed, but doesn't pollute the environment with provider-specific defaults.

**Testing Approach:**
1. Stop and remove containers: `docker compose down -v`
2. Start with clean environment: `docker compose up -d`
3. Test OpenAI provider (should use Rust default: https://api.openai.com/v1/embeddings)
4. Test Ollama provider (should use Rust default: http://localhost:11434/api/embed)
5. Test custom endpoint override (should respect user-provided value)

**Expected Behavior After Change:**
- OpenAI provider: Uses `https://api.openai.com/v1/embeddings` (from Rust code)
- Ollama provider: Uses `http://localhost:11434/api/embed` (from Rust code)
- Custom endpoint: Users can still set `EMBEDDING_API_ENDPOINT=http://custom:8080` to override
- No more cross-provider pollution from Docker defaults

## Dependencies
- **Requires:** PROVFIX-3001 (CLI cleanup should work first)
- **Recommended:** PROVFIX-1001, PROVFIX-1002 (Rust fixes complete)

This is the final cleanup ticket in Phase 4. It should be implemented after the core Rust fixes and CLI cleanup are complete.

## Risk Assessment
- **Risk**: Breaking Ollama users who rely on Docker default
  - **Mitigation**: Rust provides the same default (`http://localhost:11434`); behavior unchanged for Ollama users. They'll see no difference.

- **Risk**: Unclear documentation confuses users
  - **Mitigation**: Add clear comment in docker-compose.yml explaining that Rust handles provider-specific defaults. Update any Docker-related documentation to reference the new behavior.

- **Risk**: Environment variable precedence issues
  - **Mitigation**: Empty default (`:-`) allows explicit overrides to still work. Test with both set and unset environment variables.

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/config/docker-compose.yml` - Modify environment section to remove/clear EMBEDDING_API_ENDPOINT default

## Testing Plan

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md` Phase 4:

```bash
# Test 1: Clean Docker environment
docker compose down -v
docker compose up -d

# Test 2: OpenAI (should use Rust default)
export OPENAI_API_KEY="sk-..."
node bin/cli.cjs setup --provider=openai
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Verify: Uses https://api.openai.com/v1/embeddings

# Test 3: Ollama (should use Rust default)
node bin/cli.cjs setup --provider=ollama
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Verify: Uses http://localhost:11434/api/embed

# Test 4: Custom endpoint (should override)
export EMBEDDING_API_ENDPOINT=http://custom:8080
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Verify: Uses http://custom:8080 (if Ollama provider)
```

**Success Criteria:**
- ✅ No default endpoint in Docker Compose
- ✅ All providers still work correctly
- ✅ Environment is cleaner and less prone to cross-provider pollution
- ✅ Documentation is clear and explains the new behavior

## Success Definition

**Before this ticket:**
- Docker Compose sets `EMBEDDING_API_ENDPOINT=http://ollama:11434` for all providers
- OpenAI provider incorrectly inherits Ollama's endpoint
- Environment is polluted with provider-specific defaults

**After this ticket:**
- Docker Compose doesn't assume any provider endpoint
- Rust code handles provider-specific defaults cleanly
- Environment is cleaner and less prone to bugs
- Users can still override endpoints when needed

The environment variable contract is now clean, with Rust as the single source of truth for provider defaults.

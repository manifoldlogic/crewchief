# Ticket: PROVFIX-3001: Remove CLI Workarounds After Rust Fixes

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose (JavaScript/Node.js cleanup)
- integration-tester (end-to-end OpenAI flow)
- verify-ticket
- commit-ticket

## Summary
Remove explicit endpoint-setting workaround code from CLI that was masking the Rust endpoint resolution bug. Now that PROVFIX-1001 fixed the root cause and PROVFIX-1002 added tests, the CLI can be simplified to its proper role: setting provider and API keys, letting Rust handle endpoint resolution.

## Background
During the provider selection implementation, a workaround was added to the CLI to mask a critical bug in Rust's endpoint resolution. From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md` section "2. Workaround Applied (Must Be Removed)":

**Current workaround in 3 locations:**
```javascript
if (provider === 'openai') {
  providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
  providerEnv.EMBEDDING_DIMENSION = '1536';
  // WORKAROUND: Explicitly set OpenAI endpoint due to Rust bug
  providerEnv.EMBEDDING_API_ENDPOINT = 'https://api.openai.com/v1/embeddings';
}
```

**Why this is brittle:**
- Duplicates endpoint logic that belongs in Rust
- Violates separation of concerns (CLI orchestrates, Rust implements provider logic)
- Must be maintained in parallel with Rust code
- Duplicated in 3 places (runScan, runSetup, upsertFiles)
- Masks the real bug instead of fixing it

**After Rust fix, CLI should ONLY set:**
- `EMBEDDING_PROVIDER` (required)
- `EMBEDDING_MODEL` (provider-specific default)
- `EMBEDDING_DIMENSION` (provider-specific default)
- Provider API keys (OPENAI_API_KEY, etc.)

Rust handles all endpoint resolution based on the provider setting.

This is Phase 3, Ticket 1 of the PROVFIX implementation plan.

## Acceptance Criteria
- [ ] Remove explicit `EMBEDDING_API_ENDPOINT` setting from `runScan()` function
- [ ] Remove explicit `EMBEDDING_API_ENDPOINT` setting from `runSetup()` function
- [ ] Remove explicit `EMBEDDING_API_ENDPOINT` setting from `upsertFiles()` function
- [ ] Remove workaround comments explaining why endpoint was set
- [ ] OpenAI embeddings work without CLI setting endpoint
- [ ] No duplicate endpoint logic between CLI and Rust
- [ ] CLI code is simpler and cleaner

## Technical Requirements

### File: `/workspace/packages/maproom-mcp/bin/cli.cjs`

### 1. Function: `runScan()` (approximately line ~1495)

**Remove:**
```javascript
providerEnv.EMBEDDING_API_ENDPOINT = 'https://api.openai.com/v1/embeddings';
```

**Keep:**
```javascript
providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
providerEnv.EMBEDDING_DIMENSION = '1536';
```

**Remove:** Workaround comment about explicitly setting endpoint

### 2. Function: `runSetup()` (approximately line ~1716)

**Remove:**
```javascript
providerEnv.EMBEDDING_API_ENDPOINT = 'https://api.openai.com/v1/embeddings';
```

**Keep:**
```javascript
providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
providerEnv.EMBEDDING_DIMENSION = '1536';
```

**Remove:** Workaround comment about explicitly setting endpoint

### 3. Function: `upsertFiles()` (approximately line ~1647)

**Remove:**
```javascript
providerEnv.EMBEDDING_API_ENDPOINT = 'https://api.openai.com/v1/embeddings';
```

**Keep:**
```javascript
providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
providerEnv.EMBEDDING_DIMENSION = '1536';
```

**Remove:** Workaround comment about explicitly setting endpoint

### 4. Review Environment Handling Code

- Remove any deletion logic like `delete env.EMBEDDING_API_ENDPOINT` if present
- Simplify debug logging if it references the workaround
- Ensure provider-specific blocks remain clean and minimal

### Expected Final State:

```javascript
if (provider === 'openai') {
  providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
  providerEnv.EMBEDDING_DIMENSION = '1536';
  // NO endpoint override needed - Rust handles it
}
```

## Implementation Notes

See `.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md` section "2. Remove CLI Workarounds" for full context.

### CLI's Proper Responsibility:
- Orchestrate the user experience
- Set high-level configuration (provider, model, dimensions)
- Pass API keys securely to Rust
- Let Rust handle provider-specific logic (endpoints, auth headers, request formats)

### Why This Cleanup is Safe:
- PROVFIX-1001 fixed the root cause in Rust
- PROVFIX-1002 added comprehensive tests proving the fix works
- Workaround can be restored from git if needed (unlikely)

### Testing Approach:
After removing workarounds:
1. Run OpenAI setup: `node bin/cli.cjs setup --provider=openai`
2. Run OpenAI scan: `node bin/cli.cjs scan /workspace/packages/maproom-mcp`
3. Verify embeddings generate without connection errors
4. Check logs to confirm correct endpoint used (should show OpenAI API calls)

## Dependencies
- **Requires:** PROVFIX-1001 (Rust fix must work without CLI workaround)
- **Requires:** PROVFIX-1002 (tests must prove fix works)
- **Blocks:** PROVFIX-5001 (integration testing needs clean code)

## Risk Assessment
- **Risk**: Removing workaround breaks OpenAI functionality
  - **Mitigation**: PROVFIX-1002 tests verified fix works independently; workaround can be restored from git history if needed (though this should not be necessary)

- **Risk**: Other providers affected by cleanup
  - **Mitigation**: Changes only affect provider-specific blocks; Ollama/Google blocks not touched; each provider's configuration is isolated

- **Risk**: Hidden dependencies on workaround behavior
  - **Mitigation**: Integration test (see Testing Notes) validates full flow without workaround; manual testing before commit

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/bin/cli.cjs` (3 functions modified: runScan, runSetup, upsertFiles)

## Testing Notes

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md` section "3. CLI Workaround Removal (Integration Test)":

### Integration Test Plan:

```bash
# Setup with OpenAI (should work without workaround)
export OPENAI_API_KEY="sk-..."
node bin/cli.cjs setup --provider=openai

# Verify setup output
# Should NOT show: EMBEDDING_API_ENDPOINT: https://api.openai.com/v1/embeddings
# Should show: EMBEDDING_PROVIDER: openai

# Scan and generate embeddings
node bin/cli.cjs scan /workspace/packages/maproom-mcp

# Verify scan output
# Should show: Generated: N (N > 0)
# Should NOT show: Failed to generate code embeddings
# Should NOT show: Connection refused errors to localhost:11434
# Should show: API calls made to OpenAI
```

### Success Criteria:
- ✅ Setup completes without workaround code
- ✅ Scan generates embeddings successfully
- ✅ No connection errors or Ollama endpoint attempts
- ✅ Cost tracker shows OpenAI API usage
- ✅ CLI code is simpler and more maintainable

### Before vs. After:

**Before:**
- CLI has workaround code duplicated in 3 places
- Endpoint logic split between CLI (JavaScript) and Rust
- Brittle: changes to endpoints require updates in both places
- Violates separation of concerns

**After:**
- CLI is clean and focused on orchestration
- Rust handles all provider-specific endpoint logic
- Single source of truth for endpoint resolution
- OpenAI works perfectly without CLI intervention

## Planning References
- Analysis: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md`
  - Section: "2. Workaround Applied (Must Be Removed)"
- Architecture: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md`
  - Section: "2. Remove CLI Workarounds"
- Quality: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md`
  - Section: "3. CLI Workaround Removal (Integration Test)"
- Plan: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md`
  - Phase 3, Ticket 1

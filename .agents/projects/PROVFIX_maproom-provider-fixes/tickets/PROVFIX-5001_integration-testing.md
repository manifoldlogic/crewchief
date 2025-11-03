# Ticket: PROVFIX-5001: Integration Testing - Verify Complete Fix

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester (or general-purpose agent)
- verify-ticket

## Summary
Comprehensive integration testing to verify the complete fix works end-to-end across all providers and scenarios. This is the quality gate before considering the project complete - tests the Rust fixes, database migration, CLI cleanup, and Docker changes all working together.

## Background
From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md` Phase 5:

This phase verifies:
1. OpenAI provider works without workarounds
2. Ollama provider still works correctly
3. Environment variable precedence is correct
4. Database updates persist with `updated_at` column
5. No regressions in existing functionality

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md` section "4. Environment Variable Precedence (Manual Verification)":

Four critical test scenarios ensure the fixes work and prevent regression:
- Clean environment (defaults work)
- Wrong endpoint ignored
- Custom valid endpoint accepted
- Database schema updated correctly

This is Phase 5 of the PROVFIX implementation plan - the final validation before documentation.

## Acceptance Criteria
- [ ] **Scenario 1: OpenAI Provider (Clean Environment)**
  - [ ] Setup from scratch succeeds
  - [ ] Scan repository generates embeddings
  - [ ] Cost/token metrics show API usage
  - [ ] No connection errors to localhost:11434
- [ ] **Scenario 2: Ollama Provider (Default Endpoint)**
  - [ ] Setup with default endpoint works
  - [ ] Scan repository generates embeddings (if Ollama running)
- [ ] **Scenario 3: Environment Precedence (Wrong Endpoint Ignored)**
  - [ ] Wrong endpoint ignored by provider validation
  - [ ] Uses correct provider default
- [ ] **Scenario 4: Database Column Verification**
  - [ ] `updated_at` column exists in chunks table
  - [ ] Timestamps populate correctly
  - [ ] Timestamps update on embedding generation
- [ ] **Overall: No Regressions**
  - [ ] All existing functionality works
  - [ ] Error messages are clear
  - [ ] Logs show correct endpoints being used

## Technical Requirements

### Scenario 1: OpenAI Provider (Clean Environment)

**Purpose**: Verify OpenAI works without CLI workarounds

**Commands**:
```bash
# Clean start
docker compose -f /workspace/packages/maproom-mcp/config/docker-compose.yml down -v
docker compose -f /workspace/packages/maproom-mcp/config/docker-compose.yml up -d
unset EMBEDDING_API_ENDPOINT

# Setup
export OPENAI_API_KEY="sk-..."  # Must be valid API key
cd /workspace/packages/maproom-mcp
node bin/cli.cjs setup --provider=openai

# Verify setup output
# Expected: EMBEDDING_PROVIDER: openai
# Expected: EMBEDDING_MODEL: text-embedding-3-small
# Expected: No EMBEDDING_API_ENDPOINT set by CLI

# Scan
node bin/cli.cjs scan /workspace/packages/maproom-mcp

# Verify scan output
# Expected: Generated: N (N > 0)
# Expected: Provider: openai (1536 dimensions)
# Expected: API calls made, tokens counted, cost calculated
# Expected: No "Connection refused" errors
# Expected: No errors to localhost:11434
```

**Success Looks Like**:
```
📊 Embedding Generation Summary:
   Processed 854 chunks in 25s
   Provider: openai (1536 dimensions)
   Generated: 854, Cached: 0, Failed: 0
   API calls: 18, Tokens: 95000, Cost: $0.0019
```

### Scenario 2: Ollama Provider (Default Endpoint)

**Purpose**: Verify Ollama still works with defaults

**Commands**:
```bash
# Setup Ollama
unset EMBEDDING_API_ENDPOINT
cd /workspace/packages/maproom-mcp
node bin/cli.cjs setup --provider=ollama

# Verify setup output
# Expected: EMBEDDING_PROVIDER: ollama
# Expected: Uses http://localhost:11434/api/embed (or http://ollama:11434 in Docker)

# Scan (if Ollama is running)
node bin/cli.cjs scan /workspace/packages/maproom-mcp

# Verify
# Expected: Uses http://localhost:11434/api/embed
# Expected: Embeddings generated (if Ollama running)
# Note: If Ollama not running, expect connection error (that's expected)
```

### Scenario 3: Environment Precedence (Wrong Endpoint Ignored)

**Purpose**: Verify provider validation works

**Commands**:
```bash
# Set wrong endpoint for OpenAI
export EMBEDDING_API_ENDPOINT=http://localhost:11434/api/embed
export EMBEDDING_PROVIDER=openai
export OPENAI_API_KEY="sk-..."
cd /workspace/packages/maproom-mcp
node bin/cli.cjs scan /workspace/packages/maproom-mcp

# Verify behavior
# Expected: Ignores Ollama endpoint from environment
# Expected: Uses https://api.openai.com/v1/embeddings (OpenAI default)
# Expected: Embeddings generate successfully
# Expected: No connection errors
```

**Additional Tests**:
```bash
# Test custom valid endpoint (should work)
export EMBEDDING_API_ENDPOINT=https://api.openai.com/v1/embeddings
export EMBEDDING_PROVIDER=openai
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Expected: Uses custom OpenAI endpoint

# Test Ollama with custom endpoint (should work)
export EMBEDDING_API_ENDPOINT=http://remote-host:11434/api/embed
export EMBEDDING_PROVIDER=ollama
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Expected: Uses custom Ollama endpoint (may fail if host doesn't exist, but should try)
```

### Scenario 4: Database Column Verification

**Purpose**: Verify database migration succeeded

**Commands**:
```bash
# Check database schema
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "\d maproom.chunks"

# Expected output should include:
# updated_at | timestamp with time zone

# Verify updated_at column has data
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "SELECT chunk_id, updated_at FROM maproom.chunks LIMIT 5;"

# Expected: updated_at column with TIMESTAMPTZ values
# Expected: Timestamps are not NULL
# Expected: Timestamps are recent (if embeddings were just generated)

# Verify timestamps update on re-indexing
# (After running scan again)
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "SELECT chunk_id, updated_at FROM maproom.chunks ORDER BY updated_at DESC LIMIT 5;"

# Expected: Timestamps update when embeddings regenerate
```

## Implementation Notes

### Testing Philosophy (from quality-strategy.md)

**Focus on**:
- Preventing regression of the original bugs
- Verifying happy path works for main providers
- Testing environment precedence rules

**Don't over-test**:
- Rate limits (not changed)
- Network failures (existing error handling)
- Concurrent requests (not changed)
- Edge cases that don't matter for MVP

### What Success Looks Like

**Before Fixes**:
```
$ node bin/cli.cjs scan /workspace/packages/maproom-mcp
...
[ERROR] Failed to generate code embeddings: Connection refused (localhost:11434)
Generated: 0, Failed: 854
```

**After Fixes**:
```
$ node bin/cli.cjs scan /workspace/packages/maproom-mcp
...
📊 Embedding Generation Summary:
   Processed 854 chunks in 25s
   Provider: openai (1536 dimensions)
   Generated: 854, Cached: 0, Failed: 0
   API calls: 18, Tokens: 95000, Cost: $0.0019
```

### Execution Order

1. **Scenario 4 first** (Database verification) - Quick check, no API calls needed
2. **Scenario 2 second** (Ollama) - If Ollama running, quick test
3. **Scenario 3 third** (Environment precedence) - Fast validation tests
4. **Scenario 1 last** (OpenAI full scan) - Most comprehensive, uses API calls

### Troubleshooting

**If Scenario 1 fails** (OpenAI errors):
- Check OPENAI_API_KEY is valid
- Verify API key has credits
- Check Docker containers are running
- Review logs for actual error message
- Verify PROVFIX-1001 (Rust fix) was applied
- Verify PROVFIX-3001 (CLI cleanup) was applied

**If Scenario 2 fails** (Ollama errors):
- Expected if Ollama not running
- If Ollama IS running: verify endpoint matches container name
- Check Docker network connectivity

**If Scenario 3 fails** (Wrong endpoint not ignored):
- Verify PROVFIX-1001 (Rust fix) was applied correctly
- Check provider validation logic in config.rs
- Review unit tests from PROVFIX-1002

**If Scenario 4 fails** (Database missing column):
- Verify PROVFIX-2001 (migration) was applied
- Check migration ran: `docker exec maproom-postgres psql -U maproom -d maproom -c "\dt maproom.*"`
- Run migration manually if needed

### Requirements

**Prerequisites**:
- Docker and Docker Compose installed
- Valid OpenAI API key (for Scenario 1)
- Database containers running
- All previous PROVFIX tickets completed:
  - PROVFIX-1001 (Rust fix)
  - PROVFIX-1002 (Unit tests)
  - PROVFIX-2001 (Database migration)
  - PROVFIX-3001 (CLI cleanup)
  - PROVFIX-4001 (Docker cleanup)

**Optional**:
- Ollama running for Scenario 2 (not required)

## Dependencies

**Requires (All must be complete)**:
- PROVFIX-1001 (Fix Rust endpoint resolution)
- PROVFIX-1002 (Add endpoint unit tests)
- PROVFIX-2001 (Add updated_at column)
- PROVFIX-3001 (Remove CLI workarounds)
- PROVFIX-4001 (Clean Docker defaults)

**Blocks**:
- PROVFIX-6001 (Documentation - should document what actually works)

## Risk Assessment

- **Risk**: Integration tests fail, requiring fixes
  - **Mitigation**: Unit tests in PROVFIX-1002 should catch most issues; integration tests catch only cross-layer bugs; fixes should be minor adjustments

- **Risk**: OpenAI API key required for testing
  - **Mitigation**: Agent or user must provide valid API key; test can use minimal data to reduce cost (~$0.01 expected); can skip OpenAI tests if key unavailable and verify other scenarios only

- **Risk**: Time-consuming manual testing
  - **Mitigation**: Focus on critical scenarios only (4 scenarios defined); skip exhaustive edge cases; automated where possible

- **Risk**: Ollama not available for testing
  - **Mitigation**: Ollama test is optional; main fix targets OpenAI provider; can verify Ollama logic without running instance

## Files/Packages Affected

**None** (testing only - no code changes expected)

**If failures occur**, may need to revisit:
- `/workspace/crates/maproom/src/embedding/config.rs` (if Rust fix insufficient)
- `/workspace/packages/maproom-mcp/bin/cli.cjs` (if CLI cleanup incomplete)
- `/workspace/crates/maproom/migrations/` (if database migration issues)

## Planning References

- **Quality Strategy**: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md`
  - Complete test scenarios and expected outputs
  - Testing philosophy and what NOT to test
  - Success criteria definitions

- **Implementation Plan**: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md`
  - Phase 5: Integration Testing section
  - Dependencies on previous phases
  - Overall project objectives

- **Analysis Document**: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md`
  - Original bug description
  - Root cause analysis
  - Why this testing matters

## Success Criteria

**Before Integration Tests**:
- Individual components fixed but full flow untested
- Uncertainty about cross-layer interactions
- Risk of unforeseen bugs

**After Integration Tests Pass**:
- ✅ Complete confidence all fixes work together
- ✅ OpenAI generates embeddings without workarounds
- ✅ Ollama still works with defaults
- ✅ Environment precedence validated
- ✅ Database schema correct
- ✅ No regressions in existing functionality
- ✅ Ready for documentation and project completion

## Completion Notes

This ticket can be marked complete when **all four scenarios pass**.

If **any scenario fails**:
1. Do NOT mark this ticket complete
2. Identify which previous ticket's fix needs adjustment
3. Update/reopen the relevant ticket (PROVFIX-1001, 2001, 3001, or 4001)
4. Fix the underlying issue
5. Re-run integration tests
6. Iterate until all scenarios pass

This is the quality gate - we do not proceed to documentation (PROVFIX-6001) until all integration tests pass.

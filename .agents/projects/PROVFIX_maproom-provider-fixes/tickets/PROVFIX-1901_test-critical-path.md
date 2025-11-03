# Ticket: PROVFIX-1901: Critical Path Testing - Endpoint Resolution and Database

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner (primary - for automated tests)
- general-purpose (alternative - for smoke tests if needed)
- verify-ticket

## Summary
Focused testing ticket to validate the two most critical fixes work correctly: (1) Rust endpoint resolution prevents cross-provider pollution, and (2) Database schema supports embedding persistence. This ticket runs after PROVFIX-1002 unit tests pass and verifies the fixes work in a realistic scenario.

## Background
From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md`:

**Testing Philosophy:**
> Goal: Prevent regression and verify fixes work correctly without exhaustive ceremony.
> Approach: Focused tests that caught the original bugs + integration tests for the happy path.
> Not Testing: Edge cases that don't matter for MVP

This ticket focuses on:
1. The exact bug scenario (OpenAI inheriting Ollama endpoint) - must not regress
2. Database persistence (embeddings must save) - must not regress
3. Minimal end-to-end validation - smoke test only

**Why 900s numbering?** Following CrewChief convention for test tickets in Phase 1.

This is a Phase 1 test ticket using the 900s numbering convention for tests. It provides developer confidence that critical path works before moving to later phases.

## Acceptance Criteria
- [x] Rust unit tests pass (from PROVFIX-1002)
- [x] Quick smoke test: OpenAI doesn't use Ollama endpoint
- [x] Quick smoke test: Embeddings persist to database
- [x] No critical path regressions detected
- [x] Test execution < 5 minutes

## Technical Requirements

### Test 1: Unit Tests (Automated)
```bash
# Run Rust unit tests from PROVFIX-1002
cd /workspace/crates/maproom
cargo test --lib config_endpoint_tests -- --test-threads=1

# IMPORTANT: Must use --test-threads=1 to prevent environment variable pollution
# The tests use env::set_var() which affects the global process environment
# Running tests in parallel causes cross-test interference and random failures

# Expected: All 8 tests pass
# Key test: test_openai_ignores_ollama_endpoint (THE BUG TEST)
```

**Why this matters:** Unit tests from PROVFIX-1002 include the exact bug scenario that caused the original issue. If these pass, the Rust fix is working correctly.

### Test 2: Endpoint Resolution Smoke Test (Quick Manual)
```bash
# Set Ollama endpoint in environment
export EMBEDDING_API_ENDPOINT=http://localhost:11434/api/embed
export EMBEDDING_PROVIDER=openai
export OPENAI_API_KEY="sk-test-fake-key"

# Create minimal config and check endpoint resolution
# This can be done by:
# - Running maproom with debug logging
# - OR writing a small Rust test binary
# - OR checking logs during a quick scan

# Expected behavior:
# - Config loads with provider=openai
# - api_endpoint_url() returns "https://api.openai.com/v1/embeddings"
# - NOT "http://localhost:11434/api/embed"
```

**Why this matters:** Validates the fix works in a realistic environment variable scenario, not just in unit tests.

### Test 3: Database Persistence Smoke Test (Quick Manual)
```bash
# Verify database has updated_at column
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "\d maproom.chunks"

# Expected: updated_at column exists with type TIMESTAMPTZ

# Optional: If time allows, insert a test chunk and verify trigger works
# This validates migration applied correctly
```

**Why this matters:** Confirms the database migration from PROVFIX-2001 applied successfully and the schema supports embedding persistence.

## Implementation Notes

### Testing Approach
From quality strategy:

> We're NOT testing: Edge cases, network errors, rate limits, performance, concurrent requests

Focus testing effort on what matters:
1. **Automated first:** Run cargo test (fast, reliable)
2. **Smoke tests second:** Quick manual validation (< 2 min each)
3. **Skip if confident:** If unit tests pass and code review looks good, smoke tests are optional

### MVP Test Strategy
This ticket implements the MVP test strategy:
- **Test high-confidence critical paths only**
- **Skip exhaustive ceremony**
- **Prevent regression of known bugs**
- **Validate fixes work in realistic scenarios**

### Time Budget
- Automated unit tests: < 30 seconds
- Smoke test #1 (endpoint resolution): < 2 minutes
- Smoke test #2 (database schema): < 1 minute
- **Total: < 5 minutes**

### When to Skip Smoke Tests
Smoke tests are optional if:
- All unit tests pass cleanly
- Code review of PROVFIX-1001 and PROVFIX-2001 looks solid
- Confident in the implementation

Smoke tests provide extra confidence but aren't required for ticket completion.

## Dependencies
- **Requires:** PROVFIX-1001 (Rust fix implemented)
- **Requires:** PROVFIX-1002 (Unit tests added)
- **Requires:** PROVFIX-2001 (Database migration applied)
- **Optional:** Can run before PROVFIX-3001 (CLI cleanup doesn't affect core logic)

## Risk Assessment
- **Risk**: Tests don't catch regression
  - **Mitigation**: Unit tests in 1002 are comprehensive; this ticket adds confidence via smoke tests
  - **Likelihood**: Low - unit tests cover the exact bug scenario

- **Risk**: Testing takes too long, slows development
  - **Mitigation**: Keep tests focused, skip non-critical scenarios
  - **Time budget**: < 5 minutes total

- **Risk**: Smoke tests require complex setup
  - **Mitigation**: Make smoke tests optional; unit tests are sufficient for core validation

## Files/Packages Affected
None (testing only, no code changes)

## Relationship to PROVFIX-5001
- **This ticket (1901)**: Quick critical path validation in Phase 1
- **PROVFIX-5001**: Comprehensive integration testing in Phase 5

Think of 1901 as "developer confidence check" and 5001 as "QA gate before documentation".

1901 provides fast feedback during development. 5001 provides comprehensive validation before release.

## Success Criteria

**Before:** Fixes implemented but critical path unvalidated

**After:** High confidence critical path works, ready for integration testing

**Tests pass:**
- ✅ All 8 unit tests pass (especially test_openai_ignores_ollama_endpoint)
- ✅ Endpoint resolution smoke test confirms correct behavior
- ✅ Database schema verified
- ✅ No critical regressions

## Completion Notes
This ticket can be marked complete when:
1. Unit tests pass cleanly (`cargo test config_tests`)
2. Optional smoke tests confirm no obvious regressions (or skipped if confident)
3. verify-ticket agent confirms acceptance criteria met

Full integration testing happens in PROVFIX-5001.

## Planning References
- Quality Strategy: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md`
  - Section: "Phase 1 Critical Path Testing"
  - Section: "MVP Test Strategy"
- Plan: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md`
  - Phase 1, Test Ticket (900s series)

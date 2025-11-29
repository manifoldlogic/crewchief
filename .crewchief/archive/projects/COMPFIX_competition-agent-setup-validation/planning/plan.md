# Execution Plan: Competition Agent Setup and Validation

## Project Overview

Fix the AGENTOPT competition framework by implementing comprehensive pre-flight validation, worktree scanning orchestration, and fail-fast error handling. Transform from 0% success rate to 100% valid agent environments.

**Success Criteria:**
- All agents have access to maproom search tools
- All worktrees are indexed before agent spawn
- Setup failures detected and reported before wasting API credits
- At least 50% of agents choose to use search tool (vs 0% currently)

## Phases

### Phase 1: Core Validation Infrastructure (Week 1)

**Objective:** Build validation framework and integrate into competition runner

#### Ticket 1.1: Pre-Flight Validation Module
**Agent:** general-purpose
**Est:** 4-6 hours

Create `packages/cli/src/search-optimization/validation/pre-flight-validator.ts`

**Deliverables:**
- `PreFlightValidator` class with methods:
  - `checkDatabaseConnection()`
  - `verifyBaseBranchIndexed(repo, branch)`
  - `checkWorktreeScanned(repo, worktree)`
  - `checkMcpConfigValid(worktreePath)`
  - `checkFilePermissions(worktreePath)`
  - `validateVariantEnvironment(env)`
  - `validateCompetitionSetup(config)`

**Acceptance Criteria:**
- All validation functions return `CheckResult` with pass/fail + message
- Database connection tested with actual PostgreSQL client
- Worktree scan status checked via `maproom status --json`
- MCP config parsed and validated
- File permissions tested with read/write operations
- Comprehensive error messages for all failure modes

**Tests:**
- Unit tests for each validation function
- Mock database failures, missing configs, permission errors
- 95%+ coverage on critical paths

#### Ticket 1.2: Scan Orchestration Module
**Agent:** general-purpose
**Est:** 3-4 hours

Create `packages/cli/src/search-optimization/scan-orchestrator.ts`

**Deliverables:**
- `scanWorktree(config)` - Single worktree scan
- `scanAllWorktrees(configs)` - Sequential batch scanning
- `waitForScanCompletion(scanId, timeout)` - Poll for completion
- Scan result parsing and reporting

**Acceptance Criteria:**
- Scans executed via `spawn()` (not `execSync` - command injection protection)
- Arguments passed as array (no shell interpretation)
- Output parsed for chunk count and duration
- Errors captured and reported with context
- Progress logging for multi-worktree batches
- Fail-fast on scan errors (don't continue if scan fails)

**Tests:**
- Unit tests with mocked spawn
- Test scan success and failure paths
- Verify error messages are actionable
- 95%+ coverage

#### Ticket 1.3: Enhanced Competition Runner
**Agent:** general-purpose
**Est:** 6-8 hours

Modify `packages/cli/src/search-optimization/competition-runner.ts`

**Deliverables:**
- Phase 1: Setup (sequential)
  - Validate database connection
  - Verify base branch indexed
  - Create competition directory
  - Load variants
  - Create worktrees
  - Inject variant descriptions
  - **NEW:** Scan all worktrees
  - **NEW:** Validate all environments

- Phase 2: Validation (per-variant)
  - **NEW:** Run pre-flight checks
  - **NEW:** Fail fast on validation errors

- Phase 3: Execution (parallel)
  - Spawn agents (existing logic)
  - Collect results (existing logic)

- Phase 4: Evaluation (existing logic)

**Acceptance Criteria:**
- Setup runs sequentially (worktrees → scans → validation)
- Execution runs in parallel (agents)
- Clear console output for each phase
- Validation failures throw errors immediately
- No agents spawned if validation fails
- Competition report includes setup metrics (scan times, validation results)

**Tests:**
- Integration test: Full competition happy path
- Integration test: Database failure stops early
- Integration test: Scan failure stops early
- Integration test: Validation failure stops early
- Verify validation runs before agent spawn

#### Ticket 1.4: Security Controls
**Agent:** general-purpose
**Est:** 2-3 hours

Add security validations and sanitization

**Deliverables:**
- Variant ID validation (path traversal protection)
  ```typescript
  function validateVariantId(id: string): void {
    if (id.includes('..') || id.includes('/') || id.includes('\\')) {
      throw new Error('Invalid variant ID: path traversal detected')
    }
    if (!/^[a-zA-Z0-9_-]+$/.test(id)) {
      throw new Error('Invalid variant ID: only alphanumeric, dash, underscore')
    }
    if (id.length > 64) {
      throw new Error('Invalid variant ID: max 64 characters')
    }
  }
  ```

- Resource limits
  ```typescript
  const MAX_VARIANTS = 50
  const MAX_PARALLEL_AGENTS = 10
  const MAX_TIMEOUT = 600_000 // 10 min
  ```

- Sensitive data sanitization
  ```typescript
  function sanitizeDbUrl(url: string): string {
    return url.replace(/:\/\/([^:]+):([^@]+)@/, '://***:***@')
  }
  ```

- Command injection protection (use `spawn` with args array)

**Acceptance Criteria:**
- Variant IDs validated before use
- Resource limits enforced
- Database URLs sanitized in logs
- All `execSync` calls replaced with `spawn` + args array
- No shell interpretation in subprocess execution

**Tests:**
- Unit test: Path traversal attempts rejected
- Unit test: Resource limits enforced
- Unit test: URLs sanitized correctly
- Unit test: Command execution uses spawn (not shell)

### Phase 2: Documentation and Validation (Week 1, Days 4-5)

#### Ticket 2.1: Update Documentation
**Agent:** general-purpose
**Est:** 2-3 hours

**Deliverables:**
- Update `docs/search-optimization/competition-framework.md`
  - Add "Pre-Flight Validation" section
  - Document validation checks and error messages
  - Add troubleshooting guide
  - Update setup instructions

- Update `packages/cli/src/search-optimization/README.md`
  - Document scan orchestration
  - Add validation workflow diagram
  - Update cost estimates (include setup time)

- Update genetic optimizer scripts
  - Add validation phase to console output
  - Document expected setup time
  - Update cost estimates

**Acceptance Criteria:**
- All new validation features documented
- Error messages explained with fixes
- Setup time estimates updated
- Examples include validation phase

#### Ticket 2.2: End-to-End Validation
**Agent:** verify-ticket (manual execution)
**Est:** 2-3 hours

**Deliverables:**
- Run standard optimizer (5 variants)
- Run premium optimizer (8 variants)
- Run ultra optimizer (12 variants)
- Verify all validations pass
- Verify agents use search tool
- Document actual timing metrics

**Acceptance Criteria:**
- All 3 optimizer levels complete successfully
- Setup phase completes without errors
- At least 50% of agents use `mcp__maproom__search`
- No agents have 0 searches (all have tool access)
- Actual setup time < estimated setup time
- Competition reports show meaningful scores (not all 18-19%)

#### Ticket 2.3: Error Scenario Testing
**Agent:** verify-ticket (manual execution)
**Est:** 1-2 hours

**Deliverables:**
- Test: Database disconnected → Error caught before agent spawn
- Test: Base branch not indexed → Clear error message
- Test: Worktree scan fails → Competition stops
- Test: MCP config malformed → Validation fails
- Test: Permission denied → Error reported

**Acceptance Criteria:**
- All error scenarios caught by validation
- Error messages are actionable (tell user how to fix)
- No API credits wasted on broken setups
- Validation failures logged to console and report

### Phase 3: Performance Optimization (Optional, Week 2)

#### Ticket 3.1: Parallel Scanning (Future Enhancement)
**Agent:** TBD
**Est:** 4-6 hours
**Status:** OUT OF SCOPE for MVP

Implement concurrent worktree scanning:
- Use worker pool pattern
- Limit concurrent scans (e.g., 4 at a time)
- Handle race conditions on database writes
- Monitor for deadlocks or conflicts

**Benefit:** Reduce 120s setup → 30s setup (4x speedup)

**Risk:** Database concurrency issues, harder debugging

**Decision:** Defer to Phase 2 (sequential is fast enough)

#### Ticket 3.2: Validation Result Caching (Future Enhancement)
**Agent:** TBD
**Est:** 3-4 hours
**Status:** OUT OF SCOPE for MVP

Cache validation results across generations:
- Hash worktree state (git commit + variant ID)
- Store validation results in `.crewchief/validation-cache.json`
- Reuse if worktree unchanged
- Invalidate on state change

**Benefit:** Skip re-validation for unchanged worktrees

**Risk:** Stale cache causing false positives

**Decision:** Defer to Phase 2 (validation is fast enough)

## Timeline

### Week 1

**Days 1-2: Core Infrastructure**
- Ticket 1.1: Pre-Flight Validation Module (6h)
- Ticket 1.2: Scan Orchestration Module (4h)

**Days 3-4: Integration**
- Ticket 1.3: Enhanced Competition Runner (8h)
- Ticket 1.4: Security Controls (3h)

**Day 5: Validation**
- Ticket 2.1: Update Documentation (3h)
- Ticket 2.2: End-to-End Validation (3h)
- Ticket 2.3: Error Scenario Testing (2h)

**Total: ~29-35 hours (5-7 days at 5h/day)**

## Dependencies

### External Dependencies
- PostgreSQL running and accessible
- Base branch already indexed
- Anthropic API key configured
- Claude Code Agents SDK installed

### Internal Dependencies
- Existing competition runner (modified, not replaced)
- Maproom scan command available
- MCP server configuration format
- Variant loading infrastructure

### Dependency Chain
```
1.1 (Validation) ──┬──> 1.3 (Competition Runner)
1.2 (Scanning)   ──┘       │
                           ├──> 1.4 (Security)
                           ├──> 2.1 (Docs)
                           └──> 2.2 (E2E Test)
                                 └──> 2.3 (Error Tests)
```

**Critical Path:** 1.1 → 1.3 → 2.2 (validation module is foundation)

## Risk Mitigation

### High-Risk Items

1. **Integration with Claude SDK**
   - Risk: SDK changes break our validation
   - Mitigation: Version lock SDK in package.json
   - Fallback: Document required SDK version

2. **Database connection failures**
   - Risk: Hard to test all failure modes
   - Mitigation: Extensive unit tests with mocked pg client
   - Fallback: Manual testing with stopped PostgreSQL

3. **Scan command variations**
   - Risk: maproom output format changes
   - Mitigation: Parse JSON output (stable format)
   - Fallback: Regex parsing as backup

### Medium-Risk Items

1. **Performance regression**
   - Risk: Setup takes too long (>5min for 12 variants)
   - Mitigation: Measure actual times in E2E test
   - Fallback: Parallelize scans if needed

2. **Permission issues**
   - Risk: Platform-specific file permission differences
   - Mitigation: Test on Linux and macOS
   - Fallback: Document platform-specific setup

3. **MCP config format changes**
   - Risk: .mcp.json structure changes
   - Mitigation: Parse and validate structure
   - Fallback: Fail with clear error message

## Success Metrics

### Quantitative

- ✅ Validation catches 100% of setup failures
- ✅ 0% of competitions waste API credits on invalid setups
- ✅ At least 50% of agents use search tool (vs 0% before)
- ✅ Setup time < 5 minutes for 12 variants
- ✅ All unit tests pass (95%+ coverage on critical paths)
- ✅ Integration tests pass (happy path + 4 error scenarios)

### Qualitative

- ✅ Error messages are actionable (user knows how to fix)
- ✅ Console output clearly shows validation progress
- ✅ Genetic optimizer runs complete successfully
- ✅ Competition reports show realistic scores (not all ~18%)
- ✅ Documentation covers all validation features

## Rollout Plan

### Stage 1: Local Testing
- Developer runs all optimizer scripts
- Verify validation works on development machine
- Fix any environment-specific issues

### Stage 2: CI Testing
- Run integration tests in GitHub Actions
- Verify PostgreSQL setup in CI
- Ensure all tests pass

### Stage 3: Production Deployment
- Merge to main branch
- Update optimizer scripts to use new validation
- Announce in project README

### Stage 4: Monitoring
- Track: % competitions that fail validation
- Track: % agents that use search tool
- Track: Average setup time per variant
- Alert: If validation failure rate > 5%

## Maintenance

### Ongoing
- Monitor Dependabot alerts for security updates
- Review failed validation patterns (improve error messages)
- Update documentation when MCP format changes

### Periodic
- Quarterly: Review validation logic for improvements
- Quarterly: Run full optimizer suite to verify no regressions
- Annually: Consider Phase 2 optimizations (parallel scanning, caching)

## Agent Assignments

Based on agent capabilities from `.crewchief/agents/README.md`:

| Ticket | Agent | Rationale |
|--------|-------|-----------|
| 1.1 | general-purpose | Validation logic (TypeScript) |
| 1.2 | general-purpose | Scan orchestration (TypeScript) |
| 1.3 | general-purpose | Competition runner (TypeScript) |
| 1.4 | general-purpose | Security controls (TypeScript) |
| 2.1 | general-purpose | Documentation updates |
| 2.2 | verify-ticket | End-to-end validation |
| 2.3 | verify-ticket | Error scenario testing |

**Note:** No specialized agents needed - all work is TypeScript with standard tools

## Open Questions

1. **Should we validate tool descriptions for quality?**
   - Answer: No (out of scope - we test effectiveness, not quality)

2. **Should we retry failed scans?**
   - Answer: No (fail fast - user should fix underlying issue)

3. **Should we support offline mode (no database)?**
   - Answer: No (database is required for semantic search)

4. **Should we add telemetry/metrics collection?**
   - Answer: Future enhancement (Phase 2)

## Appendix: Validation Flow Diagram

```
Start Competition
       │
       ├─▶ Check Database Connection
       │      ├─ PASS → Continue
       │      └─ FAIL → Error: "Database connection failed"
       │
       ├─▶ Verify Base Branch Indexed
       │      ├─ PASS → Continue
       │      └─ FAIL → Error: "Base branch not indexed"
       │
       ├─▶ Create Worktrees (sequential)
       │      └─▶ For each variant:
       │             ├─ Create directory
       │             └─ Copy base files
       │
       ├─▶ Inject Variant Descriptions
       │      └─▶ For each worktree:
       │             ├─ Modify .mcp.json
       │             └─ Add custom tool description
       │
       ├─▶ Scan Worktrees (sequential)
       │      └─▶ For each worktree:
       │             ├─ Run: maproom scan
       │             ├─ Wait for completion
       │             ├─ Parse chunk count
       │             └─ FAIL if scan errors
       │
       ├─▶ Validate Environments (per-variant)
       │      └─▶ For each worktree:
       │             ├─ Check: Worktree exists
       │             ├─ Check: Indexed (chunks > 0)
       │             ├─ Check: MCP config valid
       │             ├─ Check: File permissions OK
       │             └─ FAIL if any check fails
       │
       ├─▶ Spawn Agents (parallel)
       │      └─▶ For each environment:
       │             ├─ Only if validation passed
       │             ├─ Run agent with task
       │             └─ Collect results
       │
       └─▶ Evaluate Results
              ├─ Determine winner
              ├─ Generate report
              └─ Save to disk
```

## Appendix: Error Message Examples

**Good error messages** (what we want):

```
❌ Pre-flight validation failed: Database connection failed

Troubleshooting:
- Verify PostgreSQL is running: docker ps | grep postgres
- Check MAPROOM_DATABASE_URL environment variable
- Test connection: psql $MAPROOM_DATABASE_URL -c "SELECT 1"

Current value: postgresql://maproom:***@localhost:5432/maproom
```

```
❌ Pre-flight validation failed: Base branch 'main' not indexed

Fix: Run scan on base branch first
$ crewchief-maproom scan --repo crewchief --worktree main --root /workspace

This is a one-time setup step. Subsequent scans will be fast.
```

**Bad error messages** (what we don't want):

```
Error: undefined
```

```
Scan failed
```

```
Database error: 42P01
```

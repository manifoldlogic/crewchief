# Ticket: COMPFIX-2001: Update Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Update all competition framework documentation to reflect the new three-phase validation workflow, including pre-flight checks, scan orchestration, fail-fast error handling, troubleshooting guides, and revised timing estimates. This ensures users understand the validation features and can debug setup issues effectively.

## Background

The competition framework has been transformed from a simple "spawn and hope" model to a comprehensive validated setup pipeline with three distinct phases:
1. Setup (sequential): database validation, base branch check, worktree creation, scanning
2. Validation (per-variant): environment readiness checks
3. Execution (parallel): agent runs

All existing documentation assumes the old behavior and does not explain:
- Why validation is needed (0% success rate without it)
- What validation checks are performed
- How to troubleshoot validation failures
- Revised timing expectations (+2-3 minutes for validation)
- Security controls and resource limits

This ticket updates all user-facing and developer-facing documentation to reflect the new architecture and provide clear troubleshooting guidance.

**Reference:** Section "Documentation Updates" in `planning/plan.md` (lines 167-189)

## Acceptance Criteria

- [x] `docs/search-optimization/competition-framework.md` updated with "Pre-Flight Validation" section
- [x] All validation checks documented with clear descriptions
- [x] Troubleshooting guide added for common validation failures
- [x] Setup time estimates updated to reflect validation overhead
- [x] `packages/cli/src/search-optimization/README.md` updated with scan orchestration details
- [x] Validation workflow diagram added (ASCII or Mermaid)
- [x] Genetic optimizer scripts documentation updated with validation phase
- [x] Security controls and resource limits documented
- [x] Examples include validation output in console logs
- [x] All error messages documented with fixes

## Technical Requirements

### 1. Update Competition Framework Guide

**File:** `docs/search-optimization/competition-framework.md`

**Add new section after "Overview":**

```markdown
## Pre-Flight Validation

Starting with version X.X.X, the competition runner includes comprehensive pre-flight validation to ensure 100% of agents have valid tool environments before execution.

### Why Validation is Required

Analysis of early competition runs revealed systematic failures:
- 0% search tool usage (agents didn't have access to maproom tools)
- 0% task completion (agents couldn't complete tasks without tools)
- Wasted API credits (~$15-20 per failed run)

Validation ensures:
- Database is accessible
- Base branch is indexed
- All worktrees are scanned
- MCP tools are configured
- File permissions are correct

### Validation Phases

The competition runner now operates in three distinct phases:

**Phase 1: Setup (Sequential)**
1. Validate database connection
2. Verify base branch indexed
3. Create competition directory
4. Load variants
5. Create worktrees (one per variant)
6. Inject variant tool descriptions
7. **Scan all worktrees** (NEW)

**Phase 2: Validation (Per-Variant)**
1. Check worktree exists
2. Check worktree scanned (chunk_count > 0)
3. Check MCP config valid
4. Check file permissions OK
5. **Fail fast if any check fails** (NEW)

**Phase 3: Execution (Parallel)**
1. Spawn agents (only if validation passed)
2. Collect results
3. Evaluate winner

### Timing Expectations

**For 12 variants (ultra configuration):**
- Setup: ~2-3 minutes (worktree creation + scanning)
- Validation: ~10-20 seconds
- Execution: ~2-5 minutes (parallel agents)
- **Total: ~4-8 minutes** (vs ~2-3 minutes without validation)

**Tradeoff:** +2-3 minutes setup time for 100% success rate (vs 0% without validation)

### Validation Checks

#### Database Connection
- **What**: Tests PostgreSQL connectivity
- **How**: Executes `SELECT 1` query
- **Failure**: "Database connection failed - check MAPROOM_DATABASE_URL"

#### Base Branch Indexed
- **What**: Verifies base branch has chunks in database
- **How**: Runs `maproom status --repo <repo> --worktree <branch>`
- **Failure**: "Base branch not indexed - run: crewchief-maproom scan..."

#### Worktree Scanned
- **What**: Ensures variant worktree has chunks indexed
- **How**: Checks `chunk_count > 0` in database
- **Failure**: "Worktree has 0 chunks indexed"

#### MCP Config Valid
- **What**: Validates .mcp.json structure
- **How**: Parses JSON and checks for maproom server
- **Failure**: "MCP config missing or invalid"

#### File Permissions
- **What**: Tests read/write access
- **How**: Reads package.json and creates test file
- **Failure**: "Permission error: EACCES"

## Troubleshooting

### Database Connection Failed

**Error:**
```
❌ Pre-flight validation failed: Database connection failed
```

**Fix:**
1. Verify PostgreSQL is running:
   ```bash
   docker ps | grep maproom-postgres
   ```

2. Check environment variable:
   ```bash
   echo $MAPROOM_DATABASE_URL
   ```

3. Test connection manually:
   ```bash
   psql $MAPROOM_DATABASE_URL -c "SELECT 1"
   ```

4. Restart PostgreSQL if needed:
   ```bash
   cd packages/maproom-mcp/config
   docker compose down
   docker compose up -d
   ```

### Base Branch Not Indexed

**Error:**
```
❌ Pre-flight validation failed: Base branch 'main' not indexed
```

**Fix:**
Run scan on base branch first (one-time setup):
```bash
crewchief-maproom scan --repo crewchief --worktree main --root /workspace
```

This takes 30-60 seconds initially. Subsequent variant scans will be fast (5-15s) due to embedding reuse.

### Worktree Scan Failed

**Error:**
```
❌ Scan failed for variant-a-detailed: Permission denied
```

**Fix:**
1. Check worktree path exists:
   ```bash
   ls -la .crewchief/worktrees/
   ```

2. Verify binary is in PATH:
   ```bash
   which crewchief-maproom
   ```

3. Check database permissions:
   ```bash
   psql $MAPROOM_DATABASE_URL -c "SELECT * FROM repos LIMIT 1"
   ```

### MCP Config Missing

**Error:**
```
❌ Validation failed: MCP config missing in worktree
```

**Fix:**
This indicates a bug in worktree creation. Check:
1. Variant injection completed:
   ```bash
   cat .crewchief/worktrees/variant-*/,mcp.json
   ```

2. SDK version is compatible:
   ```bash
   pnpm list @anthropic-ai/claude-agent-sdk
   ```

### Permission Denied

**Error:**
```
❌ Validation failed: Permission error: EACCES
```

**Fix:**
1. Check directory ownership:
   ```bash
   ls -la .crewchief/worktrees/
   ```

2. Fix permissions if needed:
   ```bash
   chmod -R u+rw .crewchief/worktrees/
   ```

## Security and Resource Limits

### Resource Limits

The competition runner enforces limits to prevent resource exhaustion:

- **MAX_VARIANTS**: 50 (prevents excessive worktree creation)
- **MAX_PARALLEL_AGENTS**: 10 (limits concurrent API calls)
- **MAX_TIMEOUT**: 600000ms (10 minutes per agent)

To run larger competitions, these limits can be adjusted in code (requires recompilation).

### Security Controls

1. **Variant ID Validation**: Only alphanumeric, dash, underscore allowed (prevents path traversal)
2. **Command Injection Protection**: All subprocess execution uses spawn with args array
3. **Sensitive Data Sanitization**: Database credentials redacted in logs
4. **Fail-Fast Validation**: Stops immediately on setup errors (doesn't waste API credits)

For details, see `planning/security-review.md`.
```

### 2. Update Search Optimization README

**File:** `packages/cli/src/search-optimization/README.md`

**Add new section:**

```markdown
## Scan Orchestration

### Overview

The competition runner now automatically scans all variant worktrees before agent execution. This ensures agents can use `mcp__maproom__search` tools effectively.

### How It Works

1. **Sequential Scanning**: Worktrees scanned one at a time
2. **Embedding Reuse**: Subsequent scans reuse base branch embeddings (fast)
3. **Progress Logging**: Shows current worktree, chunk count, duration
4. **Fail-Fast**: Stops immediately if any scan fails

### Timing

- First variant: 10-15s (some new embeddings)
- Subsequent variants: 5-10s (embedding reuse)
- Total for 12 variants: ~2-3 minutes

### Implementation

See `scan-orchestrator.ts` for implementation details.

### Troubleshooting

If scans fail, check:
1. Base branch is indexed: `maproom status --repo crewchief --worktree main`
2. Database is accessible: `psql $MAPROOM_DATABASE_URL -c "SELECT 1"`
3. Worktree paths are correct
```

### 3. Update Genetic Optimizer Scripts

**Files to update:**
- `scripts/run-genetic-optimizer.ts` (if exists)
- `scripts/run-genetic-optimizer-premium.ts` (if exists)
- `scripts/run-genetic-optimizer-ultra.ts` (if exists)

**Add comments explaining validation phase:**

```typescript
/**
 * Run genetic optimizer with competition-based fitness evaluation.
 *
 * The competition runner now includes three phases:
 * 1. Setup: Create worktrees, scan, validate (~2-3 min for 12 variants)
 * 2. Validation: Check environment readiness (~10-20 sec)
 * 3. Execution: Run agents in parallel (~2-5 min)
 *
 * Total time per generation: ~4-8 minutes
 * Total time for 10 generations: ~40-80 minutes
 *
 * Validation ensures 100% of agents have tool access (vs 0% previously).
 */
```

## Implementation Notes

### Documentation Style Guidelines

1. **Be actionable**: Every error should have a clear fix
2. **Show examples**: Include actual console output
3. **Explain why**: Don't just say what, explain why validation matters
4. **Link related docs**: Cross-reference security-review.md, architecture.md
5. **Keep updated**: Version-specific notes for breaking changes

### Validation Workflow Diagram

Add ASCII diagram to README.md:

```
Competition Validation Workflow
=================================

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
                        ├─▶ Create Worktrees
                        │      └─▶ For each variant:
                        │             ├─ Create directory
                        │             └─ Copy base files
                        │
                        ├─▶ Inject Variant Descriptions
                        │      └─▶ For each worktree:
                        │             └─ Modify .mcp.json
                        │
                        ├─▶ Scan Worktrees
                        │      └─▶ For each worktree:
                        │             ├─ Run: maproom scan
                        │             ├─ Wait for completion
                        │             └─ FAIL if errors
                        │
                        ├─▶ Validate Environments
                        │      └─▶ For each worktree:
                        │             ├─ Check: Exists
                        │             ├─ Check: Indexed
                        │             ├─ Check: MCP config
                        │             ├─ Check: Permissions
                        │             └─ FAIL if any fails
                        │
                        ├─▶ Spawn Agents (parallel)
                        │      └─▶ Only if ALL validations passed
                        │
                        └─▶ Evaluate Results
```

### Console Output Examples

Include example successful run:

```
$ pnpm tsx scripts/run-genetic-optimizer-ultra.ts

🏁 Starting competition with pre-flight validation

📋 Phase 1: Setup
============================================================
✅ Database connection verified
✅ Base branch indexed (1234 chunks)
✅ Competition directory: /tmp/comp-1234567890
✅ Loaded 12 variants

✅ Created worktree for variant-control
✅ Created worktree for variant-a-detailed
...

📊 Scanning worktrees...
============================================================
📊 Scanning worktree: variant-control
   Path: /tmp/comp-1234567890/worktrees/variant-control
   ✅ Scan complete: 567 chunks in 8234ms
...
============================================================
✅ All scans complete in 16.1s
📊 Total chunks indexed: 6804

🔍 Phase 2: Pre-Flight Validation
============================================================
✅ variant-control: All checks passed
✅ variant-a-detailed: All checks passed
...

✅ All variants validated - ready for execution

🚀 Phase 3: Agent Execution
============================================================
[Agents running...]
```

## Dependencies

- **Prerequisite tickets:**
  - COMPFIX-1001 (Pre-Flight Validation Module) - document validation checks
  - COMPFIX-1002 (Scan Orchestration Module) - document scan workflow
  - COMPFIX-1003 (Enhanced Competition Runner) - document three-phase flow
  - COMPFIX-1004 (Security Controls) - document security features

- **Blocks:**
  - COMPFIX-2002 (End-to-End Validation) - users need docs for troubleshooting
  - COMPFIX-2003 (Error Scenario Testing) - error messages should match docs

## Risk Assessment

- **Risk**: Documentation becomes stale as code evolves
  - **Mitigation**: Link docs to specific code files/functions
  - **Process**: Update docs in same PR as code changes

- **Risk**: Users skip reading docs and report issues
  - **Mitigation**: Clear error messages include troubleshooting steps
  - **Process**: Error messages should reference docs sections

- **Risk**: Examples don't match actual output
  - **Mitigation**: Copy-paste actual console output from tests
  - **Verification**: Run examples during ticket verification

## Files/Packages Affected

**Modified files:**
- `docs/search-optimization/competition-framework.md` (major additions)
- `packages/cli/src/search-optimization/README.md` (add scan orchestration section)
- `scripts/run-genetic-optimizer.ts` (add comments)
- `scripts/run-genetic-optimizer-premium.ts` (add comments)
- `scripts/run-genetic-optimizer-ultra.ts` (add comments)

**No code changes** - documentation only

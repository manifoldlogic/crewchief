# COMPFIX: Competition Agent Setup and Validation

## Project Summary

Fix the AGENTOPT genetic optimizer competition framework by implementing comprehensive pre-flight validation, worktree scanning orchestration, and fail-fast error handling. Transform the competition runner from 0% success rate (agents have no tools) to 100% valid agent environments.

## Problem Statement

Analysis of ultra-run-1762742953256 revealed systematic failures across all 6 completed generations:

**Current State (Broken):**
- ❌ 0% search tool usage (agents don't have access to maproom tools)
- ❌ 0% task completion (agents can't complete tasks without tools)
- ❌ 0% success rate (complete framework failure)
- ❌ Wasted API credits (~$15-20 on failed run)

**Root Causes:**
1. Agents don't have access to `mcp__maproom__search` tools
2. Worktrees never scanned (not indexed in database)
3. No pre-flight validation (tests start regardless of setup state)
4. Silent failures (no feedback when environment is broken)

**Impact:**
- Competition framework unusable for tool description optimization
- Cannot determine which variant descriptions are effective
- Wasted API costs on invalid test runs
- No data to guide tool description improvements

## Proposed Solution

Implement a three-phase competition runner with mandatory validation:

### Phase 1: Setup (Sequential)
1. Validate database connection
2. Verify base branch is indexed
3. Create variant worktrees
4. Inject variant tool descriptions
5. **NEW:** Scan all worktrees for indexing
6. **NEW:** Validate all environments

### Phase 2: Validation (Per-Variant)
1. **NEW:** Check worktree exists
2. **NEW:** Check worktree indexed (chunk_count > 0)
3. **NEW:** Check MCP config valid
4. **NEW:** Check file permissions OK
5. **NEW:** Fail fast if any check fails

### Phase 3: Execution (Parallel)
1. Spawn agents (only if validation passed)
2. Collect results
3. Evaluate winner

**Key Principles:**
- Setup sequentially, execute in parallel
- Validate before running (fail fast)
- Don't force tool usage (test natural selection based on descriptions)
- Shared database (fast embedding reuse)

## Success Metrics

**Quantitative:**
- ✅ Validation catches 100% of setup failures
- ✅ 0% of competitions waste API credits on invalid setups
- ✅ At least 50% of agents use search tool (vs 0% currently)
- ✅ Setup time < 5 minutes for 12 variants

**Qualitative:**
- ✅ Error messages are actionable (user knows how to fix)
- ✅ Genetic optimizer runs complete successfully
- ✅ Competition reports show realistic scores (not all ~18%)

## Relevant Agents

| Agent | Role | Tasks |
|-------|------|-------|
| general-purpose | Implementation | Build validation module, scan orchestrator, enhance competition runner, add security controls |
| verify-ticket | Validation | End-to-end testing, error scenario testing |
| unit-test-runner | Testing | Execute unit tests, report results |
| commit-ticket | Commit | Create conventional commits after verification |

## Planning Documents

### Core Planning
- **[Analysis](./planning/analysis.md)** - Problem space, root cause analysis, existing solutions research
- **[Architecture](./planning/architecture.md)** - Solution design, component structure, data flow
- **[Plan](./planning/plan.md)** - Phase breakdown, timeline, dependencies, success metrics

### Supporting Documents
- **[Quality Strategy](./planning/quality-strategy.md)** - Testing approach, coverage targets, risk mitigation
- **[Security Review](./planning/security-review.md)** - Threat model, security controls, risk assessment

## Key Deliverables

### Phase 1: Core Infrastructure (Week 1, Days 1-4)
1. **Pre-Flight Validation Module** - Database, base branch, worktree, MCP config, permissions checks
2. **Scan Orchestration Module** - Sequential worktree scanning with progress tracking
3. **Enhanced Competition Runner** - Three-phase flow with mandatory validation
4. **Security Controls** - Path traversal protection, resource limits, command injection prevention

### Phase 2: Validation (Week 1, Days 4-5)
1. **Documentation Updates** - Add validation features to competition framework guide
2. **End-to-End Validation** - Run all 3 optimizer levels successfully
3. **Error Scenario Testing** - Verify all failure modes are caught

## Timeline

**Total Duration:** 5-7 days (29-35 hours)

**Week 1:**
- Days 1-2: Core infrastructure (validation module, scan orchestrator)
- Days 3-4: Integration (competition runner, security controls)
- Day 5: Validation (documentation, E2E testing, error scenarios)

## Dependencies

**External:**
- PostgreSQL running and accessible
- Base branch already indexed
- Anthropic API key configured
- Claude Code Agents SDK installed

**Critical Path:** Validation Module → Competition Runner → E2E Testing

## Architecture Highlights

### Validation Checks

```typescript
interface VariantValidation {
  variantId: string
  checks: {
    worktreeExists: CheckResult      // Directory created successfully
    worktreeScanned: CheckResult     // Indexed with chunks > 0
    mcpConfigValid: CheckResult      // .mcp.json has maproom server
    toolsAccessible: CheckResult     // MCP config well-formed
    filePermissions: CheckResult     // Read/write access OK
  }
  overall: 'pass' | 'fail'
  failureReason?: string
}
```

### Fail-Fast Strategy

```typescript
// Validate BEFORE spawning any agents
const validation = await validateCompetitionSetup(config)

if (!validation.valid) {
  // Log all errors with fixes
  validation.errors.forEach(err => {
    console.error(`❌ ${err.message}`)
    console.error(`   Fix: ${err.troubleshooting}`)
  })

  // Don't waste API credits
  throw new Error('Pre-flight validation failed')
}

// Only proceed if ALL validations passed
await runAgentsInParallel(validatedEnvironments)
```

### Security Controls

1. **Variant ID validation** - Prevent path traversal
2. **Resource limits** - Max variants, parallel agents, timeout
3. **Command injection protection** - Use spawn with args array
4. **Sensitive data sanitization** - Redact credentials in logs

## Quick Start

After implementation, running competitions will look like:

```bash
# The optimizer will now validate before running
pnpm tsx scripts/run-genetic-optimizer-ultra.ts

# Output will show validation phase:
# 📋 Phase 1: Setup
# ✅ Database connection verified
# ✅ Base branch indexed (1234 chunks)
# ✅ Created 12 variant worktrees
# 📊 Scanning worktrees...
# ✅ All worktrees scanned
#
# 🔍 Phase 2: Pre-Flight Validation
# ✅ variant-a-detailed: All checks passed
# ✅ variant-b-simple: All checks passed
# ... (all 12 variants)
#
# 🚀 Phase 3: Agent Execution
# ... (agents run with tools available)
```

## Risk Assessment

**Overall Risk:** LOW (2/10)

**Mitigated Risks:**
- Command injection → Use spawn with args array
- Path traversal → Validate variant IDs
- Resource exhaustion → Enforce limits
- Wasted API credits → Fail-fast validation

**Acceptable Risks:**
- Setup time overhead (+2-3 minutes for 12 variants)
- Sequential scanning (slower than parallel, but simpler)

## Contact

For questions or issues:
- Check planning documents in `planning/`
- Review existing tickets in `tickets/` (once created)
- Follow standard ticket workflow (implement → test → verify → commit)

---

**Status:** Planning Complete ✅
**Next Step:** Create project tickets
**Command:** `/create-project-tickets COMPFIX`

# COMPFIX Ticket Index

**Project:** Competition Agent Setup and Validation
**Status:** Tickets Created
**Total Tickets:** 7 (4 in Phase 1, 3 in Phase 2)

## Overview

Fix the AGENTOPT genetic optimizer competition framework by implementing comprehensive pre-flight validation, worktree scanning orchestration, and fail-fast error handling. Transform from 0% success rate (agents have no tools) to 100% valid agent environments.

## Phase 1: Core Validation Infrastructure (4 tickets)

Build the validation framework and integrate into competition runner.

### COMPFIX-1001: Pre-Flight Validation Module
**Agent:** general-purpose
**Est:** 4-6 hours
**Status:** Not Started

Create comprehensive validation module that checks database connectivity, base branch indexing, worktree scanning, MCP configuration, and file permissions before agent execution.

**Key Deliverables:**
- `PreFlightValidator` class with 7 validation methods
- Database connection testing with pg.Client
- Worktree scan status verification via maproom status
- MCP config structure validation
- File permission read/write testing
- Actionable error messages with troubleshooting
- 95%+ test coverage on critical paths

**Files:**
- `packages/cli/src/search-optimization/validation/pre-flight-validator.ts`
- `packages/cli/src/search-optimization/validation/pre-flight-validator.test.ts`
- `packages/cli/src/search-optimization/validation/types.ts`

### COMPFIX-1002: Scan Orchestration Module
**Agent:** general-purpose
**Est:** 3-4 hours
**Status:** Not Started

Create scan orchestration to ensure all variant worktrees are indexed before agent execution. Manages sequential scanning with progress tracking and fail-fast error handling.

**Key Deliverables:**
- `scanWorktree()` function for single worktree scanning
- `scanAllWorktrees()` function for batch scanning
- Command injection protection (spawn with args array)
- Output parsing for chunk count and duration
- Clear progress logging
- 95%+ test coverage

**Files:**
- `packages/cli/src/search-optimization/scan-orchestrator.ts`
- `packages/cli/src/search-optimization/scan-orchestrator.test.ts`

**Security:** HIGH priority - replaces execSync with spawn to prevent command injection

### COMPFIX-1003: Enhanced Competition Runner
**Agent:** general-purpose
**Est:** 6-8 hours
**Status:** Not Started

Transform competition runner to three-phase flow: Setup (sequential) → Validation (per-variant) → Execution (parallel). Integrates validation module and scan orchestrator.

**Key Deliverables:**
- Phase 1: Database check, base branch check, worktree creation, scanning
- Phase 2: Per-variant validation with fail-fast
- Phase 3: Parallel agent execution (existing behavior preserved)
- Enhanced console output showing all phases
- Competition reports include setup metrics
- Integration tests for happy path + 3 error scenarios

**Files:**
- `packages/cli/src/search-optimization/competition-runner.ts` (major changes)
- `packages/cli/src/search-optimization/competition-runner.integration.test.ts`
- `packages/cli/src/search-optimization/types.ts` (add setupMetrics)

**Depends on:** COMPFIX-1001, COMPFIX-1002

### COMPFIX-1004: Security Controls
**Agent:** general-purpose
**Est:** 2-3 hours
**Status:** Not Started

Add security validations and controls: variant ID validation (path traversal protection), resource limits, sensitive data sanitization, and command injection protection audit.

**Key Deliverables:**
- Variant ID validation (alphanumeric, dash, underscore only)
- Resource limits: MAX_VARIANTS=50, MAX_PARALLEL_AGENTS=10, MAX_TIMEOUT=600000
- Database URL sanitization for logs
- Replace all execSync with spawn + args array
- Unit tests for all security controls

**Files:**
- `packages/cli/src/search-optimization/security/validators.ts`
- `packages/cli/src/search-optimization/security/validators.test.ts`
- `packages/cli/src/search-optimization/security/limits.ts`
- `packages/cli/src/search-optimization/security/limits.test.ts`
- `packages/cli/src/search-optimization/security/sanitize.ts`
- `packages/cli/src/search-optimization/security/sanitize.test.ts`

**Priority:** HIGH - required before production deployment per security review

## Phase 2: Documentation and Validation (3 tickets)

Document the new validation features and verify end-to-end behavior.

### COMPFIX-2001: Update Documentation
**Agent:** general-purpose
**Est:** 2-3 hours
**Status:** Not Started

Update all competition framework documentation with validation phase, troubleshooting guides, and revised timing estimates.

**Key Deliverables:**
- Add "Pre-Flight Validation" section to competition framework guide
- Document all validation checks with troubleshooting
- Update timing estimates (+2-3 minutes for validation)
- Add validation workflow diagram
- Document security controls and resource limits
- Update genetic optimizer scripts with validation phase notes

**Files:**
- `docs/search-optimization/competition-framework.md` (major additions)
- `packages/cli/src/search-optimization/README.md` (add scan orchestration)
- `scripts/run-genetic-optimizer*.ts` (add comments)

**Depends on:** COMPFIX-1001, COMPFIX-1002, COMPFIX-1003, COMPFIX-1004

### COMPFIX-2002: End-to-End Validation
**Agent:** verify-ticket
**Est:** 2-3 hours
**Status:** Not Started

Execute all three optimizer configurations (standard 5 variants, premium 8 variants, ultra 12 variants) end-to-end to verify validation works with real agent execution.

**Key Validation Criteria:**
- All 3 optimizer levels complete successfully
- At least 50% of agents use mcp__maproom__search (vs 0% previously)
- No agents have 0 searches (confirms tool access)
- Setup time < 5 minutes for 12 variants
- Competition scores show meaningful variation (not all ~18%)

**Deliverables:**
- Run standard, premium, ultra optimizers
- Document timing metrics (setup, validation, execution)
- Document tool usage statistics
- Save results to validation-results/e2e-results.md

**Files:**
- `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`
- `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/optimizer-*.log`

**Depends on:** All Phase 1 tickets, COMPFIX-2001

**Note:** Manual execution ticket - requires Anthropic API credits

### COMPFIX-2003: Error Scenario Testing
**Agent:** verify-ticket
**Est:** 1-2 hours
**Status:** Not Started

Manually test all documented error scenarios to verify validation catches failures before agent execution and error messages are actionable.

**Error Scenarios:**
1. Database unreachable
2. Base branch not indexed
3. Worktree scan fails
4. MCP config malformed
5. Permission denied

**Validation Criteria for Each:**
- Error caught by validation (not during agent execution)
- Error message matches documentation
- Troubleshooting steps are actionable
- No API credits wasted (no Anthropic API calls)

**Deliverables:**
- Test all 5 error scenarios
- Document actual error messages vs expected
- Verify no API waste
- Save results to validation-results/error-scenarios.md

**Files:**
- `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenarios.md`
- `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenario-*.log`

**Depends on:** All Phase 1 tickets, COMPFIX-2001

**Note:** Manual testing ticket - simulates failures

## Timeline

**Total Duration:** 5-7 days (29-35 hours)

**Week 1:**
- Days 1-2: Core infrastructure (COMPFIX-1001, COMPFIX-1002)
- Days 3-4: Integration (COMPFIX-1003, COMPFIX-1004)
- Day 5: Validation (COMPFIX-2001, COMPFIX-2002, COMPFIX-2003)

## Dependencies

### Critical Path
```
COMPFIX-1001 (Validation) ──┬──> COMPFIX-1003 (Competition Runner)
COMPFIX-1002 (Scanning)   ──┘       │
                                    ├──> COMPFIX-2001 (Docs)
COMPFIX-1004 (Security)   ──────────┤
                                    └──> COMPFIX-2002 (E2E Test)
                                          └──> COMPFIX-2003 (Error Test)
```

### External Dependencies
- PostgreSQL running and accessible
- Base branch already indexed
- Anthropic API key configured
- Claude Code Agents SDK installed

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

## Quick Reference

### Ticket Execution Order

1. **COMPFIX-1001** (Validation Module) - Foundation for all other tickets
2. **COMPFIX-1002** (Scan Orchestrator) - Independent, can be parallel with 1001
3. **COMPFIX-1003** (Competition Runner) - Requires 1001 and 1002
4. **COMPFIX-1004** (Security Controls) - Can be parallel with 1003
5. **COMPFIX-2001** (Documentation) - Requires all Phase 1 tickets
6. **COMPFIX-2002** (E2E Validation) - Requires all previous tickets
7. **COMPFIX-2003** (Error Testing) - Requires all previous tickets

### Commands

**Create tickets:** Already done (this file exists)

**Work on project:**
```bash
/work-on-project COMPFIX
```

**Work on single ticket:**
```bash
/single-ticket COMPFIX-1001
```

**Review tickets:**
```bash
/review-tickets COMPFIX
```

## Related Documents

- **README.md** - Project overview and success metrics
- **planning/analysis.md** - Problem space and root cause analysis
- **planning/architecture.md** - Solution design and component structure
- **planning/plan.md** - Phase breakdown and timeline
- **planning/quality-strategy.md** - Testing approach and coverage targets
- **planning/security-review.md** - Threat model and security controls

---

**Status:** Ready for implementation
**Next Step:** Execute tickets in order (COMPFIX-1001 first)

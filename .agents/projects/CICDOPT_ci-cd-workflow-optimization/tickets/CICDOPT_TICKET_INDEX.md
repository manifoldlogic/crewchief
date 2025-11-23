# CICDOPT Ticket Index

**Project**: CI/CD Workflow Optimization
**Status**: Tickets Created
**Total Tickets**: 18

---

## Phase 1: Quick Wins (Week 1)

**Goal**: Fix critical issues, add caching, improve efficiency (40-50% faster builds)

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| CICDOPT-1001 | Fix package.json Build Script Circular Dependency | ⏳ Not Started | github-actions-specialist |
| CICDOPT-1002 | Add Rust Caching to Release Workflows | ⏳ Not Started | github-actions-specialist |
| CICDOPT-1003 | Add pnpm Store Caching to All Workflows | ⏳ Not Started | github-actions-specialist |
| CICDOPT-1004 | Add Path Filters to Test Workflow | ⏳ Not Started | github-actions-specialist |

**Expected Impact**: 40-50% faster builds, 80% fewer unnecessary test runs

---

## Phase 2: Reusable Infrastructure (Week 2)

**Goal**: Create shared workflow components to eliminate duplication

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| CICDOPT-2001 | Create Reusable Rust Build Workflow | ⏳ Not Started | rust-indexer-engineer, github-actions-specialist |
| CICDOPT-2002 | Create Reusable TypeScript Build Workflow | ⏳ Not Started | github-actions-specialist |
| CICDOPT-2003 | Add Comprehensive Workflow Documentation | ⏳ Not Started | github-actions-specialist |

**Expected Impact**: Foundation for zero duplication, single source of truth

---

## Phase 3: Workflow Consolidation (Weeks 2-3)

**Goal**: Integrate reusables, consolidate duplicate workflows (60-70% faster releases)

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| CICDOPT-3001 | Refactor CLI Workflow to Use Reusables | ⏳ Not Started | github-actions-specialist |
| CICDOPT-3002 | Create Unified Maproom-MCP Release Workflow | ⏳ Not Started | github-actions-specialist, docker-engineer |
| CICDOPT-3003 | Delete Old Workflows and Clean Up | ⏳ Not Started | github-actions-specialist |
| CICDOPT-3004 | Update Test Workflow with Optimizations | ⏳ Not Started | github-actions-specialist |

**Expected Impact**: 60-70% faster releases, 50% less code

---

## Phase 4: VSCode Extension Publishing (Future, Week 4+)

**Goal**: Prepare for multi-marketplace extension publishing

**Prerequisites**: CICDOPT-4000 MUST be completed before any other Phase 4 ticket

| Ticket ID | Title | Status | Agent | Dependencies |
|-----------|-------|--------|-------|--------------|
| CICDOPT-4000 | Setup Marketplace Accounts and PAT Tokens | ⏳ Not Started | vscode-extension-specialist | None |
| CICDOPT-4001 | Create VSCode Extension Build Workflow | ⏳ Not Started | vscode-extension-specialist, github-actions-specialist | CICDOPT-4000 |
| CICDOPT-4002 | Add Microsoft Marketplace Publishing | ⏳ Not Started | vscode-extension-specialist | CICDOPT-4000, CICDOPT-4001 |
| CICDOPT-4003 | Add Open VSX Publishing | ⏳ Not Started | vscode-extension-specialist | CICDOPT-4000, CICDOPT-4001 |
| CICDOPT-4004 | Add GitHub Release Creation | ⏳ Not Started | github-actions-specialist | CICDOPT-4000, CICDOPT-4001 |

**Expected Impact**: Ready for multi-marketplace extension publishing when needed

---

## Ticket Dependencies

```
Phase 1 (No dependencies):
  CICDOPT-1001 → CICDOPT-1002 → CICDOPT-1003 → CICDOPT-1004

Phase 2 (Depends on Phase 1 validation):
  CICDOPT-2001
  CICDOPT-2002
  CICDOPT-2003

Phase 3 (Depends on Phase 2 validation):
  CICDOPT-3001 (depends on CICDOPT-2001, CICDOPT-2002)
  CICDOPT-3002 (depends on CICDOPT-2001, CICDOPT-2002)
  CICDOPT-3003 (depends on CICDOPT-3001, CICDOPT-3002 validated)
  CICDOPT-3004

Phase 4 (Depends on vscode-maproom readiness):
  CICDOPT-4000 (prerequisite for all Phase 4)
  CICDOPT-4001 → CICDOPT-4002, CICDOPT-4003, CICDOPT-4004
```

---

## Execution Order

**Recommended sequence**:

1. **Week 1 - Phase 1**: Execute CICDOPT-1001 → 1002 → 1003 → 1004 sequentially
2. **Monitor for 5 days**: Validate metrics (cache hit rates, build times, path filters)
3. **Week 2 - Phase 2**: Execute CICDOPT-2001, 2002, 2003 (can be parallel)
4. **Week 2-3 - Phase 3**: Execute CICDOPT-3001 → 3002 → 3004 → 3003 (cleanup last)
5. **Week 4+ - Phase 4**: Execute CICDOPT-4000 first, then 4001 → 4002/4003/4004 (parallel)

---

## Success Metrics

### Phase 1
- ✅ Docker workflow unblocked
- ✅ 40-50% faster builds
- ✅ 80% fewer unnecessary test runs
- ✅ Cache hit rate >70%

### Phase 2
- ✅ Reusable workflows tested and validated
- ✅ Documentation complete
- ✅ Ready for integration

### Phase 3
- ✅ Single workflow per package
- ✅ Zero code duplication
- ✅ 60-70% faster releases
- ✅ All production releases successful

### Phase 4
- ✅ Extension publishes to 2 marketplaces
- ✅ Automated release creation
- ✅ Pre-release support working

---

## Plan Reference

For detailed ticket specifications, see:
- **Plan**: `.agents/projects/CICDOPT_ci-cd-workflow-optimization/planning/plan.md`
- **Architecture**: `.agents/projects/CICDOPT_ci-cd-workflow-optimization/planning/architecture.md`
- **Quality Strategy**: `.agents/projects/CICDOPT_ci-cd-workflow-optimization/planning/quality-strategy.md`
- **Review Updates**: `.agents/projects/CICDOPT_ci-cd-workflow-optimization/planning/review-updates.md`

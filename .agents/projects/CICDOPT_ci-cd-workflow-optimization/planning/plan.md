# Execution Plan: CI/CD Workflow Optimization

## Project Overview

**Goal**: Optimize GitHub Actions workflows for 60-70% faster releases, eliminate duplication, add comprehensive caching

**Timeline**: 3-4 weeks aggressive implementation

**Phases**: 3 core phases + 1 future phase (VSCode)

---

## Phase 1: Quick Wins (Week 1)

**Goal**: Fix critical issues, add caching, improve efficiency

**Duration**: 3-5 days

**Expected Impact**: 40-50% faster builds, 80% fewer unnecessary test runs

### Tickets

#### CICDOPT-1001: Fix package.json Build Script Circular Dependency

**Priority**: P0 - Blocks Docker workflow

**Description**: Fix circular dependency in root package.json build script that prevents TypeScript compilation in CI.

**Acceptance Criteria**:
- [ ] package.json line 11 changed from `node packages/cli/dist/cli/index.js build` to `pnpm -r --filter='./packages/*' build`
- [ ] Local `pnpm build` succeeds from fresh checkout
- [ ] All package dist/ directories created
- [ ] Docker workflow unblocked

**Files**:
- `package.json` (line 11)

**Agent**: github-actions-specialist

**Testing**:
```bash
# Fresh checkout test
rm -rf node_modules packages/*/dist packages/*/node_modules
pnpm install && pnpm build
```

---

#### CICDOPT-1002: Add Rust Caching to Release Workflows

**Priority**: P0 - High impact optimization

**Description**: Add Swatinem/rust-cache@v2 to CLI and Maproom-MCP release workflows for 50-70% faster builds.

**Acceptance Criteria**:
- [ ] `Swatinem/rust-cache@v2` added to both workflows
- [ ] Cache configuration: workspace=`crates/maproom -> target`, shared-key=`${{ matrix.target }}`
- [ ] First run creates cache
- [ ] Second run restores cache and builds 50-70% faster
- [ ] All 4 platforms build successfully

**Files**:
- `.github/workflows/build-and-publish-cli.yml`
- `.github/workflows/build-and-publish-maproom-mcp.yml`

**Agent**: github-actions-specialist

**Testing**:
- Trigger via workflow_dispatch with dry_run=true
- Monitor cache miss → cache hit behavior
- Measure build time improvement

---

#### CICDOPT-1003: Add pnpm Store Caching to All Workflows

**Priority**: P1 - Consistent improvement across all workflows

**Description**: Add pnpm store caching to all 4 workflows for 40-60% faster dependency installation.

**Acceptance Criteria**:
- [ ] pnpm store cache added to test.yml
- [ ] pnpm store cache added to build-and-publish-cli.yml
- [ ] pnpm store cache added to build-and-publish-maproom-mcp.yml
- [ ] pnpm store cache added to publish-maproom-mcp-image.yml
- [ ] Cache key based on `pnpm-lock.yaml` hash
- [ ] Restore keys include OS fallback
- [ ] First run creates cache, second run hits cache

**Files**:
- `.github/workflows/test.yml`
- `.github/workflows/build-and-publish-cli.yml`
- `.github/workflows/build-and-publish-maproom-mcp.yml`
- `.github/workflows/publish-maproom-mcp-image.yml`

**Agent**: github-actions-specialist

**Testing**:
- Verify cache created after first run
- Verify 40-60% faster install on cache hit

---

#### CICDOPT-1004: Add Path Filters to Test Workflow

**Priority**: P1 - Reduces unnecessary CI usage

**Description**: Add path-based filtering to test workflow to skip runs on non-code changes (docs, config, planning).

**Acceptance Criteria**:
- [ ] Path filter includes: `crates/**`, `packages/*/src/**`, `packages/*/tests/**`, `**.rs`, `**.ts`, `pnpm-lock.yaml`, `Cargo.lock`
- [ ] Path filter excludes: `docs/**`, `*.md`, `.agents/**`, `.github/workflows/**` (except test.yml), `.devcontainer/**`
- [ ] Test with docs-only PR (should skip)
- [ ] Test with code PR (should run)
- [ ] Test with mixed PR (should run)

**Files**:
- `.github/workflows/test.yml`

**Agent**: github-actions-specialist

**Testing**:
- Create 3 test PRs (docs, code, mixed)
- Verify correct trigger behavior

---

### Phase 1 Success Metrics

**Before**:
- Test workflow: 5-8 min, runs on every push
- Release workflows: 12-15 min each
- Docker workflow: Fails (circular dependency)

**After Phase 1**:
- Test workflow: 3-5 min, runs 20% as often (80% reduction)
- Release workflows: 6-8 min (50% faster)
- Docker workflow: 5-6 min (working + cached)

**Validation**:
- Run test workflow twice: verify cache hit
- Run release workflow twice: verify Rust cache hit
- Trigger test on docs PR: verify skip
- Monitor metrics for 1 week

---

## Phase 2: Reusable Infrastructure (Week 2)

**Goal**: Create shared workflow components to eliminate duplication

**Duration**: 5-7 days

**Expected Impact**: Reduce workflow code by 50%, single source of truth

### Tickets

#### CICDOPT-2001: Create Reusable Rust Build Workflow

**Priority**: P1 - Foundation for consolidation

**Description**: Extract Rust build logic into reusable workflow callable by CLI and Maproom-MCP workflows.

**Acceptance Criteria**:
- [ ] New file: `.github/workflows/reusable-rust-build.yml`
- [ ] Accepts inputs: `package_name`, `crate_path`, `binary_name`, `platforms`
- [ ] Matrix builds for all platforms
- [ ] Includes Rust caching from Phase 1
- [ ] Uploads artifacts with correct naming
- [ ] Outputs artifact prefix for caller
- [ ] Test caller workflow validates reusable works

**Files**:
- `.github/workflows/reusable-rust-build.yml` (new)
- `.github/workflows/test-reusable-rust.yml` (test caller, temporary)

**Agent**: rust-indexer-engineer, github-actions-specialist

**Testing**:
- Create test caller workflow
- Trigger with workflow_dispatch
- Verify all 4 platforms build
- Verify artifacts uploaded correctly

---

#### CICDOPT-2002: Create Reusable TypeScript Build Workflow

**Priority**: P1 - Complements Rust reusable

**Description**: Extract TypeScript build logic into reusable workflow for workspace package builds.

**Acceptance Criteria**:
- [ ] New file: `.github/workflows/reusable-typescript-build.yml`
- [ ] Accepts inputs: `workspace_filter`, `artifact_name`
- [ ] Includes pnpm caching from Phase 1
- [ ] Builds specified workspace packages
- [ ] Uploads dist/ artifacts (excludes node_modules)
- [ ] Test caller validates reusable works

**Files**:
- `.github/workflows/reusable-typescript-build.yml` (new)
- `.github/workflows/test-reusable-typescript.yml` (test caller, temporary)

**Agent**: github-actions-specialist

**Testing**:
- Test with `./packages/*` filter
- Test with specific package filter
- Verify artifacts contain only dist/

---

#### CICDOPT-2003: Add Comprehensive Documentation

**Priority**: P2 - Enables team understanding

**Description**: Document reusable workflow architecture, usage patterns, and troubleshooting.

**Acceptance Criteria**:
- [ ] New file: `.github/WORKFLOWS.md`
- [ ] Documents all workflows (purpose, triggers, inputs)
- [ ] Explains reusable workflow pattern
- [ ] Provides testing procedure
- [ ] Includes rollback plan
- [ ] Troubleshooting common issues

**Files**:
- `.github/WORKFLOWS.md` (new)

**Agent**: github-actions-specialist

---

### Phase 2 Success Metrics

**Deliverables**:
- 2 reusable workflows tested and validated
- Documentation complete
- Ready for Phase 3 integration

**Validation**:
- Both reusables callable via test workflows
- All platforms build successfully
- Artifacts match expected structure

---

## Phase 3: Workflow Consolidation (Weeks 2-3)

**Goal**: Integrate reusables, consolidate duplicate workflows

**Duration**: 7-10 days

**Expected Impact**: Single workflow per package, 60-70% faster releases

### Tickets

#### CICDOPT-3001: Refactor CLI Workflow to Use Reusables

**Priority**: P1 - Validates reusable pattern

**Description**: Update CLI release workflow to call reusable Rust and TypeScript workflows instead of duplicating logic.

**Acceptance Criteria**:
- [ ] build-rust job uses reusable-rust-build.yml
- [ ] build-typescript job uses reusable-typescript-build.yml
- [ ] publish job downloads artifacts from both
- [ ] Dry-run test succeeds
- [ ] Real release succeeds
- [ ] Workflow code reduced by 50%

**Files**:
- `.github/workflows/build-and-publish-cli.yml` (refactored, rename to `release-cli.yml`)
- `.github/workflows/build-and-publish-cli.yml.old` (backup)

**Agent**: github-actions-specialist

**Testing**:
- workflow_dispatch dry-run
- Real tag trigger (monitor closely)

---

#### CICDOPT-3002: Create Unified Maproom-MCP Release Workflow

**Priority**: P0 - Eliminates biggest redundancy

**Description**: Create single workflow that handles both npm and Docker publishing for maproom-mcp, triggered by one tag.

**Acceptance Criteria**:
- [ ] New file: `.github/workflows/release-maproom-mcp.yml`
- [ ] Calls reusable-rust-build once
- [ ] Calls reusable-typescript-build once
- [ ] publish-npm job uses Rust + TypeScript artifacts
- [ ] publish-docker job uses same artifacts (parallel with npm)
- [ ] Single tag triggers entire workflow
- [ ] Dry-run succeeds
- [ ] Real release succeeds (npm + Docker both work)

**Files**:
- `.github/workflows/release-maproom-mcp.yml` (new, unified)

**Agent**: github-actions-specialist, docker-engineer

**Testing**:
- workflow_dispatch dry-run with push_docker=false
- workflow_dispatch full test with test tag
- Real release monitoring

---

#### CICDOPT-3003: Delete Old Workflows and Clean Up

**Priority**: P2 - Cleanup after consolidation

**Description**: Archive old duplicate workflows after successful consolidation.

**Acceptance Criteria**:
- [ ] Unified maproom-mcp workflow validated in production
- [ ] Old workflows moved to `.old` extension
- [ ] Test caller workflows deleted
- [ ] Git commit preserves old workflows for rollback

**Files**:
- `.github/workflows/build-and-publish-maproom-mcp.yml` → `.old`
- `.github/workflows/publish-maproom-mcp-image.yml` → `.old`
- `.github/workflows/test-reusable-*.yml` (delete)

**Agent**: github-actions-specialist

**Dependencies**: CICDOPT-3001 and CICDOPT-3002 complete + validated

---

#### CICDOPT-3004: Update Test Workflow with Optimizations

**Priority**: P2 - Apply learnings to test workflow

**Description**: Add concurrency controls and final optimizations to test workflow.

**Acceptance Criteria**:
- [ ] Concurrency group added (cancel in-progress for PRs)
- [ ] Both Rust and pnpm caching enabled
- [ ] Path filters from Phase 1 working
- [ ] Tests run efficiently (<5 min with cache)

**Files**:
- `.github/workflows/test.yml`

**Agent**: github-actions-specialist

---

### Phase 3 Success Metrics

**Before**:
- 4 workflows, 600 lines YAML, 25-30 min for maproom-mcp release

**After Phase 3**:
- 3 package workflows + 2 reusables, 300 lines package YAML + 200 reusable
- 8-10 min for unified maproom-mcp release (67% faster)
- Zero duplication

**Validation**:
- CLI release: 6-8 min ✅
- Maproom-MCP release: 8-10 min ✅
- Test workflow: 3-5 min ✅
- All workflows use reusables ✅

---

## Phase 4: VSCode Extension Publishing (Future, Week 4+)

**Goal**: Prepare for multi-marketplace extension publishing

**Duration**: 5-7 days (when vscode-maproom is ready)

**Status**: On hold until extension code exists

### Tickets

#### CICDOPT-4001: Create VSCode Extension Build Workflow

**Priority**: P2 (future)

**Description**: Create workflow for building and packaging vscode-maproom extension.

**Acceptance Criteria**:
- [ ] Calls reusable-typescript-build for compilation
- [ ] Packages extension with vsce
- [ ] Runs extension smoke tests
- [ ] Uploads .vsix artifact

**Files**:
- `.github/workflows/release-vscode-maproom.yml` (new)

**Agent**: vscode-extension-specialist, github-actions-specialist

---

#### CICDOPT-4002: Add Microsoft Marketplace Publishing

**Priority**: P2 (future)

**Description**: Add job to publish extension to VS Code Marketplace.

**Acceptance Criteria**:
- [ ] Uses VSCE_PAT secret
- [ ] Publishes from .vsix artifact
- [ ] Supports pre-release flag
- [ ] Conditional on secret presence

**Files**:
- `.github/workflows/release-vscode-maproom.yml`

**Agent**: vscode-extension-specialist

**Dependencies**: VSCE_PAT secret configured

---

#### CICDOPT-4003: Add Open VSX Publishing

**Priority**: P2 (future)

**Description**: Add job to publish extension to Open VSX Registry (parallel with VS Code Marketplace).

**Acceptance Criteria**:
- [ ] Uses OVSX_PAT secret
- [ ] Publishes from .vsix artifact
- [ ] Runs in parallel with vscode publish
- [ ] Conditional on secret presence

**Files**:
- `.github/workflows/release-vscode-maproom.yml`

**Agent**: vscode-extension-specialist

**Dependencies**: OVSX_PAT secret configured

---

#### CICDOPT-4004: Add GitHub Release Creation

**Priority**: P2 (future)

**Description**: Automatically create GitHub release with extension .vsix attachment and changelog.

**Acceptance Criteria**:
- [ ] Creates release on successful publish
- [ ] Attaches .vsix file
- [ ] Generates changelog from commits
- [ ] Supports pre-release flag

**Files**:
- `.github/workflows/release-vscode-maproom.yml`

**Agent**: github-actions-specialist

---

### Phase 4 Notes

**Prerequisites**:
- vscode-maproom package exists
- Extension packaged with vsce
- Marketplace accounts created (Microsoft + Eclipse)
- PAT tokens generated and stored as secrets

**Timeline**: Start when extension development begins

---

## Dependencies and Blockers

### Cross-Phase Dependencies

**Phase 2 depends on Phase 1**:
- Caching logic proven before reusing
- Path filters validated before documenting

**Phase 3 depends on Phase 2**:
- Reusables tested before integrating
- Documentation complete for team reference

**Phase 4 depends on vscode-maproom**:
- Can't build workflow until code exists
- Design documented, ready to implement when needed

### External Dependencies

**GitHub Actions Limits**:
- 10GB cache limit per repo (currently <1GB used)
- 90-day artifact retention (configurable)
- No rate limits expected

**Secrets Required**:
- ✅ NPM_TOKEN (exists)
- ✅ DOCKERHUB_USERNAME (exists)
- ✅ DOCKERHUB_TOKEN (exists)
- 🔲 VSCE_PAT (Phase 4 only)
- 🔲 OVSX_PAT (Phase 4 only)

---

## Risk Management

### High Risks

#### Risk: Workflow Breaks Production Release

**Mitigation**:
- Test with workflow_dispatch before tag triggers
- Use dry-run mode for npm publish
- Keep `.old` backups of all workflows
- Monitor first real release closely
- Have rollback plan ready

**Rollback Procedure**:
```bash
# If new workflow fails:
mv .github/workflows/release-cli.yml.old .github/workflows/build-and-publish-cli.yml
git add .github/workflows/
git commit -m "rollback: revert to old workflow"
git push
# Trigger old workflow manually
```

#### Risk: Cache Corruption Causes Build Failures

**Mitigation**:
- Use `cache-on-failure: true` (don't save bad caches)
- Document cache clearing procedure
- Monitor cache hit rates

**Resolution**:
```bash
# Clear specific cache
gh cache delete <cache-key>

# Clear all caches (nuclear option)
gh api repos/:owner/:repo/actions/caches -X DELETE
```

### Medium Risks

#### Risk: Reusable Workflow API Change Breaks Callers

**Mitigation**:
- Version reusables if API changes needed
- Test all callers after reusable changes
- Document reusable interface clearly

#### Risk: Path Filter Too Restrictive

**Mitigation**:
- Include workflow file in paths (self-trigger)
- Test with various PR types
- Document path patterns

---

## Monitoring and Validation

### Key Metrics to Track

**Performance Metrics**:
- Workflow duration (before/after)
- Cache hit rate (target: 80%+)
- Test run frequency (target: 80% reduction)

**Quality Metrics**:
- Workflow failure rate (target: <5%)
- Time to rollback (if needed)
- Developer satisfaction (workflow UX)

**Cost Metrics**:
- CI minutes per release (target: 60% reduction)
- Cache storage used (budget: <2GB)

### Monitoring Tools

**GitHub Actions UI**:
- Workflow duration trends
- Job duration breakdown
- Cache hit rates
- Artifact sizes

**CLI Monitoring**:
```bash
# Check recent runs
gh run list --workflow=test.yml --limit 20

# Average duration
gh run list --workflow=test.yml --limit 20 --json durationMs \
  | jq '[.[].durationMs] | add / length / 60000'  # Convert to minutes

# Cache stats
gh cache list
```

### Validation Checklist (End of Each Phase)

**Phase 1**:
- [ ] All 4 workflows have caching enabled
- [ ] Cache hit rate >70% on second run
- [ ] Test workflow skips docs-only changes
- [ ] Docker workflow unblocked and working
- [ ] Build times 40-50% faster

**Phase 2**:
- [ ] Both reusable workflows tested and validated
- [ ] Documentation complete and reviewed
- [ ] Team understands reusable pattern
- [ ] Ready for integration

**Phase 3**:
- [ ] CLI workflow uses reusables (works in production)
- [ ] Unified maproom-mcp workflow (1 tag → npm + Docker)
- [ ] Old workflows archived (with backups)
- [ ] Total time for maproom-mcp release: 8-10 min
- [ ] Zero workflow duplication

**Phase 4** (when applicable):
- [ ] Extension builds and packages
- [ ] Published to both marketplaces
- [ ] GitHub release created automatically
- [ ] .vsix attachment works

---

## Communication Plan

### Stakeholder Updates

**Weekly Updates** (during implementation):
- Progress summary
- Metrics before/after
- Blockers and risks
- Next week's focus

**Milestone Announcements**:
- Phase 1 complete: "Workflows 40% faster, 80% fewer test runs"
- Phase 2 complete: "Reusable infrastructure ready"
- Phase 3 complete: "Workflows consolidated, 60% faster releases"

### Documentation Deliverables

**During Project**:
- `.github/WORKFLOWS.md` - Architecture and usage
- Ticket updates - Progress tracking
- This plan - Reference for execution

**Post-Project**:
- Retrospective (lessons learned)
- Updated CLAUDE.md (if needed)
- Knowledge sharing session

---

## Success Criteria

### Phase 1 Success

- ✅ Docker workflow unblocked
- ✅ All workflows have caching
- ✅ 40-50% faster builds
- ✅ 80% fewer unnecessary test runs

### Phase 2 Success

- ✅ Reusable workflows tested and validated
- ✅ Documentation complete
- ✅ Zero issues found in validation

### Phase 3 Success

- ✅ Single workflow per package
- ✅ Zero code duplication
- ✅ 60-70% faster releases
- ✅ All production releases successful

### Phase 4 Success (Future)

- ✅ Extension publishes to 2 marketplaces
- ✅ Automated release creation
- ✅ Pre-release support working

### Overall Project Success

**Primary Goals**:
- ✅ 60-70% reduction in release time
- ✅ 50% reduction in workflow code
- ✅ 80% reduction in unnecessary test runs
- ✅ Zero production incidents

**Secondary Goals**:
- ✅ Improved developer experience
- ✅ Clear documentation
- ✅ Future-ready for VSCode publishing
- ✅ Team understanding of architecture

---

## Timeline Summary

```
Week 1: Phase 1 - Quick Wins
├─ Day 1-2: CICDOPT-1001, CICDOPT-1002
├─ Day 3-4: CICDOPT-1003, CICDOPT-1004
└─ Day 5: Validation and monitoring

Week 2: Phase 2 - Reusable Infrastructure
├─ Day 1-3: CICDOPT-2001, CICDOPT-2002
├─ Day 4-5: Testing and validation
└─ Weekend: CICDOPT-2003 (documentation)

Week 3: Phase 3 - Consolidation
├─ Day 1-2: CICDOPT-3001 (CLI refactor)
├─ Day 3-5: CICDOPT-3002 (Maproom-MCP unified)
└─ Weekend: CICDOPT-3003, CICDOPT-3004 (cleanup)

Week 4+: Phase 4 - VSCode (Future)
└─ Starts when vscode-maproom is ready
```

**Total**: 3 weeks for Phases 1-3, Phase 4 on separate timeline

---

## Conclusion

This execution plan provides:
- ✅ Clear ticket structure (4 tickets/phase)
- ✅ Phased approach (quick wins → infrastructure → consolidation)
- ✅ Risk mitigation at every step
- ✅ Validation criteria per phase
- ✅ Rollback procedures
- ✅ Future-ready architecture (VSCode)

**Ready to execute**: All planning complete, tickets defined, agents assigned.

**Next step**: Create tickets via `/create-project-tickets CICDOPT`

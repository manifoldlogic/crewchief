# BINPKG Ticket Index

**Project**: Integrated Rust Binary Packaging for npm
**Project Slug**: BINPKG
**Status**: Ready for Implementation
**Total Tickets**: 18 (16 implementation + 2 test tickets)

---

## Overview

This index organizes all BINPKG work tickets by implementation phase, as outlined in the project plan. Each phase builds on the previous one to create a complete automated binary packaging and release system.

**Project Goal**: Integrate Rust binary building into the npm release process so that `pnpm release:x` reliably produces complete packages with all platform binaries.

**Timeline**: 3 days (with +1 day contingency)

---

## Phase 1: GitHub Actions Workflow (Priority 1)

**Objective**: Automate multi-platform binary builds in CI
**Duration**: 1 day
**Tickets**: 7 implementation + 1 test

### Implementation Tickets

#### BINPKG-1001: Create GitHub Actions workflow structure
- **File**: `/workspace/.agents/work-tickets/BINPKG-1001_github-actions-workflow-structure.md`
- **Agent**: general-purpose
- **Effort**: 1-2 hours
- **Dependencies**: None (first ticket)
- **Status**: Ready
- **Summary**: Set up foundational GitHub Actions workflow file with triggers (v*.*.* tags, workflow_dispatch), job structure (build-binaries matrix, validate-and-publish), and matrix strategy for 4 platforms.

#### BINPKG-1002: Implement Linux x64 binary build
- **File**: `/workspace/.agents/work-tickets/BINPKG-1002_linux-x64-binary-build.md`
- **Agent**: general-purpose
- **Effort**: 2 hours
- **Dependencies**: BINPKG-1001
- **Status**: Ready
- **Summary**: Implement complete build steps for linux-x64 (most common platform) using ubuntu-latest runner and cross tool. Critical for fixing v1.3.0 production failure.

#### BINPKG-1003: Implement Linux ARM64 binary build
- **File**: `/workspace/.agents/work-tickets/BINPKG-1003_linux-arm64-binary-build.md`
- **Agent**: general-purpose
- **Effort**: 1 hour
- **Dependencies**: BINPKG-1001, BINPKG-1002 (reference)
- **Status**: Ready
- **Summary**: Implement linux-arm64 build using cross-compilation from ubuntu-latest runner. Similar pattern to 1002.

#### BINPKG-1004: Implement macOS x64 binary build
- **File**: `/workspace/.agents/work-tickets/BINPKG-1004_macos-x64-binary-build.md`
- **Agent**: general-purpose
- **Effort**: 1 hour
- **Dependencies**: BINPKG-1001
- **Status**: Ready
- **Summary**: Implement darwin-x64 native build using macos-13 runner (Intel Mac).

#### BINPKG-1005: Implement macOS ARM64 binary build
- **File**: `/workspace/.agents/work-tickets/BINPKG-1005_macos-arm64-binary-build.md`
- **Agent**: general-purpose
- **Effort**: 1 hour
- **Dependencies**: BINPKG-1001, BINPKG-1004 (reference)
- **Status**: Ready
- **Summary**: Implement darwin-arm64 native build using macos-latest runner (Apple Silicon).

#### BINPKG-1006: Implement validation job
- **File**: `/workspace/.agents/work-tickets/BINPKG-1006_validate-binary-artifacts.md`
- **Agent**: general-purpose
- **Effort**: 2-3 hours
- **Dependencies**: BINPKG-1002, BINPKG-1003, BINPKG-1004, BINPKG-1005
- **Status**: Ready
- **Summary**: Create validation job that downloads artifacts, verifies all 4 binaries exist and work, tests executability. Critical quality gate.

#### BINPKG-1007: Implement npm publish job
- **File**: `/workspace/.agents/work-tickets/BINPKG-1007_npm-publish-with-verification.md`
- **Agent**: general-purpose
- **Effort**: 2 hours
- **Dependencies**: BINPKG-1006, NPM_TOKEN secret configured
- **Status**: Ready
- **Summary**: Implement publish steps: tarball creation/verification, npm publish, post-publish verification. Respects dry_run input.

### Test Tickets

#### BINPKG-1901: Test GitHub Actions workflow (canary release)
- **File**: `/workspace/.agents/work-tickets/BINPKG-1901_canary-release-integration-test.md`
- **Agent**: test-runner
- **Effort**: 3-4 hours (includes fixing issues)
- **Dependencies**: ALL Phase 1 implementation tickets (1001-1007)
- **Status**: Blocked (waiting on implementation)
- **Summary**: End-to-end integration test with canary release (v1.3.1-canary.1). Verifies complete pipeline: tag push → builds → validation → publish → install testing.

---

## Phase 2: Local Validation Scripts (Priority 1)

**Objective**: Prevent publishing packages without binaries
**Duration**: 0.5 days
**Tickets**: 2 implementation + 1 test

### Implementation Tickets

#### BINPKG-2001: Create local binary validation script
- **File**: `/workspace/.agents/work-tickets/BINPKG-2001_local-binary-validation-script.md`
- **Agent**: general-purpose
- **Effort**: 1-2 hours
- **Dependencies**: None
- **Status**: Ready
- **Summary**: Create `scripts/validate-binaries.js` that checks all 4 platforms exist with reasonable sizes. Provides clear error messages and guidance.

#### BINPKG-2002: Add prepublishOnly hook
- **File**: `/workspace/.agents/work-tickets/BINPKG-2002_prepublish-hook-package-files.md`
- **Agent**: general-purpose
- **Effort**: 0.5 hours
- **Dependencies**: BINPKG-2001
- **Status**: Ready
- **Summary**: Update `packages/maproom-mcp/package.json` to run validation before publish. Simplify files array to `"bin"`.

### Test Tickets

#### BINPKG-2901: Test local validation script
- **File**: `/workspace/.agents/work-tickets/BINPKG-2901_test-local-validation-script.md`
- **Agent**: test-runner
- **Effort**: 1-2 hours
- **Dependencies**: BINPKG-2001, BINPKG-2002
- **Status**: Blocked (waiting on implementation)
- **Summary**: Test validation catches missing platforms, corrupted binaries, and allows complete packages. Test prepublishOnly hook integration.

---

## Phase 3: Release Script Integration (Priority 1)

**Objective**: Make `pnpm release:x` trigger full CI pipeline
**Duration**: 0.5 days
**Tickets**: 2 implementation

### Implementation Tickets

#### BINPKG-3001: Create automated release script
- **File**: `/workspace/.agents/work-tickets/BINPKG-3001_automated-release-script.md`
- **Agent**: general-purpose
- **Effort**: 2-3 hours
- **Dependencies**: None (replaces bump-version.js)
- **Status**: Ready
- **Summary**: Create `scripts/release.js` that automates: validate preconditions → bump version → commit → tag → push (triggers CI). Supports --dry-run.

#### BINPKG-3002: Update package.json release scripts
- **File**: `/workspace/.agents/work-tickets/BINPKG-3002_update-release-scripts.md`
- **Agent**: general-purpose
- **Effort**: 0.5 hours
- **Dependencies**: BINPKG-3001
- **Status**: Blocked (waiting on 3001)
- **Summary**: Update `release:patch/minor/major` scripts to call `release.js` instead of `bump-version.js` + direct publish.

---

## Phase 4: Documentation (Priority 2)

**Objective**: Document new release process
**Duration**: 0.5 days
**Tickets**: 1 implementation

### Implementation Tickets

#### BINPKG-4001: Document release process and binary packaging
- **File**: `/workspace/.agents/work-tickets/BINPKG-4001_document-release-process.md`
- **Agent**: general-purpose
- **Effort**: 2-3 hours
- **Dependencies**: BINPKG-3002
- **Status**: Blocked (waiting on implementation)
- **Summary**: Update documentation to explain new release process, binary packaging approach, troubleshooting, and emergency procedures. Create RELEASING.md or update CONTRIBUTING.md.

---

## Phase 5: Testing & Rollout (Priority 1)

**Objective**: Verify system works end-to-end
**Duration**: 0.5 days
**Tickets**: 2 test/validation

### Test & Rollout Tickets

#### BINPKG-5001: Execute dry-run release test
- **File**: `/workspace/.agents/work-tickets/BINPKG-5001_dry-run-release-test.md`
- **Agent**: test-runner
- **Effort**: 1-2 hours
- **Dependencies**: BINPKG-3001, BINPKG-3002, BINPKG-2001
- **Status**: Blocked (waiting on implementation)
- **Summary**: Test release script in dry-run mode to verify no actual changes made. Test validation failures work correctly.

#### BINPKG-5002: Execute first production release
- **File**: `/workspace/.agents/work-tickets/BINPKG-5002_execute-first-production-release.md`
- **Agent**: general-purpose
- **Effort**: 2-3 hours + 24-hour monitoring
- **Dependencies**: ALL previous tickets (especially 1901, 5001)
- **Status**: Blocked (final ticket)
- **Summary**: Execute first production release with `pnpm release:minor`. Monitor for 24 hours. Mark BINPKG project COMPLETE.

---

## Ticket Dependencies Diagram

```
Phase 1 (CI/CD):
  1001 (workflow structure)
    ├─→ 1002 (linux-x64)  ─┐
    ├─→ 1003 (linux-arm64) ─┤
    ├─→ 1004 (darwin-x64)  ─┼─→ 1006 (validation) ─→ 1007 (publish) ─→ 1901 (test)
    └─→ 1005 (darwin-arm64)─┘

Phase 2 (Local Safety):
  2001 (validation script) ─→ 2002 (prepublish hook) ─→ 2901 (test)

Phase 3 (Automation):
  3001 (release script) ─→ 3002 (package.json)

Phase 4 (Documentation):
  3002 ─→ 4001 (docs)

Phase 5 (Rollout):
  [3001, 3002, 2001] ─→ 5001 (dry-run)
  [1901, 5001, 4001] ─→ 5002 (production) ✓ PROJECT COMPLETE
```

---

## Ticket Execution Order

**Recommended Sequential Order** (respects dependencies):

### Week 1, Day 1 (Phase 1 - GitHub Actions)
1. BINPKG-1001 (workflow structure) - 1-2 hours
2. BINPKG-1002, 1003, 1004, 1005 (parallel builds) - 5 hours
3. BINPKG-1006 (validation) - 2-3 hours
4. BINPKG-1007 (publish) - 2 hours

### Week 1, Day 2 (Phase 2 & 3 - Safety & Automation)
5. BINPKG-2001 (validation script) - 1-2 hours
6. BINPKG-2002 (prepublish hook) - 0.5 hours
7. BINPKG-2901 (test validation) - 1-2 hours
8. BINPKG-3001 (release script) - 2-3 hours
9. BINPKG-3002 (package scripts) - 0.5 hours

### Week 1, Day 3 (Phase 4 & 5 - Documentation & Testing)
10. BINPKG-4001 (documentation) - 2-3 hours
11. BINPKG-5001 (dry-run test) - 1-2 hours
12. BINPKG-1901 (canary release) - 3-4 hours
13. BINPKG-5002 (production release) - 2-3 hours + monitoring

**Total**: ~27-35 hours (3-4 days with contingency)

---

## Success Metrics

### Primary Metrics
1. **Binary Completeness**: 100% of releases include all 4 binaries ✓
2. **Build Success Rate**: >95% of CI builds succeed ✓
3. **Developer Experience**: "Releasing is easy and reliable" ✓

### Secondary Metrics
1. **Build Time**: <15 minutes per release ✓
2. **Package Size**: <100MB (target ~50MB) ✓
3. **Installation Success**: >99% of installs work ✓

---

## Completion Criteria

Project is complete when:
- ✅ GitHub Actions workflow builds all 4 platforms (BINPKG-1001-1007)
- ✅ Validation script blocks incomplete publishes (BINPKG-2001-2002)
- ✅ `pnpm release:x` triggers full pipeline (BINPKG-3001-3002)
- ✅ Documentation updated (BINPKG-4001)
- ✅ At least one successful production release (BINPKG-5002)
- ✅ No critical issues reported (24-hour monitoring)

---

## Planning References

- **Project README**: `/workspace/.agents/projects/BINPKG_binary-packaging/README.md`
- **Implementation Plan**: `/workspace/.agents/projects/BINPKG_binary-packaging/planning/plan.md`
- **Architecture**: `/workspace/.agents/projects/BINPKG_binary-packaging/planning/architecture.md`
- **Quality Strategy**: `/workspace/.agents/projects/BINPKG_binary-packaging/planning/quality-strategy.md`
- **Security Review**: `/workspace/.agents/projects/BINPKG_binary-packaging/planning/security-review.md`
- **Analysis**: `/workspace/.agents/projects/BINPKG_binary-packaging/planning/analysis.md`

---

## Status Legend

- **Ready**: All dependencies met, can start immediately
- **Blocked**: Waiting on prerequisite tickets
- **In Progress**: Currently being worked on
- **Complete**: Implementation finished and verified
- **Verified**: Acceptance criteria validated

---

**Last Updated**: 2025-11-03
**Next Action**: Begin Phase 1 with BINPKG-1001 (workflow structure)

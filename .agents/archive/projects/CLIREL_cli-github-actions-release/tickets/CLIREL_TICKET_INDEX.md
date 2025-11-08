# CLIREL Ticket Index

**Project**: CLI GitHub Actions Release Automation
**Status**: Ready for execution
**Total Tickets**: 9
**Timeline**: 3-4 days (realistic estimate)

## Overview

This project migrates the `@crewchief/cli` package from manual local releases to automated GitHub Actions releases with multi-platform binary builds. The work is organized into 9 sequential phases, each with one ticket.

## Ticket Organization by Phase

### Phase 1: Old Package Deprecation

**CLIREL-1001**: Deprecate old crewchief package with migration warnings
- **Agent**: general-purpose
- **Duration**: 1 hour
- **Dependencies**: None
- **File**: `CLIREL-1001_deprecate-old-crewchief-package.md`
- **Summary**: Publish final `crewchief@1.0.0` with deprecation warnings directing users to `@crewchief/cli`

---

### Phase 2: CLI Package Configuration

**CLIREL-2001**: Reconfigure CLI package for @crewchief/cli scoped name
- **Agent**: general-purpose
- **Duration**: 2 hours
- **Dependencies**: None (can start immediately)
- **File**: `CLIREL-2001_reconfigure-cli-package-scoped-name.md`
- **Summary**: Update package.json, create .npmignore, update README for new scoped package name

---

### Phase 3: Release Script Updates

**CLIREL-3001**: Update release scripts and fix race condition
- **Agent**: general-purpose
- **Duration**: 2 hours
- **Dependencies**: CLIREL-2001 (package name must be `@crewchief/cli` first)
- **File**: `CLIREL-3001_update-release-scripts-fix-race-condition.md`
- **Summary**: Update CLI and MCP release scripts to use package-scoped tags and implement two-step push (commits first, then tags) to fix GitHub Actions race condition

---

### Phase 4: CLI GitHub Actions Workflow

**CLIREL-4001**: Create CLI GitHub Actions workflow
- **Agent**: docker-engineer (CI/CD and multi-platform build expertise)
- **Duration**: 6-10 hours (most complex ticket)
- **Dependencies**: CLIREL-2001 (package config), CLIREL-3001 (tag format)
- **File**: `CLIREL-4001_create-cli-github-actions-workflow.md`
- **Summary**: Create automated workflow for multi-platform builds (4 platforms), TypeScript compilation, binary validation, and npm publishing

**Key Components**:
- Matrix builds: linux-x64, linux-arm64, darwin-x64, darwin-arm64
- Cross-compilation for Linux ARM
- Binary validation (existence, size, execution)
- Package structure validation
- npm publish with NPM_TOKEN
- Dry-run support

---

### Phase 5: MCP Workflow Migration

**CLIREL-5001**: Update MCP workflow for package-scoped tags
- **Agent**: general-purpose
- **Duration**: 1 hour
- **Dependencies**: CLIREL-4001 (need both workflows to test isolation)
- **File**: `CLIREL-5001_update-mcp-workflow-package-scoped-tags.md`
- **Summary**: Update MCP workflow trigger from `v*.*.*` to `@crewchief/maproom-mcp@v*.*.*` to prevent cross-triggering with CLI workflow

**Key Testing**: Tag isolation verification (CLI tags don't trigger MCP workflow and vice versa)

---

### Phase 6: Security Hardening

**CLIREL-6001**: Implement security baseline
- **Agent**: general-purpose
- **Duration**: 2 hours
- **Dependencies**: None (can run in parallel with earlier phases)
- **File**: `CLIREL-6001_implement-security-baseline.md`
- **Summary**: Configure tag protection, branch protection, NPM_TOKEN secret, create SECURITY.md, and establish CODEOWNERS for workflow files

**Security Components**:
- Tag protection (maintainers only)
- Branch protection (require reviews)
- NPM_TOKEN as GitHub secret
- Vulnerability reporting process
- Workflow modification controls

---

### Phase 7: Dry-Run Validation

**CLIREL-7001**: Execute dry-run validation
- **Agent**: general-purpose
- **Duration**: 2-4 hours (includes testing and iteration)
- **Dependencies**: CLIREL-4001 (workflow), CLIREL-6001 (security)
- **File**: `CLIREL-7001_execute-dry-run-validation.md`
- **Summary**: Test complete automation end-to-end with test tag, verify all 4 platforms build successfully, validate package structure, confirm publish step is skipped

**Critical Checkpoint**: Must pass before production release

**Validation Report Includes**:
- All 4 binary builds succeed
- Binary sizes correct (5-20MB)
- TypeScript build complete
- Package structure validated
- No actual npm publish (dry_run=true)

---

### Phase 8: Production Release

**CLIREL-8001**: Execute production release
- **Agent**: general-purpose
- **Duration**: 1-2 hours (includes monitoring)
- **Dependencies**: CLIREL-7001 (dry-run must succeed)
- **File**: `CLIREL-8001_execute-production-release.md`
- **Summary**: Create `@crewchief/cli@v1.0.0` tag, trigger workflow, publish to npm, validate post-release, setup monitoring

**This is irreversible** - first real production release

**Post-Release Validation**:
- Package appears on npm
- Installation succeeds
- CLI executes on multiple platforms
- All 4 binaries present

---

### Phase 9: Documentation and Knowledge Transfer

**CLIREL-9001**: Update documentation and archive project
- **Agent**: general-purpose
- **Duration**: 2 hours
- **Dependencies**: CLIREL-8001 (production release complete)
- **File**: `CLIREL-9001_update-documentation-archive-project.md`
- **Summary**: Update README with new package name, create migration guide, document release process, create troubleshooting guide, archive project

**Deliverables**:
- README.md updated
- MIGRATION.md created
- RELEASE.md created (release process docs)
- Project archived to `.agents/archive/`
- ARCHIVE_README.md with project summary

---

## Execution Strategy

### Sequential Workflow (Critical Path)
```
CLIREL-2001 (Package Config)
    ↓
CLIREL-3001 (Release Scripts)
    ↓
CLIREL-4001 (CLI Workflow) ← Most complex, allow extra time
    ↓
CLIREL-5001 (MCP Workflow)
    ↓
CLIREL-7001 (Dry-Run) ← Critical validation checkpoint
    ↓
CLIREL-8001 (Production) ← Point of no return
    ↓
CLIREL-9001 (Documentation)
```

### Parallel Opportunities
- CLIREL-1001 (Deprecation) can run anytime
- CLIREL-6001 (Security) can run in parallel with 2001-4001

### Quality Gates
- **After CLIREL-4001**: Workflow must be syntactically valid (yamllint)
- **After CLIREL-7001**: Dry-run must succeed 100% before proceeding
- **Before CLIREL-8001**: Manual review and approval checkpoint

## Timeline Estimates

### Optimistic (18 hours)
- Phase 1: 1h
- Phase 2: 2h
- Phase 3: 1h
- Phase 4: 6h
- Phase 5: 1h
- Phase 6: 2h
- Phase 7: 2h
- Phase 8: 1h
- Phase 9: 2h

### Realistic (25 hours / 3-4 days)
- Add 4h debugging Phase 4 (workflow complexity)
- Add 2h iteration Phase 7 (dry-run fixes)
- Add 1h validation Phase 8 (post-release)

### Pessimistic (32 hours / 4-5 days)
- Add 8h Phase 4 (complex debugging)
- Add 4h Phase 7 (multiple iterations)
- Add 2h Phase 8 (rollback and retry)

## Agent Assignments

| Phase | Ticket ID | Primary Agent | Rationale |
|-------|-----------|--------------|-----------|
| 1 | CLIREL-1001 | general-purpose | Simple npm commands |
| 2 | CLIREL-2001 | general-purpose | Config file updates |
| 3 | CLIREL-3001 | general-purpose | Script modifications |
| 4 | CLIREL-4001 | docker-engineer | CI/CD and multi-platform expertise |
| 5 | CLIREL-5001 | general-purpose | Simple trigger update |
| 6 | CLIREL-6001 | general-purpose | Repository settings and docs |
| 7 | CLIREL-7001 | general-purpose | Testing and validation |
| 8 | CLIREL-8001 | general-purpose | Release execution |
| 9 | CLIREL-9001 | general-purpose | Documentation |

**Note**: docker-engineer is used for CLIREL-4001 because it has specialized expertise in CI/CD workflows, Docker/container builds (similar patterns to matrix builds), and multi-platform binary distribution.

## Risk Matrix

| Ticket | Risk Level | Key Risks | Mitigation |
|--------|-----------|-----------|------------|
| 1001 | Low | None significant | One-time manual operation |
| 2001 | Low | Missing files in tarball | Validate with npm pack |
| 3001 | Low | Race condition persists | Test with multiple releases |
| 4001 | **High** | Wrong binaries, build failures | Copy proven MCP pattern, dry-run |
| 5001 | Low | Cross-triggering | Comprehensive isolation testing |
| 6001 | Low | Config errors | Standard GitHub features |
| 7001 | Medium | Issues found | This is the point - fix before prod |
| 8001 | **High** | Broken production release | Dry-run must pass, hotfix plan |
| 9001 | Low | None | Documentation only |

## Success Metrics

### Functional
- ✅ `@crewchief/cli@1.0.0` published to npm
- ✅ Package contains all 4 platform binaries
- ✅ Independent tagging works (no cross-triggering)
- ✅ Old `crewchief` package deprecated

### Quality
- ✅ Zero manual steps after tag creation
- ✅ Validation prevents broken releases
- ✅ Workflow completes in <15 minutes
- ✅ Dry-run testing works

### Security
- ✅ NPM_TOKEN stored as secret
- ✅ Tag protection enabled
- ✅ Security baseline documented
- ✅ Incident response plan exists

### Process
- ✅ Release process documented
- ✅ Troubleshooting guide created
- ✅ Knowledge transferred to team

## Project Completion Criteria

Project is complete when:
- [x] All 9 tickets completed successfully
- [x] `@crewchief/cli@1.0.0` published and functional
- [x] Old `crewchief` package deprecated
- [x] Independent workflows operational
- [x] Security baseline implemented
- [x] Documentation complete
- [x] Project archived

## Getting Started

To begin executing this project:

```bash
# For single tickets
/single-ticket CLIREL-2001

# For entire project
/work-on-project CLIREL
```

**Recommended approach**: Execute tickets in sequential order (2001 → 3001 → 4001 → etc.) to maintain proper dependencies.

## Planning References

- **Project README**: `/workspace/.agents/projects/CLIREL_cli-github-actions-release/README.md`
- **Detailed Plan**: `/workspace/.agents/projects/CLIREL_cli-github-actions-release/planning/plan.md`
- **Architecture**: `/workspace/.agents/projects/CLIREL_cli-github-actions-release/planning/architecture.md`
- **Quality Strategy**: `/workspace/.agents/projects/CLIREL_cli-github-actions-release/planning/quality-strategy.md`
- **Security Review**: `/workspace/.agents/projects/CLIREL_cli-github-actions-release/planning/security-review.md`
- **Analysis**: `/workspace/.agents/projects/CLIREL_cli-github-actions-release/planning/analysis.md`

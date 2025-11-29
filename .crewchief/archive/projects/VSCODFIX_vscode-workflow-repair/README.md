# VSCODFIX: VSCode Workflow Repair

## Project Summary

**Goal**: Replace the failing VSCode extension release workflow with a robust, production-ready implementation that passes GitHub Actions validation and successfully publishes to both marketplaces.

**Status**: Planning Complete, Ready for Implementation
**Timeline**: 1-2 days
**Priority**: High (blocking VSCode extension releases)

## Problem Statement

The `release-vscode-maproom.yml` workflow created in CICDOPT project fails GitHub Actions validation on every push to main with "workflow file issue" errors. Despite passing local YAML validation, the workflow cannot be executed due to:

1. Job-level secret existence checks causing validation failures
2. Complex conditional logic not supported by GitHub Actions
3. Tag pattern with special characters (`@crewchief/vscode-maproom@v*`)
4. Multi-job dependencies with `always()` and result-based conditionals

**Impact**: Cannot publish VSCode extension to marketplaces via automated workflow.

## Proposed Solution

Redesign workflow with validation-friendly architecture:

**Key Changes**:
1. **workflow_dispatch trigger only** (no tag patterns)
2. **Step-level secret checks** (not job-level)
3. **continue-on-error pattern** for marketplace publishes
4. **Outcome-based conditionals** (not result-based)
5. **Simplified job structure** (linear flow)

**Benefits**:
- ✅ Passes GitHub Actions validation
- ✅ Testable with dry-run mode
- ✅ Handles partial failures gracefully
- ✅ Clear debugging path
- ✅ Production-ready

## Architecture

```
build-extension (reusable TypeScript workflow)
    ↓
package-extension (create .vsix + smoke tests)
    ↓
publish-extension (if not dry-run)
    ├─ VS Code Marketplace (step-level, continue-on-error)
    ├─ Open VSX Registry (step-level, continue-on-error)
    └─ GitHub Release (if any publish succeeded)
```

**Trigger**: Manual via `workflow_dispatch` with inputs:
- `version`: Version to release (must match package.json)
- `pre_release`: Mark as pre-release (boolean)
- `dry_run`: Build/package only, skip publishing (boolean)

## Planning Documents

- **[analysis.md](planning/analysis.md)**: Deep dive into failure causes and industry solutions
- **[architecture.md](planning/architecture.md)**: Detailed workflow design and decisions
- **[quality-strategy.md](planning/quality-strategy.md)**: Testing approach and validation layers
- **[security-review.md](planning/security-review.md)**: Security analysis and risk mitigation
- **[plan.md](planning/plan.md)**: Phase-by-phase execution plan with tasks

## Relevant Agents

### Primary
- **github-actions-specialist**: Workflow implementation
- **vscode-extension-specialist**: Extension-specific requirements

### Supporting
- **unit-test-runner**: Workflow testing and validation
- **verify-ticket**: Acceptance criteria verification (if ticketed)
- **commit-ticket**: Conventional commits (if ticketed)

## Execution Phases

### Phase 1: Implementation (Day 1)
- Create workflow file with robust structure
- Local YAML validation
- Push and verify GitHub validation

### Phase 2: Testing (Day 1-2)
- Dry-run test (build + package only)
- Download and inspect .vsix
- Optional pre-release staging test

### Phase 3: Documentation (Day 2)
- Update VSCODE_PUBLISHING.md
- Create runbook
- Document troubleshooting

### Phase 4: Production (Day 3+)
- Release v0.1.0 to production
- Post-release validation
- Monitor for issues

## Success Criteria

**Technical**:
1. Workflow passes GitHub Actions validation on every push
2. Dry-run creates valid .vsix package
3. Publishing to VS Code Marketplace succeeds
4. Publishing to Open VSX Registry succeeds
5. GitHub release created with .vsix attachment
6. Graceful handling of missing secrets

**Process**:
1. Clear documentation for future releases
2. Repeatable release process
3. Runbook for troubleshooting

## Quick Start

```bash
# After workflow is implemented:

# Test with dry-run
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0 \
  --field dry_run=true

# Monitor
gh run watch

# Production release (when ready)
gh workflow run release-vscode-maproom.yml \
  --field version=0.1.0 \
  --field dry_run=false
```

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Validation fails again | Local YAML validation, simpler structure |
| Secrets don't work | Step-level checks with env variables |
| Marketplace API issues | continue-on-error, partial success handling |
| Wrong version published | Version verification step |
| Extension broken | Comprehensive smoke tests |

## Related Work

- **CICDOPT Project**: Original CI/CD optimization that created initial workflow
- **CICDOPT-4000 through 4004**: VSCode publishing tickets
- **VSCODE_PUBLISHING.md**: Marketplace account documentation

## Timeline

**Estimated**: 1-2 days
- Implementation: 2-4 hours
- Testing: 4-8 hours
- Documentation: 2-3 hours
- Production release: 1 hour + monitoring

## Project Structure

```
.crewchief/projects/VSCODFIX_vscode-workflow-repair/
├── README.md (this file)
├── planning/
│   ├── analysis.md
│   ├── architecture.md
│   ├── quality-strategy.md
│   ├── security-review.md
│   └── plan.md
└── tickets/ (if needed)
```

## Next Steps

1. Review planning documents
2. Implement workflow file
3. Test with dry-run
4. Deploy to production

**Ready to proceed**: All planning complete, architecture validated, execution plan detailed.

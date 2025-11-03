# BINPKG: Integrated Rust Binary Packaging for npm

**Project Slug**: `BINPKG`
**Status**: Planning Complete → Ready for Ticket Generation
**Priority**: High (Production Issue)

## Problem Statement

The current release process for `@crewchief/maproom-mcp` is unreliable and has caused production failures:

1. **Manual Binary Build**: Developers must manually run `scripts/build-and-package.sh` before releasing
2. **Platform-Specific Builds**: The build script only builds for the current platform, missing other architectures
3. **Easy to Forget**: Running `pnpm release:patch` doesn't build binaries, resulting in incomplete npm packages
4. **Recent Production Incident**: Version 1.3.0 was published with only `linux-arm64` and `darwin-arm64` binaries, missing `linux-x64` (the most common Docker/devcontainer architecture)
5. **Silent Failure**: No validation prevents publishing packages without required binaries

**Impact**: Users on linux-x64 systems (most Docker containers, devcontainers, CI/CD) couldn't run `npx @crewchief/maproom-mcp setup` because the binary was missing, resulting in "Database migration failed: null" errors.

## Solution Overview

Integrate Rust binary building into the npm release process using a **Fat Package + GitHub Actions Matrix Build** approach:

### Architecture

```
Developer runs: pnpm release:patch
    ↓
1. Bump version in package.json
2. Commit version bump
3. Create git tag (v1.x.x)
4. Push tag to GitHub
    ↓
GitHub Actions: build-and-publish-maproom-mcp
    ↓
1. Matrix build: 4 platforms in parallel
   - linux-x64 (ubuntu-latest + cross)
   - linux-arm64 (ubuntu-latest + cross)
   - darwin-x64 (macos-13)
   - darwin-arm64 (macos-latest)
    ↓
2. Download all artifacts
    ↓
3. Validate: Check all 4 binaries exist
    ↓
4. npm publish
    ↓
5. Verify: Test install on multiple platforms
```

### Key Features

1. **Automated Multi-Platform Builds**: GitHub Actions workflow builds all 4 required platforms automatically
2. **Pre-Publish Validation**: Script blocks npm publish if any binaries are missing
3. **Integrated Workflow**: `pnpm release:x` triggers entire pipeline seamlessly
4. **Fat Package Pattern**: All platform binaries included in single npm package (40-60MB)
5. **Safety Net**: Multiple validation gates prevent incomplete releases

### Success Metrics

- **Reliability**: 100% of releases include all 4 binaries
- **Developer Experience**: Single command (`pnpm release:x`) handles everything
- **Build Time**: Complete release process takes <15 minutes
- **Failure Detection**: Missing binaries caught before npm publish

## Implementation Plan

### Timeline: 3 days (with +1 day contingency)

1. **Phase 1**: GitHub Actions Workflow (1 day)
   - Create `.github/workflows/build-and-publish-maproom-mcp.yml`
   - Configure matrix builds for 4 platforms
   - Implement binary validation in CI
   - Set up artifact aggregation and npm publish

2. **Phase 2**: Local Validation Scripts (0.5 days)
   - Create `scripts/validate-binaries.js`
   - Add prepublishOnly hook to package.json
   - Test validation catches missing binaries

3. **Phase 3**: Release Script Integration (0.5 days)
   - Create `scripts/release.js`
   - Make `pnpm release:x` trigger CI pipeline
   - Add git tag automation

4. **Phase 4**: Documentation (0.5 days)
   - Update CONTRIBUTING.md or README
   - Document new release process
   - Add troubleshooting guide

5. **Phase 5**: Testing & Rollout (0.5 days)
   - Execute test release (canary)
   - Verify all 4 binaries work
   - Complete first production release
   - Monitor for 24 hours

## Relevant Agents

### Phase 1: GitHub Actions Workflow
- **Assigned**: `general-purpose`
- **Tools**: Write, Edit, Bash
- **Tasks**: Create workflow file, implement matrix builds, set up validation

### Phase 2: Local Validation Scripts
- **Assigned**: `general-purpose`
- **Tools**: Write, Edit, Bash
- **Tasks**: Create validation script, update package.json, test scenarios

### Phase 3: Release Script Integration
- **Assigned**: `general-purpose`
- **Tools**: Write, Edit, Bash
- **Tasks**: Create release script, integrate with git workflow

### Phase 4: Documentation
- **Assigned**: `general-purpose`
- **Tools**: Write, Edit, Read
- **Tasks**: Update documentation, create troubleshooting guide

### Phase 5: Testing & Rollout
- **Assigned**: `general-purpose`, `test-runner`, `verify-ticket`
- **Tools**: All tools
- **Tasks**: Execute test releases, verify functionality, monitor production

## Planning Documents

Comprehensive planning documentation is available in the `planning/` directory:

1. **[analysis.md](planning/analysis.md)** - Problem analysis, current state, industry solutions, recommended approach
2. **[architecture.md](planning/architecture.md)** - System architecture, component designs, build strategies, failure modes
3. **[quality-strategy.md](planning/quality-strategy.md)** - Testing philosophy, test pyramid, quality gates, success metrics
4. **[security-review.md](planning/security-review.md)** - Threat model, risk assessment, security controls, incident response
5. **[plan.md](planning/plan.md)** - 5-phase implementation plan with detailed tasks, timelines, and completion criteria

## Risk Mitigation

### High Priority Risks

1. **GitHub Actions Quota**: Use build caching, optimize build time
2. **First Release Failure**: Thorough canary testing, rollback plan ready
3. **Platform Build Failure**: Validation catches issues, CI retry available
4. **Breaking Changes**: Maintain backward compatibility, keep `pnpm release:x` interface

### Security Considerations

- NPM_TOKEN stored in GitHub secrets
- Binary validation (size, executability, platform)
- Audit trail (GitHub Actions logs, git history, npm publish history)
- No binary signing (acceptable for MVP, can enhance post-MVP)

## Acceptance Criteria

Project is complete when:

- ✅ GitHub Actions workflow builds all 4 platforms
- ✅ Validation script blocks incomplete publishes
- ✅ `pnpm release:x` triggers full pipeline
- ✅ Documentation updated
- ✅ At least one successful production release
- ✅ No critical issues reported

## Next Steps

1. **Review and approve plan** ← Current stage
2. **Generate tickets** from implementation plan
3. **Execute Phase 1** (GitHub Actions workflow)
4. **Execute Phases 2-5** sequentially
5. **Complete with production release**

---

**Ready to begin implementation via `/create-project-tickets BINPKG`**

# Implementation Plan: Integrated Rust Binary Packaging

## Project Overview

**Goal**: Integrate Rust binary building into the npm release process so that `pnpm release:x` reliably produces complete packages with all platform binaries.

**Duration**: 2-3 days (1 developer)

**Approach**: Build CI-first, then integrate with local workflow

## Phase 1: GitHub Actions Workflow (Priority 1)

**Objective**: Automate multi-platform binary builds in CI

**Deliverables**:
1. GitHub Actions workflow file
2. Matrix build configuration for 4 platforms
3. Binary validation in CI
4. Artifact aggregation

**Agent Recommendations**:
- **general-purpose**: Create workflow file, implement matrix builds
- **test-runner**: Validate workflow executes correctly

**Tasks**:
1. Create `.github/workflows/build-and-publish-maproom-mcp.yml`
2. Configure matrix with 4 platforms:
   - linux-x64 (ubuntu-latest + cross)
   - linux-arm64 (ubuntu-latest + cross)
   - darwin-x64 (macos-13)
   - darwin-arm64 (macos-latest)
3. Implement build steps:
   - Checkout code
   - Setup Rust toolchain
   - Install cross (Linux only)
   - Build binary: `cargo build --release --target <target>`
   - Strip binary: `strip <binary>`
   - Upload artifact
4. Implement validation job:
   - Download all artifacts
   - Check 4 binaries exist
   - Check binaries are executable
   - Test binary runs: `./binary --version`
5. Implement publish job:
   - Copy binaries to packages/maproom-mcp/bin/
   - Run npm pack (dry run)
   - Verify tarball contents
   - Run npm publish

**Success Criteria**:
- Workflow triggers on `v*.*.*` tags
- All 4 platforms build successfully
- Binaries pass validation
- npm publish succeeds

**Testing**:
- Create test tag, trigger workflow
- Verify all jobs pass
- Verify binaries in artifacts
- Verify package on npm (test version)

**Estimated Time**: 1 day

## Phase 2: Local Validation Scripts (Priority 1)

**Objective**: Prevent publishing packages without binaries

**Deliverables**:
1. Binary validation script
2. Package.json prepublishOnly hook
3. Clear error messages

**Agent Recommendations**:
- **general-purpose**: Create validation script
- **test-runner**: Test validation catches missing binaries

**Tasks**:
1. Create `scripts/validate-binaries.js`:
   - Check all 4 platform directories exist
   - Check each binary is present
   - Check binary sizes are reasonable (>1MB)
   - Print clear error messages
2. Update `packages/maproom-mcp/package.json`:
   - Add prepublishOnly: `node ../../scripts/validate-binaries.js`
   - Update files array to just `"bin"`
3. Test validation:
   - Delete one platform, verify error
   - Replace binary with small file, verify error
   - With all binaries, verify pass

**Success Criteria**:
- Script runs automatically before npm publish
- Missing binaries block publish
- Error messages clearly indicate problem
- All binaries present allows publish

**Testing**:
- Test with missing platform: publish fails
- Test with all platforms: publish succeeds
- Test with corrupted binary: publish fails

**Estimated Time**: 0.5 days

## Phase 3: Release Script Integration (Priority 1)

**Objective**: Make `pnpm release:x` trigger full CI pipeline

**Deliverables**:
1. New release script
2. Updated package.json scripts
3. Git tag automation

**Agent Recommendations**:
- **general-purpose**: Create release script
- **test-runner**: Test release workflow

**Tasks**:
1. Create `scripts/release.js`:
   - Validate git working directory clean
   - Validate on main/master branch
   - Parse version bump type (patch/minor/major)
   - Bump version in packages/maproom-mcp/package.json
   - Git commit: "chore(release): bump version to X.Y.Z"
   - Git tag: "vX.Y.Z"
   - Git push with tags: `git push --follow-tags`
   - Optionally monitor GitHub Actions workflow
2. Update `packages/maproom-mcp/package.json`:
   - Change release:patch to: `node ../../scripts/release.js patch`
   - Change release:minor to: `node ../../scripts/release.js minor`
   - Change release:major to: `node ../../scripts/release.js major`
3. Add optional workflow monitoring (post-MVP):
   - Use GitHub API to check workflow status
   - Print progress indicators
   - Report success/failure

**Success Criteria**:
- `pnpm release:patch` bumps version, commits, tags, pushes
- GitHub Actions workflow triggered automatically
- Developer sees progress/status
- Existing manual publish still works (with validation)

**Testing**:
- Dry run: verify no actual changes
- Test release: verify commit, tag, push
- Verify workflow triggered on GitHub
- Verify binaries built and published

**Estimated Time**: 0.5 days

## Phase 4: Documentation (Priority 2)

**Objective**: Document new release process

**Deliverables**:
1. Updated CONTRIBUTING.md or README
2. Workflow documentation
3. Troubleshooting guide

**Agent Recommendations**:
- **general-purpose**: Write documentation

**Tasks**:
1. Document new release process:
   - How to release: `pnpm release:patch`
   - What happens (workflow overview)
   - How to monitor progress
   - How to manually publish (emergency)
2. Document workflow:
   - Workflow file location
   - How to trigger manually
   - How to read CI logs
   - Common failures and fixes
3. Document binary packaging:
   - Why we include all platforms
   - How binaries are built
   - How to add new platforms
4. Update troubleshooting:
   - Missing binaries → run GitHub Actions
   - Workflow fails → check logs
   - npm publish fails → check credentials

**Success Criteria**:
- Developer can follow docs to release
- Common issues have solutions
- Emergency procedures documented

**Estimated Time**: 0.5 days

## Phase 5: Testing & Rollout (Priority 1)

**Objective**: Verify system works end-to-end

**Deliverables**:
1. Test release completed successfully
2. Rollback plan documented
3. First production release

**Agent Recommendations**:
- **general-purpose**: Execute test releases
- **test-runner**: Verify testing procedures
- **verify-ticket**: Final validation

**Tasks**:
1. Create test branch for dry-run
2. Execute dry-run release:
   - Run `pnpm release:patch --dry-run`
   - Verify no actual changes
   - Review planned actions
3. Execute canary release (test version):
   - Create release branch
   - Run `pnpm release:patch`
   - Monitor GitHub Actions
   - Verify all 4 binaries built
   - Verify npm publish succeeded
   - Test install on linux-x64
   - Test install on macOS (if available)
   - Verify binaries work: `npx @crewchief/maproom-mcp --version`
4. Document any issues found and fixes
5. Execute first production release:
   - Merge to main
   - Run `pnpm release:minor` (new minor for process change)
   - Monitor workflow
   - Verify success
   - Test installations
6. Monitor for 24 hours:
   - Watch for user reports
   - Check npm download stats
   - Verify no issues

**Success Criteria**:
- Canary release succeeds
- Production release succeeds
- Binaries work on all platforms
- No user reports of issues

**Rollback Plan**:
1. If workflow fails: fix and re-run
2. If bad package published: `npm unpublish @crewchief/maproom-mcp@X.Y.Z`
3. If binaries broken: unpublish, fix, republish with patch version

**Estimated Time**: 0.5 days

## Optional Enhancements (Post-MVP)

### Enhancement 1: Build Caching
**Value**: Faster CI builds (~2min savings)
**Effort**: 2-4 hours
**Agent**: general-purpose

**Tasks**:
- Add Rust cache action to workflow
- Cache cargo registry and target directory
- Verify cache hits working
- Measure time savings

### Enhancement 2: Post-Publish Verification
**Value**: Automated testing of published packages
**Effort**: 4-8 hours
**Agent**: integration-tester

**Tasks**:
- Create test-published-package.yml workflow
- Matrix test on all 4 platforms
- Install package from npm
- Run basic functionality tests
- Trigger automatically after publish or manually

### Enhancement 3: Workflow Monitoring
**Value**: Better developer experience
**Effort**: 4-8 hours
**Agent**: general-purpose

**Tasks**:
- Use GitHub API in release script
- Poll workflow status
- Display progress indicators
- Report final success/failure
- Provide link to workflow run

### Enhancement 4: Windows Support
**Value**: Support Windows users
**Effort**: 8-16 hours
**Agent**: general-purpose

**Tasks**:
- Add windows-x64 to matrix
- Handle .exe extension
- Update platform detection in cli.cjs
- Test on Windows
- Update documentation

### Enhancement 5: Binary Signing
**Value**: Cryptographic authenticity
**Effort**: 16-24 hours
**Agent**: general-purpose

**Tasks**:
- Generate signing key
- Sign binaries in CI
- Distribute public key
- Verify signatures on install
- Document signing process

## Risk Mitigation

### Risk: GitHub Actions Quota Exhausted
**Likelihood**: Low (generous free tier)
**Impact**: Builds blocked

**Mitigation**:
- Monitor Actions minutes usage
- Optimize build time with caching
- Fallback: manual publish still works

### Risk: First Release Fails
**Likelihood**: Medium (new system)
**Impact**: Delayed release

**Mitigation**:
- Thorough testing with canary release
- Rollback plan ready
- Manual publish fallback
- Phase 5 specifically for this

### Risk: Platform Build Fails
**Likelihood**: Low (stable tooling)
**Impact**: Incomplete package

**Mitigation**:
- Validation catches missing platforms
- CI retry mechanism
- Manual fix and re-run

### Risk: Breaking Changes to Existing Workflow
**Likelihood**: Low (additive changes)
**Impact**: Developers confused

**Mitigation**:
- Keep pnpm release:x as interface
- Maintain backward compatibility
- Clear documentation
- Announce changes

## Dependencies

### External Dependencies
- GitHub Actions (stable) ✓
- npm registry API (stable) ✓
- Rust toolchain (stable) ✓
- cross tool (stable) ✓

### Internal Dependencies
- packages/maproom-mcp/package.json ✓
- crates/maproom/Cargo.toml ✓
- Existing build scripts ✓

**All dependencies are stable and under control.**

## Success Metrics

### Primary Metrics
1. **Binary Completeness**: 100% of releases include all 4 binaries
2. **Build Success Rate**: >95% of CI builds succeed
3. **Developer Satisfaction**: "Releasing is easy and reliable"

### Secondary Metrics
1. **Build Time**: <15 minutes per release
2. **Package Size**: <100MB
3. **Installation Success**: >99% of installs work

## Timeline

### Week 1
- **Day 1**: Phase 1 (GitHub Actions workflow)
- **Day 2**: Phase 2 (Validation scripts) + Phase 3 (Release script)
- **Day 3**: Phase 4 (Documentation) + Phase 5 (Testing)

### Total: 3 days

**Contingency**: +1 day for issues/refinement

## Completion Criteria

Project is complete when:
- [x] GitHub Actions workflow builds all 4 platforms
- [x] Validation script blocks incomplete publishes
- [x] `pnpm release:x` triggers full pipeline
- [x] Documentation updated
- [x] At least one successful production release
- [x] No critical issues reported

## Next Steps

1. Review and approve plan
2. Begin Phase 1 implementation
3. Execute phases sequentially
4. Validate after each phase
5. Complete with production release

---

**Ready to begin implementation.**

# Plan: CLI GitHub Actions Release Automation

## Project Goal

Migrate the `@crewchief/cli` package from manual local releases to automated GitHub Actions releases with multi-platform binary builds, independent versioning from `@crewchief/maproom-mcp`, and proper deprecation of the old `crewchief` package.

## Project Phases

### Phase 1: Old Package Deprecation

**Goal**: Properly deprecate the `crewchief` package and guide users to new scoped package.

**Deliverables**:
- Final `crewchief@1.0.0` published with deprecation warnings
- postinstall script displays migration message
- Package marked as deprecated on npm registry
- Documentation updated with migration guide

**Agent**: general-purpose

**Success Criteria**:
- `npm install crewchief` shows deprecation warning
- npm page displays deprecation notice
- Warning message includes `@crewchief/cli` package name

**Risk**: Low - One-time manual operation, minimal user impact

---

### Phase 2: CLI Package Configuration

**Goal**: Reconfigure CLI package for scoped name, updated metadata, and workflow-based publishing.

**Deliverables**:
- `packages/cli/package.json` updated:
  - Name: `@crewchief/cli`
  - Version: `1.0.0`
  - publishConfig: `{ "access": "public" }`
  - Remove `prepublishOnly` hook
- `packages/cli/.npmignore` created (exclude sources, include binaries)
- `packages/cli/README.md` updated with new package name
- Local `npm pack` test validates package structure

**Agent**: general-purpose

**Success Criteria**:
- `npm pack` produces tarball with correct name and structure
- package.json has all required scoped package fields
- .npmignore properly filters source files

**Risk**: Low - Configuration changes, easily reversible

---

### Phase 3: Release Script Updates

**Goal**: Update release scripts to use package-scoped tags, fix race condition with two-step push, and remove manual publishing.

**Problem**: Current maproom-mcp workflow has a race condition when using `git push --follow-tags`. The tag can arrive at GitHub before the commit is fully registered, causing the workflow trigger to fail.

**Solution**: Push in two separate steps:
1. `git push` (commits only)
2. `git push origin <tag>` (tag separately)

**Deliverables**:
- `packages/cli/scripts/release.mjs`:
  - Tag format: `@crewchief/cli@v{version}`
  - Remove `pnpm publish` command
  - Change from `git push --follow-tags` to two-step push:
    1. `git push`
    2. `git push origin @crewchief/cli@v{version}`
  - Add instructions explaining the two-step process
- `packages/maproom-mcp/scripts/release.js`:
  - Tag format: `@crewchief/maproom-mcp@v{version}`
  - Fix race condition with two-step push:
    1. `git push`
    2. `git push origin @crewchief/maproom-mcp@v{version}`
  - Ensure consistency with CLI pattern

**Agent**: general-purpose

**Success Criteria**:
- Release script creates correctly formatted tags
- Release script pushes commits first, then tags separately
- Release script does not attempt to publish
- Instructions explain why two-step push is needed
- Test run shows commits arrive before tags

**Risk**: Low - Script changes, tested in dry-run

---

### Phase 4: CLI GitHub Actions Workflow

**Goal**: Create automated multi-platform build and publish workflow for CLI package.

**Deliverables**:
- `.github/workflows/build-and-publish-cli.yml`:
  - Trigger on `@crewchief/cli@v*.*.*` tags
  - Matrix build for 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
  - TypeScript build with tsup
  - Binary validation (existence, size, execution)
  - Package assembly and npm publish
  - Post-publish verification
  - Dry-run support via workflow_dispatch
- Validation script (inline in workflow)
- Documentation in workflow comments

**Agent**: docker-engineer (for GitHub Actions YAML expertise)

**Success Criteria**:
- Dry-run workflow completes successfully
- All 4 platform binaries build and validate
- Package structure correct (all binaries included)
- Validation catches missing/wrong binaries

**Risk**: Medium - Complex workflow, needs thorough testing

**Dependencies**: Phase 2 (package configuration), Phase 3 (tag format)

---

### Phase 5: MCP Workflow Migration

**Goal**: Update MCP workflow to use package-scoped tags and ensure isolation from CLI workflow.

**Deliverables**:
- `.github/workflows/build-and-publish-maproom-mcp.yml`:
  - Update trigger to `@crewchief/maproom-mcp@v*.*.*`
  - No other changes (already working)
- Tag isolation test (verify CLI tag doesn't trigger MCP workflow and vice versa)

**Agent**: general-purpose

**Success Criteria**:
- MCP workflow triggers only on MCP tags
- CLI workflow triggers only on CLI tags
- No cross-triggering between workflows

**Risk**: Low - Minimal change to working workflow

**Dependencies**: Phase 4 (CLI workflow created)

---

### Phase 6: Security Hardening

**Goal**: Implement security baseline for production releases.

**Deliverables**:
- GitHub repository settings:
  - Tag protection rules (maintainers only)
  - Branch protection (require reviews)
  - NPM_TOKEN secret configured
- `SECURITY.md` in repository root
- CODEOWNERS file for `.github/workflows/`
- npm account 2FA enabled (documented)
- Security checklist completed

**Agent**: general-purpose

**Success Criteria**:
- Tag protection prevents unauthorized tag creation
- NPM_TOKEN stored as secret (not exposed)
- SECURITY.md published with vulnerability reporting process

**Risk**: Low - Configuration and documentation

---

### Phase 7: Dry-Run Validation

**Goal**: Test complete automation end-to-end without publishing to production.

**Deliverables**:
- Test tag created: `@crewchief/cli@v1.0.0-test`
- Workflow triggered with dry_run=true
- Validation report:
  - All platform builds succeeded
  - All binaries validated
  - Package structure correct
  - No publish step executed
- Test tag and artifacts cleaned up

**Agent**: general-purpose

**Success Criteria**:
- Workflow completes without errors
- All validation checks pass
- Artifacts contain expected binaries
- No package published to npm

**Risk**: Low - Testing only, no production impact

**Dependencies**: Phase 4 (workflow), Phase 6 (security setup)

---

### Phase 8: Production Release

**Goal**: Execute first production release of `@crewchief/cli@v1.0.0`.

**Deliverables**:
- Tag created: `@crewchief/cli@v1.0.0`
- Workflow runs and publishes package
- Post-release validation:
  - Package appears on npm
  - All 4 binaries included
  - Installation test succeeds
  - CLI executes correctly
- Release monitoring setup

**Agent**: general-purpose

**Success Criteria**:
- `@crewchief/cli@1.0.0` published to npm
- Package installs successfully on all platforms
- Binary execution test passes
- No errors in workflow logs

**Risk**: Medium - First production release, potential for issues

**Dependencies**: Phase 7 (dry-run successful)

---

### Phase 9: Documentation and Knowledge Transfer

**Goal**: Document the new release process and archive project.

**Deliverables**:
- Repository README.md updated:
  - Installation instructions for `@crewchief/cli`
  - Migration guide from `crewchief`
  - Release process documentation
- Release workflow documentation:
  - How to create releases
  - Troubleshooting common issues
  - Security incident response
- Project archived to `.crewchief/archive/projects/`

**Agent**: general-purpose

**Success Criteria**:
- README clearly explains new package name
- Release process documented for future maintainers
- Troubleshooting guide covers common scenarios

**Risk**: Low - Documentation only

---

## Agent Assignments

| Phase | Primary Agent | Rationale |
|-------|--------------|-----------|
| 1. Deprecation | general-purpose | Simple package publish and npm commands |
| 2. Package Config | general-purpose | JSON and configuration file edits |
| 3. Release Scripts | general-purpose | JavaScript file modifications |
| 4. CLI Workflow | docker-engineer | GitHub Actions YAML expertise, CI/CD focus |
| 5. MCP Workflow | general-purpose | Simple trigger update |
| 6. Security | general-purpose | Repository settings and documentation |
| 7. Dry-Run | general-purpose | Testing and validation |
| 8. Production | general-purpose | Release execution and monitoring |
| 9. Documentation | general-purpose | README and knowledge transfer |

**Note**: docker-engineer is specifically chosen for Phase 4 because it has expertise in CI/CD workflows, Docker/container builds (similar patterns to matrix builds), and multi-platform binary distribution.

## Risk Mitigation

**High-Risk Items**:
1. **Phase 4** (CLI Workflow): Most complex, requires thorough testing
   - **Mitigation**: Copy proven MCP workflow pattern, dry-run validation
2. **Phase 8** (Production Release): First real publish, irreversible
   - **Mitigation**: Phase 7 dry-run must succeed, manual review before tag push

**Medium-Risk Items**:
1. **Tag isolation** (Phase 5): Could trigger wrong workflow
   - **Mitigation**: Test with synthetic tags before production
2. **Binary validation** (Phase 4): Could miss incorrect binaries
   - **Mitigation**: Multiple validation checks (existence, size, execution)

**Low-Risk Items**:
- Phases 1, 2, 3, 6, 9: Configuration and documentation
- Phase 7: Testing only, no production impact

## Dependencies

```
Phase 1 (Deprecation)
  ↓ (no blocking dependency)
Phase 2 (Package Config)
  ↓
Phase 3 (Release Scripts) ──┐
  ↓                          ↓
Phase 4 (CLI Workflow) ←─────┘
  ↓
Phase 5 (MCP Workflow)
  ↓
Phase 6 (Security) ──┐
  ↓                  ↓
Phase 7 (Dry-Run) ←──┘
  ↓
Phase 8 (Production)
  ↓
Phase 9 (Documentation)
```

**Critical Path**: Phase 2 → 3 → 4 → 7 → 8 (Configuration → Workflow → Testing → Release)

## Timeline Estimate

**Optimistic** (all phases smooth):
- Phase 1: 1 hour
- Phase 2: 2 hours
- Phase 3: 1 hour
- Phase 4: 6 hours (workflow creation + testing)
- Phase 5: 1 hour
- Phase 6: 2 hours
- Phase 7: 2 hours (dry-run + validation)
- Phase 8: 1 hour
- Phase 9: 2 hours

**Total**: ~18 hours (2-3 days)

**Realistic** (with debugging):
- Phase 4: +4 hours (workflow debugging)
- Phase 7: +2 hours (fixing issues found)
- Phase 8: +1 hour (post-release validation)

**Total**: ~25 hours (3-4 days)

**Pessimistic** (significant issues):
- Phase 4: +8 hours (complex debugging)
- Phase 7: +4 hours (multiple iterations)
- Phase 8: +2 hours (rollback and retry)

**Total**: ~32 hours (4-5 days)

## Success Metrics

**Functional**:
- ✅ CLI package published as `@crewchief/cli@1.0.0`
- ✅ Package contains binaries for all 4 platforms
- ✅ Independent tagging works (no cross-triggering)
- ✅ Old package deprecated with migration guide

**Quality**:
- ✅ Zero manual steps after tag creation
- ✅ Validation prevents broken releases
- ✅ Workflow completes in <15 minutes
- ✅ Dry-run testing works

**Security**:
- ✅ NPM_TOKEN stored as secret
- ✅ Tag protection enabled
- ✅ Security baseline documented
- ✅ Incident response plan created

**Process**:
- ✅ Release process documented
- ✅ Troubleshooting guide created
- ✅ Knowledge transferred to team

## Rollback Plan

**If Phase 4 workflow fails validation**:
- Do not proceed to production
- Debug workflow in isolation
- Re-run Phase 7 dry-run until successful

**If Phase 8 production release fails**:
- Workflow fails → No package published, tag remains
- Package published but broken → Immediately publish v1.0.1 with fix
- Cannot fix quickly → Mark v1.0.0 as deprecated, publish v1.0.1

**If security issue discovered**:
- Immediately unpublish affected version
- Rotate NPM_TOKEN
- Review and fix vulnerability
- Publish patched version
- Issue security advisory

## Phase Completion Criteria

Each phase must meet its success criteria before proceeding to next phase. The verify-ticket agent will validate completion.

**Key checkpoints**:
- After Phase 4: Workflow created and syntax-valid
- After Phase 7: Dry-run successful, no errors
- Before Phase 8: Manual review and approval
- After Phase 8: Package installed and tested on 2+ platforms

## Project Completion

Project is complete when:
- [x] All 9 phases completed successfully
- [x] `@crewchief/cli@1.0.0` published and functional
- [x] Old `crewchief` package deprecated
- [x] Independent workflows operational
- [x] Security baseline implemented
- [x] Documentation complete
- [x] Project archived

Post-completion:
- Monitor npm downloads for anomalies (first week)
- Monitor GitHub issues for installation problems
- Review security posture after 3 months
- Consider enterprise enhancements if user base grows

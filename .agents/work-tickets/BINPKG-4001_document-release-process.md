# Ticket: BINPKG-4001: Document new release process and binary packaging

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update project documentation (README, CONTRIBUTING, or create new RELEASING.md) to document the new automated release process, explain the binary packaging approach, and provide troubleshooting guidance.

## Background
The release process has fundamentally changed from manual binary builds + publish to automated CI/CD. Previously, developers had to manually build binaries for each platform and publish to npm. Now, running `pnpm release:patch` (or minor/major) triggers a GitHub Actions workflow that automatically builds binaries for all 4 platforms and publishes the package. Developers need clear documentation on how to release, what happens behind the scenes, and how to troubleshoot issues. This prevents confusion and reduces support burden.

This ticket implements the documentation requirements outlined in the BINPKG binary packaging project plan.

## Acceptance Criteria
- [ ] Document new release process:
  - How to release: `pnpm release:patch` (or minor/major)
  - What happens automatically (workflow overview)
  - How long it takes (~12-15 minutes)
  - How to monitor progress (GitHub Actions)
- [ ] Document binary packaging approach:
  - Why we include all 4 platforms (fat package)
  - List of supported platforms (linux-x64, linux-arm64, macos-x64, macos-arm64)
  - Package size (~50MB)
  - How binaries are detected at runtime
- [ ] Document workflow details:
  - Workflow file location (.github/workflows/release-binaries.yml)
  - How to trigger manually (workflow_dispatch)
  - How to read CI logs (step-by-step)
- [ ] Document troubleshooting:
  - "Missing binaries" → wait for GitHub Actions or run workflow manually
  - "Workflow failed" → check logs, common failures (cross install, npm auth)
  - "npm publish failed" → check NPM_TOKEN, version conflict
  - Manual emergency publish: `pnpm publish` (has validation from BINPKG-2002)
- [ ] Document adding new platforms (for future):
  - Add to matrix in workflow
  - Update validation scripts
  - Test on new platform
- [ ] Choose appropriate location:
  - Option A: Create `docs/RELEASING.md`
  - Option B: Update `CONTRIBUTING.md` release section
  - Option C: Update main README with link to details
- [ ] Include verification section:
  - How to verify a release completed successfully
  - Test install commands for users

## Technical Requirements
- **Format**: Markdown
- **Sections**:
  1. Overview (fat package approach)
  2. How to Release (pnpm release:x commands)
  3. What Happens (workflow flowchart/steps)
  4. Monitoring (GitHub Actions links)
  5. Troubleshooting (common issues + fixes)
  6. Emergency Procedures (manual publish)
  7. Adding Platforms (future maintenance)
- **Include**: Code examples, links to workflow file, screenshots optional
- **Tone**: Clear, concise, action-oriented
- **Maintenance**: Note last updated date to help identify stale docs
- **Link to Source of Truth**: Reference workflow file as authoritative source

## Implementation Notes
- **Consistency**: Look at existing docs structure to maintain consistency
- **Workflow Reference**: Link to GitHub Actions workflow file (.github/workflows/release-binaries.yml)
- **Planning Docs**: Reference `.agents/projects/BINPKG_binary-packaging/` for technical details
- **Diagram Suggestion**: Consider adding a diagram showing: Developer → pnpm release → GitHub Actions → npm registry
- **Verification Section**: Add "How to verify a release" section with test install commands
- **User vs Developer Docs**: Consider separate sections for users (installing) vs developers (releasing)
- **Package Location**: Document in `packages/maproom-mcp/README.md` for user-facing docs, and main docs for developer workflow

**Example Workflow Diagram**:
```
Developer runs:           GitHub Actions:              npm Registry:
pnpm release:patch  →    Build linux-x64       →      Package published
                         Build linux-arm64             with all 4 binaries
                         Build macos-x64               (~50MB total)
                         Build macos-arm64
                         Validate binaries
                         npm publish
```

**Example Troubleshooting Entry**:
```markdown
### Missing Binaries Error

**Problem**: Error during install: "Missing binary for platform linux-x64"

**Cause**: GitHub Actions workflow hasn't completed yet

**Solution**:
1. Check GitHub Actions: https://github.com/org/repo/actions
2. Wait for "Release Binaries" workflow to complete (~12-15 minutes)
3. If workflow failed, trigger manually: Actions → Release Binaries → Run workflow
4. Verify binaries in published package on npm
```

## Dependencies
- **BINPKG-3002** - release scripts must be finalized
- GitHub Actions workflow must be deployed and tested
- Planning documentation at `.agents/projects/BINPKG_binary-packaging/`

## Risk Assessment
- **Risk**: Documentation becomes stale as workflow evolves
  - **Mitigation**: Note last updated date, link to workflow file as source of truth, add reminder to update docs when workflow changes
- **Risk**: Developers skip documentation and run old manual workflow
  - **Mitigation**: Deprecate old scripts, show clear "NEW WORKFLOW" banner in docs, add warnings in old script files
- **Risk**: Troubleshooting section doesn't cover real-world failures
  - **Mitigation**: Start with known issues, update docs when new issues are reported, link to GitHub issues for tracking
- **Risk**: Documentation location is hard to find
  - **Mitigation**: Add prominent links from README, CONTRIBUTING, and package README

## Files/Packages Affected
- **Option A (Recommended)**: CREATE `docs/RELEASING.md`
  - Dedicated release documentation
  - Easy to find and maintain
  - Can be linked from other docs
- **Option B**: MODIFY `CONTRIBUTING.md`
  - Add release section
  - Keep contribution docs together
- **Option C**: MODIFY `README.md`
  - Add release section or link to RELEASING.md
  - High visibility
- **Also Consider**: UPDATE `packages/maproom-mcp/README.md`
  - User-facing installation docs
  - Explain what users see (fat package with all binaries)
- **Links to Add**:
  - Main README → RELEASING.md or release section
  - CONTRIBUTING.md → release documentation
  - Package README → link to release docs for contributors

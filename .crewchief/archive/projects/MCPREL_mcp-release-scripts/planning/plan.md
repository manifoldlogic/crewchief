# Implementation Plan: MCP Release Scripts

## Project Overview

Update `@crewchief/maproom-mcp` release scripts to support GitHub Actions workflow by creating git commits and tags instead of directly publishing to npm.

**Complexity**: Simple
**Duration**: 1-2 hours
**Tickets**: 3-4 small tickets

## Phases

### Phase 1: Script Implementation (Core)

**Goal**: Create functional release script

**Tickets**:
1. **MCPREL-1001**: Create release.js script
   - Implement sequential workflow: bump → commit → tag → push
   - Add input validation for release type
   - Implement git command execution with error handling
   - Read new version from package.json after bump
   - Create commit with format: "chore(release): bump version to X.Y.Z"
   - Create annotated tag with format: "vX.Y.Z"
   - Push commit and tag to origin

2. **MCPREL-1002**: Update package.json scripts
   - Replace `release:patch` script to call `release.js patch`
   - Replace `release:minor` script to call `release.js minor`
   - Replace `release:major` script to call `release.js major`
   - Remove `pnpm publish` commands from all three scripts

**Agent**: general-purpose (simple JavaScript file creation and editing)

**Deliverables**:
- `/workspace/packages/maproom-mcp/scripts/release.js` (new file)
- `/workspace/packages/maproom-mcp/package.json` (modified)

**Acceptance Criteria**:
- Scripts execute without syntax errors
- Git commands run in correct sequence
- Error handling catches and reports failures
- Version, commit, tag, and push all complete successfully

---

### Phase 2: Testing & Validation (Quality Assurance)

**Goal**: Verify scripts work correctly and trigger GitHub Actions

**Tickets**:
3. **MCPREL-2001**: Manual testing
   - Test `pnpm release:patch` on feature branch
   - Verify version bump, commit message, tag format
   - Verify push to origin completes
   - Verify GitHub Actions workflows trigger (both workflows)
   - Monitor workflow execution logs for errors
   - Test error handling (invalid argument, etc.)
   - Document test results

**Agent**: general-purpose or unit-test-runner

**Deliverables**:
- Testing notes/documentation
- Verification that all operations complete
- Confirmation that GitHub Actions triggered successfully

**Acceptance Criteria**:
- Patch release completes successfully
- Commit message follows format: "chore(release): bump version to X.Y.Z"
- Tag format is correct: "vX.Y.Z"
- Both commit and tag are pushed to origin
- GitHub Actions workflows trigger on tag push:
  - `build-and-publish-maproom-mcp.yml` starts
  - `publish-maproom-mcp-image.yml` starts
  - Both workflows complete successfully (can check via GitHub UI or `gh run list`)
- Error cases handled gracefully

**Important**: Since this triggers real GitHub Actions that publish to npm and Docker Hub, testing should be done carefully. Consider:
- Testing on a non-production branch first
- Using a pre-release version (e.g., v1.3.2-test.1) that can be unpublished if needed
- Or coordinating with project owner before running real release

---

### Phase 3: Documentation (Polish - Optional)

**Goal**: Document new workflow for developers

**Tickets**:
4. **MCPREL-3001**: Update README (if needed)
   - Document new release workflow
   - Add examples of running release scripts
   - Note that GitHub Actions handles publishing

**Agent**: general-purpose

**Deliverables**:
- Updated README (if package has developer docs)
- Or skip if documentation is minimal

**Acceptance Criteria**:
- Developers understand how to use new scripts
- GitHub Actions workflow is mentioned

**Note**: This may not be needed depending on existing documentation state. Evaluate during implementation.

---

## Implementation Strategy

### Sequential Execution
Tickets should be completed in order:
1. Implement core functionality first (MCPREL-1001, 1002)
2. Test thoroughly (MCPREL-2001)
3. Document if needed (MCPREL-3001)

### Testing Approach
- Manual testing on feature branch
- Verify operations with git commands
- Clean up test tags/commits after validation

### Agent Assignment
- **general-purpose**: Handles all implementation and testing
  - Simple JavaScript file creation
  - Package.json editing
  - Manual test execution
  - Documentation updates

### No Specialized Agents Needed
This is straightforward script work - no need for specialized agents like:
- ❌ integration-tester (manual testing is sufficient)
- ❌ docker-engineer (no Docker changes)
- ❌ rust-indexer-engineer (no Rust changes)
- ❌ technical-researcher (simple problem, no research needed)

---

## Risk Management

### Risk 1: Breaking Existing Workflow
**Impact**: Medium
**Likelihood**: Low
**Mitigation**:
- Test on feature branch first
- Keep `bump-version.js` unchanged (backward compatible)
- Can revert package.json changes if needed

### Risk 2: Git Push Failures
**Impact**: Low (developer can retry manually)
**Likelihood**: Low (standard git operations)
**Mitigation**:
- Clear error messages
- Document recovery procedures
- Git provides good error output

### Risk 3: Incomplete Operations
**Impact**: Low (partial state)
**Likelihood**: Low
**Mitigation**:
- Fail fast on errors
- Document cleanup procedures
- Git operations are mostly atomic

---

## Success Criteria

Project is complete when:
1. ✅ Developer can run `pnpm release:patch` successfully
2. ✅ Version is bumped in package.json
3. ✅ Git commit is created with correct message format
4. ✅ Git tag is created with correct format (vX.Y.Z)
5. ✅ Both commit and tag are pushed to origin
6. ✅ Error cases are handled with clear messages
7. ✅ Manual testing confirms all operations work

---

## Dependencies

### External Dependencies
- Git (command-line tool) - Already required by project
- Node.js >= 18 - Already specified in package.json
- pnpm - Already used by project

### Internal Dependencies
- Existing `bump-version.js` script - Already exists, no changes needed
- Package.json structure - Stable, well-defined format

### No Blocking Dependencies
All requirements are already satisfied.

---

## Rollback Plan

If anything goes wrong:

1. **Revert package.json**:
   ```bash
   git checkout packages/maproom-mcp/package.json
   ```

2. **Delete release.js**:
   ```bash
   rm packages/maproom-mcp/scripts/release.js
   ```

3. **Clean up test tags**:
   ```bash
   git tag -d vX.Y.Z
   git push origin :refs/tags/vX.Y.Z
   ```

4. **Return to previous workflow**: Old scripts still work since `bump-version.js` is unchanged

---

## Timeline Estimate

| Phase | Tickets | Estimated Time |
|-------|---------|---------------|
| Phase 1: Implementation | MCPREL-1001, 1002 | 45-60 minutes |
| Phase 2: Testing | MCPREL-2001 | 15-30 minutes |
| Phase 3: Documentation | MCPREL-3001 | 15 minutes (if needed) |
| **Total** | **3-4 tickets** | **1-2 hours** |

---

## Deliverables Checklist

### Code
- [ ] `scripts/release.js` created
- [ ] `package.json` scripts updated
- [ ] All scripts execute without errors

### Testing
- [ ] Manual test on feature branch passed
- [ ] Error handling verified
- [ ] Commit/tag format verified

### Documentation
- [ ] README updated (if applicable)
- [ ] Test results documented

---

## Post-Implementation

### Immediate Next Steps
1. Run first real release using new scripts
2. Verify GitHub Actions workflows trigger correctly:
   - Check GitHub Actions tab: https://github.com/danielbushman/crewchief/actions
   - Or use CLI: `gh run list --workflow=build-and-publish-maproom-mcp.yml`
   - Monitor both workflows complete successfully
3. Verify published artifacts:
   - npm package: `npm view @crewchief/maproom-mcp@X.Y.Z`
   - Docker Hub: https://hub.docker.com/r/manifoldlogic/crewchief_maproom-mcp/tags
4. Monitor for any issues in first few releases

### Monitoring GitHub Actions
After pushing tag, monitor workflows:
```bash
# List recent workflow runs
gh run list --workflow=build-and-publish-maproom-mcp.yml --limit 5

# Watch specific run
gh run watch <run-id>

# View logs if needed
gh run view <run-id> --log
```

Expected result:
- Both workflows complete with green checkmarks
- npm package available at https://www.npmjs.com/package/@crewchief/maproom-mcp
- Docker images available at Docker Hub with proper tags

### Future Enhancements (Not Now)
- Add dry-run mode if needed
- Add interactive confirmation if needed
- Auto-generate changelog if needed
- Create GitHub release via API if needed
- Add pre-release version support (alpha, beta, rc)

**Decision**: Don't implement these now. Wait for actual need.

---

## Ticket Generation

When generating tickets from this plan:

**MCPREL-1001**: Focus on release.js implementation
- Clear acceptance criteria for each git operation
- Specify commit message format exactly
- Specify tag format exactly
- Include error handling requirements

**MCPREL-2002**: Focus on package.json changes
- Simple search-and-replace for scripts
- Verify no syntax errors in JSON

**MCPREL-2001**: Focus on manual testing
- Provide clear test commands
- Specify verification steps
- Document expected vs actual results

**MCPREL-3001**: (Optional) Focus on documentation
- Update relevant sections only
- Keep changes minimal
- Mention GitHub Actions integration

---

## Summary

Simple, focused project to update release automation:
- 3-4 small tickets
- 1-2 hours total time
- Low risk, high confidence
- Manual testing sufficient
- No new dependencies
- Backward compatible

Philosophy: Keep it simple, ship it, iterate if needed.

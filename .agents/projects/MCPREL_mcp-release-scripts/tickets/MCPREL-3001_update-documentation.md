# Ticket: MCPREL-3001: Update documentation for new release workflow (OPTIONAL)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Note
**This ticket is OPTIONAL.** Evaluate during implementation whether README updates are needed. If the package has minimal documentation or release process is self-explanatory from package.json scripts, this can be skipped.

## Agents
- general-purpose

## Summary
Update README or developer documentation (if it exists) to document the new git-based release workflow. Explain that running `pnpm release:patch/minor/major` creates git commits and tags that trigger automated GitHub Actions workflows for building and publishing.

## Background
With MCPREL-1001 and MCPREL-1002 complete, the release process has changed from direct npm publishing to git-tag-triggered CI/CD. If the package has developer documentation (README, CONTRIBUTING, etc.), it should be updated to reflect this new workflow so future developers understand:

1. How to release new versions
2. What happens when they run release scripts
3. That GitHub Actions handles the actual building and publishing

However, if no such documentation exists or the release process is obvious from package.json scripts alone, this ticket can be skipped.

## Acceptance Criteria
- [ ] README or developer docs evaluated for update necessity
- [ ] If updates needed: Release workflow documented clearly
- [ ] If updates needed: GitHub Actions role explained
- [ ] If updates needed: Example commands provided
- [ ] If no updates needed: Documented why (e.g., "no developer README exists", "process is self-explanatory")

## Technical Requirements

### Documentation to Check
Look for documentation files in `packages/maproom-mcp/`:
- `README.md`
- `CONTRIBUTING.md`
- `DEVELOPMENT.md`
- `docs/` directory

### Content to Add (if needed)

#### Release Process Section
```markdown
## Releasing New Versions

To release a new version of `@crewchief/maproom-mcp`:

1. **Ensure you're on main branch** with latest changes:
   ```bash
   git checkout main
   git pull origin main
   ```

2. **Run the appropriate release command**:
   ```bash
   # For bug fixes (1.3.1 → 1.3.2)
   pnpm release:patch

   # For new features (1.3.2 → 1.4.0)
   pnpm release:minor

   # For breaking changes (1.4.0 → 2.0.0)
   pnpm release:major
   ```

3. **What happens automatically**:
   - Version in `package.json` is bumped
   - Git commit is created: `chore(release): bump version to X.Y.Z`
   - Git tag is created: `vX.Y.Z`
   - Both are pushed to GitHub
   - GitHub Actions workflows trigger:
     - Builds Rust binaries for 4 platforms
     - Publishes npm package with binaries
     - Builds and publishes Docker images to Docker Hub

4. **Monitor the release**:
   ```bash
   # Watch GitHub Actions
   gh run list --workflow=build-and-publish-maproom-mcp.yml

   # Verify npm package published
   npm view @crewchief/maproom-mcp@X.Y.Z

   # Verify Docker images published
   docker pull manifoldlogic/crewchief_maproom-mcp:X.Y.Z
   ```

### CI/CD

Releases are fully automated via GitHub Actions:
- **`build-and-publish-maproom-mcp.yml`**: Builds cross-platform Rust binaries and publishes to npm
- **`publish-maproom-mcp-image.yml`**: Builds multi-platform Docker images and publishes to Docker Hub

Both workflows trigger on version tag push (`v*.*.*`).
```

### Alternative: Minimal Documentation
If no comprehensive README exists, at minimum add a comment in `package.json`:
```json
{
  "scripts": {
    "release:patch": "node scripts/release.js patch",  // Creates git tag, triggers CI/CD
    "release:minor": "node scripts/release.js minor",  // Creates git tag, triggers CI/CD
    "release:major": "node scripts/release.js major"   // Creates git tag, triggers CI/CD
  }
}
```

## Implementation Notes

### Decision Tree
1. **Check if README exists**: `ls packages/maproom-mcp/README.md`
2. **If no README**: Skip this ticket (mark as "Not needed - no developer documentation")
3. **If README exists**: Check if it documents release process
4. **If release process documented**: Update it with new workflow
5. **If release process not documented**: Consider adding it (short section)

### Keep It Minimal
Per project philosophy ("don't overdo it"), documentation should be:
- Short and clear
- Focus on what developers need to know
- Don't explain every detail of GitHub Actions
- Link to workflows for those who want details

## Dependencies
- **BLOCKED BY**: MCPREL-2001 (testing must pass before documenting)
- **SOFT DEPENDENCY**: Can be done anytime after MCPREL-1001, 1002

## Risk Assessment
- **Risk**: None (documentation only)
  - **Mitigation**: N/A
- **Risk**: Over-documentation adds maintenance burden
  - **Mitigation**: Keep minimal, focus on essential workflow

## Files/Packages Affected
- **MAYBE MODIFY**: `/workspace/packages/maproom-mcp/README.md`
- **MAYBE MODIFY**: `/workspace/packages/maproom-mcp/CONTRIBUTING.md`
- **OR**: Skip if no documentation files exist

## Evaluation Criteria

### When to Update
- ✅ README exists and has developer section
- ✅ Release process was previously documented
- ✅ Package is actively developed by multiple contributors

### When to Skip
- ❌ No README or developer documentation exists
- ❌ Package.json scripts are self-explanatory
- ❌ Release process obvious from context
- ❌ Single developer project

## Estimated Time
10-15 minutes (if updates needed)
5 minutes (if evaluation determines no update needed)

# Ticket: CICDOPT-3003: Delete Old Workflows and Clean Up

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (workflow cleanup, no tests required)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
Archive old duplicate workflows after successful consolidation and validation in production. Remove test caller workflows used for Phase 2 validation. Clean up repository to reflect the new consolidated workflow architecture.

## Background
After CICDOPT-3001 and CICDOPT-3002 complete successfully, the repository will contain:
- Old workflows backed up with `.old` extension
- Test caller workflows from Phase 2 (`test-reusable-rust.yml`, `test-reusable-typescript.yml`)
- Duplicate workflow files causing confusion in the repository

This ticket ensures a clean final state after consolidation is validated in production. Old workflows will be committed with `.old` extension (preserving them in git history), and temporary test caller workflows will be deleted.

**Safety**: Minimum 1 week after last production release and at least 2 successful production releases (one per package) before cleanup begins.

## Acceptance Criteria
- [ ] CICDOPT-3001 complete and validated in production (CLI workflow)
- [ ] CICDOPT-3002 complete and validated in production (Maproom-MCP npm workflow)
- [ ] At least 2 successful production releases with new workflows (one per package)
- [ ] Old workflows kept as `.old` for minimum 1 week after last production release
- [ ] Old workflows committed to git with `.old` extension (preserves in git history)
- [ ] Test caller workflows deleted:
  - `.github/workflows/test-reusable-rust.yml`
  - `.github/workflows/test-reusable-typescript.yml`
- [ ] `.github/WORKFLOWS.md` updated to remove references to old workflows
- [ ] No broken references in documentation
- [ ] GitHub Actions UI shows only active workflows
- [ ] Git commit messages document what was archived and why

## Technical Requirements

### Phase 1: Commit Old Workflows (Preserves in Git History)

```bash
# Commit old workflows to preserve in git history
git add .github/workflows/*.old
git commit -m "chore(ci): archive old workflows after successful consolidation

Archived workflows:
- build-and-publish-cli.yml.old (replaced by release-cli.yml)
- build-and-publish-maproom-mcp.yml.old (replaced by release-maproom-mcp.yml)

Both workflows validated in production with multiple successful releases.
Keeping .old files in repository for reference and emergency rollback.
"
```

### Phase 2: Delete Test Caller Workflows

```bash
# Delete temporary test caller workflows
git rm .github/workflows/test-reusable-rust.yml
git rm .github/workflows/test-reusable-typescript.yml

git commit -m "chore(ci): remove temporary test caller workflows

Removed:
- test-reusable-rust.yml (Phase 2 validation complete)
- test-reusable-typescript.yml (Phase 2 validation complete)

Reusable workflows validated and integrated into production release workflows.
Test callers no longer needed.
"
```

### Phase 3: Update Documentation

```bash
# Update workflow documentation
# Edit .github/WORKFLOWS.md:
# - Remove references to old workflows
# - Remove references to test caller workflows
# - Update workflow catalog to show only active workflows
# - Update workflow count and descriptions

git add .github/WORKFLOWS.md
git commit -m "docs(ci): update workflow documentation after consolidation

- Removed references to archived workflows
- Removed references to temporary test callers
- Updated workflow catalog with final consolidated structure
"

# Push all changes
git push
```

## Implementation Notes

### Safety Verification Before Cleanup

Before proceeding with cleanup, verify:

1. **Production Validation**: Both CICDOPT-3001 and CICDOPT-3002 workflows have run successfully in production
2. **Time Gate**: At least 1 week has passed since last production release
3. **Release Count**: At least 2 successful releases total (one CLI, one Maproom-MCP npm)
4. **No Issues**: No open issues or reports of problems with new workflows

### Files to Be Archived (Committed with `.old`)

- `.github/workflows/build-and-publish-cli.yml.old`
- `.github/workflows/build-and-publish-maproom-mcp.yml.old`

### Files to Be Deleted

- `.github/workflows/test-reusable-rust.yml`
- `.github/workflows/test-reusable-typescript.yml`

### Documentation Updates Required

Update `.github/WORKFLOWS.md`:
- Remove old workflow descriptions
- Remove test caller workflow descriptions
- Update workflow count
- Update workflow catalog table
- Ensure all references are to current workflows only

### Verification Steps

1. Check GitHub Actions UI: Should show only active workflows (no `.old` files listed)
2. Search codebase for broken references: `grep -r "build-and-publish-cli.yml" .github/`
3. Search for test caller references: `grep -r "test-reusable-" .github/`
4. Verify git history preservation: `git log -- .github/workflows/build-and-publish-cli.yml.old`

## Dependencies
- **CICDOPT-3001** - CLI workflow must be validated in production
- **CICDOPT-3002** - Maproom-MCP workflow must be validated in production
- **Time Gate** - Minimum 1 week after last production release
- **Release Gate** - Minimum 2 successful releases total (one per package)

## Risk Assessment
- **Risk**: Premature deletion of old workflows before production validation
  - **Mitigation**: Enforce minimum 1 week wait period and 2 successful releases
  - **Mitigation**: Commit `.old` files first (preserves in git history for emergency rollback)

- **Risk**: Broken documentation references after cleanup
  - **Mitigation**: Search entire `.github/` directory for references before deleting
  - **Mitigation**: Update documentation in same commit as deletion

- **Risk**: Confusion about what workflows are active
  - **Mitigation**: Clear commit messages documenting what was archived and why
  - **Mitigation**: Updated workflow documentation shows only active workflows

## Files/Packages Affected
- `.github/workflows/build-and-publish-cli.yml.old` (committed to git)
- `.github/workflows/build-and-publish-maproom-mcp.yml.old` (committed to git)
- `.github/workflows/test-reusable-rust.yml` (deleted)
- `.github/workflows/test-reusable-typescript.yml` (deleted)
- `.github/WORKFLOWS.md` (updated)

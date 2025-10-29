# Ticket: DKRHUB-3002: Create and Push Git Tag v1.1.10

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create annotated git tag v1.1.10 and push to GitHub to trigger the automated Docker image publishing workflow.

## Background
Git tags are the trigger mechanism for the GitHub Actions workflow. Pushing a tag matching the pattern `v*.*.*` will:
1. Trigger the publish-maproom-mcp-image.yml workflow
2. Build multi-platform images
3. Push images to Docker Hub with proper tags
4. Run security scanning

This is the "go button" for the v1.1.10 release.

Reference: DKRHUB_PLAN.md Phase 3, Task DKRHUB-3002 (lines 581-615)

## Acceptance Criteria
- [ ] PR from DKRHUB-3001 merged to main branch (or changes committed directly)
- [ ] Local repository on main branch with latest commits pulled
- [ ] Annotated tag created: `v1.1.10` with descriptive message
- [ ] Tag pushed to origin: `git push origin v1.1.10`
- [ ] GitHub Actions workflow triggered automatically (visible in Actions tab)
- [ ] Workflow run shows "publish-maproom-mcp-image" with trigger "push (tag: v1.1.10)"

## Technical Requirements
**Prerequisites**:
```bash
# 1. Ensure on main branch
git checkout main

# 2. Pull latest changes (including merged PR from DKRHUB-3001)
git pull origin main

# 3. Verify package.json version is 1.1.10
grep '"version"' packages/maproom-mcp/package.json
# Should show: "version": "1.1.10"

# 4. Verify CHANGELOG.md updated
grep -A2 "## \[1.1.10\]" CHANGELOG.md
# Should show version entry
```

**Create Annotated Tag**:
```bash
# Create tag with descriptive message
git tag -a v1.1.10 -m "Release v1.1.10: Fix Docker Hub deployment

- Fix critical v1.1.9 deployment failure
- Add Docker Hub publishing workflow
- Add multi-platform support (AMD64, ARM64)
- Update docker-compose.yml to use pre-built images

This release fixes the broken v1.1.9 where docker-compose.yml tried to build
from source using a build context that doesn't exist in deployed npm packages.
Images are now pre-built and published to Docker Hub."

# Verify tag created
git tag -l -n10 v1.1.10
```

**Push Tag**:
```bash
# Push tag to GitHub (this triggers the workflow)
git push origin v1.1.10

# Verify push succeeded
git ls-remote --tags origin | grep v1.1.10
# Should show: refs/tags/v1.1.10
```

**Monitor Workflow**:
```bash
# Option 1: Web UI
# Navigate to: https://github.com/danielbushman/crewchief/actions
# Look for workflow run with name "Publish Maproom MCP Docker Image"
# Trigger: push (tag: v1.1.10)

# Option 2: GitHub CLI
gh run list --workflow=publish-maproom-mcp-image.yml --limit 1
gh run watch  # Follow the running workflow
```

## Implementation Notes
**Annotated vs Lightweight Tags**:
- Annotated tags (with `-a`): Store tagger, date, message
- Lightweight tags (without `-a`): Just pointer to commit
- Always use annotated for releases (better metadata)

**Tag Protection**:
GitHub repository should have tag protection configured:
- Pattern: `v*.*.*`
- Protected: Yes
- Prevents force push or deletion
- Only maintainers can create

**Workflow Trigger**:
The workflow has this trigger:
```yaml
on:
  push:
    tags:
      - 'v*.*.*'
```
When tag matching pattern is pushed, GitHub:
1. Detects push event
2. Matches tag pattern
3. Starts workflow run
4. Sets GITHUB_REF to `refs/tags/v1.1.10`

**Rollback Plan**:
If workflow fails or issues discovered:
```bash
# Delete remote tag
git push origin :refs/tags/v1.1.10

# Delete local tag
git tag -d v1.1.10

# Fix issues, then recreate tag
```

## Dependencies
- DKRHUB-3001: Version and changelog must be updated first
- DKRHUB-1001 through DKRHUB-1006: Workflow must be fully implemented
- DKRHUB-1901: Workflow should be tested with pre-release tag

## Risk Assessment
- **Risk**: Workflow fails after tag pushed
  - **Mitigation**: Pre-release testing (DKRHUB-1901) validates workflow; can delete tag and retry
- **Risk**: Tag pushed to wrong commit
  - **Mitigation**: Verify HEAD is correct before tagging; can delete and recreate
- **Risk**: Typo in tag name
  - **Mitigation**: Double-check tag name before pushing; follow v{major}.{minor}.{patch} format

## Files/Packages Affected
- None (git metadata only, no file changes)
- Creates git tag: v1.1.10
- Triggers GitHub Actions workflow

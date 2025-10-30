# Ticket: DKRHUB-3006: Create Rollback Procedure

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Document and test comprehensive rollback procedure for reverting to v1.1.9 if v1.1.10 release fails or introduces critical issues. Provides safety net for production deployment.

## Background
When releasing v1.1.10 with Docker Hub images, several failure scenarios could occur:
- Docker Hub images published but broken
- npm package published but doesn't work with images
- One platform (ARM64) works but other (AMD64) fails
- Critical bug discovered after release

Without a rollback plan, users would be stuck with broken version. This ticket creates a tested procedure to quickly revert to v1.1.9 if needed.

Reference: DKRHUB_TICKETS_REVIEW_REPORT.md "Issue #7"

## Acceptance Criteria

### Documentation
- [ ] Rollback procedure documented in `DKRHUB_ROLLBACK.md`
- [ ] Step-by-step instructions for each rollback scenario
- [ ] Commands provided (copy-paste ready)
- [ ] Timeline estimates for each rollback type
- [ ] Contact information for escalation
- [ ] Pre-flight checklist (when to rollback vs. hotfix)

### Rollback Scenarios Covered
- [ ] Scenario 1: npm publish failed (easiest)
- [ ] Scenario 2: npm published, Docker Hub failed (moderate)
- [ ] Scenario 3: Both published, images broken (complex)
- [ ] Scenario 4: Partial platform failure (one arch broken)
- [ ] Scenario 5: Critical bug discovered post-release (emergency)

### Tested Procedures
- [ ] Test rollback simulation in development environment
- [ ] Verify npm unpublish works (within 72 hour window)
- [ ] Verify Docker Hub tag deletion works
- [ ] Document git tag handling (keep or delete?)
- [ ] Test user upgrade path back to v1.1.9

### Automation Scripts
- [ ] Create `scripts/rollback-v1.1.10.sh` with interactive prompts
- [ ] Script validates current state before rollback
- [ ] Script performs rollback operations
- [ ] Script verifies rollback success
- [ ] Script provides user communication template

## Technical Requirements

**File**: `packages/maproom-mcp/DKRHUB_ROLLBACK.md`

```markdown
# Rollback Procedure: v1.1.10 → v1.1.9

## Pre-Flight Checklist

Before initiating rollback, determine if rollback is necessary:

**Rollback if**:
- ❌ npm package installs but fails to start
- ❌ Docker images don't exist on Docker Hub
- ❌ Images exist but containers fail to start
- ❌ Critical security vulnerability discovered
- ❌ Data loss or corruption possible

**Hotfix instead if**:
- ⚠️ Minor bug that doesn't prevent usage
- ⚠️ Documentation error
- ⚠️ Non-critical feature broken
- ⚠️ Performance degradation (not failure)

**Severity Levels**:
- **P0 (Immediate rollback)**: Users cannot use package
- **P1 (Plan rollback)**: Users can use with workaround
- **P2 (Hotfix)**: Minor issues, hotfix preferred

## Rollback Timeline

| Scenario | Estimated Time | Risk Level |
|----------|----------------|------------|
| npm only | 15 minutes | Low |
| Docker Hub only | 30 minutes | Medium |
| Full rollback | 1-2 hours | High |
| Post-72hr npm | Manual process | Very High |

## Scenario 1: npm Publish Failed (Easiest)

**Situation**: Docker images published, but npm publish failed or was interrupted.

**Impact**: No user impact (v1.1.9 still current on npm)

**Steps**:
1. Verify current npm version:
   ```bash
   npm view @crewchief/maproom-mcp version
   ```
   Expected: `1.1.9` (v1.1.10 not published)

2. No rollback needed - proceed with fixing and re-publishing

3. Clean up Docker Hub pre-release tags:
   ```bash
   # Delete pre-release tags from Docker Hub
   # (Manual process via Docker Hub web UI)
   # Tags to delete: 1.1.10-rc1, 1.1.10-rc2, etc.
   ```

## Scenario 2: npm Published, Docker Hub Failed (Moderate)

**Situation**: v1.1.10 published to npm, but Docker Hub images missing or incomplete.

**Impact**: High - Users pull v1.1.10 npm package, images don't exist

**Steps**:
1. **Immediate**: Unpublish v1.1.10 from npm (within 72 hours)
   ```bash
   npm unpublish @crewchief/maproom-mcp@1.1.10
   ```

2. Verify unpublish succeeded:
   ```bash
   npm view @crewchief/maproom-mcp version
   ```
   Expected: `1.1.9`

3. Announce rollback:
   ```bash
   # Post to GitHub Releases
   # Title: "v1.1.10 Rolled Back - Please Use v1.1.9"
   # Body: See template below
   ```

4. Fix Docker Hub publishing issues

5. Re-test DKRHUB-1901 through DKRHUB-2904

6. Republish as v1.1.11 (skip v1.1.10)

## Scenario 3: Both Published, Images Broken (Complex)

**Situation**: v1.1.10 on npm, images on Docker Hub, but images don't work.

**Impact**: Critical - Users pull package and images, nothing works

**Steps**:
1. **Immediate**: Assess if within 72-hour npm unpublish window
   ```bash
   npm view @crewchief/maproom-mcp time
   ```

2. **If within 72 hours**: Unpublish npm package
   ```bash
   npm unpublish @crewchief/maproom-mcp@1.1.10
   ```

3. **If after 72 hours**: Publish emergency v1.1.11 that reverts changes
   ```bash
   # Revert docker-compose.yml to build from source
   # OR publish working Docker images as v1.1.11
   # Update package.json to 1.1.11
   npm publish --access public
   ```

4. Delete or retag broken Docker Hub images:
   ```bash
   # Option A: Delete tags (Docker Hub web UI)
   #   - Delete: 1.1.10, latest
   #   - Keep: 1.1.9, 1.1

   # Option B: Retag 1.1.9 as latest
   docker pull crewchief/maproom-mcp:1.1.9
   docker tag crewchief/maproom-mcp:1.1.9 crewchief/maproom-mcp:latest
   docker push crewchief/maproom-mcp:latest
   ```

5. Update GitHub Release:
   ```bash
   # Mark v1.1.10 release as "Pre-release" (not latest)
   # Create new v1.1.11 release as "Latest"
   ```

6. Communicate to users (see template below)

## Scenario 4: Partial Platform Failure (Complex)

**Situation**: AMD64 images work, but ARM64 images broken (or vice versa).

**Impact**: High - Users on one platform cannot use package

**Steps**:
1. Immediately add warning to README:
   ```markdown
   ⚠️ **Known Issue**: v1.1.10 ARM64 images are broken.
   AMD64 users can proceed. ARM64 users should use v1.1.9:

   `MAPROOM_VERSION=1.1.9 npx @crewchief/maproom-mcp start`
   ```

2. Decide on approach:
   - **Option A**: Full rollback (safest)
   - **Option B**: Hotfix ARM64 images as v1.1.10-hotfix1
   - **Option C**: Publish v1.1.11 with fix quickly

3. If hotfix approach:
   ```bash
   # Rebuild and push ARM64-specific image
   docker buildx build \
     --platform linux/arm64 \
     -f packages/maproom-mcp/config/Dockerfile.combined \
     -t crewchief/maproom-mcp:1.1.10 \
     --push \
     .
   ```

4. Test ARM64 fix thoroughly before clearing warning

## Scenario 5: Critical Bug Discovered (Emergency)

**Situation**: Security vulnerability, data loss, or critical functionality broken.

**Impact**: Critical - Immediate action required

**Steps**:
1. **Immediately**: Add deprecation warning to npm
   ```bash
   npm deprecate @crewchief/maproom-mcp@1.1.10 "CRITICAL: Security issue. Use v1.1.9 or wait for v1.1.11"
   ```

2. Follow Scenario 3 (full rollback) steps

3. File CVE if security-related

4. Communicate urgency to users via all channels:
   - GitHub Security Advisory
   - npm deprecation message
   - GitHub Releases page
   - README.md banner
   - Social media (if applicable)

## Rollback Automation Script

**File**: `scripts/rollback-v1.1.10.sh`

```bash
#!/bin/bash
set -e

echo "=== v1.1.10 Rollback Script ==="
echo ""
echo "This script helps roll back v1.1.10 to v1.1.9"
echo ""

# Verify current published version
echo "Step 1: Checking current npm version..."
CURRENT_VERSION=$(npm view @crewchief/maproom-mcp version)
echo "Current version: $CURRENT_VERSION"

if [ "$CURRENT_VERSION" != "1.1.10" ]; then
  echo "❌ Current version is not 1.1.10, rollback may not be needed"
  read -p "Continue anyway? (y/N) " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
  fi
fi

# Check 72-hour window
echo ""
echo "Step 2: Checking npm publish time..."
npm view @crewchief/maproom-mcp time --json

echo ""
read -p "Is v1.1.10 within 72 hours of publish? (y/N) " -n 1 -r
echo
WITHIN_72HR=$REPLY

# Unpublish if possible
if [[ $WITHIN_72HR =~ ^[Yy]$ ]]; then
  echo ""
  echo "Step 3: Unpublishing v1.1.10 from npm..."
  read -p "Proceed with npm unpublish? (y/N) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    npm unpublish @crewchief/maproom-mcp@1.1.10
    echo "✅ Unpublished v1.1.10"
  fi
else
  echo ""
  echo "⚠️  Beyond 72-hour window. npm unpublish not available."
  echo "You must publish v1.1.11 to supersede v1.1.10"
  read -p "Create v1.1.11 revert? (y/N) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Manual steps required:"
    echo "1. Revert docker-compose.yml changes"
    echo "2. Update package.json version to 1.1.11"
    echo "3. Run: npm publish --access public"
  fi
fi

# Docker Hub cleanup
echo ""
echo "Step 4: Docker Hub cleanup..."
echo "Manual steps required:"
echo "1. Visit: https://hub.docker.com/r/crewchief/maproom-mcp/tags"
echo "2. Delete tags: 1.1.10, 1.1.10-rc1, 1.1.10-rc2"
echo "3. Verify latest tag points to 1.1.9"

# Git tag handling
echo ""
echo "Step 5: Git tag handling..."
read -p "Delete git tag v1.1.10? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  git tag -d v1.1.10
  git push origin :refs/tags/v1.1.10
  echo "✅ Deleted git tag v1.1.10"
fi

# Communication
echo ""
echo "Step 6: User communication..."
echo "Template for GitHub Release / README:"
echo ""
cat << 'TEMPLATE'
# ⚠️ v1.1.10 Rollback Notice

**v1.1.10 has been rolled back due to critical issues.**

## Action Required

If you installed v1.1.10:
1. Uninstall: `npm uninstall -g @crewchief/maproom-mcp`
2. Reinstall v1.1.9: `npm install -g @crewchief/maproom-mcp@1.1.9`
3. Restart services: `maproom-mcp restart`

## What Happened

[Describe the issue]

## Next Steps

We are working on v1.1.11 with fixes. Expected release: [DATE]

## Questions

Please file issues at: https://github.com/danielbushman/crewchief/issues
TEMPLATE

echo ""
echo "✅ Rollback checklist complete!"
echo ""
echo "Don't forget to:"
echo "- Update README.md with warning"
echo "- Post GitHub Release"
echo "- Monitor for user reports"
```

## Verification Steps

Test rollback procedure in development:

1. **Simulate v1.1.10 publish**:
   ```bash
   # Create test npm package
   npm pack
   # Don't actually publish to production
   ```

2. **Test unpublish timing**:
   ```bash
   # Verify npm shows publish time
   npm view @crewchief/maproom-mcp@1.1.9 time
   ```

3. **Test Docker Hub tag operations**:
   ```bash
   # Use test Docker Hub repository
   # Practice deleting and retagging
   ```

4. **Test git tag operations**:
   ```bash
   # Create test tag
   git tag v1.1.10-test
   # Delete test tag
   git tag -d v1.1.10-test
   ```

5. **Review communication template**:
   - Ensure clear, actionable instructions
   - Include all necessary commands
   - Provide support contact info

## Post-Rollback Actions

After successful rollback:

1. **Root Cause Analysis**:
   - Document what went wrong
   - Update tickets to prevent recurrence
   - Add safeguards to workflow

2. **User Support**:
   - Monitor GitHub issues
   - Respond to user reports
   - Update FAQ with common questions

3. **Next Release Planning**:
   - Fix underlying issues
   - Enhanced testing before next attempt
   - Consider longer pre-release period

## Communication Template

**GitHub Release / README Banner**:
```markdown
# ⚠️ v1.1.10 Known Issues - Use v1.1.9

**Status**: v1.1.10 has critical issues. Please use v1.1.9.

## Affected Versions
- ❌ v1.1.10 (broken)
- ✅ v1.1.9 (stable, recommended)

## Install v1.1.9
```bash
npm install -g @crewchief/maproom-mcp@1.1.9
```

## What's Wrong
[Brief description of issue]

## Timeline
- [DATE]: v1.1.10 released
- [DATE]: Issues reported
- [DATE]: v1.1.10 rolled back
- [DATE]: v1.1.11 planned (with fixes)

## Support
Questions? File an issue: https://github.com/danielbushman/crewchief/issues
```

## Dependencies
- Completion of DKRHUB-3005 (triggers need for rollback if issues occur)
- Understanding of npm unpublish policies (72-hour window)
- Docker Hub account access for tag management
- GitHub repository admin access for releases

## Estimated Effort
1.5-2 hours (includes documentation, script creation, and testing)

## Related Issues
- Addresses: DKRHUB_TICKETS_REVIEW_REPORT.md "Issue #7"
- Safety net for: DKRHUB-3001 through DKRHUB-3005 (release process)
- Complements: DKRHUB-2904 (pre-release validation reduces rollback need)
```

## Implementation Notes

**When to Create This Ticket**:
- Before starting Phase 3 (Release)
- Ideally during Phase 2 (Docker Compose updates)
- Must be complete before DKRHUB-3005 (npm publish)

**Testing Rollback**:
- Use test npm scope or secondary package
- Use test Docker Hub repository
- Practice on staging environment
- Time each rollback scenario

**Legal/Policy Considerations**:
- npm unpublish policy: 72-hour window, <10% adoption
- Docker Hub doesn't have unpublish, only delete tags
- Git tags can be deleted but discouraged (history preservation)
- Semantic versioning: Don't reuse version numbers

## Files/Packages Affected
- NEW: `packages/maproom-mcp/DKRHUB_ROLLBACK.md`
- NEW: `scripts/rollback-v1.1.10.sh`
- POTENTIAL: `packages/maproom-mcp/README.md` (add warning if needed)

## Estimated Effort
1.5-2 hours (documentation + script + testing simulation)

## Related Issues
- Fixes: DKRHUB_TICKETS_REVIEW_REPORT.md "Issue #7"
- Safety net for: Phase 3 release process
- Prerequisites: Understanding of npm/Docker Hub policies

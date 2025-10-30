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

## Escalation Contacts

For rollback assistance and emergency support:

**Primary Contact:**
- **GitHub Issues**: https://github.com/danielbushman/crewchief/issues
- **Use for**: Non-urgent questions, bug reports, feature requests
- **Response time**: Within 24-48 hours

**Maintainer Contact:**
- **Project Maintainer**: @danielbushman (GitHub)
- **Use for**: Urgent rollback decisions, production issues
- **How to reach**: Tag in GitHub issue or PR with `@danielbushman`

**Emergency Rollback Decision Tree:**

**P0 (Immediate - 0-2 hours):**
- Users cannot use package at all
- Security vulnerability discovered
- **Action**: Follow rollback procedure immediately, file GitHub issue with `[CRITICAL]` prefix
- **Authority**: Any maintainer can approve immediate rollback
- **Communication**: Post GitHub issue + update README.md banner immediately

**P1 (Urgent - 2-24 hours):**
- Users can use with workaround
- Single platform (ARM64 or AMD64) broken
- **Action**: File GitHub issue with `[URGENT]` prefix, await maintainer input
- **Authority**: Maintainer approval required before rollback
- **Communication**: GitHub issue + consider README.md warning

**P2 (Normal - 24-72 hours):**
- Minor issues, hotfix preferred over rollback
- **Action**: File GitHub issue, propose hotfix approach
- **Authority**: Maintainer review and approval required
- **Communication**: GitHub issue only

**After-Hours Emergency Protocol:**
If critical P0 issue occurs outside business hours:
1. File GitHub issue with `[CRITICAL]` prefix and detailed description
2. Follow rollback procedure if you have npm publish permissions
3. Update README.md with warning banner
4. Monitor GitHub notifications for maintainer response

**Community Support:**
- **Discussion**: GitHub Discussions (https://github.com/danielbushman/crewchief/discussions)
- **Quick Questions**: GitHub Issues (tag with `question` label)

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

See: `scripts/rollback-v1.1.10.sh`

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

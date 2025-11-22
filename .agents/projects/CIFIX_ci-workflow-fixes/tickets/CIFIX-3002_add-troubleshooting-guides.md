# Ticket: CIFIX-3002: Add troubleshooting guides

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-implementation-agent
- verify-ticket
- commit-ticket

## Summary
Create dedicated troubleshooting guides in `.github/CLAUDE.md` with step-by-step debugging procedures for the most common CI failure scenarios.

## Background
While CIFIX-3001 documents issues and quick fixes, this ticket adds detailed troubleshooting procedures for:
1. Debugging test workflow failures step-by-step
2. Debugging Docker build failures with context
3. Verifying the entire CI pipeline health
4. Emergency rollback procedures

This provides developers with a systematic approach to diagnosing and fixing issues rather than just quick fixes.

**Reference**: This ticket implements the documentation portion of Phase 3 in the CIFIX project plan, providing comprehensive troubleshooting procedures beyond the basic documentation in CIFIX-3001.

## Acceptance Criteria
- [ ] "Common CI Issues" section added to `.github/CLAUDE.md`
- [ ] Step-by-step procedure for diagnosing test workflow failures
- [ ] Step-by-step procedure for diagnosing Docker build failures
- [ ] CI health check commands documented
- [ ] Rollback procedures documented for both fixes
- [ ] Each procedure includes validation commands

## Technical Requirements

### File to Modify
- **File**: `.github/CLAUDE.md`
- **Location**: Append "Common CI Issues" section after existing Troubleshooting section
- **Format**: Numbered procedures with code examples in markdown

### Content Structure
The new section must include:
1. **Debugging Test Workflow Failures** - 5-step diagnosis procedure
2. **Debugging Docker Build Failures** - 5-step diagnosis procedure
3. **CI Health Check** - Comprehensive validation commands
4. **Emergency Rollback Procedures** - For both test workflow and Docker builds

### Code Examples Required
Each procedure must include:
- Commands to check workflow/build status
- Validation commands with expected output
- Rollback git commands with examples
- Health check scripts

## Implementation Notes

Add the following section to `.github/CLAUDE.md` after the Troubleshooting section:

```markdown
## Common CI Issues

### Debugging Test Workflow Failures

**Step-by-step diagnosis:**

1. **Check workflow logs** in GitHub Actions:
   ```bash
   gh run list --workflow=test.yml --limit 5
   gh run view <run-id> --log
   ```

2. **Verify pnpm setup step**:
   - Look for "Setup pnpm" in logs
   - Check detected version matches package.json
   - Expected: `pnpm version 10.12.1`

3. **Verify packageManager field**:
   ```bash
   jq -r '.packageManager' package.json
   # Should show: pnpm@10.12.1+sha512...
   ```

4. **Check for explicit version in workflow**:
   ```bash
   grep -A 5 "pnpm/action-setup" .github/workflows/test.yml
   # Should NOT see: with: version:
   ```

5. **Rollback if needed**:
   ```bash
   git log --oneline .github/workflows/test.yml
   git revert <commit-sha>
   git push
   ```

---

### Debugging Docker Build Failures

**Step-by-step diagnosis:**

1. **Verify daemon-client dist/ exists**:
   ```bash
   ls -la packages/daemon-client/dist/
   # Must show: index.js, index.d.ts, client.js, client.d.ts
   ```

2. **Check pnpm version sync**:
   ```bash
   PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
   DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')

   if [ "$PACKAGE_PNPM" != "$DOCKERFILE_PNPM" ]; then
     echo "❌ Version mismatch: $PACKAGE_PNPM vs $DOCKERFILE_PNPM"
   else
     echo "✅ Versions match: $PACKAGE_PNPM"
   fi
   ```

3. **Test local Docker build**:
   ```bash
   pnpm build  # Ensure daemon-client built

   docker build \
     -f packages/maproom-mcp/config/Dockerfile.combined \
     -t maproom-mcp:debug \
     --progress=plain \
     .
   ```

4. **Check for common errors**:
   - "EUNSUPPORTEDPROTOCOL" → pnpm not installed in Dockerfile
   - "COPY failed" → daemon-client dist/ missing (run pnpm build)
   - "workspace: not resolved" → Missing pnpm-workspace.yaml in COPY

5. **Rollback Docker changes**:
   ```bash
   git log --oneline packages/maproom-mcp/config/Dockerfile.combined
   git revert <commit-sha>
   docker build -f packages/maproom-mcp/config/Dockerfile.combined -t rollback .
   ```

---

### CI Health Check

Run these commands to verify CI configuration is correct:

```bash
# Test workflow health
yamllint .github/workflows/test.yml
jq -r '.packageManager' package.json
grep -c "with: version:" .github/workflows/test.yml  # Should be 0

# Docker build health
pnpm build
ls -la packages/daemon-client/dist/ | wc -l  # Should show multiple files
grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined

# Release workflow health
grep -A 10 "pnpm build" .github/workflows/publish-maproom-mcp-image.yml
# Should show pnpm build step before Docker build

echo "✅ All health checks passed"
```

---

### Emergency Rollback Procedures

**If test workflow broken:**
```bash
# Option 1: Revert to previous workflow
git revert <commit-sha-of-fix>
git push

# Option 2: Temporarily add explicit version (not recommended long-term)
# Edit .github/workflows/test.yml:
# - name: Setup pnpm
#   uses: pnpm/action-setup@v4
#   with:
#     version: 10  # Temporary fix while debugging
```

**If Docker build broken:**
```bash
# Revert Dockerfile changes
git revert <commit-sha-of-dockerfile-changes>
git push

# If release is urgent, manually publish previous image:
docker pull <previous-good-image>
docker tag <previous-good-image> <registry>:<new-tag>
docker push <registry>:<new-tag>
```

**Validation after rollback:**
- Test workflow: Trigger manual run in GitHub Actions
- Docker build: Run local build and verify success
- Release workflow: Create test tag and monitor build
```

### Validation Commands

After implementation, verify:
```bash
# Verify section added
grep "## Common CI Issues" .github/CLAUDE.md

# Verify debugging procedures exist
grep -A 5 "Step-by-step diagnosis" .github/CLAUDE.md

# Verify rollback procedures documented
grep -A 5 "Emergency Rollback" .github/CLAUDE.md
```

## Dependencies
- **Requires**: CIFIX-3001 (builds on troubleshooting section in `.github/CLAUDE.md`)
- **Blocks**: None

## Risk Assessment
- **Risk**: None (documentation-only change)
  - **Mitigation**: N/A

## Files/Packages Affected
- `.github/CLAUDE.md` - Add "Common CI Issues" section with troubleshooting procedures

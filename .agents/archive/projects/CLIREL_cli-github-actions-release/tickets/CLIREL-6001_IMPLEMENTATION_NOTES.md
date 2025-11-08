# CLIREL-6001 Implementation Notes

## Ticket: Implement Security Baseline

**Status**: Partially Complete - Files created, manual configuration required
**Date**: 2025-11-08

## What Was Done (Automated)

### 1. Created SECURITY.md
**File**: `/workspace/SECURITY.md`

Security policy documenting:
- Supported versions (latest only for both packages)
- Vulnerability reporting process (security@danielbushman.com)
- Response timeline commitments (48h acknowledgment, 1 week fix timeline)
- Security measures in place
- Scope (in-scope and out-of-scope items)

### 2. Created CODEOWNERS
**File**: `/workspace/.github/CODEOWNERS`

Code ownership rules for:
- `/.github/workflows/` - Requires @danielbushman review
- `/packages/cli/scripts/release.mjs` - Requires @danielbushman review
- `/packages/maproom-mcp/scripts/release.js` - Requires @danielbushman review
- `/packages/cli/package.json` - Requires @danielbushman review
- `/packages/maproom-mcp/package.json` - Requires @danielbushman review

**Effect**: Combined with branch protection, ensures all changes to critical files get reviewed before merging.

## What Requires Manual Configuration

The following steps require manual GitHub repository settings configuration and cannot be automated:

### 1. Tag Protection Rules

**Navigate to**: Repository Settings → Tags → Protected tags → Add rule

**Create Two Rules**:

#### Rule 1: CLI Tags
- Pattern: `@crewchief/cli@v*`
- Who can create: Maintainers only (admin/write access)
- Prevent tag deletion: Yes
- Require signed commits: No

#### Rule 2: MCP Tags
- Pattern: `@crewchief/maproom-mcp@v*`
- Who can create: Maintainers only (admin/write access)
- Prevent tag deletion: Yes
- Require signed commits: No

**Verification**:
```bash
# As non-maintainer, try creating tag (should fail)
git tag @crewchief/cli@v99.99.99
git push origin @crewchief/cli@v99.99.99
# Expected: Permission denied
```

### 2. Branch Protection Rules

**Navigate to**: Repository Settings → Branches → Add rule

**Branch name pattern**: `main`

**Enable these protection rules**:
- ✅ Require a pull request before merging
  - Required approvals: 1
  - Dismiss stale reviews: Yes
  - Require review from Code Owners: Yes
- ✅ Require status checks to pass (if any exist)
  - Require branches to be up to date: No
- ✅ Do not allow bypassing the above settings
- ✅ Include administrators (even admins need reviews)
- ❌ Require signed commits (not MVP - too restrictive)
- ❌ Require linear history (not necessary)

**Verification**:
```bash
# Try direct push to main (should fail)
git checkout main
echo "test" >> README.md
git commit -am "test: direct push"
git push
# Expected: Permission denied, must use PR
```

### 3. NPM_TOKEN Secret Configuration

**Step 1: Get token from npm**
1. Go to https://npmjs.com → Account Settings → Access Tokens
2. Create new token:
   - Type: **Automation** (allows CI/CD publish)
   - Expiration: No expiration (or 1 year, then rotate)
   - Scope: **Publish** (write access)
3. Copy token (only shown once!)

**Step 2: Add to GitHub**
1. Go to Repository Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `NPM_TOKEN`
4. Value: Paste the token from npm
5. Click "Add secret"

**Verification**:
```bash
# Check secret exists (requires admin access)
gh secret list
# Should show: NPM_TOKEN

# Test in workflow
# The workflows use: ${{ secrets.NPM_TOKEN }}
# GitHub automatically masks these values in logs
```

**Security notes**:
- ⚠️ Never commit token to git
- ⚠️ Never log token in workflow output
- 🔄 Rotate immediately if exposed
- ✅ GitHub auto-masks secrets in logs

### 4. npm Account 2FA

**Enable 2FA on npm account** (https://npmjs.com):
1. Log in to npmjs.com
2. Account Settings → Two-Factor Authentication
3. Enable "Authorization and Publishing"
4. Use authenticator app (Authy, Google Authenticator, 1Password)
5. Save backup codes in secure location (password manager)

**Important**: Automation tokens bypass 2FA for publishing (this is intended behavior for CI/CD).

**Document in project**:
- ✅ SECURITY.md mentions npm account has 2FA enabled
- ✅ Store backup codes securely
- ✅ Document recovery process internally

## Pre-Release Security Checklist

Complete this checklist before first production release:

- [ ] **Tag protection enabled** - Create tag protection rules in GitHub
- [ ] **Branch protection enabled** - Configure main branch protection
- [ ] **NPM_TOKEN secret configured** - Add npm automation token to GitHub secrets
- [ ] **SECURITY.md published** - ✅ Created (commit this file)
- [ ] **CODEOWNERS enforced** - ✅ Created (commit this file) + enable branch protection
- [ ] **npm 2FA enabled** - Enable on npmjs.com account
- [ ] **First dry-run successful** - Test with workflow_dispatch dry_run=true
- [ ] **No secrets in git history** - Audit git log for accidentally committed tokens
- [ ] **Workflow permissions minimal** - Workflows use GitHub Actions secrets correctly

## Testing Branch Protection (After Configuration)

```bash
# 1. Create test branch
git checkout -b test-branch-protection

# 2. Make trivial change
echo "# Test" >> README.md
git commit -am "test: branch protection verification"
git push origin test-branch-protection

# 3. Create PR
gh pr create --title "Test branch protection" --body "Testing that branch protection requires approval"

# 4. Try to merge without approval (should fail)
gh pr merge --auto
# Expected: "Review required" or "Requires 1 approval"

# 5. Get approval (from another maintainer or use admin override if testing solo)
# Then merge
gh pr merge --squash
# Expected: Success

# 6. Cleanup
git checkout main
git pull
git branch -d test-branch-protection
```

## Security Measures Summary

### Implemented (Automated)
✅ SECURITY.md - Vulnerability reporting process documented
✅ CODEOWNERS - Critical files require code review
✅ Binary validation - Workflows validate binaries before publish
✅ Secret management - Workflows use GitHub secrets (NPM_TOKEN)
✅ Minimal permissions - Workflows use principle of least privilege

### Requires Manual Configuration
⏳ Tag protection - Prevents unauthorized releases
⏳ Branch protection - Requires code review before merge
⏳ NPM_TOKEN secret - Needs to be added to GitHub
⏳ npm 2FA - Needs to be enabled on npm account

### Not in MVP (Future Enhancements)
❌ Signed commits - Too restrictive for MVP
❌ Binary signing - Enterprise-grade, not necessary yet
❌ SBOM generation - Can add later
❌ 2FA enforcement automation - Manual verification sufficient

## Next Steps

1. **User must manually configure**:
   - Tag protection rules (2 rules: CLI and MCP)
   - Branch protection on main branch
   - NPM_TOKEN GitHub secret
   - npm account 2FA

2. **After manual configuration**:
   - Run pre-release security checklist
   - Test tag protection with synthetic tag
   - Test branch protection with test PR
   - Verify workflows can access NPM_TOKEN

3. **Before first production release**:
   - Complete all items in security checklist
   - Run CLIREL-7001 (dry-run validation)
   - Document any security exceptions

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| NPM_TOKEN leaked | Low | High | Auto-masked in logs, rotate if exposed |
| Unauthorized tag created | Medium | High | Tag protection prevents non-maintainers |
| Malicious workflow change | Low | Critical | CODEOWNERS + branch protection |
| npm account compromised | Low | Critical | 2FA enabled, monitor login activity |
| Direct push to main | Medium | Medium | Branch protection prevents |

## Notes

- Files created can be committed immediately via PR (tests branch protection)
- Manual configuration should be done before CLIREL-7001 (dry-run)
- Tag protection is most critical (prevents unauthorized releases)
- Branch protection ensures CODEOWNERS is enforced
- This is MVP security - sufficient for launch, not enterprise-grade

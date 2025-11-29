# Ticket: CLIREL-6001: Implement Security Baseline

## Status
- [x] **Task completed** - acceptance criteria met (files created, manual config documented)
- [x] **Tests pass** - related tests pass (N/A - documentation and config files)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Implement security baseline for production releases including repository protection rules, secret management, vulnerability reporting process, and workflow access controls. This establishes the minimum security posture required before first production release.

## Background

### Security Risks
Automated publishing introduces several risks:
1. **Compromised npm token**: Attacker publishes malicious package
2. **Unauthorized releases**: Non-maintainer creates release tag
3. **Workflow modification**: Attacker bypasses validation
4. **No vulnerability reporting process**: Security issues reported publicly

### MVP Security Baseline
This ticket implements pragmatic protections for MVP launch:
- Tag protection (who can create tags)
- Branch protection (code review required)
- Secret management (NPM_TOKEN security)
- Vulnerability reporting (SECURITY.md)
- Workflow protection (CODEOWNERS)

**NOT in MVP** (enterprise-grade, but overkill for now):
- Signed commits
- Binary signing
- SBOM generation
- 2FA enforcement automation

## Acceptance Criteria
- [ ] Tag protection enabled for `@crewchief/*@v*` pattern ⏳ REQUIRES MANUAL CONFIG
- [ ] Branch protection enabled on main (require 1 review) ⏳ REQUIRES MANUAL CONFIG
- [ ] NPM_TOKEN configured as repository secret ⏳ REQUIRES MANUAL CONFIG
- [x] SECURITY.md created in repository root ✅ COMPLETED
- [x] CODEOWNERS file created for `.github/workflows/` ✅ COMPLETED
- [ ] npm account has 2FA enabled (documented) ⏳ REQUIRES MANUAL CONFIG
- [x] Security checklist completed and documented ✅ COMPLETED (see IMPLEMENTATION_NOTES.md)

## Technical Requirements

### 1. Tag Protection Rules

**Navigate to**: Repository Settings → Tags → Protected tags

**Rule 1: CLI Tags**
- Pattern: `@crewchief/cli@v*`
- Who can create: Maintainers only (admin/write access)
- Prevent tag deletion: Yes
- Require signed commits: No (too restrictive for MVP)

**Rule 2: MCP Tags**
- Pattern: `@crewchief/maproom-mcp@v*`
- Who can create: Maintainers only
- Prevent tag deletion: Yes

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

**Protection rules**:
- ✅ Require a pull request before merging
  - Required approvals: 1
  - Dismiss stale reviews: Yes
  - Require review from Code Owners: Yes
- ✅ Require status checks to pass
  - Require branches to be up to date: No (too strict)
- ✅ Do not allow bypassing the above settings
- ❌ Require signed commits (not MVP)
- ❌ Require linear history (not necessary)
- ✅ Include administrators (even admins need reviews)

**Verification**:
```bash
# Try direct push to main (should fail)
git checkout main
git commit --allow-empty -m "test"
git push
# Expected: Permission denied, must use PR
```

### 3. NPM_TOKEN Secret Configuration

**Get token from npm**:
1. Go to npmjs.com → Account Settings → Access Tokens
2. Create new token:
   - Type: Automation
   - Expiration: No expiration (or 1 year)
   - Scope: Publish (write access)
3. Copy token (only shown once!)

**Add to GitHub**:
1. Repository Settings → Secrets and variables → Actions
2. New repository secret
3. Name: `NPM_TOKEN`
4. Value: `<paste token>`
5. Add secret

**Verification**:
```bash
# Check secret exists (requires admin access)
gh secret list
# Should show: NPM_TOKEN

# Workflow can access it
# ${{ secrets.NPM_TOKEN }} in YAML should resolve
```

**Security notes**:
- Never commit token to git
- Never log token in workflow
- Rotate if exposed
- GitHub auto-masks secrets in logs

### 4. Create SECURITY.md

**File**: `/workspace/SECURITY.md`

**Content**:
```markdown
# Security Policy

## Supported Versions

Only the latest version receives security updates:

| Package | Supported |
|---------|-----------|
| @crewchief/cli | Latest only |
| @crewchief/maproom-mcp | Latest only |

## Reporting a Vulnerability

**Please do NOT open public issues for security vulnerabilities.**

Instead, report vulnerabilities by emailing: security@example.com

We will respond within 48 hours and provide a fix timeline.

### What to include in your report:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Our commitment:
- We will acknowledge receipt within 48 hours
- We will provide a fix timeline within 1 week
- We will credit you in the security advisory (unless you prefer anonymity)

## Security Measures

### Build and Release Security
- npm packages published from GitHub Actions only
- Binaries built on GitHub-hosted runners
- NPM_TOKEN stored as encrypted GitHub secret
- Tag protection prevents unauthorized releases
- Binary validation before publish

### Dependency Security
- pnpm lock file ensures reproducible builds
- Regular dependency updates
- Manual cargo audit during releases

## Scope

In scope:
- crewchief CLI tool
- maproom-mcp MCP server
- GitHub Actions workflows
- Build and release automation

Out of scope:
- Third-party dependencies (report to upstream)
- Social engineering attacks
- Physical security
```

**Customize**:
- Replace `security@example.com` with actual security contact
- Add specific vulnerability categories if relevant
- Update response timeline commitments if different

### 5. Create CODEOWNERS File

**File**: `/workspace/.github/CODEOWNERS`

**Content**:
```
# CODEOWNERS file for required reviews

# GitHub Actions workflows require security review
/.github/workflows/ @OWNER-USERNAME

# Release scripts require review
/packages/cli/scripts/release.mjs @OWNER-USERNAME
/packages/maproom-mcp/scripts/release.js @OWNER-USERNAME

# Package.json changes require review (prevent accidental renames)
/packages/cli/package.json @OWNER-USERNAME
/packages/maproom-mcp/package.json @OWNER-USERNAME
```

**Customize**:
- Replace `@OWNER-USERNAME` with actual GitHub usernames
- Can use team names: `@org/team-name`
- Add other critical files as needed

**Effect**:
- PRs touching these files automatically request review from owners
- Combined with branch protection, ensures reviews happen
- Prevents accidental workflow changes

### 6. npm Account 2FA

**Enable 2FA on npm account** (npmjs.com):
1. Account Settings → Two-Factor Authentication
2. Enable "Authorization and Publishing"
3. Use authenticator app (Authy, Google Authenticator)
4. Save backup codes

**Document in project**:
- Add note to README: "npm account has 2FA enabled"
- Store backup codes securely (password manager)
- Document recovery process

**Note**: Automation tokens bypass 2FA for publishing (intended for CI/CD)

### 7. Security Checklist

**Create checklist** (can be in ticket or separate doc):
```markdown
## Pre-Release Security Checklist

- [ ] Tag protection enabled
- [ ] Branch protection enabled
- [ ] NPM_TOKEN secret configured
- [ ] SECURITY.md published
- [ ] CODEOWNERS enforced
- [ ] npm 2FA enabled
- [ ] First dry-run successful
- [ ] No secrets in git history
- [ ] Workflow permissions minimal
```

## Implementation Notes

### Order of Operations
1. Enable tag protection
2. Enable branch protection
3. Configure NPM_TOKEN secret
4. Create SECURITY.md (commit via PR to test branch protection)
5. Create CODEOWNERS (commit via PR)
6. Enable npm 2FA
7. Complete security checklist
8. Document completion in ticket

### Testing Branch Protection
```bash
# Create test branch
git checkout -b test-branch-protection

# Make trivial change
echo "test" >> README.md
git commit -am "test: branch protection"
git push origin test-branch-protection

# Create PR
gh pr create --title "Test branch protection" --body "Testing"

# Try to merge without approval (should fail)
gh pr merge --auto
# Expected: Requires 1 approval

# Get approval, then merge
# Expected: Success
```

### Verifying Tag Protection
- Use non-admin account to test tag creation
- Or temporarily remove yourself from maintainers
- Verify permission denied error

## Dependencies
- None (can implement immediately)
- Should complete before CLIREL-7001 (dry-run testing)

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| NPM_TOKEN exposed | Critical | GitHub auto-masks in logs, never commit to git |
| Tag protection too strict | Low | Maintainers can still create tags normally |
| Branch protection blocks urgent fixes | Medium | Include administrators in rule (can override if needed) |
| CODEOWNERS slows development | Low | Only affects critical files, reviews are quick |

## Files/Packages Affected
- Repository settings (tag protection, branch protection)
- Repository secrets (NPM_TOKEN)
- `/workspace/SECURITY.md` (create)
- `/workspace/.github/CODEOWNERS` (create)

## Success Metrics
- Non-maintainers cannot create release tags
- Direct commits to main are blocked
- Workflows can access NPM_TOKEN
- SECURITY.md published and visible
- PRs to workflows request CODEOWNERS review
- npm account shows 2FA enabled
- Security checklist 100% complete

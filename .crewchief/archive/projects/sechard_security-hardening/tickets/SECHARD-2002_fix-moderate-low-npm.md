# SECHARD-2002: Fix Moderate/Low npm Vulnerabilities

**Status:** Completed  
**Phase:** 2 (Execution)  
**Estimated Effort:** 30 minutes  
**Priority:** High  

---

## Summary

Fix moderate and low severity npm vulnerabilities, including prototype pollution in js-yaml and symlink issues in tmp package.

---

## Background

**Current State:**
`pnpm audit` reports 6 moderate/low vulnerabilities:
- **3 Moderate** - js-yaml prototype pollution
- **3 Low** - tmp symlink write, vite issues (if not already fixed in SECHARD-2001)

**Impact:**
- **js-yaml:** Prototype pollution can lead to property injection, DoS, or security bypass
- **tmp:** Arbitrary file write via symlink manipulation

**Affected Packages:**
```
Moderate:
- js-yaml@<4.1.1 (via packages__cli>eslint>js-yaml)

Low:
- tmp@≤0.2.3 (via packages__cli>inquirer>external-editor>tmp)
```

---

## Acceptance Criteria

1. ✅ All moderate vulnerabilities resolved
2. ✅ All low vulnerabilities resolved
3. ✅ `pnpm audit` shows 0 vulnerabilities (or only accepted risks)
4. ✅ ESLint still works correctly
5. ✅ Interactive CLI prompts still work (if using inquirer)
6. ✅ All tests pass

---

## Technical Requirements

### Vulnerability 1: js-yaml - Prototype Pollution (MODERATE)

**CVE:** CVE-2025-64718  
**GHSA:** GHSA-mh29-5h37-fv8m  
**Severity:** Moderate (CVSS 5.3)

**Description:**
Prototype pollution vulnerability in js-yaml's merge (`<<`) operator allows attackers to modify object prototypes via `__proto__` in untrusted YAML documents.

**Attack Scenario:**
```yaml
# Malicious YAML
__proto__:
  polluted: true
  
# After parsing, all objects have polluted property
Object.prototype.polluted === true
```

**Vulnerable Versions:** ≥4.0.0 <4.1.1  
**Patched Versions:** ≥4.1.1

**Path:** `packages__cli>eslint>js-yaml`

**Remediation:**
1. Update eslint to version that includes js-yaml ≥4.1.1
2. If transitive, use pnpm overrides
3. Verify ESLint configuration still works

**Workaround (if needed):**
```bash
# Run node with prototype pollution protection
node --disable-proto=delete
```

### Vulnerability 2: tmp - Symlink Write (LOW)

**GHSA:** GHSA-52f5-9888-hmc6  
**Severity:** Low

**Description:**
Allows arbitrary temporary file/directory write via symbolic link in `dir` parameter.

**Vulnerable Versions:** ≤0.2.3  
**Patched Versions:** ≥0.2.4

**Path:** `packages__cli>inquirer>external-editor>tmp`

**Remediation:**
1. Update inquirer to version that includes tmp ≥0.2.4
2. If transitive, use pnpm overrides
3. Verify interactive prompts still work

---

## Implementation Steps

### Step 1: Analyze Dependencies

```bash
# Check dependency paths
pnpm why js-yaml
pnpm why tmp

# Check current versions
pnpm list js-yaml
pnpm list tmp
pnpm list eslint
pnpm list inquirer
```

### Step 2: Identify Update Strategy

**Check if direct dependencies:**
```bash
# Search package.json files
grep -r "eslint" packages/*/package.json
grep -r "inquirer" packages/*/package.json
```

**Check for available updates:**
```bash
# See available updates
pnpm outdated eslint
pnpm outdated inquirer
```

### Step 3: Apply Updates

**Option A: Update parent packages**
```bash
# Update ESLint (should pull in newer js-yaml)
pnpm update eslint -r --latest

# Update inquirer (should pull in newer tmp)
pnpm update inquirer -r --latest
```

**Option B: Use overrides**
```json
// package.json (root)
{
  "pnpm": {
    "overrides": {
      "js-yaml": "^4.1.1",
      "tmp": "^0.2.4"
    }
  }
}
```

### Step 4: Verify Fixes

```bash
# Re-run audit
pnpm audit

# Specific package checks
pnpm audit --json | jq '.advisories | .[] | select(.module_name=="js-yaml")'
pnpm audit --json | jq '.advisories | .[] | select(.module_name=="tmp")'
```

---

## Verification Steps

1. **ESLint Verification:**
   ```bash
   # Run linting
   pnpm lint
   
   # Should work without errors
   ```

2. **Interactive CLI Verification:**
   ```bash
   # If CLI uses inquirer, test interactive prompts
   # (Manual testing required)
   ```

3. **Build Verification:**
   ```bash
   pnpm build
   ```

4. **Test Suite:**
   ```bash
   pnpm test
   ```

5. **Audit Verification:**
   ```bash
   pnpm audit --audit-level=moderate
   # Should report 0 issues
   ```

---

## Files to Modify

1. **Root `package.json`**
   - Add/update pnpm.overrides if needed

2. **Workspace `package.json` files**
   - Update eslint/inquirer versions if direct dependencies

3. **`.eslintrc.*` files**
   - Verify ESLint configs still work
   - May need minor adjustments if ESLint major version changed

4. **`pnpm-lock.yaml`**
   - Auto-updated

---

## Testing Checklist

- [ ] ESLint runs without errors
- [ ] All lint rules still work as expected
- [ ] Interactive prompts work (if using inquirer)
- [ ] Temporary file creation works (if using tmp)
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] No new console warnings

---

## Known Issues & Edge Cases

1. **ESLint Version Compatibility:**
   - Major version changes may require config updates
   - Check changelog: https://eslint.org/blog/

2. **Inquirer Breaking Changes:**
   - UI changes may affect prompt behavior
   - Test all interactive flows

3. **YAML Parsing:**
   - js-yaml 4.1.1+ may have stricter parsing
   - Verify any YAML config files still parse correctly

---

## Rollback Plan

```bash
# If updates cause issues
git checkout pnpm-lock.yaml package.json

pnpm install

# Document accepted risks instead
```

---

## Definition of Done

- [ ] `pnpm audit --audit-level=moderate` reports 0 moderate/low issues (or documented exceptions)
- [ ] ESLint works correctly
- [ ] All interactive features work
- [ ] All tests pass
- [ ] No new build warnings
- [ ] Changes committed
- [ ] Ticket marked as Complete

---

## Resources

- **js-yaml Advisory:** https://github.com/advisories/GHSA-mh29-5h37-fv8m
- **tmp Advisory:** https://github.com/advisories/GHSA-52f5-9888-hmc6
- **Prototype Pollution Prevention:** https://cheatsheetseries.owasp.org/cheatsheets/Prototype_Pollution_Prevention_Cheat_Sheet.html
- **ESLint Releases:** https://github.com/eslint/eslint/releases

---

## Notes

**Priority:** High (not critical) because:
- Prototype pollution requires parsing untrusted YAML (low likelihood in dev tools)
- tmp symlink attack requires local access (low likelihood in typical usage)
- However, should still be fixed promptly for defense-in-depth

**Dependency on SECHARD-2001:**
- Can be done in parallel, but recommended after 2001
- Some vite vulnerabilities may overlap if not fixed in 2001

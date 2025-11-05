# Quality Strategy: npm Package Deprecation

## Testing Philosophy

This is a **one-time, terminal operation** with:
- No ongoing maintenance
- No user-facing functionality (it's deprecated)
- Simple, static content
- Well-documented npm APIs

**Pragmatic Approach:** Manual verification over automated testing.

**Why No Automated Tests?**
1. Content is static (package.json, README, index.js)
2. One-time operation (not continuous integration)
3. npm provides the verification (publish succeeds/fails)
4. Manual validation is faster than writing tests
5. Cost-benefit doesn't justify test infrastructure

## Verification Strategy

### Pre-Publish Validation

**1. Local Package Testing**

```bash
# Build and inspect package
npm pack

# Expected: maproom-mcp-2.0.0.tgz (~5-10 KB)

# Extract and verify contents
tar -xzf maproom-mcp-2.0.0.tgz
ls -la package/

# Expected files:
# - package.json
# - README.md
# - index.js
```

**Verification Checklist:**
- ✅ All files present
- ✅ No unexpected files (node_modules, temp files)
- ✅ File sizes reasonable (README <15 KB, index.js <1 KB)

**2. Content Validation**

```bash
cd package

# Verify package.json
cat package.json | jq .
# Check: name, version, bin, deprecated field

# Verify README renders correctly
# (View in text editor or convert to HTML)

# Verify executable
node index.js
# Expected: Deprecation message with new package info

node index.js --help
# Expected: --help specific message
```

**Verification Checklist:**
- ✅ package.json is valid JSON
- ✅ Version is 2.0.0
- ✅ README contains deprecation notice
- ✅ index.js executable flag set (`chmod +x`)
- ✅ Shebang correct (`#!/usr/bin/env node`)
- ✅ Exit code is 1 (use `echo $?`)

**3. npm Validation**

```bash
# Verify logged in
npm whoami
# Expected: Your npm username

# Verify package ownership
npm owner ls maproom-mcp
# Expected: Your username in list

# Dry-run publish (if supported)
# npm publish --dry-run  # Check if this flag exists
```

**Verification Checklist:**
- ✅ Authenticated to npm
- ✅ Have publish rights
- ✅ No validation errors from npm

### Post-Publish Validation

**1. Immediate Verification**

```bash
# Check published successfully
npm view maproom-mcp version
# Expected: 2.0.0

npm view maproom-mcp deprecated
# Expected: Deprecation message

npm view maproom-mcp description
# Expected: "DEPRECATED: Use @crewchief/maproom-mcp instead"
```

**Verification Checklist:**
- ✅ Version 2.0.0 appears in registry
- ✅ Deprecated field set correctly
- ✅ Description shows deprecation

**2. Installation Testing**

```bash
# Test installation warning (in clean directory)
cd /tmp/test-install
npm install maproom-mcp

# Expected output:
# npm WARN deprecated maproom-mcp@2.0.0: This package has been renamed...
```

**Verification Checklist:**
- ✅ Installation succeeds
- ✅ Deprecation warning appears
- ✅ Warning mentions new package

**3. Execution Testing**

```bash
# Test npx execution
npx maproom-mcp@2.0.0

# Expected:
# - Deprecation message
# - Migration instructions
# - Exit code 1

# Test --help flag
npx maproom-mcp@2.0.0 --help

# Expected:
# - Deprecation message
# - Help-specific instructions
# - Link to new package help
```

**Verification Checklist:**
- ✅ Executable runs
- ✅ Shows deprecation message
- ✅ `--help` shows specific message
- ✅ Exits with error code

**4. npm Website Verification**

Visit: https://www.npmjs.com/package/maproom-mcp

**Verification Checklist:**
- ✅ Shows version 2.0.0
- ✅ "DEPRECATED" badge visible
- ✅ README shows deprecation notice
- ✅ Links to new package work

### Deprecation Command Validation

```bash
# Run deprecation
npm deprecate maproom-mcp "This package has been replaced by @crewchief/maproom-mcp. Please use the new package: npx @crewchief/maproom-mcp --help"

# Verify applied to all versions
npm view maproom-mcp versions --json
npm view maproom-mcp@* deprecated

# Each version should show deprecation message
```

**Verification Checklist:**
- ✅ Command succeeds
- ✅ All versions show deprecated message
- ✅ Message matches specification

## Risk Mitigation

### High-Impact Risks

**Risk 1: Publishing Wrong Content**
- **Impact:** Users see incorrect information
- **Mitigation:** Pre-publish validation with `npm pack` + manual review
- **Rollback:** Publish corrected version as 2.0.1

**Risk 2: Breaking Existing Installations**
- **Impact:** Users' existing code stops working
- **Mitigation:** Deprecation doesn't break anything, just warns
- **Validation:** Test in isolated environment first

**Risk 3: Incorrect Deprecation Message**
- **Impact:** Users confused about migration
- **Mitigation:** Review message text carefully, include user-requested `--help`
- **Rollback:** Can update deprecation message with new `npm deprecate` call

### Medium-Impact Risks

**Risk 4: Can't Publish (Permission Denied)**
- **Impact:** Can't complete deprecation
- **Mitigation:** Verify `npm whoami` and `npm owner ls` before starting
- **Resolution:** Contact npm support or package owner

**Risk 5: Version Collision**
- **Impact:** 2.0.0 already exists
- **Mitigation:** Check `npm view maproom-mcp versions` first
- **Resolution:** Use 2.0.1 or 3.0.0

### Low-Impact Risks

**Risk 6: npm Registry Downtime**
- **Impact:** Can't publish immediately
- **Mitigation:** Check https://status.npmjs.org/ first
- **Resolution:** Wait and retry later

**Risk 7: Typo in Message**
- **Impact:** Minor confusion
- **Mitigation:** Copy-paste from specification
- **Resolution:** Can't fix published README, but can update deprecation message

## Edge Cases

### Edge Case 1: Users on Old Versions

**Scenario:** Users have `maproom-mcp@1.x.x` installed

**Behavior:**
- Installation shows deprecation warning (applies to all versions)
- Doesn't automatically update
- Users must manually migrate

**No Action Needed:** This is expected behavior.

### Edge Case 2: Lock Files

**Scenario:** `package-lock.json` or `yarn.lock` has old version pinned

**Behavior:**
- Warning shows during install, but uses locked version
- Users must manually update lock file

**No Action Needed:** User controls version updates.

### Edge Case 3: Private Registries/Mirrors

**Scenario:** Corporate environments using npm mirrors

**Behavior:**
- Deprecation might not sync immediately
- Mirror sync schedule varies

**No Action Needed:** Mirror issue, not our problem.

### Edge Case 4: Offline/Cached Installations

**Scenario:** npm cache has old version

**Behavior:**
- Warning might not show if fully cached
- Next fresh install will show warning

**No Action Needed:** Cache invalidates over time.

## Quality Gates

### Gate 1: Pre-Publish (MUST PASS)

- ✅ `npm pack` succeeds
- ✅ Package size < 50 KB
- ✅ index.js executable
- ✅ README renders correctly
- ✅ package.json valid
- ✅ `npm whoami` succeeds

**If any fail:** Fix before publishing.

### Gate 2: Post-Publish (MUST PASS)

- ✅ `npm view maproom-mcp@2.0.0` succeeds
- ✅ Deprecation message visible
- ✅ npm website shows v2.0.0
- ✅ `npx maproom-mcp@2.0.0` shows message

**If any fail:** Publish 2.0.1 with fixes.

### Gate 3: Full Deprecation (MUST PASS)

- ✅ `npm install maproom-mcp` shows warning
- ✅ All versions show deprecated status
- ✅ Message includes new package name
- ✅ `--help` reference in message

**If any fail:** Re-run `npm deprecate` with corrected message.

## Documentation Standards

### User-Facing Content

**Tone:**
- Helpful, not punitive
- Action-oriented ("Use this" not "Don't use that")
- Specific commands, not vague instructions

**Required Elements:**
- ✅ Clear statement of deprecation
- ✅ Name of replacement package
- ✅ Exact installation command
- ✅ Link to new documentation
- ✅ Explanation of what changed (name only)

**Validation:**
- Read as if you're a confused user
- Every question should have an answer
- No jargon without explanation

### Technical Documentation

**For Future Reference:**
- Document what was published
- Document npm commands used
- Document verification steps
- Store in `.agents/projects/NPMDEP_npm-deprecation/`

**Not Needed:**
- API documentation (no API)
- Architecture diagrams (too simple)
- Test reports (manual verification)

## Success Definition

**Project Complete When:**

1. ✅ Version 2.0.0 published to npm
2. ✅ README visible on npm package page
3. ✅ Deprecation warning shows during install
4. ✅ `npx maproom-mcp --help` shows migration message
5. ✅ All links to new package work
6. ✅ No errors in npm registry

**Manual Verification:** Run post-publish checklist.

**Timeline:** Can verify completion in ~5 minutes after publish.

## What We're NOT Testing

**Not Testing:**
- New package functionality (different package)
- Migration process (user responsibility)
- npm registry internals (npm's responsibility)
- Historical versions (immutable)
- Download statistics (not relevant)
- Security vulnerabilities (no dependencies, minimal code)

**Rationale:** These are out of scope or not our responsibility.

## Lessons Learned Capture

**After Completion:**
1. Document any issues encountered
2. Note any unexpected npm behaviors
3. Record actual time spent vs. estimated
4. Update this strategy if we do this again

**Storage:** Add to `.agents/knowledge/npm/` for future reference.

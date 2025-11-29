# Implementation Plan: npm Package Deprecation

## Project Summary

Deprecate old `maproom-mcp` npm package by publishing a final "tombstone" version (2.0.0) that shows clear migration guidance, then applying `npm deprecate` to all versions.

## Phases and Deliverables

### Phase 1: Preparation and Validation

**Goal:** Verify current state and prepare deprecation content.

**Deliverables:**

1. **Current State Assessment**
   - Check current version of `maproom-mcp` on npm
   - Verify user has publish rights
   - Document existing versions

2. **Content Preparation**
   - Create package.json for v2.0.0
   - Create index.js executable
   - Copy README.deprecated.md to README.md
   - Validate all content

3. **Local Testing**
   - Build package with `npm pack`
   - Extract and verify contents
   - Test executable locally
   - Test --help flag

**Agent:** general-purpose (content creation and validation)

**Success Criteria:**
- ✅ `npm pack` succeeds
- ✅ Package contains exactly 3 files (package.json, README.md, index.js)
- ✅ Executable runs and shows correct message
- ✅ --help flag shows specific message
- ✅ Package size <50 KB

**Estimated Time:** 30 minutes

---

### Phase 2: Publishing

**Goal:** Publish version 2.0.0 to npm registry.

**Deliverables:**

1. **Pre-Publish Verification**
   - Verify npm authentication (`npm whoami`)
   - Verify publish rights (`npm owner ls maproom-mcp`)
   - Final content review

2. **Publish to npm**
   - Run `npm publish` from deprecation package directory
   - Monitor for errors
   - Capture output

3. **Immediate Validation**
   - Verify version appears in registry
   - Check npm package page
   - Verify README rendering

**Agent:** general-purpose (manual steps with user)

**Success Criteria:**
- ✅ Publish command succeeds
- ✅ `npm view maproom-mcp@2.0.0` returns data
- ✅ npm website shows v2.0.0
- ✅ README visible on package page
- ✅ Deprecated field set in metadata

**Estimated Time:** 15 minutes

---

### Phase 3: Deprecation Tagging

**Goal:** Apply deprecation warning to all versions.

**Deliverables:**

1. **Apply Deprecation**
   - Run `npm deprecate` command with user-specified message
   - Verify command succeeds

2. **Verify Deprecation Applied**
   - Check all versions show deprecated status
   - Test installation warning appears
   - Verify message text correct

**Agent:** general-purpose (command execution)

**Success Criteria:**
- ✅ Deprecation command succeeds
- ✅ `npm install maproom-mcp` shows warning
- ✅ Warning includes new package name
- ✅ Warning includes --help reference
- ✅ All historical versions show deprecated

**Estimated Time:** 10 minutes

---

### Phase 4: End-to-End Validation

**Goal:** Verify complete deprecation from user perspective.

**Deliverables:**

1. **Installation Testing**
   - Fresh install in test directory
   - Verify warning appears
   - Check installed version

2. **Execution Testing**
   - Run `npx maproom-mcp@2.0.0`
   - Verify migration message
   - Test --help flag
   - Check exit code

3. **Web Validation**
   - Check npm package page
   - Verify deprecated badge visible
   - Check README rendering
   - Test all links

4. **Documentation**
   - Document what was published
   - Save verification results
   - Note any issues encountered

**Agent:** general-purpose (validation and documentation)

**Success Criteria:**
- ✅ All verification tests pass
- ✅ No broken links
- ✅ Clear migration path visible
- ✅ Documentation complete

**Estimated Time:** 15 minutes

---

## Agent Assignment

All phases use **general-purpose** agent because:
- Simple, sequential tasks
- Requires user interaction (npm login, credentials)
- Manual verification steps
- No specialized domain knowledge needed
- One-time operation (no automation benefits)

## Dependencies

### Phase 1 → Phase 2
- Must validate package before publishing
- Cannot publish without content

### Phase 2 → Phase 3
- Must publish v2.0.0 before deprecating
- Deprecation references published version

### Phase 3 → Phase 4
- Must apply deprecation before final validation
- Validation checks deprecation is working

**Critical Path:** Linear progression, no parallelization possible.

## Timeline

| Phase | Duration | Dependencies | Risk |
|-------|----------|--------------|------|
| Phase 1 | 30 min | None | Low |
| Phase 2 | 15 min | Phase 1 | Medium |
| Phase 3 | 10 min | Phase 2 | Low |
| Phase 4 | 15 min | Phase 3 | Low |
| **Total** | **70 min** | | |

**Note:** Assumes no npm auth issues, no version conflicts, no registry downtime.

## Risk Mitigation

### High-Priority Risks

**1. User Lacks Publish Rights**
- **When:** Phase 2 (Publishing)
- **Mitigation:** Check in Phase 1 with `npm owner ls`
- **Fallback:** User contacts npm support or package owner

**2. Version 2.0.0 Already Exists**
- **When:** Phase 2 (Publishing)
- **Mitigation:** Check in Phase 1 with `npm view maproom-mcp versions`
- **Fallback:** Use 2.0.1 or 3.0.0

**3. npm Registry Downtime**
- **When:** Phase 2 or 3
- **Mitigation:** Check https://status.npmjs.org/ before starting
- **Fallback:** Wait and retry later

### Medium-Priority Risks

**4. Incorrect Package Contents**
- **When:** Phase 2 (Publishing)
- **Mitigation:** Thorough Phase 1 validation
- **Fallback:** Publish 2.0.1 with corrections

**5. Deprecation Message Typo**
- **When:** Phase 3 (Deprecation)
- **Mitigation:** Copy-paste from specification
- **Fallback:** Re-run `npm deprecate` with corrected message

## Rollback Plan

### If Publish Fails (Phase 2)

**Symptoms:**
- npm publish returns error
- Package doesn't appear in registry

**Actions:**
1. Review error message
2. Fix issue (version conflict, auth, etc.)
3. Retry publish
4. No cleanup needed (nothing published)

### If Wrong Content Published (Phase 2)

**Symptoms:**
- npm publish succeeds but content is wrong
- README shows incorrect info

**Actions:**
1. ⚠️ Cannot unpublish after 72 hours
2. Publish 2.0.1 with corrected content
3. Document mistake in Phase 4 notes

### If Deprecation Message Wrong (Phase 3)

**Symptoms:**
- `npm deprecate` succeeds but message has typo
- Users see incorrect migration info

**Actions:**
1. Re-run `npm deprecate` with correct message
2. Verify new message appears
3. Old message is overwritten (no harm)

## Success Metrics

**Project Complete When:**
1. ✅ Version 2.0.0 published to npm registry
2. ✅ README visible on https://www.npmjs.com/package/maproom-mcp
3. ✅ `npm install maproom-mcp` shows deprecation warning
4. ✅ Warning mentions `@crewchief/maproom-mcp`
5. ✅ Warning includes `npx @crewchief/maproom-mcp --help` reference
6. ✅ `npx maproom-mcp` shows migration message
7. ✅ `npx maproom-mcp --help` shows help-specific message
8. ✅ All links work
9. ✅ Documentation complete

**Measurement:** Manual verification checklist (see quality-strategy.md).

## Out of Scope

**Not Included in This Project:**
- Monitoring package downloads
- Responding to user support requests
- Setting up 2FA (user's responsibility)
- Migrating users' code (user's responsibility)
- Removing old package from npm (against npm policy)
- Transferring package ownership

**Future Work (If Needed):**
- Monitor for unusual download patterns (security)
- Create automated deprecation script for future use
- Document lessons learned in `.crewchief/knowledge/npm/`

## Documentation Outputs

**Generated Documentation:**
1. Phase 4 validation report
2. npm commands used (for future reference)
3. Any issues encountered and resolutions

**Storage Location:** `.crewchief/projects/NPMDEP_npm-deprecation/`

**Format:** Markdown files, command output logs.

## Phase Transition Checklist

### Phase 1 → Phase 2
- [ ] Package contents validated
- [ ] Executable tested locally
- [ ] npm authentication confirmed
- [ ] User ready to publish

### Phase 2 → Phase 3
- [ ] Version 2.0.0 visible in registry
- [ ] README rendering correct on npm website
- [ ] No publish errors

### Phase 3 → Phase 4
- [ ] Deprecation command succeeded
- [ ] At least one version shows deprecated status
- [ ] Ready for comprehensive testing

### Phase 4 → Complete
- [ ] All validation tests passed
- [ ] Documentation written
- [ ] User satisfied with result
- [ ] No outstanding issues

## Command Reference

**Pre-flight Checks:**
```bash
npm whoami                           # Verify authentication
npm owner ls maproom-mcp            # Verify publish rights
npm view maproom-mcp versions       # Check existing versions
npm view maproom-mcp@2.0.0          # Check if 2.0.0 exists
```

**Phase 1:**
```bash
cd /tmp/maproom-mcp-deprecated
npm pack                            # Build package
tar -xzf maproom-mcp-2.0.0.tgz     # Extract for inspection
node package/index.js               # Test executable
node package/index.js --help        # Test --help flag
echo $?                             # Check exit code (should be 1)
```

**Phase 2:**
```bash
npm publish                         # Publish to registry
npm view maproom-mcp@2.0.0         # Verify published
```

**Phase 3:**
```bash
npm deprecate maproom-mcp "This package has been replaced by @crewchief/maproom-mcp. Please use the new package: npx @crewchief/maproom-mcp --help"
```

**Phase 4:**
```bash
cd /tmp/test-install
npm install maproom-mcp             # Test warning
npx maproom-mcp@2.0.0              # Test execution
npx maproom-mcp@2.0.0 --help       # Test --help
```

## Notes for Implementation Agent

**Key Points:**
1. This is a **user-interactive** process (npm credentials needed)
2. Validation after each phase is **critical** (can't easily undo)
3. User specifically wants **--help flag** support
4. Deprecation message must match **exact user specification**
5. **Manual steps** are acceptable (one-time operation)

**User Interaction Points:**
- npm login (Phase 1)
- Approve publish (Phase 2)
- Verify web page (Phase 4)

**Don't Overcomplicate:**
- No automation needed
- No CI/CD setup
- No monitoring infrastructure
- Just publish and verify

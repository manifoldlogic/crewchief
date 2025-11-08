# Ticket: CLIREL-5001: Update MCP Workflow for Package-Scoped Tags

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update the existing maproom-mcp GitHub Actions workflow to trigger on package-scoped tags (`@crewchief/maproom-mcp@v*.*.*`) instead of simple version tags (`v*.*.*`). This ensures workflow isolation and prevents accidental cross-triggering with the CLI workflow.

## Background

### Current State
The MCP workflow currently triggers on simple `v*.*.*` tags. This creates ambiguity and potential conflicts once the CLI workflow is added (which uses `@crewchief/cli@v*.*.*` tags).

### Why This Matters
- **Prevents cross-triggering**: CLI tags won't trigger MCP workflow
- **Clear intent**: Tag name explicitly identifies which package it belongs to
- **Consistency**: Both packages follow same pattern
- **Future-proof**: Supports adding more packages without conflicts

### What's NOT Changing
- The workflow logic itself (matrix builds, validation, publish)
- The binary build process
- The npm publish steps
- The artifact handling

**Only the trigger changes.**

## Acceptance Criteria
- [ ] MCP workflow trigger updated to `@crewchief/maproom-mcp@v*.*.*`
- [ ] MCP workflow does NOT trigger on `@crewchief/cli@v*` tags
- [ ] CLI workflow does NOT trigger on `@crewchief/maproom-mcp@v*` tags
- [ ] Tag isolation test passes (synthetic tags created and verified)
- [ ] workflow_dispatch trigger remains unchanged (manual testing still works)
- [ ] No other workflow changes made (only trigger modification)

## Technical Requirements

### 1. Update Workflow Trigger

**File**: `.github/workflows/build-and-publish-maproom-mcp.yml`

**Current trigger** (approximately line 3-7):
```yaml
on:
  push:
    tags:
      - 'v*.*.*'  # OLD: Simple version tags
```

**New trigger**:
```yaml
on:
  push:
    tags:
      - '@crewchief/maproom-mcp@v*.*.*'  # NEW: Package-scoped tags
  workflow_dispatch:  # Keep manual trigger for testing
    inputs:
      dry_run:
        description: 'Dry run (skip publish)'
        type: boolean
        default: false
```

**That's it.** No other changes needed.

### 2. Tag Isolation Testing

**Create synthetic test tags**:
```bash
# Test CLI tag doesn't trigger MCP workflow
git tag @crewchief/cli@v1.0.0-test
git push origin @crewchief/cli@v1.0.0-test

# Check GitHub Actions
# Expected: Only CLI workflow runs, MCP workflow does NOT run

# Test MCP tag doesn't trigger CLI workflow
git tag @crewchief/maproom-mcp@v1.3.6-test
git push origin @crewchief/maproom-mcp@v1.3.6-test

# Check GitHub Actions
# Expected: Only MCP workflow runs, CLI workflow does NOT run

# Cleanup
git tag -d @crewchief/cli@v1.0.0-test @crewchief/maproom-mcp@v1.3.6-test
git push origin :refs/tags/@crewchief/cli@v1.0.0-test
git push origin :refs/tags/@crewchief/maproom-mcp@v1.3.6-test
```

**Verification**:
- Go to GitHub Actions page
- Check "Build and Publish MCP" workflow runs
- Verify it ran ONLY for `@crewchief/maproom-mcp@v*` tag
- Verify it did NOT run for `@crewchief/cli@v*` tag
- Check "Build and Publish CLI" workflow runs
- Verify opposite behavior

### 3. Backward Compatibility Note

**Old simple tags** (like `v1.3.5`) will NO LONGER trigger the workflow.

**Impact**: None if release script was updated in CLIREL-3001
- CLIREL-3001 updated release script to create `@crewchief/maproom-mcp@v*` tags
- Future releases will use new format
- Old releases still exist in git history but won't auto-publish

**If needed**: Can manually trigger workflow with workflow_dispatch for old tags

## Implementation Notes

### Why This is Low-Risk
- One-line change (trigger pattern)
- No logic changes
- Easy to revert if issues arise
- Can test with synthetic tags before real release

### Order of Operations
1. Update workflow trigger in YAML file
2. Commit and push change
3. Create synthetic test tags (both CLI and MCP)
4. Verify isolation (only correct workflow runs for each tag)
5. Delete test tags
6. Document successful isolation testing in ticket

### Common Pitfalls
- **Typo in trigger pattern**: Must be exact: `@crewchief/maproom-mcp@v*.*.*`
- **Forgetting to update release script**: CLIREL-3001 must be complete
- **Testing with real tags**: Use `-test` suffix to avoid confusion

## Dependencies
- CLIREL-3001 (Release Scripts) - Must complete first (MCP release script creates new tag format)
- CLIREL-4001 (CLI Workflow) - Must complete first (need both workflows to test isolation)

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| Old simple tags don't work | Low | Document workaround (manual workflow_dispatch) |
| Typo breaks MCP releases | Medium | Test with synthetic tags before real release |
| CLI/MCP workflows still cross-trigger | Low | Comprehensive isolation testing |
| workflow_dispatch breaks | Low | Include in change, test manually |

## Files/Packages Affected
- `.github/workflows/build-and-publish-maproom-mcp.yml` (modify trigger only)

## Success Metrics
- MCP workflow triggers on `@crewchief/maproom-mcp@v1.3.6-test`
- MCP workflow does NOT trigger on `@crewchief/cli@v1.0.0-test`
- CLI workflow triggers on `@crewchief/cli@v1.0.0-test`
- CLI workflow does NOT trigger on `@crewchief/maproom-mcp@v1.3.6-test`
- workflow_dispatch still works for both workflows
- No other workflow behavior changes

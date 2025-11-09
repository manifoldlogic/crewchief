# Ticket: MCPREL-1002: Update package.json release scripts to use new release.js

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose

## Summary
Update the three release scripts in `packages/maproom-mcp/package.json` to call the new `release.js` script instead of directly publishing to npm. Remove the `pnpm publish` commands and `--no-git-checks` flags that bypass git integration.

## Background
The current release scripts follow this pattern:
```json
"release:patch": "node scripts/bump-version.js patch && pnpm publish --access public --no-git-checks"
```

These scripts directly publish to npm, bypassing git commits and tags. With MCPREL-1001 creating the new `release.js` script and GitHub Actions workflows ready to build and publish on tag push, we need to update these scripts to only trigger the git workflow. The new scripts should simply call `release.js`, which handles version bumping, committing, tagging, and pushing.

## Acceptance Criteria
- [x] `release:patch` script updated to: `"node scripts/release.js patch"`
- [x] `release:minor` script updated to: `"node scripts/release.js minor"`
- [x] `release:major` script updated to: `"node scripts/release.js major"`
- [x] All `pnpm publish` commands removed from release scripts
- [x] All `--no-git-checks` and `--access public` flags removed
- [x] package.json remains valid JSON (no syntax errors)
- [x] Scripts execute successfully: `pnpm release:patch` runs without errors
- [x] No other scripts in package.json are modified

## Technical Requirements

### File Location
- **MODIFY**: `/workspace/packages/maproom-mcp/package.json`

### Scripts to Modify
In the `"scripts"` section, update:
- `"release:patch"`
- `"release:minor"`
- `"release:major"`

### Exact New Values
```json
"release:patch": "node scripts/release.js patch",
"release:minor": "node scripts/release.js minor",
"release:major": "node scripts/release.js major"
```

### Formatting Requirements
- Preserve existing JSON formatting and indentation (2 spaces)
- No other scripts should be modified

### Scripts to Leave Unchanged
- `build`
- `test`
- `prepublishOnly` (used by GitHub Actions workflow)
- `setup`, `scan`, `watch`, etc.

## Implementation Notes

### Approach
1. Use the Edit tool to replace each script entry
2. Simple search-and-replace operation for each of the three scripts
3. Verify JSON syntax after changes

### Example Change
```diff
- "release:patch": "node scripts/bump-version.js patch && pnpm publish --access public --no-git-checks",
+ "release:patch": "node scripts/release.js patch",
```

### Verification
After editing, verify:
- JSON is still valid (no syntax errors)
- Only the three release:* scripts were changed
- File can be parsed: `node -e "require('./packages/maproom-mcp/package.json')"`

## Dependencies
- **BLOCKED BY**: MCPREL-1001 (release.js must exist before updating package.json to call it)

## Risk Assessment
- **Risk**: JSON syntax error breaks package.json
  - **Mitigation**: Use Edit tool carefully, verify file after changes, can easily revert if needed
- **Risk**: Other scripts accidentally modified
  - **Mitigation**: Only edit the three release:* scripts, leave everything else untouched

## Files/Packages Affected
- **MODIFY**: `/workspace/packages/maproom-mcp/package.json` (scripts section only)

## Estimated Time
10-15 minutes

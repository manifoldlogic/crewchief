# Ticket: CLIREL-3001: Update Release Scripts and Fix Race Condition

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update both CLI and MCP release scripts to use package-scoped tags, implement two-step push to fix race condition, and remove manual npm publish commands (GitHub Actions will handle publishing).

## Background

### Problem: Race Condition in Current MCP Workflow
The current maproom-mcp release process uses `git push --follow-tags`, which can cause workflow trigger failures. The tag can arrive at GitHub before the commit is fully registered, causing the Actions workflow to fail because it can't find the commit that the tag references.

### Solution: Two-Step Push
Push commits and tags separately:
1. `git push` - Push commits first, wait for registration
2. `git push origin <tag>` - Then push tag separately

This ensures the commit exists on GitHub before the tag arrives, preventing the race condition.

### Package-Scoped Tags
Both packages need consistent, non-conflicting tag formats:
- CLI: `@crewchief/cli@v{version}`
- MCP: `@crewchief/maproom-mcp@v{version}`

This allows independent releases and prevents accidental cross-triggering of workflows.

## Acceptance Criteria
- [ ] CLI release script creates `@crewchief/cli@v{version}` tags
- [ ] MCP release script creates `@crewchief/maproom-mcp@v{version}` tags
- [ ] Both scripts use two-step push (commits first, then tags)
- [ ] Both scripts remove `pnpm publish` / `npm publish` commands
- [ ] Scripts print clear instructions explaining the process
- [ ] Test run confirms commits arrive before tags on GitHub
- [ ] No `--follow-tags` usage remains in either script

## Technical Requirements

### 1. Update packages/cli/scripts/release.mjs

**Current problematic code** (approximately):
```javascript
execSync(`git tag crewchief@v${nextVersion}`, { stdio: 'inherit' })
execSync('git push --follow-tags', { stdio: 'inherit' })  // RACE CONDITION
execSync('pnpm publish --access public', { stdio: 'inherit' })  // Remove
```

**New code**:
```javascript
const tag = `@crewchief/cli@v${nextVersion}`

// Create tag
execSync(`git tag ${tag}`, { stdio: 'inherit' })

// Two-step push to avoid race condition
console.log('Pushing commit...')
execSync('git push', { stdio: 'inherit' })

console.log('Pushing tag...')
execSync(`git push origin ${tag}`, { stdio: 'inherit' })

// Remove npm publish - GitHub Actions will handle it
// execSync('pnpm publish --access public', { stdio: 'inherit' })

console.log(`\n✓ Tagged and pushed ${tag}`)
console.log('  GitHub Actions will build and publish automatically')
console.log('  Monitor workflow: https://github.com/OWNER/REPO/actions\n')
```

### 2. Update packages/maproom-mcp/scripts/release.js

**Current problematic code** (approximately):
```javascript
execSync(`git tag -a v${version} -m "Release version ${version}"`, { stdio: 'inherit' })
execSync('git push --follow-tags', { stdio: 'inherit' })  // RACE CONDITION
```

**New code**:
```javascript
const tag = `@crewchief/maproom-mcp@v${version}`

// Create annotated tag
execSync(`git tag -a ${tag} -m "Release version ${version}"`, { stdio: 'inherit' })

// Two-step push to avoid race condition
console.log('Pushing commit...')
execSync('git push', { stdio: 'inherit' })

console.log('Pushing tag...')
execSync(`git push origin ${tag}`, { stdio: 'inherit' })

console.log(`\n✓ Tagged and pushed ${tag}`)
console.log('  GitHub Actions will build and publish automatically')
console.log('  Monitor workflow: https://github.com/OWNER/REPO/actions\n')
```

### 3. Tag Format Validation

**CLI tag examples**:
- `@crewchief/cli@v1.0.0`
- `@crewchief/cli@v1.0.1`
- `@crewchief/cli@v1.1.0`

**MCP tag examples**:
- `@crewchief/maproom-mcp@v1.3.5`
- `@crewchief/maproom-mcp@v1.3.6`
- `@crewchief/maproom-mcp@v2.0.0`

**Validation**:
- Tags must match pattern: `@crewchief/(cli|maproom-mcp)@v\\d+\\.\\d+\\.\\d+`
- No simple `v1.0.0` tags (prevents conflicts)
- Package name in tag must match package.json name

## Implementation Notes

### Order of Operations
1. Update CLI release script (release.mjs)
2. Update MCP release script (release.js)
3. Test CLI script with dry-run (create/delete test tag)
4. Test MCP script with dry-run (create/delete test tag)
5. Verify scripts print helpful messages
6. Commit changes

### Testing Strategy

**Dry-run test** (don't push to remote):
```bash
# CLI test
cd packages/cli
node scripts/release.mjs patch  # Creates tag locally
git tag -d @crewchief/cli@v*    # Delete test tag

# MCP test
cd packages/maproom-mcp
node scripts/release.js         # Creates tag locally
git tag -d @crewchief/maproom-mcp@v*  # Delete test tag
```

**Verify two-step push works**:
- First push should show: "Writing objects... Done"
- Second push should show: "* [new tag] @crewchief/cli@v1.0.0 -> @crewchief/cli@v1.0.0"

### Why Two-Step Push Fixes Race Condition

**Problem with `--follow-tags`**:
1. Git pushes commit and tag simultaneously
2. Tag can arrive at GitHub first
3. GitHub tries to create tag reference
4. Commit doesn't exist yet → workflow trigger fails

**Solution with two-step push**:
1. `git push` - Commit arrives and registers
2. Small delay while GitHub processes
3. `git push origin <tag>` - Tag arrives
4. Commit already exists → workflow trigger succeeds

## Dependencies
- CLIREL-2001 (Package Configuration) - Must complete first (package.json name must be `@crewchief/cli`)

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| Race condition still occurs | Medium | Test with multiple releases, monitor workflow triggers |
| Wrong tag format | Low | Validate tag matches regex pattern |
| Accidental publish during testing | Low | Scripts remove publish commands, use dry-run testing |
| Breaking existing MCP releases | Low | MCP workflow updated in Phase 5 to expect new tags |

## Files/Packages Affected
- `/workspace/packages/cli/scripts/release.mjs` (modify)
- `/workspace/packages/maproom-mcp/scripts/release.js` (modify)

## Success Metrics
- CLI script creates `@crewchief/cli@v*` tags
- MCP script creates `@crewchief/maproom-mcp@v*` tags
- Commits pushed before tags (visible in git push output)
- No npm/pnpm publish commands executed
- Clear console output explains what's happening
- Test runs complete without errors

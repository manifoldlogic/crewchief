# Ticket: CLIREL-2001: Reconfigure CLI Package for @crewchief/cli Scoped Name

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (npm pack validation successful)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update the CLI package configuration to use the scoped package name `@crewchief/cli`, configure for workflow-based publishing, and prepare package structure for automated multi-platform releases.

## Background
The CLI package is being renamed from `crewchief` to `@crewchief/cli` to:
1. Follow organizational naming convention (matching `@crewchief/maproom-mcp`)
2. Enable independent versioning and tagging from MCP package
3. Support automated GitHub Actions publishing
4. Signal breaking change with v1.0.0

This ticket focuses on package configuration only - no workflow automation yet. The package should be ready for local `npm pack` testing to validate structure before workflows are created in Phase 4.

## Acceptance Criteria
- [x] package.json name changed to `@crewchief/cli`
- [x] package.json version set to `1.0.0`
- [x] publishConfig added with `{ "access": "public" }`
- [x] prepublishOnly hook removed from package.json
- [x] .npmignore created (exclude sources, include dist/ and bin/)
- [x] README.md updated with new package name
- [x] Local `npm pack` produces correctly structured tarball
- [x] Tarball inspection shows all expected files included

## Technical Requirements

### 1. Update packages/cli/package.json

**Changes needed**:
```json
{
  "name": "@crewchief/cli",        // Changed from "crewchief"
  "version": "1.0.0",               // Major bump for breaking change
  "publishConfig": {
    "access": "public"              // Required for scoped packages
  },
  "files": [
    "dist",
    "bin",
    "README.md",
    "LICENSE"
  ]
  // Remove: "prepublishOnly": "pnpm build:all"
}
```

**Rationale**:
- Scoped packages (`@org/name`) require `publishConfig.access: "public"` to be publicly installable
- Remove `prepublishOnly` hook because GitHub Actions workflow will handle building
- `files` whitelist ensures only necessary files are published

### 2. Create packages/cli/.npmignore

```
# Exclude source files
src/
tsconfig.json
*.test.ts
__tests__/

# Exclude development files
.git
.github
node_modules
.DS_Store

# Exclude CI/build artifacts
*.log
coverage/

# Include built artifacts (explicit)
!dist/
!bin/
```

**Rationale**:
- Belt-and-suspenders approach: `files` array is primary, .npmignore is backup
- Explicitly exclude source files to prevent accidental inclusion
- Ensure dist/ and bin/ are included even if .gitignore excludes them

### 3. Update packages/cli/README.md

**Changes needed**:
- Replace all instances of `crewchief` with `@crewchief/cli`
- Update installation command: `npm install -g @crewchief/cli`
- Update any usage examples that reference the package name

**Example**:
```markdown
# CrewChief CLI

## Installation

```bash
npm install -g @crewchief/cli
```

## Usage

```bash
crewchief --help
```
```

### 4. Validation Testing

**Before committing, run**:
```bash
cd packages/cli

# Create tarball
npm pack

# Inspect contents
tar -tzf crewchief-cli-1.0.0.tgz

# Verify structure
tar -tzf crewchief-cli-1.0.0.tgz | grep -E "^package/(bin|dist)/" | head -10

# Check that source files are NOT included
tar -tzf crewchief-cli-1.0.0.tgz | grep -E "^package/src/" && echo "ERROR: src/ should not be in tarball"

# Cleanup
rm crewchief-cli-1.0.0.tgz
```

**Expected tarball contents**:
```
package/package.json
package/README.md
package/LICENSE
package/dist/cli/index.js
package/dist/...
package/bin/crewchief
package/bin/darwin-arm64/crewchief-maproom
package/bin/linux-arm64/crewchief-maproom
...
```

## Implementation Notes

**Order of operations**:
1. Update package.json (name, version, publishConfig, remove prepublishOnly)
2. Create .npmignore
3. Update README.md
4. Run `npm pack` validation
5. Inspect tarball contents
6. Commit changes

**Reversibility**:
- All changes are in git, easily reverted
- No npm publish in this ticket (just local testing)
- Safe to experiment and iterate

**Common pitfalls**:
- Forgetting `publishConfig.access: "public"` → publish will fail with "402 Payment Required"
- Including src/ in tarball → users get unnecessary source files
- Typos in package name → publish to wrong package

## Dependencies
- None (can start immediately)
- CLIREL-1001 (Deprecation) exists but is not blocking

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| Missing files in tarball | Medium | Validate with `npm pack` and `tar -tzf` |
| Scoped package publish fails | Low | Add `publishConfig.access: "public"` |
| Wrong package name format | Low | Follow exact format: `@crewchief/cli` |
| README has broken links | Low | Manual review after changes |

## Files/Packages Affected
- `/workspace/packages/cli/package.json` (modify)
- `/workspace/packages/cli/.npmignore` (create)
- `/workspace/packages/cli/README.md` (modify)

## Success Metrics
- `npm pack` creates `crewchief-cli-1.0.0.tgz` (correct scoped name)
- Tarball contains: dist/, bin/, README.md, LICENSE
- Tarball does NOT contain: src/, tsconfig.json, *.test.ts
- README accurately reflects new package name
- No references to old `crewchief` package name remain

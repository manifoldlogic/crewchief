# Architecture: MCP Release Scripts

## Design Philosophy

**Keep it simple.** This is a straightforward task: bump version, commit, tag, push. Avoid overengineering.

User's directive: "I want you to be thorough, but don't overdo it or overthink it."

## Architecture Decision

### Chosen Approach: Sequential Script
Create a single release script that performs operations in sequence:
1. Bump version (reuse existing bump-version.js)
2. Commit the change
3. Create annotated tag
4. Push commit and tag

### Why This Approach
- **Simplicity**: One script, clear sequence, easy to understand
- **Reusability**: Leverages existing bump-version.js
- **Maintainability**: Linear flow, no complex state management
- **Testability**: Can test each git operation independently
- **Debuggability**: Clear error messages at each step

## File Structure

```
packages/maproom-mcp/
├── package.json                 # MODIFY: Update release:* scripts
├── scripts/
│   ├── bump-version.js          # KEEP: Already works, no changes needed
│   └── release.js               # CREATE: New orchestration script
```

## Script Design: release.js

### Input
- Command-line argument: `patch`, `minor`, or `major`
- Example: `node scripts/release.js patch`

### Output
- Console logs for each step
- Exit code 0 on success, non-zero on failure
- Clear error messages if anything fails

### Flow
```
┌─────────────────────────────────┐
│ 1. Parse version type argument  │
│    (patch/minor/major)          │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ 2. Run bump-version.js          │
│    Updates package.json         │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ 3. Read new version             │
│    from package.json            │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ 4. Git add package.json         │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ 5. Git commit                   │
│    "chore(release): bump        │
│     version to X.Y.Z"           │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ 6. Git tag -a vX.Y.Z            │
│    "Release version X.Y.Z"      │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ 7. Git push origin HEAD         │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ 8. Git push origin vX.Y.Z       │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────────┐
│ ✅ Success                       │
│ "Released version X.Y.Z"        │
└─────────────────────────────────┘
```

### Error Handling Strategy
- **Fail fast**: Exit immediately on any error
- **Clear messages**: Explain what failed and why
- **No rollback**: Keep it simple, let developer fix manually if needed
- **Exit codes**: Non-zero exit code for CI/automation detection

### Implementation Details

#### Using Child Process for Git
```javascript
import { execSync } from 'child_process';

// Helper to run git commands
function git(command) {
  try {
    execSync(`git ${command}`, {
      stdio: 'pipe',
      encoding: 'utf8',
      cwd: path.join(__dirname, '..') // Run from package root
    });
  } catch (error) {
    console.error(`Git command failed: git ${command}`);
    console.error(error.message);
    process.exit(1);
  }
}
```

#### Version Tag Format
- Format: `v{major}.{minor}.{patch}`
- Example: `v1.3.2`
- Always lowercase 'v' prefix for consistency
- Annotated tag (not lightweight) for better git history

#### Commit Message Format
- Pattern: `chore(release): bump version to {version}`
- Example: `chore(release): bump version to 1.3.2`
- Follows Conventional Commits specification
- Scope: `release` (matches package publishing semantic area)
- Type: `chore` (infrastructure/tooling change)

#### Working Directory
All git operations use `cwd: packageRoot` to ensure commands run from correct location, regardless of where user invokes the script.

## Package.json Script Updates

### Current
```json
{
  "scripts": {
    "release:patch": "node scripts/bump-version.js patch && pnpm publish --access public --no-git-checks",
    "release:minor": "node scripts/bump-version.js minor && pnpm publish --access public --no-git-checks",
    "release:major": "node scripts/bump-version.js major && pnpm publish --access public --no-git-checks"
  }
}
```

### New
```json
{
  "scripts": {
    "release:patch": "node scripts/release.js patch",
    "release:minor": "node scripts/release.js minor",
    "release:major": "node scripts/release.js major"
  }
}
```

**Key Changes:**
- Remove `pnpm publish` entirely (GitHub Actions handles this)
- Remove `--no-git-checks` flag (we WANT git integration)
- Call new `release.js` script instead of `bump-version.js` directly

## Integration with GitHub Actions

### Confirmed Workflow Integration
**Two production workflows already exist** and trigger on `v*.*.*` tags:

#### Workflow 1: `build-and-publish-maproom-mcp.yml`
```yaml
on:
  push:
    tags:
      - 'v*.*.*'
```

**What it does**:
1. Builds Rust binaries for 4 platforms in parallel:
   - linux-x64 (cross-compilation)
   - linux-arm64 (cross-compilation)
   - darwin-x64 (native on Intel Mac)
   - darwin-arm64 (native on Apple Silicon)
2. Validates binaries (size checks, execution tests)
3. Packages binaries into npm package structure
4. Creates tarball and verifies all binaries present
5. Publishes `@crewchief/maproom-mcp` to npm registry
6. Includes retry logic and post-publish verification

#### Workflow 2: `publish-maproom-mcp-image.yml`
```yaml
on:
  push:
    tags:
      - 'v*.*.*'
```

**What it does**:
1. Builds multi-platform Docker images (linux/amd64, linux/arm64)
2. Publishes to Docker Hub: `manifoldlogic/crewchief_maproom-mcp`
3. Creates multiple tags:
   - Full version: `1.3.2`
   - Minor version: `1.3`
   - Major version: `1`
   - Latest: `latest`
4. Runs Trivy security scan
5. Includes OCI-compliant image labels

### What Happens When Release Script Runs
```
Developer runs: pnpm release:patch
         ↓
  Version bumped: 1.3.1 → 1.3.2
         ↓
  Commit created: "chore(release): bump version to 1.3.2"
         ↓
  Tag created: v1.3.2
         ↓
  Push to origin: Commit + Tag
         ↓
GitHub detects tag push
         ↓
   ┌────────────────────────┬─────────────────────────┐
   │                        │                         │
   ▼                        ▼                         ▼
Workflow 1 starts      Workflow 2 starts       (Parallel)
   │                        │
   ▼                        ▼
Build 4 binaries       Build Docker images
   │                        │
   ▼                        ▼
Package npm            Push to Docker Hub
   │                        │
   ▼                        ▼
Publish to npm         Tag: 1.3.2, 1.3, 1, latest
   │                        │
   ▼                        ▼
✅ Complete             ✅ Complete
```

**Critical**: Our release scripts ONLY need to create and push the tag. All building, packaging, and publishing is automated by CI/CD.

## Testing Strategy

### Manual Testing
Developer can test locally:
```bash
cd packages/maproom-mcp
pnpm release:patch
```

Expected outcome:
- Version bumped in package.json
- Git commit created
- Git tag created
- Both pushed to origin

### Verification
After running, check:
```bash
git log -1                    # See commit message
git tag -l                    # See new tag
git show v1.3.2              # See tag details
git ls-remote --tags origin  # Verify tag pushed
```

### Dry Run Testing
Not implementing dry-run mode (per "don't overdo it"), but developer can:
1. Test on a feature branch
2. Delete tag/commit if needed: `git tag -d v1.3.2 && git reset HEAD~1`
3. Push tags separately if needed: `git push origin v1.3.2`

## Error Scenarios and Recovery

### Scenario 1: Commit Succeeds, Tag Fails
**Cause**: Tag already exists locally

**Recovery**:
```bash
git tag -d vX.Y.Z        # Delete local tag
pnpm release:patch       # Retry
```

### Scenario 2: Tag Succeeds, Push Fails
**Cause**: Network issue, authentication problem

**Recovery**:
```bash
git push origin main     # Push commit manually
git push origin vX.Y.Z   # Push tag manually
```

### Scenario 3: Version Bumped, Commit Fails
**Cause**: Unexpected git error

**Recovery**:
```bash
git checkout packages/maproom-mcp/package.json  # Revert version
pnpm release:patch                               # Retry
```

## Dependencies

### External Dependencies (Already Present)
- Node.js >= 18 (specified in package.json)
- Git (command-line tool)
- pnpm (for running scripts)

### Node Modules (Already Present)
- `fs` (built-in): Read package.json for new version
- `path` (built-in): Resolve file paths
- `child_process` (built-in): Execute git commands

### No New Dependencies Required
All functionality uses Node.js built-ins.

## Security Considerations

### Git Credentials
Scripts assume developer has configured git credentials (SSH keys or token). No credential handling in scripts.

### Safe Operations
- Only operates on package.json (no system files)
- Only creates git commits/tags (no destructive operations)
- No network operations except git push (well-understood)
- No user input sanitization needed (version type is enum)

### Risks: None
This is a low-risk change. Worst case: developer pushes wrong version tag and needs to delete it.

## Maintenance

### Future Enhancements (Not Now)
If needed later, could add:
- Dry-run mode (`--dry-run` flag)
- Interactive confirmation
- Changelog generation
- GitHub release creation
- Pre-release version support (alpha, beta, rc)

### Backward Compatibility
- Existing `bump-version.js` remains unchanged
- Could still call it directly if needed
- No breaking changes to package.json structure

## Alternative Designs Considered

### Alternative 1: Use `npm version`
Built-in npm command that bumps version, commits, and tags.

**Pros**: One command, well-tested
**Cons**: Less control over commit message format, no push included

**Decision**: Rejected. We want specific commit message format and push behavior.

### Alternative 2: Bash Script
Simple bash script with git commands.

**Pros**: Natural for git operations
**Cons**: Less portable (Windows), inconsistent with existing Node.js scripts

**Decision**: Rejected. Stay consistent with existing JavaScript tooling.

### Alternative 3: Use Tool (semantic-release, np)
Third-party release automation tool.

**Pros**: Feature-rich, well-tested
**Cons**: Overkill for simple use case, additional dependency, learning curve

**Decision**: Rejected. User wants simple solution, not overengineered.

## Summary

Simple, focused architecture:
- One new script (`release.js`)
- Reuses existing version bumping
- Sequential operations with clear error handling
- No new dependencies
- Easy to test and maintain
- Integrates cleanly with future GitHub Actions workflow

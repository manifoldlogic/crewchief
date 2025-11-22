# Ticket: CIFIX-3001: Update project documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- documentation-specialist
- verify-ticket
- commit-ticket

## Summary
Update `.github/CLAUDE.md` with comprehensive troubleshooting section covering both test workflow and Docker build common issues, consolidating lessons learned from Phase 1 and Phase 2.

## Background
Phases 1 and 2 fixed critical CI failures related to pnpm version management (CIFIX-1002) and Docker build prerequisites (CIFIX-2004). To prevent future regressions and help developers troubleshoot similar issues, we need central documentation that:

1. Explains how pnpm version is managed in test workflows
2. Documents Docker build prerequisites (daemon-client dist/)
3. Lists common failure scenarios with fixes
4. Provides prevention strategies and best practices

This ticket consolidates implementation knowledge into a single authoritative troubleshooting guide in `.github/CLAUDE.md`.

## Acceptance Criteria
- [ ] `.github/CLAUDE.md` has "Troubleshooting Workflows" section added
- [ ] Test workflow pnpm management documented with examples
- [ ] Docker build prerequisites clearly explained
- [ ] Common failure scenarios listed with fixes (minimum 4 scenarios)
- [ ] Prevention strategies and best practices documented
- [ ] Markdown syntax is valid (verified with markdownlint or manual review)

## Technical Requirements

### File to Modify
- `.github/CLAUDE.md`

### Section Structure
Add "Troubleshooting Workflows" section with three subsections:
1. **Test Workflow** - pnpm version management and common issues
2. **Docker Build** - prerequisites and common issues
3. **Best Practices** - prevention strategies

### Content Requirements
- Code examples for verification commands
- Clear cause-effect-fix format for issues
- Validation scripts where applicable
- Cross-references to actual workflow files

## Implementation Notes

Add the following section to `.github/CLAUDE.md`:

```markdown
## Troubleshooting Workflows

### Test Workflow

**pnpm Version Management:**
- pnpm version auto-detected from `package.json` packageManager field
- To change pnpm version: Update `package.json` ONLY (not workflow YAML)
- Do NOT add explicit `version:` to `pnpm/action-setup@v4`

**Common Issues:**

#### "Multiple versions of pnpm specified"
- **Cause**: Explicit version in workflow + packageManager in package.json
- **Fix**: Remove explicit `with: version:` from `.github/workflows/test.yml`
- **Prevention**: Never add version field to pnpm/action-setup step
- **Verify**: `grep -A 3 "pnpm/action-setup" .github/workflows/test.yml` should not show `with: version:`

#### "pnpm command not found"
- **Cause**: packageManager field missing or malformed in package.json
- **Fix**: Verify `jq -r '.packageManager' package.json` returns valid value
- **Format**: Must be `pnpm@<version>+sha512...`

---

### Docker Build

**Prerequisites:**
- **CRITICAL**: Run `pnpm build` before `docker build`
- daemon-client must be compiled to dist/ directory
- Failure results in "COPY failed: file not found" error

**Common Issues:**

#### "Unsupported URL Type workspace:"
- **Cause**: npm used instead of pnpm (doesn't understand workspace: protocol)
- **Fix**: Verify Dockerfile has `RUN npm install -g pnpm@10.12.1`
- **Verify**: `grep "npm install -g pnpm" packages/maproom-mcp/config/Dockerfile.combined`

#### "daemon-client dist not found" in Docker
- **Cause**: daemon-client dist/ not built or not copied to Docker context
- **Fix**: Run `pnpm build` at repository root before `docker build`
- **Verify**: `ls -la packages/daemon-client/dist/` shows index.js, client.js

#### "pnpm version mismatch" warning
- **Cause**: Dockerfile pnpm version doesn't match package.json
- **Fix**: Update Dockerfile line 41: `RUN npm install -g pnpm@<version>`
- **Check versions**:
  ```bash
  PACKAGE_PNPM=$(jq -r '.packageManager | split("@")[1] | split("+")[0]' package.json)
  DOCKERFILE_PNPM=$(grep "npm install -g pnpm@" packages/maproom-mcp/config/Dockerfile.combined | grep -oP 'pnpm@\K[0-9.]+')
  echo "package.json: $PACKAGE_PNPM"
  echo "Dockerfile: $DOCKERFILE_PNPM"
  ```

---

### Best Practices

**Updating pnpm Version:**
1. Update `package.json` packageManager field
2. Update `packages/maproom-mcp/config/Dockerfile.combined` (line 41)
3. Verify versions match with validation script above
4. Test locally before pushing to CI

**Before Pushing:**
- Run `yamllint .github/workflows/*.yml` to validate workflow syntax
- Run `pnpm build` to ensure workspace packages build
- Run local Docker build to verify Dockerfile changes
```

### Validation Commands

After implementation, verify completeness:

```bash
# Verify section exists
grep "## Troubleshooting Workflows" .github/CLAUDE.md

# Verify Docker prerequisites documented
grep -i "critical.*pnpm build" .github/CLAUDE.md

# Verify common issues documented
grep -A 3 "Multiple versions of pnpm" .github/CLAUDE.md
grep -A 3 "daemon-client dist not found" .github/CLAUDE.md

# Validate markdown syntax (if markdownlint available)
markdownlint .github/CLAUDE.md || echo "Markdown linter not available, skipping"
```

## Dependencies

**Requires:**
- CIFIX-1002 (test workflow fixes - provides context for pnpm management)
- CIFIX-2004 (Docker build fixes - provides context for build prerequisites)

**Blocks:**
- None (documentation-only, no downstream dependencies)

## Risk Assessment

- **Risk**: None
  - **Mitigation**: Documentation-only change, no code modifications

## Files/Packages Affected

- `.github/CLAUDE.md` - Add troubleshooting section

## Planning References

- CIFIX Phase 1: Test workflow stabilization
- CIFIX Phase 2: Docker build improvements
- CIFIX-1002: pnpm version management implementation
- CIFIX-2004: Dockerfile workspace protocol fix

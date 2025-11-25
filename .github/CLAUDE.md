# CLAUDE.md - .github Directory

Working with GitHub workflows at `/.github`.

## Active Workflows

```
workflows/
├── release-maproom-mcp.yml  # npm publish for @crewchief/maproom-mcp
└── test.yml                 # CI tests
```

## Workflows

### `release-maproom-mcp.yml`
Publishes `@crewchief/maproom-mcp` to npm.
- Trigger: Version tags (`@crewchief/maproom-mcp@v*.*.*`)
- Builds TypeScript, bundles Rust binaries, publishes package

## Debug Failed Workflow

```bash
# View logs
gh run list --workflow=release-maproom-mcp.yml
gh run view <run-id> --log

# Re-run
gh run rerun <run-id>
```

## Troubleshooting Workflows

### Test Workflow

**pnpm Version Management:**
- pnpm version is auto-detected from `package.json` packageManager field
- To change pnpm version: Update `package.json` ONLY (not workflow YAML)
- Do NOT add explicit `version:` to `pnpm/action-setup@v4`

**Common Issues:**

#### "Multiple versions of pnpm specified"
- **Cause**: Explicit version in workflow + packageManager in package.json
- **Fix**: Remove explicit `with: version:` from `.github/workflows/test.yml`
- **Prevention**: Never add version field to pnpm/action-setup step

#### "pnpm command not found"
- **Cause**: packageManager field missing or malformed
- **Fix**: Verify `jq -r '.packageManager' package.json` returns valid value
- **Format**: Must be `pnpm@<version>+sha512...`

---

### CI Health Check

Run these commands to verify CI configuration is correct:

```bash
# Test workflow health
yamllint .github/workflows/test.yml
jq -r '.packageManager' package.json
grep -c "with: version:" .github/workflows/test.yml  # Should be 0

# Build health
pnpm build
ls -la packages/daemon-client/dist/ | wc -l  # Should show multiple files

echo "✅ All health checks passed"
```

---

### Emergency Rollback Procedures

**If test workflow broken:**
```bash
# Option 1: Revert to previous workflow
git revert <commit-sha-of-fix>
git push

# Option 2: Temporarily add explicit version (not recommended long-term)
# Edit .github/workflows/test.yml:
# - name: Setup pnpm
#   uses: pnpm/action-setup@v4
#   with:
#     version: 10  # Temporary fix while debugging
```

**Validation after rollback:**
- Test workflow: Trigger manual run in GitHub Actions
- Release workflow: Create test tag and monitor build

## Secrets Used

Set in repository settings (Settings → Secrets and variables → Actions):
- `NPM_TOKEN` - npm publish auth
- `GITHUB_TOKEN` - Auto-provided by GitHub

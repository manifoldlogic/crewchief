# CLAUDE.md - .github Directory

Working with GitHub workflows at `/.github`.

## Active Workflows

```
workflows/
├── release-all.yml          # Coordinated release (all packages in order)
├── release-cli.yml          # Individual: @crewchief/cli to npm
├── release-maproom-mcp.yml  # Individual: @crewchief/maproom-mcp to npm
├── release-vscode-maproom.yml # Individual: vscode-maproom to marketplaces
├── reusable-rust-build.yml  # Shared Rust binary builds
├── reusable-typescript-build.yml # Shared TypeScript builds
└── test.yml                 # CI tests (SQLite-first)
```

## CI Testing Philosophy: SQLite-First

**Default Backend:** SQLite - zero configuration, no external services required
**Integration Backend:** PostgreSQL - for team sharing and production validation

### Test Job Organization

| Job | Backend | Dependencies | Purpose |
|-----|---------|--------------|---------|
| `test-sqlite-e2e` | SQLite | None | CLI end-to-end tests |
| `test-mcp-sqlite` | SQLite | None | TypeScript MCP server tests |
| `test-rust-sqlite` | SQLite | None | Rust library tests (in-memory) |
| `test-postgres` | PostgreSQL | Service container | TypeScript integration tests |
| `test-rust-postgres` | PostgreSQL | None | Rust compilation validation |

### When to Add Tests

**Add SQLite tests (default):**
- Testing new CLI commands
- Testing MCP server features
- Unit testing Rust functions
- Most development and feature work

**Add PostgreSQL tests:**
- Testing concurrent access patterns
- Validating PostgreSQL-specific features (recursive CTEs, parallel queries)
- Team sharing / multi-user scenarios
- Production deployment validation

### Adding New Test Jobs

For SQLite tests (recommended):
```yaml
my-new-test:
  name: My Feature Test (SQLite)
  runs-on: ubuntu-latest
  steps:
    # ... test steps
    - name: Job Summary
      if: always()
      run: |
        echo "## 🗄️ My Feature Test" >> $GITHUB_STEP_SUMMARY
        echo "**Backend:** SQLite (primary)" >> $GITHUB_STEP_SUMMARY
```

For PostgreSQL tests (when needed):
```yaml
my-postgres-test:
  name: My Feature Test (PostgreSQL Integration)
  runs-on: ubuntu-latest
  services:
    postgres-test:
      image: pgvector/pgvector:pg16
      # ... service config
  steps:
    # ... test steps
    - name: Job Summary
      if: always()
      run: |
        echo "## 🐘 My Feature Test" >> $GITHUB_STEP_SUMMARY
        echo "**Backend:** PostgreSQL (integration)" >> $GITHUB_STEP_SUMMARY
```

## Workflows

### Coordinated Release System

The project uses a coordinated release system to ensure packages are released in the correct dependency order.

**Required Order:** `@crewchief/cli` → `@crewchief/daemon-client` → `@crewchief/maproom-mcp` → `vscode-maproom`

**Configuration:** `/release-config.json` defines:
- Release order and dependencies
- Version requirements (e.g., MCP requires CLI >= 1.5.0)
- Validation rules (clean working tree, passing tests, etc.)

### `release-all.yml` (Recommended)
Coordinated release of all packages in correct order.
- Trigger: Manual workflow dispatch
- Options: release type (patch/minor/major), dry run, packages to release
- Builds Rust binaries once (shared), then releases packages sequentially
- Validates dependencies before each package release

### `release-cli.yml`
Publishes `@crewchief/cli` to npm.
- Trigger: Version tags (`@crewchief/cli@v*.*.*`) or manual
- Builds TypeScript, bundles Rust binaries, publishes package

### `release-maproom-mcp.yml`
Publishes `@crewchief/maproom-mcp` to npm.
- Trigger: Version tags (`@crewchief/maproom-mcp@v*.*.*`)
- Builds TypeScript, bundles Rust binaries, publishes package

### `release-vscode-maproom.yml`
Publishes `vscode-maproom` to VS Code Marketplace and Open VSX.
- Trigger: Manual workflow dispatch (requires version input)
- Builds TypeScript, bundles Rust binaries, creates VSIX, publishes

### Local Release Commands

```bash
# Dry run to see what would be released
pnpm release:dry

# Release all packages (coordinated)
pnpm release:all          # Auto-detect changes
pnpm release:all:patch    # Patch bump all
pnpm release:all:minor    # Minor bump all
pnpm release:all:major    # Major bump all

# Release specific packages
pnpm release -p @crewchief/cli @crewchief/daemon-client

# Skip confirmation prompts
pnpm release --all -y
```

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
- `VSCE_PAT` - VS Code Marketplace Personal Access Token
- `OVSX_PAT` - Open VSX Registry Personal Access Token
- `GITHUB_TOKEN` - Auto-provided by GitHub

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
└── test.yml                 # CI tests (SQLite-only)
```

## CI Testing Philosophy: SQLite-Only

**Backend:** SQLite — zero configuration, no external services required. PostgreSQL was intentionally removed.

### Test Job Organization

| Job | Trigger | Purpose |
|-----|---------|---------|
| `changes` | Always | Detect which paths changed (~5s) |
| `test-rust` | Rust code changes | Rust `maproom` crate tests (in-memory SQLite) |
| `test-typescript` | TypeScript code changes | Unit tests for cli, vscode-maproom, daemon-client |
| `test-daemon-integration` | daemon-client or Rust changes | Integration tests requiring a running daemon binary |
| `test-sqlite-e2e` | Rust or E2E test changes | CLI end-to-end tests with SQLite backend |
| `test-mcp-sqlite` | MCP or Rust changes | MCP server SQLite backend test |
| `test-performance-regression` | Rust code changes | Performance budget validation (<20ms overhead, <10KB response) |

**Note:** `maproom-mcp` is intentionally excluded from `test-typescript`. MCP has its own dedicated job (`test-mcp-sqlite`) that runs `pnpm test:sqlite` against a pre-indexed SQLite fixture. The general TypeScript job tests cli, vscode-maproom, and daemon-client only.

### Three-Tier Test Classification

| Tier | Description | CI Behavior | Mechanism |
|------|-------------|-------------|-----------|
| Unit | No external deps, fast, deterministic | Always runs in default CI job | No skip annotation needed |
| Integration | Requires binary, database, or daemon | Separate CI job or `vitest.integration.config.ts` | `#[ignore = "reason"]` (Rust) or separate vitest config (TypeScript) |
| External | Requires Ollama, GCP, OpenAI, etc. | Never runs in CI | `#[ignore = "Requires <service>"]` or `skipIf` with reason |

**Rust:** Use `#[ignore = "Requires X"]` for integration/external tests. Run ignored tests locally with `cargo test -- --ignored`.

**TypeScript:** Use a separate `vitest.integration.config.ts` to isolate tests that need a running daemon or binary. Run with `pnpm test:integration`.

### When to Add Tests

- Testing new CLI commands → add to `packages/cli/tests/`
- Testing MCP server features → add to `packages/maproom-mcp/tests/`
- Unit testing Rust functions → add to `crates/maproom/`
- Tests requiring daemon binary → add to integration config, runs in `test-daemon-integration`
- Performance budgets → add to `crates/maproom/tests/performance_regression_test.rs`

### Adding New Test Jobs

Follow the existing pattern in `test.yml`:

```yaml
my-new-test:
  name: My Feature Test
  needs: changes
  if: ${{ needs.changes.outputs.rust == 'true' || needs.changes.outputs.workflow == 'true' }}
  runs-on: blacksmith-4vcpu-ubuntu-2404
  steps:
    - uses: actions/checkout@v4
    # ... setup and test steps
    - name: Job Summary
      if: always()
      run: |
        echo "## 🗄️ My Feature Test" >> $GITHUB_STEP_SUMMARY
        echo "**Backend:** SQLite" >> $GITHUB_STEP_SUMMARY
```

Key points:
- Gate on the `changes` job to skip when irrelevant code hasn't changed
- Use `blacksmith-4vcpu-ubuntu-2404` runners (not `ubuntu-latest`)
- Add a Job Summary step for visibility in GitHub Actions UI

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

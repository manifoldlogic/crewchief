# .github — CI/CD Workflows

## CI Testing Philosophy: SQLite-Only

Zero configuration, no external services. PostgreSQL was intentionally removed.

### Three-Tier Test Classification

| Tier | Description | CI Behavior | Mechanism |
|------|-------------|-------------|-----------|
| Unit | No external deps, fast | Always runs | No annotation needed |
| Integration | Requires binary/database/daemon | Separate CI job or `vitest.integration.config.ts` | `#[ignore = "reason"]` (Rust) or separate config (TS) |
| External | Requires Ollama/GCP/OpenAI | Never in CI | `#[ignore = "Requires <service>"]` or `skipIf` |

**Rust**: `cargo test -- --ignored` to run integration tests locally.
**TypeScript**: `pnpm test:integration` for daemon-dependent tests.

### When to Add Tests

- CLI commands → `packages/cli/tests/`
- Rust functions → `crates/maproom/`
- Daemon-dependent → `vitest.integration.config.ts`, run locally
- Performance budgets → `crates/maproom/tests/performance_regression_test.rs`

### Adding New Test Jobs

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
        echo "## My Feature Test" >> $GITHUB_STEP_SUMMARY
```

### Pitfalls

- **Blacksmith runners**: Use `blacksmith-4vcpu-ubuntu-2404`, not `ubuntu-latest`
- **pnpm version**: Auto-detected from `package.json` `packageManager` field. Do NOT add explicit `version:` to `pnpm/action-setup@v4` — causes "Multiple versions of pnpm specified" error.
- **Gate on changes job**: All test jobs use `needs: changes` to skip when irrelevant paths unchanged

## Release System

**Required order**: `@crewchief/cli` → `@crewchief/daemon-client` → `@crewchief/maproom-mcp` → `vscode-maproom`

Configuration in `/release-config.json` defines order, version requirements, and validation rules.

Workflows:
- `release-all.yml` — Coordinated release (recommended, manual dispatch)
- `release-cli.yml` — CLI to npm (tag trigger: `@crewchief/cli@v*`)
- `release-maproom-mcp.yml` — MCP to npm (tag trigger)
- `release-vscode-maproom.yml` — Extension to VS Code Marketplace + Open VSX (manual)

## Secrets

- `NPM_TOKEN` — npm publish auth
- `VSCE_PAT` — VS Code Marketplace
- `OVSX_PAT` — Open VSX Registry
- `GITHUB_TOKEN` — Auto-provided

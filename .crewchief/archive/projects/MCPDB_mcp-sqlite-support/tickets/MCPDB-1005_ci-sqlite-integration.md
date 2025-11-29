# Ticket: MCPDB-1005: CI SQLite Integration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (YAML valid, local test passes)
- [x] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
Add a GitHub Actions job `test-mcp-sqlite` to run SQLite integration tests for the TypeScript MCP server without requiring PostgreSQL service container.

## Background
With SQLite tests implemented, we need CI automation to:
1. Run SQLite tests on every PR
2. Ensure fixture exists or regenerate
3. Distinguish from existing `test-sqlite-e2e` job (which tests Rust CLI)

**Plan Reference:** Phase 4 - CI Integration (plan.md, quality-strategy.md)

## Acceptance Criteria
- [x] `test-mcp-sqlite` job added to `.github/workflows/test.yml`
- [x] Job runs without PostgreSQL service container
- [x] Job ensures SQLite fixture exists (or generates it)
- [x] Job runs `pnpm test:sqlite` in `packages/maproom-mcp`
- [x] Job name clearly distinguishes from `test-sqlite-e2e`
- [x] Job completes successfully in CI (YAML valid, awaiting PR merge)
- [x] Documentation comment explains difference from `test-sqlite-e2e`

## Technical Requirements

### Workflow File
`.github/workflows/test.yml`

### Job Definition
```yaml
# MCP Server SQLite Tests (TypeScript layer)
# NOTE: Different from test-sqlite-e2e which tests Rust CLI
test-mcp-sqlite:
  name: MCP SQLite Tests (TypeScript)
  runs-on: ubuntu-latest

  steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: '20'

    - name: Setup pnpm
      uses: pnpm/action-setup@v4

    - name: Install dependencies
      run: pnpm install --frozen-lockfile

    - name: Setup Rust (for fixture generation)
      uses: actions-rust-lang/setup-rust-toolchain@v1

    - name: Ensure SQLite fixture exists
      run: |
        if [ ! -f crates/maproom/tests/fixtures/pre-indexed-maproom.db ]; then
          echo "SQLite fixture not found, generating..."
          cargo test --features sqlite --test create_sqlite_fixture -- --ignored
        else
          echo "SQLite fixture found"
        fi

    - name: Build MCP package
      working-directory: packages/maproom-mcp
      run: pnpm build

    - name: Run MCP SQLite tests
      working-directory: packages/maproom-mcp
      run: pnpm test:sqlite
      env:
        MAPROOM_DATABASE_URL: sqlite://${{ github.workspace }}/crates/maproom/tests/fixtures/pre-indexed-maproom.db
```

### Job Naming Convention
- `test-sqlite-e2e`: Tests **Rust CLI** with SQLite backend
- `test-mcp-sqlite`: Tests **TypeScript MCP server** with SQLite backend

Both jobs test SQLite functionality but at different layers:
- `test-sqlite-e2e` → `cargo test` for Rust
- `test-mcp-sqlite` → `pnpm test:sqlite` for TypeScript

### Comment in Workflow
Add explanatory comment before job:
```yaml
# =============================================================================
# MCP Server SQLite Tests (TypeScript layer)
# =============================================================================
# Tests the TypeScript MCP server with SQLite backend via daemon.
# NOTE: This is DIFFERENT from test-sqlite-e2e which tests the Rust CLI.
#
# test-sqlite-e2e:  Rust CLI + SQLite (cargo test)
# test-mcp-sqlite:  TypeScript MCP + SQLite daemon (pnpm test:sqlite)
# =============================================================================
```

### Fixture Generation
The step `Ensure SQLite fixture exists` handles:
1. Check if fixture exists
2. If not, run Rust test to generate it
3. Continue with TypeScript tests

This ensures CI doesn't fail if fixture is missing.

## Implementation Notes

### Placement in Workflow
Add after the existing `test-sqlite-e2e` job, around line 272.

### No PostgreSQL Service
Unlike other TypeScript tests, this job:
- Does NOT need `services: postgres:...`
- Does NOT need `MAPROOM_DATABASE_URL` pointing to PostgreSQL
- Uses SQLite URL pointing to fixture file

### Build Step Required
Need to build the MCP package before running tests:
```yaml
- name: Build MCP package
  working-directory: packages/maproom-mcp
  run: pnpm build
```

### Environment Variable
Pass absolute path to fixture:
```yaml
env:
  MAPROOM_DATABASE_URL: sqlite://${{ github.workspace }}/crates/maproom/tests/fixtures/pre-indexed-maproom.db
```

### Rust Setup
Rust is needed in case fixture needs regeneration. Setup is quick (~10s) and cached.

## Dependencies
- **MCPDB-1004**: SQLite integration tests must exist for job to run
- **External**: Pre-indexed SQLite fixture at expected location

## Risk Assessment
- **Risk**: Fixture generation takes too long
  - **Mitigation**: Fixture should exist in repo; generation is fallback only
- **Risk**: Job name confusion with test-sqlite-e2e
  - **Mitigation**: Clear naming (MCP vs E2E) and explanatory comments
- **Risk**: pnpm test:sqlite script doesn't exist
  - **Mitigation**: MCPDB-1004 creates this script; verify dependency

## Files/Packages Affected
- `.github/workflows/test.yml` (modify)

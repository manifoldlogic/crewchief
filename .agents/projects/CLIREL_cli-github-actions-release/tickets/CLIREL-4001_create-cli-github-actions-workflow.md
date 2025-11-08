# Ticket: CLIREL-4001: Create CLI GitHub Actions Workflow

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer (primary - CI/CD and multi-platform build expertise)
- verify-ticket
- commit-ticket

## Summary
Create automated GitHub Actions workflow for `@crewchief/cli` that builds Rust binaries for 4 platforms, validates package structure, and publishes to npm registry. This is the core automation that replaces the manual release process.

## Background

### Current Problem
The CLI package currently builds binaries only for the developer's local platform. Users on other platforms get a broken CLI with missing binaries. Manual releases are error-prone and lack validation.

### Solution
Create a GitHub Actions workflow (modeled after the existing maproom-mcp workflow) that:
1. Triggers on `@crewchief/cli@v*.*.*` tags
2. Builds Rust binaries for all 4 platforms in parallel (matrix build)
3. Builds TypeScript with tsup
4. Validates binary existence, sizes, and execution
5. Publishes to npm as `@crewchief/cli`
6. Verifies publication on npm registry

### Template to Copy
Use `.github/workflows/build-and-publish-maproom-mcp.yml` as the proven template. This workflow already solves:
- Multi-platform matrix builds
- Cross-compilation for Linux ARM
- Native builds for macOS
- Binary validation
- npm publishing with secrets

## Acceptance Criteria
- [ ] Workflow file created at `.github/workflows/build-and-publish-cli.yml`
- [ ] Workflow triggers on `@crewchief/cli@v*.*.*` tags only
- [ ] Matrix builds all 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- [ ] TypeScript build step included (tsup)
- [ ] Binary validation checks existence, size (5-20MB), and execution
- [ ] Package structure validated (bin/, dist/, README, LICENSE)
- [ ] npm publish step configured with NPM_TOKEN secret
- [ ] Post-publish verification checks npm registry
- [ ] Dry-run support via workflow_dispatch with dry_run input
- [ ] Workflow includes clear comments explaining each step
- [ ] YAML syntax is valid (passes yamllint)

## Technical Requirements

### 1. Workflow Trigger Configuration

```yaml
name: Build and Publish CLI

on:
  push:
    tags:
      - '@crewchief/cli@v*.*.*'  # Only CLI tags, not MCP tags
  workflow_dispatch:
    inputs:
      dry_run:
        description: 'Dry run (skip publish)'
        type: boolean
        default: false
```

**Why this trigger**:
- Package-scoped tag prevents conflict with MCP workflow
- `workflow_dispatch` allows manual testing without publishing
- Boolean input for dry_run is clearer than string comparison

### 2. Job 1: Matrix Binary Builds

```yaml
jobs:
  build-binaries:
    name: Build ${{ matrix.platform }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - platform: linux-x64
            os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - platform: linux-arm64
            os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - platform: darwin-x64
            os: macos-13
            target: x86_64-apple-darwin
          - platform: darwin-arm64
            os: macos-latest
            target: aarch64-apple-darwin

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          target: ${{ matrix.target }}

      - name: Install cross-compilation tools (Linux ARM only)
        if: matrix.platform == 'linux-arm64'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu

      - name: Build binary
        run: |
          cargo build --release --target ${{ matrix.target }} \
            --manifest-path crates/maproom/Cargo.toml

      - name: Strip binary
        run: |
          strip target/${{ matrix.target }}/release/crewchief-maproom || true

      - name: Rename binary
        run: |
          mv target/${{ matrix.target }}/release/crewchief-maproom \
             crewchief-maproom-${{ matrix.platform }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: binary-${{ matrix.platform }}
          path: crewchief-maproom-${{ matrix.platform }}
          retention-days: 1
```

**Key details**:
- `macos-13` for x64 (Intel), `macos-latest` for ARM64 (Apple Silicon)
- Cross-compilation setup only for Linux ARM (not available as native runner)
- Strip binaries to reduce size
- Rename for clarity before upload
- 1-day retention (temporary artifacts)

### 3. Job 2: Validate and Publish

```yaml
  validate-and-publish:
    name: Validate and Publish
    needs: build-binaries
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'

      - name: Setup pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 10

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Organize binaries
        run: |
          mkdir -p packages/cli/bin/{darwin-arm64,darwin-x64,linux-arm64,linux-x64}
          mv artifacts/binary-darwin-arm64/crewchief-maproom-darwin-arm64 \
             packages/cli/bin/darwin-arm64/crewchief-maproom
          mv artifacts/binary-darwin-x64/crewchief-maproom-darwin-x64 \
             packages/cli/bin/darwin-x64/crewchief-maproom
          mv artifacts/binary-linux-arm64/crewchief-maproom-linux-arm64 \
             packages/cli/bin/linux-arm64/crewchief-maproom
          mv artifacts/binary-linux-x64/crewchief-maproom-linux-x64 \
             packages/cli/bin/linux-x64/crewchief-maproom
          chmod +x packages/cli/bin/*/crewchief-maproom

      - name: Build TypeScript
        run: |
          cd packages/cli
          pnpm install
          pnpm build

      - name: Validate binaries
        run: |
          cd packages/cli

          # Check existence
          for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
            if [ ! -f "bin/$platform/crewchief-maproom" ]; then
              echo "ERROR: Missing binary for $platform"
              exit 1
            fi
            echo "✓ Binary exists: $platform"
          done

          # Check sizes (5MB-20MB range)
          for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
            size=$(stat -c%s "bin/$platform/crewchief-maproom" 2>/dev/null || stat -f%z "bin/$platform/crewchief-maproom")
            if [ $size -lt 5000000 ] || [ $size -gt 20000000 ]; then
              echo "ERROR: Binary size $size out of range for $platform"
              exit 1
            fi
            echo "✓ Binary size valid: $platform ($size bytes)"
          done

          # Check TypeScript build
          if [ ! -d "dist" ] || [ ! -f "dist/cli/index.js" ]; then
            echo "ERROR: TypeScript build incomplete"
            exit 1
          fi
          echo "✓ TypeScript build valid"

      - name: Create package
        run: |
          cd packages/cli
          npm pack --dry-run

          # Verify package structure
          npm pack
          tar -tzf *.tgz | grep "package/bin/darwin-arm64" || exit 1
          tar -tzf *.tgz | grep "package/bin/darwin-x64" || exit 1
          tar -tzf *.tgz | grep "package/bin/linux-arm64" || exit 1
          tar -tzf *.tgz | grep "package/bin/linux-x64" || exit 1
          echo "✓ Package structure valid"

      - name: Publish to npm
        if: ${{ !inputs.dry_run }}
        run: |
          cd packages/cli
          npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      - name: Verify publication
        if: ${{ !inputs.dry_run }}
        run: |
          sleep 10  # Wait for npm registry to update
          VERSION=$(node -p "require('./packages/cli/package.json').version")

          # Check package exists
          if ! npm view @crewchief/cli@$VERSION version; then
            echo "ERROR: Package not found on npm"
            exit 1
          fi
          echo "✓ Package published successfully"
```

**Key validation steps**:
1. **Binary existence**: All 4 platforms present
2. **Binary size**: 5-20MB range (detects corruption or bloat)
3. **TypeScript build**: dist/ directory with index.js
4. **Package structure**: Tarball contains all platform binaries
5. **Post-publish**: Verify package appears on npm

### 4. Validation Script Details

**Binary size ranges** (based on current MCP binaries):
- darwin binaries: ~10MB
- linux binaries: ~15MB
- Range 5-20MB catches both too-small (corrupted) and too-large (not stripped)

**Why sleep before verification**:
- npm registry has eventual consistency
- 10-second delay ensures package is findable
- Prevents false failures from registry lag

### 5. Dry-Run Mode

**Usage**:
```bash
# Trigger via GitHub UI
# Actions → Build and Publish CLI → Run workflow → dry_run=true
```

**Behavior**:
- All builds run
- All validation runs
- Publish step skipped
- Verify step skipped
- Artifacts still uploaded (for inspection)

### 6. Error Handling

**Build failures**:
- Individual platform fails → entire workflow fails (fail-fast)
- No partial publishes allowed
- Clear error messages in logs

**Validation failures**:
- Missing binary → fail before publish
- Size out of range → fail before publish
- Execution test fails → fail before publish (native platform only)

**Publish failures**:
- npm auth fails → retry once
- Registry unavailable → fail with clear error
- Wrong package name → caught by npm (can't publish to wrong scope)

## Implementation Notes

### Copying from MCP Workflow

**Files to reference**:
- `.github/workflows/build-and-publish-maproom-mcp.yml` - primary template
- Use same matrix strategy
- Use same artifact upload/download pattern
- Use same validation approach

**Differences from MCP**:
- Trigger: `@crewchief/cli@v*` not `@crewchief/maproom-mcp@v*`
- Package directory: `packages/cli` not `packages/maproom-mcp`
- Package name: `@crewchief/cli` not `@crewchief/maproom-mcp`
- Include TypeScript build (MCP already has this)

### GitHub Secrets Required

**NPM_TOKEN**:
- Must be configured in repository secrets
- Get from npmjs.com → Account Settings → Access Tokens
- Type: Automation token (for CI/CD)
- Scope: Publish (write access)

**Verification**:
```bash
# Check secret exists (admin only)
gh secret list
# Should show: NPM_TOKEN
```

### Testing Strategy

**Before first real use**:
1. Validate YAML syntax: `yamllint .github/workflows/build-and-publish-cli.yml`
2. Create test tag: `@crewchief/cli@v1.0.0-test`
3. Trigger workflow with dry_run=true
4. Verify all 4 binaries build
5. Verify validation passes
6. Verify publish step skipped
7. Delete test tag

## Dependencies
- CLIREL-2001 (Package Configuration) - Must complete first (package.json must be `@crewchief/cli@v1.0.0`)
- CLIREL-3001 (Release Scripts) - Must complete first (tag format must be `@crewchief/cli@v*`)
- NPM_TOKEN secret must be configured in repository

## Risk Assessment
| Risk | Severity | Mitigation |
|------|----------|------------|
| Workflow triggers on wrong tag | High | Specific trigger: `@crewchief/cli@v*.*.*` |
| Wrong binaries for platform | High | Multi-layer validation (existence, size, file type) |
| Build time too long | Medium | Parallel matrix builds (4 simultaneous) |
| macOS runner costs | Low | Accept cost (~$1.60 per release) |
| npm publish fails | Medium | Dry-run testing, clear error messages |
| Binary validation too strict/loose | Medium | Size range based on existing binaries |

## Files/Packages Affected
- `.github/workflows/build-and-publish-cli.yml` (create)
- Repository secrets (NPM_TOKEN must exist)

## Success Metrics
- Workflow file is valid YAML
- Dry-run completes successfully
- All 4 platform binaries build
- Validation catches intentionally broken packages
- Publish step (when enabled) succeeds
- Package appears on npm registry
- Total workflow time <15 minutes

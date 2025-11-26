# Analysis: CI/CD Workflow Optimization

## Problem Definition

The CrewChief repository currently has inefficient GitHub Actions workflows that result in:

1. **Slow release cycles**: 25-30 minutes total for a single maproom-mcp release
2. **Redundant work**: Same binaries built multiple times, TypeScript compiled repeatedly
3. **Unnecessary test runs**: Tests trigger on documentation-only changes
4. **High maintenance burden**: 450+ lines of duplicated YAML across workflows
5. **Poor resource utilization**: No caching, workflows triggered simultaneously on same tag

### Current Workflow Inventory

#### 1. test.yml - Test Workflow
- **Triggers**: Every push to main, all PR events
- **Duration**: 5-8 minutes
- **Steps**:
  - PostgreSQL service container setup
  - Node.js 20 + pnpm installation
  - Full workspace dependency installation
  - Rust toolchain setup + **full release build** of crewchief-maproom
  - Database migrations
  - TypeScript test suite
- **Caching**: Rust only (Swatinem/rust-cache@v2)
- **Issues**:
  - No path filtering - runs on docs/config changes
  - No pnpm caching
  - Rebuilds Rust even when no Rust changes
  - No concurrency limits

#### 2. build-and-publish-cli.yml - CLI Package Release
- **Triggers**: Tags `@crewchief/cli@v*.*.*`, manual dispatch
- **Duration**: 12-15 minutes
- **Jobs**:
  - `build-binaries`: Matrix build for 4 platforms (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
  - `validate-and-publish`: Downloads artifacts, builds TypeScript, publishes to npm
- **Steps per platform**:
  - Rust toolchain + cross tool installation
  - Full release build with cross-compilation
  - Binary stripping and artifact upload
- **Issues**:
  - **NO Rust caching** - 8-12 minute builds every time
  - **NO pnpm caching** - Dependencies reinstalled every run
  - cross tool downloaded 4 times per run (once per Linux platform)
  - Identical to maproom-mcp workflow (duplication)

#### 3. build-and-publish-maproom-mcp.yml - Maproom MCP Package Release
- **Triggers**: Tags `@crewchief/maproom-mcp@v*.*.*`, manual dispatch
- **Duration**: 12-15 minutes
- **Structure**: Identical to CLI workflow
- **Issues**: Same as CLI workflow + triggers simultaneously with Docker workflow

#### 4. publish-maproom-mcp-image.yml - Docker Image Release
- **Triggers**: Tags `@crewchief/maproom-mcp@v*.*.*`, manual dispatch
- **Duration**: 8-10 minutes
- **Steps**:
  - Node.js 20 + pnpm setup
  - Full workspace dependency installation
  - **`pnpm build`** - builds ALL packages (currently failing - circular dependency)
  - Docker multi-platform build (linux/amd64, linux/arm64)
  - Trivy security scan
- **Caching**: Docker layer cache (GHA cache)
- **Issues**:
  - Triggers on same tag as npm publish workflow
  - Builds in parallel with npm workflow (redundant work)
  - Circular dependency in package.json build script
  - No pnpm caching

### Critical Redundancies

#### A. Simultaneous Workflows on Same Tag

When `@crewchief/maproom-mcp@v2.2.1` is pushed:
- `build-and-publish-maproom-mcp.yml` starts → builds 4 Rust binaries + TypeScript
- `publish-maproom-mcp-image.yml` starts simultaneously → builds Rust in Docker + TypeScript

**Result**: Same exact work done twice in parallel, consuming 2x CI minutes.

#### B. Duplicate Workflow Code (450+ lines)

CLI and Maproom-MCP workflows are 99% identical:
- Same 4-platform matrix
- Same Rust build commands
- Same cross-compilation setup
- Same artifact handling
- Only difference: package name

**Maintenance burden**: Every change must be applied to both files.

#### C. Missing Rust Caching in Release Workflows

- Test workflow uses `Swatinem/rust-cache@v2` → 2-4 minute builds
- Release workflows have NO caching → 8-12 minute builds
- cross tool installed 8 times per release (4× per workflow × 2 workflows)

**Impact**: Each release could be 50-70% faster with caching.

#### D. Test Workflow Runs on Non-Code Changes

No path filters means:
- Documentation PRs trigger full test suite
- `.github/` config changes run Rust builds
- `.agents/` planning docs trigger migrations

**Impact**: Estimated 80% of test runs are unnecessary.

#### E. No pnpm Store Caching

Every workflow runs `pnpm install --frozen-lockfile`:
- Downloads all 714 packages from registry
- 30-60 seconds per workflow
- No benefit from pnpm's efficient caching

**Impact**: 2-4 minutes wasted per day across all workflows.

#### F. Circular Dependency in Build Script

`package.json` line 11:
```json
"build": "node packages/cli/dist/cli/index.js build"
```

**Problem**: Tries to run compiled CLI before TypeScript is built.
**Impact**: Docker workflow fails immediately in CI.

## Industry Solutions and Best Practices

### GitHub Actions Optimization Patterns

#### 1. Reusable Workflows (workflow_call)

**Pattern**: Extract common build logic into reusable workflows that can be called by multiple workflows.

**Example from industry**:
```yaml
# .github/workflows/reusable-build.yml
on:
  workflow_call:
    inputs:
      platform:
        required: true
        type: string
    outputs:
      artifact_name:
        value: ${{ jobs.build.outputs.artifact }}

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Build for ${{ inputs.platform }}
        run: cargo build --release --target ${{ inputs.platform }}
```

**Benefits**:
- Single source of truth
- Changes propagate automatically
- Reduced YAML by 60-80%

**Adoption**: Used by Rust, TypeScript, Go ecosystems for cross-platform builds.

#### 2. Job Dependencies and Artifact Sharing

**Pattern**: Build once, share artifacts across jobs via upload/download-artifact.

**Example**:
```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build --release
      - uses: actions/upload-artifact@v4
        with:
          name: binary
          path: target/release/myapp

  test:
    needs: build
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: binary
```

**Benefits**:
- Compile once, test/publish/package multiple times
- Faster overall workflow
- Guaranteed consistent artifacts

#### 3. Comprehensive Caching Strategies

**Pattern**: Cache dependencies, build outputs, and tools.

**Rust caching** (industry standard):
```yaml
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: "crates/maproom -> target"
    shared-key: ${{ matrix.platform }}
```

**pnpm caching** (best practice):
```yaml
- name: Get pnpm store directory
  run: echo "STORE_PATH=$(pnpm store path --silent)" >> $GITHUB_ENV
- uses: actions/cache@v4
  with:
    path: ${{ env.STORE_PATH }}
    key: ${{ runner.os }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
```

**Benefits**:
- 50-70% faster builds (Rust)
- 40-60% faster installs (pnpm)
- Lower network usage

#### 4. Path-Based Workflow Filtering

**Pattern**: Only run workflows when relevant files change.

**Example**:
```yaml
on:
  push:
    paths:
      - 'src/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
  pull_request:
    paths:
      - 'src/**'
      - '**.rs'
```

**Benefits**:
- 70-90% reduction in unnecessary runs
- Faster PR feedback
- Lower CI costs

**Common exclusions**: `docs/**`, `*.md`, `.github/**` (except workflow being tested)

#### 5. Matrix Strategies with Caching

**Pattern**: Use matrix builds with shared caching for multi-platform releases.

**Example**:
```yaml
strategy:
  matrix:
    include:
      - target: x86_64-unknown-linux-gnu
        os: ubuntu-latest
        tool: cargo
      - target: aarch64-unknown-linux-gnu
        os: ubuntu-latest
        tool: cross

steps:
  - uses: Swatinem/rust-cache@v2
    with:
      shared-key: ${{ matrix.target }}
```

**Benefits**:
- Parallel builds with independent caches
- Faster overall CI time
- Scales to many platforms

### VSCode Extension Multi-Marketplace Publishing

#### Industry Patterns for Extension Publishing

**1. Microsoft VS Code Marketplace**
- **Tool**: `@vscode/vsce` CLI
- **Authentication**: Personal Access Token (VSCE_PAT)
- **Command**: `vsce publish`
- **Features**: Automatic versioning, changelog integration, pre-release support

**2. Open VSX Registry (Eclipse, VSCodium, etc.)**
- **Tool**: `ovsx` CLI
- **Authentication**: Personal Access Token (OVSX_PAT)
- **Command**: `ovsx publish`
- **Why needed**: VSCodium and other VS Code forks don't use Microsoft Marketplace

**3. Eclipse Marketplace**
- **Tool**: Eclipse Publisher
- **Less common**: Usually Open VSX is sufficient for Eclipse ecosystem

#### Common Automation Patterns

**Pattern A: Sequential Publishing** (simple, safe)
```yaml
jobs:
  publish:
    steps:
      - run: vsce package
      - run: vsce publish -p ${{ secrets.VSCE_PAT }}
      - run: ovsx publish -p ${{ secrets.OVSX_PAT }}
```

**Pattern B: Parallel Publishing** (faster)
```yaml
jobs:
  build:
    steps:
      - run: vsce package
      - uses: actions/upload-artifact@v4
        with:
          name: extension.vsix

  publish-vscode:
    needs: build
    steps:
      - run: vsce publish

  publish-ovsx:
    needs: build
    steps:
      - run: ovsx publish
```

**Pattern C: Conditional Publishing** (flexibility)
```yaml
- name: Publish to VS Code Marketplace
  if: ${{ secrets.VSCE_PAT != '' }}
  run: vsce publish

- name: Publish to Open VSX
  if: ${{ secrets.OVSX_PAT != '' }}
  run: ovsx publish
```

#### Version Management

**Industry standard**: Automated version bumping
```yaml
- name: Bump version
  run: npm version ${{ github.event.inputs.version_type }}

- name: Extract version
  id: version
  run: echo "version=$(jq -r .version package.json)" >> $GITHUB_OUTPUT

- name: Create tag
  run: git tag v${{ steps.version.outputs.version }}
```

#### Pre-Release Support

**Pattern**: Use pre-release tags for beta testing
```yaml
- name: Publish pre-release
  if: contains(github.ref, 'beta')
  run: vsce publish --pre-release
```

#### Changelog Automation

**Tools**:
- `conventional-changelog` - Generate from commit messages
- GitHub Releases API - Auto-create release notes
- `release-please` - Automated releases with conventional commits

**Example**:
```yaml
- name: Generate changelog
  uses: conventional-changelog/conventional-changelog@v1

- name: Create GitHub Release
  uses: actions/create-release@v1
  with:
    tag_name: ${{ github.ref }}
    body_path: ./CHANGELOG.md
```

### Reference Implementations

**1. Rust Foundation Workflows**
- Repository: rust-lang/rust
- Pattern: Reusable workflows for multi-platform builds
- Caching: Comprehensive sccache + cargo caching
- Artifacts: Shared between jobs

**2. TypeScript Team Workflows**
- Repository: microsoft/TypeScript
- Pattern: Matrix builds with artifact sharing
- Caching: npm cache + test result caching
- Publishing: Automated with npm provenance

**3. VSCode Extension Examples**
- Repository: microsoft/vscode-extension-samples
- Pattern: Sequential marketplace publishing
- Versioning: Automated version bumping
- Testing: Extension smoke tests before publish

## Current Project State

### Existing Infrastructure

**Strengths**:
- PostgreSQL with pgvector for integration tests
- Rust caching in test workflow (proof of concept)
- Docker layer caching working
- Clear separation of concerns (test, build, publish)

**Weaknesses**:
- No caching in critical release workflows
- Duplicate workflow code
- No path filtering
- No artifact sharing
- Workflows trigger simultaneously on same tag

### Metrics Before Optimization

**CI Performance**:
- Test workflow: 5-8 minutes (runs too often)
- CLI release: 12-15 minutes
- Maproom-MCP npm release: 12-15 minutes
- Maproom-MCP Docker release: 8-10 minutes (fails currently)
- **Total for maproom-mcp release: 25-30 minutes**

**Workflow Code**:
- Total lines: ~600 YAML
- Duplicated: ~450 lines (75%)
- Reusable workflows: 0

**Caching**:
- Rust: Only in test workflow
- pnpm: None
- Docker: Enabled but not optimized
- Cache hit rates: <10% (Rust), 0% (pnpm)

### Resource Utilization

**CI Minutes (estimated per release)**:
- CLI release: 4 platforms × 12 min = 48 minutes
- Maproom-MCP npm: 4 platforms × 12 min = 48 minutes
- Maproom-MCP Docker: 1 job × 10 min = 10 minutes
- **Total: 106 CI minutes per maproom-mcp release**

**Opportunities**:
- 50% savings from Rust caching alone
- 20% savings from pnpm caching
- 50% savings from workflow consolidation
- **Potential: 60-70% reduction in CI minutes**

## VSCode Extension Future State

### vscode-maproom Extension (Planned)

**Purpose**: VS Code extension for crewchief/maproom integration

**Publishing Requirements**:
- Microsoft VS Code Marketplace (primary)
- Open VSX Registry (VSCodium/Eclipse users)

**Version Strategy**:
- Semantic versioning aligned with maproom-mcp
- Pre-release channel for beta testing
- Automated changelog from conventional commits

**Build Requirements**:
- TypeScript compilation
- Extension packaging (vsce package)
- Webview bundling (if applicable)
- Icon/asset optimization

**Testing Requirements**:
- Extension smoke tests
- VS Code API compatibility tests
- Multi-version testing (VS Code stable + insiders)

**Publishing Workflow Design** (future):
```yaml
name: Publish VSCode Extension

on:
  push:
    tags:
      - '@crewchief/vscode-maproom@v*.*.*'

jobs:
  build:
    # Build extension
    # Run tests
    # Create .vsix package
    # Upload artifact

  publish-vscode:
    needs: build
    if: ${{ secrets.VSCE_PAT != '' }}
    # Download artifact
    # vsce publish

  publish-ovsx:
    needs: build
    if: ${{ secrets.OVSX_PAT != '' }}
    # Download artifact
    # ovsx publish

  create-release:
    needs: [publish-vscode, publish-ovsx]
    # Create GitHub release
    # Attach .vsix artifact
```

### Integration with Existing Workflows

**Shared Components**:
- Reusable TypeScript build workflow
- pnpm caching strategy
- Artifact management patterns
- Version extraction logic

**Unique Components**:
- Extension-specific build steps
- Multi-marketplace publishing
- VS Code API testing

## Research Findings

### Key Insights

1. **Reusable workflows are the solution to duplication**
   - Industry standard for monorepos
   - Reduces code by 60-80%
   - Easier to maintain and test

2. **Caching is critical for performance**
   - Rust caching: 50-70% faster builds
   - pnpm caching: 40-60% faster installs
   - Combined: 60%+ total improvement

3. **Path filters prevent waste**
   - 70-90% reduction in unnecessary runs
   - Simple to implement
   - Immediate impact

4. **Artifact sharing enables parallelization**
   - Build once, publish many times
   - Faster workflows
   - Consistent outputs

5. **Multi-marketplace publishing is standard for extensions**
   - Microsoft Marketplace + Open VSX minimum
   - Conditional publishing based on available secrets
   - Automated versioning reduces errors

### Success Patterns from Industry

**Monorepo CI/CD**:
- Separate workflows per package (different triggers)
- Shared reusable workflows for common tasks
- Comprehensive caching at all levels
- Path-based filtering to minimize runs

**Extension Publishing**:
- Build → Test → Package → Publish (linear)
- Parallel publishing to multiple marketplaces
- Automated changelog generation
- Pre-release channels for testing

**Performance Optimization**:
- Cache everything: dependencies, builds, tools
- Share artifacts between jobs
- Use matrix builds with isolated caches
- Monitor cache hit rates

## Conclusion

The current workflow setup has significant inefficiencies costing 60-70% extra CI time and creating maintenance burden through duplication. Industry-standard patterns (reusable workflows, comprehensive caching, artifact sharing, path filtering) can address all identified issues.

The aggressive optimization approach is justified by:
- Clear ROI: 60-70% faster releases
- Low risk: Industry-proven patterns
- Future-proof: Supports upcoming VSCode extension publishing
- Maintainable: Less code, clearer structure

**Next**: Design architecture implementing these patterns.

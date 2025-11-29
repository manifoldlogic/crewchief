# Architecture: CI/CD Workflow Optimization

## Overview

Design a modern, efficient CI/CD system using GitHub Actions reusable workflows, comprehensive caching, and artifact sharing to achieve 60-70% reduction in build times while eliminating duplication.

## Design Principles

1. **Build once, use many times**: Share artifacts between jobs instead of rebuilding
2. **Cache aggressively**: Cache dependencies, builds, and tools at every level
3. **Single source of truth**: Reusable workflows eliminate duplication
4. **Fail fast**: Run tests before expensive builds when possible
5. **Parallel when possible, sequential when necessary**: Maximize concurrency without dependencies
6. **MVP first, optimize later**: Ship working solution before perfection

## Target Architecture

### High-Level Workflow Structure

```
┌─────────────────────────────────────────────────────────────┐
│                    Reusable Workflows                        │
│  (Shared components called by package-specific workflows)   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  .github/workflows/reusable-rust-build.yml                  │
│  ├─ Inputs: package_name, platforms                         │
│  ├─ Matrix: [linux-x64, linux-arm64, darwin-x64, darwin-arm64] │
│  ├─ Caching: Rust (Swatinem/rust-cache@v2)                  │
│  └─ Outputs: Artifacts for all platforms                    │
│                                                              │
│  .github/workflows/reusable-typescript-build.yml            │
│  ├─ Inputs: workspace_filter (optional)                     │
│  ├─ Caching: pnpm store                                     │
│  └─ Outputs: TypeScript dist/ artifacts                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                  Package-Specific Workflows                  │
│         (Trigger on tags, orchestrate reusables)            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  test.yml (on push/PR)                                      │
│  ├─ Path filter: Only code changes                          │
│  ├─ Concurrency: Cancel in-progress for PRs                 │
│  └─ Caching: Rust + pnpm                                    │
│                                                              │
│  release-cli.yml (on @crewchief/cli@v* tags)                │
│  ├─ Job: build-rust (calls reusable)                        │
│  ├─ Job: build-typescript (calls reusable)                  │
│  └─ Job: publish-npm (depends on builds)                    │
│                                                              │
│  release-maproom-mcp.yml (on @crewchief/maproom-mcp@v*)     │
│  ├─ Job: build-rust (calls reusable)                        │
│  ├─ Job: build-typescript (calls reusable)                  │
│  ├─ Job: publish-npm (depends on builds)                    │
│  └─ Job: publish-docker (depends on builds, parallel w/ npm) │
│                                                              │
│  release-vscode-maproom.yml (future)                        │
│  ├─ Job: build-typescript (calls reusable)                  │
│  ├─ Job: package-extension                                  │
│  ├─ Job: publish-vscode (parallel)                          │
│  └─ Job: publish-ovsx (parallel)                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Phase 1: Immediate Fixes (MVP)

### 1.1 Fix package.json Build Script

**Current** (line 11):
```json
"build": "node packages/cli/dist/cli/index.js build"
```

**Problem**: Circular dependency - tries to run compiled CLI before TypeScript is built.

**Solution**:
```json
"build": "pnpm -r --filter='./packages/*' build"
```

**Rationale**:
- Uses pnpm's built-in recursive build
- Respects workspace dependencies
- No circular dependency
- Works in fresh checkouts (CI)

**Implementation**:
- File: `package.json`
- Change: Single line edit
- Risk: Low - aligns with pnpm best practices
- Testing: Run `pnpm build` in fresh checkout

### 1.2 Add Rust Caching to Release Workflows

**Files**: `build-and-publish-cli.yml`, `build-and-publish-maproom-mcp.yml`

**Implementation**:
```yaml
jobs:
  build-binaries:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            use_cross: true
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            use_cross: true
          - target: x86_64-apple-darwin
            os: macos-latest
            use_cross: false
          - target: aarch64-apple-darwin
            os: macos-latest
            use_cross: false

    steps:
      - uses: actions/checkout@v4

      # ADD THIS: Rust caching
      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: "crates/maproom -> target"
          shared-key: ${{ matrix.target }}
          cache-on-failure: true

      - name: Install cross (Linux only)
        if: matrix.use_cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      # Existing build steps...
```

**Benefits**:
- 50-70% faster builds (8-12 min → 2-4 min)
- Caches per target (isolated caches for each platform)
- Cache survives failures (useful during debugging)

**Configuration Details**:
- `workspaces`: Points to maproom crate
- `shared-key`: Isolates cache per platform
- `cache-on-failure`: Don't lose cache on transient errors

### 1.3 Add pnpm Store Caching

**Files**: All 4 workflows

**Implementation**:
```yaml
- name: Setup Node.js
  uses: actions/setup-node@v4
  with:
    node-version: '20'

- name: Setup pnpm
  uses: pnpm/action-setup@v4

# ADD THIS: pnpm store caching
- name: Get pnpm store directory
  shell: bash
  run: |
    echo "STORE_PATH=$(pnpm store path --silent)" >> $GITHUB_ENV

- name: Setup pnpm cache
  uses: actions/cache@v4
  with:
    path: ${{ env.STORE_PATH }}
    key: ${{ runner.os }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
    restore-keys: |
      ${{ runner.os }}-pnpm-store-

- name: Install dependencies
  run: pnpm install --frozen-lockfile
```

**Benefits**:
- 40-60% faster dependency installation
- Cross-workflow cache sharing
- Lower network usage

**Key Points**:
- Cache keyed on lock file hash (invalidates when dependencies change)
- Restore keys provide partial cache hits
- Works across all workflows (same cache key)

### 1.4 Add Path Filters to Test Workflow

**File**: `test.yml`

**Implementation**:
```yaml
on:
  push:
    branches: [main]
    paths:
      # Rust code
      - 'crates/**'
      - 'Cargo.toml'
      - 'Cargo.lock'

      # TypeScript code
      - 'packages/*/src/**'
      - 'packages/*/tests/**'
      - 'packages/**/package.json'

      # Dependencies
      - 'pnpm-lock.yaml'

      # Workflows
      - '.github/workflows/test.yml'

      # Migrations/schemas
      - 'packages/maproom-mcp/config/init.sql'
      - 'crates/maproom/migrations/**'

  pull_request:
    paths:
      # Same as push (DRY using YAML anchor if needed)
      - 'crates/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - 'packages/*/src/**'
      - 'packages/*/tests/**'
      - 'packages/**/package.json'
      - 'pnpm-lock.yaml'
      - '.github/workflows/test.yml'
      - 'packages/maproom-mcp/config/init.sql'
      - 'crates/maproom/migrations/**'
```

**Excluded Paths** (implicit):
- `docs/**`
- `*.md` files (except in code directories)
- `.crewchief/**`
- `.github/workflows/**` (except test.yml itself)
- `.devcontainer/**`
- Configuration files that don't affect tests

**Benefits**:
- 80% reduction in unnecessary test runs
- Faster PR feedback for docs changes
- Lower CI costs

**Edge Cases**:
- Workflow file itself always triggers (prevents breaking tests)
- Migrations always trigger (schema changes critical)

## Phase 2: Reusable Workflow Infrastructure

### 2.1 Reusable Rust Build Workflow

**File**: `.github/workflows/reusable-rust-build.yml`

**Interface Design**:
```yaml
name: Reusable Rust Build

on:
  workflow_call:
    inputs:
      package_name:
        description: 'Package name for artifact prefix'
        required: true
        type: string

      crate_path:
        description: 'Path to Rust crate (e.g., crates/maproom)'
        required: false
        type: string
        default: 'crates/maproom'

      binary_name:
        description: 'Binary name to build'
        required: false
        type: string
        default: 'crewchief-maproom'

      platforms:
        description: 'JSON array of platforms to build'
        required: false
        type: string
        default: '["linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64"]'

    outputs:
      artifact_prefix:
        description: 'Prefix for uploaded artifacts'
        value: ${{ jobs.build.outputs.artifact_prefix }}

jobs:
  build:
    name: Build ${{ matrix.config.target }}
    runs-on: ${{ matrix.config.os }}

    outputs:
      artifact_prefix: ${{ inputs.package_name }}-binaries

    strategy:
      fail-fast: false
      matrix:
        config:
          - platform: linux-x64
            target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            use_cross: true
          - platform: linux-arm64
            target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            use_cross: true
          - platform: darwin-x64
            target: x86_64-apple-darwin
            os: macos-latest
            use_cross: false
          - platform: darwin-arm64
            target: aarch64-apple-darwin
            os: macos-latest
            use_cross: false

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.config.target }}

      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: "${{ inputs.crate_path }} -> target"
          shared-key: ${{ matrix.config.target }}
          cache-on-failure: true

      - name: Install cross (Linux ARM64 only)
        if: matrix.config.use_cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Build binary
        run: |
          if [ "${{ matrix.config.use_cross }}" = "true" ]; then
            cross build --release --bin ${{ inputs.binary_name }} \
              --manifest-path ${{ inputs.crate_path }}/Cargo.toml \
              --target ${{ matrix.config.target }}
          else
            cargo build --release --bin ${{ inputs.binary_name }} \
              --manifest-path ${{ inputs.crate_path }}/Cargo.toml \
              --target ${{ matrix.config.target }}
          fi

      - name: Strip binary
        run: |
          BINARY_PATH="${{ inputs.crate_path }}/target/${{ matrix.config.target }}/release/${{ inputs.binary_name }}"
          strip --strip-all "$BINARY_PATH" 2>/dev/null || strip "$BINARY_PATH"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ inputs.package_name }}-${{ matrix.config.platform }}
          path: ${{ inputs.crate_path }}/target/${{ matrix.config.target }}/release/${{ inputs.binary_name }}
          if-no-files-found: error
          retention-days: 7
```

**Key Features**:
- Parameterized for any Rust binary
- Matrix build for all platforms
- Comprehensive caching
- Artifact upload per platform
- Fail-fast disabled (build all platforms even if one fails)

**Usage Example** (in release-cli.yml):
```yaml
jobs:
  build-rust:
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: cli
      binary_name: crewchief-maproom
      crate_path: crates/maproom
```

### 2.2 Reusable TypeScript Build Workflow

**File**: `.github/workflows/reusable-typescript-build.yml`

```yaml
name: Reusable TypeScript Build

on:
  workflow_call:
    inputs:
      workspace_filter:
        description: 'pnpm filter for packages to build (e.g., @crewchief/cli...)'
        required: false
        type: string
        default: './packages/*'

      artifact_name:
        description: 'Name for dist artifacts'
        required: false
        type: string
        default: 'typescript-dist'

    outputs:
      artifact_name:
        description: 'Name of uploaded artifact'
        value: ${{ jobs.build.outputs.artifact_name }}

jobs:
  build:
    name: Build TypeScript
    runs-on: ubuntu-latest

    outputs:
      artifact_name: ${{ inputs.artifact_name }}

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Setup pnpm
        uses: pnpm/action-setup@v4

      - name: Get pnpm store directory
        shell: bash
        run: echo "STORE_PATH=$(pnpm store path --silent)" >> $GITHUB_ENV

      - name: Setup pnpm cache
        uses: actions/cache@v4
        with:
          path: ${{ env.STORE_PATH }}
          key: ${{ runner.os }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
          restore-keys: |
            ${{ runner.os }}-pnpm-store-

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Build packages
        run: pnpm -r --filter='${{ inputs.workspace_filter }}' build

      - name: Upload dist artifacts
        uses: actions/upload-artifact@v4
        with:
          name: ${{ inputs.artifact_name }}
          path: |
            packages/*/dist/
            !packages/*/node_modules/
          if-no-files-found: error
          retention-days: 7
```

**Key Features**:
- Configurable workspace filter
- pnpm store caching
- Uploads all dist/ directories
- Excludes node_modules from artifacts

**Usage Example**:
```yaml
jobs:
  build-typescript:
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/cli...'
      artifact_name: 'cli-dist'
```

## Phase 3: Workflow Consolidation

### 3.1 Unified CLI Release Workflow

**File**: `release-cli.yml` (refactored from build-and-publish-cli.yml)

```yaml
name: Release CLI

on:
  push:
    tags:
      - '@crewchief/cli@v*.*.*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to publish (e.g., 1.2.3)'
        required: true
      dry_run:
        description: 'Dry run (skip actual publish)'
        type: boolean
        default: false

permissions:
  contents: read
  id-token: write  # For npm provenance

jobs:
  # Step 1: Build Rust binaries (reusable)
  build-rust:
    name: Build Rust Binaries
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: cli
      binary_name: crewchief-maproom
      crate_path: crates/maproom

  # Step 2: Build TypeScript (reusable)
  build-typescript:
    name: Build TypeScript
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/cli...'
      artifact_name: 'cli-typescript'

  # Step 3: Validate and publish
  publish:
    name: Publish to npm
    runs-on: ubuntu-latest
    needs: [build-rust, build-typescript]

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'

      - name: Setup pnpm
        uses: pnpm/action-setup@v4

      - name: Download Rust binaries
        uses: actions/download-artifact@v4
        with:
          pattern: cli-*
          path: packages/cli/bin/
          merge-multiple: true

      - name: Download TypeScript dist
        uses: actions/download-artifact@v4
        with:
          name: cli-typescript
          path: .

      - name: Verify binaries
        run: |
          for platform in linux-x64 linux-arm64 darwin-x64 darwin-arm64; do
            if [ ! -f "packages/cli/bin/$platform/crewchief-maproom" ]; then
              echo "Missing binary for $platform"
              exit 1
            fi
          done

      - name: Set executable permissions
        run: chmod +x packages/cli/bin/*/crewchief-maproom

      - name: Pack package
        run: |
          cd packages/cli
          pnpm pack

      - name: Publish to npm
        if: ${{ !inputs.dry_run && github.event_name != 'workflow_dispatch' }}
        run: |
          cd packages/cli
          pnpm publish --no-git-checks --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

      - name: Upload package artifact
        uses: actions/upload-artifact@v4
        with:
          name: npm-package
          path: packages/cli/*.tgz
          retention-days: 90
```

**Key Improvements**:
- Calls reusable workflows (no duplication)
- Downloads artifacts from previous jobs
- Validates all binaries present
- Dry-run support for testing
- npm provenance enabled

### 3.2 Unified Maproom-MCP Release Workflow

**File**: `release-maproom-mcp.yml` (consolidates npm + Docker)

```yaml
name: Release Maproom MCP

on:
  push:
    tags:
      - '@crewchief/maproom-mcp@v*.*.*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to publish'
        required: true
      push_docker:
        description: 'Push Docker image'
        type: boolean
        default: false

permissions:
  contents: read
  id-token: write
  packages: write

env:
  DOCKER_HUB_REPO: manifoldlogic/crewchief_maproom-mcp

jobs:
  # Step 1: Build Rust binaries (reusable)
  build-rust:
    name: Build Rust Binaries
    uses: ./.github/workflows/reusable-rust-build.yml
    with:
      package_name: maproom-mcp
      binary_name: crewchief-maproom
      crate_path: crates/maproom

  # Step 2: Build TypeScript (reusable)
  build-typescript:
    name: Build TypeScript
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/maproom-mcp... @crewchief/daemon-client...'
      artifact_name: 'maproom-mcp-typescript'

  # Step 3: Publish npm package
  publish-npm:
    name: Publish to npm
    runs-on: ubuntu-latest
    needs: [build-rust, build-typescript]

    steps:
      # Similar to CLI publish job
      # Download artifacts, validate, publish
      # [Full implementation similar to CLI]

  # Step 4: Build and publish Docker image (parallel with npm)
  publish-docker:
    name: Publish Docker Image
    runs-on: ubuntu-latest
    needs: [build-rust, build-typescript]

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Download Rust binaries
        uses: actions/download-artifact@v4
        with:
          pattern: maproom-mcp-*
          path: /tmp/binaries/
          merge-multiple: true

      - name: Download TypeScript dist
        uses: actions/download-artifact@v4
        with:
          name: maproom-mcp-typescript
          path: .

      - name: Set up QEMU
        uses: docker/setup-qemu-action@v3
        with:
          platforms: linux/amd64,linux/arm64

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to Docker Hub
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKERHUB_USERNAME }}
          password: ${{ secrets.DOCKERHUB_TOKEN }}

      - name: Extract version
        id: version
        run: |
          if [ "${{ github.event_name }}" = "workflow_dispatch" ]; then
            VERSION="${{ github.event.inputs.version }}"
          else
            TAG_NAME="${GITHUB_REF#refs/tags/}"
            VERSION="${TAG_NAME##*@v}"
          fi
          echo "full=${VERSION}" >> $GITHUB_OUTPUT
          echo "minor=$(echo ${VERSION} | cut -d. -f1-2)" >> $GITHUB_OUTPUT
          echo "major=$(echo ${VERSION} | cut -d. -f1)" >> $GITHUB_OUTPUT

      - name: Generate Docker metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.DOCKER_HUB_REPO }}
          tags: |
            type=raw,value=${{ steps.version.outputs.full }}
            type=raw,value=${{ steps.version.outputs.minor }}
            type=raw,value=${{ steps.version.outputs.major }}
            type=raw,value=latest

      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: packages/maproom-mcp/config/Dockerfile.combined
          platforms: linux/amd64,linux/arm64
          push: ${{ github.event_name != 'workflow_dispatch' || github.event.inputs.push_docker == 'true' }}
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
          build-args: |
            VERSION=${{ steps.version.outputs.full }}
            COMMIT_SHA=${{ github.sha }}

      - name: Run Trivy security scan
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: ${{ env.DOCKER_HUB_REPO }}:${{ steps.version.outputs.full }}
          format: table
          severity: CRITICAL,HIGH
          exit-code: 0
```

**Key Improvements**:
- **Single workflow** for npm + Docker (no more duplicate triggers)
- npm and Docker jobs run **in parallel** after builds complete
- Both use same Rust binaries (build once, use twice)
- Both use same TypeScript dist (build once, use twice)
- Docker build uses pre-built artifacts (no rebuild)

### 3.3 Optimized Test Workflow

**File**: `test.yml` (updated)

```yaml
name: Test

on:
  push:
    branches: [main]
    paths:
      # [Path filter from Phase 1]

  pull_request:
    paths:
      # [Path filter from Phase 1]

# Concurrency control
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

env:
  TEST_MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5434/maproom_test

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_USER: maproom
          POSTGRES_PASSWORD: maproom
          POSTGRES_DB: maproom_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5434:5432

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Setup pnpm
        uses: pnpm/action-setup@v4

      # pnpm caching (from Phase 1)
      - name: Get pnpm store directory
        run: echo "STORE_PATH=$(pnpm store path --silent)" >> $GITHUB_ENV

      - name: Setup pnpm cache
        uses: actions/cache@v4
        with:
          path: ${{ env.STORE_PATH }}
          key: ${{ runner.os }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
          restore-keys: |
            ${{ runner.os }}-pnpm-store-

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      # Rust caching (from Phase 1)
      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: "crates/maproom -> target"

      - name: Build Rust binary
        run: cargo build --release --bin crewchief-maproom

      - name: Run database migrations
        run: |
          ./target/release/crewchief-maproom db migrate

      - name: Run TypeScript tests
        run: pnpm test
```

**Key Improvements**:
- Path filtering (80% fewer runs)
- Concurrency control (cancel outdated PRs)
- Both Rust and pnpm caching
- Cleaner, more maintainable

## Phase 4: VSCode Extension Publishing (Future)

### 4.1 VSCode Extension Release Workflow

**File**: `.github/workflows/release-vscode-maproom.yml` (future)

```yaml
name: Release VSCode Extension

on:
  push:
    tags:
      - '@crewchief/vscode-maproom@v*.*.*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to publish'
        required: true
      pre_release:
        description: 'Publish as pre-release'
        type: boolean
        default: false
      publish_vscode:
        description: 'Publish to VS Code Marketplace'
        type: boolean
        default: true
      publish_ovsx:
        description: 'Publish to Open VSX'
        type: boolean
        default: true

permissions:
  contents: write  # For creating releases

jobs:
  # Step 1: Build extension
  build:
    name: Build Extension
    uses: ./.github/workflows/reusable-typescript-build.yml
    with:
      workspace_filter: '@crewchief/vscode-maproom...'
      artifact_name: 'vscode-extension-dist'

  # Step 2: Package extension
  package:
    name: Package Extension
    runs-on: ubuntu-latest
    needs: build

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
        run: |
          cd packages/vscode-maproom
          pnpm install --frozen-lockfile

      - name: Download dist artifacts
        uses: actions/download-artifact@v4
        with:
          name: vscode-extension-dist
          path: .

      - name: Install vsce
        run: pnpm add -g @vscode/vsce

      - name: Package extension
        run: |
          cd packages/vscode-maproom
          vsce package --out ../../vscode-maproom.vsix

      - name: Upload VSIX
        uses: actions/upload-artifact@v4
        with:
          name: extension-vsix
          path: vscode-maproom.vsix
          retention-days: 90

  # Step 3: Publish to VS Code Marketplace (parallel)
  publish-vscode:
    name: Publish to VS Code Marketplace
    runs-on: ubuntu-latest
    needs: package
    if: |
      (github.event_name != 'workflow_dispatch' || github.event.inputs.publish_vscode == 'true') &&
      secrets.VSCE_PAT != ''

    steps:
      - name: Download VSIX
        uses: actions/download-artifact@v4
        with:
          name: extension-vsix

      - name: Install vsce
        run: npm install -g @vscode/vsce

      - name: Publish extension
        run: |
          PRE_RELEASE_FLAG=""
          if [ "${{ github.event.inputs.pre_release }}" = "true" ]; then
            PRE_RELEASE_FLAG="--pre-release"
          fi
          vsce publish $PRE_RELEASE_FLAG --packagePath vscode-maproom.vsix -p ${{ secrets.VSCE_PAT }}

  # Step 4: Publish to Open VSX (parallel)
  publish-ovsx:
    name: Publish to Open VSX
    runs-on: ubuntu-latest
    needs: package
    if: |
      (github.event_name != 'workflow_dispatch' || github.event.inputs.publish_ovsx == 'true') &&
      secrets.OVSX_PAT != ''

    steps:
      - name: Download VSIX
        uses: actions/download-artifact@v4
        with:
          name: extension-vsix

      - name: Install ovsx
        run: npm install -g ovsx

      - name: Publish extension
        run: |
          ovsx publish vscode-maproom.vsix -p ${{ secrets.OVSX_PAT }}

  # Step 5: Create GitHub Release
  create-release:
    name: Create GitHub Release
    runs-on: ubuntu-latest
    needs: [publish-vscode, publish-ovsx]
    if: always() && (needs.publish-vscode.result == 'success' || needs.publish-ovsx.result == 'success')

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Download VSIX
        uses: actions/download-artifact@v4
        with:
          name: extension-vsix

      - name: Extract version
        id: version
        run: |
          if [ "${{ github.event_name }}" = "workflow_dispatch" ]; then
            VERSION="${{ github.event.inputs.version }}"
          else
            TAG_NAME="${GITHUB_REF#refs/tags/}"
            VERSION="${TAG_NAME##*@v}"
          fi
          echo "version=${VERSION}" >> $GITHUB_OUTPUT

      - name: Generate changelog
        id: changelog
        run: |
          # Extract changelog section for this version
          # Simplified for now - can use conventional-changelog later
          echo "changelog=See commit history for changes" >> $GITHUB_OUTPUT

      - name: Create Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: VSCode Maproom v${{ steps.version.outputs.version }}
          body: ${{ steps.changelog.outputs.changelog }}
          draft: false
          prerelease: ${{ github.event.inputs.pre_release == 'true' }}

      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ steps.create_release.outputs.upload_url }}
          asset_path: ./vscode-maproom.vsix
          asset_name: vscode-maproom-${{ steps.version.outputs.version }}.vsix
          asset_content_type: application/zip
```

**Key Features**:
- Build → Package → Publish (sequential with parallel publishing)
- Conditional publishing based on secrets
- Pre-release support
- Both marketplaces in parallel
- GitHub release with VSIX attachment
- Dry-run support via workflow_dispatch

## Technology Choices

### GitHub Actions Ecosystem

**Reusable Workflows** (chosen over composite actions):
- **Why**: Better for complete job definitions
- **Benefit**: Can use different runners per platform
- **Tradeoff**: Slightly more verbose to call

**actions/cache@v4** (pnpm):
- **Why**: Official GitHub action, well-maintained
- **Benefit**: GHA cache backend is fast
- **Tradeoff**: Limited to 10GB per repo (fine for our use)

**Swatinem/rust-cache@v2** (Rust):
- **Why**: Rust-specific caching, handles cross-compilation
- **Benefit**: Better cache invalidation than generic cache
- **Tradeoff**: Third-party action (but widely trusted)

**actions/upload-artifact@v4** / **download-artifact@v4**:
- **Why**: Official, reliable artifact sharing
- **Benefit**: Cross-job artifact sharing built-in
- **Tradeoff**: Artifacts expire (7-90 days), not permanent storage

### Build Tools

**cross** for Rust cross-compilation:
- **Why**: Industry standard for cross-compiling Rust
- **Benefit**: Docker-based, consistent environments
- **Tradeoff**: Slower than native (but cached)

**vsce** for VSCode packaging:
- **Why**: Official Microsoft tooling
- **Benefit**: Required for marketplace publishing
- **Alternative**: None (required)

**ovsx** for Open VSX:
- **Why**: Official Eclipse Foundation tooling
- **Benefit**: Compatible with vsce-packaged extensions
- **Alternative**: None (required)

## Architecture Decisions and Rationale

### Decision 1: Reusable Workflows vs Composite Actions

**Chosen**: Reusable workflows

**Rationale**:
- Need different runners per platform (ubuntu-latest vs macos-latest)
- Want isolated job contexts (security)
- Composite actions can't use different runners

**Tradeoff**: Slightly more YAML to call workflow, but clearer separation.

### Decision 2: Build Artifacts vs Rebuild in Docker

**Chosen**: Pre-build Rust binaries, use in Docker

**Rationale**:
- Rust builds are expensive (8-12 min without cache)
- Docker can COPY pre-built binaries
- Ensures exact same binary in npm package and Docker image
- Enables parallel npm/Docker publishing

**Tradeoff**: More complex artifact management, but 50% time savings.

### Decision 3: Consolidate Maproom-MCP Workflows vs Keep Separate

**Chosen**: Consolidate into single workflow

**Rationale**:
- Same tag triggers both workflows (redundant)
- Same Rust binaries needed (avoid rebuilding)
- Same TypeScript dist needed (avoid rebuilding)
- Jobs can run in parallel after build phase

**Tradeoff**: Slightly more complex workflow, but eliminates all duplication.

### Decision 4: Path Filters on Test Workflow

**Chosen**: Add comprehensive path filters

**Rationale**:
- 80% of commits don't change code (docs, planning, config)
- Tests take 5-8 minutes (waste on non-code changes)
- Simple to implement, high ROI

**Tradeoff**: Must remember to update paths when adding new code directories.

### Decision 5: pnpm Store Caching vs No Caching

**Chosen**: Cache pnpm store

**Rationale**:
- 714 packages to install (30-60 seconds uncached)
- Lock file rarely changes (high cache hit rate)
- Cross-workflow benefit (one cache for all workflows)

**Tradeoff**: 10GB cache limit (but we're well under), small maintenance overhead.

## Performance Considerations

### Expected Improvements

**Before Optimization**:
- Test workflow: 5-8 min (runs on every push/PR)
- CLI release: 12-15 min
- Maproom-MCP npm release: 12-15 min
- Maproom-MCP Docker release: 8-10 min (fails currently)
- **Total maproom-mcp release: 25-30 min**

**After Phase 1** (caching + path filters):
- Test workflow: 3-5 min (runs 80% less often)
- CLI release: 6-8 min (50% faster via caching)
- Maproom-MCP npm: 6-8 min
- Maproom-MCP Docker: 5-6 min (fixed + cached)
- **Total maproom-mcp release: 15-18 min** (still parallel)

**After Phase 2-3** (reusables + consolidation):
- Test workflow: 3-5 min
- CLI release: 6-8 min
- **Maproom-MCP unified: 8-10 min** (build once, publish both)
- **Total improvement: 60-70% faster**

### Cache Hit Rates (Projected)

**Rust** (Swatinem/rust-cache):
- First run: 0% hit (8-12 min build)
- Second run: 80-90% hit (2-4 min build)
- After dependency update: 20-30% hit (4-6 min build)

**pnpm** (actions/cache):
- First run: 0% hit (60 sec install)
- Second run: 95%+ hit (10-15 sec install)
- After lock file change: 0% hit (60 sec install)

**Docker layers** (buildx cache):
- First run: 0% hit
- Second run: 60-70% hit (base images + deps cached)
- Code-only change: 90%+ hit (only app layer rebuilt)

### Concurrency and Parallelization

**Current** (sequential bottlenecks):
```
maproom-mcp tag pushed
├─ npm workflow starts → build Rust (12 min) → publish (2 min)
└─ Docker workflow starts → build Rust (10 min) → publish (3 min)
Total: 25 min (parallel but redundant work)
```

**Optimized** (parallel after shared build):
```
maproom-mcp tag pushed
└─ Unified workflow
   ├─ build-rust (4 min with cache)
   ├─ build-typescript (2 min with cache)
   └─ parallel:
      ├─ publish-npm (2 min)
      └─ publish-docker (3 min)
Total: 9 min (4 + 2 + max(2,3))
```

## Long-Term Maintainability

### Code Organization

**Before**:
- 4 workflow files
- ~600 lines YAML
- 450 lines duplicated (75%)

**After**:
- 6 workflow files (4 package + 2 reusable)
- ~300 lines package-specific YAML
- ~200 lines reusable YAML
- 0 lines duplicated

**Maintenance Burden**:
- Changes to Rust build: 1 file (vs 2 before)
- Changes to TypeScript build: 1 file (vs 4 before)
- New package release: Copy & modify template (vs copy & modify + Docker)

### Documentation Requirements

**Must document**:
- How reusable workflows work
- When to use workflow_dispatch for testing
- How to add new platform to matrix
- Cache invalidation procedures
- VSCode extension publishing setup (future)

**Location**: `.github/WORKFLOWS.md` (new file, Phase 3)

### Testing Strategy

**Workflow Testing**:
1. yamllint validation (syntax)
2. workflow_dispatch with dry-run (logic)
3. Test on feature branch (safe testing)
4. Monitor first real release (validation)

**Rollback Plan**:
- Keep old workflows as `.old` backups
- Git history allows full revert
- Can switch back by renaming files

## Conclusion

This architecture achieves:
- ✅ 60-70% faster releases
- ✅ Zero code duplication
- ✅ Comprehensive caching
- ✅ 80% fewer unnecessary test runs
- ✅ Future-ready for VSCode extension publishing
- ✅ Industry-standard patterns
- ✅ Easy to maintain and extend

**Risk Level**: Low - all patterns proven by industry leaders (Rust, TypeScript, VSCode teams).

**Next**: Define quality strategy and security review.

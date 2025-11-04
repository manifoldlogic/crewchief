# Analysis: Integrated Rust Binary Packaging for npm

## Problem Statement

The current release process for `@crewchief/maproom-mcp` is broken and unreliable:

1. **Manual Binary Build**: Developers must manually run `scripts/build-and-package.sh` before releasing
2. **Platform-Specific Builds**: The build script only builds for the current platform (e.g., arm64), missing other architectures
3. **Easy to Forget**: Running `pnpm release:patch` doesn't build binaries, resulting in incomplete npm packages
4. **Recent Failure**: Version 1.3.0 was published with only `linux-arm64` and `darwin-arm64` binaries, missing `linux-x64` (amd64) - the most common Docker/devcontainer architecture
5. **Silent Failure**: No validation prevents publishing packages without required binaries

This resulted in a production incident where `npx @crewchief/maproom-mcp setup` failed because the Node.js CLI couldn't find the Rust binary on linux-x64 systems.

## Current State

### Existing Infrastructure

**Build Script** (`scripts/build-and-package.sh`):
- Detects current OS and architecture
- Builds Rust binary for current platform only
- Copies to `packages/maproom-mcp/bin/<platform>/`
- Platform format: `{os}-{arch}` (e.g., `linux-x64`, `darwin-arm64`)

**Release Scripts** (`packages/maproom-mcp/package.json`):
```json
{
  "scripts": {
    "release:patch": "node scripts/bump-version.js patch && pnpm publish --access public --no-git-checks",
    "release:minor": "node scripts/bump-version.js minor && pnpm publish --access public --no-git-checks",
    "release:major": "node scripts/bump-version.js major && pnpm publish --access public --no-git-checks"
  }
}
```

**Package Files** (`package.json` files array):
```json
{
  "files": [
    "bin/**/*",
    "config/docker-compose.yml",
    "dist/",
    ...
  ]
}
```

**GitHub Actions**:
- `publish-maproom-mcp-image.yml`: Publishes Docker image (works, but separate from npm)
- `cli-release.yml`: Releases CLI package (different package)
- No workflow for building multi-platform Rust binaries

### Required Platforms

Based on common deployment targets:
1. **linux-x64** (x86_64-unknown-linux-gnu): Most Docker containers, devcontainers, CI/CD
2. **linux-arm64** (aarch64-unknown-linux-gnu): ARM-based servers, Apple Silicon Docker
3. **darwin-x64** (x86_64-apple-darwin): Intel Macs
4. **darwin-arm64** (aarch64-apple-darwin): Apple Silicon Macs

## Industry Solutions

### Approach 1: Optional Dependencies (e.g., esbuild, swc)
**Pattern**: Separate npm packages per platform, installed via `optionalDependencies`

```json
{
  "optionalDependencies": {
    "@crewchief/maproom-linux-x64": "^1.0.0",
    "@crewchief/maproom-linux-arm64": "^1.0.0",
    "@crewchief/maproom-darwin-x64": "^1.0.0",
    "@crewchief/maproom-darwin-arm64": "^1.0.0"
  }
}
```

**Pros**:
- Smaller package size (only downloads platform needed)
- Standard npm pattern

**Cons**:
- Requires publishing 5+ packages per release
- Complex versioning and coordination
- More infrastructure to maintain

### Approach 2: Postinstall Script (e.g., node-pre-gyp, prebuild)
**Pattern**: Download platform-specific binary after npm install

**Pros**:
- Single package to publish
- Smaller initial download

**Cons**:
- Requires hosting binaries externally (S3, GitHub releases)
- Corporate proxies/firewalls often block downloads
- Offline installs fail
- More complex infrastructure

### Approach 3: Fat Package (e.g., @swc/core, turbo)
**Pattern**: Include all platform binaries in single npm package

**Pros**:
- Simple: one package to publish
- Works offline
- No external dependencies
- Reliable: either it works or it doesn't

**Cons**:
- Larger package size (but manageable - binaries are ~10MB each)

**Industry Examples**:
- `turbo`: Ships with all platform binaries (~40MB package)
- `@swc/core`: Ships with all platform binaries
- `@esbuild/linux-x64`, etc.: Uses optional dependencies (more complex)

### Approach 4: GitHub Actions Matrix Build
**Pattern**: Build all platforms in CI, aggregate before publishing

**Pros**:
- Automated and reliable
- Builds all platforms consistently
- Can use native compilers

**Cons**:
- Requires GitHub Actions or similar CI
- Longer build times (parallel though)

## Recommended Approach

**Hybrid: Fat Package + GitHub Actions Matrix Build**

### Why This Combination?

1. **Reliability**: Fat package ensures npm install always works
2. **Simplicity**: One package to publish, no coordination needed
3. **Automation**: GitHub Actions ensures all binaries are built
4. **Developer Experience**: `pnpm release:x` triggers full pipeline
5. **Safety**: Pre-publish validation prevents incomplete releases

### Architecture

```
Developer runs: pnpm release:patch
    ↓
1. Bump version in package.json
2. Commit version bump
3. Create git tag (v1.x.x)
4. Push tag to GitHub
    ↓
GitHub Actions: build-and-publish-maproom-mcp
    ↓
1. Matrix build: 4 platforms in parallel
   - linux-x64 (ubuntu-latest + cross)
   - linux-arm64 (ubuntu-latest + cross)
   - darwin-x64 (macos-13)
   - darwin-arm64 (macos-latest)
    ↓
2. Download all artifacts
    ↓
3. Validate: Check all 4 binaries exist
    ↓
4. npm publish
    ↓
5. Verify: Test install on multiple platforms
```

## Gap Analysis

### Missing Components

1. **GitHub Actions Workflow**: No workflow to build all platforms
2. **Pre-publish Validation**: No check that binaries exist before publishing
3. **Release Script Integration**: `pnpm release:x` doesn't trigger CI workflow
4. **Binary Detection Logic**: Node.js CLI needs robust platform detection
5. **Testing Infrastructure**: No automated test that binaries work

### Existing Components to Leverage

1. **Build Script**: Can be adapted for cross-compilation
2. **Platform Detection**: Already exists in `getMaproomBinaryPath()`
3. **Package Structure**: `bin/<platform>/` layout is good
4. **Docker Infrastructure**: Can test linux-x64 builds

## Technical Considerations

### Cross-Compilation

**Option 1: Native Builds** (Slower but simpler)
- Use GitHub Actions runners for each platform
- macOS runners: macos-13 (x64), macos-latest (arm64)
- Linux runners: ubuntu-latest (x64), ubuntu-latest (arm64)

**Option 2: Cross-Compilation** (Faster)
- Use `cross` tool for Linux targets
- Use native builds for macOS (cross-compilation difficult)

**Recommendation**: Hybrid approach
- Linux: Use `cross` from ubuntu-latest
- macOS: Use native runners

### Rust Toolchain

**Required Targets**:
```bash
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

**Build Commands**:
```bash
# Linux (using cross)
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu

# macOS (native)
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### Package Size Impact

**Current Package**: ~2MB (TypeScript dist + config)
**With All Binaries**: ~50MB (4 binaries × ~12MB each)

**Mitigation**:
- Acceptable for developer tool
- Most developers only need one platform
- turbo package is 40MB and widely adopted

### npm Publish Flow

**Current Flow**:
```
Developer → pnpm release:x → npm publish → Done
```

**Proposed Flow**:
```
Developer → pnpm release:x
    ↓
1. Bump version
2. Git commit + tag
3. Git push (tag triggers CI)
    ↓
GitHub Actions (automatic)
    ↓
1. Build all platforms
2. Validate binaries
3. npm publish
    ↓
Developer notified via GitHub
```

## Risk Assessment

### High Risks

1. **Breaking Change Risk**: Changing release process could break existing workflow
   - **Mitigation**: Keep `pnpm release:x` as entry point, add automation behind it

2. **CI Failure Risk**: GitHub Actions might fail, blocking releases
   - **Mitigation**: Allow manual override with `pnpm publish` (with validation)

3. **Binary Compatibility**: Binaries might not work on all platforms
   - **Mitigation**: Automated testing on each platform

### Medium Risks

1. **Build Time**: Matrix builds take longer than single platform
   - **Mitigation**: Parallel builds, typically ~10min total

2. **Secrets Management**: Need NPM_TOKEN in GitHub Actions
   - **Mitigation**: Use GitHub secrets, already standard practice

3. **Version Conflicts**: Developer might manually publish during CI
   - **Mitigation**: CI checks if version already published

### Low Risks

1. **Package Size**: 50MB package might be concerning
   - **Mitigation**: Standard for Rust tooling, acceptable

2. **Platform Support**: New platforms might be needed
   - **Mitigation**: Easy to add to matrix

## Success Metrics

1. **Reliability**: 100% of releases include all 4 platform binaries
2. **Developer Experience**: `pnpm release:x` is single command needed
3. **Failure Detection**: Pre-publish validation catches missing binaries
4. **Backwards Compatibility**: Existing installations continue working
5. **Build Time**: Complete release process takes <15 minutes

## Dependencies

### External Dependencies (Stable)
- GitHub Actions (stable API)
- npm registry (stable API)
- Rust toolchain (stable)
- `cross` tool (stable)

### Internal Dependencies (Owned)
- `packages/maproom-mcp/package.json`
- `scripts/bump-version.js`
- Rust crate `crates/maproom`
- `.github/workflows/` directory

All interfaces are stable and under our control.

## Conclusion

The current manual, platform-specific build process is unreliable and has caused production failures. The recommended solution is a GitHub Actions matrix build that automates building all platform binaries and integrates seamlessly with the existing `pnpm release:x` workflow.

This approach:
- **Fixes the immediate problem**: Never publish without all binaries
- **Improves developer experience**: Single command releases
- **Leverages industry patterns**: Fat package + CI builds (proven by turbo, swc)
- **Maintains simplicity**: No complex optional dependencies or postinstall scripts
- **Provides safety**: Pre-publish validation prevents bad releases

Next steps: Design the architecture and implementation plan.

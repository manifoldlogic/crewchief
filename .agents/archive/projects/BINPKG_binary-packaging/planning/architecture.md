# Architecture: Integrated Rust Binary Packaging

## Design Philosophy

**Core Principle**: Make the right thing easy and the wrong thing hard.

**MVP Goal**: Ensure `pnpm release:x` always produces complete, working packages with all platform binaries included.

**Non-Goal**: Perfect optimization - we optimize for reliability and developer experience over package size or build time.

## System Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Developer Workstation                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  $ pnpm release:patch                                           │
│      ↓                                                           │
│  1. scripts/release.js                                          │
│     - Validates working directory clean                         │
│     - Bumps version in package.json                             │
│     - Commits: "chore(release): bump version to X.Y.Z"          │
│     - Creates tag: vX.Y.Z                                       │
│     - Pushes commit + tag to origin                             │
│      ↓                                                           │
│  2. Waits for GitHub Actions (optional)                         │
│     - Monitors workflow status                                  │
│     - Reports success/failure                                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                          │
                          │ git push --follow-tags
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│                       GitHub Actions                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Workflow: build-and-publish-maproom-mcp.yml                    │
│  Trigger: tags matching "v*.*.*"                                │
│                                                                  │
│  ┌─────────────────────────────────────────────────────┐       │
│  │  Job: build-binaries (matrix, parallel)              │       │
│  │                                                       │       │
│  │  Platforms:                                           │       │
│  │  - linux-x64    (ubuntu-latest + cross)              │       │
│  │  - linux-arm64  (ubuntu-latest + cross)              │       │
│  │  - darwin-x64   (macos-13)                           │       │
│  │  - darwin-arm64 (macos-latest)                       │       │
│  │                                                       │       │
│  │  Steps per platform:                                 │       │
│  │  1. Checkout code                                    │       │
│  │  2. Setup Rust                                       │       │
│  │  3. Install cross (Linux only)                       │       │
│  │  4. Build binary for target                          │       │
│  │  5. Strip binary (reduce size)                       │       │
│  │  6. Create platform directory structure              │       │
│  │  7. Upload artifact                                  │       │
│  └─────────────────────────────────────────────────────┘       │
│                          │                                       │
│                          ↓                                       │
│  ┌─────────────────────────────────────────────────────┐       │
│  │  Job: validate-and-publish                           │       │
│  │  Depends: build-binaries                             │       │
│  │                                                       │       │
│  │  Steps:                                               │       │
│  │  1. Download all binary artifacts                    │       │
│  │  2. Validate: Check 4 binaries exist                │       │
│  │  3. Validate: Check each binary is executable        │       │
│  │  4. Copy binaries to packages/maproom-mcp/bin/       │       │
│  │  5. npm pack (test package creation)                 │       │
│  │  6. Extract and verify binaries in tarball           │       │
│  │  7. npm publish --access public                      │       │
│  │  8. Post-publish verification                        │       │
│  └─────────────────────────────────────────────────────┘       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                          │
                          │ npm publish
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│                        npm Registry                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  @crewchief/maproom-mcp@X.Y.Z                                   │
│    ├── bin/                                                     │
│    │   ├── cli.cjs                                              │
│    │   ├── linux-x64/crewchief-maproom                         │
│    │   ├── linux-arm64/crewchief-maproom                       │
│    │   ├── darwin-x64/crewchief-maproom                        │
│    │   └── darwin-arm64/crewchief-maproom                      │
│    ├── config/                                                  │
│    ├── dist/                                                    │
│    └── package.json                                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Release Script (`scripts/release.js`)

**Purpose**: Entry point for releases, replaces bump-version.js

**Responsibilities**:
- Validate preconditions (clean git, tests pass)
- Bump version in package.json
- Create git commit and tag
- Push to GitHub
- Optionally monitor CI workflow

**Interface**:
```bash
# Usage
pnpm release:patch   # calls: node scripts/release.js patch
pnpm release:minor   # calls: node scripts/release.js minor
pnpm release:major   # calls: node scripts/release.js major

# Options
--skip-ci-wait      # Don't wait for GitHub Actions
--dry-run          # Show what would happen
```

**Implementation Notes**:
- Uses `semver` package for version bumping
- Uses `execa` for git commands
- Uses GitHub API to monitor workflow (optional)
- Provides clear progress indicators

**Error Handling**:
- Validates git working directory is clean
- Validates current branch is main/master
- Validates npm credentials exist (npm whoami)
- Rolls back on failure (git reset)

### 2. GitHub Actions Workflow

**File**: `.github/workflows/build-and-publish-maproom-mcp.yml`

**Trigger**:
```yaml
on:
  push:
    tags:
      - 'v*.*.*'
  workflow_dispatch:
    inputs:
      dry_run:
        description: 'Dry run (skip publish)'
        default: false
```

**Jobs Structure**:

#### Job 1: build-binaries (Matrix)
```yaml
strategy:
  matrix:
    include:
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
        platform: linux-x64
        use_cross: true

      - os: ubuntu-latest
        target: aarch64-unknown-linux-gnu
        platform: linux-arm64
        use_cross: true

      - os: macos-13
        target: x86_64-apple-darwin
        platform: darwin-x64
        use_cross: false

      - os: macos-latest
        target: aarch64-apple-darwin
        platform: darwin-arm64
        use_cross: false
```

**Build Steps**:
1. Checkout code
2. Setup Rust toolchain
3. Install cross (if use_cross)
4. Build: `cargo build --release --target $TARGET`
5. Strip binary: `strip target/$TARGET/release/crewchief-maproom`
6. Create directory: `bin/$PLATFORM/`
7. Copy binary
8. Upload artifact

#### Job 2: validate-and-publish
```yaml
needs: build-binaries
runs-on: ubuntu-latest
```

**Validation Steps**:
1. Download all artifacts
2. Check 4 platforms exist
3. Check each binary is executable
4. Check binary sizes are reasonable (>1MB, <100MB)

**Publish Steps**:
1. Setup Node.js with npm registry auth
2. Copy binaries to packages/maproom-mcp/bin/
3. Run `npm pack` (dry run)
4. Extract and verify tarball contents
5. Run `npm publish --access public`
6. Verify package on registry

### 3. Package.json Updates

**Changes to `packages/maproom-mcp/package.json`**:

```json
{
  "scripts": {
    "prepublishOnly": "node ../../scripts/validate-binaries.js",
    "release:patch": "node ../../scripts/release.js patch",
    "release:minor": "node ../../scripts/release.js minor",
    "release:major": "node ../../scripts/release.js major"
  },
  "files": [
    "bin",  // Simplified from "bin/**/*"
    "config/docker-compose.yml",
    "config/Dockerfile.mcp-server",
    "config/init.sql",
    "dist/",
    "src/",
    "tsconfig.json",
    "README.md",
    "LICENSE"
  ]
}
```

**Rationale**:
- `prepublishOnly`: Safety net - validates binaries exist before any publish
- `release:*`: New scripts that trigger full pipeline
- `files`: Simplified to just "bin" (includes all subdirectories)

### 4. Binary Validation Script

**File**: `scripts/validate-binaries.js`

**Purpose**: Pre-publish safety check

**Validation Logic**:
```javascript
const fs = require('fs');
const path = require('path');

const REQUIRED_PLATFORMS = [
  'linux-x64',
  'linux-arm64',
  'darwin-x64',
  'darwin-arm64'
];

const binDir = path.join(__dirname, '../packages/maproom-mcp/bin');

for (const platform of REQUIRED_PLATFORMS) {
  const binaryPath = path.join(binDir, platform, 'crewchief-maproom');

  if (!fs.existsSync(binaryPath)) {
    console.error(`❌ Missing binary for ${platform}`);
    console.error(`   Expected: ${binaryPath}`);
    console.error(`\n💡 Run: pnpm build:binaries`);
    process.exit(1);
  }

  const stats = fs.statSync(binaryPath);

  if (stats.size < 1_000_000) {
    console.error(`❌ Binary too small for ${platform}: ${stats.size} bytes`);
    process.exit(1);
  }

  if (stats.size > 100_000_000) {
    console.error(`⚠️  Binary very large for ${platform}: ${stats.size} bytes`);
  }

  console.log(`✓ ${platform}: ${(stats.size / 1_000_000).toFixed(1)}MB`);
}

console.log('\n✅ All binaries present and valid\n');
```

**Integration**:
- Runs automatically before `npm publish` (via prepublishOnly hook)
- Can be run manually: `node scripts/validate-binaries.js`
- Prevents accidental incomplete publishes

### 5. Platform Detection Enhancement

**File**: `packages/maproom-mcp/bin/cli.cjs`

**Current Implementation** (getMaproomBinaryPath):
```javascript
function getMaproomBinaryPath() {
  const platform = `${process.platform}-${process.arch}`;
  const binaryName = 'crewchief-maproom';
  const binaryPath = path.join(__dirname, '..', 'bin', platform, binaryName);

  if (!fs.existsSync(binaryPath)) {
    throw new Error(`Binary not found for platform: ${platform}`);
  }

  return binaryPath;
}
```

**Enhanced Implementation**:
```javascript
function getMaproomBinaryPath() {
  // Map Node.js platform/arch to our naming convention
  const platformMap = {
    'linux-x64': 'linux-x64',
    'linux-arm64': 'linux-arm64',
    'darwin-x64': 'darwin-x64',
    'darwin-arm64': 'darwin-arm64'
  };

  const nodePlatform = `${process.platform}-${process.arch}`;
  const platform = platformMap[nodePlatform];

  if (!platform) {
    throw new Error(
      `Unsupported platform: ${nodePlatform}\n` +
      `Supported: ${Object.keys(platformMap).join(', ')}`
    );
  }

  const binaryName = 'crewchief-maproom';
  const binaryPath = path.join(__dirname, '..', 'bin', platform, binaryName);

  if (!fs.existsSync(binaryPath)) {
    throw new Error(
      `Binary not found: ${binaryPath}\n` +
      `Platform: ${platform}\n` +
      `This usually means the package was published incorrectly.\n` +
      `Please report this issue.`
    );
  }

  return binaryPath;
}
```

## Build Strategy

### Cross-Compilation vs Native Builds

**Linux Targets**: Use `cross`
- **Reason**: GitHub Actions ubuntu-latest runners are x64, need cross for arm64
- **Tooling**: `cross` handles all the complexity
- **Command**: `cross build --release --target aarch64-unknown-linux-gnu`

**macOS Targets**: Native builds
- **Reason**: Cross-compiling macOS is difficult (requires macOS SDK)
- **Tooling**: Use appropriate GitHub Actions runners
  - macos-13: Intel (x64)
  - macos-latest: Apple Silicon (arm64)
- **Command**: `cargo build --release --target <target>`

### Binary Optimization

**Stripping Symbols**:
```bash
strip target/release/crewchief-maproom
```
- Reduces binary size by 30-50%
- No functional impact (removes debug symbols)

**Cargo.toml Optimization** (already present):
```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

## Failure Modes and Recovery

### Failure Mode 1: Single Platform Build Fails

**Detection**: Matrix job failure in GitHub Actions

**Impact**: Publish blocked (validate-and-publish requires all builds)

**Recovery**:
1. Check workflow logs for specific platform failure
2. Fix issue (usually dependency or toolchain problem)
3. Re-run workflow from GitHub Actions UI
4. Or: Manually trigger with workflow_dispatch

### Failure Mode 2: All Builds Succeed but Validation Fails

**Detection**: validate-and-publish job fails

**Impact**: Binaries built but not published

**Recovery**:
1. Investigate validation error
2. If artifact issue: Re-run build-binaries
3. If validation bug: Fix script, re-run workflow

### Failure Mode 3: Publish Fails After Validation Passes

**Detection**: npm publish command fails

**Impact**: Binaries validated but not on npm

**Recovery**:
1. Check npm auth (NPM_TOKEN)
2. Check if version already published
3. Re-run validate-and-publish job
4. Or: Manually publish with `npm publish` after downloading artifacts

### Failure Mode 4: Developer Manually Publishes

**Detection**: prepublishOnly hook runs

**Impact**: Prevented by validate-binaries.js

**Recovery**: Developer must either:
1. Build all binaries locally (impractical)
2. Wait for GitHub Actions (correct approach)

## Security Considerations

### NPM_TOKEN Protection

**Storage**: GitHub repository secret

**Permissions**: Write access to npm registry

**Scope**: Limited to @crewchief organization

**Rotation**: Should be rotated periodically

### Binary Verification

**Problem**: How to trust binaries built in CI?

**Solution**: Reproducible builds (future enhancement)

**Current**: Trust GitHub Actions + code review

**Risk Level**: Low (open source project, code is public)

### Secrets in Workflow

**Required Secrets**:
- `NPM_TOKEN`: npm publish authentication
- (No other secrets needed)

**Best Practices**:
- Use environment-specific secrets
- Limit secret exposure scope
- Never log secret values

## Monitoring and Observability

### Workflow Status

**Monitoring**:
- GitHub Actions status page
- Email notifications on failure
- Optional: Release script monitors workflow

**Metrics**:
- Build time per platform
- Total publish time
- Success rate

### Package Health

**Post-Publish Checks**:
1. Verify package appears on npm
2. Test install on each platform (future enhancement)
3. Check package size

**Alerts**:
- Workflow failure: GitHub notification
- npm publish failure: GitHub notification

## Testing Strategy

### Build Testing (CI)

**Per-Platform Tests**:
1. Binary exists
2. Binary is executable
3. Binary size is reasonable
4. Binary runs: `./binary --version`

**Aggregate Tests**:
1. All 4 platforms built
2. Tarball contains all binaries
3. Package size acceptable

### Integration Testing (Post-Deploy)

**Test Matrix**:
```yaml
test-platforms:
  - linux-x64 (Docker ubuntu:latest)
  - linux-arm64 (Docker arm64v8/ubuntu)
  - darwin-x64 (macos-13)
  - darwin-arm64 (macos-latest)

test-steps:
  1. npm install @crewchief/maproom-mcp@latest
  2. Verify binary exists
  3. Run: npx @crewchief/maproom-mcp --version
  4. Run: npx @crewchief/maproom-mcp setup --provider=ollama
```

**Scope**: Optional, post-MVP enhancement

## Performance Considerations

### Build Time

**Estimated Times**:
- Linux builds (cross): ~5 minutes each
- macOS builds (native): ~10 minutes each
- Total parallel time: ~10 minutes
- Validation + publish: ~2 minutes
- **Total: ~12 minutes**

**Optimization Opportunities**:
- Cache Rust dependencies (saves ~2 minutes)
- Cache cross installation (saves ~1 minute)

### Package Size

**Component Sizes**:
- TypeScript dist: ~1MB
- Config files: ~0.1MB
- Each binary: ~12MB
- Total: ~50MB

**Comparison**:
- turbo: ~40MB (similar)
- @swc/core: ~30MB (optional deps)
- esbuild: ~8MB (optional deps)

**Conclusion**: Acceptable for developer tooling

## Migration Path

### Phase 1: Setup Infrastructure (MVP)
1. Create GitHub Actions workflow
2. Create release script
3. Create validation script
4. Update package.json
5. Test on canary release

### Phase 2: Cutover
1. Announce change to developers
2. Update documentation
3. First production release with new process

### Phase 3: Enhancements (Post-MVP)
1. Add post-publish integration tests
2. Add build caching
3. Add release monitoring dashboard
4. Add rollback automation

## Decision Log

### Decision 1: Fat Package vs Optional Dependencies

**Chosen**: Fat package (all binaries in one)

**Rationale**:
- Simpler to maintain (one package)
- More reliable (works offline)
- Industry precedent (turbo, swc)
- Package size acceptable (~50MB)

### Decision 2: GitHub Actions vs Other CI

**Chosen**: GitHub Actions

**Rationale**:
- Already used in project
- Native GitHub integration
- Free for open source
- Good runner availability

### Decision 3: Cross-Compilation vs Docker

**Chosen**: Hybrid (cross for Linux, native for macOS)

**Rationale**:
- Cross is reliable for Linux
- macOS cross-compilation difficult
- Native builds more reliable

### Decision 4: Manual vs Automatic Publish

**Chosen**: Automatic (CI publishes)

**Rationale**:
- Ensures all binaries present
- Consistent process
- Reduces human error
- Can still manual override

## Future Considerations

### Potential Enhancements

1. **Platform Support**: Add windows-x64
2. **Caching**: Speed up builds with dependency cache
3. **Testing**: Automated post-publish platform tests
4. **Monitoring**: Release dashboard
5. **Rollback**: Automated rollback on failed tests

### Scalability

**Current Design Scales To**:
- 10+ platforms (just add to matrix)
- Multiple packages (reuse workflow)
- High release frequency (CI handles it)

**Limitations**:
- GitHub Actions minutes (free tier: 2000/month)
- npm rate limits (generous)

## Summary

The architecture provides:
- ✅ **Automated**: `pnpm release:x` triggers full pipeline
- ✅ **Reliable**: Pre-publish validation prevents bad releases
- ✅ **Complete**: All 4 platforms built in CI
- ✅ **Safe**: Multiple validation points
- ✅ **Maintainable**: Standard GitHub Actions patterns
- ✅ **Extensible**: Easy to add platforms or packages

Next: Quality strategy and testing plan.

# Architecture: CLI GitHub Actions Release Automation

## Design Principles

1. **Copy proven patterns**: Replicate maproom-mcp workflow structure (already battle-tested)
2. **Independent operation**: CLI and MCP workflows must not interfere with each other
3. **Fail fast**: Validation errors block publish, prevent broken releases
4. **Platform parity**: All 4 platforms receive equal treatment (no second-class targets)
5. **Minimal disruption**: Keep local development workflows unchanged

## System Architecture

### High-Level Flow

```
Developer              Git Repository           GitHub Actions              npm Registry
   │                          │                         │                         │
   │  1. pnpm release:minor   │                         │                         │
   ├──────────────────────────>│                         │                         │
   │  (bumps version,          │                         │                         │
   │   commits, creates tag)   │                         │                         │
   │                           │                         │                         │
   │  2. git push (commits)    │                         │                         │
   ├──────────────────────────>│                         │                         │
   │                           │                         │                         │
   │  3. git push origin <tag> │                         │                         │
   ├──────────────────────────>│                         │                         │
   │  (separate to avoid race) │                         │                         │
   │                           │                         │                         │
   │                           │  4. Tag detected        │                         │
   │                           ├────────────────────────>│                         │
   │                           │  @crewchief/cli@v*.*.*  │                         │
   │                           │                         │                         │
   │                           │  5. Matrix builds       │                         │
   │                           │  (4 platforms parallel) │                         │
   │                           │      ┌──────────────────┤                         │
   │                           │      │ linux-x64        │                         │
   │                           │      │ linux-arm64      │                         │
   │                           │      │ darwin-x64       │                         │
   │                           │      │ darwin-arm64     │                         │
   │                           │      └──────────────────┤                         │
   │                           │                         │                         │
   │                           │  6. Collect artifacts   │                         │
   │                           │  7. Build TypeScript    │                         │
   │                           │  8. Validate package    │                         │
   │                           │                         │                         │
   │                           │                      9. Publish                   │
   │                           │                         ├────────────────────────>│
   │                           │                         │  @crewchief/cli@v1.0.0  │
   │                           │                         │                         │
   │                           │                     10. Verify                    │
   │                           │                         │<────────────────────────│
   │                           │                         │  (check registry)       │
   │                           │                         │                         │
```

**Note**: The two-step push (steps 2-3) is critical to avoid a race condition where the tag arrives at GitHub before the commit is fully registered, which would cause the workflow trigger to fail.

### Workflow Architecture

**File**: `.github/workflows/build-and-publish-cli.yml`

**Trigger configuration**:
```yaml
on:
  push:
    tags:
      - '@crewchief/cli@v*.*.*'  # Only CLI tags
  workflow_dispatch:
    inputs:
      dry_run:
        description: 'Dry run (skip publish)'
        type: boolean
        default: false
```

**Job structure**:
```
┌─────────────────────────────────────────────────────────────┐
│ Job 1: build-binaries                                       │
│ Strategy: matrix (4 platforms)                              │
│                                                              │
│ ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────┐ │
│ │ linux-x64  │  │linux-arm64 │  │darwin-x64  │  │darwin- │ │
│ │            │  │            │  │            │  │ arm64  │ │
│ │ - Setup    │  │ - Setup    │  │ - Setup    │  │ - Setup│ │
│ │ - Cross-   │  │ - Cross-   │  │ - Native   │  │ - Nativ│ │
│ │   compile  │  │   compile  │  │   build    │  │   build│ │
│ │ - Strip    │  │ - Strip    │  │ - Strip    │  │ - Strip│ │
│ │ - Upload   │  │ - Upload   │  │ - Upload   │  │ - Upload││
│ └────────────┘  └────────────┘  └────────────┘  └────────┘ │
│                                                              │
└──────────────────┬───────────────────────────────────────────┘
                   │ Artifacts
                   ▼
┌─────────────────────────────────────────────────────────────┐
│ Job 2: validate-and-publish                                 │
│ Depends on: build-binaries                                  │
│                                                              │
│ Steps:                                                       │
│ 1. Download all 4 binary artifacts                          │
│ 2. Organize binaries into bin/ structure                    │
│ 3. Build TypeScript (tsup)                                  │
│ 4. Create platform symlinks                                 │
│ 5. Run validation suite                                     │
│ 6. Create npm package tarball                               │
│ 7. Inspect tarball contents                                 │
│ 8. Publish to npm (if not dry-run)                          │
│ 9. Verify publication on registry                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Matrix Build Configuration

**Platform matrix**:
| Platform | Runner | Rust Target | Build Method |
|----------|--------|-------------|--------------|
| linux-x64 | ubuntu-latest | x86_64-unknown-linux-gnu | Cross-compile |
| linux-arm64 | ubuntu-latest | aarch64-unknown-linux-gnu | Cross-compile |
| darwin-x64 | macos-13 | x86_64-apple-darwin | Native |
| darwin-arm64 | macos-latest | aarch64-apple-darwin | Native |

**Cross-compilation setup** (Linux targets):
```yaml
- name: Install cross-compilation tools
  if: startsWith(matrix.os, 'ubuntu')
  run: |
    sudo apt-get update
    sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
    rustup target add ${{ matrix.target }}
```

**Build command**:
```bash
cargo build --release --target ${{ matrix.target }} --manifest-path crates/maproom/Cargo.toml
```

**Binary location**: `target/${{ matrix.target }}/release/crewchief-maproom`

**Post-processing**:
```bash
# Strip debug symbols
strip target/${{ matrix.target }}/release/crewchief-maproom

# Rename to platform-specific name
mv target/${{ matrix.target }}/release/crewchief-maproom \
   crewchief-maproom-${{ matrix.platform }}
```

**Artifact upload**:
```yaml
- name: Upload binary artifact
  uses: actions/upload-artifact@v4
  with:
    name: binary-${{ matrix.platform }}
    path: crewchief-maproom-${{ matrix.platform }}
    retention-days: 1
```

### Package Assembly

**Binary organization** (validate-and-publish job):
```
packages/cli/bin/
├── darwin-arm64/
│   └── crewchief-maproom
├── darwin-x64/
│   └── crewchief-maproom
├── linux-arm64/
│   └── crewchief-maproom
├── linux-x64/
│   └── crewchief-maproom
├── crewchief -> {current-platform}/crewchief-maproom
└── crewchief (shell wrapper - already exists)
```

**Assembly steps**:
```bash
# Download artifacts
actions/download-artifact@v4 (all 4 binaries)

# Organize into structure
mkdir -p packages/cli/bin/{darwin-arm64,darwin-x64,linux-arm64,linux-x64}
mv crewchief-maproom-darwin-arm64 packages/cli/bin/darwin-arm64/crewchief-maproom
mv crewchief-maproom-darwin-x64 packages/cli/bin/darwin-x64/crewchief-maproom
mv crewchief-maproom-linux-arm64 packages/cli/bin/linux-arm64/crewchief-maproom
mv crewchief-maproom-linux-x64 packages/cli/bin/linux-x64/crewchief-maproom

# Make executable
chmod +x packages/cli/bin/*/crewchief-maproom

# Create symlink for current platform
cd packages/cli/bin
ln -sf linux-x64/crewchief-maproom crewchief-maproom  # Default to linux-x64
```

### Validation Architecture

**Validation suite** (runs before publish):

**1. Binary existence check**:
```bash
for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
  if [ ! -f "packages/cli/bin/$platform/crewchief-maproom" ]; then
    echo "ERROR: Missing binary for $platform"
    exit 1
  fi
done
```

**2. Binary size check**:
```bash
for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
  size=$(stat -f%z "packages/cli/bin/$platform/crewchief-maproom" 2>/dev/null || \
         stat -c%s "packages/cli/bin/$platform/crewchief-maproom")
  if [ $size -lt 5000000 ] || [ $size -gt 20000000 ]; then
    echo "ERROR: Binary size $size out of range [5MB-20MB] for $platform"
    exit 1
  fi
done
```

**3. Native platform execution check**:
```bash
# Determine current platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64) ARCH="x64" ;;
  aarch64|arm64) ARCH="arm64" ;;
esac
PLATFORM="$OS-$ARCH"

# Test execution
packages/cli/bin/$PLATFORM/crewchief-maproom --version
if [ $? -ne 0 ]; then
  echo "ERROR: Binary execution failed for $PLATFORM"
  exit 1
fi
```

**4. TypeScript build check**:
```bash
if [ ! -d "packages/cli/dist" ] || [ -z "$(ls -A packages/cli/dist)" ]; then
  echo "ERROR: TypeScript build incomplete"
  exit 1
fi
```

**5. Package content check**:
```bash
npm pack --dry-run 2>&1 | tee pack-output.txt
# Check that bin/ directories are included
grep -q "bin/darwin-arm64" pack-output.txt || exit 1
grep -q "bin/darwin-x64" pack-output.txt || exit 1
grep -q "bin/linux-arm64" pack-output.txt || exit 1
grep -q "bin/linux-x64" pack-output.txt || exit 1
```

**6. Tarball inspection**:
```bash
npm pack
tar -tzf crewchief-cli-*.tgz | grep "bin/" | wc -l
# Should have >8 files (4 directories + 4 binaries + wrapper)
```

### Publication Architecture

**Pre-publish setup**:
```yaml
- name: Setup Node.js
  uses: actions/setup-node@v4
  with:
    node-version: '20'
    registry-url: 'https://registry.npmjs.org'

- name: Configure npm auth
  run: echo "//registry.npmjs.org/:_authToken=${{ secrets.NPM_TOKEN }}" > ~/.npmrc
```

**Publish step**:
```yaml
- name: Publish to npm
  if: ${{ !inputs.dry_run }}
  working-directory: packages/cli
  run: npm publish --access public
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

**Post-publish verification**:
```bash
# Wait for npm registry to update
sleep 10

# Extract version from package.json
VERSION=$(node -p "require('./packages/cli/package.json').version")

# Check package exists
npm view @crewchief/cli@$VERSION version

# Check tarball downloadable
npm pack @crewchief/cli@$VERSION

# Test installation
npm install -g @crewchief/cli@$VERSION

# Verify CLI works
crewchief --version
```

### Tag Strategy Architecture

**Package-scoped tag format**:
```
@crewchief/cli@v{major}.{minor}.{patch}

Examples:
- @crewchief/cli@v1.0.0
- @crewchief/cli@v1.0.1
- @crewchief/cli@v1.1.0
```

**Tag workflow triggers**:
```yaml
# CLI workflow
on:
  push:
    tags:
      - '@crewchief/cli@v*.*.*'

# MCP workflow (updated)
on:
  push:
    tags:
      - '@crewchief/maproom-mcp@v*.*.*'
```

**Tag creation** (release script):
```javascript
// packages/cli/scripts/release.mjs
const tag = `@crewchief/cli@v${nextVersion}`
execSync(`git tag ${tag}`, { stdio: 'inherit' })

// Two-step push to avoid race condition
execSync('git push', { stdio: 'inherit' })              // Push commits first
execSync(`git push origin ${tag}`, { stdio: 'inherit' }) // Then push tag separately
```

**Note**: The two-step push is critical. If commits and tags are pushed together with `--follow-tags`, the tag can arrive at GitHub before the commit is fully registered, causing workflow trigger failures.

**Independent versioning**:
- CLI: `@crewchief/cli@v1.0.0` (breaking rename)
- MCP: `@crewchief/maproom-mcp@v1.3.5` (unchanged)
- No coupling required
- Can release independently

### Deprecation Architecture

**Old package final release** (`crewchief@1.0.0`):

**1. Update package.json**:
```json
{
  "name": "crewchief",
  "version": "1.0.0",
  "deprecated": "This package has been renamed to @crewchief/cli. Please update your dependencies."
}
```

**2. Add postinstall warning**:
```javascript
// packages/cli-legacy/postinstall.js
console.warn('\n⚠️  DEPRECATION WARNING ⚠️');
console.warn('The "crewchief" package has been renamed to "@crewchief/cli"');
console.warn('Please update your package.json:');
console.warn('  npm uninstall crewchief');
console.warn('  npm install @crewchief/cli\n');
```

**3. Publish final version**:
```bash
cd packages/cli-legacy  # Copy of old package
npm version 1.0.0
npm publish
```

**4. Mark as deprecated on npm**:
```bash
npm deprecate crewchief@1.0.0 \
  "Package renamed to @crewchief/cli. Install @crewchief/cli instead."
```

### Configuration Changes

**File modifications required**:

**1. packages/cli/package.json**:
```json
{
  "name": "@crewchief/cli",        // Changed from "crewchief"
  "version": "1.0.0",               // Major bump for breaking change
  "publishConfig": {
    "access": "public"              // Required for scoped packages
  },
  "files": [
    "dist",
    "bin",                          // Includes all platform binaries
    "README.md",
    "LICENSE"
  ]
  // Remove "prepublishOnly": "pnpm build:all" - workflow handles build
}
```

**2. packages/cli/.npmignore**:
```
# Exclude source files
src/
tsconfig.json
*.test.ts

# Exclude development files
.git
.github
node_modules

# Include built artifacts
!dist/
!bin/
```

**3. packages/cli/scripts/release.mjs**:
```javascript
// Update tag format
const tag = `@crewchief/cli@v${nextVersion}`
execSync(`git tag ${tag}`, { stdio: 'inherit' })

// Two-step push to avoid race condition
execSync('git push', { stdio: 'inherit' })
execSync(`git push origin ${tag}`, { stdio: 'inherit' })

// Remove publish step (GitHub Actions handles it)
// execSync('pnpm publish --access public', { stdio: 'inherit' })

console.log(`✓ Tagged and pushed ${tag}`)
console.log('  GitHub Actions will build and publish automatically')
```

**4. packages/maproom-mcp/scripts/release.js**:
```javascript
// Update tag format for consistency
const tag = `@crewchief/maproom-mcp@v${version}`
execSync(`git tag -a ${tag} -m "Release version ${version}"`, { stdio: 'inherit' })

// Two-step push to avoid race condition
execSync('git push', { stdio: 'inherit' })
execSync(`git push origin ${tag}`, { stdio: 'inherit' })
```

## Component Interactions

**Build pipeline flow**:
```
[release.mjs] → [Git Tag] → [GitHub Actions Trigger]
                                     ↓
                     ┌───────────────┴───────────────┐
                     ▼                               ▼
            [Matrix: Build Binaries]      [Matrix: Build Binaries]
            (linux-x64, linux-arm64)      (darwin-x64, darwin-arm64)
                     │                               │
                     └───────────────┬───────────────┘
                                     ▼
                         [Collect Artifacts]
                                     ↓
                         [Build TypeScript]
                                     ↓
                         [Validate Package]
                                     ↓
                         [Publish to npm]
                                     ↓
                         [Verify Publication]
```

**Artifact flow**:
```
Binary Builds → Upload Artifacts → Download Artifacts → Assemble Package → Publish
    (parallel)     (temporary)        (centralized)       (validated)      (npm)
```

**Validation checkpoints**:
```
Build → Size Check → Execution Test → Package Inspection → Publish → Registry Verification
```

## Error Handling

**Build failures**:
- Individual platform build fails → Entire workflow fails (fail-fast)
- No partial publishes allowed
- Retry logic not needed (builds are deterministic)

**Validation failures**:
- Missing binary → Fail before publish
- Size out of range → Fail before publish
- Execution test fails → Fail before publish
- Package malformed → Fail before publish

**Publish failures**:
- npm authentication fails → Retry once, then fail
- Registry unavailable → Retry 3x with backoff, then fail
- Verification fails → Alert but don't block (might be registry delay)

**Rollback strategy**:
- Failed publish → Tag remains, workflow failed, no package published
- Broken package published → Publish corrected version immediately
- No automated rollback (semantic versioning prevents downgrades)

## Performance Considerations

**Build parallelization**:
- 4 platform builds run simultaneously
- Total time: ~8-12 minutes (longest single build)
- Sequential builds would take: ~35-45 minutes

**Artifact optimization**:
- Stripped binaries save 30-40% size
- Binary-only artifacts (no source code)
- 1-day retention (no long-term storage needed)

**Cache strategy**:
- Cargo registry cache: `~/.cargo/registry`
- Rust target cache: `target/` (per-platform)
- Node modules cache: `node_modules/`
- Cache keys include: OS, Rust version, Cargo.lock hash

**Runner costs** (estimated):
- Linux builds: 2 × 10 minutes × $0.008/min = $0.16
- macOS builds: 2 × 10 minutes × $0.08/min = $1.60
- Total per release: ~$1.76

## Alternative Architectures Considered

**Alternative 1: Cross-compile all platforms on Linux**:
- **Pros**: Single runner (cheaper), faster
- **Cons**: macOS cross-compilation unreliable, toolchain complex
- **Decision**: Rejected - reliability more important than cost

**Alternative 2: Unified workflow for both packages**:
- **Pros**: Less duplication, easier maintenance
- **Cons**: Complex conditional logic, harder to debug
- **Decision**: Rejected - clarity and independence more valuable

**Alternative 3: Platform-specific packages** (`@crewchief/cli-linux-x64`, etc.):
- **Pros**: Smaller downloads, optional platforms
- **Cons**: Complex postinstall, user confusion
- **Decision**: Rejected - all-in-one package is simpler

**Alternative 4: Use `semantic-release` or `changesets`**:
- **Pros**: Automated changelog, conventional commits
- **Cons**: Additional dependency, learning curve
- **Decision**: Deferred - can add later if needed

## Future Enhancements

**Not in scope for MVP, but worth considering**:

1. **Reusable workflow**: Extract common steps to `.github/workflows/build-rust-binaries.yml`
2. **Changelog generation**: Auto-generate from commit messages
3. **GitHub Releases**: Create release notes on GitHub
4. **Binary signing**: Code signing for macOS binaries
5. **Checksums**: SHA256 hashes for security verification
6. **Windows support**: Add windows-x64 platform
7. **ARM Linux native runners**: When GitHub provides them
8. **Canary releases**: Pre-release tags trigger canary publish
9. **Rollback automation**: Script to republish previous version
10. **Performance tracking**: Monitor build times, binary sizes over time

## Success Metrics

**Technical metrics**:
- ✅ All 4 platforms build successfully (100% success rate)
- ✅ Validation catches issues before publish (0 broken releases)
- ✅ Build time under 15 minutes (current: ~10 minutes)
- ✅ Package size under 100MB (current: ~60MB)

**Process metrics**:
- ✅ Zero manual steps after tag creation
- ✅ Independent CLI and MCP releases (no conflicts)
- ✅ Failed builds don't publish (fail-safe)

**User metrics**:
- ✅ CLI works on all 4 platforms after install
- ✅ Clear migration path from old package
- ✅ Deprecation warnings guide users to new package

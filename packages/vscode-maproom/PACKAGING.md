# VSIX Packaging Guide

## Quick Package Workflow

```bash
# 1. Prepare binaries from packages/cli/bin/
pnpm run prepare:binaries

# 2. Compile TypeScript
pnpm run compile

# 3. Package as VSIX
pnpm run vsce:package

# Or run all at once:
pnpm run prepackage && pnpm run vsce:package
```

## Package Scripts

- `prepare:binaries` - Copy Rust binaries from packages/cli/bin/ to bin/
- `compile` - Compile TypeScript to dist/
- `prepackage` - Run prepare:binaries + compile
- `vsce:package` - Create VSIX using vsce (no dependencies bundled)

## Binary Requirements

The extension requires platform-specific binaries in `bin/`:

```
bin/
├── darwin-arm64/maproom
├── darwin-x64/maproom
├── linux-arm64/maproom
├── linux-x64/maproom
└── win32-x64/maproom.exe
```

### Currently Available Platforms

- darwin-arm64 (macOS Apple Silicon)
- linux-arm64 (Linux ARM64)

### Missing Platforms

- darwin-x64 (macOS Intel)
- linux-x64 (Linux x86_64)
- win32-x64 (Windows)

## File Inclusion/Exclusion

### Included Files (.vscodeignore)

- Compiled JavaScript (`dist/**/*.js`)
- TypeScript declarations (`dist/**/*.d.ts`)
- Source maps (`dist/**/*.js.map`)
- Documentation (`README.md`, `CHANGELOG.md`, `TROUBLESHOOTING.md`)
- Configuration (`config/docker-compose.yml`)
- Platform binaries (`bin/**/maproom`)
- Package metadata (`package.json`)

### Excluded Files

- Source TypeScript (`src/**`)
- Tests (`test/**`)
- Development dependencies (`node_modules`)
- Build configuration (`tsconfig.json`, `vitest.config.ts`)
- Scripts (`scripts/**`)
- Coverage reports (`coverage/**`)

## Package Metadata

Key fields in `package.json`:

```json
{
  "name": "vscode-maproom",
  "displayName": "Maproom Semantic Search",
  "publisher": "crewchief",
  "version": "0.1.0",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/danielbushman/crewchief.git"
  }
}
```

## VSIX Verification

```bash
# Check file size
ls -lh vscode-maproom-0.1.0.vsix

# List contents
unzip -l vscode-maproom-0.1.0.vsix

# Verify binaries
unzip -l vscode-maproom-0.1.0.vsix | grep bin/

# Check for excluded files
unzip -l vscode-maproom-0.1.0.vsix | grep -E "(src/|test/|node_modules)"
```

## Local Installation

```bash
# Install locally for testing
code --install-extension vscode-maproom-0.1.0.vsix

# Uninstall
code --uninstall-extension crewchief.vscode-maproom
```

## Publishing to Marketplace

### Prerequisites

1. Add LICENSE file to repository
2. Build all platform binaries
3. Create publisher account at https://marketplace.visualstudio.com/manage

### Publish Command

```bash
# Login (first time only)
vsce login crewchief

# Publish
vsce publish

# Or publish specific version
vsce publish patch  # 0.1.0 -> 0.1.1
vsce publish minor  # 0.1.0 -> 0.2.0
vsce publish major  # 0.1.0 -> 1.0.0
```

## Troubleshooting

### Binary Not Found

If prepare:binaries fails, check that binaries exist in `/packages/cli/bin/{platform}/maproom`

### Permission Errors

Unix binaries must be executable (755). The prepare-binaries script handles this automatically.

### Package Too Large

Current size: ~8.9 MB (well under 50MB limit)
If size exceeds limit, check for accidentally included node_modules or test files.

### Missing Platforms Warning

The extension works on available platforms only. Missing platforms are non-fatal - users on those platforms will receive appropriate error messages.

## Size Optimization

Current breakdown:
- darwin-arm64 binary: 9.7 MB
- linux-arm64 binary: 16 MB
- Compiled code + docs: ~400 KB

To reduce size:
- Strip debug symbols from Rust binaries: `cargo build --release`
- Remove source maps: Add `*.map` to `.vscodeignore`
- Compress binaries: Use UPX (Universal Packer for eXecutables)

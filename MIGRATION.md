# Migration Guide: crewchief → @crewchief/cli

## Overview

The `crewchief` package has been renamed to `@crewchief/cli` as of v1.0.0 (November 2025).

## Why the Change?

- **Naming convention**: Aligns with org-scoped pattern (`@crewchief/*`)
- **Independent versioning**: CLI and MCP packages can release separately
- **Multi-platform binaries**: New workflow builds for all platforms automatically
- **Better organization**: Clear namespace for the CrewChief ecosystem

## Quick Migration

### 1. Uninstall Old Package

```bash
npm uninstall -g crewchief
```

### 2. Install New Package

```bash
npm install -g @crewchief/cli
```

### 3. Verify Installation

```bash
crewchief --version
# Should show: 1.0.0 or higher
```

That's it! The `crewchief` command still works the same way.

## What Changed?

### Package Name
- **Old**: `crewchief`
- **New**: `@crewchief/cli`

### Binary Distribution
- **Old**: Single platform (whoever ran the release)
- **New**: All 4 platforms included
  - linux-x64 (Intel/AMD Linux)
  - linux-arm64 (ARM Linux, Raspberry Pi, etc.)
  - darwin-x64 (Intel Mac)
  - darwin-arm64 (Apple Silicon Mac)

### Release Process
- **Old**: Manual local publish from developer machine
- **New**: Automated GitHub Actions workflow
  - Consistent builds across all platforms
  - Validated before publish
  - Dry-run testing capability

### Version Number
- **Old**: v0.1.23 (last version of unscoped package)
- **New**: v1.0.0 (first version of scoped package)

## Breaking Changes

**v1.0.0**:
- ✅ Package name changed (requires reinstall)
- ✅ CLI commands remain **exactly the same**
- ✅ Configuration files remain compatible
- ✅ Database schemas unchanged

**No functionality changes** - only the npm package name changed.

## Installation Options

### Global Installation (Recommended)

```bash
npm install -g @crewchief/cli
```

This gives you the `crewchief` command available system-wide.

### Project-Local Installation

```bash
npm install @crewchief/cli
```

Then use via `npx`:

```bash
npx crewchief --help
```

### Temporary Usage

```bash
npx @crewchief/cli --help
```

Downloads and runs temporarily without installation.

## Platform Support

The new package automatically selects the correct binary for your platform:

| Platform | Architecture | Support |
|----------|-------------|---------|
| macOS | Apple Silicon (M1/M2/M3) | ✅ |
| macOS | Intel | ✅ |
| Linux | x86_64 (Intel/AMD) | ✅ |
| Linux | ARM64 | ✅ |
| Windows | Any | ❌ Not yet supported |

The appropriate binary is selected automatically based on your `process.platform` and `process.arch`.

## Troubleshooting

### Old Package Still Installed?

Check if the old package is still present:

```bash
npm list -g crewchief
```

If found, uninstall it:

```bash
npm uninstall -g crewchief
```

### Command Not Found After Migration

Ensure the new package is installed globally:

```bash
npm install -g @crewchief/cli
which crewchief  # Should show path to binary
```

### Wrong Version Showing

Clear npm cache and reinstall:

```bash
npm cache clean --force
npm uninstall -g @crewchief/cli
npm install -g @crewchief/cli
crewchief --version  # Should be 1.0.0+
```

### Binary Not Executing

Ensure you have the right platform binary:

```bash
# Check your platform
node -e "console.log(process.platform, process.arch)"

# Reinstall to get correct binary
npm uninstall -g @crewchief/cli
npm install -g @crewchief/cli
```

## Old Package Deprecation

The old `crewchief` package (unscoped) is now **deprecated**:

- Published as `crewchief@1.0.0` with deprecation warnings
- Shows migration message on installation
- No future updates or security patches
- Will be unpublished eventually

When someone tries to install it:

```bash
npm install crewchief
```

They'll see:

```
npm WARN deprecated crewchief@1.0.0: This package has been renamed to @crewchief/cli
```

Plus a postinstall message with migration instructions.

## Support

### New Package (`@crewchief/cli`)
- ✅ Active development
- ✅ Regular updates
- ✅ Full platform support
- ✅ Automated releases
- ✅ Security patches

### Old Package (`crewchief`)
- ❌ Deprecated
- ❌ No updates
- ❌ Limited platform support
- ❌ Manual releases only

## Questions or Issues?

- **GitHub Issues**: [manifoldlogic/crewchief/issues](https://github.com/manifoldlogic/crewchief/issues)
- **npm Package**: [npmjs.com/package/@crewchief/cli](https://www.npmjs.com/package/@crewchief/cli)
- **Repository**: [github.com/manifoldlogic/crewchief](https://github.com/manifoldlogic/crewchief)

## Timeline

- **v0.1.23** - Last release of `crewchief` (unscoped)
- **November 8, 2025** - Package renamed to `@crewchief/cli@1.0.0`
- **November 8, 2025** - Old package deprecated with migration warnings
- **Future** - All development continues on `@crewchief/cli`

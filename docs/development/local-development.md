# Local Development Guide

This guide explains how to run CrewChief using local development code instead of the published npm package. This is useful when developing new features, debugging issues, or testing changes before publishing.

## Prerequisites

- Node.js 18+ and pnpm installed
- Rust toolchain installed (for building Rust binaries)
- PostgreSQL running locally (for maproom commands)

## Quick Start

```bash
# Install dependencies
pnpm install

# Build everything (TypeScript + Rust)
cd packages/cli
pnpm build:all

# Run using local code
node dist/cli/index.js --help
```

## Using Local TypeScript Code

### Method 1: Direct Node Execution (Recommended for Development)

```bash
# From packages/cli directory
node dist/cli/index.js <command> [options]

# Examples:
node dist/cli/index.js agent list
node dist/cli/index.js worktree create my-feature
node dist/cli/index.js maproom:scan
```

### Method 2: Using tsx for Development (No Build Required)

```bash
# From packages/cli directory
pnpm dev <command> [options]
# or directly:
tsx src/cli/index.ts <command> [options]

# Examples:
tsx src/cli/index.ts agent list
tsx src/cli/index.ts worktree status
```

### Method 3: Global Link (Use Local Code System-Wide)

```bash
# From packages/cli directory
pnpm build
pnpm link --global

# Now you can use 'crewchief' command anywhere with your local code
crewchief --help
```

## Using Local Rust Binaries

### Building Rust Binaries

```bash
# Build all Rust binaries for your platform
cd packages/cli
pnpm build:rust

# Or build manually from project root
cargo build --release --bin crewchief-maproom
```

### Method 1: Configuration File (Recommended)

When developing maproom, configure CrewChief to use your local build:

```javascript
// crewchief.config.local.js
export default {
  repository: {
    maproomBinaryPath: './target/release/crewchief-maproom'
  }
}
```

Build and test:
```bash
cd crates/maproom
cargo build --release
cd ../..
crewchief maproom scan  # Uses your local build
```

This approach is preferred over setting `CREWCHIEF_MAPROOM_BIN` because:
- Config persists across terminal sessions
- Can use `.local.js` to keep out of git
- Relative paths work from any location in repo

### Method 2: Environment Variable Override

```bash
# Set the path to your local binary
export CREWCHIEF_MAPROOM_BIN="./packages/cli/bin/darwin-arm64/crewchief-maproom"

# Or use absolute path
export CREWCHIEF_MAPROOM_BIN="/path/to/crewchief/packages/cli/bin/darwin-arm64/crewchief-maproom"

# Now run commands normally
crewchief maproom:scan
crewchief maproom:watch
```

### Method 3: Inline Environment Variable

```bash
# Specify the binary path inline (from project root)
CREWCHIEF_MAPROOM_BIN="./packages/cli/bin/darwin-arm64/crewchief-maproom" crewchief maproom:scan

# Or with absolute path
CREWCHIEF_MAPROOM_BIN="$(pwd)/packages/cli/bin/darwin-arm64/crewchief-maproom" crewchief maproom:scan
```

### Method 4: Direct Binary Execution

```bash
# Run the Rust binary directly
./packages/cli/bin/darwin-arm64/crewchief-maproom scan --help
./packages/cli/bin/darwin-arm64/crewchief-maproom scan --repo myrepo --worktree main
```

## Using Both Local TypeScript and Rust Code Together

### Complete Local Development Setup

```bash
# 1. Build everything
cd packages/cli
pnpm build:all  # Builds both TypeScript and Rust

# 2. Run with local TypeScript code and local Rust binary
CREWCHIEF_MAPROOM_BIN="./bin/darwin-arm64/crewchief-maproom" node dist/cli/index.js maproom:scan

# Or use tsx for TypeScript (no build needed) with local Rust binary
CREWCHIEF_MAPROOM_BIN="./bin/darwin-arm64/crewchief-maproom" tsx src/cli/index.ts maproom:scan
```

### Development Workflow Example

```bash
# Terminal 1: Watch TypeScript changes
cd packages/cli
pnpm build --watch

# Terminal 2: Rebuild Rust on changes
cargo watch -x "build --release --bin crewchief-maproom"

# Terminal 3: Run commands with local code
export CREWCHIEF_MAPROOM_BIN="./packages/cli/bin/darwin-arm64/crewchief-maproom"
node packages/cli/dist/cli/index.js maproom:scan
```

## Platform-Specific Paths

The Rust binaries are organized by platform. Replace `darwin-arm64` with your platform:

- **macOS Apple Silicon**: `darwin-arm64`
- **macOS Intel**: `darwin-x64`
- **Linux x64**: `linux-x64`
- **Linux ARM64**: `linux-arm64`
- **Windows x64**: `win32-x64`

## Troubleshooting

### Binary Not Found

If you get "crewchief-maproom not found" errors:

1. Check the binary exists:
   ```bash
   ls -la packages/cli/bin/*/crewchief-maproom
   ```

2. Rebuild the Rust binary:
   ```bash
   cd packages/cli
   pnpm build:rust
   ```

3. Use absolute path:
   ```bash
   CREWCHIEF_MAPROOM_BIN="$(pwd)/packages/cli/bin/darwin-arm64/crewchief-maproom" node dist/cli/index.js maproom:scan
   ```

### TypeScript Changes Not Reflected

1. Rebuild TypeScript:
   ```bash
   cd packages/cli
   pnpm build
   ```

2. Or use tsx for instant changes:
   ```bash
   tsx src/cli/index.ts <command>
   ```

### Database Connection Issues

For maproom commands, ensure PostgreSQL is running and set MAPROOM_DATABASE_URL:

```bash
export MAPROOM_DATABASE_URL="postgresql://user:password@localhost:5432/crewchief"
# Or use a .env file in the project root
```

## Common Development Commands

```bash
# Full rebuild
cd packages/cli
pnpm build:all

# Test maproom scan with local code
CREWCHIEF_MAPROOM_BIN="./bin/darwin-arm64/crewchief-maproom" node dist/cli/index.js maproom:scan

# Test agent commands with local code  
node dist/cli/index.js agent list
node dist/cli/index.js agent create my-agent --type cursor

# Test with specific worktree
node dist/cli/index.js worktree create test-branch
node dist/cli/index.js agent run --worktree test-branch

# Run with debug output
DEBUG=* node dist/cli/index.js <command>
```

## VSCode Launch Configuration

Add to `.vscode/launch.json` for debugging:

```json
{
  "type": "node",
  "request": "launch",
  "name": "Debug CLI Command",
  "skipFiles": ["<node_internals>/**"],
  "program": "${workspaceFolder}/packages/cli/dist/cli/index.js",
  "args": ["maproom:scan"],
  "env": {
    "CREWCHIEF_MAPROOM_BIN": "${workspaceFolder}/packages/cli/bin/darwin-arm64/crewchief-maproom",
    "MAPROOM_DATABASE_URL": "postgresql://localhost:5432/crewchief"
  },
  "cwd": "${workspaceFolder}",
  "console": "integratedTerminal"
}
```

## Summary

- **Local TypeScript**: Use `node dist/cli/index.js` or `tsx src/cli/index.ts`
- **Local Rust**: Set `CREWCHIEF_MAPROOM_BIN` environment variable
- **Both**: Combine the above techniques
- **Quick iteration**: Use `tsx` for TypeScript, `cargo watch` for Rust
- **System-wide**: Use `pnpm link --global` after building
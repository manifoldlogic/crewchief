# CLAUDE.md - CLI Package

Working with the TypeScript CLI package at `/packages/cli`.

## Directory Structure

```
src/
├── agents/        # Agent registry, runner, discovery
├── bus/           # JSONL message bus
├── cli/           # Commander.js commands
├── config/        # Zod config validation
├── git/           # Worktree operations
├── iterm/         # iTerm2 integration
├── orchestrator/  # Run management
├── terminal/      # Terminal abstraction
└── utils/         # Shared utilities
```

## Development

```bash
# Build
pnpm build              # TypeScript → dist/
pnpm build:all          # TypeScript + Rust binaries

# Test
pnpm test
pnpm test:watch

# Run without building
tsx src/cli/index.ts --help

# Code quality
pnpm lint
pnpm format
```

## Key Points

- **ESM modules** - Use `import/export`
- **Vitest** for tests (colocated with source)
- **Commander.js** for CLI (entry: `src/cli/index.ts`)
- **Zod** for config validation
- **Rust binaries** in `bin/<platform>/` (rebuild with `pnpm build:rust` when changing `crates/maproom/`)

## Components

- **agents/** - Registry and runner
- **bus/** - JSONL inter-agent messaging
- **git/** - Worktree management
- **config/** - Config loading with Zod
- **iterm/** - macOS terminal automation
- **orchestrator/** - Run scheduling
- **terminal/** - Backend abstraction

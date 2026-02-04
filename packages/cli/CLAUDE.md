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

## Terminal Providers

CrewChief supports multiple terminal backends for spawning agents:

### iTerm2 (macOS)

- **Auto-detected**: When `TERM_PROGRAM=iTerm.app`
- **Explicit use**: `--backend iterm`
- **Requirements**: iTerm2 installed, macOS only
- **Best for**: macOS development with visual pane management

### tmux (Linux/macOS)

- **Auto-detected**: When `TMUX` environment variable is set
- **Explicit use**: `--backend tmux`
- **Requirements**: tmux >= 2.1 installed
- **Best for**: Remote servers, SSH sessions, Linux environments

### Headless (Any OS)

- **Auto-detected**: When no terminal detected or `--headless` flag
- **Explicit use**: `--backend headless`
- **Requirements**: None
- **Best for**: CI/CD, REPL mode, background processes
- **Note**: Logs written to `.crewchief/runs/<runId>/logs/`

### Auto (Default)

- **Detection order**:
  1. `--headless` flag -> headless
  2. `TMUX` env var -> tmux
  3. `TERM_PROGRAM=iTerm.app` -> iterm
  4. Otherwise -> headless

### Configuration

```typescript
// .crewchief/config.ts
export default {
  terminal: {
    backend: 'auto', // or 'iterm', 'tmux', 'headless'
    tmux: {
      sessionName: 'crewchief', // Custom tmux session name
    },
    iterm: {
      sessionName: 'crewchief', // Custom iTerm session name
    },
  },
}
```

### Usage Examples

```bash
# Auto-detect backend
crewchief agent spawn claude "task"

# Explicit tmux (requires tmux installed)
crewchief agent spawn claude "task" --backend tmux

# Headless with log files
crewchief agent spawn claude "task" --backend headless
crewchief runs logs <runId>  # View logs later

# iTerm2 on macOS
crewchief agent spawn claude "task" --backend iterm
```

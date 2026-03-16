# Contributing to CrewChief

Thank you for your interest in contributing to CrewChief! This guide covers everything you need to get started.

Please review our [Code of Conduct](CODE_OF_CONDUCT.md) before contributing.

## Development Setup

### Prerequisites

- **Node.js** >= 18 (22 recommended)
- **pnpm** (package manager for TypeScript packages)
- **Rust** and **cargo** (for the maproom indexer crate)
- **Git**

### Using the DevContainer (Recommended)

The fastest way to get a working development environment is with the included devcontainer:

1. Open the repo in VS Code or Cursor
2. When prompted, choose **"Reopen in Container"** (or run the command manually: `Dev Containers: Reopen in Container`)
3. The container installs all prerequisites automatically

### Manual Setup

If you prefer not to use the devcontainer:

1. Install the prerequisites listed above
2. Clone the repository and install dependencies:
   ```bash
   git clone https://github.com/crewchief-org/crewchief.git
   cd crewchief
   pnpm install
   ```
3. Build the Rust binary:
   ```bash
   cd crates/maproom
   cargo build
   ```
4. Build the TypeScript packages:
   ```bash
   pnpm build
   ```

## Configuration

This project uses the gitignore + example pattern for personal settings:

1. Copy `.claude/settings.example.json` to `.claude/settings.json`
2. Customize as needed for your environment
3. The `.claude/settings.json` file is gitignored and will not be committed

For personal devcontainer customizations (additional mounts, env vars), create
`.devcontainer/docker-compose.override.yml` (also gitignored).

## Building and Testing

This is a pnpm monorepo with TypeScript packages under `packages/` and a Rust crate under `crates/maproom/`.

### TypeScript

```bash
pnpm install        # Install dependencies
pnpm build          # Build all TypeScript packages
pnpm test           # Run tests across all packages
```

Per-package commands (from the package directory):

| Package                | Build                    | Test                 |
| ---------------------- | ------------------------ | -------------------- |
| `packages/cli`         | `pnpm build` (uses tsup) | `pnpm test` (vitest) |
| `packages/maproom-mcp` | `pnpm build` (uses tsc)  | `pnpm test` (vitest) |

### Rust

```bash
cd crates/maproom
cargo build          # Build the maproom binary
cargo test           # Run tests
```

## Code Style

### TypeScript

- Follow the existing patterns in the codebase
- ESM-only — use `import`/`export` with explicit `.js` extensions in import paths
- Run linting and formatting:
  ```bash
  pnpm lint           # ESLint
  pnpm format:check   # Prettier (check)
  pnpm format         # Prettier (fix)
  ```

### Rust

- Format with `cargo fmt`
- Lint with `cargo clippy`

## Submitting a Pull Request

1. **Fork** the repository and clone your fork
2. **Create a branch** for your changes (`git checkout -b my-feature`)
3. **Make your changes** and add tests where appropriate
4. **Run tests** to confirm everything passes (`pnpm test` and/or `cargo test`)
5. **Commit** with a clear, descriptive message
6. **Push** your branch and open a pull request against `main`

### What reviewers look for

- Tests for new functionality
- Consistent code style with the rest of the project
- Clear commit messages
- No unrelated changes bundled into the PR

## CLAUDE.md Convention

This project uses `CLAUDE.md` files to provide context to [Claude Code](https://claude.ai/code) (Anthropic's coding assistant). Each package and crate has its own `CLAUDE.md` containing:

- Build and test commands specific to that component
- Common pitfalls and troubleshooting tips
- Architecture notes relevant to the component

If your contribution adds significant new patterns, changes architecture, or introduces new subsystems, please update the relevant `CLAUDE.md` file so future contributors (and Claude Code) stay informed.

See the root `CLAUDE.md` for a full index of all `CLAUDE.md` files in the monorepo.

## Getting Help

- Check package-level READMEs for detailed documentation on individual components
- Review `docs/` for architecture docs and troubleshooting guides
- Open an issue if you have questions or run into problems

# CLI Package

## Pitfalls

- **ESM-only**: All imports must use `import/export` with explicit `.js` extensions in import paths
- **Colocated tests**: Tests live next to source files (not in a separate `tests/` dir). Framework is Vitest.
- **Rust binary location**: Built binaries land in `bin/<platform>/crewchief-maproom` (e.g., `bin/darwin-arm64/`). Rebuild with `pnpm build:rust` after Rust changes.
- **Commander.js entry**: `src/cli/index.ts` — all subcommands registered there

## Subsystems

- `src/search-optimization/` has its own CLAUDE.md with benchmark guidance

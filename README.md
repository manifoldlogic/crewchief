# crewchief

Semantic code search toolkit for AI-assisted development.

![CI](https://github.com/manifoldlogic/crewchief/actions/workflows/test.yml/badge.svg)
![Release CLI](https://github.com/manifoldlogic/crewchief/actions/workflows/release-cli.yml/badge.svg)

## What is crewchief?

crewchief indexes your codebase using tree-sitter, stores chunks in a local SQLite database, and enables semantic search via embeddings and full-text search. It can be integrated with Claude Code via MCP for AI-assisted code navigation.

## Monorepo Layout

| Package / Crate                                       | Description                                             | Docs                                        |
| ----------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------- |
| [`packages/cli`](packages/cli/)                       | TypeScript CLI for worktree and agent management        | [README](packages/cli/README.md)            |
| [`packages/maproom-mcp`](packages/maproom-mcp/)       | MCP server (**no longer actively maintained** — use `@crewchief/cli` and `maproom` directly) | [README](packages/maproom-mcp/README.md)    |
| [`packages/vscode-maproom`](packages/vscode-maproom/) | VS Code extension (**no longer actively maintained** — use `@crewchief/cli` and `maproom` directly) | [README](packages/vscode-maproom/README.md) |
| [`crates/maproom`](crates/maproom/)                   | Rust indexer — file watching, embedding, SQLite storage | [README](crates/maproom/README.md)          |
| [`packages/daemon-client`](packages/daemon-client/)   | Daemon RPC client library                               | [README](packages/daemon-client/README.md)  |

## Getting Started

### CLI (recommended)

```bash
npm install -g @crewchief/cli
crewchief --help
```

See the [CLI README](packages/cli/README.md) for detailed installation instructions and usage.

### Rust Indexer

```bash
cargo install maproom
```

See the [maproom README](crates/maproom/README.md) for usage and configuration.

## Deprecated Packages

The following packages are **no longer actively maintained**:

- **`@crewchief/maproom-mcp`** — The standalone MCP server. Use [`@crewchief/cli`](https://www.npmjs.com/package/@crewchief/cli) instead, which includes integrated MCP support.
- **`vscode-maproom`** — The VS Code extension. Use [`@crewchief/cli`](https://www.npmjs.com/package/@crewchief/cli) and the [`maproom`](https://crates.io/crates/maproom) binary directly.

These packages remain in the repository for reference but will not receive new features or bug fixes.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the [MIT License](LICENSE).

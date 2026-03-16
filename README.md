# crewchief

Semantic code search toolkit for AI-assisted development.

![CI](https://github.com/danielbushman/crewchief/actions/workflows/test.yml/badge.svg)
![Release CLI](https://github.com/danielbushman/crewchief/actions/workflows/release-cli.yml/badge.svg)
![Release MCP](https://github.com/danielbushman/crewchief/actions/workflows/release-maproom-mcp.yml/badge.svg)
![Release VS Code](https://github.com/danielbushman/crewchief/actions/workflows/release-vscode-maproom.yml/badge.svg)

## What is crewchief?

crewchief indexes your codebase using tree-sitter, stores chunks in a local SQLite database, and enables semantic search via embeddings and full-text search. It integrates with Claude Code via MCP to provide AI-assisted code navigation, and includes a VS Code extension for interactive use.

## Monorepo Layout

| Package / Crate                                       | Description                                             | Docs                                        |
| ----------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------- |
| [`packages/cli`](packages/cli/)                       | TypeScript CLI for worktree and agent management        | [README](packages/cli/README.md)            |
| [`packages/maproom-mcp`](packages/maproom-mcp/)       | MCP server exposing semantic search to Claude Code      | [README](packages/maproom-mcp/README.md)    |
| [`packages/vscode-maproom`](packages/vscode-maproom/) | VS Code extension for maproom integration               | [README](packages/vscode-maproom/README.md) |
| [`crates/maproom`](crates/maproom/)                   | Rust indexer — file watching, embedding, SQLite storage | [README](crates/maproom/README.md)          |
| [`packages/daemon-client`](packages/daemon-client/)   | Daemon RPC client library                               | [README](packages/daemon-client/README.md)  |

## Getting Started

### CLI (recommended)

```bash
npm install -g @crewchief/cli
crewchief --help
```

See the [CLI README](packages/cli/README.md) for detailed installation instructions and usage.

### VS Code Extension

Install the **Maproom Semantic Search** extension from the VS Code marketplace. See the [extension README](packages/vscode-maproom/README.md) for setup details.

### Rust Indexer

```bash
cargo install maproom
```

See the [maproom README](crates/maproom/README.md) for usage and configuration.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the [MIT License](LICENSE).

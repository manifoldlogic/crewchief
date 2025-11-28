# Maproom Semantic Search

> Semantic code search powered by Maproom - index and search your codebase by meaning, not just text.

Maproom brings AI-powered semantic search to Visual Studio Code. Index your workspace once, then search by concept instead of keywords. Find code by what it does, not just what it's called.

## Quick Start

Get started in minutes with zero configuration:

1. **Install the extension** from marketplace
2. **Create an index** using the CLI:
   ```bash
   crewchief-maproom scan /path/to/your/repo
   ```
3. **Search!** Use "Maproom: Search" command (`Cmd+Shift+P` / `Ctrl+Shift+P`)

That's it! The extension auto-detects `~/.maproom/maproom.db` - no Docker required.

## Features

- **Semantic Search** - Find code by meaning using AI embeddings, not just text matching
- **Zero-Config SQLite** - Works immediately with existing `~/.maproom/maproom.db`
- **Real-Time Indexing** - File watching keeps your index fresh as you code
- **Provider Choice** - Use Ollama (local, free), OpenAI, or Google Gemini for embeddings
- **Crash Recovery** - Automatic restart with exponential backoff if processes crash
- **Secure Credentials** - API keys stored safely in VSCode SecretStorage
- **Status Tracking** - Real-time status bar showing indexed files and database status

## System Requirements

| Requirement | Minimum | Recommended |
|------------|---------|-------------|
| VSCode | 1.85.0+ | Latest stable |
| RAM | 2GB | 4GB |
| Free Disk Space | 500MB | 2GB |

**No Docker required!** Just install the extension and create an index.

## Platform Support

| Platform | Architecture | Status | Notes |
|----------|--------------|--------|-------|
| Linux | x64 | Supported | Full support |
| Linux | arm64 | Supported | Full support |
| macOS | arm64 (M1+) | Supported | Full support |
| macOS | x64 (Intel) | Supported | Full support |
| Windows | x64 | Experimental | File watching may be slower |

**Windows users**: Experimental support. Please report any issues you encounter.

## Installation

### From VSIX

1. Download the latest `.vsix` file from releases
2. Open VSCode
3. Open Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`)
4. Run: `Extensions: Install from VSIX...`
5. Select the downloaded file
6. Reload VSCode when prompted

**Or via command line**:
```bash
code --install-extension vscode-maproom-0.3.3.vsix
```

### From Source (Development)

```bash
git clone https://github.com/danielbushman/crewchief.git
cd crewchief/packages/vscode-maproom
pnpm install
pnpm compile
code --extensionDevelopmentPath=$(pwd)
```

## Settings Reference

| Setting | Description | Default |
|---------|-------------|---------|
| `maproom.database.sqlitePath` | Custom SQLite path (empty = `~/.maproom/maproom.db`) | `""` |

## Commands

Open Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`) and run:

| Command | Description | When to Use |
|---------|-------------|-------------|
| `Maproom: Setup` | Re-run setup wizard | Change embedding provider or update API key |
| `Maproom: Show Output` | Open Maproom output channel | View detailed logs, debug issues |
| `Maproom: Restart Watchers` | Restart file watching processes | If file watching stops or seems stuck |
| `Maproom: Show Status` | Show current Maproom status | Check database status and connection |

## Status Bar

The Maproom status bar item shows current state:

- **Starting...** - Services initializing
- **Indexing: 1,234 files** - Scan in progress
- **Watching: 5,678 files** - Active and ready
- **Error** - Click to view details in output channel

The tooltip shows additional information:
- Database path
- Last indexed timestamp

## Embedding Providers

### Ollama (Recommended for Privacy)

**Pros**: Free, private, runs locally, no API keys
**Cons**: Requires 4GB RAM, slower on first run (model download)

**Setup**:
1. Install Ollama: https://ollama.ai/download
2. Run: `ollama pull nomic-embed-text`
3. Keep Ollama running while using the extension

The setup wizard auto-detects if Ollama is running on port 11434.

### OpenAI

**Pros**: Fast, accurate, reliable
**Cons**: Requires API key, costs money per request

**Setup**:
1. Get API key: https://platform.openai.com/api-keys
2. Enter key when prompted (stored securely)
3. Uses `text-embedding-ada-002` model

### Google Gemini

**Pros**: Fast, competitive pricing
**Cons**: Requires API key and Google Cloud setup

**Setup**:
1. Enable Vertex AI API: https://console.cloud.google.com/
2. Get API key: https://cloud.google.com/vertex-ai/docs/authentication
3. Enter key when prompted (stored securely)

## Troubleshooting

### "SQLite database not found"

**Error**: `SQLite database not found at: /Users/you/.maproom/maproom.db`

**Cause**: No index has been created yet.

**Solution**: Create an index using the CLI:
```bash
crewchief-maproom scan /path/to/your/repo
```

### Custom SQLite Path Not Working

**Cause**: Path may contain unsupported characters or doesn't exist.

**Solutions**:
1. Ensure the path exists and is readable
2. Use absolute paths (not relative)
3. Tilde (`~`) is supported for home directory
4. Check the Output channel for detailed error messages

### Binary Permission Denied (Linux/macOS)

**Error**: `EACCES: permission denied`

**Solution**:
```bash
chmod +x ~/.vscode/extensions/manifoldlogic.vscode-maproom-*/bin/*/crewchief-maproom
```

### Ollama Not Detected

**Solutions**:
1. Verify Ollama is running: `ollama list`
2. Check Ollama API: `curl http://localhost:11434`
3. Or choose OpenAI/Google instead

### Process Keeps Crashing

**Solutions**:
1. Check output channel: `Maproom: Show Output`
2. Try restarting: `Maproom: Restart Watchers`
3. Check system resources (RAM, disk space)

### Slow Performance

**Solutions**:
1. Large repos (>10k files) can take 5-10 minutes initially
2. Subsequent updates are incremental (much faster)

## Known Limitations

- **Windows Support**: Experimental. File watching may be slower or miss some events.
- **Large Repositories**: Initial scan of >10,000 files can take 5-10 minutes.
- **Memory Usage**: Embedding requires 2-4GB RAM for large codebases.
- **Binary Files**: Only text files are indexed.
- **Network Required**: OpenAI and Google providers require active internet.
- **Ollama Model**: Must download ~500MB model on first use (one-time).

## Architecture

Maproom uses a multi-process architecture:

```
VSCode Extension (Node.js)
  ├── Database Checker → SQLite file detection
  ├── Process Orchestrator → File watchers
  │   ├── Initial scanner - One-time full scan
  │   └── File watcher - Real-time change detection
  └── Setup Wizard → Provider selection and credentials
```

**Key components**:
- **Database Checker**: SQLite configuration and availability
- **Setup Wizard**: Provider selection and credential management
- **Status Bar**: Real-time status and file count
- **Crash Recovery**: Exponential backoff restart (5 attempts)

## Development

### Prerequisites

- Node.js 20+
- pnpm 8+
- VSCode 1.85+

### Setup

```bash
# Install dependencies
pnpm install

# Compile TypeScript
pnpm compile

# Run tests
pnpm test

# Run tests with coverage
pnpm test:coverage

# Watch mode
pnpm test:watch
```

### Package Extension

```bash
# Install vsce
npm install -g @vscode/vsce

# Package extension
pnpm vsce:package

# Output: vscode-maproom-0.3.3.vsix
```

## Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure tests pass: `pnpm test`
5. Submit a pull request

**Development guidelines**:
- Maintain >70% test coverage
- Follow TypeScript strict mode
- Use ESM modules (import/export)
- Add JSDoc comments for public APIs
- Update CHANGELOG.md

## License

MIT License - see LICENSE file for details

## Support

- **Bug Reports**: https://github.com/danielbushman/crewchief/issues
- **Feature Requests**: https://github.com/danielbushman/crewchief/issues
- **Documentation**: https://github.com/danielbushman/crewchief/tree/main/packages/vscode-maproom

## Acknowledgements

Built with:
- [VSCode Extension API](https://code.visualstudio.com/api)
- [SQLite](https://www.sqlite.org/) for zero-config storage
- [Ollama](https://ollama.ai/) for local embeddings
- [Maproom](https://github.com/danielbushman/crewchief) semantic search engine

---

**Status**: Beta (v0.3.3) - Actively developed, feedback welcome!

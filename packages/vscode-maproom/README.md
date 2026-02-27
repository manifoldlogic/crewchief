# Maproom Semantic Search

Search your code by meaning, not just text. Find functions by what they do, not what they're called.

## Before You Install

**Start Ollama first** for the smoothest experience:

```bash
# Install Ollama from https://ollama.ai/download, then:
ollama pull mxbai-embed-large
ollama serve
```

This downloads the embedding model (~670MB) upfront so the extension activates quickly.

## Quick Start

1. **Ensure Ollama is running** (see above)
2. **Install the extension** from marketplace
3. **Open a git repository** in VSCode
4. **Wait for indexing** - watch the status bar for progress

The extension auto-indexes your workspace and starts watching for changes. No manual CLI commands needed.

## What Happens on First Run

1. Extension detects Ollama on `localhost:11434`
2. Creates database at `~/.maproom/maproom.db`
3. Scans and indexes your codebase (progress shown in status bar)
4. Starts watching for file changes

**Large codebases**: Initial scan can take several minutes for repositories with thousands of files. Subsequent updates are incremental and fast.

## Platform Support

| Platform | Status |
|----------|--------|
| macOS (Apple Silicon) | Full support |
| macOS (Intel) | Full support |
| Linux (x64) | Full support |
| Linux (arm64) | Full support |
| Windows (x64) | Limited support |

**Windows users**: Experimental support only. Process shutdown and file watching may not work reliably. Please report issues you encounter.

## DevContainer / Remote Development

The extension auto-detects Ollama on `host.docker.internal:11434` for Docker Desktop users (Mac/Windows).

**Linux Docker users**: You'll need to configure the endpoint manually:

1. Open Settings (`Cmd+,` / `Ctrl+,`)
2. Search for "maproom ollama"
3. Set `maproom.ollama.endpoint` to your host's IP (e.g., `http://172.17.0.1:11434`)

## Embedding Providers

### Ollama (Default)

Free, private, runs locally. Recommended for most users.

**Requirements**:
- Ollama installed and running
- ~4GB RAM for embedding model
- ~670MB disk space for `mxbai-embed-large` model

### OpenAI

Fast cloud-based embeddings. Requires API key and costs per request.

1. Run `Maproom: Setup` from Command Palette
2. Select OpenAI
3. Enter your API key from https://platform.openai.com/api-keys

### Google Vertex AI

Cloud-based alternative. Requires API key and Google Cloud setup.

1. Run `Maproom: Setup` from Command Palette
2. Select Google
3. Enter your API key

## Commands

| Command | Description |
|---------|-------------|
| `Maproom: Setup` | Change embedding provider or API keys |
| `Maproom: Show Output` | View detailed logs |
| `Maproom: Show Status` | Check database and process status |
| `Maproom: Restart Watchers` | Restart file monitoring |

## Status Bar

The status bar shows current state:

- **Starting...** - Services initializing
- **Indexing: N files** - Initial scan in progress
- **Watching: N files** - Ready and monitoring changes
- **Error** - Click to view details

## Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `maproom.database.sqlitePath` | Custom database path | `~/.maproom/maproom.db` |
| `maproom.ollama.endpoint` | Ollama API URL | `http://127.0.0.1:11434` |

## Troubleshooting

### "Ollama is not running"

```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama
ollama serve
```

### "Binary not found" or "Permission denied"

The extension should auto-fix permissions, but if needed:

```bash
chmod +x ~/.vscode/extensions/manifoldlogic.vscode-maproom-*/bin/*/maproom
```

### Extension seems stuck on "Starting..."

1. Open Output panel: `Maproom: Show Output`
2. Look for error messages
3. Check if Ollama is running and the model is pulled
4. Try `Maproom: Restart Watchers`

### Slow initial indexing

Normal for large codebases. The status bar shows progress. Subsequent file changes are indexed incrementally and much faster.

## Requirements

- **Git**: Required (workspace must be a git repository)
- **VSCode**: 1.85.0+
- **RAM**: 2GB minimum, 4GB recommended
- **Disk**: 500MB for extension + database

## Known Limitations

- Windows support is experimental
- Only git repositories are supported
- Binary files are not indexed
- Initial scan of very large repos (>10k files) takes time
- OpenAI/Google providers require internet connection

## Support

- **Issues**: https://github.com/danielbushman/crewchief/issues
- **Documentation**: https://github.com/danielbushman/crewchief

## License

MIT License

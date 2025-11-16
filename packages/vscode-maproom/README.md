# Maproom Semantic Search

> Semantic code search powered by Maproom - index and search your codebase by meaning, not just text.

Maproom brings AI-powered semantic search to Visual Studio Code. Index your workspace once, then search by concept instead of keywords. Find code by what it does, not just what it's called.

## Features

- **Semantic Search** - Find code by meaning using AI embeddings, not just text matching
- **Real-Time Indexing** - File watching keeps your index fresh as you code
- **Docker Integration** - Managed PostgreSQL, Ollama, and MCP services with zero manual setup
- **Provider Choice** - Use Ollama (local, free), OpenAI, or Google Gemini for embeddings
- **Crash Recovery** - Automatic restart with exponential backoff if processes crash
- **Secure Credentials** - API keys stored safely in VSCode SecretStorage
- **Status Tracking** - Real-time status bar showing indexed files and last update
- **Progress Notifications** - Clear feedback during initial scanning and indexing

## System Requirements

| Requirement | Minimum | Recommended |
|------------|---------|-------------|
| VSCode | 1.85.0+ | Latest stable |
| Docker Desktop | 24.0+ | Latest stable |
| RAM | 4GB | 8GB |
| Free Disk Space | 2GB | 5GB |

**Note**: Docker Desktop must be running before activating the extension.

## Platform Support

| Platform | Architecture | Status | Notes |
|----------|--------------|--------|-------|
| Linux | x64 | ✅ Supported | Full support |
| Linux | arm64 | ✅ Supported | Full support |
| macOS | arm64 (M1+) | ✅ Supported | Full support |
| macOS | x64 (Intel) | ✅ Supported | Full support |
| Windows | x64 | ⚠️ Experimental | File watching may be slower |

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
code --install-extension vscode-maproom-0.1.0.vsix
```

### From Source (Development)

```bash
git clone https://github.com/crewchief/vscode-maproom
cd vscode-maproom
pnpm install
pnpm compile
code --extensionDevelopmentPath=$(pwd)
```

## Getting Started

### First Launch

1. **Open a workspace** in VSCode
   - The extension activates automatically on startup
   - Docker services start in the background

2. **Complete setup wizard** (appears automatically)
   - Choose your embedding provider:
     - **Ollama** (Recommended) - Free, local, private
     - **OpenAI** - Requires API key, fast and accurate
     - **Google Gemini** - Requires API key

   ![Setup Wizard](docs/images/setup-wizard.png)
   *Screenshot pending - shows provider selection QuickPick*

3. **Wait for initial scan** (progress shown in notification)
   - Large workspaces (>10k files) may take 5-10 minutes
   - Progress updates every few seconds
   - Status bar shows file count as indexing progresses

   ![Progress Notification](docs/images/progress-notification.png)
   *Screenshot pending - shows indexing progress*

4. **Start searching!**
   - Status bar shows "Watching: X files" when ready
   - Search functionality available (integration coming soon)

### Provider Details

#### Ollama (Recommended)

**Pros**: Free, private, runs locally, no API keys
**Cons**: Requires 4GB RAM, slower on first run (model download)

**Setup**:
1. Install Ollama: https://ollama.ai/download
2. Run: `ollama pull nomic-embed-text`
3. Keep Ollama running while using the extension

The setup wizard auto-detects if Ollama is running on port 11434.

#### OpenAI

**Pros**: Fast, accurate, reliable
**Cons**: Requires API key, costs money per request

**Setup**:
1. Get API key: https://platform.openai.com/api-keys
2. Enter key when prompted (stored securely)
3. Uses `text-embedding-ada-002` model

#### Google Gemini

**Pros**: Fast, competitive pricing
**Cons**: Requires API key and Google Cloud setup

**Setup**:
1. Enable Vertex AI API: https://console.cloud.google.com/
2. Get API key: https://cloud.google.com/vertex-ai/docs/authentication
3. Enter key when prompted (stored securely)

## Usage

### Status Bar

The Maproom status bar item shows current state:

- **Starting...** - Docker services initializing
- **Indexing: 1,234 files** - Initial scan in progress
- **Watching: 5,678 files** (Updated: 2m ago) - Active and ready
- **Error** - Click to view details in output channel

![Status Bar](docs/images/status-bar.png)
*Screenshot pending - shows status bar in different states*

**Timestamp behavior**:
- Shows "just now" for updates within 1 minute
- Shows "Xm ago" for recent updates
- Shows "Xh ago" for older updates
- Shows "Xd ago" for very old updates

### Commands

Open Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`) and run:

| Command | Description | When to Use |
|---------|-------------|-------------|
| `Maproom: Setup` | Re-run setup wizard | Change embedding provider or update API key |
| `Maproom: Show Output` | Open Maproom output channel | View detailed logs, debug issues |
| `Maproom: Restart Watchers` | Restart file watching processes | If file watching stops or seems stuck |

### Settings

Currently configured via setup wizard. Future versions may add workspace settings.

**Internal state** (workspace state, not user-configurable):
- `maproom.provider` - Selected embedding provider
- API credentials stored in VSCode SecretStorage

## Troubleshooting

For detailed troubleshooting, see [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md).

### Quick Solutions

#### Docker services failed to start

**Error**: "Docker services failed to start" or "Health check timeout"

**Solutions**:
1. Ensure Docker Desktop is running
2. Check Docker has 4GB+ memory allocated (Settings → Resources)
3. Verify Docker works: `docker ps` in terminal
4. Try manually: `docker compose -f config/docker-compose.yml up`
5. Restart VSCode

#### Binary permission denied (Linux/macOS)

**Error**: "EACCES: permission denied" or "spawn EACCES"

**Solution**:
```bash
# Find extension directory
ls ~/.vscode/extensions/crewchief.vscode-maproom-*

# Make binary executable
chmod +x ~/.vscode/extensions/crewchief.vscode-maproom-*/bin/*/crewchief-maproom
```

#### Ollama not detected

**Error**: Setup wizard doesn't show Ollama as "Recommended"

**Solutions**:
1. Verify Ollama is running: `ollama list`
2. Check Ollama API: `curl http://localhost:11434`
3. If Ollama is on different port, select it anyway (port configurable in future)
4. Or choose OpenAI/Google instead

#### Invalid API credentials

**Error**: "Authentication failed" or "Invalid API key"

**Solutions**:
1. Re-run setup: `Maproom: Setup` command
2. Verify API key is correct (check for whitespace)
3. For OpenAI: https://platform.openai.com/api-keys
4. For Google: https://cloud.google.com/vertex-ai/docs/authentication

#### Process keeps crashing

**Error**: "Maproom watcher crashed after 5 restart attempts"

**Solutions**:
1. Check output channel: `Maproom: Show Output`
2. Look for error details in logs
3. Try restarting: `Maproom: Restart Watchers`
4. Check system resources (RAM, disk space)
5. Report issue with log excerpt (see TROUBLESHOOTING.md)

#### Slow performance

**Issue**: Indexing is very slow or never completes

**Solutions**:
1. Check Docker Desktop resource allocation (needs 4GB+ RAM)
2. Large repos (>10k files) can take 5-10 minutes initially
3. Subsequent updates are incremental (much faster)
4. Monitor progress in status bar
5. Check output channel for detailed progress

#### File watching not working

**Issue**: Changes not detected, index becomes stale

**Solutions**:
1. Run `Maproom: Restart Watchers` command
2. Check output channel for watcher errors
3. Windows users: May experience delays (experimental support)
4. Verify file is within workspace root
5. Check file isn't in `.gitignore` or binary file

## Known Limitations

- **Windows Support**: Experimental. File watching may be slower or miss some events.
- **Large Repositories**: Initial scan of >10,000 files can take 5-10 minutes.
- **Memory Usage**: Embedding requires 2-4GB RAM for large codebases.
- **Binary Files**: Only text files are indexed (images, PDFs, compiled binaries ignored).
- **Network Required**: OpenAI and Google providers require active internet connection.
- **Ollama Model**: Must download ~500MB model on first use (one-time).
- **Search UI**: Semantic search integration coming soon (extension indexes but search UI not yet implemented).

## Architecture

Maproom uses a multi-process architecture:

```
VSCode Extension (Node.js)
  ├── Docker Manager → Docker Compose services
  │   ├── PostgreSQL (pgvector) - Vector database
  │   ├── Ollama - Local embedding generation
  │   └── Maproom MCP - Indexing & search backend
  └── Process Orchestrator → File watchers
      ├── Initial scanner - One-time full scan
      └── File watcher - Real-time change detection
```

**Key components**:
- **Setup Wizard**: Provider selection and credential management
- **Status Bar**: Real-time status and file count
- **Crash Recovery**: Exponential backoff restart (5 attempts)
- **NDJSON Parser**: Processes file scan events with metadata
- **Secrets Manager**: Secure credential storage via VSCode API

**Technology stack**:
- TypeScript (ESM modules)
- Vitest for testing (71% coverage)
- Docker Compose for service orchestration
- PostgreSQL with pgvector extension
- Ollama for local embeddings

## Development

### Prerequisites

- Node.js 20+
- pnpm 8+
- Docker Desktop
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

### Testing

```bash
# All tests
pnpm test

# Specific test file
pnpm test src/docker/manager.test.ts

# Integration tests
pnpm test src/test/integration.test.ts

# Coverage report
pnpm test:coverage
```

**Current coverage**: 71% overall (270 tests passing)
- `src/config`: 96%
- `src/ui`: 94%
- `src/process`: 81%
- `src/utils`: 100%

### Package Extension

```bash
# Install vsce
npm install -g @vscode/vsce

# Package extension
vsce package

# Output: vscode-maproom-0.1.0.vsix
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

- **Bug Reports**: https://github.com/crewchief/vscode-maproom/issues
- **Feature Requests**: https://github.com/crewchief/vscode-maproom/issues
- **Documentation**: https://github.com/crewchief/vscode-maproom/blob/main/docs/
- **Troubleshooting**: See [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)

## Acknowledgements

Built with:
- [VSCode Extension API](https://code.visualstudio.com/api)
- [PostgreSQL](https://www.postgresql.org/) with [pgvector](https://github.com/pgvector/pgvector)
- [Ollama](https://ollama.ai/) for local embeddings
- [Maproom](https://github.com/crewchief/maproom) semantic search engine

---

**Status**: Beta (v0.1.0) - Actively developed, feedback welcome!

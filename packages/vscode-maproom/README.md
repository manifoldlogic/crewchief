# Maproom Semantic Search

> Semantic code search powered by Maproom - index and search your codebase by meaning, not just text.

Maproom brings AI-powered semantic search to Visual Studio Code. Index your workspace once, then search by concept instead of keywords. Find code by what it does, not just what it's called.

## Quick Start (SQLite - Recommended)

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
- **Zero-Config SQLite** - Works immediately with existing `~/.maproom/maproom.db` (no Docker needed!)
- **Real-Time Indexing** - File watching keeps your index fresh as you code
- **Optional PostgreSQL** - Team sharing with Docker-managed PostgreSQL and pgvector
- **Provider Choice** - Use Ollama (local, free), OpenAI, or Google Gemini for embeddings
- **Crash Recovery** - Automatic restart with exponential backoff if processes crash
- **Secure Credentials** - API keys stored safely in VSCode SecretStorage
- **Status Tracking** - Real-time status bar showing indexed files and database mode

## System Requirements

### SQLite Mode (Default)

| Requirement | Minimum | Recommended |
|------------|---------|-------------|
| VSCode | 1.85.0+ | Latest stable |
| RAM | 2GB | 4GB |
| Free Disk Space | 500MB | 2GB |

**No Docker required!** Just install the extension and create an index.

### PostgreSQL Mode (Advanced)

| Requirement | Minimum | Recommended |
|------------|---------|-------------|
| VSCode | 1.85.0+ | Latest stable |
| Docker Desktop | 24.0+ | Latest stable |
| RAM | 4GB | 8GB |
| Free Disk Space | 2GB | 5GB |

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
| `maproom.database.provider` | Database backend: `sqlite` or `postgres` | `sqlite` |
| `maproom.database.sqlitePath` | Custom SQLite path (empty = `~/.maproom/maproom.db`) | `""` |
| `maproom.database.host` | PostgreSQL host (postgres mode only) | `localhost` |
| `maproom.database.port` | PostgreSQL port (postgres mode only) | `5432` |
| `maproom.database.user` | PostgreSQL username (postgres mode only) | `maproom` |
| `maproom.database.password` | PostgreSQL password (postgres mode only) | `maproom` |
| `maproom.database.name` | PostgreSQL database name (postgres mode only) | `maproom` |

## Commands

Open Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`) and run:

| Command | Description | When to Use |
|---------|-------------|-------------|
| `Maproom: Setup` | Re-run setup wizard | Change embedding provider or update API key |
| `Maproom: Show Output` | Open Maproom output channel | View detailed logs, debug issues |
| `Maproom: Restart Watchers` | Restart file watching processes | If file watching stops or seems stuck |
| `Maproom: Show Status` | Show current Maproom status | Check database mode and connection |

## Status Bar

The Maproom status bar item shows current state:

- **Starting...** - Services initializing
- **Indexing: 1,234 files** - Scan in progress
- **Watching: 5,678 files** - Active and ready
- **Error** - Click to view details in output channel

The tooltip shows additional information:
- Current database mode (SQLite or PostgreSQL)
- Database path (for SQLite mode)
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

## Advanced: PostgreSQL Setup (Team Sharing)

For team environments where you want a shared code index, use PostgreSQL mode:

### 1. Change Database Provider

Open VS Code settings and set:
```json
{
  "maproom.database.provider": "postgres"
}
```

### 2. Start Docker Containers

```bash
cd ~/.vscode/extensions/manifoldlogic.vscode-maproom-*/config
docker compose up -d
```

### 3. Configure Connection (if needed)

Default settings work out-of-the-box with the bundled Docker Compose file. For custom PostgreSQL instances:

```json
{
  "maproom.database.provider": "postgres",
  "maproom.database.host": "your-postgres-host",
  "maproom.database.port": 5432,
  "maproom.database.user": "maproom",
  "maproom.database.password": "your-password",
  "maproom.database.name": "maproom"
}
```

### 4. Reload VSCode

Run "Developer: Reload Window" command to apply changes.

## Migrating Between Backends

### PostgreSQL to SQLite

If you've been using PostgreSQL and want to switch to SQLite:

1. **Create SQLite index**:
   ```bash
   crewchief-maproom scan /path/to/your/repo
   ```

2. **Update settings**:
   Open VS Code settings and change:
   ```json
   {
     "maproom.database.provider": "sqlite"
   }
   ```

3. **Reload window**:
   Run "Developer: Reload Window" command

4. **Stop Docker (optional)**:
   If you no longer need PostgreSQL:
   ```bash
   docker compose down
   ```

### SQLite to PostgreSQL

To migrate from SQLite to PostgreSQL for team sharing:

1. **Start PostgreSQL**:
   ```bash
   cd ~/.vscode/extensions/manifoldlogic.vscode-maproom-*/config
   docker compose up -d
   ```

2. **Update settings**:
   ```json
   {
     "maproom.database.provider": "postgres"
   }
   ```

3. **Re-index your codebase**:
   The extension will prompt to re-scan your workspace.

## Troubleshooting

### SQLite Issues

#### "SQLite database not found"

**Error**: `SQLite database not found at: /Users/you/.maproom/maproom.db`

**Cause**: No index has been created yet.

**Solution**: Create an index using the CLI:
```bash
crewchief-maproom scan /path/to/your/repo
```

#### Custom SQLite Path Not Working

**Cause**: Path may contain unsupported characters or doesn't exist.

**Solutions**:
1. Ensure the path exists and is readable
2. Use absolute paths (not relative)
3. Tilde (`~`) is supported for home directory
4. Check the Output channel for detailed error messages

### PostgreSQL Issues

#### "Cannot connect to PostgreSQL"

**Error**: `Cannot connect to PostgreSQL at localhost:5432`

**Cause**: PostgreSQL container not running or misconfigured.

**Solutions**:
1. Ensure Docker Desktop is running
2. Start containers: `docker compose up -d`
3. Verify containers are healthy: `docker ps`
4. Check settings match your PostgreSQL configuration

#### "Maproom requires Docker Desktop to be running"

**Cause**: Docker Desktop is not installed or not running (PostgreSQL mode only).

**Solutions**:
1. Install Docker Desktop from https://www.docker.com/products/docker-desktop
2. Start Docker Desktop and wait for it to fully initialize
3. Verify Docker is running: `docker ps`
4. Reload VSCode window
5. **Or switch to SQLite mode** - change `maproom.database.provider` to `sqlite`

#### Port Conflicts (5432, 3000)

**Cause**: Another service is using required ports.

**Solutions**:
1. Check what's using the ports:
   - Linux/macOS: `lsof -i :5432`
   - Windows: `netstat -ano | findstr :5432`
2. Stop the conflicting service
3. Or modify port mappings in the Docker Compose file

### General Issues

#### Binary Permission Denied (Linux/macOS)

**Error**: `EACCES: permission denied`

**Solution**:
```bash
chmod +x ~/.vscode/extensions/manifoldlogic.vscode-maproom-*/bin/*/crewchief-maproom
```

#### Ollama Not Detected

**Solutions**:
1. Verify Ollama is running: `ollama list`
2. Check Ollama API: `curl http://localhost:11434`
3. Or choose OpenAI/Google instead

#### Process Keeps Crashing

**Solutions**:
1. Check output channel: `Maproom: Show Output`
2. Try restarting: `Maproom: Restart Watchers`
3. Check system resources (RAM, disk space)

#### Slow Performance

**Solutions**:
1. Large repos (>10k files) can take 5-10 minutes initially
2. Subsequent updates are incremental (much faster)
3. For PostgreSQL: ensure Docker has 4GB+ RAM allocated

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
  ├── Database Checker → SQLite file or PostgreSQL connection
  ├── Process Orchestrator → File watchers
  │   ├── Initial scanner - One-time full scan
  │   └── File watcher - Real-time change detection
  └── (PostgreSQL mode only)
      └── Docker Manager → Docker Compose services
          ├── PostgreSQL (pgvector) - Vector database
          └── Ollama - Local embedding generation
```

**Key components**:
- **Database Checker**: Unified SQLite/PostgreSQL configuration
- **Setup Wizard**: Provider selection and credential management
- **Status Bar**: Real-time status, file count, and database mode
- **Crash Recovery**: Exponential backoff restart (5 attempts)

## Development

### Prerequisites

- Node.js 20+
- pnpm 8+
- Docker Desktop (only for PostgreSQL testing)
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
- [PostgreSQL](https://www.postgresql.org/) with [pgvector](https://github.com/pgvector/pgvector) for team sharing
- [Ollama](https://ollama.ai/) for local embeddings
- [Maproom](https://github.com/danielbushman/crewchief) semantic search engine

---

**Status**: Beta (v0.3.3) - Actively developed, feedback welcome!

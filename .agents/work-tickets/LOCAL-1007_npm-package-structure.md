# Ticket: LOCAL-1007: Create npm package structure for @crewchief/maproom-mcp

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Create the npm package structure that will wrap the Docker Compose stack and provide zero-configuration user experience via npx. This package will contain embedded Docker configuration files and serve as the distribution mechanism for the LOCAL project.

## Background
The LOCAL project aims to provide a fully containerized Maproom MCP service with local LLM embeddings that requires zero configuration from users. While the Docker Compose stack provides the infrastructure (created in LOCAL-1003), users need a simple way to start the stack without manually managing docker-compose.yml files or configuration.

The @crewchief/maproom-mcp npm package solves this by:
- Providing a single command entry point: `npx @crewchief/maproom-mcp`
- Embedding all necessary configuration files (docker-compose.yml, init.sql, postgresql.conf)
- Abstracting Docker complexity from end users
- Ensuring cross-platform compatibility via npx

This ticket creates the package structure and configuration. The CLI wrapper implementation will be handled in LOCAL-1008.

## Acceptance Criteria
- [x] Directory structure created at `/workspace/packages/maproom-mcp/`
- [x] package.json created with correct metadata:
  - name: `@crewchief/maproom-mcp`
  - bin entry: `maproom-mcp` → `./bin/cli.js`
  - keywords: mcp, embeddings, ollama, semantic-search, code-search
  - MIT license
  - repository link configured
- [x] `config/` directory created with embedded files:
  - `docker-compose.yml` (copied from LOCAL-1003 output)
  - `init.sql` (copied from LOCAL-1002 output)
  - `postgresql.conf` (created with performance tuning settings)
  - `Dockerfile.maproom` (symlinked or copied from LOCAL-1001 output)
- [x] `bin/` directory created (empty, ready for LOCAL-1008)
- [x] README.md created with:
  - Installation instructions (`npx @crewchief/maproom-mcp`)
  - Quick start guide
  - Prerequisites (Docker Desktop or Docker with Compose plugin)
  - Basic troubleshooting
- [x] `.npmignore` created to exclude development files
- [x] Package can be installed locally with `npm link`
- [x] Running `npm link` and `maproom-mcp --help` shows basic usage (will be placeholder until LOCAL-1008)

## Technical Requirements

### Directory Structure
```
packages/maproom-mcp/
├── package.json
├── README.md
├── .npmignore
├── bin/
│   └── cli.js (placeholder for LOCAL-1008)
├── config/
│   ├── docker-compose.yml
│   ├── init.sql
│   ├── postgresql.conf
│   └── Dockerfile.maproom
└── LICENSE (MIT)
```

### package.json Requirements
```json
{
  "name": "@crewchief/maproom-mcp",
  "version": "1.0.0",
  "description": "Maproom MCP server with local LLM embeddings - zero configuration required",
  "bin": {
    "maproom-mcp": "./bin/cli.js"
  },
  "scripts": {
    "test": "node bin/cli.js --test",
    "dev": "node bin/cli.js"
  },
  "keywords": ["mcp", "embeddings", "ollama", "semantic-search", "code-search"],
  "author": "CrewChief",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/your-org/crewchief.git",
    "directory": "packages/maproom-mcp"
  },
  "engines": {
    "node": ">=18.0.0"
  }
}
```

### postgresql.conf Content
```conf
# postgresql.conf overrides for maproom
max_connections = 100
shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1
effective_io_concurrency = 200
work_mem = 4MB
min_wal_size = 1GB
max_wal_size = 4GB
```

### .npmignore Requirements
Exclude:
- `*.test.js`
- `.github/`
- `node_modules/`
- `.DS_Store`
- `*.log`
- Development files not needed for distribution

### README.md Requirements
Must include:
- Project description
- Prerequisites (Docker Desktop 4.x+ or Docker with Compose v2 plugin)
- Installation: `npx @crewchief/maproom-mcp`
- Quick start example
- Configuration reference (environment variables)
- Basic troubleshooting (Docker not running, port conflicts)
- Link to full documentation

## Implementation Notes

### File Sources
- **docker-compose.yml**: Copy from LOCAL-1003 output location (likely `crates/maproom/docker/docker-compose.yml` or similar)
- **init.sql**: Copy from LOCAL-1002 output location (likely `crates/maproom/migrations/init.sql` or similar)
- **Dockerfile.maproom**: Reference LOCAL-1001 output (likely `crates/maproom/Dockerfile`)

### CLI Placeholder (bin/cli.js)
Create a simple placeholder for LOCAL-1008:
```javascript
#!/usr/bin/env node
console.log('Maproom MCP - CLI wrapper will be implemented in LOCAL-1008');
console.log('Usage: maproom-mcp [command]');
console.log('Commands:');
console.log('  start   - Start the Maproom MCP stack');
console.log('  stop    - Stop the Maproom MCP stack');
console.log('  status  - Check service status');
console.log('  logs    - View service logs');
process.exit(0);
```

Make it executable: `chmod +x bin/cli.js`

### Testing Locally
After creating the package:
```bash
cd /workspace/packages/maproom-mcp
npm link
maproom-mcp --help  # Should show placeholder message
npm unlink -g @crewchief/maproom-mcp  # Clean up
```

### PostgreSQL Configuration Rationale
The postgresql.conf settings are tuned for:
- **Moderate workload**: 100 connections, 256MB shared buffers
- **Vector operations**: Higher effective_io_concurrency (200) for parallel index scans
- **Write-heavy indexing**: Larger WAL sizes (1-4GB) to reduce checkpoints
- **Search performance**: Lower random_page_cost (1.1) assumes SSD storage

These settings are suitable for development and moderate production use. For large-scale deployments, users can mount custom postgresql.conf.

## Dependencies
- **LOCAL-1003**: Requires docker-compose.yml to be created (BLOCKED until this is complete)
- **LOCAL-1002**: Requires init.sql to be created (BLOCKED until this is complete)
- **LOCAL-1001**: Requires Dockerfile.maproom to reference (BLOCKED until this is complete)

**Note**: If dependencies are not yet complete, create the package structure with placeholder files and update them when dependencies are ready.

## Risk Assessment
- **Risk**: Dependencies (LOCAL-1001, LOCAL-1002, LOCAL-1003) may not be complete yet
  - **Mitigation**: Create package structure with placeholder files; update when dependencies deliver. Document expected file locations in README.

- **Risk**: npm package name @crewchief/maproom-mcp may not be available on npm registry
  - **Mitigation**: Check availability with `npm search @crewchief/maproom-mcp` before publishing. Consider alternative names if needed.

- **Risk**: Users may not have Docker Compose v2 plugin installed
  - **Mitigation**: Clear prerequisites in README; CLI wrapper (LOCAL-1008) will detect and provide helpful error messages.

- **Risk**: Package size may be large due to embedded binaries
  - **Mitigation**: Use .npmignore to exclude unnecessary files; docker-compose.yml references Docker images (not embedded binaries).

## Files/Packages Affected
- **New directory**: `/workspace/packages/maproom-mcp/`
- **New files**:
  - `/workspace/packages/maproom-mcp/package.json`
  - `/workspace/packages/maproom-mcp/README.md`
  - `/workspace/packages/maproom-mcp/.npmignore`
  - `/workspace/packages/maproom-mcp/LICENSE`
  - `/workspace/packages/maproom-mcp/bin/cli.js` (placeholder)
  - `/workspace/packages/maproom-mcp/config/docker-compose.yml`
  - `/workspace/packages/maproom-mcp/config/init.sql`
  - `/workspace/packages/maproom-mcp/config/postgresql.conf`
  - `/workspace/packages/maproom-mcp/config/Dockerfile.maproom`

## Implementation Notes (Completed)

Successfully created npm package structure at `/workspace/packages/maproom-mcp/` with all required files:

**Created Files:**
- `package.json` - Package metadata with bin entry pointing to `./bin/cli.js`
- `bin/cli.js` - Executable placeholder CLI script (chmod +x applied)
- `config/docker-compose.yml` - Copied from `/workspace/config/docker-compose.yml` (verified identical)
- `config/init.sql` - Copied from `/workspace/config/init.sql` (verified identical)
- `config/Dockerfile.maproom` - Copied from `/workspace/Dockerfile.maproom` (verified identical)
- `config/postgresql.conf` - Created with performance tuning settings for vector operations
- `README.md` - Comprehensive documentation with installation, quick start, troubleshooting
- `.npmignore` - Excludes test files, development files, logs, editor directories
- `LICENSE` - MIT license

**Verification Tests:**
- `npm link` succeeded - package can be installed locally
- `maproom-mcp --help` executed successfully and showed placeholder CLI output
- `npm unlink -g @crewchief/maproom-mcp` succeeded - cleanup complete
- All source files verified to match destination files (diff passed)

**Ready for:**
- LOCAL-1008: CLI wrapper implementation (bin/cli.js is ready to be replaced)
- Package publishing to npm registry (structure is complete)

All acceptance criteria met. Package is ready for CLI wrapper implementation in next ticket.

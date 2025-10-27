# Ticket: LOCAL-2503: Update npm Package Structure for Publication

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the npm package structure at `/workspace/packages/maproom-mcp/` to ensure all required files are properly packaged for npm publication, including the CLI wrapper, Docker configuration files, initialization scripts, and documentation. This enables the complete `npx -y @crewchief/maproom-mcp` workflow.

## Background
Phase 2.5 completes the bridge between containerized infrastructure and npm distribution. The previous tickets created:
- LOCAL-2501: Containerized TypeScript MCP server (Dockerfile.mcp-server)
- LOCAL-2502: CLI wrapper for Docker orchestration and stdio proxy (bin/cli.js)

This final ticket ensures the npm package is correctly structured for publication so that when users run `npx -y @crewchief/maproom-mcp`, they receive a complete, working package with all necessary files.

The npm package must include:
1. Compiled CLI wrapper (`bin/cli.js`)
2. Docker configuration files (docker-compose.yml, Dockerfiles)
3. Database initialization scripts (init.sql)
4. TypeScript MCP server source (for containerization)
5. Documentation (README.md)
6. Metadata (package.json, LICENSE)

## Acceptance Criteria
- [ ] `package.json` updated with all required metadata and dependencies
- [ ] `package.json` has correct `bin` entry: `"maproom-mcp": "./bin/cli.js"`
- [ ] `package.json` has `files` array listing all files to include in npm package
- [ ] `.npmignore` created to exclude unnecessary files (tests, dev configs, etc.)
- [ ] `bin/cli.js` is executable (`chmod +x`) and has proper shebang
- [ ] Docker configuration directory created: `config/`
  - `config/docker-compose.yml`
  - `config/Dockerfile.mcp-server`
  - `config/init.sql` (PostgreSQL schema)
  - `config/.env.example` (optional)
- [ ] TypeScript source included for containerization: `src/`
- [ ] `package.json` scripts added: `build`, `test`, `prepublishOnly`
- [ ] All dependencies correctly specified (runtime vs. dev)
- [ ] Package builds successfully: `npm pack` creates tarball
- [ ] Tarball size is reasonable (< 500KB, excluding node_modules)
- [ ] Test installation: `npx ./crewchief-maproom-mcp-<version>.tgz` works
- [ ] README.md includes usage instructions (created in LOCAL-3002)
- [ ] LICENSE file included
- [ ] Repository metadata correct in package.json

## Technical Requirements

### package.json Updates

```json
{
  "name": "@crewchief/maproom-mcp",
  "version": "1.0.0",
  "description": "Maproom MCP server with local LLM embeddings - zero configuration required",
  "main": "dist/index.js",
  "bin": {
    "maproom-mcp": "./bin/cli.js"
  },
  "files": [
    "bin/",
    "config/",
    "dist/",
    "src/",
    "README.md",
    "LICENSE"
  ],
  "scripts": {
    "build": "tsc",
    "test": "node bin/cli.js --test || echo 'Test mode not yet implemented'",
    "prepublishOnly": "npm run build",
    "dev": "node bin/cli.js"
  },
  "keywords": [
    "mcp",
    "embeddings",
    "ollama",
    "semantic-search",
    "code-search",
    "local-llm",
    "docker",
    "postgresql",
    "pgvector"
  ],
  "author": "CrewChief",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/your-org/crewchief.git",
    "directory": "packages/maproom-mcp"
  },
  "bugs": {
    "url": "https://github.com/your-org/crewchief/issues"
  },
  "homepage": "https://github.com/your-org/crewchief/tree/main/packages/maproom-mcp#readme",
  "engines": {
    "node": ">=18.0.0"
  },
  "dependencies": {
    "pg": "^8.11.0",
    "pino": "^8.19.0"
  },
  "devDependencies": {
    "typescript": "^5.3.0",
    "@types/node": "^20.0.0",
    "@types/pg": "^8.11.0"
  }
}
```

### Files Array
The `files` array in package.json determines what gets published to npm. Include:

```json
"files": [
  "bin/cli.js",           // CLI wrapper
  "config/",              // Docker configs (compose, Dockerfiles, init.sql)
  "dist/",                // Compiled TypeScript (MCP server)
  "src/",                 // TypeScript source (for docker build)
  "tsconfig.json",        // TypeScript config (for docker build)
  "README.md",            // Documentation
  "LICENSE"               // License file
]
```

**Why include `src/`?** The Dockerfile.mcp-server needs TypeScript source to compile the MCP server inside the container. The `dist/` is for local use, but the container builds from source.

### .npmignore File
Create `.npmignore` to exclude files not needed in the published package:

```
# Development
*.log
*.log.*
npm-debug.log*
.DS_Store
.vscode/
.idea/

# Tests
test/
tests/
*.test.ts
*.test.js
*.spec.ts
*.spec.js

# Build artifacts
*.tgz
coverage/
.nyc_output/

# Git
.git/
.gitignore

# CI/CD
.github/
.gitlab-ci.yml
.travis.yml

# Documentation (only if excessive)
docs/development/
docs/internal/

# Temporary files
tmp/
temp/
*.tmp
```

### Directory Structure
After this ticket, the package should have this structure:

```
packages/maproom-mcp/
├── bin/
│   └── cli.js              # CLI wrapper (executable)
├── config/
│   ├── docker-compose.yml  # Docker Compose configuration
│   ├── Dockerfile.mcp-server  # MCP server Dockerfile
│   └── init.sql            # PostgreSQL schema
├── src/
│   └── index.ts            # TypeScript MCP server source
├── dist/                   # Compiled JavaScript (from tsc)
│   └── index.js
├── tsconfig.json           # TypeScript configuration
├── package.json            # npm package metadata
├── .npmignore              # Files to exclude from npm
├── README.md               # Usage documentation
└── LICENSE                 # MIT or other license
```

### Scripts to Add

#### build
```json
"build": "tsc"
```
Compiles TypeScript source (`src/index.ts`) to JavaScript (`dist/index.js`).

#### prepublishOnly
```json
"prepublishOnly": "npm run build"
```
Automatically builds before publishing to npm (ensures dist/ is up-to-date).

#### test
```json
"test": "node bin/cli.js --test"
```
Test script that validates the CLI wrapper works (may be a simple smoke test).

#### dev
```json
"dev": "node bin/cli.js"
```
Run the CLI wrapper locally for development/testing.

### Dependencies vs. devDependencies

**Runtime dependencies** (required when package runs):
```json
"dependencies": {
  "pg": "^8.11.0",      // PostgreSQL client (used by MCP server)
  "pino": "^8.19.0"     // Logger (used by MCP server)
}
```

**Development dependencies** (only needed for building):
```json
"devDependencies": {
  "typescript": "^5.3.0",
  "@types/node": "^20.0.0",
  "@types/pg": "^8.11.0"
}
```

**Note**: The CLI wrapper (`bin/cli.js`) should be plain JavaScript (no compilation needed) or compiled before packaging. If CLI is TypeScript, add build step.

### Configuration Files to Package

#### config/docker-compose.yml
Copy from `/workspace/docker-compose.yml` (or create specific version):

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    # ... (full configuration)

  ollama:
    image: ollama/ollama:latest
    # ... (full configuration)

  maproom-mcp:
    build:
      context: .
      dockerfile: config/Dockerfile.mcp-server
    # ... (full configuration)
```

**Note**: The `dockerfile` path is relative to the context where `docker compose` runs (i.e., `~/.maproom-mcp/`). Adjust paths accordingly.

#### config/Dockerfile.mcp-server
Copy from `/workspace/Dockerfile.mcp-server` (created in LOCAL-2501).

#### config/init.sql
Copy PostgreSQL schema from `/workspace/init.sql` or extract from LOCAL-1002.

### Testing the Package

#### Local Pack and Install
```bash
cd /workspace/packages/maproom-mcp

# Build the package
npm run build

# Create tarball
npm pack
# Output: crewchief-maproom-mcp-1.0.0.tgz

# Test installation
npx ./crewchief-maproom-mcp-1.0.0.tgz
# Should start Docker services and MCP server
```

#### Verify Package Contents
```bash
tar -tzf crewchief-maproom-mcp-1.0.0.tgz
```

Should include:
```
package/package.json
package/bin/cli.js
package/config/docker-compose.yml
package/config/Dockerfile.mcp-server
package/config/init.sql
package/dist/index.js
package/src/index.ts
package/tsconfig.json
package/README.md
package/LICENSE
```

### License File
If not present, create `LICENSE` file (MIT recommended):

```
MIT License

Copyright (c) 2025 CrewChief

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### README.md Placeholder
If LOCAL-3002 (README creation) is not yet complete, create minimal placeholder:

```markdown
# @crewchief/maproom-mcp

Maproom MCP server with local LLM embeddings - zero configuration required.

## Quick Start

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
```

Full documentation coming soon.
```

The complete README will be added in LOCAL-3002.

## Implementation Notes

### Relative Paths in docker-compose.yml
When the CLI copies `docker-compose.yml` to `~/.maproom-mcp/`, paths must be relative to that directory:

```yaml
maproom-mcp:
  build:
    context: .
    dockerfile: config/Dockerfile.mcp-server  # Relative to ~/.maproom-mcp/
```

The CLI should copy files maintaining this structure:
```
~/.maproom-mcp/
├── docker-compose.yml
└── config/
    ├── Dockerfile.mcp-server
    └── init.sql
```

**Alternative**: Embed Dockerfile as base64 string in CLI and write on first run (avoids file copying complexity).

### Package Size Optimization
Target: < 500KB (excluding node_modules)

Techniques:
- Exclude unnecessary files via `.npmignore`
- Don't include test files
- Don't include development documentation
- Don't include source maps (unless needed)

Check package size:
```bash
npm pack --dry-run
# Shows what would be included

du -sh crewchief-maproom-mcp-1.0.0.tgz
# Shows actual tarball size
```

### Executable Permissions
The `bin/cli.js` must be executable. Set before packaging:

```bash
chmod +x bin/cli.js
git update-index --chmod=+x bin/cli.js  # Persist in git
```

npm automatically sets execute permissions for files in `bin/`.

### TypeScript Configuration
The `tsconfig.json` should output to `dist/`:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "node",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": false
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "test", "tests"]
}
```

### Publishing Checklist (for LOCAL-3008)
This ticket prepares the package; LOCAL-3008 will handle actual publication. Pre-publish checklist:

- [ ] `npm run build` succeeds
- [ ] `npm pack` creates tarball
- [ ] `npx ./crewchief-maproom-mcp-*.tgz` works locally
- [ ] All files in `files` array are present in tarball
- [ ] No sensitive files included (secrets, .env, etc.)
- [ ] README.md is complete and accurate
- [ ] Version number follows semver
- [ ] Repository URL is correct
- [ ] License is correct

## Dependencies
- **Blocked by**: LOCAL-2501 (containerize MCP server) - needs Dockerfile.mcp-server
- **Blocked by**: LOCAL-2502 (CLI wrapper) - needs bin/cli.js
- **Blocks**: LOCAL-3001 (test npx flow)
- **Blocks**: LOCAL-3008 (npm publish test release)
- **Related**: LOCAL-3002 (README documentation) - can happen in parallel

## Risk Assessment

### Risk: Missing files in npm package
- **Impact**: High - package won't work when installed
- **Mitigation**: Test with `npm pack` and inspect tarball contents
- **Mitigation**: Verify with `npx ./package.tgz` before publishing

### Risk: File paths incorrect after copying to ~/.maproom-mcp
- **Impact**: High - Docker Compose will fail
- **Mitigation**: Test complete flow with local package installation
- **Mitigation**: Use relative paths consistently in docker-compose.yml

### Risk: Executable permissions lost
- **Impact**: Medium - CLI won't run
- **Mitigation**: Set `chmod +x` and commit to git with `git update-index --chmod=+x`
- **Mitigation**: npm handles bin/ permissions automatically

### Risk: Package size too large
- **Impact**: Low - slower downloads, more bandwidth
- **Mitigation**: Exclude unnecessary files via .npmignore
- **Mitigation**: Monitor tarball size with `npm pack --dry-run`

### Risk: Missing dependencies
- **Impact**: High - package crashes on install/run
- **Mitigation**: Test with clean node_modules: `rm -rf node_modules && npm install`
- **Mitigation**: Run CLI wrapper after fresh install

### Risk: TypeScript compilation errors on publish
- **Impact**: High - `prepublishOnly` fails, blocking publish
- **Mitigation**: Run `npm run build` manually before publish
- **Mitigation**: Fix TypeScript errors in CI before merging

### Risk: Incorrect repository metadata
- **Impact**: Low - confusing for users, broken links
- **Mitigation**: Verify GitHub URLs are correct
- **Mitigation**: Check package.json against published packages

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/package.json` (updated)
- `/workspace/packages/maproom-mcp/.npmignore` (new file)
- `/workspace/packages/maproom-mcp/LICENSE` (new file, if missing)
- `/workspace/packages/maproom-mcp/README.md` (updated, basic version)
- `/workspace/packages/maproom-mcp/config/` (new directory)
  - `docker-compose.yml` (copied from workspace root)
  - `Dockerfile.mcp-server` (copied from LOCAL-2501)
  - `init.sql` (copied from LOCAL-1002)
- `/workspace/packages/maproom-mcp/tsconfig.json` (verify/update)
- `/workspace/packages/maproom-mcp/bin/cli.js` (verify executable)

## Success Metrics
After implementation:
1. `npm run build` compiles TypeScript successfully
2. `npm pack` creates tarball with all required files
3. Tarball size < 500KB
4. `tar -tzf <tarball>` shows correct file list
5. `npx ./<tarball>` starts Docker services successfully
6. CLI wrapper works with packaged docker-compose.yml
7. All acceptance criteria met
8. Ready for test publication (LOCAL-3008)

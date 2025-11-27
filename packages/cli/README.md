# CrewChief CLI

A powerful command-line tool for git worktree management, semantic code search, and AI agent orchestration.

## Installation

```bash
# Check compatibility before installing
npx @crewchief/cli doctor

# Install globally
npm install -g @crewchief/cli

# Now use directly
crewchief --help
```

(Also works with yarn, pnpm, bun, and other npm-compatible package managers)

## Requirements

- **Node.js >= 18**
- **Git** (for worktree management)
- **PostgreSQL** (for semantic search features - see setup below)
- **macOS with [iTerm2](https://iterm2.com/downloads.html)** (for agent orchestration features)
- **CLI agent tools** (`claude`, `gemini`, etc.) must be installed for agent orchestration

## Quick Start

### Basic Worktree Management

```bash
# Create and switch to a new worktree
crewchief worktree create feature-branch

# List all worktrees
crewchief worktree list

# Switch to an existing worktree (creates if needed)
crewchief worktree use feature-branch

# Merge worktree changes back to source branch
crewchief worktree merge feature-branch

# Clean up worktrees
crewchief worktree clean --all
```

### Semantic Code Search

#### Recommended: Using Maproom MCP

For the best experience with AI assistants like Claude and Cursor, use the Maproom MCP server. This allows AI assistants to search your indexed codebase directly.

See the [maproom-mcp package](https://www.npmjs.com/package/maproom-mcp) for installation and setup instructions.

#### Direct CLI Usage (Requires PostgreSQL)

First, set up your database connection:

```bash
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"
```

Then initialize and use semantic search:

```bash
# Initialize database
crewchief maproom db migrate

# Index your codebase
crewchief maproom scan

# Search semantically
crewchief maproom search "authentication flow"

# Watch for changes and auto-index
crewchief maproom watch
```

### AI Agent Orchestration

Agent orchestration supports two modes: **iTerm2** (default) for visual terminal management on macOS, and **Headless** for non-interactive or server environments.

#### iTerm2 Mode (Default)

Requires iTerm2 on macOS. Agents run in dedicated terminal panes.

```bash
# Spawn AI agents with dedicated worktrees
crewchief spawn claude "implement-auth"
crewchief spawn gemini "code-review"

# Spawn multiple agents at once
crewchief spawn claude,gemini "fix-bug"

# List running agents
crewchief agent list

# Send messages to agents
crewchief agent message implement-auth__claude "Add OAuth support"

# Send message to all agents on a task
crewchief agent message implement-auth --all "Update approach"
```

#### Headless Mode

For non-iTerm2 environments (servers, CI/CD, other terminals). Agents run as background processes.

```bash
# Spawn agents in headless mode
crewchief spawn mock-agent "task-name" --headless

# Spawn multiple agents in headless mode
crewchief spawn claude,gemini "fix-bug" --headless

# Headless agents communicate via stdin pipe
# The process stays alive to stream logs and manage child processes
```

**Note:** In headless mode, agent panes are logical processes rather than terminal panes. Messages are sent via stdin pipe to the agent process.

## Database Setup

Semantic code search requires PostgreSQL with the pgvector extension. Choose one of the following setup options:

### Option 1: Using Docker Compose (Recommended)

The easiest way is to use the Docker setup from the maproom-mcp package:

```bash
# Clone or navigate to the maproom-mcp directory
cd packages/maproom-mcp

# Start PostgreSQL with pgvector
docker compose up -d postgres

# Verify database is running
docker compose ps

# Set connection string
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5433/maproom"
```

**Note:** The Docker setup maps port 5433 (not 5432) to avoid conflicts with local PostgreSQL installations.

**Connection String Format:**

```
postgresql://[user]:[password]@[host]:[port]/[database]
```

### Option 2: Local PostgreSQL Installation

```bash
# macOS with Homebrew
brew install postgresql@14 pgvector
brew services start postgresql@14

# Create database
createdb maproom

# Install pgvector extension
psql maproom -c "CREATE EXTENSION IF NOT EXISTS vector;"

# Set connection string
export MAPROOM_DATABASE_URL="postgresql://localhost:5432/maproom"
```

### Option 3: Cloud Database

Use any PostgreSQL provider that supports pgvector:

- **Supabase**: Includes pgvector by default
- **Neon**: Contact support to enable pgvector
- **AWS RDS**: Enable pgvector extension manually

Set the connection string they provide:

```bash
export MAPROOM_DATABASE_URL="postgresql://user:password@your-db-host.com:5432/database"
```

### Environment Variable Fallback Hierarchy

The system checks environment variables in this order:

1. `MAPROOM_DATABASE_URL` **(recommended)** - Current standard
2. Component-specific (e.g., `MAPROOM_DB_HOST`, `MAPROOM_DB_PORT`)
3. `PG_DATABASE_URL` - Legacy support (deprecated)
4. `DATABASE_URL` - Generic fallback

**Best Practice:** Always use `MAPROOM_DATABASE_URL` for new configurations.

## Embedding Provider Setup

Semantic search uses embeddings to understand code semantically. Choose one provider:

### OpenAI (Recommended for Production)

Fast, high-quality embeddings with minimal setup:

```bash
# Set provider and API key
export MAPROOM_EMBEDDING_PROVIDER=openai
export OPENAI_API_KEY=sk-your-api-key-here

# Optional: specify model (default: text-embedding-3-small)
export MAPROOM_EMBEDDING_MODEL=text-embedding-3-small

# Index your codebase
crewchief maproom scan
```

**Cost:** ~$0.02 per 1GB of code indexed

### Google Vertex AI

Enterprise-grade embeddings with Google Cloud integration:

```bash
# Set provider
export MAPROOM_EMBEDDING_PROVIDER=google

# Configure Google Cloud credentials
export GOOGLE_PROJECT_ID=your-project-id
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json

# Optional: specify region (default: us-west1)
export GOOGLE_VERTEX_REGION=us-central1

# Index your codebase
crewchief maproom scan
```

**Requirements:**

- Google Cloud project with Vertex AI API enabled
- Service account with Vertex AI permissions
- Download service account JSON key

### Ollama (Local, Private)

Run embeddings locally with no API costs or external dependencies:

```bash
# Install Ollama (macOS)
brew install ollama

# Or use Docker from maproom-mcp:
cd packages/maproom-mcp
docker compose up -d ollama

# Set provider
export MAPROOM_EMBEDDING_PROVIDER=ollama

# Optional: specify model (default: nomic-embed-text)
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text

# Index your codebase
crewchief maproom scan
```

**Pros:** Free, private, no internet required
**Cons:** Slower than cloud providers, requires local compute

**Docker Note:** The maproom-mcp Docker setup automatically pulls the `nomic-embed-text` model on first startup.

## Performance Optimization

The maproom indexer includes several performance features to handle large codebases efficiently.

### Incremental Scanning (Default)

By default, `maproom scan` uses **incremental scanning** to only index changed files:

- Compares git tree SHA between runs
- Skips unchanged files automatically
- Dramatically faster for subsequent scans (10x+ speedup)
- Use `--force` to bypass and scan all files

```bash
# Incremental scan (default) - only changed files
crewchief maproom scan

# Force full re-index - scan all files
crewchief maproom scan --force
```

**When to use `--force`:**

- After schema migrations
- When switching embedding providers
- If incremental scan produces unexpected results

### Parallel Processing

Enable parallel processing for large repositories (>10k files):

```bash
# Enable parallel mode with default workers (4)
crewchief maproom scan --parallel

# Customize worker count for multi-core systems
crewchief maproom scan --parallel --parallel-workers 8
```

**Performance:** 4x+ faster on large codebases with multi-core CPUs.

**Trade-off:** Higher memory usage during indexing.

### Batch Size Tuning

Adjust batch sizes for optimal performance based on your environment:

```bash
# Larger batches for faster indexing (more memory)
crewchief maproom scan --batch-size 100 --embedding-batch-size 100

# Smaller batches for memory-constrained environments
crewchief maproom scan --batch-size 25 --embedding-batch-size 25
```

**Parameters:**

- `--batch-size` - Database insert batch size (default: 50)
- `--embedding-batch-size` - Embedding generation batch size (default: 50)

**Recommendation:** Start with defaults, increase for faster indexing on powerful machines.

## Configuration

Run the interactive setup wizard on first use:

```bash
crewchief setup
```

This will guide you through:

- Repository type (standard or monorepo)
- Main branch name
- Files to copy to new worktrees (.env files, etc.)
- Whether to update LLM guide files (CLAUDE.md, etc.)

The wizard creates a `crewchief.config.js` file in your project root with your preferences.

**Tip:** Add `.crewchief` to your `.gitignore` file to avoid committing worktree data.

### Manual Configuration

You can also manually create `crewchief.config.js`:

```javascript
export default {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
  },
  worktree: {
    // Auto-copy .env files to new worktrees
    copyIgnoredFiles: ['.env', '.env.local'],
    copyFromPath: '.',
    overwriteStrategy: 'skip', // 'skip', 'overwrite', or 'backup'
  },
  terminal: {
    backend: 'iterm',
    iterm: {
      sessionName: 'crewchief',
    },
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false,
  },
}
```

## Schema & Features

The maproom database schema has evolved to support advanced features like content addressing, deduplication, and branch-aware search.

### Migration 0018: Content-Addressed Storage (blob_sha)

Added `blob_sha` column to the `chunks` table for content-addressed storage:

```sql
ALTER TABLE maproom.chunks ADD COLUMN blob_sha TEXT NOT NULL;
CREATE INDEX idx_chunks_blob_sha ON maproom.chunks(blob_sha);
```

**Benefits:**

- Each chunk has a unique content-based hash (git-compatible blob SHA)
- Foundation for embedding deduplication
- Enables efficient content tracking across worktrees

**Implementation:** Uses `compute_git_blob_sha(text)` PostgreSQL function matching git's blob hash format.

### Migration 0019: Deduplicated Embeddings (code_embeddings)

Created dedicated `code_embeddings` table with HNSW vector index:

```sql
CREATE TABLE maproom.code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536) NOT NULL,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_embeddings_vector ON maproom.code_embeddings
  USING hnsw (embedding vector_cosine_ops);
```

**Benefits:**

- **70-90% storage reduction** for typical codebases
- Significantly faster search queries (HNSW index)
- One embedding per unique content blob (no duplicates)
- Reduced embedding generation costs

**Impact:** Large codebases see dramatic storage savings as identical code blocks across files/branches share a single embedding.

### Migration 0020: Worktree Tracking (worktree_ids)

Added worktree tracking to support multi-worktree workflows:

```sql
ALTER TABLE maproom.chunks
  ADD COLUMN worktree_ids JSONB DEFAULT '[]'::jsonb NOT NULL;

CREATE INDEX idx_chunks_worktree_ids ON maproom.chunks
  USING gin (worktree_ids);
```

**Benefits:**

- Branch-aware search (find code in specific worktrees)
- Incremental indexing per worktree
- Efficient worktree cleanup when branches are deleted
- Supports parallel development workflows

**Example Query:**

```sql
-- Find chunks in specific worktree
SELECT * FROM maproom.chunks WHERE worktree_ids ? '123';
```

**For more details:** See [Database Architecture](../../docs/architecture/DATABASE_ARCHITECTURE.md) and [migration files](../../crates/maproom/migrations/).

## Command Reference

All commands below should be prefixed with `crewchief`. For example: `crewchief worktree create feature-branch`

### Worktree Commands

| Command                        | Description                            |
| ------------------------------ | -------------------------------------- |
| `worktree create <name>`       | Create a new worktree                  |
| `worktree list`                | List all worktrees                     |
| `worktree use <name>`          | Switch to worktree (creates if needed) |
| `worktree merge <name>`        | Merge worktree changes back            |
| `worktree clean`               | Remove worktrees                       |
| `worktree copy-ignored <name>` | Copy .env files to worktree            |

### Maproom Commands (Semantic Search)

| Command                       | Description                    |
| ----------------------------- | ------------------------------ |
| `maproom db migrate`          | Initialize PostgreSQL database |
| `maproom scan`                | Index your codebase            |
| `maproom search <query>`      | Search code semantically       |
| `maproom watch`               | Auto-index on file changes     |
| `maproom upsert [files...]`   | Update specific files          |
| `maproom generate-embeddings` | Generate embeddings for chunks |

**Note:** For AI assistant integration, install [maproom-mcp](https://www.npmjs.com/package/maproom-mcp) instead of using these commands directly.

### Agent Commands (iTerm2 Required)

| Command                      | Description           |
| ---------------------------- | --------------------- |
| `spawn <agents> [task]`      | Spawn AI agents       |
| `agent list`                 | List running agents   |
| `agent message <name> <msg>` | Send message to agent |
| `agent close <id>`           | Close agent pane      |

### System Commands

| Command  | Description                      |
| -------- | -------------------------------- |
| `setup`  | Interactive configuration wizard |
| `doctor` | Check dependencies               |

## PostgreSQL Setup for Semantic Search

The semantic search features require PostgreSQL. Here's a quick setup:

### Option 1: Local PostgreSQL

```bash
# macOS with Homebrew
brew install postgresql@14
brew services start postgresql@14

# Create database
createdb maproom

# Set connection string
export MAPROOM_DATABASE_URL="postgresql://localhost:5432/maproom"
```

### Option 2: Docker

```bash
docker run -d \
  --name maproom-postgres \
  -e POSTGRES_DB=maproom \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres:14

export MAPROOM_DATABASE_URL="postgresql://postgres:password@localhost:5432/maproom"
```

### Option 3: Cloud Database

Use any PostgreSQL provider (Supabase, Neon, etc.) and set the connection string they provide.

## Supported File Types for Indexing

- TypeScript (.ts, .tsx)
- JavaScript (.js, .jsx)
- Markdown (.md, .mdx)
- JSON (.json)
- YAML (.yaml, .yml)
- TOML (.toml)
- Rust (.rs)

## Alternative Installation Methods

### Run without installing

```bash
npx @crewchief/cli --help
```

### Install in a project

```bash
npm install @crewchief/cli
# Then run with:
npx crewchief --help
```

## Security Best Practices

Proper credential management is essential when working with database connections and API keys.

### Option 1: .env File (Development)

Create a `.env` file in your project root with restricted permissions:

```bash
# Create .env file (never commit!)
cat > .env <<EOF
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
MAPROOM_EMBEDDING_PROVIDER=openai
OPENAI_API_KEY=sk-...
EOF

# Restrict permissions to owner-only
chmod 600 .env

# Add to .gitignore
echo ".env" >> .gitignore
```

**Automatic loading with direnv:**

```bash
# Install direnv (macOS)
brew install direnv

# Add hook to your shell (~/.bashrc or ~/.zshrc)
eval "$(direnv hook bash)"  # or 'zsh'

# Create .envrc instead of .env
cat > .envrc <<EOF
export MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
export OPENAI_API_KEY=sk-...
EOF

# Allow the directory
direnv allow
```

### Option 2: Secret Manager (Production)

Use external secret management for production environments:

**AWS Secrets Manager:**

```bash
# Fetch API key from AWS Secrets Manager
export OPENAI_API_KEY=$(aws secretsmanager get-secret-value \
  --secret-id openai-key \
  --query SecretString \
  --output text)

# Fetch database URL
export MAPROOM_DATABASE_URL=$(aws secretsmanager get-secret-value \
  --secret-id maproom-db-url \
  --query SecretString \
  --output text)

# Run crewchief commands
crewchief maproom scan
```

**HashiCorp Vault:**

```bash
# Authenticate to Vault
vault login -method=token

# Fetch credentials from Vault KV store
export OPENAI_API_KEY=$(vault kv get -field=key maproom/openai)
export MAPROOM_DATABASE_URL=$(vault kv get -field=url maproom/db)

# Run crewchief commands
crewchief maproom scan
```

**Benefits of secret managers:**

- Centralized credential storage
- Audit logging of access
- Automatic rotation support
- Fine-grained access control
- No credentials in source code or shell history

### Security Warnings

**Critical:**

- **Never commit credentials to git** - Always use `.gitignore` for `.env` files
- **Rotate API keys regularly** - Especially after potential exposure
- **Use read-only database credentials** when possible for indexing workloads
- **Environment variables are visible** to all processes - use `.env` files or secret managers for better isolation
- **Consider IAM roles** instead of static credentials (AWS, GCP) for cloud deployments

**Connection string exposure:**

```bash
# ❌ BAD: Credentials visible in process list
export MAPROOM_DATABASE_URL="postgresql://user:secret@host/db"

# ✅ GOOD: Load from protected .env file
source .env  # or use direnv

# ✅ BETTER: Use secret manager
export MAPROOM_DATABASE_URL=$(vault kv get -field=url maproom/db)
```

**File permissions:**

```bash
# Verify .env permissions (should be 600)
ls -la .env
# -rw------- 1 user group 123 Jan 10 12:00 .env

# Fix if needed
chmod 600 .env
```

## Troubleshooting

### Check system dependencies

```bash
crewchief doctor
```

This will check for:

- Node.js version (>= 18 required)
- Git installation
- PostgreSQL connectivity (if `MAPROOM_DATABASE_URL` is set)
- iTerm2 availability (macOS only)

### Common Issues

#### Database Connection Errors

**Symptom:** `PostgreSQL connection failed` or `ECONNREFUSED`

**Solutions:**

1. **Verify PostgreSQL is running:**

   ```bash
   # For Docker:
   docker compose ps

   # For Homebrew:
   brew services list | grep postgresql
   ```

2. **Check environment variable:**

   ```bash
   echo $MAPROOM_DATABASE_URL
   # Should output: postgresql://maproom:maproom@localhost:5433/maproom
   ```

3. **Test connection manually:**

   ```bash
   psql $MAPROOM_DATABASE_URL -c "SELECT version();"
   ```

4. **Verify port mapping:**
   - Docker setup uses port **5433** (not 5432)
   - Local PostgreSQL typically uses port **5432**
   - Cloud databases use provider-specific ports

**Common mistakes:**

- Using `PG_DATABASE_URL` instead of `MAPROOM_DATABASE_URL` (deprecated)
- Wrong port number in connection string
- PostgreSQL not started or crashed
- Firewall blocking connection

#### Embedding Provider Errors

**Symptom:** `Embedding generation failed` or `Provider not configured`

**Solutions:**

**For OpenAI:**

1. Verify API key is set:

   ```bash
   echo $OPENAI_API_KEY
   # Should start with: sk-...
   ```

2. Check API key permissions at https://platform.openai.com/api-keys

3. Ensure you have credits available

**For Google Vertex AI:**

1. Verify environment variables:

   ```bash
   echo $GOOGLE_PROJECT_ID
   echo $GOOGLE_APPLICATION_CREDENTIALS
   ```

2. Test service account authentication:

   ```bash
   gcloud auth activate-service-account --key-file=$GOOGLE_APPLICATION_CREDENTIALS
   ```

3. Ensure Vertex AI API is enabled in your project

**For Ollama:**

1. Check if Ollama is running:

   ```bash
   # For Docker:
   docker compose ps ollama

   # For local:
   ollama list
   ```

2. Verify model is pulled:

   ```bash
   ollama list | grep nomic-embed-text
   ```

3. If model missing, pull it manually:
   ```bash
   ollama pull nomic-embed-text
   ```

#### Binary Compatibility Issues

**Symptom:** `crewchief-maproom binary not found` or `command not found`

**Solutions:**

1. **Rebuild the Rust binary:**

   ```bash
   cd packages/cli
   pnpm build:rust
   ```

2. **Verify binary exists:**

   ```bash
   ls -la packages/cli/bin/$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)/
   ```

3. **Check platform compatibility:**
   - macOS: darwin-arm64 (Apple Silicon) or darwin-x64 (Intel)
   - Linux: linux-x64 or linux-arm64
   - Windows: Not currently supported for direct Rust binary usage

#### iTerm2 Not Found

**Symptom:** `iTerm2 is required to run CrewChief` or agent features don't work

**Solutions:**

1. Install iTerm2: https://iterm2.com/downloads.html

2. Ensure you're running commands _inside_ iTerm2, not Terminal.app

3. Verify iTerm2 version (>= 3.4 recommended):
   - iTerm2 → About iTerm2

**Note:** Agent orchestration features are macOS-only and require iTerm2. Worktree and semantic search work on all platforms.

#### Worktree Creation Failed

**Symptom:** `fatal: not a git repository` or `worktree creation failed`

**Solutions:**

1. Ensure you're in a git repository:

   ```bash
   git status
   ```

2. Verify at least one commit exists:

   ```bash
   git log
   ```

3. Check for uncommitted changes that might conflict:
   ```bash
   git status
   ```

#### Environment Variable Debugging

If you're unsure which variables are set:

```bash
# Check all MAPROOM variables
env | grep MAPROOM

# Check database connection
env | grep -E "(MAPROOM_DATABASE_URL|PG_DATABASE_URL|DATABASE_URL)"

# Check embedding provider
env | grep -E "(MAPROOM_EMBEDDING|OPENAI|GOOGLE|OLLAMA)"
```

### Getting Help

If issues persist:

1. Run `crewchief doctor` and share the output
2. Check logs: `~/.crewchief/logs/` (if exists)
3. Open an issue at https://github.com/your-org/crewchief/issues with:
   - Error message (remove sensitive data)
   - Output of `crewchief doctor`
   - Platform and Node.js version

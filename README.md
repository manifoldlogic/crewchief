# CrewChief

A multi-tool CLI for git worktree management, semantic code search, and AI agent orchestration.

## Requirements

**macOS with [iTerm2](https://iterm2.com/downloads.html)**  
> ⚠️ iTerm2 is required for agent orchestration features.

## What's Working

✅ **Git Worktree Management** - Simplify creating, listing, and navigating git worktrees
✅ **Semantic Code Search** - Index and search code, docs, and configs using PostgreSQL
✅ **Automatic Branch Detection** ✨ - Auto-index branches on switch (no manual scan needed)
✅ **Grep-Impossible Task Framework** - Scientific validation framework for semantic search value
✅ **MCP Integration** - Maproom MCP server for AI assistants (Claude, Cursor)
✅ **Multi-Format Support** - TypeScript, JavaScript, Rust, Markdown, JSON, YAML, TOML
✅ **Agent Orchestration** - Spawn AI agents in iTerm2 panes with isolated worktrees
✅ **Agent Communication** - Send messages to agents with proper text submission (chr(13) for Claude)

## What's In Progress

⚠️ **Competition Mode** - Run multiple agents on the same task and compare results

## Installation

### Option 1: Run Without Installing

```bash
# Run directly with npx (downloads and runs temporarily)
npx @crewchief/cli --help

# Or with pnpm dlx
pnpm dlx @crewchief/cli --help

# Or with yarn dlx
yarn dlx @crewchief/cli --help
```

### Option 2: Install in Your Project

```bash
# Install as a project dependency
npm install @crewchief/cli
# or
pnpm add @crewchief/cli
# or
yarn add @crewchief/cli

# Run with npx/pnpm/yarn
npx crewchief --help
pnpm crewchief --help
yarn crewchief --help
```

### Option 3: Install Globally (Recommended)

```bash
# Install globally via npm
npm install -g @crewchief/cli

# Or with pnpm
pnpm add -g @crewchief/cli

# Or with yarn
yarn global add @crewchief/cli

# Now use directly
crewchief --help
crewchief --version
```

### Migrating from Old `crewchief` Package

If you previously installed the unscoped `crewchief` package (v0.x), you'll need to migrate to the new `@crewchief/cli` package:

```bash
# Uninstall old package
npm uninstall -g crewchief

# Install new scoped package
npm install -g @crewchief/cli

# Verify installation
crewchief --version  # Should show 1.0.0 or higher
```

**What changed in v1.0.0:**
- Package renamed from `crewchief` → `@crewchief/cli`
- All 4 platforms now supported (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- Automated GitHub Actions releases
- No breaking functionality changes

See [MIGRATION.md](MIGRATION.md) for full migration guide.

## Quick Start

```bash
# Set up database (required for maproom features)
export PG_DATABASE_URL="postgres://user:password@localhost:5432/maproom"
crewchief maproom:db

# Index your code (auto-detects git context)
# Embeddings are generated automatically during scan!
crewchief maproom:scan
# ✅ Scan completed successfully!
#    Files processed: 150
#    Total chunks: 1234
# 🔄 Generating embeddings for new chunks...
#    Found 1234 chunks needing embeddings
# 📊 Embedding Generation Summary:
#    Processed 1234 chunks in 15.3s (80.6 chunks/s)

# Search semantically (works immediately after scan!)
crewchief maproom:search "authentication flow"

# Manage worktrees
crewchief worktree create feature-branch
crewchief worktree list
crewchief worktree use feature-branch

# Auto-copy .env files to new worktrees (configure in crewchief.config.js)
crewchief worktree copy-ignored feature-branch

# Merge worktree changes back to source branch
crewchief worktree merge feature-branch

# Spawn AI agents in iTerm2 (REQUIRES iTerm2)
crewchief spawn claude "implement-auth"      # Creates worktree 'implement-auth__claude' and launches Claude
crewchief spawn gemini "code-review"         # Creates worktree 'code-review__gemini' and launches Gemini
crewchief spawn claude,gemini "fix-bug"      # Spawn BOTH agents at once with smart splitting
crewchief agent list                          # List all running agents with their full names
crewchief agent message implement-auth__claude "Add OAuth support"  # Send task to specific Claude agent
crewchief agent message fix-bug__claude --file prompt.md  # Send file contents as prompt to agent
crewchief agent message fix-bug --all "Update approach"  # Send to ALL agents working on fix-bug
crewchief agent message "*" --all "Status update"  # Broadcast to ALL running agents
```

## Grep-Impossible Task Framework

Scientific validation framework for semantic code search. Provides rigorous, objective proof that semantic search delivers measurable value over traditional grep-based tools through 30+ benchmark tasks across three tiers.

### Three-Tier Validation

**Tier 1: Grep-Impossible** - Tasks grep fundamentally cannot solve (<30% success rate)
- Transitive dependency analysis
- Architectural flow tracing
- Negative space detection (finding code that lacks properties)
- **Proves**: Semantic search can solve problems grep cannot

**Tier 2: Grep-Hard** - Tasks where semantic search is significantly more efficient
- Conceptual similarity (finding patterns across different naming)
- Ambiguity resolution (disambiguating through context)
- Cross-cutting concerns (scattered functionality)
- **Proves**: 30-50% faster and more accurate than grep

**Tier 3: Real-World** - Natural developer scenarios without tool coercion
- Code review, debugging, refactoring tasks
- Voluntary tool selection based on task characteristics
- **Proves**: Developers naturally adopt when appropriate

### Key Features

- **Objective Validation**: Binary pass/fail criteria, no subjective judgment
- **Natural Selection**: Agents choose tools organically—no coercion
- **Ecological Validity**: All tasks based on real development workflows
- **Statistical Rigor**: p < 0.05 significance testing, cross-project validation
- **Integration Ready**: Works with genetic optimization for tool description evolution

See [Search Evaluation Architecture](docs/architecture/SEARCH_EVALUATION.md) for comprehensive details and [Search Optimization Framework](docs/search-optimization/) for implementation guides.

## Project Structure

```
crewchief/
├── packages/
│   ├── cli/           # Main TypeScript CLI
│   └── maproom-mcp/   # MCP server for AI assistants
├── crates/
│   └── maproom/       # Rust indexing engine
└── .agents/           # Project planning, tickets, and knowledge base
```

## Documentation

- [CLI README](packages/cli/README.md) - Detailed command reference
- [Architecture Spec](.agents/knowledge/cli/specification.md) - Full vision with implementation status
- [Testing Report](TESTING_REPORT.md) - Features that need verification
- **[Search Optimization Framework](docs/search-optimization/)** - Grep-impossible task design and validation

## Embedding Configuration

Embeddings are **automatically generated** during `scan` and `upsert` operations, enabling semantic search out-of-the-box.

### Provider Options

Configure your embedding provider in `.env`:

**Option 1: Ollama (Local, Free, Default)**
```bash
EMBEDDING_PROVIDER=ollama
EMBEDDING_MODEL=nomic-embed-text
EMBEDDING_DIMENSION=768
```

**Option 2: OpenAI (Cloud, Requires API Key)**
```bash
EMBEDDING_PROVIDER=openai
OPENAI_API_KEY=your-key-here
EMBEDDING_MODEL=text-embedding-3-small
EMBEDDING_DIMENSION=1536
```

### Control Auto-Generation

```bash
# Disable auto-generation (for testing or manual control)
crewchief maproom:scan --generate-embeddings=false

# Adjust batch size for performance tuning
crewchief maproom:scan --embedding-batch-size=100

# Generate embeddings manually later
crewchief-maproom generate-embeddings
```

### Performance Tuning

Fine-tune embedding generation performance in `.env`:

```bash
EMBEDDING_BATCH_SIZE=50                    # Chunks per batch
EMBEDDING_PARALLEL_ENABLED=true            # Enable parallel processing
EMBEDDING_PARALLEL_SUB_BATCH_SIZE=25      # Sub-batch size
EMBEDDING_PARALLEL_MAX_CONCURRENCY=4      # Concurrent requests
```

## Requirements

- Node.js >= 18
- PostgreSQL (for Maproom)
- Git
- iTerm2 (required for agent features, on macOS)
- **Ollama** (optional, for local embeddings) - [Install Ollama](https://ollama.ai/download)

## Contributing

This project is actively being developed. Key areas that need work:

1. Completing the agent orchestration features
2. Implementing evaluation metrics for competition mode

See the [specification](.agents/knowledge/cli/specification.md) for the full roadmap.

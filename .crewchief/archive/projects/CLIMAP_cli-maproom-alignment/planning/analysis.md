# Analysis: CLI-Maproom Alignment

**Project:** CLIMAP - CLI-Maproom Alignment
**Date:** 2025-01-10

## Problem Definition

The TypeScript CLI package (`/workspace/packages/cli`) serves as the primary user interface for CrewChief, including git worktree management and semantic code search via maproom. Over time, significant changes to the maproom Rust binary and maproom-mcp server have created misalignment with the CLI package.

### Core Issues

1. **Deprecated Environment Variables**
   - CLI documentation references `PG_DATABASE_URL` (PostgreSQL naming)
   - Maproom now uses `MAPROOM_DATABASE_URL` with 4-tier fallback system
   - Users following CLI docs will set wrong variables

2. **Missing Feature Documentation**
   - Embedding provider setup not documented (OpenAI, Google, Ollama)
   - New performance flags not mentioned (`--force`, `--parallel`, `--provider`)
   - Schema evolution not explained (blob_sha, code_embeddings, worktree tracking)

3. **Command Naming Inconsistency**
   - Maproom commands use `:` prefix pattern: `maproom:scan`, `maproom:search`
   - Other CLI features use subcommand pattern: `worktree create`, `worktree list`
   - Inconsistency suggests maproom is a separate tool, not integrated feature
   - Makes CLI feel less unified

4. **Feature Gaps**
   - Three new Rust commands not exposed: `branch-watch`, `cache`, `generate-embeddings`
   - Performance optimizations unknown to users (parallel processing, incremental scanning)

5. **Lack of Environment Validation**
   - No pre-flight checks before running Rust binary
   - Users encounter cryptic Rust errors for missing configuration
   - Poor developer experience for new users

## Current State Assessment

### CLI Architecture (Good Foundation)

The CLI uses a **pure forwarding model** for maproom commands:

```typescript
// packages/cli/src/cli/maproom.ts
function runMaproomForward(args: string[]) {
  const bin = resolvePackagedMaproomBin()
  const res = spawnSync(bin, args, { stdio: 'inherit' })
}
```

**Implications:**
- CLI doesn't directly interact with maproom APIs
- Changes to Rust internal APIs don't break CLI
- CLI only needs to document new commands/flags
- Adding validation layer is safe (no risk of API mismatch)

### Current Command Structure

```typescript
// Inconsistent pattern: colon-separated
program.command('maproom:scan')
program.command('maproom:search')
program.command('maproom:upsert')
program.command('maproom:watch')
program.command('maproom:db')

// Compared to other features: subcommands
program.command('worktree').command('create')
program.command('worktree').command('list')
program.command('worktree').command('clean')
```

### Existing Maproom Documentation Gaps

From `/workspace/packages/cli/README.md`:

**Database Connection (4 locations):**
```bash
# Current (wrong)
export PG_DATABASE_URL="postgres://localhost:5432/maproom"

# Should be
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom"
```

**Missing Sections:**
- No embedding provider setup instructions
- No performance optimization guidance
- No schema version or migration status
- No troubleshooting for common errors

## Industry Solutions & Patterns

### Command Naming Patterns

**Subcommand Pattern (Standard):**
```bash
git worktree add <path>
git worktree list
git worktree remove <path>

docker container ls
docker container rm <id>

kubectl get pods
kubectl delete pod <name>
```

**Colon-Separated Pattern (Rare):**
```bash
npm run:script
# (But npm also uses subcommands: npm install, npm test)
```

**Analysis:** Industry standard heavily favors subcommand pattern. Users expect `<tool> <noun> <verb>` structure.

### CLI Environment Validation Patterns

**Pre-flight Checks (Common):**
- Docker checks daemon running before commands
- kubectl checks kubeconfig before operations
- Terraform validates config before plan/apply

**Pattern:**
1. Validate required environment variables
2. Test critical dependencies (database connection)
3. Show actionable error messages
4. Link to setup documentation

### Documentation Patterns

**Effective CLI Documentation:**
- Quick start with minimal setup
- Separate sections for each major feature
- Environment variable reference table
- Troubleshooting section with common errors
- Performance tuning guidance

## Research Findings

### Maproom Environment Variables (from crates/maproom/src/config)

**Database Connection (4-tier fallback):**
1. `MAPROOM_DATABASE_URL` (recommended)
2. Component-based: `MAPROOM_DB_HOST`, `MAPROOM_DB_PORT`, `MAPROOM_DB_NAME`
3. Fallback: `PG_DATABASE_URL` (backward compatibility)
4. Fallback: `DATABASE_URL`

**Embedding Providers:**
```bash
# Required
MAPROOM_EMBEDDING_PROVIDER=ollama|openai|google

# Provider-specific
# OpenAI
OPENAI_API_KEY=sk-...
MAPROOM_OPENAI_API_KEY=sk-...  # Override

# Google
GOOGLE_PROJECT_ID=my-project
GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
MAPROOM_GOOGLE_PROJECT_ID=...  # Override
MAPROOM_GOOGLE_APPLICATION_CREDENTIALS=...  # Override

# Ollama (local)
MAPROOM_EMBEDDING_API_ENDPOINT=http://localhost:11434/api/embed
```

### New Rust Commands (from crates/maproom/src/main.rs)

**BranchWatch:**
```rust
BranchWatch => {
    // Auto-index worktrees when switching branches
    // Monitors git events, triggers incremental scans
}
```

**Cache:**
```rust
Cache(CacheCommand) => {
    // Manage LRU caches
    // Subcommands: clear, stats
}
```

**GenerateEmbeddings:**
```rust
GenerateEmbeddings { chunk_ids, batch_size, provider } => {
    // Manual embedding generation for indexed chunks
    // Useful for backfilling after schema changes
}
```

### New Scan Flags

**Performance:**
- `--force` - Bypass incremental mode, full re-scan
- `--parallel` - Enable parallel batch processing
- `--parallel-workers N` - Worker count (default: 4)
- `--batch-size N` - Database batch size (default: 50)
- `--embedding-batch-size N` - Embedding batch size (default: 50)

**Configuration:**
- `--provider PROVIDER` - Override `MAPROOM_EMBEDDING_PROVIDER`
- `--generate-embeddings` - Auto-generate embeddings (default: true)

### Schema Evolution (migrations 0018-0020)

**Migration 0018 (blob_sha):**
- Added `blob_sha TEXT NOT NULL` to chunks
- Foundation for content-addressed storage
- Enables deduplication

**Migration 0019 (code_embeddings):**
- Created dedicated `code_embeddings` table
- Deduplicated from per-chunk storage
- HNSW vector index for fast similarity search
- 70-90% storage reduction

**Migration 0020 (worktree_tracking):**
- Added `worktree_ids JSONB` to chunks
- Created `worktree_index_state` table
- Enables branch-aware search
- Supports incremental indexing per worktree

## Impact Analysis

### User Impact

**Current Pain Points:**
1. **Setup failure** - Wrong env var → connection failure → frustration
2. **Missing features** - Can't discover parallel mode, branch-watch, etc.
3. **Inconsistent UX** - Colon commands don't match rest of CLI
4. **Poor discoverability** - `--help` doesn't show new flags

**Expected Improvements:**
1. **Smooth onboarding** - Correct env vars, validation, helpful errors
2. **Feature awareness** - Documentation exposes capabilities
3. **Consistent UX** - All commands follow same pattern
4. **Better performance** - Users learn about `--parallel` flag

### Developer Impact

**Maintenance Benefits:**
1. **Single pattern** - Subcommands only, easier to understand
2. **Validation layer** - Catch errors before Rust binary
3. **Documentation parity** - CLI docs match maproom reality

**Migration Risks:**
1. **Breaking change** - Users with scripts using `maproom:scan`
2. **Alias needed** - Temporary backward compatibility
3. **Communication** - Clear changelog and migration guide

## Conclusion

The CLI package needs comprehensive alignment across documentation, command structure, and validation. The pure forwarding architecture makes this low-risk—no API integration to break. The biggest change is command naming, which requires careful migration planning for backward compatibility.

**Key Insights:**
1. Problem is primarily **documentation and UX**, not deep technical debt
2. **Command naming refactor** is justified—aligns with industry standard
3. **Environment validation** adds significant value for new users
4. **Low risk** due to forwarding architecture
5. **High impact** for user experience and feature discoverability

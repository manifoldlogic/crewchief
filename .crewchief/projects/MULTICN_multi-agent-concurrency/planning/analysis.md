# Analysis: Multi-Agent Concurrency for Maproom

## Problem Definition

### Context

Maproom is a semantic code search system that indexes codebases using tree-sitter parsing and vector embeddings. It stores data in SQLite with WAL mode enabled. The system includes:

1. **Rust daemon** (`crewchief-maproom serve`) - Handles search and indexing
2. **TypeScript client** (`daemon-client`) - JSON-RPC communication layer
3. **MCP server** (`maproom-mcp`) - Exposes daemon to Claude Code

### Current Architecture

```
Agent 1 (worktree A)           Agent 2 (worktree B)
        │                              │
        ▼                              ▼
   MCP Client 1                   MCP Client 2
        │                              │
        ▼                              ▼
  DaemonClient 1                 DaemonClient 2
        │                              │
        ▼                              ▼
   Daemon 1                        Daemon 2
  (spawn via stdin/stdout)       (spawn via stdin/stdout)
        │                              │
        └──────────┬───────────────────┘
                   ▼
            SQLite Database
           (WAL mode, file locks)
```

### Problem Symptoms

1. **SQLITE_BUSY errors**: With 5s busy_timeout, concurrent indexing fails
2. **Memory overhead**: 3 agents = ~300MB daemon memory (3 × 100MB)
3. **Unpredictable latency**: File-level lock contention causes spikes
4. **No resource sharing**: Each daemon has its own r2d2 connection pool

### User Requirements (Confirmed)

- **Primary use case**: Multiple agents on different worktrees/branches
- **Change tolerance**: Moderate architectural changes acceptable
- **Cross-worktree search**: Not required
- **Embedding deduplication**: Must preserve (cost savings critical)

## Existing Industry Solutions

### 1. SQLite WAL Mode (Current)

**Mechanism**: Write-Ahead Logging allows concurrent readers while a single writer appends to the WAL file.

**Limitations**:
- Single writer at any time (database-level lock)
- Multiple processes compete via OS file locks
- busy_timeout only delays; doesn't solve contention

### 2. Connection Pooling (Current: r2d2)

**Current state**: Each daemon has a 10-connection pool
**Problem**: Multiple daemons = multiple pools = no coordination

### 3. PostgreSQL Connection Poolers (PgBouncer, etc.)

**Approach**: Central process manages connection pool, clients connect to pooler
**Applicable pattern**: Single daemon acting as connection coordinator

### 4. Application-Level Write Serialization

**Approach**: Queue writes through single coordinator process
**Examples**: Redis, SQLite in server mode (not standalone)
**Applicable pattern**: Shared daemon serializes all writes

### 5. Per-Instance Databases with Sync

**Approach**: Each client has its own database, sync mechanism coordinates
**Applicable pattern**: Per-worktree databases (considered but not chosen)

## Current Project State

### SQLite Configuration

```rust
// crates/maproom/src/db/sqlite/mod.rs
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;  // Only 5 seconds
```

### Daemon Architecture

```rust
// crates/maproom/src/daemon/mod.rs
pub async fn run() -> Result<()> {
    let store = Arc::new(connect().await?);
    let embedding_service = EmbeddingService::from_env().await?;

    // Reads from stdin, writes to stdout
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    // ...
}
```

### Client Spawning

```typescript
// packages/daemon-client/src/client.ts
private async start(): Promise<void> {
    this.daemonProcess = spawn(this.config.binaryPath, ['serve'], {
        stdio: ['pipe', 'pipe', 'pipe'],
        // Each client spawns its own daemon
    });
}
```

### Data Model

- **Embeddings**: Deduplicated by `blob_sha` (git content hash)
- **Chunks**: Linked to worktrees via `chunk_worktrees` junction table
- **Same code across worktrees shares embeddings** (must preserve)

## Research Findings

### Concurrency Analysis

| Scenario | Current Behavior | Root Cause |
|----------|------------------|------------|
| 2 agents searching | Works (WAL concurrent reads) | Reads don't block |
| 2 agents indexing | SQLITE_BUSY errors | Single writer at file level |
| Search during index | Works but may wait | Reader waits for checkpoint |
| 3+ agents indexing | Severe contention | 5s timeout insufficient |

### Memory Profile

| Component | Size | Per-Agent |
|-----------|------|-----------|
| Daemon process | ~50MB base | ~50MB |
| r2d2 pool (10 conn) | ~10MB | ~10MB |
| Embedding cache | ~40MB | ~40MB |
| **Total** | ~100MB | × N agents |

### WAL Mode Limitations

1. **Checkpoint blocking**: During WAL checkpoint, all readers must wait
2. **WAL file growth**: Without checkpoints, WAL can grow unbounded
3. **Cross-process coordination**: OS file locks, not application-controlled

### Shared Daemon Benefits

1. **Single writer queue**: All writes serialized at application level
2. **Memory sharing**: One daemon serves all agents (~100MB total)
3. **Controlled checkpoints**: Daemon can checkpoint during idle periods
4. **Connection pool sharing**: Single r2d2 pool (10 connections for all)

## Key Insights

### Why Shared Daemon Over Per-Worktree Databases

1. **Simpler**: No cross-database queries for embeddings
2. **Embedding sharing**: Central database preserves blob_sha deduplication
3. **No sync complexity**: No need to coordinate multiple databases
4. **User doesn't need cross-worktree search**: Single database sufficient

### Why Unix Socket Over TCP

1. **Performance**: ~10-20% faster (no network stack)
2. **Security**: File permissions (uid-based access control)
3. **Discovery**: Fixed path (`/tmp/maproom-{uid}.sock`)
4. **No firewall issues**: Local only

### Why Connect-or-Spawn Pattern

1. **Transparent**: First client starts daemon, others join
2. **No manual daemon management**: Users don't run `maproom daemon start`
3. **Graceful handling**: Lock file prevents race conditions
4. **Backward compatible**: Can fall back to stdio mode

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Daemon crash affects all clients | High | Auto-restart via DaemonLifecycle, circuit breaker |
| Socket permission issues | Medium | Automatic fallback to stdio mode |
| Windows compatibility | Low (out of scope) | Windows auto-detects and uses stdio mode |
| Orphan daemon processes | Medium | PID file with O_EXCL, idle timeout (5min) |
| Connect-or-spawn race condition | High | proper-lockfile library, double-check pattern |
| Message framing corruption | Medium | tokio_util::LengthDelimitedCodec (battle-tested) |

## Conclusion

The shared daemon via Unix socket approach is the optimal solution for the stated requirements:

1. **Addresses primary use case**: Multiple agents on different worktrees
2. **Moderate change scope**: Aligns with user's risk tolerance
3. **Preserves embedding deduplication**: Central database maintained
4. **No cross-worktree complexity**: Not needed per requirements

Implementation should proceed in two phases: SQLite optimizations first (quick wins, foundation), then shared daemon architecture (main solution).

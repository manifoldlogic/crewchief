# Architecture: Multi-Agent Concurrency for Maproom

## Solution Overview

Convert maproom from per-client daemon spawning to a shared daemon architecture via Unix socket server. This serializes writes at the application level, eliminating SQLite file-lock contention.

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│ Shared Daemon (crewchief-maproom serve --socket)                     │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ Unix Socket Listener                                            │ │
│  │ Path: /tmp/maproom-{uid}.sock                                   │ │
│  │ Backlog: 128 connections                                        │ │
│  └─────────────────────────┬──────────────────────────────────────┘ │
│                            │                                         │
│  ┌─────────────────────────▼──────────────────────────────────────┐ │
│  │ Connection Manager                                              │ │
│  │ • Accept loop spawns per-client tasks                          │ │
│  │ • Session tracking: client_id → metadata                       │ │
│  │ • Request/response multiplexing via request IDs                │ │
│  │ • Graceful disconnect handling                                  │ │
│  └─────────────────────────┬──────────────────────────────────────┘ │
│                            │                                         │
│  ┌─────────────────────────▼──────────────────────────────────────┐ │
│  │ Shared State (Arc<DaemonState>)                                │ │
│  │ • SqliteStore: single r2d2 pool (10 connections)              │ │
│  │ • EmbeddingService: shared API client                         │ │
│  │ • SessionRegistry: connected client tracking                   │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ Lifecycle Manager                                               │ │
│  │ • PID file: /tmp/maproom-{uid}.pid                             │ │
│  │ • Idle timeout: 5 minutes with no clients                      │ │
│  │ • SIGTERM: graceful shutdown                                   │ │
│  │ • SIGHUP: reload configuration                                 │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
           ▲              ▲              ▲
           │              │              │
      Agent 1        Agent 2        Agent 3
    (worktree A)   (worktree B)   (worktree C)
           │              │              │
    DaemonClient   DaemonClient   DaemonClient
    (socket mode)  (socket mode)  (socket mode)
```

## Component Design

### 1. Socket Server (Rust)

**Location**: `crates/maproom/src/daemon/server.rs`

```rust
pub struct SocketServer {
    socket_path: PathBuf,
    state: Arc<DaemonState>,
    shutdown_tx: broadcast::Sender<()>,
}

impl SocketServer {
    pub async fn run(config: ServerConfig) -> Result<()> {
        // 1. Write PID file with flock
        let _pid_guard = PidFile::create(&config.pid_path)?;

        // 2. Create Unix socket
        let listener = UnixListener::bind(&config.socket_path)?;

        // 3. Initialize shared state
        let state = Arc::new(DaemonState::new().await?);

        // 4. Accept loop
        loop {
            tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    let state = state.clone();
                    tokio::spawn(handle_client(stream, state));
                }
                _ = shutdown_signal() => break,
            }
        }

        // 5. Cleanup
        std::fs::remove_file(&config.socket_path)?;
        Ok(())
    }
}
```

### 2. Session Management

**Location**: `crates/maproom/src/daemon/session.rs`

```rust
pub struct Session {
    pub id: Uuid,
    pub connected_at: Instant,
    pub request_count: AtomicU64,
    response_tx: mpsc::Sender<JsonRpcResponse>,
}

pub struct SessionRegistry {
    sessions: DashMap<Uuid, Session>,
}

impl SessionRegistry {
    pub fn register(&self, response_tx: mpsc::Sender<JsonRpcResponse>) -> Uuid;
    pub fn unregister(&self, id: &Uuid);
    pub fn active_count(&self) -> usize;
    pub fn broadcast(&self, notification: JsonRpcNotification);
}
```

### 3. Protocol Layer

**Location**: `crates/maproom/src/daemon/protocol.rs`

**Wire format**: Length-prefixed JSON-RPC

```
┌────────────┬────────────────────────────────────┐
│ 4 bytes    │ N bytes                            │
│ (big-end)  │ JSON-RPC message                   │
│ length=N   │ (request or response)              │
└────────────┴────────────────────────────────────┘
```

```rust
pub struct LengthPrefixedCodec;

impl Decoder for LengthPrefixedCodec {
    type Item = JsonRpcMessage;
    // Read 4-byte length, then N bytes of JSON
}

impl Encoder<JsonRpcMessage> for LengthPrefixedCodec {
    // Write 4-byte length, then JSON bytes
}
```

### 4. TypeScript Client Updates

**Location**: `packages/daemon-client/src/socket.ts`

```typescript
export class SocketConnection implements Connection {
    private socket: net.Socket;
    private pendingRequests = new Map<number, PendingRequest>();
    private buffer = Buffer.alloc(0);

    async connect(socketPath: string): Promise<void> {
        this.socket = net.createConnection(socketPath);
        this.socket.on('data', this.handleData.bind(this));
        await this.waitForConnection();
    }

    private handleData(data: Buffer): void {
        this.buffer = Buffer.concat([this.buffer, data]);
        while (this.buffer.length >= 4) {
            const length = this.buffer.readUInt32BE(0);
            if (this.buffer.length < 4 + length) break;

            const json = this.buffer.slice(4, 4 + length).toString('utf8');
            const message = JSON.parse(json);
            this.handleMessage(message);

            this.buffer = this.buffer.slice(4 + length);
        }
    }

    async sendRequest<T>(method: string, params?: unknown): Promise<T> {
        const id = this.nextId++;
        const message = { jsonrpc: '2.0', method, params, id };
        return this.writeAndWait(message);
    }
}
```

### 5. Connect-or-Spawn Logic

**Location**: `packages/daemon-client/src/discovery.ts`

```typescript
export async function connectOrSpawn(config: DaemonConfig): Promise<Connection> {
    const socketPath = getSocketPath();

    // Try existing daemon first
    try {
        const conn = new SocketConnection();
        await conn.connect(socketPath);
        return conn;
    } catch {
        // No daemon running
    }

    // Acquire lock to prevent race condition
    const lockPath = getLockPath();
    const lock = await acquireLock(lockPath);

    try {
        // Double-check (another process may have started daemon)
        try {
            const conn = new SocketConnection();
            await conn.connect(socketPath);
            return conn;
        } catch {
            // Still no daemon
        }

        // Spawn daemon
        const daemon = spawn(config.binaryPath, ['serve', '--socket'], {
            detached: true,
            stdio: 'ignore',
        });
        daemon.unref();

        // Wait for socket
        await waitForSocket(socketPath, config.startupTimeout);

        // Connect
        const conn = new SocketConnection();
        await conn.connect(socketPath);
        return conn;
    } finally {
        await releaseLock(lock);
    }
}
```

## SQLite Optimizations

### Enhanced PRAGMA Configuration

```rust
const SQLITE_PRAGMAS: &str = r#"
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    PRAGMA busy_timeout = 30000;        -- 30s (was 5s)
    PRAGMA wal_autocheckpoint = 10000;  -- ~40MB threshold
    PRAGMA cache_size = -65536;         -- 64MB page cache
    PRAGMA mmap_size = 268435456;       -- 256MB mmap
    PRAGMA foreign_keys = ON;
"#;
```

### Configurable Pool Sizes

```rust
pub struct SqliteConfig {
    pub read_pool_size: u32,      // Default: 20
    pub write_pool_size: u32,     // Default: 3
    pub busy_timeout_ms: u64,     // Default: 30000
    pub cache_size_kb: i32,       // Default: 65536
}

impl SqliteConfig {
    pub fn from_env() -> Self {
        Self {
            read_pool_size: env_or("MAPROOM_SQLITE_READ_POOL_SIZE", 20),
            write_pool_size: env_or("MAPROOM_SQLITE_WRITE_POOL_SIZE", 3),
            // ...
        }
    }
}
```

### Retry Logic for Writes

```rust
pub async fn write_with_retry<F, T>(&self, op: F) -> Result<T>
where
    F: Fn(&mut Connection) -> Result<T> + Clone,
{
    let mut delay = Duration::from_millis(50);
    for attempt in 0..5 {
        match self.write(op.clone()).await {
            Ok(result) => return Ok(result),
            Err(e) if is_busy_error(&e) && attempt < 4 => {
                tracing::warn!(attempt, "SQLITE_BUSY, retrying");
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

## Technology Choices

| Choice | Decision | Rationale |
|--------|----------|-----------|
| Socket type | Unix domain socket | Faster than TCP, file-permission security |
| Protocol | Length-prefixed JSON-RPC | Binary-safe framing, existing JSON-RPC messages |
| Session tracking | DashMap | Lock-free concurrent hashmap |
| Codec | tokio_util LengthDelimitedCodec | Battle-tested framing |
| Lifecycle | PID file + idle timeout | Prevents orphans, auto-cleanup |

## Performance Considerations

### Expected Improvements

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Memory (3 agents) | ~300MB | ~100MB | 67% reduction |
| SQLITE_BUSY errors | Frequent | Eliminated | 100% |
| Connection pools | 30 (3×10) | 10 (shared) | 67% reduction |
| Write latency p99 | Unpredictable | Queued, predictable | Stable |

### Latency Impact

- **First request**: ~50ms additional (socket connect vs pipe)
- **Subsequent requests**: ~1ms additional (socket vs pipe overhead)
- **Trade-off**: Acceptable for reliability gains

## Long-term Maintainability

### Extensibility Points

1. **New RPC methods**: Add to handler dispatch table
2. **New protocols**: Codec trait allows alternatives (gRPC future)
3. **Metrics**: Session registry tracks request counts
4. **Rate limiting**: Semaphore-based per-client limits (future)

### Migration Path

1. **Phase 1**: Socket mode opt-in via `--socket` flag
2. **Phase 2**: Socket mode default, stdio fallback
3. **Phase 3**: Deprecate stdio mode (after stability proven)

## Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `crates/maproom/src/daemon/server.rs` | Unix socket server |
| `crates/maproom/src/daemon/session.rs` | Session management |
| `crates/maproom/src/daemon/protocol.rs` | Length-prefixed framing |
| `packages/daemon-client/src/socket.ts` | Socket connection |
| `packages/daemon-client/src/discovery.ts` | Connect-or-spawn |

### Modified Files

| File | Changes |
|------|---------|
| `crates/maproom/src/daemon/mod.rs` | Add `run_server()` |
| `crates/maproom/src/main.rs` | Add `--socket` flag |
| `crates/maproom/src/db/sqlite/mod.rs` | PRAGMA config, retry |
| `packages/daemon-client/src/client.ts` | Connection mode abstraction |
| `packages/daemon-client/src/lifecycle.ts` | Socket lifecycle |

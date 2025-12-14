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
│  │ • Lock file: /tmp/maproom-{uid}.lock                           │ │
│  │ • Idle timeout: 5 minutes with no clients (AtomicUsize counter)│ │
│  │ • SIGTERM: graceful shutdown with request draining             │ │
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
    response_tx: mpsc::Sender<JsonRpcResponse>,
}

pub struct SessionRegistry {
    sessions: DashMap<Uuid, Session>,
    active_count: AtomicUsize,  // For idle timeout tracking
}

impl SessionRegistry {
    pub fn register(&self, response_tx: mpsc::Sender<JsonRpcResponse>) -> Uuid {
        // Increments active_count, logs connection
    }
    pub fn unregister(&self, id: &Uuid) {
        // Decrements active_count, logs disconnection
    }
    pub fn active_count(&self) -> usize;
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
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB

pub struct JsonRpcCodec {
    inner: LengthDelimitedCodec,
}

impl JsonRpcCodec {
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::builder()
                .max_frame_length(MAX_MESSAGE_SIZE)
                .length_field_type::<u32>()
                .big_endian()
                .new_codec(),
        }
    }
}

impl Decoder for JsonRpcCodec {
    type Item = JsonRpcMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Delegate to battle-tested LengthDelimitedCodec, then parse JSON
        if let Some(bytes) = self.inner.decode(src)? {
            let message = serde_json::from_slice(&bytes)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<JsonRpcMessage> for JsonRpcCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: JsonRpcMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize to JSON, then delegate to LengthDelimitedCodec
        let json = serde_json::to_vec(&item)?;
        self.inner.encode(json.into(), dst)
    }
}

// Protocol versioning for compatibility checking
pub const PROTOCOL_VERSION: u32 = 1;

pub struct Handshake {
    pub version: u32,
    pub client_id: Uuid,
}
```

### 4. Connection Abstraction Layer

**Critical for dual-mode support (socket/stdio fallback)**

**Location**: `packages/daemon-client/src/connection.ts` (new)

```typescript
// Abstract interface for transport-agnostic communication
export interface Connection {
    sendRequest<T>(method: string, params?: unknown): Promise<T>;
    close(): Promise<void>;
    isConnected(): boolean;
    on(event: 'error' | 'close', handler: (err?: Error) => void): void;
}

// Connection mode selection
export enum ConnectionMode {
    Socket = 'socket',
    Stdio = 'stdio',
    Auto = 'auto',  // Try socket, fallback to stdio
}

export interface ConnectionConfig {
    mode: ConnectionMode;
    socketPath?: string;
    binaryPath?: string;
    startupTimeout?: number;
}

// Factory function
export async function createConnection(config: ConnectionConfig): Promise<Connection> {
    const mode = config.mode === ConnectionMode.Auto
        ? await detectMode(config)
        : config.mode;

    switch (mode) {
        case ConnectionMode.Socket:
            return new SocketConnection(config);
        case ConnectionMode.Stdio:
            return new StdioConnection(config);
        default:
            throw new Error(`Unknown connection mode: ${mode}`);
    }
}

async function detectMode(config: ConnectionConfig): Promise<ConnectionMode> {
    // On Windows, always use stdio
    if (process.platform === 'win32') {
        return ConnectionMode.Stdio;
    }

    // Try connecting to existing socket
    try {
        const testSocket = net.createConnection(config.socketPath);
        await waitForConnection(testSocket, 100);  // Fast timeout
        testSocket.destroy();
        return ConnectionMode.Socket;
    } catch {
        // Socket not available, could spawn daemon or fallback
        // Check env var override
        if (process.env.MAPROOM_CONNECTION_MODE === 'stdio') {
            return ConnectionMode.Stdio;
        }
        return ConnectionMode.Socket;  // Will trigger daemon spawn
    }
}
```

### 5. Socket Connection Implementation

**Location**: `packages/daemon-client/src/socket.ts` (new)

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

### 6. Stdio Connection Implementation

**Location**: `packages/daemon-client/src/stdio.ts` (refactored from existing client.ts)

```typescript
export class StdioConnection implements Connection {
    private daemonProcess: ChildProcess;
    private pendingRequests = new Map<number, PendingRequest>();
    private nextId = 1;
    private lifecycle: DaemonLifecycle;

    async connect(config: ConnectionConfig): Promise<void> {
        // Existing daemon spawn logic moved here
        this.daemonProcess = spawn(config.binaryPath, ['serve'], {
            stdio: ['pipe', 'pipe', 'pipe'],
        });

        // Reuse existing DaemonLifecycle patterns
        this.lifecycle = new DaemonLifecycle({
            restartOnFailure: true,
            maxFailures: 3,
            resetWindow: 60000,
        });

        this.setupHandlers();
    }

    async sendRequest<T>(method: string, params?: unknown): Promise<T> {
        // Existing JSON-RPC over stdio logic (unchanged)
        const id = this.nextId++;
        const message = { jsonrpc: '2.0', method, params, id };
        return this.writeAndWait(message);
    }

    // ... existing implementation
}
```

### 7. Connect-or-Spawn State Machine

**Location**: `packages/daemon-client/src/discovery.ts` (new)

**State machine for race-free daemon startup:**

```
┌─────────┐
│ Initial │
└────┬────┘
     │
     ▼
┌──────────────────┐
│ Try Connect      │────── Success ─────┐
│ (existing socket)│                    │
└────┬─────────────┘                    │
     │ Fail                             │
     ▼                                  │
┌──────────────────┐                    │
│ Acquire Lock     │                    │
│ (proper-lockfile)│                    │
└────┬─────────────┘                    │
     │ timeout: 5s                      │
     ▼                                  │
┌──────────────────┐                    │
│ Double-check     │────── Success ─────┤
│ (race condition) │                    │
└────┬─────────────┘                    │
     │ Still no socket                  │
     ▼                                  │
┌──────────────────┐                    │
│ Spawn Daemon     │                    │
│ (detached)       │                    │
└────┬─────────────┘                    │
     │                                  │
     ▼                                  │
┌──────────────────┐                    │
│ Wait for Socket  │────── Success ─────┤
│ (10s timeout)    │                    │
└────┬─────────────┘                    │
     │ Fail                             │
     ▼                                  ▼
┌──────────────────┐              ┌──────────┐
│ Release Lock     │              │ Connected│
│ Throw Error      │              └──────────┘
└──────────────────┘
```

**Implementation:**

```typescript
import { lock } from 'proper-lockfile';

export async function connectOrSpawn(config: DaemonConfig): Promise<Connection> {
    const socketPath = getSocketPath();
    const lockPath = getLockPath();  // /tmp/maproom-{uid}.lock

    // 1. Try existing daemon first (fast path)
    try {
        const conn = new SocketConnection();
        await conn.connect(socketPath);
        log.debug('Connected to existing daemon');
        return conn;
    } catch (err) {
        log.debug('No existing daemon found', { error: err.message });
    }

    // 2. Acquire lock to prevent race condition
    let release: () => Promise<void>;
    try {
        release = await lock(lockPath, {
            retries: { retries: 10, minTimeout: 100 },
            stale: 30000,  // Lock expires after 30s
        });
    } catch (err) {
        throw new DaemonStartupError('Failed to acquire lock', { cause: err });
    }

    try {
        // 3. Double-check (another process may have spawned daemon)
        try {
            const conn = new SocketConnection();
            await conn.connect(socketPath);
            log.debug('Another process spawned daemon');
            return conn;
        } catch {
            log.debug('Verified no daemon exists, will spawn');
        }

        // 4. Spawn daemon
        log.info('Spawning new daemon', { socketPath });
        const daemon = spawn(config.binaryPath, ['serve', '--socket', socketPath], {
            detached: true,
            stdio: 'ignore',
            env: { ...process.env },
        });
        daemon.unref();

        // 5. Wait for socket to appear
        await waitForSocket(socketPath, {
            timeout: config.startupTimeout || 10000,
            pollInterval: 100,
        });

        // 6. Connect
        const conn = new SocketConnection();
        await conn.connect(socketPath);
        log.info('Successfully spawned and connected to daemon');
        return conn;
    } finally {
        await release();
    }
}

async function waitForSocket(socketPath: string, options: WaitOptions): Promise<void> {
    const start = Date.now();
    while (Date.now() - start < options.timeout) {
        if (fs.existsSync(socketPath)) {
            // Socket file exists, try connecting
            try {
                const testSocket = net.createConnection(socketPath);
                await new Promise((resolve, reject) => {
                    testSocket.on('connect', resolve);
                    testSocket.on('error', reject);
                    setTimeout(() => reject(new Error('timeout')), 500);
                });
                testSocket.destroy();
                return;  // Success
            } catch {
                // Socket file exists but not ready
            }
        }
        await sleep(options.pollInterval);
    }
    throw new DaemonStartupError(`Socket not ready after ${options.timeout}ms`);
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

### Configurable SQLite Settings

**Pattern: Nested config structs following SearchConfig/EmbeddingConfig conventions**

**Location**: `crates/maproom/src/config/sqlite_config.rs` (new)

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    pub pool: PoolConfig,
    pub pragmas: PragmaConfig,
    pub retry: RetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub max_size: u32,          // Default: 10 (shared daemon needs fewer than per-daemon)
    pub min_idle: Option<u32>,  // Default: None (let r2d2 decide)
    pub connection_timeout_ms: u64,  // Default: 30000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PragmaConfig {
    pub busy_timeout_ms: u64,       // Default: 30000 (was 5000)
    pub wal_autocheckpoint: u32,    // Default: 10000 pages (~40MB)
    pub cache_size_kb: i32,         // Default: 65536 (64MB, negative = KB)
    pub mmap_size_bytes: u64,       // Default: 268435456 (256MB)
    pub synchronous: String,        // Default: "NORMAL"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,      // Default: 5
    pub base_delay_ms: u64,     // Default: 50
    pub max_delay_ms: u64,      // Default: 5000
    pub exponential: bool,      // Default: true (50, 100, 200, 400, 800)
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            pragmas: PragmaConfig::default(),
            retry: RetryConfig::default(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: None,
            connection_timeout_ms: 30000,
        }
    }
}

impl Default for PragmaConfig {
    fn default() -> Self {
        Self {
            busy_timeout_ms: 30000,
            wal_autocheckpoint: 10000,
            cache_size_kb: 65536,
            mmap_size_bytes: 268435456,
            synchronous: "NORMAL".to_string(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 50,
            max_delay_ms: 5000,
            exponential: true,
        }
    }
}

impl SqliteConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let config = Self {
            pool: PoolConfig {
                max_size: env_or("MAPROOM_SQLITE_POOL_SIZE", 10),
                min_idle: env_opt("MAPROOM_SQLITE_MIN_IDLE"),
                connection_timeout_ms: env_or("MAPROOM_SQLITE_TIMEOUT_MS", 30000),
            },
            pragmas: PragmaConfig {
                busy_timeout_ms: env_or("MAPROOM_SQLITE_BUSY_TIMEOUT_MS", 30000),
                wal_autocheckpoint: env_or("MAPROOM_SQLITE_WAL_CHECKPOINT", 10000),
                cache_size_kb: env_or("MAPROOM_SQLITE_CACHE_SIZE_KB", 65536),
                mmap_size_bytes: env_or("MAPROOM_SQLITE_MMAP_SIZE", 268435456),
                synchronous: env_or("MAPROOM_SQLITE_SYNCHRONOUS", "NORMAL".to_string()),
            },
            retry: RetryConfig {
                max_attempts: env_or("MAPROOM_SQLITE_RETRY_ATTEMPTS", 5),
                base_delay_ms: env_or("MAPROOM_SQLITE_RETRY_BASE_MS", 50),
                max_delay_ms: env_or("MAPROOM_SQLITE_RETRY_MAX_MS", 5000),
                exponential: env_or("MAPROOM_SQLITE_RETRY_EXPONENTIAL", true),
            },
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.pool.max_size == 0 {
            return Err(ConfigError::InvalidPoolSize);
        }
        if self.pragmas.busy_timeout_ms < 1000 {
            return Err(ConfigError::BusyTimeoutTooLow);
        }
        if self.retry.max_attempts == 0 {
            return Err(ConfigError::InvalidRetryConfig);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Pool size must be > 0")]
    InvalidPoolSize,
    #[error("Busy timeout should be >= 1000ms")]
    BusyTimeoutTooLow,
    #[error("Retry attempts must be > 0")]
    InvalidRetryConfig,
}

// Helper functions
fn env_or<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_opt<T: FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}
```

**Note**: Pool size changes require daemon restart (no runtime reconfiguration in MVP).

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

## Reused Components from Existing Codebase

**Critical: Extend existing patterns rather than reinventing**

### 1. Lifecycle Management (`packages/daemon-client/src/lifecycle.ts`)

**Existing capabilities:**
- Restart attempts with 60s reset window
- Exponential backoff (2^n * base delay)
- Circuit breaker after max failures
- Graceful shutdown with request draining

**Integration approach:**
- `SocketConnection` will use `DaemonLifecycle` for connection retry logic
- Configuration: `maxFailures: 3`, `resetWindow: 60000ms`
- Socket reconnection reuses same circuit breaker pattern

**Code location:** MULTICN-2005, MULTICN-2006

### 2. Error Hierarchy (`packages/daemon-client/src/errors.ts`)

**Existing pattern:**
- Base `DaemonError` class with `code` field
- Specific error types: `DaemonStartupError`, `DaemonCommunicationError`, etc.
- Stack trace capture and helper methods

**New socket-specific errors (extend existing):**

```typescript
export class SocketConnectionError extends DaemonCommunicationError {
    constructor(message: string, options?: { cause?: Error }) {
        super(message, options);
        this.code = 'SOCKET_CONNECTION_ERROR';
    }
}

export class SocketTimeoutError extends DaemonCommunicationError {
    constructor(socketPath: string, timeout: number) {
        super(`Socket connection timeout after ${timeout}ms: ${socketPath}`);
        this.code = 'SOCKET_TIMEOUT';
    }
}

export class DaemonLockError extends DaemonStartupError {
    constructor(message: string) {
        super(message);
        this.code = 'DAEMON_LOCK_ERROR';
    }
}
```

**Code location:** MULTICN-2005

### 3. RPC Protocol (`packages/daemon-client/src/rpc.ts`)

**Existing functionality:**
- JSON-RPC 2.0 message creation (`createRequest`, `createResponse`)
- Request ID generation
- Message validation

**Reuse approach:**
- Keep all existing RPC message structures unchanged
- `JsonRpcCodec` (Rust) and `SocketConnection` (TypeScript) both use same JSON structures
- Only add length-prefix framing at transport layer
- No duplication of protocol logic

**Code location:** MULTICN-2001 (Rust codec), MULTICN-2005 (TypeScript socket)

### 4. Config Pattern (`crates/maproom/src/config/`)

**Existing pattern (from `search_config.rs`, `embedding_config.rs`):**
- Nested structs for logical grouping
- `Default` trait implementation
- `from_env()` method with env var parsing
- `validate()` method with thiserror
- `serde` for serialization

**Applied to SqliteConfig:**
- `SqliteConfig { pool, pragmas, retry }` (nested)
- Follows exact same structure as `SearchConfig`
- Environment variable naming: `MAPROOM_SQLITE_*`
- Validation in `validate()` method

**Code location:** MULTICN-1002

### 5. EmbeddingService Thread Safety

**Status:** Already thread-safe (verified)
- Uses internal `RwLock` for rate limiting
- Can be shared via `Arc<EmbeddingService>` across sessions
- No changes needed for multi-client support

**Mitigation:** None required, existing code handles concurrent access

## Long-term Maintainability

### Extensibility Points

1. **New RPC methods**: Add to handler dispatch table
2. **New protocols**: Codec trait allows alternatives (gRPC future)
3. **Rate limiting**: Semaphore-based per-client limits (future, deferred)

### Migration Path

1. **Phase 1**: Socket mode opt-in via `--socket` flag
2. **Phase 2**: Socket mode default, stdio fallback automatic
3. **Phase 3**: Deprecate stdio mode (after 6+ months stability)

### Out of Scope for MVP

**Deferred features (not in Phase 1 or Phase 2):**
- SIGHUP config reload (daemon restart acceptable for MVP)
- Session metrics tracking (per-session request_count)
- Broadcast notifications (no use case identified yet)
- Runtime pool reconfiguration (restart required for config changes)

## Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `crates/maproom/src/daemon/server.rs` | Unix socket server |
| `crates/maproom/src/daemon/session.rs` | Session management |
| `crates/maproom/src/daemon/protocol.rs` | JsonRpcCodec with tokio_util framing |
| `crates/maproom/src/config/sqlite_config.rs` | Nested SqliteConfig (pool/pragmas/retry) |
| `packages/daemon-client/src/connection.ts` | Connection interface abstraction |
| `packages/daemon-client/src/socket.ts` | SocketConnection implementation |
| `packages/daemon-client/src/stdio.ts` | StdioConnection (refactored from client.ts) |
| `packages/daemon-client/src/discovery.ts` | Connect-or-spawn with proper-lockfile |

### Modified Files

| File | Changes |
|------|---------|
| `crates/maproom/src/daemon/mod.rs` | Add `run_server()` for socket mode |
| `crates/maproom/src/main.rs` | Add `--socket` flag and path option |
| `crates/maproom/src/db/sqlite/mod.rs` | Enhanced PRAGMAs, retry logic, load SqliteConfig |
| `packages/daemon-client/src/client.ts` | Use Connection interface, delegate to socket/stdio |
| `packages/daemon-client/src/errors.ts` | Add SocketConnectionError, SocketTimeoutError, DaemonLockError |
| `packages/daemon-client/src/lifecycle.ts` | Extend for socket connection management |

### Dependencies to Add

| Package | Purpose | Location |
|---------|---------|----------|
| `proper-lockfile` | Race-free lock file handling | TypeScript (daemon-client) |
| (tokio-util already present) | LengthDelimitedCodec | Rust (maproom) |

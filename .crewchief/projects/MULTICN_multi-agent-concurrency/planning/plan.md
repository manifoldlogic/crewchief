# Execution Plan: Multi-Agent Concurrency

## Overview

Two-phase implementation converting maproom to a shared daemon architecture:

1. **Phase 1: SQLite Foundation** - Enhanced configuration and retry logic
2. **Phase 2: Shared Daemon** - Unix socket server and client updates

## Phase 0: Baseline Capture

**Goal**: Establish performance baseline before changes

**Agent**: `rust-indexer-engineer`

**Tasks**:
1. Run `crewchief-maproom bench` (if exists) or create minimal benchmark script
2. Capture metrics:
   - Search latency (p50, p95, p99) for 100 queries
   - Index time for 1000 files
   - Memory usage with 1 agent, 3 agents
3. Save to `planning/performance-baseline.json`
4. Commit baseline data

**Estimated time**: 1 hour

## Phase 1: SQLite Foundation

**Goal**: Better handling of concurrent access before daemon changes

**Agent**: `rust-indexer-engineer`

### Tickets

#### MULTICN-1001: Enhanced PRAGMA Configuration

Update SQLite connection initialization with optimized settings:
- `busy_timeout = 30000` (30s, was 5s)
- `wal_autocheckpoint = 10000` (~40MB threshold)
- `cache_size = -65536` (64MB page cache)
- `mmap_size = 268435456` (256MB memory-mapped I/O)

**Files**: `crates/maproom/src/db/sqlite/mod.rs`

**Acceptance Criteria**:
- [ ] All 4 PRAGMA settings present in connection init code
- [ ] Verified by running concurrent indexing test that previously failed with SQLITE_BUSY
- [ ] Test: Spawn 3 indexing processes simultaneously, all complete without SQLITE_BUSY errors
- [ ] Log output shows increased busy_timeout value on startup

**Verification Steps**:
```bash
# Run concurrent test
cargo test --test multi_agent_indexing -- --nocapture
# Should complete without "SQLITE_BUSY" in output

# Verify pragmas in logs
cargo run --bin crewchief-maproom serve 2>&1 | grep -E "busy_timeout|wal_autocheckpoint"
```

#### MULTICN-1002: SQLite Configuration Struct

Create nested SqliteConfig following existing config patterns (SearchConfig/EmbeddingConfig):
- Nested structs: `PoolConfig`, `PragmaConfig`, `RetryConfig`
- Environment variables: `MAPROOM_SQLITE_*`
- Validation with `thiserror`
- `Default` trait, `from_env()`, `validate()` methods

**Files**: `crates/maproom/src/config/sqlite_config.rs` (new), `crates/maproom/src/db/sqlite/mod.rs`

**Acceptance Criteria**:
- [ ] SqliteConfig struct matches nested pattern from SearchConfig
- [ ] Verified by setting env var `MAPROOM_SQLITE_BUSY_TIMEOUT_MS=60000` and confirming log output shows 60s timeout
- [ ] Test: Unit test that `SqliteConfig::from_env()` parses all env vars correctly
- [ ] Test: Unit test that `validate()` rejects invalid values (pool size 0, timeout < 1000)

**Verification Steps**:
```bash
# Test env var loading
MAPROOM_SQLITE_BUSY_TIMEOUT_MS=60000 cargo run --bin crewchief-maproom serve
# Check logs show "busy_timeout: 60000ms"

# Run unit tests
cargo test --lib sqlite_config
```

#### MULTICN-1003: Write Retry Logic

Implement `write_with_retry()` wrapper:
- 5 attempts with exponential backoff (50ms → 100ms → 200ms → 400ms → 800ms)
- Catch SQLITE_BUSY and DatabaseLocked errors
- Log warnings on retry for observability

**Files**: `crates/maproom/src/db/sqlite/mod.rs`

**Acceptance Criteria**:
- [ ] `write_with_retry()` method exists and is used for all write operations
- [ ] Verified by unit test that simulates SQLITE_BUSY and confirms exponential backoff retry behavior
- [ ] Test: Mock test shows 5 attempts with delays: 50, 100, 200, 400, 800 ms
- [ ] Test: After 5 failures, returns error (doesn't retry forever)
- [ ] Logs show "SQLITE_BUSY, retrying (attempt N/5)" on contention

**Verification Steps**:
```bash
# Run unit test with BUSY simulation
cargo test --lib write_with_retry -- --nocapture
# Should see 5 retry attempts with increasing delays

# Integration test
cargo test --test sqlite_retry_integration
```

### Phase 1 Testing

- Unit tests for retry logic edge cases
- Integration test: concurrent writes with busy simulation

---

## Phase 2: Shared Daemon Architecture

**Goal**: Single daemon serves all agents via Unix socket

### Phase 2a: Rust Socket Server

**Agent**: `rust-indexer-engineer`, `process-management-specialist`

#### MULTICN-2001: JSON-RPC Codec with Length Framing

Implement binary framing for JSON-RPC messages using tokio_util:
- Use `tokio_util::codec::LengthDelimitedCodec` (battle-tested, not custom implementation)
- 4-byte big-endian length prefix
- Maximum message size: 10MB (enforced by codec)
- Protocol version field for compatibility checking (version 1)
- Reuse existing JSON-RPC message structures (no duplication)

**Files**: `crates/maproom/src/daemon/protocol.rs` (new)

**Acceptance Criteria**:
- [ ] `JsonRpcCodec` uses `LengthDelimitedCodec::builder()` with max_frame_length(10MB)
- [ ] Test: Encode/decode round-trip preserves JSON-RPC message
- [ ] Test: Partial read simulation (split message) doesn't corrupt data
- [ ] Test: Oversized message (>10MB) is rejected with error
- [ ] Protocol version constant defined (PROTOCOL_VERSION = 1)
- [ ] Handshake struct includes version field

**Verification Steps**:
```bash
cargo test --lib protocol -- --nocapture
# Should pass: round_trip, partial_reads, oversized_reject tests
```

#### MULTICN-2002: Session Management

Create session tracking infrastructure:
- Session struct: UUID, connected_at timestamp, response_tx channel
- SessionRegistry with DashMap for concurrent access
- Atomic counter (AtomicUsize) for idle timeout tracking
- Session lifecycle: register (increments counter), unregister (decrements counter)
- NO broadcast capability (deferred, no use case yet)
- NO per-session metrics (deferred to post-MVP)

**Files**: `crates/maproom/src/daemon/session.rs` (new)

**Acceptance Criteria**:
- [ ] Session struct has only essential fields (no request_count metric)
- [ ] SessionRegistry uses DashMap for lock-free concurrent access
- [ ] active_count() method returns AtomicUsize value
- [ ] register() logs connection and increments counter
- [ ] unregister() logs disconnection and decrements counter
- [ ] Test: Concurrent register/unregister from 10 threads maintains accurate count

**Verification Steps**:
```bash
cargo test --lib session -- --nocapture
# Should pass: concurrent_registration, active_count_tracking tests
```

#### MULTICN-2003: Unix Socket Server

Implement socket-based daemon server:
- UnixListener with 0600 permissions
- Accept loop with per-client task spawning
- Shared state via Arc<DaemonState>
- PID file with O_EXCL + flock

**Files**: `crates/maproom/src/daemon/server.rs` (new), `crates/maproom/src/daemon/mod.rs`

#### MULTICN-2004: Daemon Lifecycle Management

Implement process lifecycle features:
- Idle timeout (5 minutes with no clients)
- Graceful shutdown on SIGTERM
- PID file cleanup
- `--socket` flag for serve command

**Files**: `crates/maproom/src/daemon/server.rs`, `crates/maproom/src/main.rs`

### Phase 2b: TypeScript Client Updates

**Agent**: `vscode-extension-specialist`

#### MULTICN-2005: Socket Connection Class

Create SocketConnection implementing Connection interface:
- Length-prefixed message reading
- Buffer management for partial reads
- Request/response multiplexing

**Files**: `packages/daemon-client/src/socket.ts` (new)

#### MULTICN-2006: Connect-or-Spawn Logic

Implement daemon discovery and auto-start:
- Try existing socket first (fast path)
- Use `proper-lockfile` library for race-free coordination
- Lock file: `/tmp/maproom-{uid}.lock` (separate from `.sock`)
- Double-check pattern (check → lock → check again → spawn)
- Spawn daemon detached if needed
- Wait for socket with timeout (10s)
- Reuse DaemonLifecycle patterns for retry logic

**Files**: `packages/daemon-client/src/discovery.ts` (new)

**Dependencies**: Add `proper-lockfile` to package.json

**Acceptance Criteria**:
- [ ] Uses `proper-lockfile` library (not custom lock implementation)
- [ ] Lock file is `/tmp/maproom-{uid}.lock` (distinct from socket path)
- [ ] Implements double-check pattern (try connect before and after lock)
- [ ] Spawns with `detached: true, stdio: 'ignore'` and calls `daemon.unref()`
- [ ] Waits up to 10s for socket to become available
- [ ] Test: 5 concurrent clients calling connectOrSpawn() only create 1 daemon
- [ ] Test: PID verification shows only 1 maproom daemon process after concurrent starts

**Verification Steps**:
```bash
pnpm test daemon-client -- connect-or-spawn
# Should pass: concurrent_spawn_race test
```

#### MULTICN-2007: Client Connection Mode Abstraction

Update DaemonClient to support multiple connection modes:
- Connection interface: `sendRequest()`, `close()`, `isConnected()`, `on()`
- SocketConnection and StdioConnection implement interface
- `MAPROOM_CONNECTION_MODE` environment variable (socket|stdio|auto)
- Auto-detect: Windows → stdio, Unix → try socket then fallback stdio
- Reuse existing DaemonLifecycle for connection management
- Extend existing error hierarchy (SocketConnectionError, SocketTimeoutError, DaemonLockError)

**Files**:
- `packages/daemon-client/src/connection.ts` (new - interface)
- `packages/daemon-client/src/client.ts` (use Connection interface)
- `packages/daemon-client/src/stdio.ts` (new - refactor from client.ts)
- `packages/daemon-client/src/errors.ts` (add socket errors)

**Acceptance Criteria**:
- [ ] Connection interface defined with 4 methods (sendRequest, close, isConnected, on)
- [ ] SocketConnection and StdioConnection both implement Connection interface
- [ ] Windows platform automatically uses stdio mode
- [ ] `MAPROOM_CONNECTION_MODE=stdio` forces stdio mode on Unix
- [ ] Auto mode tries socket, falls back to stdio on failure
- [ ] Existing error classes extended (not replaced)
- [ ] Test: Both connection modes pass identical test suite
- [ ] Test: Mode detection logic works correctly on different platforms

**Verification Steps**:
```bash
# Test stdio mode explicitly
MAPROOM_CONNECTION_MODE=stdio pnpm test daemon-client
# Should pass all existing tests

# Test socket mode explicitly
MAPROOM_CONNECTION_MODE=socket pnpm test daemon-client
# Should pass all existing tests

# Test auto-detection
pnpm test daemon-client -- connection-mode
```

### Phase 2 Testing

- Unit: Protocol codec, session management
- Integration: Connect-or-spawn race condition
- Integration: Multi-client request routing
- Integration: Graceful shutdown with in-flight requests

---

## Agent Assignments

| Ticket | Primary Agent | Secondary |
|--------|--------------|-----------|
| MULTICN-1001 | rust-indexer-engineer | - |
| MULTICN-1002 | rust-indexer-engineer | - |
| MULTICN-1003 | rust-indexer-engineer | - |
| MULTICN-2001 | rust-indexer-engineer | - |
| MULTICN-2002 | rust-indexer-engineer | - |
| MULTICN-2003 | rust-indexer-engineer | process-management-specialist |
| MULTICN-2004 | process-management-specialist | rust-indexer-engineer |
| MULTICN-2005 | vscode-extension-specialist | - |
| MULTICN-2006 | vscode-extension-specialist | process-management-specialist |
| MULTICN-2007 | vscode-extension-specialist | - |

## Dependencies

```
Phase 1 (can run in parallel):
MULTICN-1001 ──┬── MULTICN-1002 ──── MULTICN-1003
               │
Phase 2a (sequential):
               │
MULTICN-2001 ──┴── MULTICN-2002 ──── MULTICN-2003 ──── MULTICN-2004
                                                            │
Phase 2b (after 2001):                                      │
MULTICN-2005 ──── MULTICN-2006 ──── MULTICN-2007 ──────────┘
```

## Security Checkpoints

- **After MULTICN-2003**: Verify socket permissions (0600)
- **After MULTICN-2003**: Verify PID file uses O_EXCL
- **After MULTICN-2001**: Verify message size limit enforced
- **Final**: Security-focused code review

## Quality Gates

### Phase 1 Complete When
- [ ] Enhanced PRAGMAs in place
- [ ] SqliteConfig struct with env vars
- [ ] Retry logic with tests
- [ ] No regressions in existing tests

### Phase 2 Complete When
- [ ] Socket server accepts connections
- [ ] Multi-client concurrent requests work
- [ ] Connect-or-spawn race condition test passes
- [ ] Graceful shutdown test passes
- [ ] Existing daemon-client tests pass with both modes

## Rollback Plan

1. **Socket mode is opt-in**: Default remains stdio until proven
2. **Environment variable override**: `MAPROOM_CONNECTION_MODE=stdio`
3. **No breaking changes**: stdio mode continues to work
4. **Quick disable**: Remove `--socket` flag returns to old behavior

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| SQLITE_BUSY errors | 0 in normal use | 10-min stress test with 3 agents |
| Memory (3 agents) | <150MB | Process monitoring |
| Startup latency | <100ms | First request timing |
| Request latency delta | <5ms | Socket vs stdio comparison |

## Out of Scope for MVP

**Explicitly deferred to post-MVP / future phases:**

### Features Not Included
- **SIGHUP config reload**: Daemon restart is acceptable for rare config changes
- **Session metrics tracking**: Per-session request_count, bandwidth stats
- **Broadcast notifications**: No use case identified (could be added for cache invalidation later)
- **Runtime pool reconfiguration**: Pool size changes require daemon restart
- **TCP fallback for Windows**: Windows users use stdio mode (`MAPROOM_CONNECTION_MODE=stdio`)
- **Authentication/authorization**: Not needed for single-user workstation use
- **Rate limiting per client**: Self-DoS not a concern for personal dev tools
- **Audit logging**: Operational feature, not MVP-critical
- **Manual daemon management**: No `maproom daemon start/stop/status` commands in MVP

### Future Enhancements (Documented for Later)
- **Windows named pipes**: Alternative to Unix sockets for native Windows support
- **Protocol versioning negotiation**: More sophisticated version checking beyond major version
- **Connection pooling per worktree**: If cross-worktree operations become common
- **Metrics endpoint**: HTTP endpoint for Prometheus-style metrics
- **Configuration hot-reload**: SIGHUP or file watcher for config changes

### Rationale
These features add complexity without addressing the core problem (concurrent agent SQLite contention). They can be added incrementally based on user feedback after MVP stabilizes.

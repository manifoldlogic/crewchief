# Execution Plan: Multi-Agent Concurrency

## Overview

Two-phase implementation converting maproom to a shared daemon architecture:

1. **Phase 1: SQLite Foundation** - Enhanced configuration and retry logic
2. **Phase 2: Shared Daemon** - Unix socket server and client updates

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

#### MULTICN-1002: SQLite Configuration Struct

Create configurable SqliteConfig with environment variable support:
- `MAPROOM_SQLITE_READ_POOL_SIZE`
- `MAPROOM_SQLITE_WRITE_POOL_SIZE`
- `MAPROOM_SQLITE_BUSY_TIMEOUT_MS`

**Files**: `crates/maproom/src/db/sqlite/mod.rs`, `crates/maproom/src/config/`

#### MULTICN-1003: Write Retry Logic

Implement `write_with_retry()` wrapper:
- 5 attempts with exponential backoff (50ms → 800ms)
- Catch SQLITE_BUSY and DatabaseLocked errors
- Log warnings on retry for observability

**Files**: `crates/maproom/src/db/sqlite/mod.rs`

### Phase 1 Testing

- Unit tests for retry logic edge cases
- Integration test: concurrent writes with busy simulation

---

## Phase 2: Shared Daemon Architecture

**Goal**: Single daemon serves all agents via Unix socket

### Phase 2a: Rust Socket Server

**Agent**: `rust-indexer-engineer`, `process-management-specialist`

#### MULTICN-2001: Length-Prefixed Protocol

Implement binary framing for JSON-RPC messages:
- 4-byte big-endian length prefix
- Message size limit (10MB)
- Codec implementation using tokio_util

**Files**: `crates/maproom/src/daemon/protocol.rs` (new)

#### MULTICN-2002: Session Management

Create session tracking infrastructure:
- Session struct with UUID, timestamps, metrics
- SessionRegistry with DashMap for concurrent access
- Session lifecycle (register, unregister, broadcast)

**Files**: `crates/maproom/src/daemon/session.rs` (new)

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
- Try existing socket first
- Lock file to prevent race condition
- Spawn daemon detached if needed
- Wait for socket with timeout

**Files**: `packages/daemon-client/src/discovery.ts` (new)

#### MULTICN-2007: Client Connection Mode Abstraction

Update DaemonClient to support multiple connection modes:
- Connection interface abstraction
- `MAPROOM_CONNECTION_MODE` environment variable
- Auto-detect with socket preference, stdio fallback

**Files**: `packages/daemon-client/src/client.ts`, `packages/daemon-client/src/lifecycle.ts`

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

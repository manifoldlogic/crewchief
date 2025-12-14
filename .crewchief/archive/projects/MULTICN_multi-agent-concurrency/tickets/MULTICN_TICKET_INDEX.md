# MULTICN Ticket Index

## Overview

Multi-Agent Concurrency project - Convert maproom to shared daemon architecture for handling concurrent agent access.

**Total Tickets**: 11 (1 Phase 0, 3 Phase 1, 7 Phase 2)

## Phase 0: Baseline Capture (1 ticket)

**Goal**: Establish performance baseline before changes

| Ticket | Title | Agent | Est. Time |
|--------|-------|-------|-----------|
| [MULTICN-0001](MULTICN-0001_performance-baseline-capture.md) | Capture Performance Baseline | rust-indexer-engineer | 1 hour |

**Phase 0 Deliverable**: `planning/performance-baseline.json` with search latency, index time, and memory metrics.

---

## Phase 1: SQLite Foundation (3 tickets)

**Goal**: Better handling of concurrent access before daemon changes

| Ticket | Title | Agent | Est. Time | Dependencies |
|--------|-------|-------|-----------|--------------|
| [MULTICN-1001](MULTICN-1001_enhanced-pragma-configuration.md) | Enhanced PRAGMA Configuration | rust-indexer-engineer | 2-3 hours | MULTICN-0001 |
| [MULTICN-1002](MULTICN-1002_sqlite-configuration-struct.md) | SQLite Configuration Struct | rust-indexer-engineer | 3-4 hours | MULTICN-1001 |
| [MULTICN-1003](MULTICN-1003_write-retry-logic.md) | Write Retry Logic | rust-indexer-engineer | 4-5 hours | MULTICN-1002 |

**Phase 1 Deliverables**:
- Enhanced SQLite PRAGMAs (busy_timeout=30s, WAL checkpoint, cache, mmap)
- Configurable SqliteConfig with environment variable support
- Automatic retry logic with exponential backoff for SQLITE_BUSY errors

**Acceptance for Phase 1 Complete**:
- [ ] Enhanced PRAGMAs in place
- [ ] SqliteConfig struct with env vars
- [ ] Retry logic with tests
- [ ] No regressions in existing tests

---

## Phase 2: Shared Daemon Architecture (7 tickets)

**Goal**: Single daemon serves all agents via Unix socket

### Phase 2a: Rust Socket Server (4 tickets)

| Ticket | Title | Agents | Est. Time | Dependencies |
|--------|-------|--------|-----------|--------------|
| [MULTICN-2001](MULTICN-2001_json-rpc-codec-length-framing.md) | JSON-RPC Codec with Length Framing | rust-indexer-engineer | 3-4 hours | Phase 1 complete |
| [MULTICN-2002](MULTICN-2002_session-management.md) | Session Management | rust-indexer-engineer | 3-4 hours | MULTICN-2001 |
| [MULTICN-2003](MULTICN-2003_unix-socket-server.md) | Unix Socket Server | rust-indexer-engineer, process-management-specialist | 5-6 hours | MULTICN-2001, MULTICN-2002 |
| [MULTICN-2004](MULTICN-2004_daemon-lifecycle-management.md) | Daemon Lifecycle Management | process-management-specialist, rust-indexer-engineer | 4-5 hours | MULTICN-2003 |

**Phase 2a Deliverables**:
- Length-prefixed JSON-RPC codec using tokio_util
- Session tracking with DashMap and atomic counters
- Unix socket server with PID file locking
- Idle timeout and graceful shutdown on SIGTERM

### Phase 2b: TypeScript Client Updates (3 tickets)

| Ticket | Title | Agent | Est. Time | Dependencies |
|--------|-------|-------|-----------|--------------|
| [MULTICN-2005](MULTICN-2005_socket-connection-class.md) | Socket Connection Class | vscode-extension-specialist | 4-5 hours | MULTICN-2001 |
| [MULTICN-2006](MULTICN-2006_connect-or-spawn-logic.md) | Connect-or-Spawn Logic | vscode-extension-specialist, process-management-specialist | 5-6 hours | MULTICN-2005 |
| [MULTICN-2007](MULTICN-2007_client-connection-mode-abstraction.md) | Client Connection Mode Abstraction | vscode-extension-specialist | 4-5 hours | MULTICN-2005, MULTICN-2006 |

**Phase 2b Deliverables**:
- SocketConnection class with length-prefixed framing
- Connect-or-spawn with proper-lockfile coordination
- Connection interface abstraction (socket/stdio/auto modes)
- Dual-mode compatibility testing

**Acceptance for Phase 2 Complete**:
- [ ] Socket server accepts connections
- [ ] Multi-client concurrent requests work
- [ ] Connect-or-spawn race condition test passes
- [ ] Graceful shutdown test passes
- [ ] Existing daemon-client tests pass with both modes (stdio and socket)

---

## Dependency Graph

```
Phase 0:
MULTICN-0001 (Baseline Capture)
      │
      ▼
Phase 1 (Sequential):
MULTICN-1001 (Enhanced PRAGMAs)
      │
      ▼
MULTICN-1002 (SQLite Config)
      │
      ▼
MULTICN-1003 (Write Retry)
      │
      ▼
Phase 2a (Sequential):
MULTICN-2001 (JSON-RPC Codec) ────┐
      │                           │
      ▼                           │
MULTICN-2002 (Session Management) │
      │                           │
      ▼                           │
MULTICN-2003 (Socket Server)      │
      │                           │
      ▼                           │
MULTICN-2004 (Lifecycle)          │
      │                           │
      ▼                           │
Phase 2b (after 2001):            │
MULTICN-2005 (Socket Connection) ◄┘
      │
      ▼
MULTICN-2006 (Connect-or-Spawn)
      │
      ▼
MULTICN-2007 (Connection Mode Abstraction)
```

## Ticket Status Legend

- [ ] **Task completed** - Implementation done, acceptance criteria met
- [ ] **Tests pass** - Tests executed and passing (or N/A for docs-only)
- [ ] **Verified** - Checked by verify-ticket agent

## Key Milestones

1. **Phase 0 Complete**: Baseline metrics captured
2. **Phase 1 Complete**: SQLite optimizations reduce SQLITE_BUSY errors
3. **Phase 2a Complete**: Socket server operational
4. **Phase 2b Complete**: Clients can connect via socket
5. **Project Complete**: All quality gates pass (see below)

## Quality Gates

### After Phase 1
- No SQLITE_BUSY errors in stress test (3 agents, 10 minutes)
- Existing tests still pass
- Configuration can be loaded from environment variables

### After Phase 2
- Socket mode works with multiple clients
- Connect-or-spawn prevents race conditions (5 concurrent clients → 1 daemon)
- Graceful shutdown completes in-flight requests
- Stdio mode still works (backward compatibility)
- Memory usage <150MB with 3 agents (vs ~300MB before)

## Security Checkpoints

- **After MULTICN-2003**: Verify socket permissions (0600)
- **After MULTICN-2003**: Verify PID file uses O_EXCL + flock
- **After MULTICN-2001**: Verify message size limit enforced (10MB)
- **Final**: Security-focused code review

## Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|--------------------|
| SQLITE_BUSY errors | 0 in normal use | 10-min stress test with 3 agents |
| Memory (3 agents) | <150MB total | Process monitoring (RSS) |
| Startup latency | <100ms | First request timing |
| Request latency delta | <5ms | Socket vs stdio comparison |
| Connect-or-spawn race | 1 daemon only | 5 concurrent clients spawn count |

## Out of Scope for MVP

**Explicitly deferred to post-MVP:**
- SIGHUP config reload
- Per-session metrics tracking (request_count, bandwidth)
- Broadcast notifications
- Runtime pool reconfiguration
- TCP fallback for Windows (Windows uses stdio mode)
- Authentication/authorization
- Rate limiting per client
- Audit logging
- Manual daemon management commands (start/stop/status)

See [plan.md](../planning/plan.md) for full rationale.

## Rollback Plan

1. **Socket mode is opt-in**: Default remains stdio until proven
2. **Environment variable override**: `MAPROOM_CONNECTION_MODE=stdio`
3. **No breaking changes**: stdio mode continues to work
4. **Quick disable**: Remove `--socket` flag returns to old behavior

## Notes

- All Rust tickets include unit tests and integration tests
- All TypeScript tickets include unit tests
- Dual-mode testing ensures stdio fallback works
- Platform-specific behavior documented (Windows→stdio, Unix→auto)

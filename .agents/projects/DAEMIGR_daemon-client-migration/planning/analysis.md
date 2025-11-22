# DAEMIGR Project Analysis

## Executive Summary

The DAEMIGR (Daemon Client Migration) project completes the MAPDAEMON architecture by migrating TypeScript clients from process-spawning to daemon-based communication. This migration realizes 20-50x performance improvements for MCP server search requests while maintaining backward compatibility and system reliability.

## Problem Definition

### Current State

The MAPDAEMON project successfully implemented a high-performance Rust daemon with JSON-RPC over stdio, connection pooling, and optimized search execution. However, **TypeScript clients still spawn new processes for each search request**, preventing us from realizing the performance benefits of the daemon architecture.

**Performance Impact:**
- **Current MCP Server:** 160-400ms per search (process spawn + DB connection + query)
- **With Daemon (warm):** 10-50ms per search (3-40x improvement)
- **Pain Point:** AI assistants making multiple queries experience cumulative delays

### Problem Scope

**In Scope:**
- MCP Server search tool migration (HIGH priority, high-impact)
- New `daemon-client` package for TypeScript/JavaScript applications
- Process lifecycle management and crash recovery
- JSON-RPC protocol handling and error recovery

**Out of Scope (Phase 1):**
- VSCode extension `watch` command (already optimal, long-running)
- VSCode extension `branch-watch` command (already optimal, long-running)
- VSCode extension `scan` command (MEDIUM priority, can migrate in Phase 2)
- CLI usage (direct binary execution is appropriate)

### Root Cause Analysis

**Why does this problem exist?**

1. **Historical Context:** Initial implementation used process spawning (simple, proven)
2. **Evolution:** MAPDAEMON added daemon capability but didn't migrate clients
3. **Priority:** Daemon implementation prioritized over client migration
4. **Complexity:** Client migration requires careful lifecycle management

**Why hasn't it been solved?**

- Daemon implementation was Phase 1 (foundation)
- Client migration is Phase 2 (realization)
- Requires new daemon-client package (reusable abstraction)
- Integration testing needed for production confidence

## Existing Solutions Analysis

### Industry Approaches

#### Language Server Protocol (LSP) Clients
**Example:** VSCode language client library

**Approach:**
- Long-running server process per workspace
- JSON-RPC over stdin/stdout
- Client manages server lifecycle
- Health checks and auto-restart

**Lessons Learned:**
- ✅ Process-per-instance is proven pattern
- ✅ JSON-RPC over stdio is simple and secure
- ✅ Auto-restart with exponential backoff works well
- ⚠️ Need robust shutdown handling (signals, timeouts)

#### Database Connection Pooling
**Example:** `pg-pool`, `mysql2/promise`

**Approach:**
- Single connection pool reused across requests
- Lazy initialization on first query
- Health checks before query execution
- Circuit breaker on repeated failures

**Lessons Learned:**
- ✅ Lazy initialization reduces startup overhead
- ✅ Health checks prevent failed requests
- ✅ Pool exhaustion needs backpressure handling
- ⚠️ Connection leaks require explicit cleanup

#### Process Managers (PM2, systemd)
**Example:** PM2 for Node.js applications

**Approach:**
- External process supervisor
- Auto-restart on crash
- Resource limits and monitoring
- Log aggregation

**Lessons Learned:**
- ✅ External supervision is more robust than in-process
- ⚠️ Complex deployment (requires PM2 installed)
- ❌ Not suitable for library-embedded daemon

### CrewChief Existing Patterns

#### VSCode Extension Orchestrator
**Location:** `packages/vscode-extension/src/orchestrator.ts`

**Pattern:**
```typescript
class Orchestrator {
  private processes: Map<string, ChildProcess>

  async spawn(command: string, args: string[]): Promise<ChildProcess>
  async kill(pid: number): Promise<void>

  // Health monitoring
  async checkHealth(): Promise<boolean>

  // Cleanup on extension deactivate
  async cleanup(): Promise<void>
}
```

**Strengths:**
- ✅ Process lifecycle management proven in production
- ✅ Cleanup on extension shutdown prevents leaks
- ✅ Health monitoring detects stale processes

**Gaps for Daemon Use Case:**
- ⚠️ No auto-restart logic (manual recovery)
- ⚠️ No JSON-RPC request/response matching
- ⚠️ No backpressure handling for concurrent requests

#### MCP Server Process Spawning
**Location:** `packages/maproom-mcp/src/utils/process.ts`

**Pattern:**
```typescript
async function trySpawnWithCandidates(
  candidates: string[],
  args: string[],
  options: SpawnOptions
): Promise<{ stdout: string; stderr: string }>
```

**Strengths:**
- ✅ Binary discovery logic (multi-platform paths)
- ✅ Error handling for spawn failures
- ✅ Timeout handling for hung processes

**Gaps for Daemon Use Case:**
- ❌ No process reuse (spawn per request)
- ❌ No state management (stateless)
- ⚠️ Should be deprecated but kept for VSCode scan usage

## Stakeholder Analysis

### AI Assistant Users (via MCP Server)

**Impact:** HIGH POSITIVE
- **Benefit:** 3-40x faster search responses
- **User Experience:** More responsive multi-query interactions
- **Pain Point Solved:** Cumulative latency from multiple searches

**Risks:**
- 🟢 Minimal: Migration is transparent to users
- 🟡 Fallback available if daemon unstable

**Needs:**
- Reliability (no regressions)
- Performance (realize promised improvements)
- Transparency (no breaking changes)

### Developers (CrewChief Contributors)

**Impact:** POSITIVE (long-term), NEUTRAL (short-term)
- **Benefit:** Better performance for development workflows
- **Benefit:** Cleaner architecture (separation of concerns)
- **Challenge:** Slightly more complex debugging (daemon indirection)

**Risks:**
- 🟡 Learning curve for daemon architecture
- 🟡 Debugging requires understanding IPC

**Needs:**
- Clear documentation (architecture, debugging)
- Good error messages (actionable, contextual)
- Development tools (logs, health checks)

### DevOps/Deployment

**Impact:** NEUTRAL (Phase 1 - MCP server is already deployed)
- **Benefit:** No deployment complexity change (daemon embedded)
- **Benefit:** Better resource usage (connection pooling)

**Risks:**
- 🟢 Minimal: MCP server deployment unchanged
- 🟡 Need monitoring for daemon crashes

**Needs:**
- Observability (structured logs, metrics)
- Deployment docs (environment variables, troubleshooting)
- Runbooks (restart procedures, debugging)

## Technical Dependencies

### Prerequisites (Already Complete)

✅ **MAPDAEMON Implementation**
- Rust daemon with JSON-RPC server (`crewchief-maproom serve`)
- Protocol: JSON-RPC 2.0 over stdin/stdout
- Status: Production-ready, tested

✅ **PostgreSQL Schema Stable**
- No schema changes required for migration
- Connection pooling already implemented in daemon
- Status: Stable, no breaking changes expected

✅ **Binary Distribution**
- Platform-specific binaries: `packages/cli/bin/{platform}/crewchief-maproom`
- Binary discovery logic: `packages/maproom-mcp/src/utils/process.ts`
- Status: Working, used by current spawning approach

### External Dependencies

**Runtime:**
- **Node.js:** >=18 (for daemon-client package)
- **TypeScript:** >=5.0 (for type safety)
- **PostgreSQL:** >=13 (for maproom database)

**Development:**
- **Vitest:** Testing framework (already in monorepo)
- **ESLint/Prettier:** Code quality (already configured)
- **pnpm:** Package manager (already in use)

### Internal Dependencies

**New Package:**
- `packages/daemon-client/` - Core library for daemon communication
  - Used by: `maproom-mcp`, potentially `vscode-extension` (Phase 2)
  - Exports: `DaemonClient` class, error types, config interfaces

**Modified Packages:**
- `packages/maproom-mcp/` - MCP server integration
  - Changes: `src/tools/search.ts` (replace spawning with daemon)
  - Changes: `src/daemon.ts` (new, singleton management)
  - Dependency: `daemon-client` package

**Preserved Code:**
- `packages/maproom-mcp/src/utils/process.ts` - Keep for VSCode scan usage
  - Mark as deprecated for MCP usage
  - Still needed by VSCode extension

## Risk Assessment

### Technical Risks

#### Risk: Daemon Process Management Complexity
- **Likelihood:** Medium
- **Impact:** Medium (failed requests, poor UX)
- **Symptoms:** Zombie processes, resource leaks, hung requests
- **Mitigation:**
  - Adopt proven patterns from VSCode extension orchestrator
  - Comprehensive process lifecycle tests
  - Explicit cleanup in all exit paths (normal, error, signal)
  - Process health monitoring with timeout-based detection

#### Risk: JSON-RPC Protocol Incompatibilities
- **Likelihood:** Low (protocol already tested in MAPDAEMON)
- **Impact:** High (broken search, critical functionality)
- **Symptoms:** Parse errors, mismatched responses, protocol violations
- **Mitigation:**
  - Integration tests covering all RPC methods
  - Protocol version checking (future-proofing)
  - Strict JSON schema validation
  - Comprehensive error handling for malformed responses

#### Risk: Resource Leaks (Memory, File Descriptors)
- **Likelihood:** Low
- **Impact:** Medium (degraded performance over time)
- **Symptoms:** Growing memory usage, file descriptor exhaustion
- **Mitigation:**
  - Automated leak detection tests (1000+ request cycles)
  - Explicit resource cleanup (streams, processes, timers)
  - Process termination guarantees (SIGTERM → SIGKILL escalation)
  - Health monitoring for abnormal resource usage

#### Risk: Concurrent Request Handling
- **Likelihood:** Medium (MCP servers may receive concurrent requests)
- **Impact:** Medium (race conditions, request/response mismatches)
- **Symptoms:** Wrong responses, timeouts, data corruption
- **Mitigation:**
  - Request ID-based response matching
  - Concurrent request tests (10, 50, 100 simultaneous)
  - Promise-based async handling (no callback hell)
  - Request queue overflow handling (backpressure)

### Operational Risks

#### Risk: Daemon Crashes in Production
- **Likelihood:** Low (tested Rust implementation)
- **Impact:** Medium (degraded performance, auto-restart overhead)
- **Symptoms:** Search failures, timeout errors, log spam
- **Mitigation:**
  - Auto-restart with exponential backoff
  - Circuit breaker after max restart attempts (5)
  - Comprehensive logging (crash dumps, stack traces)
  - Graceful degradation (optional fallback to spawning)

#### Risk: Silent Failures (Daemon Unhealthy but Running)
- **Likelihood:** Low
- **Impact:** High (failed searches with no auto-recovery)
- **Symptoms:** Timeouts, error responses, stale data
- **Mitigation:**
  - Health check before each request (lightweight ping)
  - Timeout-based liveness detection
  - Restart on consecutive failures (3 strikes)
  - Observability (structured logs, error counts)

#### Risk: Deployment Complexity Increase
- **Likelihood:** Low (Phase 1 targets MCP server, already deployed)
- **Impact:** Low (minimal deployment changes)
- **Symptoms:** Deployment failures, environment misconfiguration
- **Mitigation:**
  - Environment variable validation at startup
  - Clear error messages for missing config
  - Deployment documentation (runbooks, troubleshooting)
  - Backward compatibility (keep old code paths initially)

## Success Metrics

### Performance Metrics

**Cold Start Latency (First Request):**
- **Target:** 200-550ms
- **Baseline:** 160-400ms (current spawning)
- **Measurement:** Time from request to first response
- **Success Criteria:** Within ±50ms of baseline (acceptable tradeoff)

**Warm Request Latency (Subsequent Requests):**
- **Target:** < 10-50ms
- **Baseline:** 160-400ms (current spawning)
- **Measurement:** Time from request to response (daemon already running)
- **Success Criteria:** 20-50x improvement (< 50ms consistently)

**Throughput (Concurrent Requests):**
- **Target:** 50+ req/s
- **Baseline:** ~5-10 req/s (spawning bottleneck)
- **Measurement:** Requests per second under sustained load
- **Success Criteria:** Handle Claude Code typical usage (5-10 concurrent queries)

**Resource Usage:**
- **Memory:** < 100MB additional (daemon + client overhead)
- **CPU:** < 15% average (idle daemon should be 0%)
- **Connections:** 1 pool vs N connections (improvement)
- **Success Criteria:** No memory leaks over 1000 requests, stable resource usage

### Quality Metrics

**Test Coverage:**
- **Target:** > 80% for daemon-client package
- **Measurement:** Line coverage via vitest
- **Success Criteria:** All critical paths covered (lifecycle, RPC, errors)

**Integration Test Pass Rate:**
- **Target:** 100%
- **Measurement:** CI/CD pipeline success rate
- **Success Criteria:** Zero flaky tests, reliable builds

**Regression Issues:**
- **Target:** 0 regressions in MCP server functionality
- **Measurement:** Existing integration tests must pass
- **Success Criteria:** All current search functionality preserved

### Adoption Metrics

**MCP Server Migration:**
- **Target:** 100% (all search requests via daemon)
- **Measurement:** Code deployment, feature flags
- **Success Criteria:** No spawning code paths in MCP search tool

**Daemon Stability:**
- **Target:** < 1% restart rate
- **Measurement:** Restart attempts / total requests
- **Success Criteria:** Daemon stays healthy under normal usage

**User-Reported Issues:**
- **Target:** 0 critical issues
- **Measurement:** GitHub issues, user feedback
- **Success Criteria:** No performance regressions, no data corruption

## Alternative Approaches

### Alternative 1: Keep Process Spawning
**Description:** Don't migrate, keep current spawning approach

**Pros:**
- ✅ Zero migration work
- ✅ Proven approach (working today)
- ✅ Simple mental model

**Cons:**
- ❌ Performance remains poor (160-400ms per request)
- ❌ No connection pooling (N connections)
- ❌ Defeats purpose of MAPDAEMON investment

**Decision:** **REJECTED** - Fails to realize MAPDAEMON benefits, wastes prior investment

### Alternative 2: Shared Daemon (Multiple Clients)
**Description:** Single daemon shared by MCP server, VSCode extension, CLI

**Pros:**
- ✅ Better resource usage (one daemon for all clients)
- ✅ Shared connection pool
- ✅ Central monitoring point

**Cons:**
- ❌ Complex IPC (need socket/named pipe coordination)
- ❌ Lifecycle coupling (one client crash affects others)
- ❌ Port management (who owns the daemon?)
- ❌ Security concerns (multiple processes accessing same socket)

**Decision:** **REJECTED for Phase 1** - Too complex, can revisit in Phase 2 if needed

### Alternative 3: gRPC/HTTP API Instead of JSON-RPC
**Description:** Expose daemon over HTTP/gRPC instead of stdio

**Pros:**
- ✅ Standard protocols (better tooling)
- ✅ Network-accessible (remote clients possible)
- ✅ Schema enforcement (protobuf/OpenAPI)

**Cons:**
- ❌ Port management (find available port, conflicts)
- ❌ Network overhead (loopback latency)
- ❌ Security concerns (authentication, authorization)
- ❌ Firewall/NAT issues (deployment complexity)

**Decision:** **REJECTED** - JSON-RPC over stdio is simpler, more secure, no ports needed

### Alternative 4: Embed Rust as Native Node.js Module
**Description:** Compile Rust as native addon (NAPI), direct FFI calls

**Pros:**
- ✅ No separate process (simplest deployment)
- ✅ Direct FFI calls (lowest latency)
- ✅ No IPC complexity

**Cons:**
- ❌ Complex build (platform-specific binaries)
- ❌ Harder debugging (mixed Rust/JS stack traces)
- ❌ Crash propagation (Rust panic kills Node process)
- ❌ No process isolation (memory leaks affect parent)

**Decision:** **REJECTED** - Process separation is cleaner, more maintainable, safer

## Recommended Approach

### Decision: Separate `daemon-client` Package

**Architecture:**
- **New package:** `packages/daemon-client/` - Reusable TypeScript library
- **Daemon model:** Process-per-MCP-instance (isolated, simple lifecycle)
- **Protocol:** JSON-RPC 2.0 over stdin/stdout (proven, secure)
- **Integration:** MCP server imports daemon-client, replaces spawning

**Justification:**

1. **Separation of Concerns**
   - daemon-client is a reusable library (can be used by VSCode extension in Phase 2)
   - MCP server focuses on MCP protocol, delegates daemon management
   - Clear boundaries: client ↔ daemon ↔ database

2. **Simple Lifecycle Management**
   - Daemon owned by client (starts with MCP server, stops on shutdown)
   - No shared state between clients (isolated failures)
   - Proven pattern from VSCode extension orchestrator

3. **Gradual Migration Path**
   - Phase 1: MCP server migration (high-impact, low-risk)
   - Phase 2: VSCode scan migration (optional, medium-impact)
   - Fallback: Keep spawning code temporarily (safety net)

4. **Operational Simplicity**
   - No ports to manage (stdio communication)
   - No authentication needed (same-machine IPC)
   - No deployment changes (daemon embedded in client)

5. **Fault Isolation**
   - One client crash doesn't affect others
   - Auto-restart on daemon failure (per-client recovery)
   - Circuit breaker prevents restart loops

### Implementation Strategy

**Phase 1: Foundation (This Project)**
1. Create `daemon-client` package (TypeScript library)
2. Migrate MCP server search tool to daemon
3. Comprehensive testing (unit, integration, performance)
4. Documentation (API docs, troubleshooting, runbooks)

**Phase 2: Expansion (Future)**
1. Migrate VSCode `scan` command to daemon (optional)
2. Shared daemon exploration (if resource usage justifies)
3. Additional tools (context, upsert) via daemon (if latency-sensitive)

---

**Analysis Complete:** 2025-11-22
**Recommendation:** PROCEED with daemon-client package implementation

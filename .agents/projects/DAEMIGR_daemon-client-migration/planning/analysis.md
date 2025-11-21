# DAEMIGR Project Analysis

## Project Context

The **DAEMIGR** (Daemon Client Migration) project addresses the final integration step of the MAPDAEMON architecture. While MAPDAEMON successfully implemented a high-performance daemon in Rust, the TypeScript clients (MCP server and VSCode extension) still use the old process-spawning approach, preventing us from realizing the performance benefits.

## Current System Analysis

### Client Inventory

| Client | Type | Current Approach | Migration Priority |
|--------|------|-----------------|-------------------|
| **MCP Server** | Search requests | Spawn per request | **HIGH** (frequent, high-impact) |
| **VSCode: watch** | File monitoring | Long-running process | **NONE** (already optimal) |
| **VSCode: branch-watch** | Git monitoring | Long-running process | **NONE** (already optimal) |
| **VSCode: scan** | Initial indexing | Spawn per scan | **MEDIUM** (infrequent) |
| **CLI** | Direct invocation | Direct binary exec | **NONE** (not applicable) |

### Performance Analysis

**Current MCP Server (Process Spawning):**
```
Request → Spawn Process (~100-200ms)
       → DB Connection (~50-150ms)
       → Query Execution (~10-50ms)
       → Response
       → Process Exit + Connection Close
Total: ~160-400ms per request
```

**With Daemon:**
```
First Request → Daemon Start (~200-500ms)
             → Query Execution (~10-50ms)
             → Response
Total: ~210-550ms (similar to current)

Subsequent Requests → JSON-RPC (~0.5ms)
                   → Query Execution (~10-50ms, pooled connection)
                   → Response
Total: ~10.5-50.5ms (3-40x improvement)
```

### Code Hotspots

**Files requiring changes:**
1. **`packages/maproom-mcp/src/tools/search.ts`** (268 lines)
   - Lines 233-291: Binary spawning logic
   - Lines 307-308: Chunk ID fetching (keep)
   - Lines 311-343: Result transformation (keep)

2. **`packages/maproom-mcp/src/utils/process.ts`** (301 lines)
   - `trySpawnWithCandidates()` function
   - Mark as deprecated, don't remove (VSCode still uses for scan)

3. **New: `packages/daemon-client/src/`** (to be created)
   - Core daemon lifecycle management
   - JSON-RPC protocol handling
   - Error recovery and health checking

## Stakeholder Impact

### For MCP Server Users (AI Assistants)
**Impact:** POSITIVE
- Faster search responses (3-40x improvement)
- More responsive AI interactions
- Better experience with multiple queries

**Risks:** Minimal
- Transparent to end users
- Fallback to old behavior if daemon fails

### For Developers
**Impact:** POSITIVE (long-term), NEUTRAL (short-term)
- Better performance for development workflows
- Slightly more complex debugging (daemon indirection)
- Better architecture (cleaner separation)

**Risks:** Low
- Well-tested daemon implementation
- Gradual migration strategy
- Comprehensive documentation

### For VSCode Extension Users
**Impact:** NEUTRAL (out of scope for Phase 1)
- No immediate changes
- watch/branch-watch already optimal
- scan operation can be migrated later (Phase 2)

## Technical Dependencies

### Prerequisites
- ✅ **MAPDAEMON complete** - Daemon implementation ready
- ✅ **PostgreSQL schema stable** - No schema changes needed
- ✅ **JSON-RPC protocol defined** - Protocol implemented in daemon

### External Dependencies
- **Node.js:** >=18 (for daemon-client package)
- **TypeScript:** >=5.0 (for type safety)
- **Execa/Child Process:** For process management

### Internal Dependencies
- **maproom-mcp:** Depends on new daemon-client package
- **Daemon binary:** Must be available at runtime

## Risk Assessment

### Technical Risks

**Risk: Daemon process management complexity**
- **Likelihood:** Medium
- **Impact:** Medium
- **Mitigation:** Use battle-tested patterns from VSCode extension's orchestrator

**Risk: JSON-RPC protocol incompatibilities**
- **Likelihood:** Low (already tested in MAPDAEMON)
- **Impact:** High (broken search)
- **Mitigation:** Integration tests, protocol versioning

**Risk: Resource leaks (zombie processes)**
- **Likelihood:** Low
- **Impact:** Medium
- **Mitigation:** Proper cleanup in daemon-client, test coverage

### Operational Risks

**Risk: Daemon crashes in production**
- **Likelihood:** Low (tested implementation)
- **Impact:** Medium (degraded performance, fallback to spawn)
- **Mitigation:** Auto-restart, health checks, comprehensive logging

**Risk: Deployment complexity increase**
- **Likelihood:** Medium
- **Impact:** Low
- **Mitigation:** Clear documentation, simple configuration

## Success Metrics

### Performance Metrics
- **Cold start latency:** ~200-500ms (baseline: ~200-500ms)
- **Warm request latency:** < 10ms (baseline: ~200-500ms)
- **Improvement factor:** 20-50x for warm requests
- **Memory usage:** < 100MB additional (daemon overhead)

### Quality Metrics
- **Test coverage:** > 80% for daemon-client package
- **Integration test pass rate:** 100%
- **Regression issues:** 0

### Adoption Metrics
- **MCP server migration:** 100% (all search requests via daemon)
- **Fallback rate:** < 1% (daemon should be stable)
- **User-reported issues:** 0

## Alternative Approaches Considered

### Alternative 1: Keep Process Spawning
**Pros:** No migration work, proven approach
**Cons:** Performance remains poor, no connection pooling
**Decision:** Rejected - defeats purpose of MAPDAEMON

### Alternative 2: Shared Daemon (Multiple Clients)
**Pros:** Single daemon for all clients, better resource usage
**Cons:** Complex IPC, coordination overhead, coupled lifecycles
**Decision:** Rejected for Phase 1 - too complex, can revisit later

### Alternative 3: gRPC/HTTP API Instead of JSON-RPC
**Pros:** More standard, better tooling
**Cons:** Port management, network overhead, security concerns
**Decision:** Rejected - JSON-RPC over stdio is simpler, more secure

### Alternative 4: Embed Rust as Native Module
**Pros:** No separate process, direct FFI calls
**Cons:** Complex build, platform-specific binaries, harder debugging
**Decision:** Rejected - process separation is cleaner, more maintainable

## Recommended Approach

**Decision:** Implement separate daemon-client package with process-per-MCP-instance architecture

**Justification:**
1. Clean separation of concerns (daemon-client is reusable)
2. Simple lifecycle management (daemon owned by client)
3. Proven patterns from VSCode extension orchestrator
4. Gradual migration path (can fallback to spawning)
5. Isolated failures (one client crash doesn't affect others)

---

**Analysis Complete:** 2025-11-21  
**Recommendation:** PROCEED with implementation

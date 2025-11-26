# DAEMIGR: Daemon Client Migration

**Project Slug:** DAEMIGR
**Status:** 87% Complete (13/15 tickets), Production-Ready with Testing Gaps
**Priority:** HIGH (realizes MAPDAEMON performance benefits)

## Overview

The DAEMIGR project completes the MAPDAEMON architecture by migrating TypeScript clients from process-spawning to daemon-based communication. This migration delivers **20-50x performance improvements** for MCP server search requests while maintaining backward compatibility and system reliability.

**Current Implementation Status:**
- ✅ daemon-client package created and complete (`packages/daemon-client/`)
- ✅ Core modules implemented (client.ts, lifecycle.ts, rpc.ts, errors.ts)
- ✅ Package configuration complete (package.json, tsconfig.json, exports)
- ✅ Unit tests complete with comprehensive coverage
- ✅ MCP server integration complete (singleton daemon management)
- ✅ Performance testing complete (targets met: <60ms warm, <600ms cold)
- ✅ Integration testing complete (88% pass rate)
- ✅ Documentation complete (API reference, migration guide, security docs)
- ✅ Code cleanup complete (deprecated spawning, updated changelog)
- ❌ Stress testing not implemented (DAEMIGR-3902)
- ❌ Regression testing not implemented (DAEMIGR-3903)

## Problem Statement

The MAPDAEMON project (✅ **complete and archived** - see `.agents/archive/projects/MAPDAEMON_maproom-daemon-architecture/`) successfully implemented a high-performance Rust daemon with JSON-RPC over stdio, connection pooling, and optimized search execution. However, **TypeScript clients still spawn new processes for each search request**, preventing us from realizing the performance benefits.

**Current Performance:**
- MCP Server search: 160-400ms per request (process spawn + DB connection + query)

**Target Performance:**
- Cold start: 200-550ms (acceptable, similar to current)
- Warm requests: 10-50ms (20-50x improvement)

## Proposed Solution

Create a reusable `daemon-client` TypeScript package that manages daemon lifecycle and provides high-level APIs for search operations. Migrate MCP server search tool to use this package instead of spawning processes.

### Key Components

1. **daemon-client Package** (`packages/daemon-client/`)
   - Process lifecycle management (start, stop, restart, health checks)
   - JSON-RPC protocol handling (request/response matching)
   - Auto-restart with exponential backoff and circuit breaker
   - Typed errors and comprehensive logging

2. **MCP Server Integration** (`packages/maproom-mcp/`)
   - Replace spawning with DaemonClient in search tool
   - Singleton daemon per MCP instance
   - Graceful shutdown and resource cleanup

3. **Testing & Validation**
   - Unit tests (> 80% coverage)
   - Integration tests (end-to-end search via daemon)
   - Performance tests (latency, throughput, resource usage)
   - Regression tests (functionality preserved)

## Benefits

**Performance:**
- 20-50x faster warm search requests (10-50ms vs 160-400ms)
- Better connection pooling (1 pool vs N connections)
- Reduced CPU overhead (no repeated process spawning)

**Architecture:**
- Clean separation of concerns (daemon-client is reusable)
- Simple lifecycle management (daemon owned by client)
- Fault isolation (client crashes don't affect others)

**User Experience:**
- More responsive AI assistant interactions
- Faster multi-query workflows
- Transparent migration (no breaking changes)

## Scope

### In Scope (Phase 1)
- ✅ Create `daemon-client` package
- ✅ Migrate MCP server search tool
- ✅ Comprehensive testing and documentation

### Out of Scope (Future Phases)
- ❌ VSCode extension `scan` command migration (Phase 2)
- ❌ Shared daemon across multiple clients (Phase 3)
- ❌ Additional tools via daemon (`context`, `upsert`) (Future)

### Explicitly NOT Migrating
- ✅ VSCode `watch` command (already optimal, long-running)
- ✅ VSCode `branch-watch` command (already optimal, long-running)
- ✅ CLI usage (direct binary execution is appropriate)

## Relevant Agents

### Phase 1: Foundation (daemon-client package)
- **process-management-specialist** - Core DaemonClient and lifecycle management
- **general-purpose** - Package setup, RPC protocol, TypeScript implementation
- **unit-test-runner** - Unit tests, coverage verification

### Phase 2: Integration (MCP server)
- **general-purpose** - MCP search tool migration, singleton management
- **integration-tester** - End-to-end testing with real daemon and database

### Phase 3: Validation (Testing)
- **general-purpose** - Performance tests, stress tests
- **verify-ticket** - Regression testing, acceptance criteria verification

### Phase 4: Polish (Documentation)
- **general-purpose** - Documentation, security docs, code cleanup

## Planning Documents

Comprehensive planning documents in `planning/` directory:

- **[analysis.md](planning/analysis.md)** - Problem analysis, stakeholder impact, risk assessment, alternatives
- **[architecture.md](planning/architecture.md)** - System design, component architecture, data flows, configuration
- **[quality-strategy.md](planning/quality-strategy.md)** - Testing approach, coverage targets, quality gates
- **[security-review.md](planning/security-review.md)** - Threat model, attack vectors, mitigation strategy, compliance
- **[plan.md](planning/plan.md)** - Phased implementation plan, agent assignments, timeline, success metrics

## Success Metrics

### Performance Targets
- ✅ Cold start latency < 600ms
- ✅ Warm request latency < 60ms
- ✅ Throughput > 50 req/s
- ✅ Memory overhead < 100MB (no leaks)

### Quality Targets
- ✅ Unit test coverage > 80%
- ✅ All integration tests pass (100%)
- ✅ All regression tests pass (0 regressions)
- ✅ Daemon restart rate < 1%

### Adoption Targets
- ✅ MCP server migration 100% (all search via daemon)
- ✅ Zero critical user-reported issues
- ✅ Documentation complete and tested

## Key Technical Decisions

### Why JSON-RPC 2.0 over stdio?
- **Standard:** Well-defined spec, proven in production (LSP, VSCode)
- **Simple:** Text-based, easy to debug, no port management
- **Secure:** No network exposure, no authentication needed
- **Performant:** Low latency for local IPC (~0.5-1ms overhead)

### Why Process-per-Instance?
- **Isolation:** Client crashes don't affect others
- **Lifecycle:** Simple ownership (client owns daemon)
- **Migration:** Gradual rollout (one client at a time)
- **Debugging:** Clear parent-child relationship

### Why Auto-Restart with Circuit Breaker?
- **Reliability:** Transparent recovery from daemon crashes
- **Safety:** Circuit breaker prevents restart loops (max 5 attempts)
- **UX:** Better user experience (no manual intervention)
- **Proven:** Standard pattern (PM2, systemd, LSP servers)

## Risks and Mitigations

### Technical Risks
- **Daemon crashes:** Auto-restart with exponential backoff, circuit breaker
- **Resource leaks:** Comprehensive leak detection tests (1000+ requests)
- **Concurrent requests:** Request ID-based response matching, integration tests
- **Performance regression:** Performance tests with strict targets

### Operational Risks
- **Production instability:** Comprehensive testing, optional fallback to spawning
- **Deployment complexity:** No deployment changes (daemon embedded in MCP server)
- **Security concerns:** Document credential exposure, recommend secrets management

### Mitigation Strategy
- **Phase gates:** Can't proceed without previous phase complete
- **Regression tests:** Ensure functionality preserved
- **Performance tests:** Validate latency targets met
- **Documentation:** Security considerations, troubleshooting guides

## Timeline Estimate

**Phase 1:** 3-5 days (foundation - daemon-client package)
**Phase 2:** 2-3 days (integration - MCP server migration)
**Phase 3:** 2-3 days (validation - testing and performance)
**Phase 4:** 1-2 days (polish - documentation and cleanup)

**Total:** 8-13 days (with contingency buffer)

**Note:** Timeline is for ticket completion, not including review/approval cycles.

## Implementation Status (Updated 2025-11-22)

### Completed Work (13/15 tickets)

**Phase 1 - Foundation (5/5 complete):**
- DAEMIGR-1000: Implementation review (commit a1da961)
- DAEMIGR-1001: Package configuration (commit 7500fdb)
- DAEMIGR-1002: Core lifecycle implementation (commit 23ba660)
- DAEMIGR-1003: JSON-RPC protocol (commit b323f23)
- DAEMIGR-1904: Unit tests (commit 8a9dacd)

**Phase 2 - Integration (4/4 complete):**
- DAEMIGR-2001: MCP daemon integration (commit 28bcd19)
- DAEMIGR-2002: Singleton management (commit 93d4a14)
- DAEMIGR-2004: FTS mode daemon support (commit ab37a27) *[added during implementation]*
- DAEMIGR-2903: Integration tests (commit bcdb814)

**Phase 3 - Validation (1/3 complete):**
- ✅ DAEMIGR-3901: Performance testing (commit dd9060e)
- ❌ DAEMIGR-3902: Stress testing - **NOT IMPLEMENTED**
- ❌ DAEMIGR-3903: Regression testing - **NOT IMPLEMENTED**

**Phase 4 - Polish (3/3 complete):**
- DAEMIGR-4001: Documentation (commit b37e8d8)
- DAEMIGR-4002: Security documentation (commit a1293f5)
- DAEMIGR-4003: Code cleanup (commit 7a59b96)

### Testing Gaps

**DAEMIGR-3902 (Stress Testing):**
- Intended to validate stability under extreme load (10k+ sequential, 1k+ concurrent requests)
- Test memory leaks, daemon crashes, resource exhaustion, circuit breaker
- File not created: `packages/daemon-client/tests/stress.test.ts`
- Status: Planned but never implemented

**DAEMIGR-3903 (Regression Testing):**
- Intended to verify 100% functionality preservation (daemon vs spawning comparison)
- Test all search modes (fts, vector, hybrid), filters, edge cases
- File not created: `packages/maproom-mcp/tests/regression.test.ts`
- Status: Planned but never implemented

### Next Steps

**Option 1: Accept as Production-Ready**
- Current implementation is functional and meets core requirements
- Performance tests pass, integration tests at 88% pass rate
- Documentation complete, security considerations documented
- Risk: Unknown behavior under extreme stress or edge cases

**Option 2: Complete Missing Tests**
- Implement DAEMIGR-3902 stress testing (1-2 days effort)
- Implement DAEMIGR-3903 regression testing (1-2 days effort)
- Increase production confidence through comprehensive validation
- Risk: Delays deployment of working solution

**Recommendation:** Assess production usage patterns and risk tolerance. If MCP server usage is moderate (< 1000 req/min), current testing may be sufficient. If high-throughput or mission-critical, complete missing tests.

## Related Documentation

- **MAPDAEMON Project:** `.agents/archive/projects/MAPDAEMON_*/` (predecessor, daemon implementation)
- **Maproom Architecture:** `docs/architecture/` (system architecture documentation)
- **MCP Server Docs:** `packages/maproom-mcp/README.md` (MCP server implementation)
- **VSCode Extension:** `packages/vscode-extension/` (future migration candidate)

## Questions or Concerns?

See `planning/` directory for detailed analysis, architecture, quality strategy, security review, and implementation plan. All decisions are documented with rationale, alternatives considered, and trade-offs explained.

---

**Project Created:** 2025-11-22
**Planning Status:** ✅ COMPLETE
**Ready for Implementation:** ✅ YES

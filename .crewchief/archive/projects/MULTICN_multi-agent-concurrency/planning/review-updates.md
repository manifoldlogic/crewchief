# Project Review Updates

**Original Review Date:** 2025-11-27
**Updates Completed:** 2025-12-05
**Update Status:** Complete

## Summary

| Category | Issues Found | Issues Fixed |
|----------|--------------|--------------|
| Critical Issues | 2 | 2 |
| Boundary Violations | 0 | 0 |
| High-Risk Areas | 4 | 4 |
| Gaps & Ambiguities | 9 | 9 |
| Scope Adjustments | 3 | 3 |
| Reinvention/Reuse | 4 | 4 |

## Critical Issues Addressed

### Issue 1: Underspecified Phase 1 Acceptance Criteria

**Original Problem:** Phase 1 tickets (MULTICN-1001 through 1003) lacked concrete, measurable acceptance criteria. Agents couldn't verify completion objectively.

**Changes Made:**
- **plan.md**: Added specific acceptance criteria for MULTICN-1001 (verify with concurrent indexing test that previously failed with SQLITE_BUSY)
- **plan.md**: Added acceptance criteria for MULTICN-1002 (verify env vars work via log output showing new config values)
- **plan.md**: Added acceptance criteria for MULTICN-1003 (verify with unit test simulating SQLITE_BUSY and confirming exponential backoff retry behavior)
- **plan.md**: Added "Verification Steps" section to each ticket with concrete test commands

**Result:** Issue resolved - All Phase 1 tickets now have programmatically verifiable acceptance criteria

### Issue 2: Dual-Mode Compatibility Not Designed

**Original Problem:** Rollback plan depends on stdio/socket dual-mode support, but architecture.md didn't design the Connection interface abstraction. Risk of Phase 2 rework.

**Changes Made:**
- **architecture.md**: Added "Connection Abstraction Layer" section defining the `Connection` interface
- **architecture.md**: Specified `StdioConnection` and `SocketConnection` implementations
- **architecture.md**: Clarified connection mode selection logic (auto-detect with socket preference, stdio fallback)
- **architecture.md**: Added connection lifecycle management integration with existing DaemonLifecycle

**Result:** Issue resolved - Connection interface explicitly designed with clear separation between transport-agnostic and transport-specific code

## Reinvention & Reuse Issues Fixed

### Missed Reuse 1: DaemonLifecycle Patterns

**Original Problem:** Plan proposed creating new lifecycle management in discovery.ts instead of reusing existing lifecycle.ts patterns (circuit breaker, exponential backoff, restart logic).

**Changes Made:**
- **architecture.md**: Added "Reused Components" section documenting lifecycle pattern reuse
- **architecture.md**: Specified extending DaemonLifecycle for socket connection management
- **plan.md**: Updated MULTICN-2006 to explicitly reuse lifecycle patterns from daemon-client/lifecycle.ts
- **plan.md**: Added reference to circuit breaker configuration (maxFailures, resetWindow)

**Result:** Existing lifecycle patterns will be reused, not reinvented

### Missed Reuse 2: Error Hierarchy Extension

**Original Problem:** Plan didn't mention extending the existing error hierarchy in daemon-client/errors.ts for socket-specific errors.

**Changes Made:**
- **architecture.md**: Added error hierarchy section showing SocketConnectionError, SocketTimeoutError extending existing DaemonError classes
- **plan.md**: Updated MULTICN-2005 to extend existing error classes
- **architecture.md**: Documented error code conventions to match existing pattern

**Result:** Socket errors integrate with existing error handling patterns

### Missed Reuse 3: RPC Protocol Reuse

**Original Problem:** Plan mentioned creating length-prefixed codec but didn't clarify relationship to existing RpcProtocol class.

**Changes Made:**
- **architecture.md**: Clarified that RpcProtocol class is reused for message creation/parsing
- **architecture.md**: Specified that LengthPrefixedCodec wraps serialization only (transport layer)
- **architecture.md**: Kept JSON-RPC message structures unchanged

**Result:** Clear separation - reuse JSON-RPC logic, add length-prefix framing

### Pattern Violation: SqliteConfig Structure

**Original Problem:** Plan showed flat SqliteConfig, inconsistent with existing nested config pattern (SearchConfig, EmbeddingConfig).

**Changes Made:**
- **architecture.md**: Updated SqliteConfig to use nested struct pattern (PoolConfig, PragmaConfig, RetryConfig)
- **architecture.md**: Added Default trait, from_env(), validate() methods matching existing pattern
- **architecture.md**: Added thiserror for config validation errors
- **plan.md**: Updated MULTICN-1002 implementation to match nested pattern

**Result:** SqliteConfig follows established codebase patterns

## High-Risk Mitigations

### Risk 1: Connect-or-Spawn Race Condition Complexity

**Mitigation Applied:**
- **architecture.md**: Added explicit state machine diagram for connect-or-spawn logic
- **architecture.md**: Specified using proper-lockfile library (proven coordination primitive)
- **architecture.md**: Added timeout configuration (socket wait: 10s, lock acquire: 5s)
- **plan.md**: Added detailed logging requirements for production debugging
- **quality-strategy.md**: Already had comprehensive race condition test - verified adequate

**Risk Level:** Reduced from High to Medium

### Risk 2: Idle Timeout State Management

**Mitigation Applied:**
- **architecture.md**: Specified atomic counter (AtomicUsize) for active connection tracking
- **architecture.md**: Added connection lifecycle hooks (on_connect increments, on_disconnect decrements)
- **architecture.md**: Added logging for connection count changes
- **plan.md**: Added manual shutdown command to Phase 2 scope (maproom daemon stop)

**Risk Level:** Reduced from Medium to Low

### Risk 3: Message Framing Edge Cases

**Mitigation Applied:**
- **architecture.md**: Specified using tokio_util::codec::LengthDelimitedCodec (battle-tested)
- **architecture.md**: Added maximum message size check (10MB limit)
- **quality-strategy.md**: Already had partial read tests - verified adequate
- **architecture.md**: Removed custom framing implementation from design

**Risk Level:** Reduced from Medium to Low

### Risk 4: Windows Compatibility (Out of Scope)

**Status:** Explicitly marked as out of scope in review. Windows users will use stdio fallback mode.

**Documentation Updates:**
- **analysis.md**: Updated risks table to note Windows uses stdio fallback
- **architecture.md**: Removed Windows TCP fallback references
- **plan.md**: Clarified socket mode is Unix-only, stdio fallback automatic on Windows

**Risk Level:** N/A - Out of scope

## Gaps Filled

### Requirements Gaps

**Gap 1: SIGHUP Reload Behavior**
- **Decision:** Deferred to post-MVP (not required for MVP)
- **architecture.md**: Removed SIGHUP from Lifecycle Manager diagram
- **plan.md**: Moved SIGHUP handling to "Future Enhancements" section
- **Rationale:** Config changes are rare, daemon restart is acceptable for MVP

**Gap 2: Daemon Version Compatibility**
- **Added to architecture.md**: Protocol version field in handshake
- **Added to architecture.md**: Client sends version in initial ping
- **Added to architecture.md**: Server returns error if version mismatch (major version)
- **Implementation:** MULTICN-2001 scope expanded to include version check

**Gap 3: Lock File Location**
- **Clarified in architecture.md**: Lock file is `/tmp/maproom-{uid}.lock` (socket is `.sock`)
- **Added to architecture.md**: Cleanup logic removes both lock and socket on exit
- **Rationale:** Separate extensions prevent confusion

### Technical Gaps

**Gap 4: Pool Reconfiguration**
- **Clarified in architecture.md**: Pool size changes require daemon restart (no runtime reconfiguration in MVP)
- **Added to plan.md**: Documentation requirement to note restart needed for config changes
- **Rationale:** Runtime pool resizing adds complexity not justified for MVP

**Gap 5: Embedding Service Thread Safety**
- **Verified in architecture.md**: EmbeddingService already uses internal RwLock for rate limiting
- **Added to architecture.md**: Note that existing rate limiter handles concurrent requests
- **Added to plan.md**: No changes needed to EmbeddingService (already thread-safe)

**Gap 6: WAL Checkpoint Strategy**
- **Specified in architecture.md**: Use SQLite auto-checkpoint with wal_autocheckpoint=10000
- **Added to architecture.md**: Manual checkpoint during idle timeout (before shutdown)
- **Implementation:** Use `PRAGMA wal_checkpoint(TRUNCATE)` on idle shutdown
- **Added to plan.md**: MULTICN-1001 includes wal_autocheckpoint pragma

### Process Gaps

**Gap 7: Migration Testing**
- **Added to quality-strategy.md**: "Dual-Mode Compatibility Testing" section
- **Added explicit requirement**: Run existing test suite with MAPROOM_CONNECTION_MODE=stdio
- **Added explicit requirement**: Run existing test suite with MAPROOM_CONNECTION_MODE=socket
- **Success criteria**: Both modes pass identical test suite

**Gap 8: Performance Baseline**
- **Added to plan.md**: "Phase 0: Baseline Capture" prerequisite
- **Baseline metrics**: Current search latency (p50, p95, p99), index time for 1000 files, memory usage with 1/3 agents
- **Tool**: Use existing `crewchief-maproom bench` command (if exists) or add minimal benchmarking script
- **Storage**: Save baseline to planning/performance-baseline.json for comparison

**Gap 9: Test Harness for Multi-Process Scenarios**
- **Added to quality-strategy.md**: "Multi-Process Test Harness" implementation details
- **Approach**: Use cargo test with process spawning, unique socket paths per test
- **Cleanup**: Automatic test fixture cleanup with Drop trait
- **CI consideration**: Run with --test-threads=1 to prevent socket path conflicts

## Scope Optimizations

### Scope Creep Removed

**Item 1: SIGHUP Config Reload**
- **Status:** Moved to "Future Enhancements" section in plan.md
- **Rationale:** Not required for MVP; daemon restart is acceptable for rare config changes
- **Effort saved:** ~4 hours (signal handling, partial config reload logic, testing)

**Item 2: Session Metrics Tracking**
- **Status:** Removed `request_count: AtomicU64` from Session struct
- **architecture.md**: Simplified Session struct to only essential fields (id, connected_at, response_tx)
- **Rationale:** Nice observability feature but not needed for MVP functionality
- **Effort saved:** ~2 hours (metrics collection, aggregation, exposure)

**Item 3: Broadcast Capability**
- **Status:** Removed `SessionRegistry.broadcast()` method from architecture
- **Rationale:** No use case identified in current requirements
- **Future consideration**: If needed for cache invalidation notifications, can add later
- **Effort saved:** ~3 hours (broadcast implementation, testing)

### MVP Boundaries Clarified

- **plan.md**: Added explicit "Out of Scope for MVP" section listing deferred features
- **plan.md**: Defined MVP success criteria focused on core concurrency fix
- **architecture.md**: Removed speculative future features from component designs
- **Total effort saved:** ~9 hours (can be reallocated or reduce timeline)

## Document Change Summary

| Document | Lines Modified | Key Changes |
|----------|----------------|-------------|
| architecture.md | ~150 | Added Connection interface, reused components section, state machine diagram, nested SqliteConfig, removed scope creep |
| plan.md | ~120 | Added acceptance criteria for Phase 1, baseline capture phase, verification steps, out-of-scope section, lifecycle reuse |
| quality-strategy.md | ~40 | Added dual-mode testing section, migration test requirements, baseline comparison |
| analysis.md | ~15 | Updated Windows risk note, clarified scope boundaries |
| security-review.md | ~5 | Minor clarification on lock file naming |

**Total changes:** ~330 lines across 5 documents

## Alignment Improvements

### MVP Discipline
- **Before:** Some nice-to-have features mixed with essential functionality
- **After:** Clear MVP scope with deferred features documented separately
- **Impact:** Reduced timeline risk, clearer success criteria

### Reuse & Patterns
- **Before:** Some reinvention of existing patterns (lifecycle, errors, config)
- **After:** Explicit reuse documented with integration approach
- **Impact:** Less code to write, better consistency, fewer bugs

### Agent Executability
- **Before:** Vague acceptance criteria ("update config", "handle errors")
- **After:** Specific, verifiable criteria with test commands
- **Impact:** Agents can objectively verify completion, less rework

### Risk Management
- **Before:** Risks identified but some mitigations underspecified
- **After:** Concrete mitigation approaches with specific libraries/patterns
- **Impact:** Higher confidence in execution, clearer implementation path

## Verification

**Re-review Recommended:** Yes

**Expected Result:** All critical issues resolved, gaps filled, scope optimized for MVP success

**Confidence Level:** High - Changes are concrete, specific, and maintain consistency across documents

## Next Steps

1. **Recommended:** Run `/workstream:project-review MULTICN` to verify all issues resolved
2. **If review passes:** Proceed to `/workstream:project-tickets MULTICN` to generate tickets
3. **After tickets created:** Run ticket review to ensure ticket quality
4. **Then:** Execute with `/workstream:project-work MULTICN`

## Notes

- No tickets existed yet, so ticket updates were not applicable
- All changes maintain backwards compatibility (stdio mode remains available)
- Updates prioritized execution readiness over cosmetic improvements
- Deferred features are documented for future consideration, not lost

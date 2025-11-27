# Project Review: MULTICN - Multi-Agent Concurrency

**Review Date:** 2025-11-27
**Project Status:** Proceed with Caution
**Overall Risk:** Medium
**Tickets Created:** No - Pre-ticket review

## Executive Summary

The MULTICN project is well-conceived and addresses a real pain point: multiple Claude Code agents cannot work concurrently due to SQLite write contention. The proposed solution—a shared daemon via Unix socket—is architecturally sound and appropriate for the problem.

However, the review identified several concerns that should be addressed before ticket creation:

1. **Significant reuse opportunities exist** but aren't fully leveraged in the plan. The existing `daemon-client` has mature patterns (lifecycle management, circuit breakers, error hierarchy) that should be extended rather than rebuilt.

2. **Phase 1 scope is underspecified** for ticket creation. The SQLite configuration work needs more concrete acceptance criteria.

3. **Testing strategy has gaps** around the stdio/socket dual-mode compatibility that's critical for the rollback plan.

4. ~~**Windows compatibility is mentioned but not planned**~~ - Explicitly out of scope per user decision. Unix socket only; Windows users can use stdio fallback.

The project should proceed after addressing these gaps. Success probability is high given the well-researched analysis.

## Critical Issues (Blockers)

### Issue 1: Underspecified Phase 1 Acceptance Criteria

**Severity:** Critical
**Category:** Requirements
**Description:** The plan for Phase 1 tickets (MULTICN-1001 through 1003) lacks concrete acceptance criteria. For example, MULTICN-1001 says "Update SQLite connection initialization" but doesn't specify:
- How to verify the changes work (what test?)
- Expected behavior change (measurable?)
- Backwards compatibility requirements

**Impact:** Agents cannot verify completion; tickets will be subjective "looks good" assessments.

**Required Action:**
- Add measurable acceptance criteria to each Phase 1 ticket outline
- Specify test cases that prove the change works
- Define performance benchmarks (e.g., "busy_timeout allows 30s wait under contention")

**Documents Affected:** plan.md

### Issue 2: Dual-Mode Compatibility Not Designed

**Severity:** Critical
**Category:** Architecture
**Description:** The rollback plan depends on `MAPROOM_CONNECTION_MODE=stdio` working alongside socket mode, but the architecture document doesn't design this abstraction. The `DaemonClient` class currently has stdio deeply embedded—it's not clear how both modes coexist.

**Impact:** Without a `Connection` interface abstraction designed upfront, Phase 2b tickets may require refactoring Phase 2a work, causing rework.

**Required Action:**
- Add a "Connection Abstraction" section to architecture.md
- Define the `Connection` interface that both `StdioConnection` and `SocketConnection` implement
- Clarify which methods live in the interface vs transport-specific code

**Documents Affected:** architecture.md

## Reinvention & Duplication Analysis

### Missed Reuse Opportunities

**Available Component:** `packages/daemon-client/src/lifecycle.ts`
**Could Solve:** Connection lifecycle management, circuit breaker, exponential backoff
**Integration Method:** Extend existing class or extract reusable patterns
**Integration Effort:** Low
**Recommendation:** The plan proposes creating new lifecycle management in `discovery.ts`. Instead, extend `DaemonLifecycle` or extract its core patterns (circuit breaker, backoff calculation) into a shared utility. The existing code handles:
- Restart attempts with reset window (60s)
- Exponential backoff (2^n * base)
- Circuit breaker after max failures
- Graceful shutdown with request draining

**Available Component:** `packages/daemon-client/src/errors.ts`
**Could Solve:** Error hierarchy for socket failures
**Integration Method:** Extend existing error classes
**Integration Effort:** Low
**Recommendation:** Add `SocketConnectionError`, `SocketTimeoutError` extending the existing hierarchy rather than creating new error types. The current errors have `code` fields, stack trace capture, and helper methods.

**Available Component:** `packages/daemon-client/src/rpc.ts`
**Could Solve:** JSON-RPC protocol encoding/decoding
**Integration Method:** Reuse directly, add length-prefix codec alongside
**Integration Effort:** Low
**Recommendation:** Keep `RpcProtocol` for message creation/parsing. Add a new `LengthPrefixedCodec` that wraps the serialization with 4-byte length prefix. Don't duplicate the JSON-RPC message structures.

**Available Component:** `crates/maproom/src/config/search_config.rs`
**Could Solve:** Pattern for SqliteConfig struct
**Integration Method:** Follow established pattern
**Integration Effort:** Low
**Recommendation:** The plan mentions creating SqliteConfig but doesn't reference the existing config patterns. Follow `SearchConfig` pattern exactly: nested structs, `Default` trait, `from_env()`, `validate()` method, `thiserror` for errors.

### Pattern Violations

**Existing Pattern:** Nested config structs with validation
**Proposed Deviation:** Plan shows flat SqliteConfig
**Consistency Impact:** Inconsistent with SearchConfig, EmbeddingConfig patterns
**Recommendation:** Use nested pattern:
```rust
pub struct SqliteConfig {
    pub pool: PoolConfig,
    pub pragmas: PragmaConfig,
    pub retry: RetryConfig,
}
```

### Appropriate Reuse Identified

The plan correctly identifies:
- ✅ Reusing `Arc<DaemonState>` for shared state
- ✅ Reusing `handle_request()` handler logic
- ✅ Reusing JSON-RPC protocol structures
- ✅ Keeping existing daemon methods (ping, search)

## High-Risk Areas (Warnings)

### Risk 1: Connect-or-Spawn Race Condition Complexity

**Risk Level:** High
**Category:** Technical
**Description:** The connect-or-spawn logic involves lock files, double-checking, daemon spawning, and socket waiting. This is complex coordination code with many failure modes.
**Probability:** Medium
**Impact:** High - Could cause daemon duplication or connection failures
**Mitigation:**
- Add explicit state machine diagram to architecture
- Consider using a proven library (e.g., `proper-lockfile` in Node.js)
- Add comprehensive timeout and retry configuration
- Include detailed logging for debugging production issues

### ~~Risk 2: Cross-Platform Socket Path Handling~~ (Out of Scope)

**Status:** Explicitly out of scope per user decision. Windows users will use stdio fallback mode (`MAPROOM_CONNECTION_MODE=stdio`). No TCP fallback needed for MVP.

### Risk 3: Idle Timeout State Management

**Risk Level:** Medium
**Category:** Technical
**Description:** The daemon must track "no clients connected" state for 5-minute idle timeout. With socket connections coming and going, accurate tracking is non-trivial.
**Probability:** Medium
**Impact:** Low - Worst case: daemon doesn't shutdown or shuts down prematurely
**Mitigation:**
- Use atomic counter for active connections
- Log connection count changes for debugging
- Add manual shutdown command (`maproom daemon stop`) as fallback

### Risk 4: Message Framing Edge Cases

**Risk Level:** Medium
**Category:** Technical
**Description:** Length-prefixed framing has edge cases: partial reads, oversized messages, corrupt length bytes.
**Probability:** Medium
**Impact:** Medium - Corrupted messages could crash clients
**Mitigation:**
- Use `tokio_util::codec::LengthDelimitedCodec` (battle-tested)
- Don't implement custom framing from scratch
- Add explicit tests for partial read scenarios
- Include maximum message size check (plan mentions 10MB limit - good)

## Gaps & Ambiguities

### Requirements Gaps

- **SIGHUP reload behavior**: Plan mentions SIGHUP for config reload but doesn't specify what gets reloaded. Is it safe to reload database config mid-operation?
- **Daemon version compatibility**: What happens if client expects newer protocol than daemon supports? Need version negotiation or clear error.
- **Lock file location**: Plan uses `/tmp/maproom-{uid}.lock` but this may conflict with `/tmp/maproom-{uid}.sock`. Clarify naming scheme.

### Technical Gaps

- **Pool reconfiguration**: Plan mentions configurable pool sizes but doesn't address whether changes require daemon restart.
- **Embedding service sharing**: Multiple clients may trigger parallel embedding requests. Is `EmbeddingService` thread-safe? Rate limiting?
- **WAL checkpoint strategy**: Plan mentions "checkpoint during idle" but doesn't specify implementation. Manual PRAGMA wal_checkpoint? Automatic?

### Process Gaps

- **Migration testing**: How to test that existing stdio clients work unchanged? Need explicit regression test requirement.
- **Performance baseline**: Plan has success metrics but no baseline. Need to capture current latency/memory before changes.

## Scope & Feasibility Concerns

### Scope Creep Indicators

- **SIGHUP reload**: Nice-to-have, not required for MVP. Consider deferring.
- **Session metrics tracking**: Plan mentions `request_count: AtomicU64` per session. Is this needed for MVP or just nice observability?
- **Broadcast capability**: `SessionRegistry.broadcast()` method in plan—what's the use case? If none immediate, defer.

### Feasibility Challenges

- **Testing multi-process scenarios**: Integration tests spawning multiple clients are complex. May need test harness infrastructure not currently present.
- **CI timeout risks**: Multi-agent stress tests could be flaky or slow in CI. Plan appropriately.

## Alignment Assessment

### MVP Discipline
**Rating:** Adequate
- ✅ Two-phase approach is sensible
- ✅ Phase 1 provides standalone value (better SQLite handling)
- ⚠️ Some Phase 2 features (SIGHUP, session metrics) could be deferred
- ⚠️ Broadcast capability has no stated use case

### Pragmatism Score
**Rating:** Strong
- ✅ Unix socket vs TCP choice is well-reasoned
- ✅ Leverages existing WAL mode rather than reimplementing
- ✅ Keeps stdio as fallback rather than big-bang migration
- ✅ 5-minute idle timeout is reasonable

### Agent Compatibility
**Rating:** Adequate
- ✅ Ticket sizing appears appropriate (2-8 hour range)
- ⚠️ Phase 1 tickets need clearer acceptance criteria for agent verification
- ⚠️ Some tickets have "Secondary Agent" assignments—clarify handoff protocol
- ✅ Files to modify are explicitly listed per ticket

### Codebase Integration
**Rating:** Adequate
- ✅ Correctly identifies existing patterns to follow
- ⚠️ Doesn't fully leverage existing lifecycle/error patterns
- ⚠️ SqliteConfig pattern not specified to match existing config patterns
- ✅ Preserves backwards compatibility via connection mode abstraction

### Separation of Concerns
**Rating:** Strong
- ✅ Clear separation: protocol layer, session layer, handler layer
- ✅ Transport-agnostic handler design
- ✅ Connection interface abstraction (though needs explicit design)
- ✅ Database layer unchanged

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [ ] Plan is detailed enough to create tickets from (Phase 1 needs more detail)
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [ ] Dependencies on existing systems documented (lifecycle reuse not specified)

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [ ] Integration points are well-defined (Connection interface not designed)
- [x] Performance requirements are clear
- [x] Error handling is specified
- [ ] Existing tools/libraries identified for reuse (partial)
- [ ] No unnecessary duplication of functionality (some duplication risk)

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [ ] Verification criteria are explicit (Phase 1 weak)
- [x] Handoffs are defined
- [x] Rollback plan exists
- [x] Integration with existing workflows considered

### Integration & Reuse
- [ ] Existing solutions evaluated before building new (lifecycle.ts not leveraged)
- [x] Current patterns and conventions followed
- [ ] Reusable components identified (partial - errors.ts, lifecycle.ts underused)
- [x] Integration points with existing systems mapped
- [ ] No reinvention of available functionality (some risk)
- [x] Proper integration methods chosen
- [x] Component boundaries respected
- [x] Public interfaces used (not internals)
- [x] Appropriate coupling levels maintained

### Risk
- [x] Major risks are identified
- [x] Mitigation strategies exist
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Add Connection interface design** to architecture.md
   - Define `Connection` interface with `sendRequest`, `close`, `isConnected`
   - Show how `StdioConnection` and `SocketConnection` implement it
   - Clarify where connection mode selection happens

2. **Specify Phase 1 acceptance criteria** in plan.md
   - MULTICN-1001: "Verified by running concurrent indexing test that previously failed"
   - MULTICN-1002: "Verified by setting env var and confirming log output shows new values"
   - MULTICN-1003: "Verified by unit test that simulates SQLITE_BUSY and confirms retry behavior"

3. **Document existing component reuse** in architecture.md
   - Add section: "Reused Components from daemon-client"
   - List: error hierarchy extension, lifecycle patterns, RPC protocol

4. ~~**Add Windows design**~~ - Out of scope. Windows users use stdio fallback.

### Phase 1 Adjustments

- Consider adding baseline performance capture as "Phase 0" or prerequisite
- Ensure SqliteConfig follows nested struct pattern from SearchConfig

### Risk Mitigations

- Use `tokio_util::codec::LengthDelimitedCodec` rather than custom implementation
- Add connection count logging from day one for debugging
- Create explicit test fixtures for multi-client scenarios before Phase 2

### Documentation Updates

- **architecture.md**: Add Connection interface, reuse documentation
- **plan.md**: Add acceptance criteria for all Phase 1 tickets
- **quality-strategy.md**: Add dual-mode compatibility testing requirement

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

**Primary concerns:**
1. Phase 1 tickets lack measurable acceptance criteria for agent verification
2. Connection interface abstraction not explicitly designed (risks Phase 2 rework)
3. Existing lifecycle/error patterns should be explicitly reused

### Recommended Path Forward

**REVISE THEN PROCEED:** Address critical issues (acceptance criteria, Connection interface design) before creating tickets. The architectural approach is sound; the planning documents need refinement for agent executability.

### Success Probability
Given current state: 70%
After recommended changes: 90%

### Final Notes

This is a well-researched project solving a real problem. The analysis document shows deep understanding of SQLite concurrency, and the phased approach is pragmatic. The main gaps are in execution readiness—making the plan specific enough for agents to verify their work, and explicitly leveraging existing codebase patterns.

The two-phase structure is excellent: Phase 1 delivers standalone value (better SQLite handling helps even with current architecture) while Phase 2 provides the main solution. This de-risks the project significantly.

Recommend spending 1-2 hours updating the planning documents before ticket creation. This small upfront investment will prevent significant rework during execution.

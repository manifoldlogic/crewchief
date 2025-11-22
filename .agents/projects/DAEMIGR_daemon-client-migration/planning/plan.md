# DAEMIGR Implementation Plan

## Overview

This plan outlines the phased implementation of the daemon-client migration. Work is organized into distinct phases with clear deliverables and dependencies. Each phase builds on the previous, allowing for incremental validation and risk mitigation.

## Phase Organization

### Phase 1: Foundation (daemon-client package)
**Goal:** Complete and test core daemon communication library

**Duration:** ~1-2 days (reduced from 3-5 days due to existing implementation)

**Current Status:** ~50-70% complete (core modules exist, tests/docs pending)

**Deliverables:**
- `packages/daemon-client/` package code reviewed and gaps identified
- Missing functionality completed (tests, docs, edge cases)
- Unit tests passing (> 80% coverage)
- API documentation complete

**Success Criteria:**
- DaemonClient can start/stop daemon ✅ (implemented)
- DaemonClient can send/receive JSON-RPC ✅ (implemented)
- Auto-restart works (exponential backoff, circuit breaker) ⏳ (verify)
- No memory leaks (1000 request test) ⏳ (test needed)

**Phase Completion Gate:**
- [ ] All unit tests pass with >80% coverage
- [ ] Code review findings addressed
- [ ] All acceptance criteria met for Tickets 0-4
- [ ] Memory leak test passes
- [ ] No critical bugs identified

### Phase 2: Integration (MCP server migration)
**Goal:** Migrate MCP search tool to use daemon

**Duration:** ~2-3 days

**Deliverables:**
- MCP server uses DaemonClient for search
- Singleton management implemented
- Integration tests passing
- Performance targets met

**Success Criteria:**
- MCP search via daemon (no spawning)
- Warm requests < 60ms (20x improvement)
- All regression tests pass
- Error handling works (graceful failures)

**Phase Completion Gate:**
- [ ] Phase 1 complete (all gates passed)
- [ ] All integration tests pass with >80% coverage
- [ ] Performance targets met (cold <600ms, warm <60ms)
- [ ] All acceptance criteria met for Tickets 5-7
- [ ] No regressions identified

### Phase 3: Validation (Performance & Testing)
**Goal:** Comprehensive testing and performance validation

**Duration:** ~2-3 days

**Deliverables:**
- Performance tests implemented
- Stress tests passing
- Regression suite complete
- Documentation finalized

**Success Criteria:**
- Cold start < 600ms
- Warm requests < 60ms consistently
- No memory leaks (1000+ requests)
- Concurrent requests work (50 simultaneous)

**Phase Completion Gate:**
- [ ] Phase 2 complete (all gates passed)
- [ ] All performance tests pass (cold <600ms, warm <60ms, throughput >50 req/s)
- [ ] All stress tests pass (no leaks, no crashes, concurrent load handled)
- [ ] All regression tests pass (100% functionality preserved)
- [ ] All acceptance criteria met for Tickets 8-10

### Phase 4: Polish (Documentation & Cleanup)
**Goal:** Production-ready release

**Duration:** ~1-2 days

**Deliverables:**
- README and API docs complete
- Migration guide written
- Security considerations documented
- Deprecation notices added

**Success Criteria:**
- Documentation reviewed and approved
- Security review complete
- Migration guide tested by external user
- Old spawning code marked deprecated

**Phase Completion Gate:**
- [ ] Phase 3 complete (all gates passed)
- [ ] All documentation complete and reviewed
- [ ] Security considerations documented
- [ ] All acceptance criteria met for Tickets 11-13
- [ ] Project ready for production deployment

## Agent Assignments

### Phase 1: Foundation

#### daemon-client Package Setup
**Agent:** general-purpose
**Skills:** TypeScript package setup, build configuration
**Ticket:** Package structure and build setup

#### DaemonClient Core Implementation
**Agent:** process-management-specialist
**Skills:** Process lifecycle, IPC, resource cleanup
**Ticket:** DaemonClient and DaemonLifecycle classes

#### JSON-RPC Protocol Implementation
**Agent:** general-purpose
**Skills:** Protocol handling, serialization, error codes
**Ticket:** RpcProtocol class and error types

#### daemon-client Unit Tests
**Agent:** unit-test-runner
**Skills:** Vitest, mocking, coverage reporting
**Ticket:** Unit tests for all modules (> 80% coverage)

### Phase 2: Integration

#### MCP Server Daemon Integration
**Agent:** general-purpose
**Skills:** MCP tool modification, import refactoring
**Ticket:** Replace spawning with DaemonClient in search tool

#### Singleton Management
**Agent:** general-purpose
**Skills:** Singleton pattern, resource management
**Ticket:** Daemon singleton management in MCP server

#### Integration Tests
**Agent:** integration-tester
**Skills:** End-to-end testing, real database, real daemon
**Ticket:** MCP search integration tests

### Phase 3: Validation

#### Performance Testing
**Agent:** general-purpose
**Skills:** Benchmarking, latency measurement, profiling
**Ticket:** Performance tests and benchmarks

#### Stress Testing
**Agent:** general-purpose
**Skills:** Load testing, concurrency, resource monitoring
**Ticket:** Stress tests and resource leak detection

#### Regression Testing
**Agent:** verify-ticket
**Skills:** Backward compatibility, result verification
**Ticket:** Regression test suite

### Phase 4: Polish

#### Documentation
**Agent:** general-purpose
**Skills:** Technical writing, API documentation
**Ticket:** README, API docs, migration guide

#### Security Documentation
**Agent:** general-purpose
**Skills:** Security documentation, threat modeling
**Ticket:** Security considerations and best practices

#### Code Cleanup
**Agent:** general-purpose
**Skills:** Deprecation notices, code organization
**Ticket:** Mark old code deprecated, final cleanup

## Detailed Phase Plans

### Phase 1: Foundation

#### Ticket 0: Review Existing Implementation
**Description:** Code review of existing `packages/daemon-client/` to identify gaps and quality issues

**Tasks:**
- Read and analyze all existing modules (client.ts, lifecycle.ts, rpc.ts, errors.ts)
- Review package configuration (package.json, tsconfig.json, vitest.config.ts)
- Identify missing functionality (tests, edge cases, error handling)
- Identify code quality issues (TypeScript types, error handling, resource cleanup)
- Create list of gaps to address in subsequent tickets
- Document findings in ticket or planning doc

**Acceptance Criteria:**
- All existing code read and understood
- Gap list created with specific items to address
- Code quality assessment complete
- Recommendations documented for Tickets 1-4

**Dependencies:** None

**Agent:** general-purpose

**Estimated Effort:** 0.5 days

---

#### Ticket 1: Complete Package Configuration
**Description:** Complete any missing package configuration and build setup

**Tasks:**
- Verify package.json completeness (exports, types, scripts)
- Verify TypeScript configuration (strict mode, module resolution)
- Verify Vitest configuration (coverage, test environment)
- Add any missing build scripts or configurations
- Ensure package builds and tests run correctly

**Acceptance Criteria:**
- Package builds successfully (`pnpm build`)
- Tests run successfully (`pnpm test`)
- Linter passes (`pnpm lint`)
- Package exports configured correctly

**Dependencies:** Ticket 0 (code review complete)

**Agent:** general-purpose

**Estimated Effort:** 0.25 days

---

#### Ticket 2: Complete Core Implementation
**Description:** Complete any missing functionality in DaemonClient and DaemonLifecycle

**Tasks:**
- Review existing `src/lifecycle.ts` - complete missing features (graceful shutdown, resource cleanup)
- Review existing `src/client.ts` - complete missing features (error handling, edge cases)
- Review existing `src/types.ts` - ensure all interfaces complete and properly typed
- Verify request queue management (Map<id, PendingRequest>) works correctly
- Verify health checking logic (ping before request) implemented
- Add any missing edge case handling identified in Ticket 0

**Acceptance Criteria:**
- DaemonClient can start/stop daemon ✅ (verify existing)
- Request IDs are sequential with rollover handling
- Responses matched to requests by ID ✅ (verify existing)
- Auto-restart works (exponential backoff) ⏳ (verify/complete)
- Circuit breaker triggers after 5 restarts ⏳ (verify/complete)
- Graceful shutdown waits for in-flight requests
- All resources cleaned up (streams, listeners, processes)

**Dependencies:** Ticket 1 (package config complete)

**Agent:** process-management-specialist

**Estimated Effort:** 0.5 days

---

#### Ticket 3: Complete JSON-RPC Protocol Implementation
**Description:** Complete and validate JSON-RPC 2.0 protocol handling

**Tasks:**
- Review existing `src/rpc.ts` - verify createRequest, parseResponse, isError complete
- Review existing `src/errors.ts` - verify error hierarchy and error codes complete
- Add any missing protocol validation (jsonrpc version, required fields)
- Verify error code mapping (JSON-RPC codes → DaemonError types)
- Add error serialization (DaemonError → JSON-RPC error object)
- Document error format mapping (as specified in architecture.md)

**Acceptance Criteria:**
- createRequest generates valid JSON-RPC 2.0 ✅ (verify existing)
- parseResponse validates and parses correctly ✅ (verify existing)
- Malformed JSON rejected with error ⏳ (verify/add)
- Error codes mapped correctly per architecture.md table
- Orphaned responses handled gracefully (logged, not crashed)

**Dependencies:** Ticket 1 (package config complete)

**Agent:** general-purpose

**Estimated Effort:** 0.25 days

---

#### Ticket 4: Create daemon-client Unit Tests
**Description:** Comprehensive unit tests for all modules (currently missing)

**Tasks:**
- Create `tests/client.test.ts` - test lifecycle, requests, health checks, error handling
- Create `tests/lifecycle.test.ts` - test start, stop, restart, crash recovery, backoff
- Create `tests/rpc.test.ts` - test protocol validation, parsing, errors, serialization
- Create mock daemon helper - simulate stdout, crashes, slow responses, malformed output
- Test all edge cases identified in Ticket 0 review
- Add memory leak test (1000 requests, measure growth with gc())
- Achieve > 80% code coverage

**Acceptance Criteria:**
- All unit tests pass (100%)
- Code coverage > 80% (measured by vitest)
- Mocks simulate realistic daemon behavior (normal, slow, crash, malformed)
- Edge cases tested (crashes, timeouts, malformed input, orphaned responses)
- Memory leak test passes (<10MB growth over 1000 requests)
- Circuit breaker test passes (triggers after 5 crashes)

**Dependencies:** Ticket 2 (core complete), Ticket 3 (RPC complete)

**Agent:** unit-test-runner

**Estimated Effort:** 1 day

---

### Phase 2: Integration

#### Ticket 5: MCP Server Daemon Integration
**Description:** Replace spawning with DaemonClient in MCP search tool

**Tasks:**
- Modify `packages/maproom-mcp/src/tools/search.ts`
- Replace spawning logic (lines 233-291) with daemon.search()
- Import DaemonClient from daemon-client package
- Preserve chunk ID fetching (lines 307-343)
- Handle errors (convert RpcError to MCP error)

**Acceptance Criteria:**
- MCP search uses daemon (no spawning)
- Chunk IDs fetched correctly
- Errors handled gracefully (user-friendly messages)
- Existing search functionality preserved

**Dependencies:** Ticket 4 (unit tests passing)

**Agent:** general-purpose

---

#### Ticket 6: Singleton Management
**Description:** Implement daemon singleton for MCP server

**Tasks:**
- Create `packages/maproom-mcp/src/daemon.ts`
- Implement getDaemonClient() (singleton factory)
- Implement closeDaemonClient() (cleanup)
- Add SIGTERM handler (graceful shutdown)
- Configure daemon (binary path, env vars, timeouts)

**Acceptance Criteria:**
- One daemon per MCP server instance
- Daemon shared across search invocations
- Graceful shutdown on SIGTERM
- Environment variables passed correctly

**Dependencies:** Ticket 5 (MCP integration)

**Agent:** general-purpose

---

#### Ticket 7: Integration Tests
**Description:** End-to-end tests for MCP search via daemon

**Tasks:**
- Create `packages/maproom-mcp/tests/search-integration.test.ts`
- Test basic search (via daemon, chunk IDs included)
- Test daemon lifecycle (start on first request, reuse)
- Test concurrent requests (10, 50 simultaneous)
- Test error scenarios (repo not found, daemon crash)

**Acceptance Criteria:**
- All integration tests pass
- Real daemon, real database, real MCP code
- Concurrent requests work correctly
- Error handling verified

**Dependencies:** Ticket 6 (singleton management)

**Agent:** integration-tester

---

### Phase 3: Validation

#### Ticket 8: Performance Testing
**Description:** Validate latency targets, resource usage, and connection pool behavior

**Tasks:**
- Create `packages/daemon-client/tests/performance.test.ts`
- Measure cold start latency (first request)
- Measure warm request latency (subsequent)
- Test sequential load (100 requests)
- Test concurrent load (50 requests)
- Test pool exhaustion behavior (spawn concurrent > pool_size, verify queuing/timeout)
- Measure memory usage (1000 requests)
- Document connection pool sizing recommendations

**Acceptance Criteria:**
- Cold start < 600ms
- Warm requests < 60ms (median)
- No memory leaks (< 10MB growth over 1000 requests)
- Throughput > 50 req/s
- Pool exhaustion handled gracefully (requests queue then timeout, no crashes)
- Connection pool sizing documented (formula: pool_size >= concurrent/2)

**Dependencies:** Ticket 7 (integration tests passing)

**Agent:** general-purpose

**Estimated Effort:** 1 day

---

#### Ticket 9: Stress Testing
**Description:** Stress tests and resource leak detection

**Tasks:**
- Create `packages/daemon-client/tests/stress.test.ts`
- Test long-running stability (10,000 requests)
- Test high concurrency (1,000 simultaneous)
- Test daemon crash recovery (10 crashes)
- Test rapid start/stop (100 cycles)
- Monitor file descriptors, connections

**Acceptance Criteria:**
- System stable under stress
- Daemon recovers from crashes
- No file descriptor leaks
- No database connection leaks

**Dependencies:** Ticket 8 (performance tests)

**Agent:** general-purpose

---

#### Ticket 10: Regression Testing
**Description:** Verify no functionality lost from spawning approach

**Tasks:**
- Create `packages/maproom-mcp/tests/regression.test.ts`
- Compare results (daemon vs spawning)
- Test all search modes (fts, vector, hybrid)
- Test all filters (repo, worktree, file_type)
- Test debug mode, large results, empty results

**Acceptance Criteria:**
- Results identical to spawning approach
- All existing search scenarios work
- No performance regressions (warm faster, cold acceptable)
- Error messages equivalent or better

**Dependencies:** Ticket 7 (integration tests)

**Agent:** verify-ticket

---

### Phase 4: Polish

#### Ticket 11: Documentation
**Description:** Complete README, API docs, migration guide

**Tasks:**
- Write `packages/daemon-client/README.md`
- Write API documentation (DaemonClient, config, errors)
- Write migration guide (MCP server example)
- Write troubleshooting guide (common issues, debugging)
- Update root CLAUDE.md (reference daemon-client)

**Acceptance Criteria:**
- README complete (installation, usage, examples)
- API docs complete (all public methods documented)
- Migration guide tested by external user
- Troubleshooting guide covers common issues

**Dependencies:** Ticket 10 (regression tests passing)

**Agent:** general-purpose

---

#### Ticket 12: Security Documentation
**Description:** Document security considerations and best practices

**Tasks:**
- Document environment variable credential exposure
- Document resource limits recommendations
- Document binary integrity considerations
- Provide incident response guide
- Recommend secrets management for production

**Acceptance Criteria:**
- Security review (from planning) summarized in README
- Deployment best practices documented
- Incident response procedures documented
- Compliance considerations noted

**Dependencies:** Ticket 11 (documentation)

**Agent:** general-purpose

---

#### Ticket 13: Code Cleanup
**Description:** Mark old code deprecated, final cleanup

**Tasks:**
- Add deprecation notice to `trySpawnWithCandidates()` (keep for VSCode)
- Add comments explaining daemon migration
- Remove unused imports
- Final linting pass
- Update CHANGELOG

**Acceptance Criteria:**
- Old spawning code marked deprecated (not removed)
- Comments explain migration path
- All linters pass
- CHANGELOG updated with breaking changes

**Dependencies:** Ticket 12 (security docs)

**Agent:** general-purpose

---

## Risk Mitigation

### Risk: Daemon Instability in Production

**Mitigation Strategy:**
- Comprehensive testing (unit, integration, stress)
- Auto-restart with circuit breaker (max 5 attempts)
- Optional fallback to spawning (configuration flag)
- Monitoring and alerting (restart rate, error rate)

**Contingency Plan:**
- If daemon restart rate > 10%, revert to spawning
- Add feature flag to disable daemon (emergency rollback)
- Investigate crashes, fix root cause, redeploy

### Risk: Performance Regression

**Mitigation Strategy:**
- Performance tests with strict targets (cold < 600ms, warm < 60ms)
- Regression tests comparing to spawning approach
- Profiling and optimization (if needed)

**Contingency Plan:**
- If performance worse than spawning, investigate bottleneck
- Optimize hot paths (serialization, IPC, parsing)
- If unfixable, document trade-off, adjust expectations

### Risk: Integration Issues (MCP Server)

**Mitigation Strategy:**
- Integration tests with real daemon, real database
- Regression tests ensuring functionality preserved
- Incremental rollout (feature flag, gradual migration)

**Contingency Plan:**
- If integration broken, rollback MCP changes
- Keep old spawning code (safety net)
- Fix integration issue, redeploy

### Risk: Delayed Timeline

**Mitigation Strategy:**
- Ticket-based tracking (clear progress visibility)
- Phase gates (can't proceed without previous phase complete)
- Buffer time in estimates (~20% contingency)

**Contingency Plan:**
- If timeline slips, prioritize core functionality (Phase 1-2)
- Defer polish (Phase 4) to post-MVP
- Ship with "beta" label, iterate based on feedback

## Dependencies and Blockers

### External Dependencies

**None** - All dependencies already in place:
- ✅ Rust daemon (`crewchief-maproom serve`) implemented
- ✅ PostgreSQL schema stable
- ✅ Binary distribution working

### Internal Dependencies

**Phase 2 blocked by Phase 1:**
- MCP integration requires daemon-client package complete

**Phase 3 blocked by Phase 2:**
- Performance testing requires MCP integration complete

**Phase 4 blocked by Phase 3:**
- Documentation requires testing complete

### Technical Blockers

**None anticipated** - Prototype has been proven, architecture is sound

## Success Metrics (Review)

### Performance (from quality-strategy.md)
- ✅ Cold start < 600ms
- ✅ Warm requests < 60ms
- ✅ Throughput > 50 req/s
- ✅ Memory stable (< 100MB, no leaks)

### Quality (from quality-strategy.md)
- ✅ Unit test coverage > 80%
- ✅ All integration tests pass
- ✅ All regression tests pass
- ✅ No performance regressions

### Adoption (from analysis.md)
- ✅ MCP server uses daemon (100%)
- ✅ Daemon restart rate < 1%
- ✅ Zero critical user-reported issues

## Timeline Estimate

**Phase 1:** 1-2 days (foundation - reduced due to existing implementation ~50-70% complete)
  - Ticket 0: 0.5 days (code review)
  - Ticket 1: 0.25 days (complete package config)
  - Ticket 2: 0.5 days (complete core implementation)
  - Ticket 3: 0.25 days (complete RPC/errors)
  - Ticket 4: 1 day (create unit tests)

**Phase 2:** 2-3 days (integration)
  - Ticket 5: 1 day (MCP server daemon integration)
  - Ticket 6: 0.5 days (singleton management)
  - Ticket 7: 1 day (integration tests)

**Phase 3:** 2-3 days (validation)
  - Ticket 8: 1 day (performance testing)
  - Ticket 9: 1 day (stress testing)
  - Ticket 10: 1 day (regression testing)

**Phase 4:** 1-2 days (polish)
  - Ticket 11: 1 day (documentation)
  - Ticket 12: 0.5 days (security documentation)
  - Ticket 13: 0.5 days (code cleanup)

**Total:** 6-10 days (with contingency) - reduced from 8-13 days

**Note:** Timeline is for ticket completion, not including review/approval cycles. Reduced estimate reflects existing implementation (~50-70% complete) and focused scope.

## Rollout Strategy

### Phase 1: Canary (Internal Testing)
- Deploy to development environment
- Internal team testing (developers, QA)
- Monitor daemon stability, performance
- Fix critical issues before wider rollout

### Phase 2: Beta (Limited Users)
- Deploy to subset of MCP users (opt-in)
- Collect feedback, telemetry
- Monitor error rates, restart rates
- Iterate based on feedback

### Phase 3: General Availability
- Deploy to all MCP users
- Monitor production metrics
- Respond to incidents quickly
- Continuous improvement based on data

## Post-MVP Roadmap

### VSCode Extension Migration (Phase 2 Project)
**Priority:** MEDIUM
**Effort:** Medium (reuse daemon-client)
**Impact:** Medium (scan command, infrequent operation)

### Shared Daemon Exploration (Phase 3 Project)
**Priority:** LOW
**Effort:** High (socket IPC, lifecycle management)
**Impact:** Low (resource optimization, not user-facing)

### Additional Tools via Daemon (Future)
**Priority:** LOW
**Effort:** Low (reuse existing daemon)
**Impact:** Low (`context`, `upsert` latency improvements)

---

**Implementation Plan Complete:** 2025-11-22
**Status:** Ready for ticket creation
**Next Step:** Use `/create-project-tickets DAEMIGR` to generate tickets

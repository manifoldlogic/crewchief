# VSCDAEMN Implementation Plan

## Overview

Migrate VSCode extension's `scan` command from process spawning to daemon-client pattern, achieving consistency with MCP server and enabling removal of deprecated spawning utilities.

## Phase Organization

### Phase 1: Daemon Integration (1-2 days)
**Goal**: Integrate daemon-client package into VSCode extension

**Deliverables**:
- `src/daemon/index.ts` - Daemon singleton management
- daemon-client added to package.json dependencies
- Extension imports and configures DaemonClient

**Success Criteria**:
- Daemon starts on first scan trigger
- Environment variables passed correctly
- Binary path resolved from extension root
- Daemon shutdown on extension deactivation

**Agent**: general-purpose

**Tickets**:
- VSCDAEMN-1001: Add daemon-client dependency
- VSCDAEMN-1002: Create daemon singleton module
- VSCDAEMN-1003: Wire daemon to extension lifecycle

### Phase 2: Scan Migration (1-2 days)
**Goal**: Replace spawning with daemon in scan operation

**Deliverables**:
- `src/process/scan.ts` - Modified to use DaemonClient.scan()
- Progress handling adapted for JSON-RPC responses
- Error handling for daemon failures
- VSCode progress API integration

**Success Criteria**:
- Scan completes successfully via daemon
- Progress notification displays correctly
- Status bar updates on completion
- Error messages user-friendly

**Agent**: general-purpose

**Tickets**:
- VSCDAEMN-2001: Migrate scan.ts to use daemon
- VSCDAEMN-2002: Adapt progress handling for RPC
- VSCDAEMN-2003: Error handling and fallback

### Phase 3: Testing & Validation (1 day)
**Goal**: Comprehensive testing and validation

**Deliverables**:
- Unit tests for daemon integration (> 80% coverage)
- Integration tests for full scan E2E
- Regression tests (backward compatibility)

**Success Criteria**:
- All unit tests pass
- Integration tests pass (real daemon, real database)
- Regression tests pass (same results as spawning)
- Performance targets met (cold < 300ms, warm < 100ms)

**Agent**: unit-test-runner, integration-tester

**Tickets**:
- VSCDAEMN-3001: Unit tests for daemon module
- VSCDAEMN-3002: Integration tests for scan
- VSCDAEMN-3003: Regression tests

### Phase 4: Cleanup & Documentation (1 day)
**Goal**: Remove deprecated code and update documentation

**Deliverables**:
- Remove `packages/maproom-mcp/src/utils/process.ts`
- Remove spawning utilities from `src/utils/index.ts`
- Update extension README with security notes
- Update CHANGELOG with migration details

**Success Criteria**:
- Deprecated spawning code removed
- No references to trySpawnWithCandidates()
- Documentation updated and reviewed
- All tests still passing

**Agent**: general-purpose

**Tickets**:
- VSCDAEMN-4001: Remove deprecated spawning code
- VSCDAEMN-4002: Update extension documentation
- VSCDAEMN-4003: Update CHANGELOG

## Agent Assignments

### Phase 1: Daemon Integration
- **general-purpose**: Package setup, daemon module, extension lifecycle

### Phase 2: Scan Migration
- **general-purpose**: Scan migration, progress adaptation, error handling

### Phase 3: Testing & Validation
- **unit-test-runner**: Unit tests for daemon integration
- **integration-tester**: E2E tests for scan operation
- **general-purpose**: Regression tests

### Phase 4: Cleanup & Documentation
- **general-purpose**: Code cleanup, documentation updates

## Detailed Phase Plans

### Phase 1: Daemon Integration

**Ticket VSCDAEMN-1001: Add daemon-client dependency**
- Add `@maproom/daemon-client` to package.json
- Run `pnpm install`
- Verify TypeScript types resolve

**Ticket VSCDAEMN-1002: Create daemon singleton module**
- Create `src/daemon/index.ts`
- Implement `getDaemonClient(config)`
- Implement `shutdownDaemon()`
- Implement `isDaemonHealthy()`

**Ticket VSCDAEMN-1003: Wire daemon to extension lifecycle**
- Import daemon module in `extension.ts`
- Call `getDaemonClient()` on first scan
- Call `shutdownDaemon()` in `deactivate()`
- Add error handling for daemon start failure

### Phase 2: Scan Migration

**Ticket VSCDAEMN-2001: Migrate scan.ts to use daemon**
- Replace `spawn()` with `getDaemonClient().scan()`
- Remove `StdoutParser` usage
- Pass workspace path and config to daemon.scan()

**Ticket VSCDAEMN-2002: Adapt progress handling for RPC**
- Implement `onProgress` callback for scan
- Map RPC progress events to VSCode progress API
- Maintain same UX (notification with percentage)

**Ticket VSCDAEMN-2003: Error handling and fallback**
- Handle DaemonStartError (show user-friendly message)
- Handle DaemonTimeoutError (suggest checking PostgreSQL)
- Handle DaemonCrashError (offer to restart)
- Log errors to output channel

### Phase 3: Testing & Validation

**Ticket VSCDAEMN-3001: Unit tests for daemon module**
- Test `getDaemonClient()` singleton behavior
- Test daemon configuration (binary path, env vars)
- Test `shutdownDaemon()` cleanup
- Test error scenarios

**Ticket VSCDAEMN-3002: Integration tests for scan**
- Test full scan E2E (real daemon, real database)
- Test progress callback invocation
- Test concurrent scans
- Test daemon crash recovery

**Ticket VSCDAEMN-3003: Regression tests**
- Compare scan results (daemon vs spawning)
- Test all existing commands still work
- Test no breaking changes to user workflows

### Phase 4: Cleanup & Documentation

**Ticket VSCDAEMN-4001: Remove deprecated spawning code**
- Delete `packages/maproom-mcp/src/utils/process.ts`
- Remove spawning utilities from `src/utils/index.ts`
- Remove any remaining references to `trySpawnWithCandidates()`
- Verify all tests still pass

**Ticket VSCDAEMN-4002: Update extension documentation**
- Add security notes to README (credential exposure)
- Document daemon lifecycle and auto-restart
- Add troubleshooting section (daemon won't start, etc.)
- Update architecture diagram

**Ticket VSCDAEMN-4003: Update CHANGELOG**
- Document breaking changes (none expected)
- Document performance improvements (20-50x)
- Document new daemon dependency
- Link to DAEMIGR migration guide

## Success Metrics

### Performance
- Cold scan latency < 300ms
- Warm scan latency < 100ms
- Extension activation < 500ms (no regression)
- Memory usage < 150MB baseline

### Quality
- Unit test coverage > 80%
- Integration tests pass (100%)
- Regression tests pass (0 regressions)
- Manual testing on 3 platforms (macOS, Linux, Windows)

### Adoption
- Extension uses daemon for scan (100%)
- Deprecated spawning code removed (100%)
- All tests passing after migration

## Risk Mitigation

| Risk | Mitigation | Contingency |
|------|-----------|-------------|
| Daemon fails to start | Auto-restart with circuit breaker | Show error message, manual restart command |
| Progress events lost | Test progress callback thoroughly | Fallback to simple progress indicator |
| Extension activation slower | Start daemon asynchronously | No blocking during activation |
| Database unavailable | postgres-checker prevents daemon start | Show error, link to setup wizard |

## Timeline Estimate

**Phase 1**: 1-2 days (daemon integration)
**Phase 2**: 1-2 days (scan migration)
**Phase 3**: 1 day (testing & validation)
**Phase 4**: 1 day (cleanup & documentation)

**Total**: 4-6 days (with contingency buffer)

## Rollout Plan

1. **Internal Testing**: Developer machines, 3 platforms
2. **Beta Release**: Pre-release version (0.2.0-beta.1)
3. **Stable Release**: Stable version (0.2.0)
4. **Monitor**: Crash reports, error logs, user feedback

## Conclusion

The VSCode extension daemon migration is a **well-scoped, low-risk project** with clear deliverables, success metrics, and rollout plan. Proceed with Phase 1 (Daemon Integration).

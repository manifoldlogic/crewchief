# MCPSTART: MCP Provider Startup Fix - Implementation Plan

## Project Overview

**Goal**: Fix the MCP provider startup issue where Ollama containers start despite explicit configuration for Google Vertex AI or OpenAI embeddings.

**Success Criteria**:
1. When `EMBEDDING_PROVIDER=google`, Ollama does NOT start
2. When `EMBEDDING_PROVIDER=openai`, Ollama does NOT start
3. When `EMBEDDING_PROVIDER=ollama` or unset, Ollama DOES start
4. Fix works with published npm package (`npx @crewchief/maproom-mcp@latest`)
5. Diagnostic logging provides clear visibility into configuration

## Implementation Phases

### Phase 1: Diagnostic Infrastructure (v1.1.8)

**Goal**: Add comprehensive logging to understand what's actually happening

**Deliverables**:
- Environment variable logging at CLI startup
- Docker command logging before execution
- Container state logging after operations
- Redacted output for sensitive values

**Agent**: `docker-engineer`

**Tickets**:
1. Add environment variable diagnostic logging
2. Add Docker command execution logging
3. Add container state verification logging
4. Implement credential redaction in logs

**Testing**: Verify diagnostic logs show expected information

**Risks**: None - purely additive, no behavior changes

**Timeline**: 1-2 days

---

### Phase 2: Environment Propagation Fix (v1.1.9)

**Goal**: Ensure environment variables flow correctly from MCP client to Docker Compose

**Deliverables**:
- Explicit environment passing in all `spawn()` calls
- Docker Compose config file verification
- Environment variable presence validation

**Agent**: `docker-engineer`

**Tickets**:
1. Add explicit `env` parameter to all spawn() calls
2. Implement docker-compose.yml verification on startup
3. Add validation that required env vars are present for non-default providers

**Testing**: Integration tests verify env vars reach Docker Compose

**Risks**: Medium - modifying spawn behavior could introduce regressions

**Timeline**: 2-3 days

---

### Phase 3: Clean State Management (v1.1.9)

**Goal**: Ensure containers are in expected state before operations

**Deliverables**:
- Pre-flight container cleanup
- Explicit service removal for unnecessary services
- Graceful handling of existing containers

**Agent**: `docker-engineer`

**Tickets**:
1. Implement pre-flight container state check
2. Add explicit stop and remove for unnecessary services
3. Add verification of final container state

**Testing**: Integration tests verify correct containers running after startup

**Risks**: Medium - stopping containers could affect running services

**Timeline**: 2-3 days

---

### Phase 4: Integration Testing (Parallel with Phases 2-3)

**Goal**: Comprehensive automated testing to prevent regressions

**Deliverables**:
- Bash script with 7 critical test cases
- Container state verification
- Published package testing
- CI/CD integration

**Agent**: `integration-tester`

**Tickets**:
1. Create integration test script framework
2. Implement 5 automated test cases
3. Add diagnostic log verification
4. Add published package test
5. Create CI/CD workflow (optional)

**Testing**: Tests themselves are the validation

**Risks**: Low - tests don't affect production code

**Timeline**: 2-3 days (parallel with implementation)

---

### Phase 5: Security Hardening (v1.1.9)

**Goal**: Address security considerations identified in review

**Deliverables**:
- Services bound to localhost
- Credential redaction (from Phase 1)
- npm audit check before publish

**Agent**: `docker-engineer`

**Tickets**:
1. Update docker-compose.yml to bind to 127.0.0.1
2. Add npm audit to prepublishOnly script
3. Document security best practices

**Testing**: Manual verification of service bindings and audit checks

**Risks**: Low - defensive improvements

**Timeline**: 1 day

---

### Phase 6: Documentation & Publishing (v1.1.9)

**Goal**: Document the fix and publish to npm

**Deliverables**:
- Updated README with troubleshooting section
- Security documentation
- Configuration examples
- Published npm package

**Agent**: `docker-engineer`

**Tickets**:
1. Update README with troubleshooting guide
2. Create configuration examples
3. Update CHANGELOG
4. Publish to npm with 2FA

**Testing**: Manual verification with real MCP client

**Risks**: Low - documentation changes

**Timeline**: 1 day

---

### Phase 7: Service Profiles (Optional - v1.2.0)

**Goal**: Long-term architectural improvement using Docker Compose profiles

**Deliverables**:
- docker-compose.yml with profile-based service definitions
- CLI updated to use `--profile` flags
- Docker Compose version compatibility check

**Agent**: `docker-engineer`

**Tickets**:
1. Add profile definitions to docker-compose.yml
2. Update CLI to use profile-based service selection
3. Add Docker Compose version detection
4. Implement fallback for older versions

**Testing**: Integration tests updated for profile-based startup

**Risks**: Medium - architectural change, compatibility issues

**Timeline**: 3-4 days

**Decision**: Ship Phases 1-6 first, then evaluate if Phase 7 is needed based on user feedback

---

## Ticket Creation Order

**Week 1** (Phases 1-2):
1. MCPSTART-001: Add environment variable diagnostic logging
2. MCPSTART-002: Add Docker command execution logging
3. MCPSTART-003: Add container state verification logging
4. MCPSTART-004: Implement credential redaction in logs
5. MCPSTART-005: Add explicit env parameter to spawn() calls
6. MCPSTART-006: Implement docker-compose.yml verification
7. MCPSTART-007: Add env var presence validation

**Week 2** (Phases 3-5):
8. MCPSTART-008: Implement pre-flight container cleanup
9. MCPSTART-009: Add explicit stop/remove for unnecessary services
10. MCPSTART-010: Add final container state verification
11. MCPSTART-011: Create integration test framework
12. MCPSTART-012: Implement automated test cases
13. MCPSTART-013: Update docker-compose.yml localhost binding
14. MCPSTART-014: Add npm audit check

**Week 3** (Phase 6 + Polish):
15. MCPSTART-015: Update README with troubleshooting
16. MCPSTART-016: Create configuration examples
17. MCPSTART-017: Manual testing with real MCP clients
18. MCPSTART-018: Publish v1.1.9 to npm

**Future** (Phase 7 - Optional):
19. MCPSTART-019: Add profile definitions to docker-compose.yml
20. MCPSTART-020: Update CLI for profile-based startup
21. MCPSTART-021: Add Docker Compose version detection
22. MCPSTART-022: Implement version fallback logic

## Dependencies

**External Dependencies**:
- Docker Compose v2.0+ (for profiles in Phase 7)
- npm access with 2FA for publishing
- Real MCP client (Claude Desktop or Cursor) for manual testing

**Internal Dependencies**:
- Phase 2 requires Phase 1 (diagnostic logs help debug env propagation)
- Phase 3 requires Phase 2 (clean state depends on knowing what to clean)
- Phase 6 requires Phases 1-5 (can't publish until tests pass)
- Phase 7 is independent (can be done anytime post-v1.1.9)

## Critical Path

```
Phase 1 (Diagnostics)
    ↓
Phase 2 (Env Propagation) ← Phase 4 (Testing) in parallel
    ↓
Phase 3 (Clean State)
    ↓
Phase 5 (Security)
    ↓
Phase 6 (Documentation & Publish)
    ↓
Phase 7 (Profiles) - Optional
```

**Minimum Viable Fix**: Phases 1-3 + sufficient testing
**Production Ready**: Phases 1-6
**Best-in-Class**: Phases 1-7

## Testing Strategy by Phase

**Phase 1**: Manual verification of diagnostic logs
**Phase 2**: Integration tests verify env vars present
**Phase 3**: Integration tests verify container state
**Phase 4**: Test suite itself is the deliverable
**Phase 5**: Manual security verification
**Phase 6**: Manual testing with real MCP client

## Rollout Strategy

**v1.1.8** (Diagnostics):
- Safe to ship immediately
- No behavior changes
- Helps users debug their own issues

**v1.1.9** (Full Fix):
- Ship after all integration tests pass
- Manual verification with at least one real MCP client
- Monitor for issues in first 48 hours

**v1.2.0** (Profiles - Optional):
- Ship only if users report issues with current approach
- Requires Docker Compose v2.0+ (breaking change)
- Provide migration guide

## Success Metrics

**Phase 1 Complete When**:
- ✅ Diagnostic logs show all environment variables
- ✅ Docker commands logged before execution
- ✅ Container state logged after operations
- ✅ Sensitive values redacted

**Phase 2 Complete When**:
- ✅ Integration tests verify env vars reach CLI
- ✅ docker-compose.yml validation passes
- ✅ Required env vars checked before startup

**Phase 3 Complete When**:
- ✅ Pre-flight cleanup removes old containers
- ✅ Unnecessary services stopped and removed
- ✅ Final state matches expected state

**Phase 4 Complete When**:
- ✅ All 7 critical test cases pass
- ✅ Tests run reliably (no flakiness)
- ✅ Clear output on pass/fail

**Phase 5 Complete When**:
- ✅ Services bound to localhost only
- ✅ npm audit passes before publish
- ✅ Security docs added to README

**Phase 6 Complete When**:
- ✅ README has troubleshooting section
- ✅ Configuration examples provided
- ✅ Manual test with real MCP client passes
- ✅ Published to npm successfully

**Project Complete When**:
- ✅ All Phase 1-6 deliverables complete
- ✅ User reports confirm fix works
- ✅ No regressions for Ollama users
- ✅ Diagnostics help users self-serve

## Risk Management

**Risk**: Changes break existing Ollama users
**Mitigation**: Comprehensive testing, default to Ollama behavior
**Response**: Fast rollback, emergency patch release

**Risk**: Published package behaves differently than local
**Mitigation**: Test actual published package before announcing
**Response**: Investigate npx behavior, add workaround

**Risk**: MCP clients don't pass env vars correctly
**Mitigation**: Clear error messages, fallback to zero-config
**Response**: Document per-client configuration, report to client authors

**Risk**: Integration tests are flaky
**Mitigation**: Aggressive cleanup, clear timeouts, retry logic
**Response**: Debug and fix flakiness before proceeding

## Resource Requirements

**Time**: 2-3 weeks for Phases 1-6
**Human Input**:
- 2FA code for npm publishing
- Manual testing with real MCP client
- Final approval before publish

**Tools Needed**:
- Docker and Docker Compose installed
- Node.js 18+
- npm account with publish access
- MCP client (Claude Desktop or Cursor) for testing

## Communication Plan

**During Development**:
- Mark ticket checkboxes as work progresses
- Log key decisions in ticket comments
- Update diagnostic logs with findings

**Before Publishing**:
- Update CHANGELOG with all changes
- Test with real MCP client
- Verify all acceptance criteria met

**After Publishing**:
- Monitor GitHub issues for reports
- Be ready to hotfix if critical issues found
- Document common problems in README

## Conclusion

This is a **phased approach** that builds confidence incrementally:

1. **Phase 1**: See what's happening (diagnostics)
2. **Phase 2**: Fix the root cause (env propagation)
3. **Phase 3**: Ensure reliability (clean state)
4. **Phase 4**: Prevent regressions (testing)
5. **Phase 5**: Cover security bases (hardening)
6. **Phase 6**: Ship it (documentation & publish)
7. **Phase 7**: Improve architecture (profiles - optional)

Each phase delivers value independently, allowing early shipment if needed. The critical path is clear, dependencies are minimal, and rollback is straightforward.

**Target Ship Date**: v1.1.9 ready in 2-3 weeks

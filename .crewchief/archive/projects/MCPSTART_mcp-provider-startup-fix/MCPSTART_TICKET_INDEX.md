# MCPSTART Ticket Index

## Project Overview

**Project**: MCP Provider Startup Fix
**Project Slug**: MCPSTART
**Goal**: Fix Ollama container startup when EMBEDDING_PROVIDER=google/openai configured
**Target Version**: v1.1.9 (Phases 1-6), v1.2.0 (Phase 7 optional)

**Problem**: Despite two previous fix attempts (MCP-008, MCP-011), Ollama containers still start when users explicitly configure Google Vertex AI or OpenAI embeddings in `.mcp.json`. This breaks the provider selection logic and wastes resources.

**Root Cause**: Environment variables from `.mcp.json` are not propagating correctly through the MCP client → CLI → Docker Compose chain.

**Project Documents**:
- Analysis: `/workspace/.crewchief/projects/MCPSTART-mcp-provider-startup-fix/MCPSTART_ANALYSIS.md`
- Architecture: `/workspace/.crewchief/projects/MCPSTART-mcp-provider-startup-fix/MCPSTART_ARCHITECTURE.md`
- Quality Strategy: `/workspace/.crewchief/projects/MCPSTART-mcp-provider-startup-fix/MCPSTART_QUALITY_STRATEGY.md`
- Security Review: `/workspace/.crewchief/projects/MCPSTART-mcp-provider-startup-fix/MCPSTART_SECURITY_REVIEW.md`
- Plan: `/workspace/.crewchief/projects/MCPSTART-mcp-provider-startup-fix/MCPSTART_PLAN.md`

---

## Phase 1: Diagnostic Infrastructure (v1.1.8)

**Goal**: Add comprehensive logging to understand what's happening
**Timeline**: 1-2 days
**Risk**: None - purely additive

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| MCPSTART-1001 | Add environment variable diagnostic logging | docker-engineer | Pending | None |
| MCPSTART-1002 | Add Docker command execution logging | docker-engineer | Pending | 1001 |
| MCPSTART-1003 | Add container state verification logging | docker-engineer | Pending | 1001, 1002 |
| MCPSTART-1004 | Implement credential redaction in logs | docker-engineer | Pending | 1001, 1002, 1003 |

**Deliverables**:
- Environment variable logging at CLI startup
- Docker command logging before execution
- Container state logging after operations
- Redacted output for sensitive values

**Success Criteria**:
- ✅ Diagnostic logs show all environment variables
- ✅ Docker commands logged before execution
- ✅ Container state logged after operations
- ✅ Sensitive values redacted

---

## Phase 2: Environment Propagation Fix (v1.1.9)

**Goal**: Ensure environment variables flow correctly from MCP client to Docker Compose
**Timeline**: 2-3 days
**Risk**: Medium - modifying spawn behavior

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| MCPSTART-2001 | Add explicit env parameter to spawn() calls | docker-engineer | Pending | Phase 1 complete |
| MCPSTART-2002 | Implement docker-compose.yml verification | docker-engineer | Pending | 2001 |
| MCPSTART-2003 | Add validation for required env vars per provider | docker-engineer | Pending | 2001, 2002 |

**Deliverables**:
- Explicit environment passing in all spawn() calls
- Docker Compose config file verification
- Environment variable presence validation

**Success Criteria**:
- ✅ Integration tests verify env vars reach CLI
- ✅ docker-compose.yml validation passes
- ✅ Required env vars checked before startup

**CRITICAL**: MCPSTART-2001 is the **CORE FIX** for the Ollama startup issue

---

## Phase 3: Clean State Management (v1.1.9)

**Goal**: Ensure containers are in expected state before operations
**Timeline**: 2-3 days
**Risk**: Medium - stopping containers affects state

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| MCPSTART-3001 | Implement pre-flight container state check | docker-engineer | Pending | Phase 2 complete |
| MCPSTART-3002 | Add explicit stop and remove for unnecessary services | docker-engineer | Pending | 3001 |
| MCPSTART-3003 | Add verification of final container state | docker-engineer | Pending | 3001, 3002 |

**Deliverables**:
- Pre-flight container cleanup
- Explicit service removal for unnecessary services
- Graceful handling of existing containers

**Success Criteria**:
- ✅ Pre-flight cleanup removes old containers
- ✅ Unnecessary services stopped and removed
- ✅ Final state matches expected state

---

## Phase 4: Integration Testing (Parallel with Phases 2-3)

**Goal**: Comprehensive automated testing to prevent regressions
**Timeline**: 2-3 days (parallel)
**Risk**: Low - tests don't affect production code

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| MCPSTART-4001 | Create integration test script framework | integration-tester | Pending | None (can start early) |
| MCPSTART-4002 | Implement automated test cases for provider startup | integration-tester | Pending | 4001 |

**Deliverables**:
- Bash script with 7 critical test cases
- Container state verification
- Published package testing
- CI/CD integration (optional)

**Test Cases**:
1. Google provider - Ollama does NOT start
2. Default (no provider) - Ollama DOES start
3. OpenAI provider - Ollama does NOT start
4. Explicit EMBEDDING_PROVIDER=ollama - Ollama DOES start
5. Diagnostic logs verification
6. Published package works via npx
7. Service selection messaging

**Success Criteria**:
- ✅ All 7 critical test cases pass
- ✅ Tests run reliably (no flakiness)
- ✅ Clear output on pass/fail

---

## Phase 5: Security Hardening (v1.1.9)

**Goal**: Address security considerations identified in review
**Timeline**: 1 day
**Risk**: Low - defensive improvements

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| MCPSTART-5001 | Update docker-compose.yml to bind to localhost | docker-engineer | Pending | None |
| MCPSTART-5002 | Add npm audit to prepublishOnly script | docker-engineer | Pending | None |
| MCPSTART-5003 | Document security best practices | docker-engineer | Pending | 5001, 5002 |

**Deliverables**:
- Services bound to localhost (127.0.0.1)
- npm audit check before publish
- Security documentation

**Success Criteria**:
- ✅ Services bound to localhost only
- ✅ npm audit passes before publish
- ✅ Security docs added to README

---

## Phase 6: Documentation & Publishing (v1.1.9)

**Goal**: Document the fix and publish to npm
**Timeline**: 1 day
**Risk**: Low - documentation changes

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| MCPSTART-6001 | Update README with troubleshooting guide | docker-engineer | Pending | Phases 1-5 complete |
| MCPSTART-6002 | Create configuration examples file | docker-engineer | Pending | None |
| MCPSTART-6003 | Update CHANGELOG for v1.1.9 release | docker-engineer | Pending | Phases 1-5 complete |
| MCPSTART-6004 | Publish v1.1.9 to npm with 2FA | docker-engineer | Pending | ALL previous tickets |

**Deliverables**:
- Updated README with troubleshooting section
- docker-compose.env.example with all provider configs
- CHANGELOG documenting all changes
- Published npm package

**Success Criteria**:
- ✅ README has troubleshooting section
- ✅ Configuration examples provided
- ✅ Manual test with real MCP client passes
- ✅ Published to npm successfully

**CRITICAL**: MCPSTART-6004 requires human input for 2FA code

---

## Phase 7: Service Profiles (Optional - v1.2.0)

**Goal**: Long-term architectural improvement using Docker Compose profiles
**Timeline**: 3-4 days
**Risk**: Medium - architectural change
**Decision**: Ship v1.1.9 first, evaluate Phase 7 based on user feedback

| Ticket ID | Title | Agent | Status | Dependencies |
|-----------|-------|-------|--------|--------------|
| MCPSTART-7001 | Add profile definitions to docker-compose.yml | docker-engineer | Pending | v1.1.9 shipped |
| MCPSTART-7002 | Update CLI to use profile-based service selection | docker-engineer | Pending | 7001 |
| MCPSTART-7003 | Add Docker Compose version detection | docker-engineer | Pending | 7002 |
| MCPSTART-7004 | Update integration tests for profile-based startup | integration-tester | Pending | 7001, 7002, 7003 |

**Deliverables**:
- docker-compose.yml with profile-based service definitions
- CLI updated to use --profile flags
- Docker Compose version compatibility check
- Updated integration tests

**Success Criteria**:
- ✅ Profiles work with Docker Compose v2.0+
- ✅ Fallback for older versions (if applicable)
- ✅ Integration tests updated and passing
- ✅ Cleaner architecture than Phase 3 approach

**Decision Criteria**: Proceed with Phase 7 only if:
- User reports indicate Phase 1-6 fix is insufficient
- There's demand for cleaner architecture
- Docker Compose v2.0+ adoption is universal

---

## Critical Path

```
Phase 1 (Diagnostics) → Phase 2 (Env Propagation) → Phase 3 (Clean State)
                              ↓
                         Phase 4 (Testing - parallel)
                              ↓
                         Phase 5 (Security)
                              ↓
                         Phase 6 (Documentation & Publish)
                              ↓
                       [Optional] Phase 7 (Profiles)
```

**Minimum Viable Fix**: Phases 1-3 + sufficient testing
**Production Ready**: Phases 1-6
**Best-in-Class**: Phases 1-7

---

## Total Ticket Count

- **Phase 1**: 4 tickets
- **Phase 2**: 3 tickets
- **Phase 3**: 3 tickets
- **Phase 4**: 2 tickets
- **Phase 5**: 3 tickets
- **Phase 6**: 4 tickets
- **Phase 7**: 4 tickets (optional)

**Total**: 23 tickets (19 required + 4 optional)

---

## Agent Summary

**Primary Agents**:
- **docker-engineer**: 17 tickets (Phases 1, 2, 3, 5, 6, 7)
- **integration-tester**: 4 tickets (Phases 4, 7)

**Supporting Agents** (all tickets):
- **verify-ticket**: Acceptance criteria verification
- **commit-ticket**: Git commit creation

---

## Success Metrics

**Project Complete When**:
1. ✅ EMBEDDING_PROVIDER=google does NOT start Ollama
2. ✅ EMBEDDING_PROVIDER=openai does NOT start Ollama
3. ✅ EMBEDDING_PROVIDER=ollama or unset DOES start Ollama
4. ✅ Published package works via npx @crewchief/maproom-mcp@latest
5. ✅ Diagnostic logging enables user self-service debugging
6. ✅ No regressions for existing Ollama users
7. ✅ Manual test with real MCP client (Claude Desktop) passes

---

## Files Affected

**Primary Files**:
- `packages/maproom-mcp/bin/cli.cjs` - Core CLI logic (Phases 1-3, 7)
- `packages/maproom-mcp/config/docker-compose.yml` - Service definitions (Phases 5, 7)
- `packages/maproom-mcp/package.json` - Version and scripts (Phases 5, 6)
- `packages/maproom-mcp/README.md` - Documentation (Phases 5, 6)
- `packages/maproom-mcp/CHANGELOG.md` - Release notes (Phase 6)

**New Files**:
- `packages/maproom-mcp/tests/startup-integration.sh` - Integration tests (Phase 4)
- `packages/maproom-mcp/config/docker-compose.env.example` - Config examples (Phase 6)
- `packages/maproom-mcp/SECURITY.md` - Security docs (Phase 5, optional)

---

## Reference Links

**Related Tickets (Previous Attempts)**:
- MCP-008 (commit 5b7f1e4): First fix attempt - env var syntax in docker-compose.yml
- MCP-011 (commit 3bb0071): Second fix attempt - auto-update and service stop

**Project Planning**:
- See `/workspace/.crewchief/projects/MCPSTART-mcp-provider-startup-fix/README.md` for full project overview

**Test Strategy**:
- See `MCPSTART_QUALITY_STRATEGY.md` for comprehensive testing approach

**Security Review**:
- See `MCPSTART_SECURITY_REVIEW.md` for security analysis and recommendations

---

## Execution Order

**Week 1** (Phases 1-2):
1. MCPSTART-1001 → 1002 → 1003 → 1004
2. MCPSTART-2001 → 2002 → 2003

**Week 2** (Phases 3-5):
3. MCPSTART-3001 → 3002 → 3003
4. MCPSTART-4001 → 4002 (parallel with Phase 3)
5. MCPSTART-5001, 5002, 5003 (parallel)

**Week 3** (Phase 6):
6. MCPSTART-6001, 6002 (parallel)
7. MCPSTART-6003 → 6004 (sequential, 6004 requires ALL previous)

**Future** (Phase 7 - Optional):
8. MCPSTART-7001 → 7002 → 7003 → 7004

---

*Created: 2025-01-29*
*Project Slug: MCPSTART*
*Target Release: v1.1.9*

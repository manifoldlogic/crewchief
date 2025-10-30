# MCPSTART: MCP Provider Startup Fix

## Problem

The Maproom MCP server continues to start Ollama containers even when explicitly configured to use Google Vertex AI or OpenAI embeddings via `.mcp.json`. Despite two previous fix attempts (MCP-008 and MCP-011), users report that Ollama still starts when it shouldn't.

## Root Cause

Environment variables from the MCP client configuration (`.mcp.json`) are not flowing correctly through the chain:

```
MCP Client → npx → CLI Script → Docker Compose → Container Environment
```

The break could be at any step, and previous fixes didn't include sufficient diagnostic logging to identify exactly where.

## Solution

This project implements a comprehensive fix with four key components:

1. **Diagnostic Infrastructure**: Detailed logging at every step to show what's actually happening
2. **Environment Propagation**: Explicit passing of env vars through all process boundaries
3. **Clean State Management**: Ensuring containers are in the expected state before operations
4. **Integration Testing**: Automated tests that verify correct behavior

## Success Criteria

The fix is complete when:

- ✅ `EMBEDDING_PROVIDER=google` does NOT start Ollama container
- ✅ `EMBEDDING_PROVIDER=openai` does NOT start Ollama container
- ✅ `EMBEDDING_PROVIDER=ollama` or unset DOES start Ollama container
- ✅ Fix works via published npm package (`npx @crewchief/maproom-mcp@latest`)
- ✅ Diagnostic logs provide clear visibility for debugging

## Project Documents

### Analysis & Planning
- **[MCPSTART_ANALYSIS.md](./MCPSTART_ANALYSIS.md)**: Deep analysis of the problem, root cause hypotheses, and gap analysis
- **[MCPSTART_PLAN.md](./MCPSTART_PLAN.md)**: Implementation phases, timeline, and ticket breakdown

### Technical Design
- **[MCPSTART_ARCHITECTURE.md](./MCPSTART_ARCHITECTURE.md)**: Solution architecture with code examples and implementation strategy
- **[MCPSTART_QUALITY_STRATEGY.md](./MCPSTART_QUALITY_STRATEGY.md)**: Testing approach, integration test suite, and validation criteria
- **[MCPSTART_SECURITY_REVIEW.md](./MCPSTART_SECURITY_REVIEW.md)**: Security analysis and hardening recommendations

### Agent Coordination
- **[MCPSTART_AGENT_SUGGESTIONS.md](./MCPSTART_AGENT_SUGGESTIONS.md)**: Agent assignments and workflow

## Implementation Phases

### Phase 1: Diagnostic Infrastructure (v1.1.8)
Add comprehensive logging to understand what's happening:
- Environment variable logging at startup
- Docker command logging before execution
- Container state logging after operations
- Credential redaction for sensitive values

**Agent**: `docker-engineer`

### Phase 2: Environment Propagation Fix (v1.1.9)
Ensure env vars flow correctly:
- Explicit `env` parameter in all `spawn()` calls
- Docker Compose config verification
- Environment variable presence validation

**Agent**: `docker-engineer`

### Phase 3: Clean State Management (v1.1.9)
Ensure containers are in expected state:
- Pre-flight container cleanup
- Explicit stop/remove for unnecessary services
- Final state verification

**Agent**: `docker-engineer`

### Phase 4: Integration Testing
Comprehensive automated testing:
- 7 critical test cases covering all scenarios
- Container state verification
- Published package testing
- CI/CD integration (optional)

**Agent**: `integration-tester`

### Phase 5: Security Hardening (v1.1.9)
Address security considerations:
- Bind services to localhost only
- Credential redaction in logs
- npm audit check before publish

**Agent**: `docker-engineer`

### Phase 6: Documentation & Publishing (v1.1.9)
Document and ship:
- README with troubleshooting guide
- Configuration examples
- Manual testing with real MCP client
- Publish to npm

**Agent**: `docker-engineer`

### Phase 7: Service Profiles (Optional - v1.2.0)
Architectural improvement using Docker Compose profiles:
- Profile-based service definitions
- `--profile` flag usage in CLI
- Docker Compose version compatibility

**Agent**: `docker-engineer`

## Timeline

- **Week 1**: Phases 1-2 (Diagnostics + Env Propagation)
- **Week 2**: Phases 3-5 (Clean State + Testing + Security)
- **Week 3**: Phase 6 (Documentation + Publishing)
- **Future**: Phase 7 (Optional architectural improvement)

**Target**: v1.1.9 ready in 2-3 weeks

## Files Affected

- `packages/maproom-mcp/bin/cli.cjs` - CLI script with diagnostics and env propagation
- `packages/maproom-mcp/config/docker-compose.yml` - Service configurations and bindings
- `packages/maproom-mcp/package.json` - Version and prepublish scripts
- `packages/maproom-mcp/tests/startup-integration.sh` - Integration test suite
- `packages/maproom-mcp/README.md` - Documentation updates

## Key Technical Decisions

1. **Phased Approach**: Ship diagnostics first (low risk), then fixes, then improvements
2. **Explicit Over Implicit**: Don't rely on Docker Compose defaults, be explicit
3. **Fail Fast**: Clear error messages when configuration is wrong
4. **Pragmatic Testing**: Focus on integration tests that verify the real issue
5. **Security Baseline**: Cover obvious risks without over-engineering

## Relevant Agents

- **docker-engineer**: Primary implementation agent for all phases
- **integration-tester**: Creates and maintains test suite
- **mcp-tools-engineer**: MCP protocol expertise if needed
- **verify-ticket**: Verifies acceptance criteria
- **commit-ticket**: Creates proper commit messages

## Related Work

- **MCP-008** (commit 5b7f1e4): First fix attempt - updated docker-compose.yml to use env vars
- **MCP-011** (commit 3bb0071): Second fix attempt - added auto-update and service stop logic
- **MCPSTART**: Third and comprehensive fix with diagnostics, testing, and verification

## Quick Start

To begin implementation:

```bash
# Review project documents
cat MCPSTART_PLAN.md

# Start with Phase 1 tickets
# See MCPSTART_PLAN.md for ticket list

# Run integration tests (after Phase 4)
bash packages/maproom-mcp/tests/startup-integration.sh
```

## Questions?

Review the project documents:
- Not sure what's wrong? → **MCPSTART_ANALYSIS.md**
- How to fix it? → **MCPSTART_ARCHITECTURE.md**
- How to test it? → **MCPSTART_QUALITY_STRATEGY.md**
- Security concerns? → **MCPSTART_SECURITY_REVIEW.md**
- What's the plan? → **MCPSTART_PLAN.md**

---

**Project Status**: Ready for ticket creation and implementation
**Target Version**: 1.1.9
**Expected Duration**: 2-3 weeks
**Risk Level**: Medium (touching core startup logic)

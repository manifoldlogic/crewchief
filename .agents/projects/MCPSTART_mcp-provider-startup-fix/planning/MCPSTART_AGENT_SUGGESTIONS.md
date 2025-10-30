# MCPSTART: Agent Suggestions

## Required Agents

All agents needed for this project **already exist** in the CrewChief agent registry. No new agents need to be created.

## Agent Assignments by Phase

### Phase 1: Diagnostic Infrastructure

**Primary Agent**: `docker-engineer`
- Expertise in Docker Compose configuration and container orchestration
- Will add diagnostic logging to `bin/cli.cjs`
- Will implement environment variable logging
- Will add Docker command logging

**Supporting Agent**: `mcp-tools-engineer`
- Expertise in MCP server implementation
- Can verify MCP protocol compliance
- May assist with stdio transport debugging if needed

### Phase 2: Environment Propagation Fix

**Primary Agent**: `docker-engineer`
- Will implement explicit environment passing to spawn() calls
- Will add environment variable propagation
- Will verify Docker Compose receives correct env vars

**Supporting Agent**: `integration-tester`
- Will create integration test suite
- Will verify environment variable flow end-to-end

### Phase 3: Clean State Management

**Primary Agent**: `docker-engineer`
- Will implement container cleanup logic
- Will add service stop/remove commands
- Will ensure clean state before operations

### Phase 4: Service Profiles (Optional)

**Primary Agent**: `docker-engineer`
- Will modify docker-compose.yml to use profiles
- Will update CLI to use profile-based service selection
- Will handle Docker Compose version compatibility

### Testing & Verification

**Primary Agent**: `integration-tester`
- Will create comprehensive integration test script
- Will implement all 7 critical test cases
- Will verify container state after operations

**Supporting Agent**: `verify-ticket`
- Will verify acceptance criteria for each ticket
- Will ensure all requirements are met before commit

### Security Hardening

**Primary Agent**: `docker-engineer`
- Will implement credential redaction in logs
- Will update docker-compose.yml to bind to localhost
- Will add npm audit check to package.json

**Supporting Agent**: `monitoring-observability-engineer`
- May assist with diagnostic logging best practices
- Can review log formatting and structure

### Documentation

**Primary Agent**: `docker-engineer`
- Will update README with troubleshooting guide
- Will document security best practices
- Will add configuration examples

## Why No New Agents?

This project requires:
1. **Docker/Docker Compose expertise** → `docker-engineer` ✅
2. **Integration testing** → `integration-tester` ✅
3. **MCP protocol knowledge** → `mcp-tools-engineer` ✅
4. **Security review** → Covered in security document ✅
5. **Verification** → `verify-ticket` ✅
6. **Commit management** → `commit-ticket` ✅

All capabilities needed are already available in the existing agent registry.

## Agent Expertise Requirements

For successful completion, agents should understand:

**Docker Engineer Must Know**:
- Docker Compose service orchestration
- Environment variable propagation in Node.js `spawn()`
- Container lifecycle management (start, stop, remove)
- Docker Compose profiles (for Phase 4)
- Docker networking and port binding
- Docker security best practices

**Integration Tester Must Know**:
- Bash scripting for test automation
- Docker CLI commands
- Process management (timeouts, cleanup)
- Container state inspection
- Test result verification

**MCP Tools Engineer Should Know** (if needed):
- MCP protocol specification
- stdio transport implementation
- Environment variable passing in MCP clients
- Debugging MCP server connectivity

## Agent Coordination

For each ticket:
1. **Implementation**: docker-engineer implements the change
2. **Testing**: integration-tester runs test suite
3. **Verification**: verify-ticket checks acceptance criteria
4. **Commit**: commit-ticket creates proper commit message

This linear workflow ensures quality at each step without agent confusion or overlap.

## Potential Challenges for Agents

**Challenge 1: Published Package Testing**
- Testing via `npx` requires network access to npm registry
- May need manual verification by human
- **Mitigation**: Create local test that simulates npx behavior

**Challenge 2: Real MCP Client Testing**
- Requires Claude Desktop or Cursor to be installed
- Hard to automate
- **Mitigation**: Provide manual testing checklist, automate container state verification

**Challenge 3: Docker Compose Version Compatibility**
- Different Docker Compose versions may behave differently
- Profiles require v2.0+
- **Mitigation**: Version detection and fallback logic

**Challenge 4: Stale Container State**
- Previous test runs may leave containers running
- Can cause false failures
- **Mitigation**: Aggressive cleanup in test setup/teardown

## Success Factors

Agents will succeed if:
- ✅ Each agent works on one phase at a time (no parallel confusion)
- ✅ Integration tests are comprehensive and reliable
- ✅ Diagnostic logging provides clear visibility into issues
- ✅ Test failures are easy to debug (clear error messages)
- ✅ Each ticket has clear acceptance criteria

## Communication Between Agents

When integration-tester finds a bug:
1. Report specific test failure with logs
2. Identify which component failed (env vars, service selection, container state)
3. Hand back to docker-engineer for fix
4. Rerun tests after fix

When verify-ticket identifies gap:
1. Report which acceptance criterion is not met
2. Provide evidence (test output, container state)
3. Hand back to docker-engineer for completion
4. Re-verify after changes

## Agent Workflow Example

**Ticket**: "Add diagnostic environment variable logging"

1. **docker-engineer** reads ticket, implements diagnostic logging
2. **docker-engineer** marks "Task completed" checkbox
3. **integration-tester** runs test suite, verifies logs contain expected info
4. **integration-tester** marks "Tests pass" checkbox if successful
5. **verify-ticket** checks acceptance criteria, verifies log output
6. **verify-ticket** marks "Verified" checkbox if criteria met
7. **commit-ticket** creates commit with proper message
8. **Ticket complete** ✅

If any step fails, return to step 1 with feedback.

## Tools Available to Agents

All agents have access to:
- ✅ `Bash` tool for running Docker commands
- ✅ `Read` tool for reading configuration files
- ✅ `Edit` tool for modifying code
- ✅ `Write` tool for creating new files
- ✅ `Grep` and `Glob` tools for finding relevant code

No additional tools or permissions needed.

## Conclusion

**No new agents are required for this project.** The existing agent registry has all the expertise needed:

- `docker-engineer` for Docker/container work
- `integration-tester` for test automation
- `mcp-tools-engineer` for MCP protocol issues (if needed)
- `verify-ticket` for acceptance criteria verification
- `commit-ticket` for proper git commits

The project is well-scoped and agents have clear responsibilities for each phase.

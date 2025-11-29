# Agent Suggestions: MCP Extension Initialization

## Required Agents

This project requires only 4 specialized agents due to the dramatically simplified scope:

### 1. vscode-extension-specialist

**When to Use**: All VSCode extension development tasks

**Expertise**:
- Extension activation and lifecycle management
- VSCode API patterns (commands, configuration)
- MCP configuration file format
- Package.json contribution points

**Responsibilities**:
- Implement MCPConfigWriter class
- Enhance setup wizard to call config writer
- Modify extension activation for first-time prompt
- Handle workspace folder detection

**Example Tasks**:
- Create `src/config/mcp-writer.ts` (~80 lines)
- Enhance `src/ui/setupWizard.ts` (+50 lines)
- Modify `src/extension.ts` activation (+20 lines)
- Write unit tests for config generation

**Why Essential**: This is the primary implementation agent. All new code is VSCode-specific, making this agent responsible for ~90% of the work.

---

### 2. unit-test-runner

**When to Use**: Execute tests and report results

**Expertise**:
- Running test suites (vitest)
- Interpreting test output
- Identifying failing tests

**Responsibilities**:
- Run `pnpm test` in vscode-maproom package
- Report pass/fail status clearly
- Identify specific failures

**Why Essential**: Following ticket workflow, tests must run before verification.

---

### 3. verify-ticket

**When to Use**: Verify acceptance criteria are met

**Expertise**:
- Acceptance criteria validation
- Manual testing verification
- Documentation checking

**Responsibilities**:
- Verify all acceptance criteria from ticket
- Check manual testing completed
- Ensure no regressions

**Why Essential**: Systematic verification prevents incomplete work from being committed.

---

### 4. commit-ticket

**When to Use**: Create final commit after verification passes

**Expertise**:
- Conventional Commit format
- Git operations

**Responsibilities**:
- Create well-formatted commit message
- Stage changes and commit

**Why Essential**: Ensures consistent commit history and proper attribution.

## Agent Workflow

**Simplified Workflow** (2 tickets only):

```
vscode-extension-specialist → Implements both tickets
   ↓
unit-test-runner → Executes tests
   ↓ (if tests pass)
verify-ticket → Checks acceptance criteria
   ↓ (if verification passes)
commit-ticket → Creates commit
```

If tests fail or verification fails, return to vscode-extension-specialist to fix issues.

## Specific Agent Assignments

### MCPINIT-1001: MCP Configuration Writer

**Primary**: `vscode-extension-specialist`
- Implement MCPConfigWriter class (~80 lines)
- Handle file system operations
- Merge with existing configurations
- Write unit tests

**Workflow**: `vscode-extension-specialist` → `unit-test-runner` → `verify-ticket` → `commit-ticket`

---

### MCPINIT-1002: Setup Wizard Integration

**Primary**: `vscode-extension-specialist`
- Enhance existing setup wizard (+50 lines)
- Modify extension activation (+20 lines)
- Integrate with MCPConfigWriter
- Write integration tests

**Workflow**: `vscode-extension-specialist` → `unit-test-runner` → `verify-ticket` → `commit-ticket`

## Cross-Cutting Concerns

### Security

**Agent**: `vscode-extension-specialist`
- Validate all file paths before writing
- Ensure environment variable syntax for credentials
- Review configuration merge logic

**Checkpoint**: Security review during verify-ticket step

### Testing

**Agent**: `unit-test-runner`
- Execute unit and integration tests
- Report clear pass/fail status

**Checkpoint**: Tests must pass before verification

### Documentation

**Agent**: `vscode-extension-specialist`
- Add inline code comments
- Update relevant documentation

**Checkpoint**: Documentation verified during verify-ticket step

## Agent Coordination

### Handoff Points

**Simple Sequential Flow**:

```
vscode-extension-specialist completes implementation
   ↓
unit-test-runner executes tests
   ↓
verify-ticket checks acceptance criteria
   ↓
commit-ticket creates commit
   ↓
Move to next ticket (if any)
```

**No Parallel Work**: Ticket 1002 depends on 1001, so must be sequential.

## Success Criteria by Agent

### vscode-extension-specialist
- [ ] MCPConfigWriter correctly writes `.vscode/mcp.json`
- [ ] Merges with existing MCP servers (doesn't overwrite)
- [ ] Environment variable syntax correct for all providers
- [ ] Setup wizard calls config writer after provider selection
- [ ] Extension activation prompts for setup on first run
- [ ] All unit tests written and passing
- [ ] Code is clear and well-commented

### unit-test-runner
- [ ] All tests executed successfully
- [ ] Test results clearly reported
- [ ] Failing tests identified specifically

### verify-ticket
- [ ] All acceptance criteria checked
- [ ] Manual testing completed
- [ ] No regressions identified
- [ ] Documentation updated

### commit-ticket
- [ ] Commit message follows Conventional Commit format
- [ ] All changes staged correctly
- [ ] Commit created successfully

## Escalation Path

Simplified escalation (only one implementation agent):

1. **Test Failures**: Return to vscode-extension-specialist with failure details
2. **Verification Failures**: Return to vscode-extension-specialist to complete work
3. **Commit Failures**: Check for conflicts or permissions, retry

## Agent Availability

All agents available in `.crewchief/agents/` directory:

- ✅ `vscode-extension-specialist` - Available
- ✅ `unit-test-runner` - Available
- ✅ `verify-ticket` - Available
- ✅ `commit-ticket` - Available

## Conclusion

This project requires only **4 agents** due to dramatically simplified scope:

1. **vscode-extension-specialist** - Implements all code (~150 lines total)
2. **unit-test-runner** - Test execution and reporting
3. **verify-ticket** - Quality assurance
4. **commit-ticket** - Git operations

**Work Reduction**: Originally planned 8 agents with complex coordination. Now just 1 implementation agent handles everything.

**Why So Simple**:
- No subprocess management (was: process-management-specialist)
- No status monitoring (was: database-engineer)
- No Docker orchestration (was: integration-tester for mocking)
- No credential UI (existing wizard handles it)

The vscode-extension-specialist does 90% of the work, with the other 3 agents providing standard ticket workflow support (test → verify → commit).

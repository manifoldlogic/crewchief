# Agent Suggestions: Watch Change Detection Fix

## Existing Agents (Sufficient)

This project can be completed entirely with existing agents. No new agents need to be created.

### Primary Implementation Agent

**rust-indexer-engineer**

**Why this agent**: This project requires changes to the Rust indexer codebase (`crates/maproom/`), specifically the incremental indexing pipeline. The rust-indexer-engineer agent specializes in:
- Modifying crates/maproom/ code
- Working with tree-sitter and indexing logic
- Understanding async Rust architecture
- Database interactions with PostgreSQL

**Responsibilities**:
1. Implement `normalize_to_relpath()` function in new `path_utils.rs` module
2. Refactor `processor_task` in `indexer/mod.rs` to fix change detection logic
3. Update `IncrementalProcessor` path handling in `incremental/processor.rs`
4. Add file size limits for DoS protection
5. Write unit tests for path normalization
6. Write integration tests for change detection

**Ticket assignments**: WATCHFIX-1001 through WATCHFIX-1005 (all implementation tickets)

### Testing Agent

**unit-test-runner**

**Why this agent**: After rust-indexer-engineer implements the fix, we need to verify all tests pass without making any code changes.

**Responsibilities**:
1. Execute `cargo test` in crates/maproom
2. Report test results clearly
3. Identify any failing tests
4. No code modifications (observation only)

**Ticket assignments**: All tickets after implementation (verify tests pass)

### Verification Agent

**verify-ticket**

**Why this agent**: After implementation and testing, verify each ticket meets its acceptance criteria.

**Responsibilities**:
1. Check that all code changes match ticket requirements
2. Verify tests are written and passing
3. Ensure documentation is updated
4. Confirm no regressions in existing functionality

**Ticket assignments**: All tickets (final verification before commit)

### Commit Agent

**commit-ticket**

**Why this agent**: Create properly formatted Conventional Commits after verification.

**Responsibilities**:
1. Generate commit message following project conventions
2. Include ticket reference
3. Scope commits correctly (e.g., `fix(indexer):`, `test(watch):`)
4. Add co-author attribution

**Ticket assignments**: All tickets (final step)

## Why No New Agents Needed

### Considered: watch-command-specialist

**Proposed role**: Specialist in watch command async architecture and event processing.

**Why not needed**: The rust-indexer-engineer already has sufficient expertise:
- Understands maproom architecture
- Can work with async Rust code
- Knows the incremental indexing system
- Has access to all necessary context

**Verdict**: Unnecessary specialization. Existing agent is sufficient.

### Considered: path-normalization-expert

**Proposed role**: Specialist in cross-platform path handling and filesystem operations.

**Why not needed**: Path normalization is straightforward:
- Use `Path::strip_prefix()` (standard library)
- 20-30 lines of code total
- Well-documented Rust Path APIs
- Not complex enough to warrant specialized agent

**Verdict**: Unnecessary specialization. Basic Rust knowledge is sufficient.

### Considered: integration-test-specialist

**Proposed role**: Specialist in writing database integration tests.

**Why not needed**: The rust-indexer-engineer can write integration tests:
- Already familiar with test database setup
- Knows the schema and data model
- Can use existing test utilities
- Integration tests follow same patterns as unit tests

**Verdict**: Unnecessary specialization. Standard testing skills are sufficient.

## Agent Workflow

```
For each ticket:

1. rust-indexer-engineer implements the fix
   ↓
2. rust-indexer-engineer writes tests
   ↓
3. unit-test-runner executes tests
   ↓
4. If tests fail:
   ← return to rust-indexer-engineer to fix
   ↓
5. verify-ticket checks acceptance criteria
   ↓
6. If verification fails:
   ← return to rust-indexer-engineer to address
   ↓
7. commit-ticket creates commit
   ↓
8. Move to next ticket
```

## Coordination Notes

**Single agent approach**: Since only one agent (rust-indexer-engineer) does implementation, there's minimal coordination overhead.

**Clear handoffs**: Each step (implement → test → verify → commit) has clear completion criteria.

**No parallel work needed**: Tickets should be completed sequentially to avoid conflicts in the same codebase files.

**No inter-agent communication**: Agents operate independently via the ticket workflow. No custom message passing needed.

## Agent Capabilities Required

### rust-indexer-engineer must be able to:
- [x] Read and understand Rust async code
- [x] Modify existing functions (processor_task)
- [x] Create new modules (path_utils.rs)
- [x] Write unit tests with #[test]
- [x] Write async tests with #[tokio::test]
- [x] Use cargo test
- [x] Understand PostgreSQL queries
- [x] Work with Path and PathBuf types
- [x] Handle Result and Option types
- [x] Use tracing macros (info!, warn!, debug!)

### unit-test-runner must be able to:
- [x] Execute cargo test
- [x] Parse test output
- [x] Report pass/fail status
- [x] Identify which tests failed
- [x] NO code modification capability

### verify-ticket must be able to:
- [x] Read ticket acceptance criteria
- [x] Check file changes against criteria
- [x] Verify tests exist and pass
- [x] Compare implementation to requirements
- [x] Report verification status

### commit-ticket must be able to:
- [x] Generate Conventional Commit messages
- [x] Determine correct scope (fix, test, refactor)
- [x] Include ticket reference
- [x] Execute git commands
- [x] Add co-author attribution

## Conclusion

**No new agents needed.** The existing rust-indexer-engineer agent, combined with the standard ticket workflow agents (unit-test-runner, verify-ticket, commit-ticket), provides all capabilities required to complete this project successfully.

The simplicity of using existing agents reduces overhead, maintains consistency with other projects, and allows agents to leverage their existing knowledge of the codebase.

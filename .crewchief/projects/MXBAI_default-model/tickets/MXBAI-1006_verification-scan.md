# Ticket: [MXBAI-1006]: Phase 1 Verification Scan

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Run comprehensive verification scan after all Phase 1 code and test changes to ensure no unexpected references to old defaults remain, and execute all test suites to confirm everything passes.

## Background
This ticket implements Phase 1, Deliverables 5 and 6 from plan.md. After completing all code and test updates, we must verify that no locations were missed and all tests pass. This is a critical quality gate before moving to Phase 2 documentation updates.

Reference: plan.md Phase 1, Deliverables 5 "Run Test Suites" and 6 "Verification Scan"

## Acceptance Criteria
- [ ] All Rust tests pass: `cargo test -p crewchief-maproom` exits with code 0
- [ ] All VSCode extension tests pass: `pnpm test` in vscode-maproom exits 0
- [ ] All MCP server tests pass: `pnpm test` in maproom-mcp exits 0
- [ ] Verification grep scan shows only expected "nomic-embed-text" references
- [ ] Verification grep scan shows only expected "768" references
- [ ] Manual CLI test confirms zero-config uses mxbai-embed-large
- [ ] Manual CLI test confirms explicit nomic-embed-text still works

## Technical Requirements
**Test Suite Execution**:
```bash
# Rust tests
cd /workspace/crates/maproom
cargo test -p crewchief-maproom

# TypeScript tests - VSCode extension
cd /workspace/packages/vscode-maproom
pnpm test

# TypeScript tests - MCP server
cd /workspace/packages/maproom-mcp
pnpm test
```

**Verification Scan** (grep for remaining references):
```bash
# Scan for nomic-embed-text in code (should only be in sanitization logic and backward compat tests)
grep -rn "nomic-embed-text" crates/maproom/src/ packages/vscode-maproom/src/ packages/maproom-mcp/src/

# Scan for 768 dimension in code (should only be in sanitization logic and backward compat tests)
grep -rn "768" crates/maproom/src/ packages/vscode-maproom/src/ packages/maproom-mcp/src/

# Scan .env.example
grep -n "nomic-embed-text\|768" crates/maproom/.env.example
```

**Expected References** (these should remain):
- `ollama.rs` embed_batch_raw(): Sanitization conditional checks `if self.model == "nomic-embed-text"`
- `factory.rs` or test files: Backward compatibility test with explicit env vars
- `.env.example`: Backward compatibility comment mentioning old model as option

**Unexpected References** (fail if found):
- DEFAULT_MODEL constant still set to "nomic-embed-text"
- default_config() still using 768
- Factory fallback still using "nomic-embed-text"
- Any new hardcoded references not covered above

**Manual CLI Tests**:
```bash
# Test 1: Zero-config uses mxbai
unset MAPROOM_EMBEDDING_MODEL
unset MAPROOM_EMBEDDING_DIMENSION
cd /workspace
cargo run --bin crewchief-maproom -- status --repo test
# Expected log: "Using provider: ollama (model: mxbai-embed-large, dimension: 1024)"

# Test 2: Explicit nomic still works
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
cargo run --bin crewchief-maproom -- status --repo test
# Expected log: "Using provider: ollama (model: nomic-embed-text, dimension: 768)"
```

## Implementation Notes
**Purpose**:
- Catch any missed locations from tickets MXBAI-1001 through MXBAI-1005
- Verify all test suites pass as validation gate
- Confirm backward compatibility preserved
- Provide confidence before documentation phase

**Failure Handling**:
- If tests fail: Fix assertions in relevant ticket (MXBAI-1004 or MXBAI-1005)
- If unexpected references found: Update code in relevant ticket (MXBAI-1001 or MXBAI-1002)
- If manual tests fail: Debug factory.rs or ollama.rs defaults

**Success Criteria from plan.md**:
- All tests pass (exit code 0)
- Backward compatibility verified (explicit nomic-embed-text works)
- Verification scan shows no unexpected references

## Dependencies
- **Critical dependencies**:
  - MXBAI-1001 (Rust constants)
  - MXBAI-1002 (TypeScript constants)
  - MXBAI-1003 (Configuration examples)
  - MXBAI-1004 (Rust tests)
  - MXBAI-1005 (TypeScript tests)
- **External dependency**: None

## Risk Assessment
- **Risk**: Tests fail unexpectedly
  - **Mitigation**: Review test output, identify which ticket needs fixes

- **Risk**: Unexpected references found
  - **Mitigation**: Grep output will show exact locations, update relevant ticket

- **Risk**: Manual CLI tests fail
  - **Mitigation**: Check logs for actual vs expected output, debug factory.rs

## Files/Packages Affected
- All code files from MXBAI-1001 through MXBAI-1005 (verification only, no changes)
- Test suites across Rust and TypeScript

## Verification Notes
Tests pass: **CRITICAL** - All three test suites must pass (Rust, VSCode, MCP)

verify-ticket agent should check:
- [ ] Test execution output shows all tests passing
- [ ] Grep scan results documented and only expected references found
- [ ] Manual CLI test output matches expected log messages
- [ ] No new failures introduced by Phase 1 changes
- [ ] Ticket includes full test output and grep results for audit trail

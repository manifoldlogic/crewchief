# Ticket: [MXBAI-1005]: Update TypeScript Test Assertions

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
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update TypeScript test assertions in VSCode extension and MCP server to reflect new default model (mxbai-embed-large), and update test mocks to validate correct model.

## Background
This ticket addresses Phase 1, Deliverable 4 from plan.md (TypeScript portion). After changes in MXBAI-1002, TypeScript tests will fail because they assert the old default model. The grep audit identified 8+ assertions in model-manager.test.ts and 10+ test cases in provider-detection.test.ts.

Reference: plan.md Phase 1, Deliverable 4 "Update Test Assertions (TypeScript tests)"

## Acceptance Criteria
- [ ] model-manager.test.ts DEFAULT_EMBEDDING_MODEL assertion updated to 'mxbai-embed-large'
- [ ] model-manager.test.ts mock expectations updated (8+ assertions)
- [ ] provider-detection.test.ts test mocks updated with mxbai-embed-large (10+ test cases)
- [ ] Warning message expectations updated to reference "ollama pull mxbai-embed-large"
- [ ] All VSCode extension tests pass: `pnpm test` in vscode-maproom exits 0
- [ ] All MCP server tests pass: `pnpm test` in maproom-mcp exits 0

## Technical Requirements
**VSCode Extension Tests** (`packages/vscode-maproom/src/ollama/model-manager.test.ts`):
- Line 359 (approximate): Update assertion
  - Change: `expect(DEFAULT_EMBEDDING_MODEL).toBe('nomic-embed-text')`
  - To: `expect(DEFAULT_EMBEDDING_MODEL).toBe('mxbai-embed-large')`

- Search for all mock expectations referencing 'nomic-embed-text':
  ```bash
  grep -rn "nomic-embed-text" packages/vscode-maproom/src/ollama/model-manager.test.ts
  ```
  Update each to 'mxbai-embed-large'

**MCP Server Tests** (`packages/maproom-mcp/tests/provider-detection.test.ts`):
- Update test mocks to include 'mxbai-embed-large' in model lists
- Update test cases checking for model validation
- Update warning message expectations:
  - Change: expectations containing "ollama pull nomic-embed-text"
  - To: "ollama pull mxbai-embed-large"

Search for test mocks:
```bash
grep -rn "nomic-embed-text" packages/maproom-mcp/tests/
```

## Implementation Notes
**Strategy**:
1. Run tests to see which ones fail: `pnpm test`
2. Update each failing assertion to match new defaults
3. Update mock data to include mxbai-embed-large model
4. Re-run tests to verify all pass

**Test patterns to update**:
- Constant value assertions
- Mock function expectations (expect model manager to check for specific model)
- Provider detection mocks (models list should include mxbai-embed-large)
- Warning/error message expectations

**What NOT to change**:
- Tests for backward compatibility (if any test explicit nomic-embed-text config)
- Test structure or logic
- Mock patterns or setup (only the model name values)

**Pattern from quality-strategy.md**:
```typescript
// VSCode test update example:
test('DEFAULT_EMBEDDING_MODEL constant', () => {
  expect(DEFAULT_EMBEDDING_MODEL).toBe('mxbai-embed-large')  // ← UPDATE
})

// MCP test update example:
test('detects Ollama with embedding model', async () => {
  // Mock should include mxbai-embed-large in models array
  const mockModels = [
    { name: 'mxbai-embed-large' },  // ← UPDATE
    // other models...
  ]
})
```

## Dependencies
- **Critical dependency**: MXBAI-1002 (TypeScript constants must be updated first)
- **External dependency**: None

## Risk Assessment
- **Risk**: Breaking test mocks
  - **Mitigation**: Run tests after each change, verify mock structure unchanged

- **Risk**: Missing test locations
  - **Mitigation**: Use grep to find all occurrences in test files

- **Risk**: Tests fail for unexpected reasons
  - **Mitigation**: Read test output carefully, update only model-related assertions

## Files/Packages Affected
- `/workspace/packages/vscode-maproom/src/ollama/model-manager.test.ts`
- `/workspace/packages/maproom-mcp/tests/provider-detection.test.ts`
- Other test files identified by grep search

## Verification Notes
verify-ticket agent should check:
- [ ] VSCode extension tests pass: `pnpm test` in packages/vscode-maproom exits 0
- [ ] MCP server tests pass: `pnpm test` in packages/maproom-mcp exits 0
- [ ] Test output shows no failures or warnings
- [ ] Mock structures unchanged (only model name values updated)
- [ ] grep for "nomic-embed-text" in test files shows only expected references (if any backward compat tests exist)

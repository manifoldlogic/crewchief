# Ticket: [MXBAI-1002]: Update TypeScript Default Model Constants

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
- verify-ticket
- commit-ticket

## Summary
Update TypeScript constants to change the default embedding model from nomic-embed-text to mxbai-embed-large in VSCode extension model manager and MCP server provider detection.

## Background
This ticket implements Decisions 5 and 6 from architecture.md. The VSCode extension and MCP server both have hardcoded model references that must match the Rust daemon's defaults to provide a consistent zero-config experience across all integration layers.

Reference: plan.md Phase 1, Deliverable 2 "Update TypeScript Constants"

## Acceptance Criteria
- [ ] DEFAULT_EMBEDDING_MODEL in model-manager.ts changed from 'nomic-embed-text' to 'mxbai-embed-large'
- [ ] Provider detection model check in provider-detection.ts changed to validate 'mxbai-embed-large'
- [ ] Warning message in provider-detection.ts updated to suggest `ollama pull mxbai-embed-large`
- [ ] All changes compile without TypeScript errors
- [ ] No other code changes needed

## Technical Requirements
- Modify `packages/vscode-maproom/src/ollama/model-manager.ts` line 16:
  - Change: `export const DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'`
  - To: `export const DEFAULT_EMBEDDING_MODEL = 'mxbai-embed-large'`

- Modify `packages/maproom-mcp/src/utils/provider-detection.ts` line 126:
  - Change: `const hasEmbedModel = models.some((m: any) => m.name.includes('nomic-embed-text'))`
  - To: `const hasEmbedModel = models.some((m: any) => m.name.includes('mxbai-embed-large'))`

- Update warning message in provider-detection.ts (find line with "ollama pull nomic-embed-text"):
  - Change suggestion from `ollama pull nomic-embed-text` to `ollama pull mxbai-embed-large`

## Implementation Notes
**Why these changes matter**:
- Extension's ensureOllamaModel() must download correct model during activation
- MCP server's provider detection must validate correct model exists
- Ensures zero-config experience works consistently (extension, MCP, daemon all use same default)

**Pattern to follow**:
- Simple string constant changes
- No logic changes
- Maintain consistency with Rust defaults from MXBAI-1001

**Verification**:
After changes, compile TypeScript to ensure no errors:
```bash
cd /workspace/packages/vscode-maproom
pnpm build

cd /workspace/packages/maproom-mcp
pnpm build
```

## Dependencies
- **Logical dependency**: MXBAI-1001 (Rust constants should be updated first for consistency)
- **External dependency**: None

## Risk Assessment
- **Risk**: TypeScript compilation errors
  - **Mitigation**: Run pnpm build immediately after changes

- **Risk**: Missing warning message update
  - **Mitigation**: Grep for "nomic-embed-text" in provider-detection.ts to find all references

## Files/Packages Affected
- `/workspace/packages/vscode-maproom/src/ollama/model-manager.ts` (line 16)
- `/workspace/packages/maproom-mcp/src/utils/provider-detection.ts` (line 126 and warning message)

## Verification Notes
verify-ticket agent should check:
- [ ] Both file locations modified correctly
- [ ] Warning message updated to reference mxbai-embed-large
- [ ] TypeScript compiles without errors (pnpm build passes in both packages)
- [ ] No other unintended changes to these files
- [ ] Constants are strings (not accidentally changed to numbers or other types)

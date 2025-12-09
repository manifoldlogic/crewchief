# Ticket: [SRCHFIX-1004]: Search for Old Field Name Usage

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
- typescript-expert
- test-runner
- verify-ticket
- commit-ticket

## Summary
Search the codebase for usage of `chunk_index` and `chunkIndex` to ensure renaming to `chunk_id` doesn't break any existing code.

## Background
We're renaming the TypeScript field from `chunk_index` to `chunk_id` to match Rust conventions. Before making this change, we need to verify there are no consumers depending on the old name. Since the field was always 0 (broken), we expect no actual usage, but we must verify.

This ticket implements Task 1.4 from the execution plan: Search for chunk_index Usage.

## Acceptance Criteria
- [ ] Searched for `chunk_index` (exact match) in all TypeScript files
- [ ] Searched for `chunkIndex` (camelCase variant) in all TypeScript files
- [ ] Documented all findings in ticket completion notes
- [ ] Replaced any usage with `chunk_id` (or confirmed only interface definition found)
- [ ] TypeScript compilation succeeds after any replacements

## Technical Requirements
**Search patterns**:
- `chunk_index` (exact match, case-sensitive)
- `chunkIndex` (camelCase variant)

**Directories to search**:
- `/workspace/packages/maproom-mcp/src/**/*.ts`
- `/workspace/packages/vscode-maproom/src/**/*.ts`
- `/workspace/packages/daemon-client/src/**/*.ts`
- `/workspace/crates/*/` (check Rust files too, though unlikely)

**Expected result**: Only find the interface definitions being updated in SRCHFIX-1002.

**Action if found**: Replace with `chunk_id` and document in completion notes.

## Implementation Notes
**Search approach**:
1. Use grep/ripgrep to search for patterns
2. Review each match to determine if it's:
   - Interface definition (expected, being updated in SRCHFIX-1002)
   - Active usage (unexpected, needs replacement)
   - Comment or documentation (update to use chunk_id)
3. Document findings even if no unexpected usage found

**Commands**:
```bash
# Search for chunk_index
rg "chunk_index" packages/maproom-mcp/src packages/vscode-maproom/src packages/daemon-client/src

# Search for chunkIndex (camelCase)
rg "chunkIndex" packages/maproom-mcp/src packages/vscode-maproom/src packages/daemon-client/src
```

**Risk mitigation**: This search prevents breaking changes. If usage is found, we either:
1. Update the usage to chunk_id (preferred)
2. Document why it needs to stay chunk_index (unlikely)

## Dependencies
- Should be completed before or in parallel with SRCHFIX-1002
- Independent of other Phase 1 tickets

## Risk Assessment
- **Risk**: Miss a usage that breaks at runtime
  - **Mitigation**: Comprehensive search across all packages, including tests
- **Risk**: False positives in comments or strings
  - **Mitigation**: Review each match manually to confirm it's actual usage
- **Risk**: Dynamic property access (e.g., hit['chunk_index'])
  - **Mitigation**: Search includes string patterns, runtime tests catch any issues

## Files/Packages Affected
- `/workspace/packages/daemon-client/src/**/*.ts`
- `/workspace/packages/maproom-mcp/src/**/*.ts`
- `/workspace/packages/vscode-maproom/src/**/*.ts`

## Verification Notes
Document in completion notes:
1. Exact search commands used
2. Number of matches found for each pattern
3. Context of each match (interface vs usage)
4. Any replacements made
5. Confirmation that TypeScript compilation succeeds

Example completion note format:
```
Search Results:
- chunk_index: 2 matches
  - packages/daemon-client/src/client.ts:35 - interface definition (updated in SRCHFIX-1002)
  - packages/maproom-mcp/src/daemon-client/client.ts:40 - vendored interface (updated in SRCHFIX-1002)
- chunkIndex: 0 matches

Conclusion: No unexpected usage found. Safe to rename.
```

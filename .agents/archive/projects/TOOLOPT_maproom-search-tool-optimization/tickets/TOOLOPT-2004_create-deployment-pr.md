# Ticket: TOOLOPT-2004: Create pull request with validation evidence

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
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Commit changes and create pull request with comprehensive test results and documentation links.

## Background
Production deployment requires PR review. Package all changes with validation evidence to demonstrate improvement is proven and safe.

This ticket is part of Phase 2 (Production Deployment) of the TOOLOPT project, creating the deployment PR with full evidence.

## Acceptance Criteria
- [ ] Changes committed with proper conventional commit message
- [ ] PR created with complete description including:
  - [ ] Summary of change (tool description update)
  - [ ] Performance validation results (+1.9% improvement)
  - [ ] Integration test confirmation
  - [ ] Link to optimization documentation
  - [ ] Risk assessment (low - content-only change)
- [ ] Test evidence included in PR description
- [ ] Code review requested
- [ ] CI/CD checks passing

## Technical Requirements
- Commit message format:
  ```
  feat(maproom-mcp): update search tool description with optimized variant

  Deploy variant-a-detailed (19.6% performance) replacing control variant
  (17.7%). Validation confirms +1.9% improvement in agent search quality.

  - Update tool description in search.ts
  - Based on 10-generation genetic optimization
  - See docs/optimization/ for detailed findings

  Performance validation:
  - Control: 17.7%
  - Winner: 19.6%
  - Delta: +1.9%

  🤖 Generated with [Claude Code](https://claude.com/claude-code)

  Co-Authored-By: Claude <noreply@anthropic.com>
  ```
- PR description includes:
  - Test results from TOOLOPT-2001
  - Integration test confirmation from TOOLOPT-2003
  - Documentation link to docs/optimization/
  - Security review summary (low risk - content-only change)
  - Before/after comparison

## Implementation Notes
- Use `gh pr create` for PR creation
- Include validation numbers in PR body
- Link to docs/optimization/README.md
- Request review from maintainers
- Ensure all checks green before merge
- Attach test output files as evidence
- Highlight that this is a content-only change (no API modifications)

## Dependencies
- TOOLOPT-2001 (validation test results)
- TOOLOPT-2002 (code changes)
- TOOLOPT-2003 (integration test confirmation)

## Risk Assessment
- **Risk**: CI failures blocking merge
  - **Mitigation**: Ensure local build/test passes first, verify all checks
- **Risk**: Review delays
  - **Mitigation**: Provide comprehensive evidence, clear description

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/src/tools/search.ts` (committed)
- PR metadata (new)
- Git commit history

## Estimated Time
15 minutes

# Ticket: NPMDEP-4001: End-to-End Validation of Deprecation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Perform comprehensive end-to-end validation from user perspective: test installation warnings, executable behavior (normal and --help), verify npm website display, test all links, and document complete project results.

## Background
Phase 4 - End-to-End Validation. This is the final validation after publishing v2.0.0 (NPMDEP-2001) and applying deprecation (NPMDEP-3001). We must verify the complete user experience works correctly from all touchpoints:
- Installation (deprecation warning)
- Execution (migration message)
- Web (npm package page)

This ticket validates all 9 project success criteria from plan.md are met:
1. Version 2.0.0 published to npm registry
2. README visible on package page
3. npm install shows deprecation warning
4. Warning mentions @crewchief/maproom-mcp
5. Warning includes --help reference
6. npx maproom-mcp shows migration message
7. npx maproom-mcp --help shows help-specific message
8. All links work
9. Documentation complete

## Acceptance Criteria
- [ ] Fresh `npm install maproom-mcp` shows deprecation warning in output
- [ ] Warning text includes "@crewchief/maproom-mcp" and "--help" reference
- [ ] `npx maproom-mcp@2.0.0` executes and shows migration message
- [ ] `npx maproom-mcp@2.0.0 --help` shows help-specific message
- [ ] Both executions exit with code 1
- [ ] npm website https://www.npmjs.com/package/maproom-mcp shows v2.0.0
- [ ] "DEPRECATED" badge visible on npm website
- [ ] README renders correctly on npm website
- [ ] All links in README functional (test each link)
- [ ] Complete validation report documenting all findings

## Technical Requirements

**Installation Testing:**
```bash
# Test in clean directory
cd /tmp/final-validation-test
npm install maproom-mcp 2>&1 | tee install-output.txt
grep -i deprecat install-output.txt
```

**Execution Testing:**
```bash
# Test normal execution
npx maproom-mcp@2.0.0 2>&1 | tee normal-output.txt
echo "Exit code: $?"  # Should be 1

# Test --help flag
npx maproom-mcp@2.0.0 --help 2>&1 | tee help-output.txt
echo "Exit code: $?"  # Should be 1

# Verify messages contain required elements
grep "@crewchief/maproom-mcp" normal-output.txt
grep "@crewchief/maproom-mcp --help" help-output.txt
```

**Web Validation (Manual):**
- Visit https://www.npmjs.com/package/maproom-mcp
- Verify version 2.0.0 shows as latest
- Verify "DEPRECATED" badge visible
- Verify README displays full deprecation notice
- Test each link in README:
  - Link to @crewchief/maproom-mcp package
  - Link to GitHub repository
  - Link to issue tracker
  - Any other links in deprecation notice

**Documentation Requirements:**
- Create final-validation-report.md with:
  - Test execution results
  - Screenshots of npm website (optional but helpful)
  - Status of each success criterion (pass/fail)
  - Any issues or anomalies discovered
  - Recommendations for any follow-up
- Create project-completion-summary.md with:
  - Summary of what was accomplished
  - Verification that all tickets completed
  - Final state of npm package
  - Timestamp of completion

## Implementation Notes
- This is the **final quality gate** before project completion
- All 9 success criteria from plan.md must be verified
- Document everything comprehensively for audit trail
- If any criteria fail, determine if:
  - Acceptable as-is (document why)
  - Requires fix (create follow-up ticket)
- Pay special attention to --help flag (user-specified requirement)
- Test from fresh environment (simulates actual user experience)
- Capture all terminal output for documentation

From quality-strategy.md:
- Phase 4 validation is a MUST PASS quality gate
- No broken links tolerated
- Clear migration path must be visible
- Documentation completeness is critical

## Dependencies
- **Blocks on:** NPMDEP-3001 (deprecation must be applied first)
- **Blocks:** Project completion
- **Required:** npm package published and deprecated

## Risk Assessment
- **Risk:** Broken links in README
  - **Mitigation:** Test every single link manually
  - **Impact:** Medium - confuses users about migration path
- **Risk:** Incorrect message display
  - **Mitigation:** Compare actual output against specifications
  - **Impact:** Low - cosmetic if messages generally correct
- **Risk:** Exit code not 1
  - **Mitigation:** Explicit test with echo $?
  - **Impact:** Low - doesn't prevent migration
- **Risk:** Missing --help reference
  - **Mitigation:** grep for exact text in outputs
  - **Impact:** High - user specifically requested this

## Files/Packages Affected
- `/tmp/final-validation-test/` (test directory, temporary)
- `.crewchief/projects/NPMDEP_npm-deprecation/final-validation-report.md` (new, comprehensive results)
- `.crewchief/projects/NPMDEP_npm-deprecation/project-completion-summary.md` (new, final summary)
- `.crewchief/projects/NPMDEP_npm-deprecation/install-output.txt` (new, captured output)
- `.crewchief/projects/NPMDEP_npm-deprecation/normal-output.txt` (new, captured output)
- `.crewchief/projects/NPMDEP_npm-deprecation/help-output.txt` (new, captured output)

**Estimated Time:** 15 minutes

# Ticket: VSMAP-4002: Execute manual testing checklist across platforms

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (manual testing coordination)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Run comprehensive manual tests on Linux, macOS, Windows. Test devcontainer modes (DinD, DooD). Document results and file bugs if found.

## Background
This continues Phase 4 (Polish & Testing) of the VSMAP plan. While integration tests (VSMAP-4001) cover core workflows, manual testing catches platform-specific issues, UI/UX problems, and edge cases. We need to verify the extension works across all supported platforms and Docker configurations before release.

Reference: VSMAP_PLAN.md Phase 4 "Polish & Testing - Manual Testing Protocol"

## Acceptance Criteria
- [ ] Manual tests executed on Linux x64 (Ubuntu 22.04+ or equivalent)
- [ ] Manual tests executed on macOS arm64 (M1/M2/M3 recommended)
- [ ] Manual tests executed on macOS x64 (optional, if available)
- [ ] Manual tests executed on Windows x64 (marked as experimental)
- [ ] DevContainer tests executed in DinD (Docker-in-Docker) mode
- [ ] DevContainer tests executed in DooD (Docker-outside-of-Docker) mode
- [ ] All critical flows pass: activation, indexing, error handling
- [ ] Test results documented in test report with platform details
- [ ] Bugs filed as new tickets if critical issues found

## Technical Requirements
- Use test checklist from quality-strategy.md
- Test on minimum 3 platforms (Linux, macOS, Windows)
- Test Docker failure scenarios (services not starting)
- Test binary error scenarios (corrupted binary, missing permissions)
- Test crash recovery (kill processes during indexing)
- Test initial scan, incremental updates, git branch switching
- Document results in structured format with:
  - Platform details (OS, version, architecture)
  - Test scenario
  - Result (pass/fail)
  - Screenshots if issues found
  - Reproduction steps for failures

## Implementation Notes
Manual testing checklist (based on quality-strategy.md):

**1. Extension Activation**
- [ ] Extension loads without errors
- [ ] Docker services start within 30s
- [ ] Status bar appears with "Starting..." → "Watching"
- [ ] Output channel shows Docker startup logs

**2. Setup Wizard**
- [ ] Wizard appears on first activation
- [ ] Ollama detection works (if Ollama running)
- [ ] Provider selection persists
- [ ] Credential input works (password masked)
- [ ] Initial scan triggers automatically

**3. Indexing**
- [ ] Initial scan shows progress notification
- [ ] Progress updates every ~5%
- [ ] File counts increment correctly
- [ ] Scan completes without errors
- [ ] Status bar updates to "Indexed: N files"

**4. Error Handling**
- [ ] Docker not running: clear error message
- [ ] Binary missing: actionable error
- [ ] Invalid credentials: helpful message
- [ ] Process crash: automatic restart

**5. Crash Recovery**
- [ ] Kill watch process → auto-restart
- [ ] Multiple crashes → exponential backoff
- [ ] 5 crashes → circuit breaker notification
- [ ] Manual restart command works

**6. Platform-Specific**
- [ ] Binary executes (check permissions on Linux/macOS)
- [ ] Docker socket accessible
- [ ] File watching works (create/edit files)
- [ ] Git operations trigger re-scan

**7. DevContainer**
- [ ] Extension activates in container
- [ ] DinD mode: nested Docker works
- [ ] DooD mode: host Docker accessible
- [ ] Database persists across rebuilds

**Test Report Format:**
```markdown
# VSMAP Manual Test Report

## Test Environment
- **Date**: 2025-11-16
- **Extension Version**: 0.1.0
- **Tester**: [Name]

## Platform: Linux x64 (Ubuntu 22.04)
| Scenario | Result | Notes |
|----------|--------|-------|
| Extension Activation | PASS | Loaded in 3.2s |
| Setup Wizard | PASS | Ollama detected correctly |
| Initial Scan | PASS | 1,234 files in 45s |
| ... | ... | ... |

## Platform: macOS arm64 (M2)
...

## Bugs Found
1. **VSMAP-XXXX**: Status bar flickers on Windows
   - Platform: Windows 11 x64
   - Reproduction: ...
   - Severity: Low
```

Filing bugs:
- Create new tickets for any critical or high-severity issues
- Use format: `VSMAP-40XX_bug-description.md`
- Include platform, reproduction steps, screenshots
- Mark severity: Critical (blocks release), High, Medium, Low

## Dependencies
- VSMAP-4001 (integration tests) should be complete for baseline
- Access to test machines/VMs for each platform
- Test workspace with representative code (~1000+ files)

## Risk Assessment
- **Risk**: May discover critical bugs late in process
  - **Mitigation**: Allocate time for bug fixes before release
- **Risk**: Windows testing may reveal platform-specific issues
  - **Mitigation**: Document as experimental, provide workarounds
- **Risk**: Devcontainer testing requires Docker expertise
  - **Mitigation**: Provide clear test instructions, test both modes

## Files/Packages Affected
- `docs/test-report.md` (new file, test results documentation)
- `.agents/projects/VSMAP_vscode-maproom-extension/tickets/` (potential bug tickets)
- `README.md` (update with known issues from testing)

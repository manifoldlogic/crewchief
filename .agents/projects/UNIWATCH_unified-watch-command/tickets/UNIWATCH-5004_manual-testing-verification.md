# Ticket: UNIWATCH-5004: Manual Testing and Final Verification

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (manual testing, no automated tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- verify-ticket
- commit-ticket

## Summary
Execute manual testing checklist to validate real-world developer workflow and verify all acceptance criteria are met.

## Background
Automated tests catch most issues, but manual testing validates the actual user experience, NDJSON event formatting, error messages, and edge cases that are difficult to automate.

This is the final ticket in Phase 5 (Testing & Verification) which validates all implementation work from Phases 1-4 before final release. Manual testing ensures the unified watch command meets real-world developer needs.

## Acceptance Criteria
- [ ] Complete manual testing checklist (documented below)
- [ ] All checklist items pass
- [ ] No regressions found
- [ ] NDJSON events correctly formatted
- [ ] Error messages clear and helpful
- [ ] Documentation accurate
- [ ] Final sign-off documented in ticket comments

## Manual Testing Checklist

### Functional Testing
- [ ] Start `maproom watch` on main branch (no flags)
- [ ] Verify "Watching..." message appears
- [ ] Edit a file → verify file_processed NDJSON event
- [ ] Switch to feature branch → verify branch_switched NDJSON event
- [ ] Edit a file → verify file_processed event with new worktree
- [ ] Switch back to main → verify branch_switched event
- [ ] Edit a file → verify indexed to main worktree
- [ ] Rapid branch switches (3+ in 1 second) → verify debouncing

### Backward Compatibility
- [ ] Run `maproom watch --worktree main` → verify deprecation warning
- [ ] Verify command still works
- [ ] Run `maproom branch-watch` → verify deprecation warning
- [ ] Verify branch-watch still works

### Error Scenarios
- [ ] Stop database → verify watch doesn't crash
- [ ] Restart database → verify watch recovers
- [ ] Delete .git/HEAD → verify warning logged, file watching continues
- [ ] Invalid repository path → verify clear error message

### NDJSON Event Validation
- [ ] All events are valid single-line JSON
- [ ] branch_switched event has all required fields:
  - `event_type: "branch_switched"`
  - `timestamp` (ISO 8601)
  - `repo_name`
  - `from_branch`
  - `to_branch`
  - `from_worktree_name`
  - `to_worktree_name`
- [ ] Timestamps are ISO 8601 format
- [ ] Events parseable by `jq`

### Performance
- [ ] CPU usage <5% while idle
- [ ] Memory usage <25MB
- [ ] Branch switch detected within 2 seconds
- [ ] File changes detected within 1 second

### Documentation
- [ ] CLAUDE.md accurately describes unified watch
- [ ] Help text (--help) is correct
- [ ] Examples in docs actually work
- [ ] No broken links

## Technical Requirements
- Use real maproom binary (not test mode)
- Use production database (or production-like environment)
- Test on actual development repository
- Document results in ticket comments
- Execute from: `crates/maproom/` directory
- Binary path: `cargo build --release && ./target/release/crewchief-maproom`

## Implementation Notes

### How to Execute Manual Tests

For each checklist item:

1. **Execute the test**
   - Follow the exact steps described
   - Use real maproom binary
   - Document exact commands used

2. **Document result** (pass/fail)
   - Mark checkbox in this ticket
   - Add comment with details

3. **If fail**, document:
   - What happened
   - Expected vs actual behavior
   - Steps to reproduce
   - Error messages or logs

4. **Create follow-up ticket** for failures (if needed)

### Example Documentation Format

```markdown
## Test Results

### Functional Testing
✓ Start `maproom watch` - PASS
  - Command: `./target/release/crewchief-maproom watch /workspace`
  - Output: "Watching /workspace for branch switches..."

✓ Edit file - PASS
  - Edited: `test.txt`
  - Event: `{"event_type":"file_processed","timestamp":"2025-11-16T14:30:00Z",...}`

✗ Switch branch - FAIL
  - Expected: branch_switched event
  - Actual: No event emitted
  - Steps: `git checkout feature`
  - Follow-up: Created ticket UNIWATCH-5005
```

### NDJSON Event Testing

Use `jq` to validate events:
```bash
# Watch output and parse JSON
./target/release/crewchief-maproom watch | while read -r line; do
  echo "$line" | jq '.' || echo "INVALID JSON: $line"
done
```

### Performance Testing

Use system monitoring tools:
```bash
# Monitor CPU and memory
top -p $(pgrep crewchief-maproom)

# Or use htop
htop -p $(pgrep crewchief-maproom)
```

## Dependencies
- UNIWATCH-5001 (Execute and Verify Unit Tests) - MUST pass
- UNIWATCH-5002 (Create and Execute Integration Tests) - MUST pass
- UNIWATCH-5003 (Create End-to-End Bash Test Script) - MUST pass
- ALL Phase 1-4 tickets complete (UNIWATCH-1001 through UNIWATCH-4003)

## Risk Assessment
- **Risk**: Manual testing is subjective
  - **Mitigation**: Use concrete checklist with measurable outcomes; document exact commands and outputs

- **Risk**: Edge cases might be missed
  - **Mitigation**: Focus on critical user workflows; test error scenarios explicitly

- **Risk**: Documentation might be outdated
  - **Mitigation**: Follow documentation exactly as written; update docs if incorrect

- **Risk**: Performance varies by environment
  - **Mitigation**: Test on representative hardware; document environment specs

## Files/Packages Affected
- No files modified (testing only)
- Verification of:
  - `crates/maproom/src/indexer/watcher.rs`
  - `crates/maproom/src/cli.rs`
  - `crates/maproom/CLAUDE.md`
  - Help text output
  - NDJSON event output

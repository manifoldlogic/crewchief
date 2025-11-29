# Ticket: MRPROG-2004: Manual testing across terminal environments for watch command

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Conduct manual testing of the watch command across different terminal environments to verify minimal output displays correctly, dots appear in real-time, and there's no output corruption. This validates the Phase 2 implementation before moving to Phase 3.

## Background
Terminal output behavior varies across different terminal emulators, multiplexers, and output contexts (TTY vs redirected). Manual testing ensures the watch minimal output works correctly in common development environments.

This is pragmatic compatibility testing: verify it works in the terminals developers actually use, not exhaustive coverage of every possible environment.

This ticket is the final task in Phase 2 (Watch Minimal Output) as defined in the MRPROG project plan. It validates that the watch command's minimal output mode works correctly across real-world terminal environments before proceeding to Phase 3 (Scan Progress).

## Acceptance Criteria
- [ ] Tested in 3+ different terminal emulators (iTerm2, Terminal.app, Windows Terminal, etc.)
- [ ] Tested in tmux and/or screen multiplexer
- [ ] Tested with output redirected to file (non-TTY)
- [ ] Verified dots appear in real-time (not buffered until completion)
- [ ] Verified no output corruption or garbled text
- [ ] Verified timing information displays correctly
- [ ] Tested with 1 file change and 5+ file changes
- [ ] Testing results documented in validation report
- [ ] Any issues found are documented and resolved
- [ ] --verbose mode tested in at least 2 environments

## Technical Requirements

### Testing Matrix

| Environment | Minimal Mode | Verbose Mode | Notes |
|------------|--------------|--------------|-------|
| iTerm2 (macOS) | ☐ Pass | ☐ Pass | Primary dev environment |
| Terminal.app (macOS) | ☐ Pass | ☐ Pass | Default macOS terminal |
| Windows Terminal | ☐ Pass | ☐ Pass | Primary Windows environment |
| WSL2 Ubuntu | ☐ Pass | ☐ Pass | Common dev setup |
| tmux | ☐ Pass | ☐ Pass | Terminal multiplexer |
| screen | ☐ Pass | ☐ Skip | Optional |
| VS Code terminal | ☐ Pass | ☐ Pass | Common IDE |
| Non-TTY (redirect) | ☐ Pass | ☐ Pass | Log file output |

### Per-Environment Checklist

For each environment, verify:
1. Watch starts without errors
2. File change triggers output
3. Change count displayed correctly ("🔄 N file(s) changed")
4. "Indexing: " appears
5. Dots appear one-by-one (not all at once at end)
6. Completion message appears ("✅ Done in X.Xs")
7. Timing is accurate (within reason)
8. No garbled output, corruption, or artifacts
9. Multiple change events work correctly
10. Ctrl+C stops watch cleanly

### Test Procedure

**1. Setup test repository:**
```bash
cd /tmp
mkdir watch-test && cd watch-test
git init
for i in {1..5}; do echo "file $i" > file$i.txt; done
git add . && git commit -m "initial"
```

**2. Test minimal mode (each environment):**
```bash
maproom watch

# In another terminal:
echo "changed" > file1.txt
# Verify: See "🔄 1 file(s) changed", "Indexing: .", "✅ Done in X.Xs"

# Change multiple files:
for i in {1..3}; do echo "updated $i" > file$i.txt; done
# Verify: See "🔄 3 file(s) changed", "Indexing: ...", "✅ Done in X.Xs"
```

**3. Test verbose mode:**
```bash
maproom watch --verbose

# Trigger change
echo "verbose test" > file1.txt
# Verify: See "Detected changes in 1 file(s)", "Re-indexing...", file path, "Index updated"
```

**4. Test non-TTY:**
```bash
maproom watch > watch-output.log 2>&1 &
WATCH_PID=$!

echo "non-tty test" > file1.txt
sleep 5
kill $WATCH_PID

cat watch-output.log
# Verify: Output is readable, no control characters garbling
```

**5. Test in tmux:**
```bash
tmux new-session -d -s watch-test
tmux send-keys -t watch-test "cd /tmp/watch-test && maproom watch" C-m
sleep 2
echo "tmux test" > /tmp/watch-test/file1.txt
sleep 3
tmux capture-pane -t watch-test -p
tmux kill-session -t watch-test
# Verify: Output captured shows expected format
```

## Implementation Notes

### Testing Approach
1. Work through testing matrix systematically
2. Document each environment's results in validation report
3. Note any visual artifacts or issues
4. Take screenshots if helpful for documentation
5. If issues found, create sub-tickets to fix
6. Retest after fixes

### Validation Report Structure
Create `.crewchief/projects/MRPROG_maproom-progress-ux/testing/phase2-validation-report.md` with:
- Testing matrix with Pass/Fail results
- Screenshots or output captures
- Any issues discovered
- Recommendations for improvements
- Sign-off for Phase 2 completion

### Known Considerations
- Emoji support varies by terminal (acceptable, already used in codebase)
- Tmux may buffer output differently
- Non-TTY contexts won't show real-time dots (buffered until newline)
- Windows Terminal may render differently than Unix terminals

## Dependencies
- **BLOCKED BY**: MRPROG-2001 (watch minimal mode implementation)
- **BLOCKED BY**: MRPROG-2002 (--verbose flag)
- **BLOCKED BY**: MRPROG-2003 (integration tests passing)

## Risk Assessment
- **Risk**: Some terminals might not support emoji
  - **Mitigation**: Acceptable, emoji already used elsewhere in codebase

- **Risk**: Tmux might buffer output differently
  - **Mitigation**: If so, document as known limitation; most users watch directly

- **Risk**: Non-TTY might need different handling
  - **Mitigation**: Dots should still work, just not flushed mid-line; verify output is still readable

- **Risk**: Limited access to Windows/WSL environments
  - **Mitigation**: Test on available platforms; community can report issues on others

## Files/Packages Affected
- **CREATE**: `.crewchief/projects/MRPROG_maproom-progress-ux/testing/phase2-validation-report.md`
- **READ**: Output from `maproom watch` command
- **READ**: Output from `maproom watch --verbose` command

## Estimated Effort
2-3 hours

## Success Criteria
- Works in 5+ common environments
- No critical visual corruption
- Dots appear in real-time in TTY contexts
- Non-TTY output is readable (even if not real-time dots)
- Validation report documents all testing

## References
- Quality strategy: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/quality-strategy.md` (Manual Testing section)
- Plan: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md` (Phase 2, task 4)
- Architecture: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/architecture.md` (Testing Strategy)

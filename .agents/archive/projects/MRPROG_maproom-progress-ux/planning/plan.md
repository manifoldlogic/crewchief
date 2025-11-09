# Implementation Plan: Maproom Progress UX Enhancement

## Project Phases

This project follows a **build-test-ship** approach with three clear phases:

### Phase 1: Progress Tracking Foundation
Build the core progress tracking infrastructure

### Phase 2: Watch Minimal Output
Enhance watch command with minimal output mode

### Phase 3: Polish & Documentation
Finalize help text, test across terminals, ship

## Phase 1: Progress Tracking Foundation

**Goal**: Add real-time progress indicator to scan command

**Deliverables**:
1. ProgressTracker module with output formatting
2. Integration with scan_worktree function
3. TTY detection and adaptive output
4. Timing emphasis in scan output
5. Unit tests for progress calculations
6. Performance benchmarks

**Agent Assignment**: `rust-indexer-engineer` (or general Rust agent)

**Success Criteria**:
- ✅ `maproom scan` shows real-time progress during indexing
- ✅ Progress updates appear every 200-500ms
- ✅ Final output shows "Completed in X.Xs" prominently
- ✅ TTY mode uses line overwriting, non-TTY uses periodic updates
- ✅ Performance overhead <5% (verified via benchmark)
- ✅ Unit tests pass with >80% coverage of ProgressTracker

**Tasks** (will be broken into tickets):
1. Create `crates/maproom/src/progress.rs` module
   - Define `OutputMode` enum
   - Implement `ProgressTracker` struct
   - Add progress calculation methods
   - Implement TTY detection
   - Add throttling logic

2. Modify `crates/maproom/src/indexer/mod.rs`
   - Add optional `progress: Option<&ProgressTracker>` parameter to `scan_worktree()`
   - Integrate progress updates in file processing loop
   - Integrate progress updates in embedding generation
   - Add timing measurement
   - Format final output with prominent duration

3. Update `crates/maproom/src/main.rs`
   - Add `--verbose` flag to `Commands::Scan`
   - Create ProgressTracker in scan handler
   - Pass progress tracker to `scan_worktree()`
   - Handle OutputMode selection

4. Write unit tests
   - Test percentage calculations (edge cases: 0, 1, large numbers)
   - Test throttling logic
   - Test concurrent updates (AtomicUsize safety)
   - Test TTY vs non-TTY formatting

5. Add performance benchmarks
   - Benchmark scan without progress
   - Benchmark scan with progress
   - Verify <5% overhead

6. Update help text
   - Clarify default directory behavior
   - Document `--verbose` flag
   - Show usage examples

**Dependencies**: None (self-contained)

**Risks**:
- Progress updates might slow down indexing → Mitigated by throttling and benchmarking
- TTY detection might fail → Mitigated by safe fallback to non-TTY

**Estimated Complexity**: Medium (new module + integration points)

---

## Phase 2: Watch Minimal Output

**Goal**: Make watch command output minimal and glanceable by default

**Deliverables**:
1. Minimal output mode for watch_worktree
2. Compact change detection output (3 lines vs. current 5-7)
3. Dot-per-file progress indicator
4. Timing for each re-index event
5. `--verbose` flag to restore detailed output
6. Integration tests for watch modes

**Agent Assignment**: `rust-indexer-engineer` (or general Rust agent)

**Success Criteria**:
- ✅ `maproom watch` shows minimal output by default
- ✅ Change events display: "🔄 N files changed"
- ✅ Indexing shows: "Indexing: ....." (one dot per file)
- ✅ Completion shows: "✅ Done in X.Xs"
- ✅ `--verbose` flag restores old detailed output
- ✅ Integration tests verify both output modes

**Tasks** (will be broken into tickets):
1. Modify `crates/maproom/src/indexer/mod.rs` - `watch_worktree()`
   - Add `output_mode: OutputMode` parameter
   - Implement minimal output branch:
     - Print change count
     - Print "Indexing: " without newline
     - Print dot per file
     - Print completion with timing
   - Keep existing verbose output in verbose branch
   - Add timing measurement for each event

2. Update `crates/maproom/src/main.rs`
   - Add `--verbose` flag to `Commands::Watch`
   - Determine OutputMode from flag
   - Pass output_mode to `watch_worktree()`

3. Write integration tests
   - Test watch with minimal output (default)
   - Test watch with `--verbose` flag
   - Verify output format matches expected
   - Test with multiple file changes

4. Manual testing across terminals
   - Test in iTerm2, Terminal.app, Windows Terminal
   - Test in tmux/screen
   - Test non-TTY (redirected to file)
   - Verify dots appear in real-time
   - Verify no output corruption

**Dependencies**: Phase 1 (OutputMode enum, timing utilities)

**Risks**:
- Dots might not flush immediately → Mitigated by explicit stdout flush
- Output might be garbled in some terminals → Mitigated by manual testing

**Estimated Complexity**: Low-Medium (similar pattern to Phase 1, smaller scope)

---

## Phase 3: Polish & Documentation

**Goal**: Final polish, comprehensive testing, documentation updates

**Deliverables**:
1. Updated help text emphasizing defaults
2. Updated README/docs with new features
3. CI integration for tests and benchmarks
4. Manual testing report across environments
5. Performance validation
6. User-facing changelog entry

**Agent Assignment**: `general-purpose` or `technical-researcher` (for docs)

**Success Criteria**:
- ✅ Help text clearly documents default directory behavior
- ✅ `--help` output shows examples of new features
- ✅ CI runs all tests and benchmarks
- ✅ Manual testing completed in 5+ environments
- ✅ Documentation updated with new UX features
- ✅ Changelog entry written

**Tasks** (will be broken into tickets):
1. Polish help text
   - Update scan/watch command descriptions
   - Add examples: `maproom scan` (uses current dir)
   - Document `--verbose` flag behavior
   - Show typical output examples

2. Update documentation
   - Update `packages/maproom-mcp/README.md`
   - Add section on progress indicators
   - Add section on output modes
   - Include screenshots/examples of output

3. CI integration
   - Ensure `cargo test` runs in CI
   - Add `cargo bench` to CI (informational, not blocking)
   - Add `cargo clippy` check
   - Add `cargo audit` check

4. Manual testing matrix
   - Test on macOS (iTerm2, Terminal.app)
   - Test on Windows (Windows Terminal, WSL2)
   - Test on Linux (GNOME Terminal, tmux)
   - Test in VS Code integrated terminal
   - Test non-TTY (redirect to log file)
   - Document results

5. Performance validation
   - Run benchmarks on realistic codebase
   - Verify <5% overhead claim
   - Document results in performance report

6. Write changelog entry
   - Summarize new features (progress, minimal watch)
   - Document new flags (`--verbose`)
   - Note default behavior (current dir)
   - Include migration notes (none needed)

**Dependencies**: Phase 1 & 2 complete

**Risks**: None (polish only)

**Estimated Complexity**: Low (documentation and validation)

---

## Ticket Structure

Each phase will be broken into granular tickets following the pattern:

```
MRPROG-1001: Create ProgressTracker module
MRPROG-1002: Integrate ProgressTracker with scan_worktree
MRPROG-1003: Add TTY detection and adaptive output
MRPROG-1004: Add timing measurement and emphasis
MRPROG-1005: Write ProgressTracker unit tests
MRPROG-1006: Add performance benchmarks for scan
MRPROG-1007: Update scan command help text

MRPROG-1008: Implement minimal output mode for watch_worktree
MRPROG-1009: Add --verbose flag to watch command
MRPROG-1010: Write integration tests for watch modes
MRPROG-1011: Manual testing across terminal environments

MRPROG-1012: Polish help text with examples
MRPROG-1013: Update maproom-mcp README documentation
MRPROG-1014: Integrate tests and benchmarks into CI
MRPROG-1015: Performance validation and report
MRPROG-1016: Write changelog entry
```

**Estimated Total**: 15-20 tickets

---

## Implementation Order

**Sequential dependencies**:
1. Phase 1 must complete before Phase 2 (OutputMode dependency)
2. Phase 2 must complete before Phase 3 (can't polish incomplete work)

**Parallel opportunities**:
- Within Phase 1: Tests can be written alongside implementation
- Within Phase 2: Integration tests can be written while implementing
- Phase 3: Documentation can be drafted early, updated at end

**Recommended workflow**:
1. Implement ProgressTracker module (1-2 tickets)
2. Integrate with scan (1-2 tickets)
3. Write tests (1-2 tickets)
4. Implement watch minimal mode (1-2 tickets)
5. Write integration tests (1 ticket)
6. Manual testing (1 ticket)
7. Polish and documentation (2-3 tickets)

---

## Testing Strategy Per Phase

**Phase 1 Testing**:
- Unit tests for ProgressTracker (during implementation)
- Performance benchmarks (before merging)
- Manual smoke test in local terminal

**Phase 2 Testing**:
- Integration tests for watch modes (during implementation)
- Manual testing in 3-5 terminals (before merging)
- Regression testing (existing tests still pass)

**Phase 3 Testing**:
- Full test suite run
- Cross-platform validation
- User acceptance testing (dogfooding)

---

## Rollout Plan

**Merge Strategy**: Feature branch → PR → main

**Feature Flag**: None needed (changes are additive, `--verbose` preserves old behavior)

**Rollback Plan**: If issues arise post-merge, revert PR and investigate

**User Communication**:
- Update changelog with new features
- No breaking changes, no migration guide needed
- Users automatically get better UX on next update

---

## Success Metrics

**Quantitative**:
- Progress updates appear every 200-500ms during scan
- Watch output reduced from 5-7 lines to 3 lines per event
- Performance overhead <5%
- All tests pass (unit + integration)
- Zero regressions in existing functionality

**Qualitative**:
- Developers can tell if scan is progressing or stuck
- Watch output is glanceable without demanding attention
- Help text makes default directory behavior clear
- Output feels polished and professional

---

## Timeline Estimate

**Phase 1**: 2-3 days (implementation + tests)
**Phase 2**: 1-2 days (implementation + tests)
**Phase 3**: 1 day (polish + docs)

**Total**: 4-6 days (single developer, full-time)

**Buffer**: Add 20% for unexpected issues → **5-7 days**

---

## Dependencies & Prerequisites

**Technical Prerequisites**:
- Rust toolchain (already present)
- `atty` crate for TTY detection (add to Cargo.toml)
- Test fixtures (small/medium repos for integration tests)

**Knowledge Prerequisites**:
- Familiarity with Rust async/await (for indexer integration)
- Understanding of terminal output formatting
- Basic understanding of existing indexer architecture

**External Dependencies**: None (self-contained changes)

---

## Risk Mitigation

**Risk 1: Performance degradation**
- **Mitigation**: Throttle updates, benchmark early
- **Fallback**: Make progress optional via env var if needed

**Risk 2: Terminal compatibility issues**
- **Mitigation**: Test in common terminals, fallback to non-TTY
- **Fallback**: Document known issues, `--verbose` as workaround

**Risk 3: Scope creep**
- **Mitigation**: Stick to MVP features, defer enhancements
- **Definition of Done**: Clear success criteria per phase

---

## Future Enhancements (Out of Scope)

**Post-MVP ideas** (not in this project):
- JSON output mode for programmatic parsing
- Colored output with ANSI codes
- Progress bars (fancy terminal UI)
- Configurable output templates
- Machine-readable log format

**Rationale**: Keep MVP focused. These can be separate projects if there's demand.

---

## Completion Checklist

Project is complete when:
- [ ] All Phase 1 tickets verified and committed
- [ ] All Phase 2 tickets verified and committed
- [ ] All Phase 3 tickets verified and committed
- [ ] All tests passing (unit + integration + manual)
- [ ] Performance benchmarks show <5% overhead
- [ ] Documentation updated
- [ ] Changelog entry written
- [ ] PR merged to main
- [ ] User acceptance testing completed (dogfooding)

---

## Summary

This plan delivers **high-value UX improvements** with **minimal risk** through three clear phases:

1. **Foundation**: Build progress tracking infrastructure
2. **Enhancement**: Add minimal watch output
3. **Polish**: Test, document, ship

**Key insight**: Focus on user experience wins (visible progress, quiet watch) without touching core indexing logic. Safe, additive changes that make the tool more pleasant to use daily.

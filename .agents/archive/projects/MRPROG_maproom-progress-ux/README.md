# MRPROG: Maproom Progress UX Enhancement

**Status**: Planning Complete, Ready for Implementation
**Project Slug**: `MRPROG`
**Complexity**: Medium
**Timeline**: 5-7 days

## Problem

The current `maproom scan` and `watch` commands have poor UX:

1. **Invisible Progress**: Scan goes silent for 30+ seconds on large codebases, creating anxiety
2. **Verbose Watch Output**: Watch mode floods the console with 5-7 lines per event, creating noise
3. **Implicit Defaults**: Users don't know the commands default to current directory

This makes the tool less pleasant to use during daily development.

## Solution

Enhance output formatting with:

1. **Real-time scan progress**: Show file/chunk counts, percentages, ETA during indexing
2. **Minimal watch mode**: Compact 3-line output with dot-per-file indicator
3. **Timing emphasis**: Prominent "Completed in X.Xs" for all operations
4. **Clear help text**: Document default directory behavior

**Key insight**: Modern CLI UX is about **just-enough information, just-in-time**. Not silent, not verbose, but informative and calm.

## Example Output

**Before (scan)**:
```
🔍 Scanning worktree: main @ a1b2c3d4
   Repository: crewchief
   Path: /workspace
Using provider: ollama
Running indexer...
[30 seconds of silence...]
✅ Scan completed successfully!
   Files processed: 1250
```

**After (scan)**:
```
🔍 Scanning worktree: main @ a1b2c3d4
   Repository: crewchief
   Path: /workspace
Using provider: ollama

Processing: 450/1250 files (36%) | Embeddings: 2500/6000 (42%) | ETA: 45s
[updates in place every 200ms]

✅ Completed in 45.2s

   Files processed: 1250
   Files skipped: 150
   Total chunks: 6000
```

**Before (watch)**:
```
Detected changes in 5 file(s)
Re-indexing...
  - /workspace/src/main.rs
  - /workspace/src/lib.rs
  - /workspace/src/utils.rs
  - /workspace/tests/integration.rs
  - /workspace/README.md
Index updated
```

**After (watch)**:
```
🔄 5 files changed
Indexing: ..... ✅ Done in 2.3s
```

## Architecture

**Approach**: Enhance presentation layer without touching core indexing logic.

**New Components**:
- `ProgressTracker` module (`crates/maproom/src/progress.rs`)
- `OutputMode` enum (Minimal/Verbose)
- TTY detection for adaptive output

**Modified Components**:
- `scan_worktree()`: Add optional progress parameter
- `watch_worktree()`: Add output_mode parameter
- CLI arg parsing: Add `--verbose` flag

**Key Design Decision**: Progress tracking is injected as optional dependency, keeping indexer logic pure.

## Implementation Phases

### Phase 1: Progress Tracking Foundation
- Create ProgressTracker module
- Integrate with scan command
- Add TTY detection and adaptive output
- Unit tests and performance benchmarks

**Success**: Scan shows real-time progress, <5% overhead

### Phase 2: Watch Minimal Output
- Implement minimal output mode for watch
- Add dot-per-file indicator
- Integration tests for both modes

**Success**: Watch output reduced from 5-7 lines to 3 lines per event

### Phase 3: Polish & Documentation
- Update help text and documentation
- Manual testing across terminals
- CI integration and performance validation

**Success**: Professional polish, comprehensive testing

## Testing Strategy

**Pragmatic approach**: Test what matters (correctness, performance, compatibility), not what doesn't (exact formatting, exotic terminals).

**Key Tests**:
- Unit: Progress calculations, throttling, concurrent updates
- Integration: Scan/watch output formats, --verbose flag
- Performance: <5% overhead via benchmarks
- Manual: Works in 5+ common terminals

**Test Coverage**: 80%+ of ProgressTracker, 100% of core workflows

## Security

**Risk Level**: MINIMAL

This is purely output formatting. Zero new attack surface, zero new data handling.

**Analysis**:
- No terminal injection (only numeric counters displayed)
- No DoS (output throttled and bounded)
- No information disclosure (same data as existing output)
- Memory safe (pure Rust, no unsafe code)

**Verdict**: No security concerns. Standard code review sufficient.

## Relevant Agents

**Implementation**:
- Primary: `rust-indexer-engineer` (or general Rust agent)
- Testing: `unit-test-runner`, `integration-tester`

**Workflow**:
- `verify-ticket`: Validate acceptance criteria
- `commit-ticket`: Create conventional commits

## Planning Documents

Comprehensive planning available in `planning/` directory:

- [analysis.md](planning/analysis.md) - Deep problem understanding and requirements
- [architecture.md](planning/architecture.md) - Technical design and implementation approach
- [quality-strategy.md](planning/quality-strategy.md) - Pragmatic testing strategy
- [security-review.md](planning/security-review.md) - Security assessment (minimal risk)
- [plan.md](planning/plan.md) - Phased implementation plan with tickets

## Success Criteria

**Quantitative**:
- Progress updates every 200-500ms during scan
- Watch output: 3 lines per event (down from 5-7)
- Performance overhead: <5%
- Test coverage: 80%+ of new code

**Qualitative**:
- Users can tell if scan is progressing
- Watch output is glanceable
- Help text is clear
- Output feels professional

## Next Steps

1. Run `/create-project-tickets MRPROG` to generate individual work tickets
2. Run `/work-on-project MRPROG` to execute all tickets sequentially
3. Each ticket follows: implement → test → verify → commit workflow

## Timeline

**Estimated**: 5-7 days (single developer, full-time)

**Breakdown**:
- Phase 1: 2-3 days
- Phase 2: 1-2 days
- Phase 3: 1 day
- Buffer: 20%

## Dependencies

**Technical**:
- Add `atty` crate for TTY detection
- Test fixtures (small/medium repos)

**External**: None (self-contained)

---

**Project Type**: UX Enhancement
**Risk Level**: Low (cosmetic changes only)
**User Impact**: High (daily workflow improvement)
**Complexity**: Medium (new module + integration)

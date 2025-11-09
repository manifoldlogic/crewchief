# Analysis: Maproom Progress UX Enhancement

## Problem Space

### The Core User Experience Gap

The current maproom-mcp `scan` and `watch` commands suffer from poor UX in three distinct areas:

1. **Invisible Progress During Scan**: Users run `maproom scan` on a large codebase and see nothing for 30+ seconds except "Running indexer...". No indication of progress, no estimate of completion time, no way to know if it's hung or just slow. This creates anxiety and uncertainty.

2. **Verbose Watch Output**: The `watch` command floods the console with multi-line explanatory messages every time files change. For developers with the watcher running in a terminal pane, this creates noise pollution that obscures other output and makes it hard to understand state at a glance.

3. **Implicit Directory Behavior**: While the code already defaults to the current directory when no path is specified, this behavior isn't clear to users. They habitually type `maproom scan .` because they don't know the `.` is optional.

### Why This Matters

**Developer Context Switching**: The primary use case for maproom is continuous indexing during development. Developers need to know indexing is happening without constantly monitoring it. Current verbose output demands attention; better UX provides just-enough information at-a-glance.

**Trust in Long-Running Operations**: When scanning a large monorepo (10,000+ files), users need confidence the operation is progressing. Silent operations lasting minutes feel broken. Progress indicators build trust.

**Terminal Real Estate**: Developers run watch commands in background terminal panes. Compact output means they can monitor status without dedicating excessive screen space or filtering through noise.

## Current Industry Solutions

### Reference Implementations

**npm/pnpm/yarn**: Package managers solved this years ago:
- Progress bars with file counts during install
- Minimal output modes for CI environments
- Timing emphasis ("Done in 12.3s")
- Spinners for indeterminate operations

**ripgrep/fd/other CLIs**:
- Progress indicators for searches on large directories
- Real-time counts (e.g., "Searched 1250/5000 files")
- Minimal output modes (`--quiet`, `--json`)

**Webpack/Vite/build tools**:
- Percentage-based progress (e.g., "Building... 45%")
- Per-file indicators during watch mode (`.` per changed file)
- Clear completion messages with timing

**Git operations**:
- Object counting during clones/fetches
- Delta compression progress
- Network transfer speeds and estimates

### Key Patterns

1. **Dual-mode output**: Verbose for debugging, minimal for daily use
2. **Progress percentages**: When total work is known upfront
3. **Activity indicators**: When total work is unknown (spinners, dots)
4. **Timing emphasis**: Always show how long operations took
5. **At-a-glance status**: One-line summaries that update in place

## Current State Analysis

### What Exists Today

Based on codebase research (`packages/maproom-mcp/bin/cli.cjs`, `crates/maproom/src/indexer/mod.rs`):

**Scan Command**:
- **CLI wrapper** (`cli.cjs` lines 1495-1583):
  - Already defaults to `process.cwd()` if no path provided (line 1497)
  - Prints repository metadata (name, worktree, commit)
  - Shows embedding provider being used
  - Outputs "Running indexer..." then goes silent

- **Rust implementation** (`indexer/mod.rs` lines 286-435):
  - Prints start message with worktree/commit info
  - Silent during processing (no progress updates)
  - Prints final summary: files processed/skipped, chunks, size, languages
  - No timing information displayed to user

**Watch Command**:
- **CLI wrapper** (`cli.cjs` lines 1588-1663):
  - Already defaults to `process.cwd()` if no path provided (line 1589)
  - Verbose startup: repository, worktree, provider, debounce settings
  - Per-change output: "Detected changes in N file(s)", "Re-indexing...", "Index updated"
  - Multiple `console.error` lines per event

- **Rust implementation** (`indexer/mod.rs` lines 525+):
  - Database connection validation messages
  - File system watch loop with `tracing::info!` logging
  - No minimal output mode

**Embedding Generation Progress** (`crates/maproom/src/main.rs` lines 233-298):
- Has some progress tracking during auto-embedding generation
- Shows "Processing batch X/Y" messages
- Could be leveraged for scan progress

### What's Missing

1. **No real-time progress during scan**:
   - No indication of files processed vs. total
   - No percentage complete
   - No time estimates
   - No way to tell if it's stuck

2. **No minimal watch mode**:
   - Can't suppress verbose explanations
   - Each change event prints 3-5 lines
   - No compact representation

3. **No timing emphasis**:
   - Scan doesn't prominently show duration
   - Watch doesn't show per-event timing
   - Users can't tell if performance is degrading

4. **Unclear defaults**:
   - Help text doesn't emphasize current directory default
   - Users don't know they can omit the path argument

## Root Causes

### Architectural Decisions

The current implementation was optimized for **correctness and functionality**, not UX polish:

1. **Rust stdout separation**: Rust indexer uses `println!` for output, which goes to stdout. CLI wrapper uses `console.error` for its own messages. This creates output stream coordination complexity.

2. **No progress protocol**: There's no mechanism for the Rust indexer to report progress back to the Node.js CLI wrapper. The wrapper spawns the Rust process and waits for completion.

3. **Logging vs. UX**: Current output uses logging-style verbosity (helpful for debugging) rather than user-focused minimalism.

### Historical Context

This is a classic **v1 → v2 maturity issue**:
- **v1 priority**: Make it work (✅ achieved)
- **v2 priority**: Make it pleasant to use (← we are here)

The foundation is solid. Now we're adding the UX layer that makes daily usage delightful.

## Requirements Synthesis

### User Stories

**Story 1: Long-running scan confidence**
> As a developer indexing a large codebase, I want to see real-time progress during scanning so I know the operation is advancing and can estimate when it will complete.

**Story 2: Glanceable watch status**
> As a developer with a watch command running in a terminal pane, I want minimal output that shows status at-a-glance so I can monitor without visual noise.

**Story 3: Zero-friction invocation**
> As a developer working in my repository, I want to run `maproom scan` without specifying the current directory so I have fewer characters to type.

**Story 4: Performance visibility**
> As a developer optimizing my workflow, I want to see how long operations take so I can identify performance issues and track improvements.

### Functional Requirements

**FR1: Scan progress indicator**
- Show files processed count and total (e.g., "Processing: 450/1200 files (37%)")
- Show chunks with embeddings count (e.g., "Embeddings: 2500/6000 (42%)")
- Estimate time remaining based on current rate
- Update in place (no scrolling output)

**FR2: Watch minimal output mode**
- Default to minimal mode
- One-line change detection: "🔄 5 files changed"
- One-char-per-file progress: "Indexing: ....." (. = file processing)
- Completion with timing: "✅ Done in 2.3s"

**FR3: Default directory behavior**
- Scan and watch commands default to current directory when path omitted
- Help text clearly documents this behavior
- Output shows which directory was used

**FR4: Timing emphasis**
- Scan: Show total duration prominently at end ("✅ Completed in 45.2s")
- Watch: Show per-event duration ("✅ Done in 2.3s")
- Use consistent formatting (seconds with 1 decimal place)

### Non-Functional Requirements

**NFR1: Performance**: Progress updates must not slow indexing by >5%
**NFR2: Compatibility**: Existing `--verbose` flags should still work
**NFR3: Testability**: Output modes must be programmatically verifiable
**NFR4: Terminal compatibility**: Must work in common terminals (iTerm, Terminal.app, Windows Terminal, tmux)

## Research Findings

### Progress Indicator Patterns

**Pattern 1: Known total (percentage)**
```
Processing: 450/1200 files (37%) [==============>          ] ETA: 45s
```
**Pros**: Clear completion estimate, visual progress bar
**Cons**: Requires knowing total upfront, complex terminal manipulation

**Pattern 2: Simple counter**
```
Processing: 450/1200 files (37%) | Embeddings: 2500/6000 (42%)
```
**Pros**: Simple, no terminal escapes needed, works everywhere
**Cons**: No visual bar, less immediate

**Pattern 3: Spinner with counts**
```
⠋ Processing files... 450/1200 (37%)
```
**Pros**: Shows activity, minimal screen space
**Cons**: Can be distracting, harder to read in logs

**Decision**: Use **Pattern 2** for MVP. It's simple, testable, works in all terminals, and provides the key information. Can enhance with progress bars later if needed.

### Minimal Watch Output Patterns

**Pattern 1: Status line (overwrites)**
```
🔄 Indexing: ..... ✅ Done in 2.3s
```
**Pros**: Single line, very clean
**Cons**: Complex terminal manipulation, doesn't work in all contexts

**Pattern 2: Compact multi-line**
```
🔄 5 files changed
Indexing: .....
✅ Done in 2.3s
```
**Pros**: Simple output, clear state transitions, works everywhere
**Cons**: 3 lines per event (still much better than current 5+)

**Pattern 3: One-liner**
```
[12:34:56] ✅ Indexed 5 files in 2.3s
```
**Pros**: Absolute minimum output
**Cons**: Loses some status clarity

**Decision**: Use **Pattern 2** for MVP. Three lines is acceptable for watch events (vs. current 5-7), and it provides clear state visibility. The dot-per-file indicator gives satisfying real-time feedback.

### Terminal Output Techniques

**For progress updates (scan)**:
- Use `\r` (carriage return) to overwrite current line
- Track terminal width to avoid wrapping
- Fallback to new lines if terminal width unavailable
- Always end with `\n` for final message

**For minimal output (watch)**:
- Use simple `println!` for state transitions
- No ANSI escape codes needed for MVP
- Emit dots as files complete (natural streaming)

## Constraints and Assumptions

### Technical Constraints

**TC1: Node.js ↔ Rust boundary**: CLI wrapper spawns Rust binary as subprocess. Progress must flow through stdout/stderr.

**TC2: Embedding provider latency**: Embedding generation dominates scan time. Progress tracking must account for this.

**TC3: File system events**: Watch mode uses notify-rs crate. Event debouncing happens in Rust.

**TC4: No breaking changes**: Existing command-line flags and behavior must remain compatible.

### Assumptions

**A1**: Users run commands in interactive terminals (not just CI/scripting)
**A2**: Progress updates every 100-500ms are sufficient (not real-time per file)
**A3**: Users prefer minimal output by default, verbose on request
**A4**: Terminal width is at least 80 characters (reasonable default)

## Success Criteria

### Quantitative Metrics

1. **Scan progress visible**: Progress updates appear at least every 500ms during active indexing
2. **Watch output reduction**: 3 lines per event (down from 5-7 currently)
3. **Performance overhead**: <5% slowdown from progress tracking
4. **Timing accuracy**: Duration measurements within 50ms of actual

### Qualitative Outcomes

1. **User confidence**: Developers can tell if scan is progressing or stuck
2. **Reduced distraction**: Watch output is glanceable without demanding attention
3. **Discoverability**: Help text makes default directory behavior clear
4. **Professional polish**: Output feels comparable to modern CLI tools

## Dependencies

### Internal Dependencies
- `packages/maproom-mcp/bin/cli.cjs` - CLI wrapper
- `crates/maproom/src/indexer/mod.rs` - Indexer implementation
- `crates/maproom/src/main.rs` - CLI argument parsing

### External Dependencies
- None (uses existing runtime and libraries)

### Data Dependencies
- File counts (from tree-sitter traversal)
- Chunk counts (from indexing process)
- Embedding counts (from database queries)

## Open Questions

**Q1**: Should progress indicator show file count or chunk count as primary metric?
**A1**: Both. Files are easier to understand, chunks show actual work granularity. Show files primarily, chunks secondarily.

**Q2**: Should we add a `--quiet` flag or make minimal the default?
**A2**: Make minimal the default for watch. Scan should show progress (not quiet). Add `--verbose` to restore old behavior.

**Q3**: How to handle progress updates when terminal isn't a TTY (e.g., in logs)?
**A3**: Detect TTY. If not a TTY, fall back to periodic summary lines (every 10% or every N seconds) rather than overwriting.

**Q4**: Should timing use wall clock or CPU time?
**A4**: Wall clock. Users care about actual wait time, not CPU consumption.

## Conclusion

This is a high-value, low-risk UX enhancement. The core functionality is solid; we're adding polish that makes daily usage more pleasant. The changes are localized to output formatting and don't touch core indexing logic.

**Key insight**: Modern CLI UX is about **just-enough information, just-in-time**. Not silent (anxiety-inducing), not verbose (attention-demanding), but informative and calm.

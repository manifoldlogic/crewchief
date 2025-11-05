# Architecture: Maproom Progress UX Enhancement

## Design Philosophy

**Principle**: Enhance output formatting without touching core indexing logic.

This is purely a presentation layer improvement. All changes are additive and non-breaking. The indexing engine, database operations, and embedding generation remain untouched. We're adding telemetry collection and formatted output.

## System Context

### Current Architecture

```
┌─────────────────────────────────────────┐
│  User invokes: maproom scan|watch       │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Node.js CLI Wrapper                    │
│  (packages/maproom-mcp/bin/cli.cjs)     │
│  - Parse args                           │
│  - Set environment                      │
│  - Spawn Rust binary                    │
│  - Stream stdout/stderr                 │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Rust Binary (maproom)                  │
│  (crates/maproom/src/main.rs)           │
│  - Parse CLI args                       │
│  - Initialize database                  │
│  - Call indexer functions               │
│  - Print output via println!            │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Indexer Module                         │
│  (crates/maproom/src/indexer/mod.rs)    │
│  - scan_worktree()                      │
│  - scan_worktree_parallel()             │
│  - watch_worktree()                     │
│  - Core indexing logic                  │
└─────────────────────────────────────────┘
```

### Enhanced Architecture

```
┌─────────────────────────────────────────┐
│  User invokes: maproom scan|watch       │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Node.js CLI Wrapper                    │
│  (packages/maproom-mcp/bin/cli.cjs)     │
│  - Parse args (incl. --verbose)         │
│  - Set environment                      │
│  - Spawn Rust binary                    │
│  - Stream stdout/stderr                 │
│  - [NO CHANGES needed here]             │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Rust Binary (maproom)                  │
│  (crates/maproom/src/main.rs)           │
│  - Parse CLI args (+verbose flag)  ◄────┼─ NEW
│  - Detect TTY for output mode      ◄────┼─ NEW
│  - Initialize database                  │
│  - Call indexer with output mode   ◄────┼─ NEW
│  - Format timing prominently       ◄────┼─ NEW
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Progress Tracker (NEW)                 │
│  (crates/maproom/src/progress.rs)       │
│  - ProgressTracker struct               │
│  - update() method                      │
│  - finish() method                      │
│  - OutputMode enum (Minimal/Verbose)    │
│  - TTY detection                        │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Indexer Module (ENHANCED)              │
│  (crates/maproom/src/indexer/mod.rs)    │
│  - scan_worktree(+progress_tracker) ◄───┼─ NEW
│  - watch_worktree(+output_mode)     ◄───┼─ NEW
│  - Emit progress updates            ◄───┼─ NEW
│  - Minimal watch output             ◄───┼─ NEW
│  - Core logic UNCHANGED                 │
└─────────────────────────────────────────┘
```

## Component Design

### 1. Progress Tracker (NEW)

**Location**: `crates/maproom/src/progress.rs`

**Purpose**: Track and format scan progress updates.

**Interface**:
```rust
pub enum OutputMode {
    Minimal,  // Compact output for daily use
    Verbose,  // Detailed output for debugging
}

pub struct ProgressTracker {
    mode: OutputMode,
    is_tty: bool,
    start_time: Instant,
    total_files: Option<usize>,
    total_chunks: Option<usize>,
    processed_files: AtomicUsize,
    processed_chunks: AtomicUsize,
    last_update: Mutex<Instant>,
}

impl ProgressTracker {
    pub fn new(mode: OutputMode) -> Self;

    pub fn set_totals(&self, files: usize, chunks: Option<usize>);

    pub fn update_files(&self, count: usize);
    pub fn update_chunks(&self, count: usize);

    pub fn should_print(&self) -> bool; // Throttle updates

    pub fn print_progress(&self);
    pub fn finish(&self);
}
```

**Behavior**:
- **TTY detection**: Use `atty::is(Stream::Stdout)` to detect interactive terminal
- **Update throttling**: Only print progress every 200ms to avoid flooding output
- **Line overwriting**: If TTY, use `\r` to overwrite current line; otherwise, print new line every 10%
- **Progress calculation**: Compute percentage from processed/total, estimate ETA from current rate

**Output examples**:

*TTY mode (overwrites line)*:
```
Processing: 450/1200 files (37%) | Embeddings: 2500/6000 (42%) | ETA: 45s
```

*Non-TTY mode (periodic updates)*:
```
Progress: 10% complete (120/1200 files)
Progress: 20% complete (240/1200 files)
...
```

### 2. Indexer Module Enhancements

**File**: `crates/maproom/src/indexer/mod.rs`

**Changes to `scan_worktree()`**:

```rust
pub async fn scan_worktree(
    // ... existing params ...
    progress: Option<&ProgressTracker>,  // NEW
) -> Result<IndexStats> {
    let start = Instant::now();

    // Print start message (existing)
    println!("🔍 Scanning worktree: {} @ {}", worktree, &commit[..8]);
    println!("   Repository: {}", repo);

    // Discover files and get count
    let files = discover_files(&root)?;
    if let Some(p) = &progress {
        p.set_totals(files.len(), None);
    }

    // Process files
    for (i, file) in files.iter().enumerate() {
        process_file(file)?;

        if let Some(p) = &progress {
            p.update_files(i + 1);
            if p.should_print() {
                p.print_progress();
            }
        }
    }

    // Generate embeddings with progress
    let total_chunks = count_chunks_without_embeddings()?;
    if let Some(p) = &progress {
        p.set_totals(files.len(), Some(total_chunks));
    }

    let embedded = generate_embeddings_with_progress(&progress)?;

    // Finish progress and show timing
    if let Some(p) = &progress {
        p.finish();
    }

    let duration = start.elapsed();
    println!("\n✅ Completed in {:.1}s", duration.as_secs_f64());

    // Print summary (existing, unchanged)
    println!("   Files processed: {}", stats.files_processed);
    // ... rest of summary ...

    Ok(stats)
}
```

**Changes to `watch_worktree()`**:

```rust
pub async fn watch_worktree(
    // ... existing params ...
    output_mode: OutputMode,  // NEW
) -> Result<()> {
    println!("👀 Watching: {} (Ctrl+C to stop)", root.display());

    loop {
        // Wait for changes...
        let changed_files = rx.recv().await?;

        match output_mode {
            OutputMode::Minimal => {
                // NEW: Compact output
                println!("🔄 {} file(s) changed", changed_files.len());

                let start = Instant::now();
                print!("Indexing: ");
                std::io::stdout().flush()?;

                for file in &changed_files {
                    process_file(file)?;
                    print!(".");
                    std::io::stdout().flush()?;
                }

                let duration = start.elapsed();
                println!(" ✅ Done in {:.1}s", duration.as_secs_f64());
            }
            OutputMode::Verbose => {
                // OLD: Existing verbose output
                println!("Detected changes in {} file(s)", changed_files.len());
                println!("Re-indexing...");

                for file in &changed_files {
                    println!("  - {}", file.display());
                    process_file(file)?;
                }

                println!("Index updated");
            }
        }
    }
}
```

### 3. CLI Argument Parsing

**File**: `crates/maproom/src/main.rs`

**Changes to `Commands` enum**:

```rust
#[derive(Parser)]
enum Commands {
    Scan {
        // ... existing fields ...

        #[arg(long, help = "Show detailed output")]
        verbose: bool,  // NEW
    },

    Watch {
        // ... existing fields ...

        #[arg(long, help = "Show detailed output")]
        verbose: bool,  // NEW
    },
}
```

**Changes to command handlers**:

```rust
Commands::Scan { path, repo, worktree, commit, verbose, .. } => {
    let mode = if verbose {
        OutputMode::Verbose
    } else {
        OutputMode::Minimal
    };

    let progress = ProgressTracker::new(mode);

    indexer::scan_worktree(
        // ... existing args ...
        Some(&progress),  // NEW
    ).await?;
}

Commands::Watch { path, repo, worktree, debounce, verbose, .. } => {
    let mode = if verbose {
        OutputMode::Verbose
    } else {
        OutputMode::Minimal
    };

    indexer::watch_worktree(
        // ... existing args ...
        mode,  // NEW
    ).await?;
}
```

### 4. Embedding Progress Integration

**File**: `crates/maproom/src/main.rs` (auto-embeddings section)

**Current code** (lines 233-298) already has some progress tracking:

```rust
println!("📊 Processing embeddings in batches of {}", batch_size);
// ...
println!("   Batch {}/{}: {} chunks", batch_num, total_batches, batch.len());
```

**Enhancement**: Integrate with ProgressTracker to show unified progress.

```rust
fn generate_embeddings_with_progress(
    progress: &Option<&ProgressTracker>,
) -> Result<usize> {
    let chunks = get_chunks_without_embeddings()?;
    let total = chunks.len();

    if let Some(p) = progress {
        p.set_totals(/* files already set */, Some(total));
    }

    for (i, batch) in chunks.chunks(BATCH_SIZE).enumerate() {
        generate_batch_embeddings(batch)?;

        if let Some(p) = progress {
            p.update_chunks((i + 1) * batch.len());
            if p.should_print() {
                p.print_progress();
            }
        }
    }

    Ok(total)
}
```

## Data Flow

### Scan Command Flow

```
User runs: maproom scan
    ↓
main.rs parses args, creates ProgressTracker
    ↓
Calls scan_worktree() with progress tracker
    ↓
scan_worktree():
    1. Discover files → set_totals(file_count)
    2. For each file:
        - Process file
        - update_files(count)
        - Check should_print() → print_progress()
    3. Count chunks needing embeddings → set_totals(files, chunks)
    4. For each embedding batch:
        - Generate embeddings
        - update_chunks(count)
        - Check should_print() → print_progress()
    5. Call finish() → show timing
    6. Print summary
```

### Watch Command Flow

```
User runs: maproom watch
    ↓
main.rs parses args, determines OutputMode
    ↓
Calls watch_worktree() with output_mode
    ↓
watch_worktree() enters event loop:
    ↓
    Wait for file changes...
    ↓
    Changes detected
    ↓
    If Minimal mode:
        - Print "🔄 N files changed"
        - Start timer
        - Print "Indexing: "
        - For each file: print ".", process file
        - Print " ✅ Done in X.Xs"
    ↓
    If Verbose mode:
        - Print "Detected changes in N file(s)"
        - Print "Re-indexing..."
        - For each file: print filename, process file
        - Print "Index updated"
```

## Key Design Decisions

### Decision 1: Progress Tracker as Separate Module

**Rationale**:
- Keeps indexer module focused on indexing logic
- Makes progress tracking reusable (could use for other operations)
- Easier to test output formatting independently
- Clear separation of concerns

**Alternative considered**: Inline progress in indexer
**Why rejected**: Would clutter indexer code, harder to maintain

### Decision 2: Optional Progress Parameter

**Signature**: `scan_worktree(..., progress: Option<&ProgressTracker>)`

**Rationale**:
- Backward compatible (can pass None)
- No runtime overhead when progress not needed
- Clear that progress is optional feature, not core to indexing

**Alternative considered**: Always create progress tracker
**Why rejected**: Would force progress overhead even when not wanted (e.g., tests)

### Decision 3: OutputMode Enum vs. Boolean

**Choice**: `OutputMode::Minimal` / `OutputMode::Verbose`

**Rationale**:
- More extensible (could add `Quiet`, `Json`, etc. later)
- Self-documenting code
- Clear intent vs. `verbose: bool`

**Alternative considered**: `--quiet` and `--verbose` flags
**Why rejected**: Creates three states (normal/quiet/verbose), more complex

### Decision 4: Default to Minimal for Watch

**Choice**: Watch uses Minimal output by default, Verbose on `--verbose`

**Rationale**:
- Watch is long-running; minimize noise
- Scan is occasional; show progress by default
- Aligns with user stories (glanceable watch status)

**Alternative considered**: Keep current verbose output as default
**Why rejected**: User explicitly requested minimal output for watch

### Decision 5: TTY Detection for Line Overwriting

**Choice**: Only overwrite lines in TTY; use periodic updates otherwise

**Rationale**:
- Carriage returns (`\r`) don't work in log files
- CI environments aren't TTYs
- Prevents garbled output in non-interactive contexts

**Alternative considered**: Always overwrite or always new lines
**Why rejected**: Need different behavior for different contexts

## Performance Considerations

### Progress Update Throttling

**Problem**: Updating progress on every file would flood output.

**Solution**: Only print every 200ms via `should_print()` check.

```rust
pub fn should_print(&self) -> bool {
    let mut last = self.last_update.lock().unwrap();
    let now = Instant::now();
    if now.duration_since(*last) > Duration::from_millis(200) {
        *last = now;
        true
    } else {
        false
    }
}
```

**Impact**: Minimal overhead (atomic increment + occasional mutex lock).

### Atomic Counters

**Choice**: Use `AtomicUsize` for file/chunk counts.

**Rationale**:
- Thread-safe without mutex overhead
- Parallel indexing can update counts from multiple threads
- Fast atomic increment operations

### stdout Flushing

**Watch mode** calls `stdout().flush()` after each dot.

**Rationale**:
- Ensures dots appear immediately (satisfying real-time feedback)
- Minimal cost (syscall once per file, not per chunk)

**Alternative considered**: Buffer dots, flush at end
**Why rejected**: Loses real-time feedback benefit

## Error Handling

### Progress Tracker Failures

**Principle**: Progress formatting errors must not fail indexing.

**Implementation**:
```rust
impl ProgressTracker {
    pub fn print_progress(&self) {
        if let Err(e) = self.try_print_progress() {
            // Log error but don't propagate
            eprintln!("Warning: Progress output failed: {}", e);
        }
    }
}
```

### TTY Detection Failures

**Fallback**: If TTY detection fails, assume non-TTY (safe default).

```rust
let is_tty = atty::is(Stream::Stdout).unwrap_or(false);
```

## Testing Strategy

See `quality-strategy.md` for full details, but architecturally:

### Unit Tests
- ProgressTracker: percentage calculations, ETA estimates, throttling
- OutputMode: correct mode selection from flags

### Integration Tests
- Scan with progress: verify output format
- Watch minimal mode: verify compact output
- Watch verbose mode: verify detailed output

### Mock TTY Testing
- Use pty crate to create pseudo-terminals for testing TTY behavior
- Verify line overwriting works correctly
- Verify fallback to periodic updates in non-TTY

## Backwards Compatibility

### Existing Behavior Preserved

1. **Default paths**: Already defaults to current dir; just making it clearer
2. **Output format**: Summary statistics unchanged
3. **Exit codes**: No changes to success/failure codes
4. **CLI flags**: All existing flags still work

### New Flags

- `--verbose`: Opt-in to old verbose output
- No breaking changes to existing scripts

### Environment Variables

No changes to existing environment variable handling (e.g., `OLLAMA_HOST`).

## Migration Path

### Phase 1: Add progress tracking (non-breaking)
- Add ProgressTracker module
- Add optional progress parameter to scan_worktree
- Default to no progress (None)
- Existing behavior unchanged

### Phase 2: Enable progress by default
- Create progress tracker in main.rs
- Pass to scan_worktree
- Users see new progress output

### Phase 3: Add minimal watch mode
- Add OutputMode to watch_worktree
- Default to Minimal
- Users see new compact output

**Rollback plan**: If issues arise, set default back to Verbose mode via single-line change.

## Open Issues

### Issue 1: Progress during parallel scanning

**Question**: `scan_worktree_parallel()` uses multiple threads. How to coordinate progress updates?

**Answer**: AtomicUsize counters are thread-safe. Each thread can update independently. ProgressTracker.print_progress() is already throttled, so multiple threads won't flood output.

### Issue 2: ETA accuracy

**Question**: How accurate should ETA estimates be?

**Answer**: "Good enough" for MVP. Simple calculation: `(total - processed) * (elapsed / processed)`. Won't account for embedding generation slowdowns, but provides ballpark estimate.

**Future enhancement**: Track separate rates for file processing vs. embedding generation.

### Issue 3: Terminal width detection

**Question**: Should we detect terminal width and adjust progress format?

**Answer**: Not for MVP. Assume 80+ character width. Most modern terminals exceed this. If truncation is an issue, we can add terminal width detection later.

## Summary

This architecture enhances user experience without modifying core functionality. The key insight is **separation of concerns**: indexing logic stays pure, progress tracking is injected as an optional dependency.

**Minimal changes, maximum impact.**

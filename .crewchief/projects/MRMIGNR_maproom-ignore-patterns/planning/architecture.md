# Architecture: maproom ignore patterns

## Overview

This solution introduces a **unified ignore pattern system** that combines `.gitignore`, `.maproomignore`, and CLI overrides into a single decision point used by both scan and watch operations.

**Key architectural principle**: Reuse existing `ignore` crate infrastructure (already handling `.gitignore`) and extend it with custom `.maproomignore` support via the `OverrideBuilder` API.

## Design Decisions

### Decision 1: Use `ignore` crate's OverrideBuilder

**Context:** We need to layer `.maproomignore` patterns on top of existing `.gitignore` handling without reimplementing glob matching.

**Decision:** Use `ignore::overrides::OverrideBuilder` to add `.maproomignore` patterns as negative overrides (exclusions).

**Rationale:**
- Already in dependencies (used by WalkBuilder)
- Handles complex glob patterns correctly
- Performant (used by ripgrep)
- Composable with existing gitignore logic
- No need to fork or reimplement pattern matching

### Decision 2: Keep IgnorePatternMatcher for watch filtering

**Context:** Git status emits events for all tracked files. We can't prevent git from seeing them, but we can filter events before processing.

**Decision:** Enhance `IgnorePatternMatcher` to read `.maproomignore` and use it as a post-filter in the watch pipeline.

**Rationale:**
- Git poller can't be told to ignore specific files (it uses `git status`)
- Filtering events is fast (happens once per change, not per file during scan)
- Reuses existing tested ignore module structure
- Maintains separation of concerns (git detection vs indexing decision)

### Decision 3: Single source of truth for pattern loading

**Context:** Both scan and watch need to read `.maproomignore`. Logic should be shared to guarantee consistency.

**Decision:** Create `load_ignore_patterns()` function in `crates/maproom/src/incremental/ignore.rs` that returns structured pattern data for both code paths to consume.

**Rationale:**
- DRY principle
- Guaranteed identical parsing logic
- Single place to add new pattern sources (future: `~/.config/crewchief/maproomignore`)
- Easier to test pattern precedence

### Decision 4: Patterns are relative to repository root

**Context:** `.maproomignore` can only exist at repo root (where `scan --path` points). Watch operates per-worktree but needs consistent behavior.

**Decision:** All patterns are interpreted relative to the repository root path, matching git's `.gitignore` semantics.

**Rationale:**
- Matches user mental model from git
- Simplifies path normalization (already have root in both operations)
- No ambiguity about pattern application
- Future-proof for potential `.maproomignore` support in subdirectories

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Pattern matching | `globset` crate (via `ignore` crate) | Already used, battle-tested, fast |
| File reading | `std::fs::read_to_string` | Simple, sufficient for ignore files |
| Scan integration | `OverrideBuilder` API | Native support in WalkBuilder |
| Watch integration | Post-filter on FileEvent | GitPoller output can't be modified |
| Pattern format | Gitignore-style globs | User familiarity, well-documented |

## Component Design

### Component 1: Pattern Loader (`ignore.rs`)

**Responsibilities:**
- Read `.maproomignore` file from disk (if exists)
- Parse patterns (skip comments, blank lines)
- Combine with default patterns
- Return structured `Vec<String>` for consumers

**Interface:**
```rust
pub fn load_ignore_patterns(root: &Path) -> Result<Vec<String>> {
    let mut patterns = DEFAULT_IGNORE_PATTERNS.iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let maproomignore_path = root.join(".maproomignore");
    if maproomignore_path.exists() {
        let content = std::fs::read_to_string(&maproomignore_path)?;
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                patterns.push(line.to_string());
            }
        }
    }

    Ok(patterns)
}
```

**Changes to IgnorePatternMatcher:**
- Add `from_repository(root: &Path)` constructor
- Reads both `.gitignore` and `.maproomignore`
- Merges with defaults

### Component 2: Scan Integration (`indexer/mod.rs`)

**Current state:**
```rust
walk.git_ignore(true);  // Handles .gitignore
```

**Enhanced:**
```rust
walk.git_ignore(true);  // Still handle .gitignore

// Add .maproomignore patterns
let maproomignore_patterns = load_ignore_patterns(&root_abs)?;
let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);
for pattern in maproomignore_patterns {
    ob.add(&format!("!{}", pattern))?;  // Negative override = exclude
}
walk.overrides(ob.build()?);
```

**Note:** The `exclude` parameter in `scan_worktree()` is for programmatic use only (e.g., daemon integration). CLI flag support is deferred to Phase 2.

### Component 3: Watch Integration (`worktree_watcher.rs`)

**Current:** GitPoller emits FileEvent → event_conversion_task → IndexingEvent

**Enhanced:** GitPoller emits FileEvent → event_conversion_task (with filter) → IndexingEvent

**Integration Point:**
- File: `crates/maproom/src/incremental/worktree_watcher.rs`
- Function: `event_conversion_task()` (async task, lines 139-163)
- Location: Inside the `while let Some(file_event) = file_event_rx.recv().await` loop (line 144)

Add filter before converting to IndexingEvent:
```rust
/// Task that converts FileEvents to IndexingEvents with worktree tagging.
async fn event_conversion_task(
    worktree_id: WorktreeId,
    mut file_event_rx: mpsc::Receiver<FileEvent>,
    indexing_event_tx: mpsc::Sender<IndexingEvent>,
) {
    // Load ignore patterns once at start
    let ignore_matcher = IgnorePatternMatcher::from_repository(&repo_root)
        .expect("Failed to load ignore patterns");

    while let Some(file_event) = file_event_rx.recv().await {
        // NEW: Filter events based on .maproomignore patterns
        let path = file_event.path();
        if ignore_matcher.should_ignore(path) {
            debug!("Ignoring event for maproomignore path: {}", path.display());
            continue;
        }

        // Existing conversion logic
        let timestamp = SystemTime::now();
        let indexing_event =
            IndexingEvent::from_file_event(worktree_id.clone(), file_event, timestamp);

        if let Err(e) = indexing_event_tx.send(indexing_event).await {
            warn!("Failed to send indexing event for worktree {}: {}", worktree_id, e);
            return;
        }
    }
}
```

**Note:** Pattern loading happens once per watcher start. If `.maproomignore` changes during watch, watcher restart is required (hot-reload not supported in MVP).

## Data Flow

### Scan Operation

```
User runs: crewchief-maproom scan --path /repo --repo myrepo

1. Load patterns
   ├─ Read DEFAULT_IGNORE_PATTERNS
   ├─ Read /repo/.maproomignore (if exists)
   └─ Read CLI --exclude patterns

2. Configure WalkBuilder
   ├─ .git_ignore(true)          // .gitignore handled
   └─ .overrides(patterns)        // .maproomignore + CLI excludes

3. Walk filesystem
   └─ WalkBuilder filters paths automatically

4. Index remaining files
```

### Watch Operation

```
GitPoller polls: git status --porcelain

1. Git emits changes (respects .gitignore automatically)

2. Load ignore patterns once per watcher
   ├─ Read DEFAULT_IGNORE_PATTERNS
   └─ Read /repo/.maproomignore (if exists)

3. Filter each FileEvent
   ├─ Check if path matches .maproomignore patterns
   └─ Skip if ignored, process if not

4. Index changed file
```

## Integration Points

### With existing WalkBuilder (scan)

- Uses same `ignore` crate infrastructure
- `.gitignore` handling unchanged
- Programmatic `exclude` parameter continues to work (not user-facing in MVP)
- Precedence: .maproomignore > .gitignore > defaults

### With GitPoller (watch)

- GitPoller code unchanged (still polls git status)
- Filter added in event conversion task (worktree_watcher.rs)
- Pattern matcher loaded once at watcher start
- No performance impact (filtering is O(patterns) per event, not per file)
- Restart required if .maproomignore changes

### With IncrementalProcessor

- Processor receives pre-filtered events
- No awareness of ignore logic needed
- Clean separation of concerns

## Performance Considerations

### Scan Performance

- Pattern compilation happens once per scan
- Globset matching is highly optimized (used by ripgrep)
- No measurable impact vs current implementation

### Watch Performance

- Pattern loading: Once per watcher start (negligible)
- Per-event filtering: ~100-500ns per file (globset is fast)
- For 1000 file changes: ~0.5ms overhead (acceptable)

### Memory

- Compiled GlobSet: ~few KB per pattern
- Typical .maproomignore: 10-50 patterns = <100KB
- Negligible compared to tree-sitter parser memory

## Maintainability

### Code Clarity

- Single `load_ignore_patterns()` function as source of truth
- Both scan and watch use same patterns
- Easy to trace: "why was this file ignored?" → check patterns

### Testing

- Unit tests for pattern loading
- Unit tests for precedence
- Integration tests for scan + watch behavior
- Reuse existing test infrastructure

### Future Extensions

Easy to add:
- Global ignore file: `~/.config/crewchief/maproomignore`
- Per-worktree overrides: `.maproomignore.local`
- Environment variable: `MAPROOM_IGNORE_PATTERNS`

Pattern loading function centralizes this logic.

## Edge Cases Handled

1. **Missing .maproomignore**: Falls back to defaults (no error)
2. **Invalid glob pattern**: Returns error during scan/watch startup (fail-fast, prevents incorrect indexing)
3. **Patterns with leading slash**: Treated as relative to root (git semantics)
4. **Patterns in subdirectories**: Not supported in MVP (can add later)
5. **Empty .maproomignore**: Valid, just uses defaults
6. **Comment-only .maproomignore**: Valid, just uses defaults
7. **.maproomignore changes during watch**: Restart required (hot-reload not supported in MVP)
8. **Path normalization**: Patterns match against relative paths (using existing `normalize_to_relpath()` for consistency)

## Migration Strategy

### Backward Compatibility

- Repos without `.maproomignore`: No change in behavior
- Programmatic `exclude` parameter: Continues to work (for daemon/internal use)
- `.gitignore` handling: Completely unchanged
- No CLI changes: No breaking changes to user-facing commands

### Rollout

1. Deploy new version with `.maproomignore` support
2. Users opt-in by creating `.maproomignore` in their repos
3. No database migration needed
4. No config file changes needed
5. Existing worktrees re-index naturally on next scan/watch

### Documentation

- Update `crates/maproom/CLAUDE.md` with `.maproomignore` section
- Add example `.maproomignore` file to docs
- CLI help text for scan command mentions `.maproomignore`

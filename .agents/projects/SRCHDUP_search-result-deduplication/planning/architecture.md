# Architecture: Search Result Deduplication

## Architecture Overview

This document describes the technical design for search result deduplication in the maproom search pipeline. The solution adds a post-fusion deduplication step that groups identical chunks and selects a representative.

## High-Level Design

### Pipeline Modification

```
┌─────────────────────────────────────────────────────────────────┐
│                    Search Pipeline                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Query → Parser → Executors (FTS/Vector/Graph/Signals)          │
│                          ↓                                       │
│                   RankedResults                                  │
│                          ↓                                       │
│                  Fusion (RRF)                                    │
│                          ↓                                       │
│              ┌───────────────────────┐                          │
│              │  NEW: Deduplicator    │                          │
│              │  ───────────────────  │                          │
│              │  1. Group by identity │                          │
│              │  2. Select best rep   │                          │
│              │  3. Return unique set │                          │
│              └───────────────────────┘                          │
│                          ↓                                       │
│              FinalSearchResults (deduplicated)                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Dedup location** | After fusion | Preserves full scoring before grouping |
| **Identity key** | (relpath, symbol_name, start_line) | Available in results, sufficient precision |
| **Selection strategy** | Highest score first | Preserves ranking intent |
| **Default behavior** | Enabled | Users benefit immediately |
| **API surface** | SearchOptions flag | Non-breaking, opt-out available |

## Detailed Design

### 1. Identity Key Definition

Chunks are considered "duplicates" if they represent the same logical code unit.

```rust
/// Unique identity for a code chunk across worktrees.
/// Chunks with the same ChunkIdentity are considered duplicates.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIdentity {
    /// Relative path to the file
    pub relpath: String,
    /// Symbol name (or empty string if none)
    pub symbol_name: String,
    /// Starting line number
    pub start_line: i32,
}

impl ChunkIdentity {
    pub fn from_result(result: &ChunkSearchResult) -> Self {
        Self {
            relpath: result.relpath.clone(),
            symbol_name: result.symbol_name.clone().unwrap_or_default(),
            start_line: result.start_line,
        }
    }
}
```

### 2. Deduplication Module

New module: `crates/maproom/src/search/dedup.rs`

```rust
//! Search result deduplication.
//!
//! This module provides deduplication of search results across worktrees.
//! When the same code exists in multiple worktrees, only the highest-scoring
//! instance is returned.

use std::collections::HashMap;
use crate::search::results::ChunkSearchResult;

/// Configuration for deduplication behavior.
#[derive(Debug, Clone)]
pub struct DeduplicationConfig {
    /// Enable deduplication (default: true)
    pub enabled: bool,
    /// Selection strategy for choosing representative
    pub strategy: SelectionStrategy,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: SelectionStrategy::HighestScore,
        }
    }
}

/// Strategy for selecting the representative chunk from duplicates.
#[derive(Debug, Clone, Copy, Default)]
pub enum SelectionStrategy {
    /// Select the chunk with the highest score
    #[default]
    HighestScore,
    /// Prefer "main" worktree, then highest score
    PreferMain,
}

/// Deduplicate search results, keeping only the best representative per identity.
///
/// # Arguments
/// * `results` - Vector of search results (may contain duplicates)
/// * `config` - Deduplication configuration
///
/// # Returns
/// Vector of unique results, maintaining score order
pub fn deduplicate(
    results: Vec<ChunkSearchResult>,
    config: &DeduplicationConfig,
) -> Vec<ChunkSearchResult> {
    if !config.enabled || results.is_empty() {
        return results;
    }

    let mut groups: HashMap<ChunkIdentity, Vec<ChunkSearchResult>> = HashMap::new();

    // Group results by identity
    for result in results {
        let identity = ChunkIdentity::from_result(&result);
        groups.entry(identity).or_default().push(result);
    }

    // Select best representative from each group
    let mut deduplicated: Vec<ChunkSearchResult> = groups
        .into_values()
        .map(|mut group| select_representative(&mut group, config.strategy))
        .collect();

    // Re-sort by score (grouping may have disrupted order)
    deduplicated.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    deduplicated
}

/// Select the best representative from a group of duplicate chunks.
fn select_representative(
    group: &mut Vec<ChunkSearchResult>,
    strategy: SelectionStrategy,
) -> ChunkSearchResult {
    match strategy {
        SelectionStrategy::HighestScore => {
            group.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            group.remove(0)
        }
        SelectionStrategy::PreferMain => {
            // Future: Add worktree_name to ChunkSearchResult for this
            // For now, fall back to highest score
            group.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            group.remove(0)
        }
    }
}
```

### 3. SearchOptions Extension

Add deduplication flag to existing SearchOptions:

```rust
// In crates/maproom/src/search/results.rs

pub struct SearchOptions {
    pub repo_id: i64,
    pub worktree_id: Option<i64>,
    pub limit: usize,
    pub mode: SearchMode,
    /// Whether to deduplicate results across worktrees (default: true)
    pub deduplicate: bool,
}

impl SearchOptions {
    pub fn new(repo_id: i64, worktree_id: Option<i64>, limit: usize) -> Self {
        Self {
            repo_id,
            worktree_id,
            limit,
            mode: SearchMode::default(),
            deduplicate: true,  // Enable by default
        }
    }

    /// Disable deduplication
    pub fn without_dedup(mut self) -> Self {
        self.deduplicate = false;
        self
    }
}
```

### 4. Pipeline Integration

Modify `SearchPipeline::search()` to call deduplication:

```rust
// In crates/maproom/src/search/pipeline.rs

pub async fn search(
    &self,
    query: &str,
    options: &SearchOptions,
) -> Result<FinalSearchResults, PipelineError> {
    // ... existing query processing and fusion ...

    // Apply deduplication if enabled
    let final_results = if options.deduplicate {
        let config = DeduplicationConfig::default();
        dedup::deduplicate(fused_results, &config)
    } else {
        fused_results
    };

    Ok(FinalSearchResults::new(query.to_string(), final_results, metadata))
}
```

### 5. MCP Tool Update

The MCP `search` tool exposes deduplication via parameters:

```typescript
// In packages/maproom-mcp/src/tools/search.ts

interface SearchParams {
  query: string;
  repo?: string;
  worktree?: string;
  limit?: number;
  mode?: 'fts' | 'vector' | 'hybrid';
  deduplicate?: boolean;  // NEW: default true
}
```

## Data Flow

### Input: Raw Search Results

```
Results: [
  {chunk_id: 1, relpath: "src/auth.rs", symbol_name: "validate", score: 0.95, start_line: 10},
  {chunk_id: 2, relpath: "src/auth.rs", symbol_name: "validate", score: 0.90, start_line: 10},  // duplicate
  {chunk_id: 3, relpath: "src/auth.rs", symbol_name: "validate", score: 0.85, start_line: 10},  // duplicate
  {chunk_id: 4, relpath: "src/utils.rs", symbol_name: "helper", score: 0.80, start_line: 5},
]
```

### Processing: Group by Identity

```
Groups: {
  ("src/auth.rs", "validate", 10): [
    {chunk_id: 1, score: 0.95},
    {chunk_id: 2, score: 0.90},
    {chunk_id: 3, score: 0.85},
  ],
  ("src/utils.rs", "helper", 5): [
    {chunk_id: 4, score: 0.80},
  ],
}
```

### Output: Deduplicated Results

```
Results: [
  {chunk_id: 1, relpath: "src/auth.rs", symbol_name: "validate", score: 0.95, start_line: 10},
  {chunk_id: 4, relpath: "src/utils.rs", symbol_name: "helper", score: 0.80, start_line: 5},
]
```

## Performance Considerations

### Time Complexity

- Grouping: O(n) where n = number of results
- Selection: O(k log k) where k = group size (typically small)
- Re-sorting: O(m log m) where m = unique groups
- **Total: O(n log n)** - dominated by re-sorting

### Memory Overhead

- HashMap for grouping: O(n) temporary
- Groups storage: O(n) temporary
- Final vector: O(m) where m ≤ n

### Benchmarking Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Latency increase | <10ms for 1000 results | Negligible UX impact |
| Memory overhead | <1MB for 1000 results | Well within typical usage |
| Duplicate reduction | >80% in polluted indexes | Significant improvement |

## Future Enhancements (Out of Scope)

### 1. Worktree Priority Selection

Add `worktree_name` to ChunkSearchResult to enable "prefer main" strategy:

```rust
pub struct ChunkSearchResult {
    // ... existing fields ...
    pub worktree_name: Option<String>,  // Future addition
}
```

### 2. Content-Based Identity

Use `blob_sha` for exact content matching (already available in database):

```rust
pub struct ChunkIdentity {
    pub blob_sha: String,  // Content hash - exact match
}
```

### 3. Fuzzy Deduplication

Group by (relpath, symbol_name) only, ignoring start_line for files with minor edits.

### 4. Duplicate Count Metadata

Track how many duplicates were collapsed:

```rust
pub struct ChunkSearchResult {
    // ... existing fields ...
    pub duplicate_count: usize,  // Number of collapsed duplicates
}
```

## Module Structure

```
crates/maproom/src/search/
├── mod.rs              # Add: pub mod dedup;
├── dedup.rs            # NEW: Deduplication logic
├── results.rs          # Modify: SearchOptions.deduplicate
├── pipeline.rs         # Modify: Call dedup in search()
└── ...existing files...
```

## API Contract

### Rust API

```rust
// Enable deduplication (default)
let options = SearchOptions::new(repo_id, worktree_id, 10);

// Disable deduplication
let options = SearchOptions::new(repo_id, worktree_id, 10).without_dedup();

// Results are automatically deduplicated
let results = pipeline.search("validate_provider", &options).await?;
```

### MCP API

```json
{
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {
      "query": "validate_provider",
      "repo": "crewchief",
      "deduplicate": true
    }
  }
}
```

## Dependencies

### Internal Dependencies
- `crate::search::results::ChunkSearchResult` - Result type
- `crate::search::pipeline::SearchPipeline` - Integration point

### External Dependencies
- None required (uses standard HashMap)

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Over-deduplication hides legitimate variants | Users miss important variations | Make configurable, default to score-based selection |
| Line number drift causes false negatives | Near-duplicates not grouped | Future: add fuzzy line matching option |
| Performance regression | Slower search | Benchmark, optimize HashMap usage |
| Breaking change to result counts | Downstream code expects N results | Document, use feature flag initially |

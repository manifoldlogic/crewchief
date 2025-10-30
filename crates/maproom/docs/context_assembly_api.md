# Context Assembly API Reference

Version: 1.0.0
Last Updated: 2025-10-24

## Overview

The Context Assembly system provides intelligent code context gathering for AI assistants. It retrieves relevant code chunks based on relationships (callers, callees, tests) while respecting token budgets and maintaining quality standards.

## Core Types

### `ContextAssembler`

The main trait for context assembly implementations.

```rust
pub trait ContextAssembler {
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        options: ExpandOptions,
    ) -> Result<ContextBundle>;
}
```

**Implementations:**
- `BasicContextAssembler` - Sequential assembly (simpler, slower)
- `ParallelContextAssembler` - Concurrent assembly (recommended for production)

### `ExpandOptions`

Configuration for relationship expansion.

```rust
pub struct ExpandOptions {
    pub tests: bool,        // Include test chunks (default: true)
    pub callers: bool,      // Include calling chunks (default: true)
    pub callees: bool,      // Include called chunks (default: true)
    pub docs: bool,         // Include documentation (default: false)
    pub config: bool,       // Include config files (default: false)
    pub max_depth: usize,   // Max relationship depth (default: 2)
}
```

**Usage Example:**
```rust
let options = ExpandOptions {
    tests: true,
    callers: true,
    callees: false,  // Exclude callees
    max_depth: 1,    // Only direct relationships
    ..Default::default()
};
```

### `ContextBundle`

The assembled context result.

```rust
pub struct ContextBundle {
    pub items: Vec<ContextItem>,     // Assembled context items
    pub total_tokens: usize,         // Tokens consumed
    pub budget_tokens: usize,        // Original budget
    pub budget_remaining: usize,     // Tokens left
    pub truncated: bool,             // Whether truncation occurred
    pub metadata: BundleMetadata,    // Assembly metadata
    pub warnings: Option<Vec<String>>, // Any warnings
}
```

### `ContextItem`

A single piece of context (code chunk).

```rust
pub struct ContextItem {
    pub relpath: String,        // File path relative to repo root
    pub range: LineRange,       // Start/end line numbers
    pub role: ItemRole,         // primary | test | caller | callee
    pub reason: String,         // Why this item was included
    pub content: String,        // Actual source code
    pub tokens: usize,          // Token count for this item
    pub symbol_name: Option<String>,  // Function/class name
    pub kind: Option<String>,   // Symbol kind (function, class, etc.)
}
```

### `ItemRole`

Describes why a context item was included.

```rust
pub enum ItemRole {
    Primary,   // The target chunk requested
    Test,      // Test for the primary chunk
    Caller,    // Chunk that calls the primary
    Callee,    // Chunk called by the primary
    Import,    // Imported by the primary
    Export,    // Exported by the primary
    Related,   // Other relationship
}
```

## Assembler Implementations

### BasicContextAssembler

Sequential assembly implementation.

```rust
pub struct BasicContextAssembler {
    pool: PgPool,
    file_loader: FileLoader,
}

impl BasicContextAssembler {
    pub fn new(pool: PgPool, cache: ContextCache) -> Self { ... }
    pub fn new_without_cache(pool: PgPool) -> Self { ... }
}
```

**When to use:**
- Testing and development
- Simple single-threaded applications
- When you need predictable, deterministic behavior

**Example:**
```rust
use crewchief_maproom::context::BasicContextAssembler;
use crewchief_maproom::db::create_pool;

let pool = create_pool().await?;
let assembler = BasicContextAssembler::new_without_cache(pool);

let bundle = assembler.assemble(chunk_id, 6000, ExpandOptions::default()).await?;
println!("Assembled {} items using {} tokens",
    bundle.items.len(),
    bundle.total_tokens
);
```

### ParallelContextAssembler

Concurrent assembly implementation (recommended).

```rust
pub struct ParallelContextAssembler {
    pool: PgPool,
    file_loader: FileLoader,
    cache: ContextCache,
}

impl ParallelContextAssembler {
    pub fn new(pool: PgPool, cache_config: CacheConfig) -> Self { ... }
}
```

**Performance:**
- 80% faster than BasicContextAssembler (5x speedup)
- p95 latency: ~8ms (vs 40ms sequential)
- Concurrent database queries and file I/O
- Thread-safe budget management

**When to use:**
- Production environments
- High-throughput applications
- When performance is critical

**Example:**
```rust
use crewchief_maproom::context::{ParallelContextAssembler, ExpandOptions};
use crewchief_maproom::context::cache::CacheConfig;
use crewchief_maproom::db::create_pool;

let pool = create_pool().await?;
let cache_config = CacheConfig::default();
let assembler = ParallelContextAssembler::new(pool, cache_config);

let options = ExpandOptions {
    tests: true,
    callers: true,
    callees: true,
    max_depth: 2,
    ..Default::default()
};

let bundle = assembler.assemble(chunk_id, 6000, options).await?;
```

## Budget Management

### TokenBudgetManager

Manages token budget allocation across categories.

```rust
pub struct TokenBudgetManager {
    total_budget: usize,
    allocations: HashMap<String, usize>,
    reserved: HashMap<String, usize>,
}

impl TokenBudgetManager {
    pub fn new(total_budget: usize, allocations: BudgetAllocations) -> Self { ... }
    pub fn reserve(&mut self, category: &str, tokens: usize) -> bool { ... }
    pub fn release(&mut self, category: &str, tokens: usize) { ... }
    pub fn remaining(&self, category: &str) -> usize { ... }
}
```

**Default Allocations:**
- Primary chunk: 40% of total budget
- Tests: 30% of total budget
- Callers: 15% of total budget
- Callees: 15% of total budget

**Example:**
```rust
use crewchief_maproom::context::budget::{TokenBudgetManager, BudgetAllocations};

let allocations = BudgetAllocations {
    primary: 0.40,
    tests: 0.30,
    callers: 0.15,
    callees: 0.15,
};

let mut budget = TokenBudgetManager::new(6000, allocations);

// Try to reserve tokens for a test chunk
if budget.reserve("tests", 500) {
    // Tokens reserved successfully
    println!("Tests budget remaining: {}", budget.remaining("tests"));
} else {
    println!("Insufficient budget for test chunk");
}
```

### SharedBudgetManager

Thread-safe budget manager for concurrent assembly.

```rust
pub struct SharedBudgetManager {
    inner: Arc<Mutex<TokenBudgetManager>>,
}

impl SharedBudgetManager {
    pub fn new(total_budget: usize, allocations: BudgetAllocations) -> Self { ... }
    pub fn reserve(&self, category: &str, tokens: usize) -> bool { ... }
    pub fn release(&self, category: &str, tokens: usize) { ... }
}
```

**Usage in parallel contexts:**
```rust
use crewchief_maproom::context::budget::SharedBudgetManager;
use tokio::task::JoinSet;

let budget = SharedBudgetManager::new(6000, BudgetAllocations::default());
let mut tasks = JoinSet::new();

// Spawn concurrent tasks that share the budget
for chunk in related_chunks {
    let budget_clone = budget.clone();
    tasks.spawn(async move {
        if budget_clone.reserve("tests", chunk.tokens) {
            // Process chunk
        }
    });
}
```

## Caching

### ContextCache

PostgreSQL-backed cache for assembled bundles.

```rust
pub struct ContextCache {
    pool: PgPool,
    stats: Arc<CacheStats>,
    config: CacheConfig,
}

impl ContextCache {
    pub fn new(pool: PgPool, config: CacheConfig) -> Self { ... }
    pub async fn get(&self, chunk_id: i64, options: &ExpandOptions)
        -> Result<Option<ContextBundle>> { ... }
    pub async fn put(&self, chunk_id: i64, options: &ExpandOptions, bundle: &ContextBundle)
        -> Result<()> { ... }
    pub async fn invalidate(&self, chunk_id: i64) -> Result<()> { ... }
    pub fn stats(&self) -> CacheStats { ... }
}
```

**Cache Configuration:**
```rust
pub struct CacheConfig {
    pub enabled: bool,       // Enable/disable caching (default: true)
    pub ttl_seconds: i64,    // Time-to-live (default: 3600)
    pub max_entries: i64,    // Max cache entries (default: 1000)
    pub hit_rate_target: f64, // Target hit rate (default: 0.60)
}
```

**Example:**
```rust
use crewchief_maproom::context::cache::{ContextCache, CacheConfig};

let config = CacheConfig {
    enabled: true,
    ttl_seconds: 7200,  // 2 hours
    max_entries: 5000,
    hit_rate_target: 0.70,
};

let cache = ContextCache::new(pool, config);

// Try cache first
if let Some(bundle) = cache.get(chunk_id, &options).await? {
    println!("Cache hit! Stats: {:?}", cache.stats());
    return Ok(bundle);
}

// Assemble and cache
let bundle = assemble_context(...).await?;
cache.put(chunk_id, &options, &bundle).await?;
```

## Graph Traversal

### Relationship Queries

Load related chunks from the database.

```rust
pub async fn load_relationships_parallel(
    pool: &PgPool,
    chunk_id: i64,
    options: &ExpandOptions,
) -> Result<RelatedChunks> { ... }

pub struct RelatedChunks {
    pub tests: Vec<ChunkMetadata>,
    pub callers: Vec<ChunkMetadata>,
    pub callees: Vec<ChunkMetadata>,
}
```

**Example:**
```rust
use crewchief_maproom::context::graph::load_relationships_parallel;

let related = load_relationships_parallel(&pool, chunk_id, &options).await?;

println!("Found {} tests", related.tests.len());
println!("Found {} callers", related.callers.len());
println!("Found {} callees", related.callees.len());
```

## Error Handling

All assembly functions return `Result<T, anyhow::Error>`.

**Common Error Scenarios:**
- **Chunk not found**: chunk_id doesn't exist in database
- **File not found**: Source file no longer exists in worktree
- **Database error**: Connection or query failures
- **Budget exceeded**: Primary chunk alone exceeds budget

**Example Error Handling:**
```rust
match assembler.assemble(chunk_id, budget, options).await {
    Ok(bundle) => {
        if bundle.truncated {
            println!("Warning: Context was truncated due to budget");
        }
        if let Some(warnings) = &bundle.warnings {
            for warning in warnings {
                eprintln!("Warning: {}", warning);
            }
        }
        Ok(bundle)
    }
    Err(e) => {
        if e.to_string().contains("Chunk not found") {
            eprintln!("Invalid chunk_id: {}", chunk_id);
        } else if e.to_string().contains("File not found") {
            eprintln!("Source file missing, may need re-indexing");
        }
        Err(e)
    }
}
```

## Performance Considerations

**Token Counting:**
- Uses simple estimation: ~4 characters per token
- Fast but approximate (not tiktoken-accurate)
- Good enough for budget management

**Recommendations:**
1. Use `ParallelContextAssembler` for production (5x faster)
2. Enable caching for frequently requested chunks
3. Set appropriate `max_depth` (default 2 is good for most cases)
4. Monitor budget usage via `bundle.total_tokens`
5. Handle `bundle.truncated` flag gracefully

**Memory Usage:**
- Assemblers are lightweight (few MB)
- Cache uses PostgreSQL storage (JSONB)
- File content is streamed, not held in memory
- Concurrent operations use tokio tasks (efficient)

## See Also

- [Configuration Guide](context_configuration.md) - Budget and strategy configuration
- [Custom Strategies](custom_strategies.md) - Implementing language-specific strategies
- [Performance Tuning](context_performance_tuning.md) - Optimization techniques
- [Usage Examples](../examples/context_usage.rs) - Runnable code examples

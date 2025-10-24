# Context Assembly Configuration Guide

Version: 1.0.0
Last Updated: 2025-10-24

## Overview

This guide covers configuration options for the Context Assembly system, including budget allocation, caching strategies, and relationship expansion settings.

## Budget Configuration

### Default Budget Allocations

The system allocates tokens across categories to ensure balanced context:

```rust
pub struct BudgetAllocations {
    pub primary: f64,   // 40% - The requested chunk
    pub tests: f64,     // 30% - Test chunks
    pub callers: f64,   // 15% - Chunks that call this
    pub callees: f64,   // 15% - Chunks called by this
}

impl Default for BudgetAllocations {
    fn default() -> Self {
        Self {
            primary: 0.40,
            tests: 0.30,
            callers: 0.15,
            callees: 0.15,
        }
    }
}
```

**Rationale:**
- Primary chunk gets largest allocation (it's what the user requested)
- Tests are high priority (help understand usage and expectations)
- Call relationships split equally (both directions important)

### Custom Budget Allocations

Adjust allocations based on your use case:

```rust
use crewchief_maproom::context::budget::BudgetAllocations;

// Test-heavy configuration (for test-driven development)
let test_focused = BudgetAllocations {
    primary: 0.30,
    tests: 0.50,    // Give tests 50% of budget
    callers: 0.10,
    callees: 0.10,
};

// Call-graph focused (for understanding dependencies)
let dependency_focused = BudgetAllocations {
    primary: 0.30,
    tests: 0.10,
    callers: 0.30,  // Emphasize who calls this
    callees: 0.30,  // And what it calls
};

// Primary-only (minimal context)
let minimal = BudgetAllocations {
    primary: 1.0,   // 100% to primary chunk
    tests: 0.0,
    callers: 0.0,
    callees: 0.0,
};
```

### Budget Sizes

**Recommended Budget Ranges:**
- **Minimal**: 1,000 tokens - Primary chunk only
- **Standard**: 6,000 tokens (default) - Primary + key relationships
- **Comprehensive**: 12,000 tokens - Extensive context
- **Maximum**: 20,000 tokens - Deep relationship traversal

**Example Configurations:**
```rust
// Quick context for autocomplete
let quick_budget = 2000;
let quick_options = ExpandOptions {
    tests: false,
    callers: false,
    callees: true,   // Only what it calls
    max_depth: 1,
    ..Default::default()
};

// Standard context for code understanding
let standard_budget = 6000;
let standard_options = ExpandOptions::default();

// Deep analysis context
let deep_budget = 15000;
let deep_options = ExpandOptions {
    tests: true,
    callers: true,
    callees: true,
    docs: true,
    config: true,
    max_depth: 3,   // Three levels deep
};
```

## Expand Options Configuration

### Basic Relationship Types

```rust
pub struct ExpandOptions {
    pub tests: bool,        // Include test chunks
    pub callers: bool,      // Include calling chunks
    pub callees: bool,      // Include called chunks
    pub docs: bool,         // Include documentation
    pub config: bool,       // Include config files
    pub max_depth: usize,   // Traversal depth limit
}
```

### Common Configurations

**Test-Driven Development:**
```rust
let tdd_options = ExpandOptions {
    tests: true,      // Essential for TDD
    callers: false,   // Less relevant
    callees: true,    // See what code uses
    docs: false,
    config: false,
    max_depth: 1,
};
```

**API Understanding:**
```rust
let api_options = ExpandOptions {
    tests: true,      // See usage examples
    callers: true,    // Who uses this API
    callees: false,   // Internal implementation less relevant
    docs: true,       // Documentation important
    config: false,
    max_depth: 2,
};
```

**Bug Investigation:**
```rust
let debug_options = ExpandOptions {
    tests: true,      // Failing tests
    callers: true,    // Where it's called from
    callees: true,    // What it depends on
    docs: false,
    config: true,     // Config may affect behavior
    max_depth: 2,
};
```

**Refactoring:**
```rust
let refactor_options = ExpandOptions {
    tests: true,      // Must update tests
    callers: true,    // All callers need review
    callees: true,    // Dependencies may change
    docs: true,       // Docs need updates
    config: false,
    max_depth: 1,     // Direct relationships only
};
```

### Depth Configuration

**max_depth** controls how many relationship "hops" to traverse:

- **depth = 1**: Direct relationships only
  - Tests directly testing this chunk
  - Functions directly calling this chunk
  - Functions directly called by this chunk

- **depth = 2** (default): One level deeper
  - Tests + tests of callees
  - Callers + callers of callers
  - Callees + callees of callees

- **depth = 3+**: Multi-level traversal
  - Useful for understanding complex dependency chains
  - Can quickly consume budget
  - May include less relevant chunks

**Depth Selection Guide:**
```
Depth 1: Quick context, autocomplete, simple questions
Depth 2: Standard context, understanding, moderate questions (RECOMMENDED)
Depth 3: Deep analysis, architecture understanding, complex refactoring
Depth 4+: Rare, architectural analysis only (expensive)
```

## Cache Configuration

### CacheConfig

```rust
pub struct CacheConfig {
    pub enabled: bool,       // Enable/disable caching
    pub ttl_seconds: i64,    // Time to live
    pub max_entries: i64,    // Maximum cache entries
    pub hit_rate_target: f64, // Target cache hit rate
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 3600,     // 1 hour
            max_entries: 1000,
            hit_rate_target: 0.60, // 60% hit rate goal
        }
    }
}
```

### Environment-Specific Configurations

**Development:**
```rust
let dev_cache = CacheConfig {
    enabled: true,
    ttl_seconds: 300,      // 5 minutes (code changes frequently)
    max_entries: 500,      // Smaller cache
    hit_rate_target: 0.40, // Lower expectation
};
```

**Production:**
```rust
let prod_cache = CacheConfig {
    enabled: true,
    ttl_seconds: 7200,     // 2 hours (stable code)
    max_entries: 5000,     // Larger cache
    hit_rate_target: 0.75, // Higher expectation
};
```

**CI/CD:**
```rust
let ci_cache = CacheConfig {
    enabled: false,        // Disable for clean builds
    ..Default::default()
};
```

**Read-Heavy Workload:**
```rust
let read_heavy_cache = CacheConfig {
    enabled: true,
    ttl_seconds: 14400,    // 4 hours (very stable)
    max_entries: 10000,    // Large cache
    hit_rate_target: 0.85, // Very high hit rate
};
```

### Cache Monitoring

Monitor cache performance to tune configuration:

```rust
let stats = cache.stats();
println!("Cache hits: {}", stats.hits);
println!("Cache misses: {}", stats.misses);
println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("Evictions: {}", stats.evictions);

// Adjust TTL if hit rate too low
if stats.hit_rate() < cache_config.hit_rate_target {
    eprintln!("Cache hit rate below target, consider increasing TTL");
}
```

## Database Configuration

### Connection Pool

Configure PostgreSQL connection pool for optimal performance:

```rust
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(10)      // Concurrent assemblers
    .min_connections(2)       // Keep warm connections
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(300))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

**Tuning Guidelines:**
- `max_connections`: 2-5x number of concurrent assemblers
- `min_connections`: 1-2 for low latency warm starts
- `acquire_timeout`: 3-5 seconds reasonable
- `idle_timeout`: 5-10 minutes for connection recycling
- `max_lifetime`: 30-60 minutes to prevent stale connections

## Complete Configuration Example

### Application-Level Configuration

```rust
use crewchief_maproom::context::{
    ParallelContextAssembler,
    ExpandOptions,
};
use crewchief_maproom::context::budget::BudgetAllocations;
use crewchief_maproom::context::cache::CacheConfig;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ContextAssemblyConfig {
    pub default_budget: usize,
    pub budget_allocations: BudgetAllocations,
    pub expand_options: ExpandOptions,
    pub cache_config: CacheConfig,
}

impl Default for ContextAssemblyConfig {
    fn default() -> Self {
        Self {
            default_budget: 6000,
            budget_allocations: BudgetAllocations::default(),
            expand_options: ExpandOptions::default(),
            cache_config: CacheConfig::default(),
        }
    }
}

impl ContextAssemblyConfig {
    pub fn development() -> Self {
        Self {
            default_budget: 4000,
            budget_allocations: BudgetAllocations::default(),
            expand_options: ExpandOptions {
                max_depth: 1,  // Shallow for speed
                ..Default::default()
            },
            cache_config: CacheConfig {
                ttl_seconds: 300,  // 5 min
                max_entries: 500,
                ..Default::default()
            },
        }
    }

    pub fn production() -> Self {
        Self {
            default_budget: 8000,
            budget_allocations: BudgetAllocations::default(),
            expand_options: ExpandOptions::default(),
            cache_config: CacheConfig {
                ttl_seconds: 7200,  // 2 hours
                max_entries: 5000,
                hit_rate_target: 0.75,
                ..Default::default()
            },
        }
    }

    pub async fn create_assembler(&self) -> Result<ParallelContextAssembler> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&std::env::var("DATABASE_URL")?)
            .await?;

        Ok(ParallelContextAssembler::new(
            pool,
            self.cache_config.clone(),
        ))
    }
}
```

### Usage in Application

```rust
// Load configuration
let config = if cfg!(debug_assertions) {
    ContextAssemblyConfig::development()
} else {
    ContextAssemblyConfig::production()
};

// Create assembler
let assembler = config.create_assembler().await?;

// Assemble context with configured defaults
let bundle = assembler.assemble(
    chunk_id,
    config.default_budget,
    config.expand_options.clone(),
).await?;
```

## Environment Variables

Recommended environment variables for configuration:

```bash
# Required
DATABASE_URL=postgresql://user:pass@localhost:5432/maproom

# Optional tuning
CONTEXT_CACHE_ENABLED=true
CONTEXT_CACHE_TTL=3600
CONTEXT_CACHE_MAX_ENTRIES=1000
CONTEXT_DEFAULT_BUDGET=6000
CONTEXT_MAX_DEPTH=2
```

## Configuration Best Practices

1. **Start with defaults** - They're well-tuned for most use cases
2. **Monitor cache hit rates** - Adjust TTL and max_entries accordingly
3. **Use depth=2** for most cases - Good balance of context vs. budget
4. **Adjust budget allocations** based on your workflow (TDD, debugging, etc.)
5. **Enable caching in production** - Significant performance improvement
6. **Disable caching in CI/CD** - Ensure clean, reproducible builds
7. **Profile before optimizing** - Measure actual performance impact
8. **Test configuration changes** - Verify behavior with representative queries

## See Also

- [API Reference](context_assembly_api.md) - Core types and functions
- [Performance Tuning](context_performance_tuning.md) - Optimization techniques
- [Custom Strategies](custom_strategies.md) - Language-specific behavior

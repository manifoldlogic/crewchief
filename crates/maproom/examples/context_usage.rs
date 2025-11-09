//! Context Assembly Usage Examples
//!
//! This file demonstrates common usage patterns for the Context Assembly system.
//!
//! NOTE: This is a reference example showing the intended API design.
//! Some features shown here represent the ideal interface and may not be
//! fully implemented yet. Refer to actual implementation in src/context/ for
//! current working code.
//!
//! Run with: cargo run --example context_usage

use anyhow::Result;
use crewchief_maproom::context::budget::BudgetAllocations;
use crewchief_maproom::context::cache::CacheConfig;
use crewchief_maproom::context::{BasicContextAssembler, ExpandOptions, ParallelContextAssembler};
use crewchief_maproom::db::create_pool;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    println!("Context Assembly Usage Examples\n");
    println!("================================\n");

    // Create database connection pool
    let pool = create_pool().await?;

    // Example 1: Basic assembly with defaults
    println!("Example 1: Basic Context Assembly");
    println!("----------------------------------");
    basic_assembly(&pool).await?;
    println!();

    // Example 2: Parallel assembly (recommended for production)
    println!("Example 2: Parallel Assembly (Production)");
    println!("------------------------------------------");
    parallel_assembly(&pool).await?;
    println!();

    // Example 3: Custom expand options
    println!("Example 3: Custom Expand Options");
    println!("---------------------------------");
    custom_expand_options(&pool).await?;
    println!();

    // Example 4: Budget configuration
    println!("Example 4: Budget Configuration");
    println!("-------------------------------");
    budget_configuration(&pool).await?;
    println!();

    // Example 5: Caching configuration
    println!("Example 5: Caching Configuration");
    println!("--------------------------------");
    caching_configuration(&pool).await?;
    println!();

    // Example 6: Error handling
    println!("Example 6: Error Handling");
    println!("-------------------------");
    error_handling(&pool).await?;
    println!();

    // Example 7: Working with results
    println!("Example 7: Working with Results");
    println!("-------------------------------");
    working_with_results(&pool).await?;
    println!();

    Ok(())
}

/// Example 1: Basic assembly with default settings
async fn basic_assembly(pool: &PgPool) -> Result<()> {
    use crewchief_maproom::context::BasicContextAssembler;

    // Create assembler (simple, sequential)
    let assembler = BasicContextAssembler::new_without_cache(pool.clone());

    // Assemble context for chunk ID 1 with default options
    let chunk_id = 1;
    let budget = 6000; // Default budget
    let options = ExpandOptions::default();

    match assembler.assemble(chunk_id, budget, options).await {
        Ok(bundle) => {
            println!("✓ Assembly successful!");
            println!("  Items: {}", bundle.items.len());
            println!(
                "  Tokens used: {}/{}",
                bundle.total_tokens, bundle.budget_tokens
            );
            println!("  Truncated: {}", bundle.truncated);

            // Show items
            for (i, item) in bundle.items.iter().enumerate() {
                println!(
                    "  {}. {} ({:?}) - {} tokens",
                    i + 1,
                    item.relpath,
                    item.role,
                    item.tokens
                );
            }
        }
        Err(e) => {
            println!("✗ Assembly failed: {}", e);
        }
    }

    Ok(())
}

/// Example 2: Parallel assembly (5x faster)
async fn parallel_assembly(pool: &PgPool) -> Result<()> {
    use crewchief_maproom::context::cache::CacheConfig;
    use crewchief_maproom::context::ParallelContextAssembler;
    use std::time::Instant;

    // Create parallel assembler with caching
    let cache_config = CacheConfig::default();
    let assembler = ParallelContextAssembler::new(pool.clone(), cache_config);

    // Measure assembly time
    let chunk_id = 1;
    let budget = 6000;
    let options = ExpandOptions::default();

    let start = Instant::now();
    let bundle = assembler.assemble(chunk_id, budget, options).await?;
    let duration = start.elapsed();

    println!("✓ Parallel assembly completed in {:?}", duration);
    println!("  Items: {}", bundle.items.len());
    println!("  Tokens: {}/{}", bundle.total_tokens, bundle.budget_tokens);
    println!("  p95 target: <120ms (recommended)");

    if duration.as_millis() < 120 {
        println!("  ✓ Within p95 target!");
    } else {
        println!("  ⚠ Exceeds p95 target");
    }

    Ok(())
}

/// Example 3: Custom expand options for different use cases
async fn custom_expand_options(pool: &PgPool) -> Result<()> {
    let assembler = ParallelContextAssembler::new(pool.clone(), CacheConfig::default());
    let chunk_id = 1;
    let budget = 6000;

    // Test-driven development: Focus on tests
    println!("TDD Configuration:");
    let tdd_options = ExpandOptions {
        tests: true,
        callers: false,
        callees: true,
        docs: false,
        config: false,
        max_depth: 1,
    };
    let bundle = assembler.assemble(chunk_id, budget, tdd_options).await?;
    println!(
        "  Items: {} (focused on tests and dependencies)",
        bundle.items.len()
    );

    // API understanding: Include documentation
    println!("\nAPI Understanding Configuration:");
    let api_options = ExpandOptions {
        tests: true,
        callers: true,
        callees: false,
        docs: true,
        config: false,
        max_depth: 2,
    };
    let bundle = assembler.assemble(chunk_id, budget, api_options).await?;
    println!(
        "  Items: {} (includes docs and callers)",
        bundle.items.len()
    );

    // Minimal context for quick lookups
    println!("\nMinimal Configuration:");
    let minimal_options = ExpandOptions {
        tests: false,
        callers: false,
        callees: false,
        docs: false,
        config: false,
        max_depth: 0,
    };
    let bundle = assembler.assemble(chunk_id, 2000, minimal_options).await?;
    println!("  Items: {} (primary chunk only)", bundle.items.len());

    Ok(())
}

/// Example 4: Budget configuration
async fn budget_configuration(pool: &PgPool) -> Result<()> {
    // Custom budget allocations
    let _test_focused = BudgetAllocations {
        primary: 0.30,
        tests: 0.50, // Give tests 50% of budget
        callers: 0.10,
        callees: 0.10,
    };

    let _dependency_focused = BudgetAllocations {
        primary: 0.30,
        tests: 0.10,
        callers: 0.30, // Emphasize call graph
        callees: 0.30,
    };

    // Use different budget sizes
    let assembler = ParallelContextAssembler::new(pool.clone(), CacheConfig::default());
    let chunk_id = 1;
    let options = ExpandOptions::default();

    // Small budget for quick context
    let small_bundle = assembler.assemble(chunk_id, 2000, options.clone()).await?;
    println!(
        "Small budget (2000 tokens): {} items",
        small_bundle.items.len()
    );

    // Standard budget
    let standard_bundle = assembler.assemble(chunk_id, 6000, options.clone()).await?;
    println!(
        "Standard budget (6000 tokens): {} items",
        standard_bundle.items.len()
    );

    // Large budget for comprehensive context
    let large_bundle = assembler.assemble(chunk_id, 15000, options).await?;
    println!(
        "Large budget (15000 tokens): {} items",
        large_bundle.items.len()
    );

    Ok(())
}

/// Example 5: Caching configuration
async fn caching_configuration(pool: &PgPool) -> Result<()> {
    use crewchief_maproom::context::cache::CacheConfig;
    use std::time::Instant;

    // Development cache (short TTL)
    let dev_cache = CacheConfig {
        enabled: true,
        ttl_seconds: 300, // 5 minutes
        max_entries: 500,
        hit_rate_target: 0.40,
    };

    let assembler = ParallelContextAssembler::new(pool.clone(), dev_cache);
    let chunk_id = 1;
    let options = ExpandOptions::default();

    // First call: cache miss
    let start = Instant::now();
    let _bundle1 = assembler.assemble(chunk_id, 6000, options.clone()).await?;
    let miss_duration = start.elapsed();
    println!("First call (cache miss): {:?}", miss_duration);

    // Second call: cache hit
    let start = Instant::now();
    let _bundle2 = assembler.assemble(chunk_id, 6000, options).await?;
    let hit_duration = start.elapsed();
    println!("Second call (cache hit): {:?}", hit_duration);

    println!(
        "Speedup: {:.1}x faster",
        miss_duration.as_secs_f64() / hit_duration.as_secs_f64()
    );

    Ok(())
}

/// Example 6: Error handling patterns
async fn error_handling(pool: &PgPool) -> Result<()> {
    let assembler = ParallelContextAssembler::new(pool.clone(), CacheConfig::default());

    // Handle missing chunk
    let invalid_chunk_id = 999999;
    match assembler
        .assemble(invalid_chunk_id, 6000, ExpandOptions::default())
        .await
    {
        Ok(_) => println!("Unexpected success"),
        Err(e) => {
            if e.to_string().contains("Chunk not found") {
                println!("✓ Correctly handled missing chunk: {}", e);
            } else {
                println!("✗ Unexpected error: {}", e);
            }
        }
    }

    // Handle budget exceeded
    let chunk_id = 1;
    let tiny_budget = 50; // Too small
    match assembler
        .assemble(chunk_id, tiny_budget, ExpandOptions::default())
        .await
    {
        Ok(bundle) => {
            if bundle.truncated {
                println!("✓ Assembly succeeded but truncated");
                println!("  Tokens: {}/{}", bundle.total_tokens, bundle.budget_tokens);
                if let Some(warnings) = &bundle.warnings {
                    for warning in warnings {
                        println!("  Warning: {}", warning);
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Assembly failed: {}", e);
        }
    }

    Ok(())
}

/// Example 7: Working with assembly results
async fn working_with_results(pool: &PgPool) -> Result<()> {
    use crewchief_maproom::context::types::ItemRole;

    let assembler = ParallelContextAssembler::new(pool.clone(), CacheConfig::default());
    let bundle = assembler
        .assemble(1, 6000, ExpandOptions::default())
        .await?;

    // Filter items by role
    let primary_items: Vec<_> = bundle
        .items
        .iter()
        .filter(|item| matches!(item.role, ItemRole::Primary))
        .collect();
    println!("Primary items: {}", primary_items.len());

    let test_items: Vec<_> = bundle
        .items
        .iter()
        .filter(|item| matches!(item.role, ItemRole::Test))
        .collect();
    println!("Test items: {}", test_items.len());

    // Find items from specific file
    let calculator_items: Vec<_> = bundle
        .items
        .iter()
        .filter(|item| item.relpath.contains("calculator"))
        .collect();
    println!("Calculator items: {}", calculator_items.len());

    // Calculate statistics
    let total_lines: usize = bundle
        .items
        .iter()
        .map(|item| item.range.end - item.range.start + 1)
        .sum();
    println!("Total lines of code: {}", total_lines);

    let avg_tokens_per_item = if !bundle.items.is_empty() {
        bundle.total_tokens / bundle.items.len()
    } else {
        0
    };
    println!("Average tokens per item: {}", avg_tokens_per_item);

    // Check budget utilization
    let budget_used_pct = (bundle.total_tokens as f64 / bundle.budget_tokens as f64) * 100.0;
    println!("Budget utilization: {:.1}%", budget_used_pct);

    if bundle.truncated {
        println!("⚠ Context was truncated - consider increasing budget");
    } else {
        println!("✓ Full context assembled within budget");
    }

    // Display metadata
    println!("\nMetadata:");
    println!("  Chunk ID: {}", bundle.metadata.chunk_id);
    println!("  Worktree: {}", bundle.metadata.worktree);
    println!("  Expand Options: {:?}", bundle.metadata.expand_options);

    Ok(())
}

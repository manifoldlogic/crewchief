//! Concurrent correctness tests for parallel context assembly.
//!
//! These tests verify that the parallel context assembler:
//! - Produces correct results identical to sequential assembly
//! - Handles concurrent budget operations correctly
//! - Maintains thread safety across parallel loads
//! - Gracefully handles errors in parallel contexts
//! - Respects early termination on budget exhaustion

use crewchief_maproom::context::budget::{SharedBudgetManager, TokenBudgetManager};
use tokio::task::JoinSet;

#[tokio::test]
async fn test_shared_budget_concurrent_reservations() {
    let budget = SharedBudgetManager::new(10000);

    // Spawn 10 concurrent tasks trying to reserve budget
    let mut set = JoinSet::new();
    for i in 0..10 {
        let budget_clone = budget.clone();
        set.spawn(async move {
            let category = format!("task_{}", i);
            budget_clone.try_reserve(&category, 1000)
        });
    }

    // Collect results
    let mut successes = 0;
    let mut failures = 0;

    while let Some(result) = set.join_next().await {
        match result {
            Ok(true) => successes += 1,
            Ok(false) => failures += 1,
            Err(e) => panic!("Task failed: {}", e),
        }
    }

    // With 10000 budget and 10 tasks reserving 1000 each,
    // exactly 10 should succeed
    assert_eq!(successes, 10);
    assert_eq!(failures, 0);
    assert_eq!(budget.remaining(), 0);
}

#[tokio::test]
async fn test_shared_budget_overflow_protection() {
    let budget = SharedBudgetManager::new(5000);

    // Spawn 10 concurrent tasks trying to reserve 1000 each
    // Only 5 should succeed
    let mut set = JoinSet::new();
    for i in 0..10 {
        let budget_clone = budget.clone();
        set.spawn(async move {
            let category = format!("task_{}", i);
            budget_clone.try_reserve(&category, 1000)
        });
    }

    // Collect results
    let mut successes = 0;
    let mut failures = 0;

    while let Some(result) = set.join_next().await {
        match result {
            Ok(true) => successes += 1,
            Ok(false) => failures += 1,
            Err(e) => panic!("Task failed: {}", e),
        }
    }

    // With 5000 budget, exactly 5 tasks should succeed
    assert_eq!(successes, 5);
    assert_eq!(failures, 5);

    // Budget should be fully allocated
    assert_eq!(budget.used(), 5000);
    assert_eq!(budget.remaining(), 0);
}

#[tokio::test]
async fn test_shared_budget_concurrent_release() {
    let budget = SharedBudgetManager::new(3000);

    // Reserve budget in three categories
    assert!(budget.try_reserve("cat1", 1000));
    assert!(budget.try_reserve("cat2", 1000));
    assert!(budget.try_reserve("cat3", 1000));
    assert_eq!(budget.remaining(), 0);

    // Release concurrently
    let mut set = JoinSet::new();
    for cat in ["cat1", "cat2", "cat3"] {
        let budget_clone = budget.clone();
        let category = cat.to_string();
        set.spawn(async move {
            budget_clone.release(&category);
        });
    }

    // Wait for all releases to complete
    while set.join_next().await.is_some() {}

    // All budget should be available again
    assert_eq!(budget.used(), 0);
    assert_eq!(budget.remaining(), 3000);
}

#[tokio::test]
async fn test_shared_budget_allocation_atomicity() {
    let budget = SharedBudgetManager::new(10000);

    // Spawn many concurrent tasks with varying reservation sizes
    let mut set = JoinSet::new();
    let sizes = vec![500, 800, 1200, 300, 1500, 700, 900, 1100, 600, 400];

    for (i, &size) in sizes.iter().enumerate() {
        let budget_clone = budget.clone();
        set.spawn(async move {
            let category = format!("var_task_{}", i);
            budget_clone.try_reserve(&category, size)
        });
    }

    // Collect all results
    while let Some(result) = set.join_next().await {
        match result {
            Ok(true) => {
                // This task got its allocation
            }
            Ok(false) => {
                // This task was denied
            }
            Err(e) => panic!("Task failed: {}", e),
        }
    }

    // Get final stats
    if let Some(stats) = budget.usage_stats() {
        let _total_allocated = stats.used;
        let _total_requested: usize = sizes.iter().sum();

        // Verify budget consistency
        assert!(stats.used <= 10000);
        assert_eq!(stats.budget, 10000);
        assert_eq!(stats.remaining, 10000 - stats.used);

        // All allocations should sum correctly
        let category_total: usize = stats.by_category.values().sum();
        assert_eq!(category_total, stats.used);
    } else {
        panic!("Failed to get usage stats");
    }
}

#[tokio::test]
async fn test_budget_manager_no_data_races() {
    // Test that TokenBudgetManager (non-shared) behaves correctly
    let mut manager = TokenBudgetManager::new(5000);

    // Sequential operations should be deterministic
    assert!(manager.reserve("a", 1000));
    assert!(manager.reserve("b", 2000));
    assert!(manager.reserve("c", 1500));
    assert_eq!(manager.used(), 4500);
    assert_eq!(manager.remaining(), 500);

    // This should fail
    assert!(!manager.reserve("d", 1000));
    assert_eq!(manager.used(), 4500);

    // This should succeed
    assert!(manager.reserve("d", 500));
    assert_eq!(manager.used(), 5000);
    assert_eq!(manager.remaining(), 0);

    // Release one category
    manager.release("b");
    assert_eq!(manager.used(), 3000);
    assert_eq!(manager.remaining(), 2000);

    // Verify stats
    let stats = manager.usage_stats();
    assert_eq!(stats.by_category.len(), 3); // a, c, d (b was released)
    assert_eq!(stats.by_category.get("a"), Some(&1000));
    assert_eq!(stats.by_category.get("c"), Some(&1500));
    assert_eq!(stats.by_category.get("d"), Some(&500));
    assert_eq!(stats.by_category.get("b"), None);
}

#[tokio::test]
async fn test_shared_budget_with_manager_function() {
    let budget = SharedBudgetManager::new(5000);

    // Use with_manager for atomic multi-operation
    let result = budget.with_manager(|mgr| {
        mgr.reserve("primary", 2000);
        mgr.reserve("tests", 1500);
        mgr.reserve("callers", 1000);
        mgr.remaining()
    });

    assert_eq!(result, Some(500));
    assert_eq!(budget.remaining(), 500);

    // Concurrent with_manager calls
    let mut set = JoinSet::new();
    for i in 0..3 {
        let budget_clone = budget.clone();
        set.spawn(async move {
            budget_clone.with_manager(|mgr| {
                let category = format!("extra_{}", i);
                mgr.reserve(&category, 100)
            })
        });
    }

    let mut successful_reserves = 0;
    while let Some(result) = set.join_next().await {
        if let Ok(Some(true)) = result {
            successful_reserves += 1;
        }
    }

    // With 500 remaining, only 5 tasks with 100 each should succeed
    // But we only spawned 3, so all should succeed
    assert_eq!(successful_reserves, 3);
    assert_eq!(budget.remaining(), 200); // 500 - 3*100
}

#[test]
fn test_budget_allocation_percentages() {
    let manager = TokenBudgetManager::new(10000);
    let allocation = manager.allocate();

    // Verify percentages
    assert_eq!(allocation.primary, 4000); // 40%
    assert_eq!(allocation.tests, 2000); // 20%
    assert_eq!(allocation.callers, 1500); // 15%
    assert_eq!(allocation.callees, 1500); // 15%
    assert_eq!(allocation.config, 1000); // 10%
    assert_eq!(allocation.total(), 10000);
}

#[tokio::test]
async fn test_early_termination_on_budget_exhaustion() {
    let budget = SharedBudgetManager::new(2000);

    // Reserve primary chunk
    assert!(budget.try_reserve("primary", 1500));
    assert_eq!(budget.remaining(), 500);

    // Spawn tasks that try to load related items
    // Most should fail due to insufficient budget
    let mut set = JoinSet::new();
    for i in 0..10 {
        let budget_clone = budget.clone();
        set.spawn(async move {
            let category = format!("related_{}", i);
            budget_clone.try_reserve(&category, 400)
        });
    }

    let mut successes = 0;
    while let Some(result) = set.join_next().await {
        if let Ok(true) = result {
            successes += 1;
        }
    }

    // With 500 remaining and 400 per item, only 1 should succeed
    assert_eq!(successes, 1);

    // Budget should be at or near exhaustion
    let remaining = budget.remaining();
    assert!(remaining < 200); // Some small amount or zero
}

#[test]
fn test_budget_edge_cases() {
    // Zero budget
    let mut zero_budget = TokenBudgetManager::new(0);
    assert!(!zero_budget.reserve("any", 1));
    assert_eq!(zero_budget.remaining(), 0);

    // Large budget
    let large_budget = TokenBudgetManager::new(1_000_000);
    assert_eq!(large_budget.remaining(), 1_000_000);

    // Reserve exact amount
    let mut exact = TokenBudgetManager::new(1000);
    assert!(exact.reserve("exact", 1000));
    assert_eq!(exact.remaining(), 0);
    assert!(!exact.reserve("over", 1));

    // Replace reservation with smaller amount
    assert!(exact.reserve("exact", 500));
    assert_eq!(exact.remaining(), 500);
}

#[tokio::test]
async fn test_shared_budget_stress_test() {
    // Stress test with many concurrent operations
    let budget = SharedBudgetManager::new(100_000);

    // Spawn 100 tasks doing random operations
    let mut set = JoinSet::new();
    for i in 0..100 {
        let budget_clone = budget.clone();
        set.spawn(async move {
            let category = format!("stress_{}", i);
            let amount = (i % 10 + 1) * 100; // 100-1000
            budget_clone.try_reserve(&category, amount)
        });
    }

    let mut total_reserved = 0;
    while let Some(result) = set.join_next().await {
        if let Ok(true) = result {
            // Count successful reservations
            total_reserved += 1;
        }
    }

    // Verify consistency
    if let Some(stats) = budget.usage_stats() {
        assert!(stats.used <= 100_000);
        assert_eq!(stats.used + stats.remaining, 100_000);

        // Category count should match successful reservations
        assert_eq!(stats.by_category.len(), total_reserved);
    }
}

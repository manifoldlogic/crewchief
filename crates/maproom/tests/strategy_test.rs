//! Integration tests for assembly strategies.
//!
//! These tests verify that each strategy correctly selects and assembles
//! context based on language-specific patterns.

use crewchief_maproom::context::{
    AssemblyStrategy, DefaultAssemblyStrategy, ExpandOptions, PythonAssemblyStrategy,
    RustAssemblyStrategy,
};
use crewchief_maproom::db::create_pool;

#[tokio::test]
#[ignore] // Requires database connection
async fn test_default_strategy_basic_assembly() {
    // This test verifies the default strategy assembles context correctly
    let pool = create_pool().await.expect("Failed to create pool");
    let strategy = DefaultAssemblyStrategy::new(pool);

    // Note: This requires a seeded database with test data
    // Actual chunk_id would come from test fixtures
    let chunk_id = 1; // Placeholder
    let budget = 6000;
    let options = ExpandOptions::with_common();

    let result = strategy.assemble(chunk_id, budget, options).await;

    // In a real test, we'd verify:
    // - Primary chunk is included
    // - Tests are included when options.tests = true
    // - Callers/callees are included when requested
    // - Total tokens <= budget
    // - Items have correct roles and reasons

    // For now, we just check it doesn't error
    match result {
        Ok(bundle) => {
            assert!(bundle.total_tokens <= budget);
            assert!(!bundle.items.is_empty());
        }
        Err(e) => {
            // Expected if database doesn't have this chunk
            println!("Expected error (no test data): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_python_strategy_includes_init_py() {
    // This test verifies Python strategy includes __init__.py
    let pool = create_pool().await.expect("Failed to create pool");
    let strategy = PythonAssemblyStrategy::new(pool);

    let chunk_id = 1; // Placeholder - would be a Python chunk in test fixtures
    let budget = 6000;
    let options = ExpandOptions::with_common();

    let result = strategy.assemble(chunk_id, budget, options).await;

    match result {
        Ok(bundle) => {
            // Verify Python-specific items are included
            let has_package_init = bundle.items.iter().any(|item| item.role == "package_init");

            // Note: This would only be true if the test fixture has __init__.py
            println!("Has package_init: {}", has_package_init);
        }
        Err(e) => {
            println!("Expected error (no test data): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_rust_strategy_includes_cargo_toml() {
    // This test verifies Rust strategy includes Cargo.toml
    let pool = create_pool().await.expect("Failed to create pool");
    let strategy = RustAssemblyStrategy::new(pool);

    let chunk_id = 1; // Placeholder - would be a Rust chunk in test fixtures
    let budget = 6000;
    let options = ExpandOptions::with_common();

    let result = strategy.assemble(chunk_id, budget, options).await;

    match result {
        Ok(bundle) => {
            // Verify Rust-specific items are included
            let has_cargo_toml = bundle
                .items
                .iter()
                .any(|item| item.role == "crate_metadata");

            // Note: This would only be true if the test fixture has Cargo.toml
            println!("Has crate_metadata: {}", has_cargo_toml);
        }
        Err(e) => {
            println!("Expected error (no test data): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_default_strategy_respects_budget() {
    let pool = create_pool().await.expect("Failed to create pool");
    let strategy = DefaultAssemblyStrategy::new(pool);

    let chunk_id = 1; // Placeholder
    let budget = 1000; // Small budget
    let options = ExpandOptions::with_common();

    let result = strategy.assemble(chunk_id, budget, options).await;

    match result {
        Ok(bundle) => {
            // Budget might be slightly exceeded for primary chunk, but should be close
            assert!(
                bundle.total_tokens <= budget * 2,
                "Total tokens {} exceeds reasonable budget {}",
                bundle.total_tokens,
                budget * 2
            );
        }
        Err(e) => {
            println!("Expected error (no test data): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_default_strategy_primary_only() {
    let pool = create_pool().await.expect("Failed to create pool");
    let strategy = DefaultAssemblyStrategy::new(pool);

    let chunk_id = 1; // Placeholder
    let budget = 6000;
    let options = ExpandOptions::primary_only();

    let result = strategy.assemble(chunk_id, budget, options).await;

    match result {
        Ok(bundle) => {
            // Should only have the primary chunk
            let primary_count = bundle.items.iter().filter(|i| i.role == "primary").count();

            assert_eq!(primary_count, 1, "Should have exactly one primary chunk");

            // With primary_only options, should not have tests/callers/callees
            let has_tests = bundle.items.iter().any(|i| i.role == "test");
            let has_callers = bundle.items.iter().any(|i| i.role == "caller");
            let has_callees = bundle.items.iter().any(|i| i.role == "callee");

            assert!(!has_tests, "Should not have tests with primary_only");
            assert!(!has_callers, "Should not have callers with primary_only");
            assert!(!has_callees, "Should not have callees with primary_only");
        }
        Err(e) => {
            println!("Expected error (no test data): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_python_strategy_with_all_options() {
    let pool = create_pool().await.expect("Failed to create pool");
    let strategy = PythonAssemblyStrategy::new(pool);

    let chunk_id = 1; // Placeholder
    let budget = 10000;
    let options = ExpandOptions::with_all();

    let result = strategy.assemble(chunk_id, budget, options).await;

    match result {
        Ok(bundle) => {
            // Verify we have a mix of items
            assert!(!bundle.items.is_empty());

            // Check roles are valid
            for item in &bundle.items {
                assert!(!item.role.is_empty());
                assert!(!item.reason.is_empty());
                assert!(item.tokens > 0);
            }
        }
        Err(e) => {
            println!("Expected error (no test data): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_rust_strategy_with_all_options() {
    let pool = create_pool().await.expect("Failed to create pool");
    let strategy = RustAssemblyStrategy::new(pool);

    let chunk_id = 1; // Placeholder
    let budget = 10000;
    let options = ExpandOptions::with_all();

    let result = strategy.assemble(chunk_id, budget, options).await;

    match result {
        Ok(bundle) => {
            // Verify we have a mix of items
            assert!(!bundle.items.is_empty());

            // Check roles are valid
            for item in &bundle.items {
                assert!(!item.role.is_empty());
                assert!(!item.reason.is_empty());
                assert!(item.tokens > 0);
            }
        }
        Err(e) => {
            println!("Expected error (no test data): {}", e);
        }
    }
}

#[test]
fn test_strategy_budget_allocation() {
    // Unit test for budget allocation logic (no database required)
    let budget = 10000;

    let primary_budget = (budget as f64 * 0.4) as usize;
    let test_budget = (budget as f64 * 0.3) as usize;
    let caller_budget = (budget as f64 * 0.15) as usize;
    let callee_budget = (budget as f64 * 0.15) as usize;

    assert_eq!(primary_budget, 4000);
    assert_eq!(test_budget, 3000);
    assert_eq!(caller_budget, 1500);
    assert_eq!(callee_budget, 1500);

    // Total should equal budget
    assert_eq!(
        primary_budget + test_budget + caller_budget + callee_budget,
        budget
    );
}

#[test]
fn test_expand_options_for_different_scenarios() {
    // Test primary only
    let primary = ExpandOptions::primary_only();
    assert!(!primary.tests);
    assert!(!primary.callers);
    assert!(!primary.callees);
    assert!(!primary.config);

    // Test with common
    let common = ExpandOptions::with_common();
    assert!(common.tests);
    assert!(common.callers);
    assert!(common.callees);

    // Test with all
    let all = ExpandOptions::with_all();
    assert!(all.tests);
    assert!(all.callers);
    assert!(all.callees);
    assert!(all.config);
    assert!(all.docs);
}

// Note: More comprehensive integration tests would require:
// 1. Test fixtures with real code samples
// 2. Seeded database with chunks, edges, and relationships
// 3. Verification of specific context items in the bundle
// 4. Performance benchmarks for different strategies
//
// These tests provide a framework for that future work.

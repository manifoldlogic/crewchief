//! MLLANG-1005.3010: Pathological case test for doc comment extraction
//!
//! Tests the worst-case performance of backward-walking doc comment extraction.
//! This test creates a file with 10,000 blank lines followed by 100 functions,
//! forcing each doc comment extraction to walk back through thousands of lines.

use crewchief_maproom::indexer::parser;
use std::time::Instant;

/// Generate pathological C source: many blank lines followed by symbols
///
/// Creates a file with `blank_lines` blank lines at the start, followed by
/// `function_count` functions. Each function has a doc comment immediately
/// before it, forcing the backward walk to traverse all the blank lines.
fn generate_pathological_source(blank_lines: usize, function_count: usize) -> String {
    let mut source = String::new();

    // Header
    source.push_str("#include <stdio.h>\n");
    source.push_str("#include <stdlib.h>\n\n");

    // Many blank lines at the start (pathological case for backward walk)
    for _ in 0..blank_lines {
        source.push('\n');
    }

    // Functions clustered at the end, each with a doc comment
    for i in 0..function_count {
        source.push_str(&format!("// Function {} documentation\n", i));
        source.push_str(&format!("int func_{}(void) {{ return {}; }}\n\n", i, i));
    }

    source
}

#[test]
fn test_pathological_10k_blank_lines() {
    // Worst case: 10,000 blank lines followed by 100 functions
    // Each doc comment extraction must walk backward through 10,000+ lines
    let source = generate_pathological_source(10_000, 100);

    let line_count = source.lines().count();
    let bytes = source.len();

    println!("Pathological test input:");
    println!("  Lines: {}", line_count);
    println!("  Bytes: {}", bytes);
    println!("  Blank lines: 10,000");
    println!("  Functions: 100");

    let start = Instant::now();
    let chunks = parser::extract_chunks(&source, "c");
    let duration = start.elapsed();

    println!("\nResults:");
    println!("  Chunks extracted: {}", chunks.len());
    println!("  Parse time: {:?}", duration);
    println!(
        "  Time per chunk: {:.2}ms",
        duration.as_secs_f64() * 1000.0 / chunks.len() as f64
    );

    // Should extract all functions
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    assert_eq!(func_count, 100, "Should extract all 100 functions");

    // Should extract all doc comments (verify backward walk succeeds)
    let funcs_with_docs = chunks
        .iter()
        .filter(|c| c.kind == "func" && c.docstring.is_some())
        .count();
    assert_eq!(
        funcs_with_docs, 100,
        "All functions should have doc comments extracted"
    );

    // Performance threshold: if this takes >500ms, optimization may be worthwhile
    // (Real-world files are <100ms, so 5x overhead indicates a problem)
    println!("\nPerformance assessment:");
    if duration.as_millis() > 2000 {
        println!("  SLOW: >2s - optimization STRONGLY recommended");
    } else if duration.as_millis() > 500 {
        println!("  MODERATE: 500ms-2s - optimization may be worthwhile");
    } else {
        println!("  FAST: <500ms - optimization likely NOT worthwhile");
    }

    // Don't fail the test on performance, just report it
    // (We want to measure, not enforce a threshold)
}

#[test]
fn test_pathological_50k_blank_lines() {
    // Even more extreme: 50,000 blank lines followed by 100 functions
    let source = generate_pathological_source(50_000, 100);

    let line_count = source.lines().count();
    let bytes = source.len();

    println!("Extreme pathological test input:");
    println!("  Lines: {}", line_count);
    println!("  Bytes: {}", bytes);
    println!("  Blank lines: 50,000");
    println!("  Functions: 100");

    let start = Instant::now();
    let chunks = parser::extract_chunks(&source, "c");
    let duration = start.elapsed();

    println!("\nResults:");
    println!("  Chunks extracted: {}", chunks.len());
    println!("  Parse time: {:?}", duration);
    println!(
        "  Time per chunk: {:.2}ms",
        duration.as_secs_f64() * 1000.0 / chunks.len() as f64
    );

    // Should extract all functions
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    assert_eq!(func_count, 100, "Should extract all 100 functions");

    // Should extract all doc comments
    let funcs_with_docs = chunks
        .iter()
        .filter(|c| c.kind == "func" && c.docstring.is_some())
        .count();
    assert_eq!(
        funcs_with_docs, 100,
        "All functions should have doc comments extracted"
    );

    // Performance threshold
    println!("\nPerformance assessment:");
    if duration.as_millis() > 5000 {
        println!("  VERY SLOW: >5s - optimization CRITICAL");
    } else if duration.as_millis() > 2000 {
        println!("  SLOW: 2s-5s - optimization recommended");
    } else if duration.as_millis() > 500 {
        println!("  MODERATE: 500ms-2s - optimization may be worthwhile");
    } else {
        println!("  FAST: <500ms - optimization likely NOT worthwhile");
    }
}

#[test]
fn test_realistic_with_scattered_comments() {
    // Realistic case: comments scattered throughout the file
    // This represents normal code where functions are distributed
    let mut source = String::new();

    source.push_str("#include <stdio.h>\n\n");

    // Generate 100 functions with some spacing between them
    for i in 0..100 {
        // Add some blank lines between functions (realistic spacing)
        for _ in 0..3 {
            source.push('\n');
        }

        source.push_str(&format!("// Function {} documentation\n", i));
        source.push_str(&format!("int func_{}(void) {{ return {}; }}\n", i, i));
    }

    let line_count = source.lines().count();

    println!("Realistic test input:");
    println!("  Lines: {}", line_count);
    println!("  Functions: 100 (scattered throughout)");

    let start = Instant::now();
    let chunks = parser::extract_chunks(&source, "c");
    let duration = start.elapsed();

    println!("\nResults:");
    println!("  Chunks extracted: {}", chunks.len());
    println!("  Parse time: {:?}", duration);

    // Should be very fast for realistic code
    assert!(
        duration.as_millis() < 500,
        "Realistic code should parse quickly: {:?}",
        duration
    );
}

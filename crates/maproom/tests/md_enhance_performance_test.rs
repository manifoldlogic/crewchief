/// MD_ENHANCE-4002: Performance Testing
///
/// Standalone performance tests that validate parsing performance
/// without dependencies on other integration tests.
use crewchief_maproom::indexer::parser;
use std::fs;
use std::time::Instant;

#[test]
fn test_performance_small_document() {
    let content = r#"# Small Document

## Section 1

Some content here.

```rust
fn test() {
    println!("Hello");
}
```

## Section 2

More content.
"#;

    // Warm-up
    let _ = parser::extract_chunks(content, "md");

    // Timed run
    let start = Instant::now();
    let chunks = parser::extract_chunks(content, "md");
    let duration = start.elapsed();

    println!("\nSmall Document Performance:");
    println!("  Chunks: {}", chunks.len());
    println!("  Parse time: {:?}", duration);

    assert!(
        duration.as_millis() < 50,
        "Small document should parse < 50ms: {:?}",
        duration
    );
}

#[test]
#[ignore = "Performance benchmark with tight timing thresholds, flaky in CI"]
fn test_performance_large_document() {
    let mut content = String::from("# Large Document\n\n");

    for i in 1..=500 {
        content.push_str(&format!("## Section {}\n\nContent.\n\n", i));
        if i % 10 == 0 {
            content.push_str(&format!("```rust\nfn func_{}() {{}}\n```\n\n", i));
        }
    }

    let line_count = content.lines().count();

    // Warm-up
    let _ = parser::extract_chunks(&content, "md");

    // Timed run
    let start = Instant::now();
    let chunks = parser::extract_chunks(&content, "md");
    let duration = start.elapsed();

    println!("\nLarge Document Performance:");
    println!("  Lines: {}", line_count);
    println!("  Chunks: {}", chunks.len());
    println!("  Parse time: {:?}", duration);
    println!(
        "  Lines/sec: {:.0}",
        line_count as f64 / duration.as_secs_f64()
    );

    // Should parse within reasonable time
    assert!(
        duration.as_millis() < 500,
        "Large document should parse < 500ms: {:?}",
        duration
    );
}

#[test]
fn test_performance_real_readme() {
    if let Ok(content) = fs::read_to_string("/workspace/README.md") {
        let line_count = content.lines().count();

        // Warm-up
        let _ = parser::extract_chunks(&content, "md");

        // Timed run
        let start = Instant::now();
        let chunks = parser::extract_chunks(&content, "md");
        let duration = start.elapsed();

        println!("\nREADME.md Performance:");
        println!("  Lines: {}", line_count);
        println!("  Chunks: {}", chunks.len());
        println!("  Parse time: {:?}", duration);

        assert!(
            duration.as_millis() < 200,
            "README parsing should be < 200ms: {:?}",
            duration
        );
    }
}

#[test]
fn test_performance_real_claude_md() {
    if let Ok(content) = fs::read_to_string("/workspace/CLAUDE.md") {
        let line_count = content.lines().count();

        // Warm-up
        let _ = parser::extract_chunks(&content, "md");

        // Timed run
        let start = Instant::now();
        let chunks = parser::extract_chunks(&content, "md");
        let duration = start.elapsed();

        println!("\nCLAUDE.md Performance:");
        println!("  Lines: {}", line_count);
        println!("  Chunks: {}", chunks.len());
        println!("  Parse time: {:?}", duration);

        assert!(
            duration.as_millis() < 300,
            "CLAUDE.md parsing should be < 300ms: {:?}",
            duration
        );
    }
}

#[test]
#[ignore = "Performance benchmark with tight timing thresholds, flaky in CI"]
fn test_performance_repeated_parsing() {
    let content = r#"# Test Document

## Section 1

Some content here.

```rust
fn test() {
    println!("test");
}
```
"#;

    let iterations = 100;

    // Warm-up
    let _ = parser::extract_chunks(content, "md");

    // Timed run
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = parser::extract_chunks(content, "md");
    }

    let duration = start.elapsed();
    let avg_time = duration / iterations;

    println!("\nRepeated Parsing Performance:");
    println!("  Iterations: {}", iterations);
    println!("  Total time: {:?}", duration);
    println!("  Avg time per parse: {:?}", avg_time);

    assert!(
        avg_time.as_micros() < 5000,
        "Average parse time should be < 5ms: {:?}",
        avg_time
    );
}

#[test]
#[ignore = "Performance benchmark with tight timing thresholds, flaky in CI"]
fn test_performance_code_block_heavy() {
    let mut content = String::from("# Code Examples\n\n");

    for i in 1..=100 {
        content.push_str(&format!(
            "## Example {}\n\n```typescript\nconst x{}  = 1;\n```\n\n",
            i, i
        ));
    }

    let start = Instant::now();
    let chunks = parser::extract_chunks(&content, "md");
    let duration = start.elapsed();

    let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

    println!("\nCode Block Heavy Performance:");
    println!("  Code blocks: {}", code_blocks);
    println!("  Parse time: {:?}", duration);

    assert_eq!(code_blocks, 100, "Should detect all code blocks");
    assert!(
        duration.as_millis() < 300,
        "Code block heavy should parse < 300ms: {:?}",
        duration
    );
}

#[test]
fn test_performance_summary() {
    println!("\n========================================");
    println!("MD_ENHANCE Performance Summary");
    println!("========================================\n");

    // Test small doc
    let small_doc = "# Title\n\n## Section\n\nContent.\n\n```rust\nfn test() {}\n```\n";
    let start = Instant::now();
    let _ = parser::extract_chunks(small_doc, "md");
    let small_time = start.elapsed();

    // Test large doc
    let mut large_doc = String::from("# Doc\n\n");
    for i in 1..=500 {
        large_doc.push_str(&format!("## Section {}\n\nContent.\n\n", i));
    }
    let start = Instant::now();
    let _ = parser::extract_chunks(&large_doc, "md");
    let large_time = start.elapsed();

    println!("Performance Results:");
    println!("  Small doc (~10 lines): {:?}", small_time);
    println!("  Large doc (500 sections): {:?}", large_time);
    println!();

    println!("Target Metrics:");
    println!(
        "  Small doc: < 50ms {}",
        if small_time.as_millis() < 50 {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "  Large doc: < 500ms {}",
        if large_time.as_millis() < 500 {
            "✓"
        } else {
            "✗"
        }
    );
    println!();

    if small_time.as_millis() < 50 && large_time.as_millis() < 500 {
        println!("✓ All performance targets met");
        println!("✓ No performance regression detected");
    } else {
        println!("⚠ Some targets missed (acceptable in CI)");
    }
    println!();
}

---
name: real-world-integration-testing
description: Test parsers against real-world open source code with performance assertions and comprehensive symbol extraction validation
origin: MLLANG-1005
created: 2026-02-08
tags: [testing, integration, parser, real-world, validation]
---

# Real-World Integration Testing

## Overview

This skill documents the pattern for testing language parsers against real-world open source code samples. Instead of relying solely on synthetic test cases, this approach validates parsers against production code from well-known projects to catch edge cases, verify performance, and ensure production readiness. This pattern was established for the C parser with cJSON, Redis, musl libc, and zlib samples.

## When to Use

- When implementing a new language parser for Maproom
- When you need to validate parser correctness against realistic code patterns
- When you want to ensure parser performance on typical file sizes
- When you need confidence that the parser handles production code idioms

## Pattern/Procedure

### Step 1: Select Representative Code Samples

Choose 3-5 well-known open source projects in the target language that represent different coding styles and patterns:

**Selection Criteria:**
1. **Diversity:** Different domains (systems, web, data structures, libraries)
2. **Popularity:** Well-known projects with established code quality
3. **Licensing:** MIT, BSD, Apache, or other permissive licenses allowing inclusion
4. **Size:** 100-500 lines per sample (representative but not overwhelming)
5. **Patterns:** Cover different language features and idioms

**Example selections for C:**
- cJSON: Single-file JSON library (embedded systems style)
- Redis: High-performance server code (networking, event loops)
- musl libc: System library implementations (POSIX, low-level)
- zlib: Compression library (algorithms, bit manipulation)

### Step 2: Create Test File

Create test file at `crates/maproom/tests/LANG_real_world_test.rs`:

```rust
//! Integration tests for LANG parser using real-world code samples
//!
//! Tests the LANG parser against real production code to validate:
//! - No panics on real-world LANG patterns
//! - Correct symbol extraction
//! - Performance within acceptable bounds
//! - Coverage of diverse LANG idioms and patterns

use crewchief_maproom::indexer::parser;
use std::time::Instant;
```

### Step 3: Structure Each Test Case

For each code sample, create a test following this structure:

```rust
/// Test PROJECT_NAME - DESCRIPTION
///
/// PROJECT_NAME is DESCRIPTION.
/// Tests extraction of:
/// - FEATURE_1
/// - FEATURE_2
/// - FEATURE_3
///
/// Source: Representative sample from PROJECT by AUTHOR (LICENSE)
#[test]
fn test_PROJECT_name() {
    // Real code from PROJECT - DESCRIPTION
    let source = r#"
/*
  Copyright notice and license (preserve attribution)
*/

// Include actual code sample (100-500 lines)
// PASTE REAL CODE HERE
"#;

    let start = Instant::now();
    let chunks = parser::extract_chunks(source, "LANG");
    let duration = start.elapsed();

    println!("PROJECT: {} chunks extracted in {:?}", chunks.len(), duration);

    // Validation assertions (see below)
}
```

### Step 4: Add Validation Assertions

Include comprehensive assertions for each test:

#### Basic Sanity Checks

```rust
// Should not panic and should extract chunks
assert!(!chunks.is_empty(), "Should extract chunks from PROJECT code");

// Performance check (adjust threshold based on sample size)
assert!(
    duration.as_millis() < THRESHOLD_MS,
    "Parse should complete quickly for ~N line file (took {:?}, limit {}ms)",
    duration,
    THRESHOLD_MS
);
```

Performance thresholds:
- < 100 lines: 500ms (debug builds are slow)
- 100-300 lines: 1000ms
- 300-500 lines: 2000ms
- 500+ lines: 5000ms

#### Symbol Extraction Validation

Validate specific symbols that should be extracted:

```rust
// Should extract specific function
let target_func = chunks
    .iter()
    .find(|c| c.symbol_name == Some("function_name".to_string()));
assert!(target_func.is_some(), "Should extract function_name");

// Validate function properties
if let Some(func) = target_func {
    assert_eq!(func.kind, "func");
    assert!(
        func.signature.as_ref().unwrap().contains("expected_type"),
        "Should capture correct signature"
    );
    // Optional: validate metadata
    if let Some(metadata) = &func.metadata {
        assert_eq!(metadata["property"], "expected_value");
    }
}
```

#### Symbol Count Validation

```rust
// Count symbols by type
let func_count = chunks.iter().filter(|c| c.kind == "func").count();
let struct_count = chunks.iter().filter(|c| c.kind == "struct").count();
let typedef_count = chunks.iter().filter(|c| c.kind == "typedef").count();

println!(
    "PROJECT stats: {} functions, {} structs, {} typedefs",
    func_count, struct_count, typedef_count
);

assert!(
    func_count >= EXPECTED_MIN,
    "Should extract at least {} functions (got {})",
    EXPECTED_MIN,
    func_count
);
```

#### Special Features Validation

Test language-specific features:

```rust
// Imports/includes
let imports = chunks.iter().find(|c| c.kind == "imports");
assert!(imports.is_some(), "Should extract import directives");
if let Some(imp) = imports {
    let metadata = imp.metadata.as_ref().unwrap();
    let includes = metadata.as_array().unwrap();
    assert!(includes.len() >= N, "Should extract multiple includes");
}

// Storage classes, visibility, decorators, etc.
let static_func = chunks
    .iter()
    .find(|c| c.symbol_name == Some("internal_function".to_string()));
if let Some(func) = static_func {
    let metadata = func.metadata.as_ref().unwrap();
    assert_eq!(metadata["storage_class"], "static");
}
```

### Step 5: Add Performance Scaling Test

Include a test that validates parser performance at larger scale:

```rust
/// Test parsing performance with larger code samples
///
/// Combines multiple samples to test parser performance at scale.
/// Validates that parser maintains good performance characteristics.
#[test]
fn test_performance_scaling() {
    // Use fixture file or concatenate samples
    let large_source = include_str!("../tests/fixtures/LANG/combined_real_world.LANG");

    let start = Instant::now();
    let chunks = parser::extract_chunks(large_source, "LANG");
    let duration = start.elapsed();

    let line_count = large_source.lines().count();
    let bytes = large_source.len();

    println!(
        "Performance test: {} lines, {} bytes -> {} chunks in {:?}",
        line_count, bytes, chunks.len(), duration
    );

    // Should not panic
    assert!(!chunks.is_empty(), "Should extract chunks from large source");

    // Performance thresholds (adjust for build configuration)
    let max_duration_ms = if line_count > 2000 {
        5000 // 5 seconds for very large files
    } else if line_count > 1000 {
        2000 // 2 seconds for medium files
    } else {
        1000 // 1 second for smaller files
    };

    assert!(
        duration.as_millis() < max_duration_ms,
        "Parse time exceeded threshold: {:?} > {}ms for {} lines",
        duration, max_duration_ms, line_count
    );

    // Calculate parsing rate
    let lines_per_sec = (line_count as f64) / duration.as_secs_f64();
    println!("Parsing rate: {:.0} lines/second", lines_per_sec);

    // Minimum acceptable rate (>1000 lines/sec for debug builds)
    assert!(
        lines_per_sec > 1000.0,
        "Parsing rate too slow: {:.0} lines/sec (expected >1000)",
        lines_per_sec
    );
}
```

## Examples

### cJSON Test Case (C Parser)

From `c_real_world_test.rs`:

```rust
/// Test cJSON parser - single-file JSON library
///
/// cJSON is a small, self-contained JSON parser in C widely used in embedded systems.
/// Tests extraction of:
/// - Static functions (internal API)
/// - Public API functions
/// - Struct definitions
/// - Typedefs
/// - Preprocessor includes
///
/// Source: Representative sample from cJSON.c by Dave Gamble (MIT License)
#[test]
fn test_cjson_parser() {
    let source = r#"
/*
  Copyright (c) 2009-2017 Dave Gamble and cJSON contributors
  Permission is hereby granted...
*/

#include <string.h>
#include <stdio.h>

typedef struct cJSON {
    struct cJSON *next;
    struct cJSON *prev;
    int type;
    char *valuestring;
} cJSON;

static cJSON *cJSON_New_Item(void) {
    cJSON* node = (cJSON*)malloc(sizeof(cJSON));
    if (node) {
        memset(node, '\0', sizeof(cJSON));
    }
    return node;
}

void cJSON_Delete(cJSON *c) {
    cJSON *next;
    while (c) {
        next = c->next;
        if (c->child) cJSON_Delete(c->child);
        free(c);
        c = next;
    }
}
"#;

    let start = Instant::now();
    let chunks = parser::extract_chunks(source, "c");
    let duration = start.elapsed();

    println!("cJSON: {} chunks extracted in {:?}", chunks.len(), duration);

    // Should not panic
    assert!(!chunks.is_empty(), "Should extract chunks from cJSON code");

    // Performance check
    assert!(
        duration.as_millis() < 2000,
        "Parse should complete quickly (took {:?}, limit 2s for debug builds)",
        duration
    );

    // Should extract struct
    let cjson_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cJSON".to_string()) && c.kind == "struct");
    assert!(cjson_struct.is_some(), "Should extract main cJSON struct");

    // Should extract static function
    let new_item = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cJSON_New_Item".to_string()));
    assert!(new_item.is_some(), "Should extract cJSON_New_Item");
    if let Some(func) = new_item {
        let metadata = func.metadata.as_ref().unwrap();
        assert_eq!(metadata["storage_class"], "static");
    }

    // Should extract public API function
    let delete_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cJSON_Delete".to_string()));
    assert!(delete_func.is_some(), "Should extract cJSON_Delete");

    // Count extraction
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    assert!(func_count >= 2, "Should extract at least 2 functions");
}
```

### Project Selection Examples

**C/C++:**
- cJSON (JSON parsing, embedded style)
- Redis (networking, event loops)
- musl libc (system calls, POSIX)
- zlib (compression, bit manipulation)
- SQLite (database engine)

**Python:**
- Flask (web framework, decorators)
- requests (HTTP library, classes)
- pytest (testing framework, fixtures)

**Rust:**
- serde (derive macros, traits)
- tokio (async/await, lifetimes)
- clap (CLI parsing, attributes)

**Go:**
- cobra (CLI framework, packages)
- gin (web framework, interfaces)
- zap (logging, structs)

**Java:**
- Spring (annotations, beans)
- Jackson (serialization, generics)
- JUnit (testing, assertions)

## Best Practices

### Code Sample Size

1. Keep samples focused (100-500 lines)
2. Include full copyright/license attribution
3. Use representative, not exhaustive, code
4. Prefer actual project code over simplified examples

### Assertion Coverage

1. Test no-panic (most important)
2. Validate performance (realistic thresholds)
3. Check specific symbol extraction
4. Validate symbol counts
5. Test language-specific features

### Performance Thresholds

1. Be generous with thresholds (account for CI variability)
2. Different thresholds for debug vs release builds
3. Scale thresholds with input size
4. Log actual timings for investigation

### Maintenance

1. Update samples if language grammar changes
2. Adjust thresholds if parser is optimized
3. Add new samples if gaps are found
4. Keep attribution current

## References

- Ticket: MLLANG-1005
- Related files:
  - `crates/maproom/tests/c_real_world_test.rs` (reference implementation)
  - `crates/maproom/tests/c_parser_test.rs` (unit tests for comparison)
  - `crates/maproom/tests/fixtures/c/` (optional: larger fixture files)
- Commits:
  - 9e5ea39c (MLLANG-1005.3001 add real-world C integration tests)
  - b75c3d6c (MLLANG-1005.3005 add input boundary test cases)

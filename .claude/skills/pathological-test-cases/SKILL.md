---
name: pathological-test-cases
description: Test parser robustness with pathological inputs including empty files, syntax errors, whitespace-only, and edge cases
origin: MLLANG-1005
created: 2026-02-08
tags: [testing, robustness, edge-cases, parser, validation]
---

# Pathological Test Cases

## Overview

This skill documents the standard set of pathological test cases that should be included when testing any language parser in Maproom. These tests ensure parsers handle degenerate inputs gracefully without panicking, returning empty results or partial results as appropriate. Pathological testing is critical for production robustness since parsers will encounter malformed, incomplete, and edge-case files in real-world codebases.

## When to Use

- When implementing a new language parser for Maproom
- When adding robustness testing to existing parsers
- When debugging parser crashes or panics
- When validating parser behavior on incomplete or generated code

## Pattern/Procedure

### Required Test Cases

Every language parser should include these five pathological test cases:

#### 1. Empty File Test

**Purpose:** Verify parser handles completely empty input without panicking.

```rust
#[test]
fn test_LANG_empty_file() {
    let source = "";

    let chunks = parser::extract_chunks(source, "LANG");

    // Empty file should return empty chunks
    assert_eq!(chunks.len(), 0, "Empty file should produce no chunks");
}
```

**Expected behavior:** Return empty Vec, no panic, no error logging (silence is golden for valid empty files).

#### 2. Syntax Error Test

**Purpose:** Verify parser handles malformed syntax without panicking.

```rust
#[test]
fn test_LANG_syntax_error() {
    let source = r#"
MALFORMED SYNTAX THAT CANNOT BE PARSED
missing closing delimiters
unclosed strings "
broken structure {{{{
"#;

    // Parser should not panic on malformed code
    let chunks = parser::extract_chunks(source, "LANG");

    // May return empty or partial results, but should not crash
    // The key is that we reach this line without panicking
    let _ = chunks.len();
}
```

**Expected behavior:** Return empty or partial chunks (depending on parser error recovery), warn log is acceptable, but must not panic.

**Language-specific malformed examples:**

**C/C++:**
```c
int broken_function(
    // Missing closing paren and body
struct {{{{ invalid
```

**Python:**
```python
def broken_function(
    # Missing closing paren and body
class InvalidSyntax
    # Missing colon
```

**Rust:**
```rust
fn broken_function(
    // Missing closing paren and body
struct InvalidSyntax {
    // Missing closing brace
```

**JavaScript/TypeScript:**
```javascript
function broken(
// Missing closing paren and body
class InvalidSyntax {
// Missing closing brace
```

**Java:**
```java
public void broken(
// Missing closing paren and body
class InvalidSyntax {
// Missing closing brace
```

#### 3. Whitespace-Only File Test

**Purpose:** Verify parser handles files with only whitespace characters.

```rust
#[test]
fn test_LANG_whitespace_only_file() {
    let source = "   \n\n\t\t\n    \n\t  \n\n";

    let chunks = parser::extract_chunks(source, "LANG");

    // File with only whitespace should produce no chunks
    assert_eq!(
        chunks.len(),
        0,
        "Whitespace-only file should produce no chunks"
    );
}
```

**Expected behavior:** Return empty Vec (whitespace is not a symbol).

#### 4. Comment-Only File Test

**Purpose:** Verify parser handles files with only comments (no actual code).

```rust
#[test]
fn test_LANG_comment_only_file() {
    let source = r#"
COMMENT SYNTAX HERE
COMMENT SYNTAX HERE

MULTILINE COMMENT SYNTAX
with multiple lines
END COMMENT SYNTAX

MORE COMMENTS
"#;

    let chunks = parser::extract_chunks(source, "LANG");

    // File with only comments should produce no chunks
    assert_eq!(
        chunks.len(),
        0,
        "Comment-only file should produce no chunks"
    );
}
```

**Language-specific comment examples:**

**C/C++/Java:**
```c
// This is a line comment
// Another line comment

/*
 * This is a block comment
 * with multiple lines
 */

// More line comments
/* Single line block comment */
```

**Python:**
```python
# This is a comment
# Another comment

"""
This is a docstring but in a comment-only file
it shouldn't be extracted as a module docstring
"""

# More comments
```

**Rust:**
```rust
// This is a line comment
// Another line comment

/*
 * This is a block comment
 * with multiple lines
 */

/// Doc comment but no actual code
```

**JavaScript/TypeScript:**
```javascript
// This is a line comment
// Another line comment

/*
 * This is a block comment
 * with multiple lines
 */

// More comments
```

**Expected behavior:** Return empty Vec (comments without associated code are not symbols).

#### 5. Mixed Whitespace and Comments Test

**Purpose:** Verify parser handles files with interleaved whitespace and comments.

```rust
#[test]
fn test_LANG_mixed_whitespace_comments() {
    let source = r#"

    COMMENT WITH LEADING WHITESPACE

        BLOCK COMMENT WITH SPACES

ANOTHER COMMENT



"#;

    let chunks = parser::extract_chunks(source, "LANG");

    // File with mixed whitespace and comments should produce no chunks
    assert_eq!(
        chunks.len(),
        0,
        "Mixed whitespace and comments should produce no chunks"
    );
}
```

**Expected behavior:** Return empty Vec.

### Optional Advanced Test Cases

#### 6. Language-Specific Edge Cases

Add tests for language-specific edge cases that are valid but unusual:

**C: Preprocessor-Only File**
```rust
#[test]
fn test_c_preprocessor_only_file() {
    let source = r#"
#ifndef MY_HEADER_H
#define MY_HEADER_H

#endif // MY_HEADER_H
"#;

    // Parser should not panic on preprocessor-only files
    let chunks = parser::extract_chunks(source, "c");

    // Header guard only - may be empty or may have imports chunk
    // The key is that we don't panic and return a valid Vec
    let _ = chunks.len();
}
```

**Python: Encoding Declaration Only**
```rust
#[test]
fn test_python_encoding_only_file() {
    let source = "# -*- coding: utf-8 -*-\n";

    let chunks = parser::extract_chunks(source, "py");

    assert_eq!(chunks.len(), 0, "Encoding declaration only should produce no chunks");
}
```

**JavaScript: Use Strict Only**
```rust
#[test]
fn test_javascript_use_strict_only() {
    let source = "'use strict';\n";

    let chunks = parser::extract_chunks(source, "js");

    assert_eq!(chunks.len(), 0, "Use strict directive only should produce no chunks");
}
```

#### 7. Large Input Stress Test

Test parser behavior with very large inputs (10MB+):

```rust
#[test]
fn test_LANG_large_input() {
    // Generate a 10MB file of repetitive but valid code
    let mut source = String::with_capacity(10 * 1024 * 1024);
    for i in 0..100000 {
        source.push_str(&format!("function func_{}() {{ return {}; }}\n", i, i));
    }

    let start = std::time::Instant::now();
    let chunks = parser::extract_chunks(&source, "LANG");
    let duration = start.elapsed();

    // Should complete in reasonable time (adjust threshold)
    assert!(
        duration.as_secs() < 30,
        "Large file parse should complete in <30s (took {:?})",
        duration
    );

    // Should extract expected number of symbols
    assert!(chunks.len() > 90000, "Should extract most functions");
}
```

#### 8. Unicode and Special Characters

Test parser handling of Unicode identifiers and special characters:

```rust
#[test]
fn test_LANG_unicode_identifiers() {
    let source = r#"
function 函数名称() { }
class Klasse_Ñame { }
const переменная = 42;
"#;

    let chunks = parser::extract_chunks(source, "LANG");

    // Should handle Unicode identifiers if language supports them
    // At minimum, should not panic
    let _ = chunks.len();
}
```

## Examples

### C Pathological Tests (Complete Set)

From `c_parser_test.rs`:

```rust
#[test]
fn test_c_empty_file() {
    let source = "";
    let chunks = parser::extract_chunks(source, "c");
    assert_eq!(chunks.len(), 0, "Empty file should produce no chunks");
}

#[test]
fn test_c_syntax_error() {
    let source = r#"
int broken_function(
    // Missing closing paren and body
struct {{{{ invalid
"#;
    let chunks = parser::extract_chunks(source, "c");
    let _ = chunks.len(); // Should not panic
}

#[test]
fn test_c_whitespace_only_file() {
    let source = "   \n\n\t\t\n    \n\t  \n\n";
    let chunks = parser::extract_chunks(source, "c");
    assert_eq!(chunks.len(), 0, "Whitespace-only file should produce no chunks");
}

#[test]
fn test_c_comment_only_file() {
    let source = r#"
// This is a line comment
// Another line comment

/*
 * This is a block comment
 * with multiple lines
 */

// More line comments
/* Single line block comment */
"#;
    let chunks = parser::extract_chunks(source, "c");
    assert_eq!(chunks.len(), 0, "Comment-only file should produce no chunks");
}

#[test]
fn test_c_mixed_whitespace_comments() {
    let source = r#"

    // Comment with leading whitespace

        /* Block comment with spaces */

// Another comment



"#;
    let chunks = parser::extract_chunks(source, "c");
    assert_eq!(chunks.len(), 0, "Mixed whitespace and comments should produce no chunks");
}

#[test]
fn test_c_preprocessor_only_file() {
    let source = r#"
#ifndef MY_HEADER_H
#define MY_HEADER_H

#endif // MY_HEADER_H
"#;
    let chunks = parser::extract_chunks(source, "c");
    let _ = chunks.len(); // Valid preprocessor code, should not panic
}
```

## Test Organization

### File Placement

Place pathological tests in the main parser test file:

```
crates/maproom/tests/
├── LANG_parser_test.rs          # Unit tests + pathological tests
├── LANG_real_world_test.rs      # Real-world integration tests
└── fixtures/
    └── LANG/
        └── pathological/         # Optional: large fixture files
            ├── large_input.LANG
            └── unicode.LANG
```

### Test Naming Convention

Use consistent naming:
- `test_LANG_empty_file`
- `test_LANG_syntax_error`
- `test_LANG_whitespace_only_file`
- `test_LANG_comment_only_file`
- `test_LANG_mixed_whitespace_comments`
- `test_LANG_SPECIFIC_edge_case` (language-specific)

### Documentation

Add module-level comment explaining the test set:

```rust
//! Pathological test cases for LANG parser
//!
//! These tests validate parser robustness with edge cases:
//! - Empty files
//! - Malformed syntax
//! - Whitespace-only files
//! - Comment-only files
//! - Mixed whitespace and comments
//! - LANG-specific edge cases
```

## Expected Behavior Summary

| Test Case | Expected Result | Panic OK? | Empty Vec OK? |
|-----------|----------------|-----------|---------------|
| Empty file | Empty Vec | No | Yes |
| Syntax error | Empty or partial Vec | No | Yes |
| Whitespace only | Empty Vec | No | Yes |
| Comments only | Empty Vec | No | Yes |
| Mixed whitespace/comments | Empty Vec | No | Yes |
| Language edge case | Varies | No | Depends |

**Key principle:** Parsers must NEVER panic on any input. Returning empty or partial results is acceptable for malformed input.

## Best Practices

### Test Coverage

1. Include all five core pathological tests for every parser
2. Add language-specific edge case tests
3. Run tests in CI to catch regressions

### Assertion Strategy

1. Use `assert_eq!(chunks.len(), 0, "...")` for expected-empty cases
2. Use `let _ = chunks.len();` for "should not panic" cases
3. Avoid over-specifying behavior for malformed input (allow parser flexibility)

### Error Handling

1. Parsers should log warnings for parse failures (using tracing::warn)
2. Parsers should NOT propagate panics to callers
3. Malformed input should return empty Vec, not error Result

### Maintenance

1. Add new edge cases as bugs are discovered
2. Update tests if parser error recovery improves
3. Keep test cases minimal (don't test every possible syntax error)

## References

- Ticket: MLLANG-1005
- Related files:
  - `crates/maproom/tests/c_parser_test.rs` (reference implementation, tests at lines 367-472)
  - `crates/maproom/src/indexer/parser/c_lang.rs` (parser with tracing::warn for failures)
- Commits:
  - a5dea708 (MLLANG-1005.2001 basic edge case tests)
  - b75c3d6c (MLLANG-1005.3005 add input boundary test cases)
  - 08b50347 (MLLANG-1005.3010 add pathological comment extraction tests)

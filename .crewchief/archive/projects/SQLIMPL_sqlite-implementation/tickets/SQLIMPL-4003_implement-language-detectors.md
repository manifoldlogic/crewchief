# Ticket: SQLIMPL-4003: Implement Language Detectors

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Phase 4 - OPTIONAL ENHANCEMENT:** This ticket is part of the optional context assembly phase. Defer if timeline pressure.

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement language-specific detectors for identifying code patterns like JSX components and React hooks. These may require tree-sitter queries, not just SQL.

## Background
The language detectors at `src/context/detectors/` have stubbed methods for pattern detection. These analyze code to identify language-specific constructs that need special context handling.

**Note:** These may require tree-sitter integration, not just database queries.

This ticket implements Plan Phase 4, Ticket 4003: "Implement Language Detectors".

## Acceptance Criteria
- [x] JSX detector identifies React components
- [x] JSX detector finds component props and children
- [x] JSX detector identifies component usage patterns
- [x] Hooks detector identifies React hooks (useState, useEffect, etc.)
- [x] Hooks detector finds hook dependencies
- [x] Hooks detector identifies custom hooks
- [x] Detector tests pass

## Technical Requirements
- May use tree-sitter queries for AST analysis
- May delegate to SqliteStore for relationship queries
- Pattern matching for language-specific constructs
- Handle multiple file types (JSX, TSX)

## Implementation Notes

### Current Stubs

#### JSX Detector (3 methods)
```rust
// src/context/detectors/jsx.rs:79
// detect_components() - stub

// src/context/detectors/jsx.rs:98
// detect_props() - stub

// src/context/detectors/jsx.rs:117
// detect_usage() - stub
```

#### Hooks Detector (3 methods)
```rust
// src/context/detectors/hooks.rs:118
// detect_hooks() - stub

// src/context/detectors/hooks.rs:135
// detect_dependencies() - stub

// src/context/detectors/hooks.rs:159
// detect_custom_hooks() - stub
```

### Target Implementation Approach

#### JSX Component Detection
```rust
pub fn detect_components(&self, chunk: &Chunk) -> Vec<ComponentInfo> {
    let mut components = Vec::new();

    // Option 1: Tree-sitter query
    if let Some(tree) = &chunk.parsed_tree {
        let query = tree_sitter::Query::new(
            tree_sitter_typescript::language_tsx(),
            "(jsx_element) @component"
        )?;
        // Extract component names, props, etc.
    }

    // Option 2: Pattern matching on content
    let re = Regex::new(r"<([A-Z][a-zA-Z]*)")?;
    for cap in re.captures_iter(&chunk.content) {
        components.push(ComponentInfo {
            name: cap[1].to_string(),
            // ...
        });
    }

    components
}
```

#### Hooks Detection
```rust
pub fn detect_hooks(&self, chunk: &Chunk) -> Vec<HookInfo> {
    let mut hooks = Vec::new();

    // Pattern: useState, useEffect, useMemo, useCallback, etc.
    let re = Regex::new(r"use[A-Z][a-zA-Z]*\s*\(")?;
    for mat in re.find_iter(&chunk.content) {
        hooks.push(HookInfo {
            name: mat.as_str().trim_end_matches('(').to_string(),
            // ...
        });
    }

    hooks
}
```

## Dependencies
- Phase 1 Complete (tests compile)

## Risk Assessment
- **Risk**: Tree-sitter queries may be complex
  - **Mitigation**: Start with regex patterns; add tree-sitter later
- **Risk**: False positives in detection
  - **Mitigation**: Conservative matching; document limitations

## Files/Packages Affected
- `crates/maproom/src/context/detectors/jsx.rs`
- `crates/maproom/src/context/detectors/hooks.rs`

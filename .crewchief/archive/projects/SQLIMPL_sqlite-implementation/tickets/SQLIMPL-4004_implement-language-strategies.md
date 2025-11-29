# Ticket: SQLIMPL-4004: Implement Language Strategies

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
Implement language-specific context expansion strategies for React, Python, and Rust. These strategies determine how to expand context based on language patterns.

## Background
The language strategies at `src/context/strategies/` provide language-aware context expansion. They use the detectors (SQLIMPL-4003) and graph (SQLIMPL-4002) to build comprehensive context.

This ticket implements Plan Phase 4, Ticket 4004: "Implement Language Strategies".

## Acceptance Criteria
- [x] React strategy expands context for components, hooks, and props
- [x] Python strategy expands context for classes, functions, and imports
- [x] Rust strategy expands context for modules, traits, and implementations
- [x] Language-specific context expansion works correctly
- [x] Strategy tests pass
- [x] Phase 4 gate achieved: context assembly returns expanded results

## Technical Requirements
- Use detectors from SQLIMPL-4003 for pattern identification
- Use context graph from SQLIMPL-4002 for relationship traversal
- Each strategy implements common interface
- Strategies composable for multi-language files

## Implementation Notes

### Files to Implement
- `src/context/strategies/react.rs`
- `src/context/strategies/python.rs`
- `src/context/strategies/rust.rs`

### Common Strategy Interface
```rust
pub trait ContextStrategy {
    /// Expand context for a given chunk
    async fn expand(&self, chunk: &Chunk, depth: i32) -> Result<ExpandedContext>;

    /// Check if this strategy applies to the chunk
    fn applies_to(&self, chunk: &Chunk) -> bool;

    /// Get related chunks based on language patterns
    async fn get_related(&self, chunk: &Chunk) -> Result<Vec<RelatedChunk>>;
}
```

### React Strategy
```rust
impl ContextStrategy for ReactStrategy {
    async fn expand(&self, chunk: &Chunk, depth: i32) -> Result<ExpandedContext> {
        let mut context = ExpandedContext::new(chunk);

        // 1. Detect components and hooks
        let components = self.jsx_detector.detect_components(chunk);
        let hooks = self.hooks_detector.detect_hooks(chunk);

        // 2. Find related components (parent/child relationships)
        for component in components {
            let related = self.graph.find_component_usage(&component.name).await?;
            context.add_related(related);
        }

        // 3. Find hook implementations for custom hooks
        for hook in hooks {
            if hook.is_custom() {
                let impl_chunks = self.find_hook_implementation(&hook.name).await?;
                context.add_related(impl_chunks);
            }
        }

        // 4. Add imports and dependencies
        let imports = self.graph.get_imports(chunk.id, depth).await?;
        context.add_related(imports);

        Ok(context)
    }

    fn applies_to(&self, chunk: &Chunk) -> bool {
        matches!(chunk.language(), Language::TypescriptReact | Language::JavascriptReact)
    }
}
```

### Python Strategy
```rust
impl ContextStrategy for PythonStrategy {
    async fn expand(&self, chunk: &Chunk, depth: i32) -> Result<ExpandedContext> {
        let mut context = ExpandedContext::new(chunk);

        // 1. Find class hierarchy (base classes, subclasses)
        if chunk.kind == ChunkKind::Class {
            let bases = self.graph.find_base_classes(chunk.id).await?;
            let subclasses = self.graph.find_subclasses(chunk.id).await?;
            context.add_related(bases);
            context.add_related(subclasses);
        }

        // 2. Find function callers/callees
        if chunk.kind == ChunkKind::Function {
            let callers = self.graph.get_callers(chunk.id, depth).await?;
            let callees = self.graph.get_callees(chunk.id, depth).await?;
            context.add_related(callers);
            context.add_related(callees);
        }

        // 3. Add imports
        let imports = self.graph.get_imports(chunk.id, depth).await?;
        context.add_related(imports);

        Ok(context)
    }
}
```

### Rust Strategy
```rust
impl ContextStrategy for RustStrategy {
    async fn expand(&self, chunk: &Chunk, depth: i32) -> Result<ExpandedContext> {
        let mut context = ExpandedContext::new(chunk);

        // 1. Find trait implementations
        if chunk.kind == ChunkKind::Trait {
            let impls = self.graph.find_implementations(chunk.id).await?;
            context.add_related(impls);
        }

        // 2. Find module hierarchy
        let module_chunks = self.graph.find_module_contents(chunk.id).await?;
        context.add_related(module_chunks);

        // 3. Find macro usages
        if chunk.is_macro_definition() {
            let usages = self.graph.find_macro_usages(&chunk.name).await?;
            context.add_related(usages);
        }

        Ok(context)
    }
}
```

## Dependencies
- SQLIMPL-4002 (Context Graph)
- SQLIMPL-4003 (Language Detectors)

## Risk Assessment
- **Risk**: Complex language patterns may not be fully supported
  - **Mitigation**: Start with common patterns; document limitations
- **Risk**: Strategy composition may be complex
  - **Mitigation**: Use simple dispatch based on file extension

## Files/Packages Affected
- `crates/maproom/src/context/strategies/react.rs`
- `crates/maproom/src/context/strategies/python.rs`
- `crates/maproom/src/context/strategies/rust.rs`
- `crates/maproom/src/context/strategies/mod.rs`

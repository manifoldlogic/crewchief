# Ticket: CONTEXT_ASM-2003: React-Specific Assembly Logic

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement React-specific assembly strategy that detects React components, finds route definitions, includes hooks and context providers, and handles JSX relationships to provide optimal context for React-based codebases.

## Background
React applications have unique structural patterns that require specialized context assembly logic. Components need their route definitions, hook dependencies, and JSX relationships included in context bundles for Claude to understand the full picture. This ticket implements the React strategy outlined in the CONTEXT_ASM architecture document (Phase 2, Week 4, Task 1).

The React strategy extends the default assembly strategy with React-specific enhancements:
- Component detection via naming conventions (PascalCase .tsx files)
- Route discovery in routing files (App.tsx, routes/*)
- Hook usage tracking (useState, useEffect, custom hooks)
- React Context provider detection
- JSX parent-child relationships

## Acceptance Criteria
- [ ] React components detected via naming conventions and file patterns
- [ ] Route definitions found and included for component files
- [ ] Hooks (built-in and custom) identified and included in context
- [ ] React Context providers detected and included
- [ ] JSX relationships (parent components, children) handled correctly
- [ ] Configuration supports React-specific options (include_routes, include_hooks, component_patterns)
- [ ] Unit tests cover component detection, route finding, hook detection, and JSX relationships
- [ ] Integration tests verify React strategy assembles complete context bundles

## Technical Requirements

### React Component Detection
- Implement `ComponentDetector` that identifies React components:
  - PascalCase naming convention in .tsx files
  - Files matching `component_patterns` from config (e.g., "components/**/*.tsx")
  - Presence of JSX return statements
  - Function/class components

### Route Discovery
- Implement route finding logic:
  - Search routing files (App.tsx, routes/*, router.tsx)
  - Identify route definitions that reference components
  - Support common routing libraries (React Router, Next.js, etc.)
  - Link components to their route paths

### Hook Detection
- Implement `HookDetector` that finds:
  - Built-in React hooks (useState, useEffect, useContext, etc.)
  - Custom hooks (use* naming convention)
  - Hook dependencies and relationships
  - Hook usage patterns within components

### JSX Relationship Handling
- Implement JSX relationship tracking:
  - Parent components that render target component
  - Child components rendered by target component
  - Props passed between components
  - Component composition patterns

### Configuration Support
- Support React-specific config options:
  ```yaml
  strategies:
    react:
      include_routes: true
      include_hooks: true
      component_patterns: ["*.tsx", "components/**"]
  ```

## Implementation Notes

### Rust Module Structure
```rust
// crates/maproom/src/context/strategies/react.rs
pub struct ReactAssemblyStrategy {
    base_strategy: DefaultAssemblyStrategy,
    config: ReactConfig,
}

impl ReactAssemblyStrategy {
    pub async fn assemble(&self, target: &Chunk, budget: usize) -> Result<Vec<ContextItem>> {
        let mut items = self.base_strategy.assemble(target, budget).await?;

        // React-specific enhancements
        if self.is_component(target) {
            if self.config.include_routes {
                if let Some(route) = self.find_route(target).await? {
                    items.push(self.format_chunk(route, "route"));
                }
            }

            if self.config.include_hooks {
                let hooks = self.find_used_hooks(target).await?;
                for hook in hooks {
                    items.push(self.format_chunk(hook, "hook"));
                }
            }
        }

        Ok(items)
    }
}
```

### Component Detection
```rust
// crates/maproom/src/context/detectors/component.rs
pub struct ComponentDetector;

impl ComponentDetector {
    pub fn is_component(&self, chunk: &Chunk) -> bool {
        // Check file extension
        if !chunk.file.relpath.ends_with(".tsx") {
            return false;
        }

        // Check PascalCase naming
        let file_name = chunk.file.relpath.file_stem();
        if !self.is_pascal_case(file_name) {
            return false;
        }

        // Check for JSX return (via tree-sitter)
        self.has_jsx_return(chunk)
    }
}
```

### Hook Detection
```rust
// crates/maproom/src/context/detectors/hooks.rs
pub struct HookDetector {
    db: Pool<Postgres>,
}

impl HookDetector {
    pub async fn find_used_hooks(&self, component: &Chunk) -> Result<Vec<Chunk>> {
        // Query for hook identifiers in component
        let hook_calls = self.find_hook_calls(component).await?;

        // Find hook definitions
        let mut hooks = Vec::new();
        for call in hook_calls {
            if let Some(definition) = self.find_hook_definition(&call).await? {
                hooks.push(definition);
            }
        }

        Ok(hooks)
    }
}
```

### Testing Strategy
- Unit tests for each detector (component, hooks)
- Integration tests with sample React projects
- Test various React patterns (function components, class components, hooks, context)
- Verify route finding across different routing libraries
- Test JSX relationship extraction

### Performance Considerations
- Cache component detection results
- Batch hook lookups to reduce database queries
- Limit depth of JSX relationship traversal
- Use efficient tree-sitter queries for pattern matching

## Dependencies
- **CONTEXT_ASM-1004** (Content Formatting) - Required for formatting chunks with roles
- Tree-sitter parser with JSX support
- Database schema with relationship tables

## Risk Assessment
- **Risk**: False positives in component detection (PascalCase utils, types, etc.)
  - **Mitigation**: Combine multiple heuristics (naming + JSX presence + file patterns)

- **Risk**: Route finding may not work with all routing libraries
  - **Mitigation**: Support most common libraries (React Router, Next.js) initially, make extensible

- **Risk**: Hook detection complexity with custom hooks
  - **Mitigation**: Use naming convention (use* prefix) and tree-sitter pattern matching

- **Risk**: JSX relationships can create large context graphs
  - **Mitigation**: Implement depth limits and budget controls

## Files/Packages Affected
- `crates/maproom/src/context/strategies/react.rs` - React assembly strategy implementation
- `crates/maproom/src/context/detectors/component.rs` - Component detection logic
- `crates/maproom/src/context/detectors/hooks.rs` - Hook detection logic
- `crates/maproom/src/context/detectors/mod.rs` - Module exports for detectors
- `crates/maproom/src/context/config.rs` - React configuration options
- `crates/maproom/tests/context/react_strategy_test.rs` - Unit tests for React strategy
- `crates/maproom/tests/context/component_detector_test.rs` - Component detection tests
- `crates/maproom/tests/context/hook_detector_test.rs` - Hook detection tests
- `crates/maproom/tests/integration/react_assembly_test.rs` - Integration tests

# Custom Strategy Implementation Guide

Version: 1.0.0
Last Updated: 2025-10-24

## Overview

The Context Assembly system supports language-specific strategies for intelligent context gathering. This guide shows how to implement custom strategies for new programming languages or specialized use cases.

## Strategy System Architecture

### AssemblyStrategy Trait

The core trait that all strategies must implement:

```rust
pub trait AssemblyStrategy: Send + Sync {
    /// Detect if this strategy applies to the given chunk
    fn applies_to(&self, chunk: &ChunkMetadata) -> bool;

    /// Prioritize which relationships to include
    fn prioritize_relationships(
        &self,
        chunk: &ChunkMetadata,
        relationships: &RelatedChunks,
    ) -> Vec<PrioritizedItem>;

    /// Calculate importance score for a chunk
    fn calculate_importance(
        &self,
        chunk: &ChunkMetadata,
        context: &AssemblyContext,
    ) -> f64;
}
```

### Built-in Strategies

The system includes these pre-built strategies:

1. **ReactStrategy** - React/JSX components, hooks, and patterns
2. **TypeScriptStrategy** - TypeScript/JavaScript modules
3. **RustStrategy** - Rust modules, traits, and implementations
4. **PythonStrategy** - Python classes and modules
5. **MarkdownStrategy** - Documentation files
6. **GenericStrategy** - Fallback for unknown languages

## Implementing a Custom Strategy

### Step 1: Define the Strategy Struct

```rust
use crewchief_maproom::context::strategy::{AssemblyStrategy, AssemblyContext};
use crewchief_maproom::context::types::{ChunkMetadata, RelatedChunks, PrioritizedItem};

pub struct GoStrategy {
    // Configuration fields
    prioritize_interfaces: bool,
    include_test_files: bool,
}

impl GoStrategy {
    pub fn new() -> Self {
        Self {
            prioritize_interfaces: true,
            include_test_files: true,
        }
    }

    pub fn with_config(prioritize_interfaces: bool, include_test_files: bool) -> Self {
        Self {
            prioritize_interfaces,
            include_test_files,
        }
    }
}
```

### Step 2: Implement `applies_to`

Determine when this strategy should be used:

```rust
impl AssemblyStrategy for GoStrategy {
    fn applies_to(&self, chunk: &ChunkMetadata) -> bool {
        // Check file extension
        if chunk.relpath.ends_with(".go") {
            return true;
        }

        // Check language metadata if available
        if let Some(ref language) = chunk.language {
            return language == "go" || language == "golang";
        }

        false
    }

    // ... other methods
}
```

**Detection Strategies:**
- File extension (`.go`, `.py`, `.rs`, etc.)
- Language metadata from database
- File path patterns (`src/`, `lib/`, `__tests__/`)
- Symbol kind (`function`, `class`, `interface`)

### Step 3: Implement `prioritize_relationships`

Decide which related chunks are most important:

```rust
impl AssemblyStrategy for GoStrategy {
    fn prioritize_relationships(
        &self,
        chunk: &ChunkMetadata,
        relationships: &RelatedChunks,
    ) -> Vec<PrioritizedItem> {
        let mut items = Vec::new();

        // Prioritize test files
        if self.include_test_files {
            for test in &relationships.tests {
                items.push(PrioritizedItem {
                    chunk: test.clone(),
                    priority: 10.0,  // High priority
                    reason: "Test coverage for this function".to_string(),
                });
            }
        }

        // Prioritize interface implementations
        for callee in &relationships.callees {
            let priority = if callee.kind.as_deref() == Some("interface") && self.prioritize_interfaces {
                9.0  // Very high priority
            } else {
                5.0  // Normal priority
            };

            items.push(PrioritizedItem {
                chunk: callee.clone(),
                priority,
                reason: format!("Called by {}", chunk.symbol_name.as_deref().unwrap_or("this code")),
            });
        }

        // Callers get medium priority
        for caller in &relationships.callers {
            items.push(PrioritizedItem {
                chunk: caller.clone(),
                priority: 6.0,
                reason: format!("Calls {}", chunk.symbol_name.as_deref().unwrap_or("this")),
            });
        }

        // Sort by priority (highest first)
        items.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
        items
    }

    // ... other methods
}
```

**Priority Guidelines:**
- **10.0**: Critical (tests, main interfaces)
- **8.0-9.0**: Very high (direct dependencies, exports)
- **5.0-7.0**: High (callers, callees, imports)
- **3.0-4.0**: Medium (documentation, config)
- **1.0-2.0**: Low (distant relationships)

### Step 4: Implement `calculate_importance`

Score how important a chunk is:

```rust
impl AssemblyStrategy for GoStrategy {
    fn calculate_importance(
        &self,
        chunk: &ChunkMetadata,
        context: &AssemblyContext,
    ) -> f64 {
        let mut score = 1.0;

        // Boost exported functions
        if chunk.is_exported.unwrap_or(false) {
            score *= 1.5;
        }

        // Boost interface definitions
        if chunk.kind.as_deref() == Some("interface") {
            score *= 2.0;
        }

        // Boost if in the same package
        if self.is_same_package(&chunk.relpath, &context.target_chunk.relpath) {
            score *= 1.3;
        }

        // Boost based on relationship distance
        if let Some(distance) = chunk.relationship_distance {
            score *= 0.8_f64.powi(distance as i32);
        }

        // Penalize test files unless we're looking at a test
        if chunk.relpath.ends_with("_test.go") && !context.target_chunk.relpath.ends_with("_test.go") {
            score *= 0.7;
        }

        score.clamp(0.0, 10.0)
    }
}

impl GoStrategy {
    fn is_same_package(&self, path1: &str, path2: &str) -> bool {
        // Go packages are defined by directory
        let dir1 = std::path::Path::new(path1).parent();
        let dir2 = std::path::Path::new(path2).parent();
        dir1 == dir2
    }
}
```

**Importance Factors:**
- **Visibility**: Exported/public symbols more important
- **Type**: Interfaces, classes, main functions boosted
- **Proximity**: Same package/module boosted
- **Distance**: Decay by relationship hops
- **Usage**: High-fanout (many callers) boosted

## Complete Example: Python Strategy

```rust
use crewchief_maproom::context::strategy::{AssemblyStrategy, AssemblyContext};
use crewchief_maproom::context::types::{ChunkMetadata, RelatedChunks, PrioritizedItem};

pub struct PythonStrategy {
    prioritize_classes: bool,
    include_dunder_methods: bool,
}

impl PythonStrategy {
    pub fn new() -> Self {
        Self {
            prioritize_classes: true,
            include_dunder_methods: false,
        }
    }
}

impl AssemblyStrategy for PythonStrategy {
    fn applies_to(&self, chunk: &ChunkMetadata) -> bool {
        chunk.relpath.ends_with(".py") ||
        chunk.language.as_deref() == Some("python")
    }

    fn prioritize_relationships(
        &self,
        chunk: &ChunkMetadata,
        relationships: &RelatedChunks,
    ) -> Vec<PrioritizedItem> {
        let mut items = Vec::new();

        // Tests are high priority
        for test in &relationships.tests {
            let priority = if test.relpath.contains("test_") || test.relpath.contains("_test") {
                10.0
            } else {
                8.0
            };

            items.push(PrioritizedItem {
                chunk: test.clone(),
                priority,
                reason: "Test for this code".to_string(),
            });
        }

        // Base classes and superclasses
        for callee in &relationships.callees {
            let priority = if callee.kind.as_deref() == Some("class_definition") {
                if self.prioritize_classes {
                    9.0  // Base classes very important
                } else {
                    7.0
                }
            } else if callee.kind.as_deref() == Some("function_definition") {
                6.0
            } else {
                5.0
            };

            items.push(PrioritizedItem {
                chunk: callee.clone(),
                priority,
                reason: "Dependency of this code".to_string(),
            });
        }

        // Filter dunder methods if configured
        let callers: Vec<_> = if !self.include_dunder_methods {
            relationships.callers.iter()
                .filter(|c| !c.symbol_name.as_deref().unwrap_or("").starts_with("__"))
                .cloned()
                .collect()
        } else {
            relationships.callers.clone()
        };

        for caller in callers {
            items.push(PrioritizedItem {
                chunk: caller,
                priority: 6.0,
                reason: "Uses this code".to_string(),
            });
        }

        items.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
        items
    }

    fn calculate_importance(
        &self,
        chunk: &ChunkMetadata,
        context: &AssemblyContext,
    ) -> f64 {
        let mut score = 1.0;

        // Boost classes
        if chunk.kind.as_deref() == Some("class_definition") {
            score *= 1.5;
        }

        // Boost decorators (like @property, @staticmethod)
        if chunk.symbol_name.as_deref().map_or(false, |n| n.starts_with('@')) {
            score *= 1.3;
        }

        // Same module bonus
        if self.is_same_module(&chunk.relpath, &context.target_chunk.relpath) {
            score *= 1.4;
        }

        // Apply distance decay
        if let Some(distance) = chunk.relationship_distance {
            score *= 0.75_f64.powi(distance as i32);
        }

        score.clamp(0.0, 10.0)
    }
}

impl PythonStrategy {
    fn is_same_module(&self, path1: &str, path2: &str) -> bool {
        // Python modules correspond to .py files or __init__.py directories
        let without_ext1 = path1.trim_end_matches(".py");
        let without_ext2 = path2.trim_end_matches(".py");

        let parent1 = std::path::Path::new(without_ext1).parent();
        let parent2 = std::path::Path::new(without_ext2).parent();

        parent1 == parent2
    }
}
```

## Registering Strategies

### Strategy Registry

```rust
use crewchief_maproom::context::strategy::StrategyRegistry;

let mut registry = StrategyRegistry::new();

// Register your custom strategy
registry.register(Box::new(GoStrategy::new()));
registry.register(Box::new(PythonStrategy::new()));

// Strategies are tried in order until one matches
```

### Integration with Assembler

```rust
use crewchief_maproom::context::ParallelContextAssembler;

let assembler = ParallelContextAssembler::new_with_strategies(
    pool,
    cache_config,
    vec![
        Box::new(ReactStrategy::new()),
        Box::new(GoStrategy::new()),
        Box::new(PythonStrategy::new()),
        Box::new(GenericStrategy::new()),  // Fallback
    ],
);
```

## Testing Custom Strategies

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_applies_to_go_files() {
        let strategy = GoStrategy::new();
        let chunk = ChunkMetadata {
            relpath: "main.go".to_string(),
            ..Default::default()
        };

        assert!(strategy.applies_to(&chunk));
    }

    #[test]
    fn test_prioritizes_interfaces() {
        let strategy = GoStrategy::new();
        let chunk = ChunkMetadata {
            relpath: "handler.go".to_string(),
            symbol_name: Some("HandleRequest".to_string()),
            ..Default::default()
        };

        let relationships = RelatedChunks {
            tests: vec![],
            callers: vec![],
            callees: vec![
                ChunkMetadata {
                    relpath: "types.go".to_string(),
                    kind: Some("interface".to_string()),
                    symbol_name: Some("Handler".to_string()),
                    ..Default::default()
                },
            ],
        };

        let items = strategy.prioritize_relationships(&chunk, &relationships);
        assert_eq!(items[0].priority, 9.0);  // Interface should be high priority
    }

    #[test]
    fn test_importance_scoring() {
        let strategy = GoStrategy::new();
        let chunk = ChunkMetadata {
            relpath: "pkg/handler.go".to_string(),
            kind: Some("interface".to_string()),
            is_exported: Some(true),
            relationship_distance: Some(1),
            ..Default::default()
        };

        let context = AssemblyContext {
            target_chunk: ChunkMetadata {
                relpath: "pkg/main.go".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let score = strategy.calculate_importance(&chunk, &context);
        assert!(score > 2.0);  // Exported interface in same package should score high
    }
}
```

## Advanced Patterns

### Composite Strategies

Combine multiple strategies:

```rust
pub struct CompositeStrategy {
    strategies: Vec<Box<dyn AssemblyStrategy>>,
}

impl AssemblyStrategy for CompositeStrategy {
    fn applies_to(&self, chunk: &ChunkMetadata) -> bool {
        // Applies if any sub-strategy applies
        self.strategies.iter().any(|s| s.applies_to(chunk))
    }

    fn prioritize_relationships(
        &self,
        chunk: &ChunkMetadata,
        relationships: &RelatedChunks,
    ) -> Vec<PrioritizedItem> {
        // Combine results from all applicable strategies
        let mut all_items = Vec::new();

        for strategy in &self.strategies {
            if strategy.applies_to(chunk) {
                let items = strategy.prioritize_relationships(chunk, relationships);
                all_items.extend(items);
            }
        }

        // Deduplicate and re-sort
        all_items.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
        all_items
    }

    fn calculate_importance(
        &self,
        chunk: &ChunkMetadata,
        context: &AssemblyContext,
    ) -> f64 {
        // Average importance from all applicable strategies
        let applicable: Vec<_> = self.strategies.iter()
            .filter(|s| s.applies_to(chunk))
            .collect();

        if applicable.is_empty() {
            return 1.0;
        }

        let sum: f64 = applicable.iter()
            .map(|s| s.calculate_importance(chunk, context))
            .sum();

        sum / applicable.len() as f64
    }
}
```

### Configurable Strategies

Use builder pattern for configuration:

```rust
pub struct ConfigurableStrategy {
    language: String,
    test_priority: f64,
    same_dir_bonus: f64,
    max_distance: usize,
}

impl ConfigurableStrategy {
    pub fn builder(language: &str) -> ConfigurableStrategyBuilder {
        ConfigurableStrategyBuilder::new(language)
    }
}

pub struct ConfigurableStrategyBuilder {
    language: String,
    test_priority: f64,
    same_dir_bonus: f64,
    max_distance: usize,
}

impl ConfigurableStrategyBuilder {
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            test_priority: 10.0,
            same_dir_bonus: 1.3,
            max_distance: 3,
        }
    }

    pub fn test_priority(mut self, priority: f64) -> Self {
        self.test_priority = priority;
        self
    }

    pub fn same_dir_bonus(mut self, bonus: f64) -> Self {
        self.same_dir_bonus = bonus;
        self
    }

    pub fn max_distance(mut self, distance: usize) -> Self {
        self.max_distance = distance;
        self
    }

    pub fn build(self) -> ConfigurableStrategy {
        ConfigurableStrategy {
            language: self.language,
            test_priority: self.test_priority,
            same_dir_bonus: self.same_dir_bonus,
            max_distance: self.max_distance,
        }
    }
}

// Usage
let go_strategy = ConfigurableStrategy::builder("go")
    .test_priority(9.0)
    .same_dir_bonus(1.5)
    .max_distance(2)
    .build();
```

## Best Practices

1. **Keep strategies focused** - One strategy per language or paradigm
2. **Use sensible defaults** - Works well out of the box
3. **Make strategies configurable** - Builder pattern for options
4. **Test thoroughly** - Unit tests for each method
5. **Document reasoning** - Explain priority and importance calculations
6. **Clamp scores** - Use `clamp(0.0, 10.0)` to prevent extreme values
7. **Handle missing data** - Use `Option` and defaults gracefully
8. **Optimize for common case** - Prioritize what users need most often

## See Also

- [API Reference](context_assembly_api.md) - Core types and traits
- [Configuration Guide](context_configuration.md) - Strategy registration
- [Performance Tuning](context_performance_tuning.md) - Strategy optimization

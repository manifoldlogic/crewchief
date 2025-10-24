# Ticket: CONTEXT_ASM-2004: Language Strategy Framework

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Create a flexible strategy framework for language-specific context assembly in Maproom. This framework will enable intelligent context selection based on the programming language being worked with, starting with Python and Rust strategies.

## Background
Different programming languages have different patterns for code organization, testing, and dependencies. A TypeScript project might have `index.ts` and `package.json`, while a Python project has `__init__.py` and `requirements.txt`. Similarly, import patterns, class hierarchies, and module structures vary by language.

The context assembler needs language-specific strategies to select the most relevant context for each language. This ticket implements the strategy pattern with automatic language detection and configurable strategy selection, building upon the React strategy pattern demonstrated in the architecture doc.

## Acceptance Criteria
- [ ] `AssemblyStrategy` trait created with `assemble()` method signature
- [ ] Language detection implemented based on file extension and chunk metadata
- [ ] `DefaultAssemblyStrategy` implemented as fallback (following architecture doc lines 116-144)
- [ ] `PythonAssemblyStrategy` implemented with Python-specific patterns
- [ ] `RustAssemblyStrategy` implemented with Rust-specific patterns
- [ ] Strategy selection configurable via maproom config
- [ ] Automatic fallback to default strategy when language-specific strategy unavailable
- [ ] Unit tests verify language detection accuracy
- [ ] Unit tests verify each strategy's behavior
- [ ] Integration tests demonstrate strategy selection based on file type

## Technical Requirements

### Strategy Trait
- Define `AssemblyStrategy` trait in `crates/maproom/src/context/strategy.rs`
- Core method: `async fn assemble(&self, target: &Chunk, budget: usize) -> Result<Vec<ContextItem>>`
- Additional helper methods for finding related code (tests, callers, callees, config)

### Language Detection
- Create `LanguageDetector` in `crates/maproom/src/context/language_detector.rs`
- Detect language from file extension (`.py`, `.rs`, `.ts`, `.tsx`, etc.)
- Fall back to chunk metadata if extension ambiguous
- Support language override via configuration

### Default Strategy
- Implement baseline strategy following architecture doc (lines 116-144):
  - Primary chunk (40% of budget)
  - Direct test file
  - One top caller
  - One top callee
  - Config file if relevant
- This becomes the base implementation other strategies extend

### Python Strategy
- Extend `DefaultAssemblyStrategy`
- Include `__init__.py` for package context
- Include `requirements.txt` for dependency context
- Prioritize class hierarchies (parent classes, child classes)
- Include docstrings and type hints
- Detect test files using `test_*.py` or `*_test.py` patterns

### Rust Strategy
- Extend `DefaultAssemblyStrategy`
- Include `Cargo.toml` for crate metadata
- Include trait implementations (`impl Trait for Type`)
- Include module structure (`mod.rs`, `lib.rs`)
- Detect test modules using `#[cfg(test)]` and `#[test]` attributes
- Include macro definitions when referenced

### Configuration
- Follow config pattern from architecture doc (lines 214-230)
- Enable/disable strategies per language
- Configure strategy-specific options (e.g., `include_routes`, `component_patterns` for React)
- Set budget ratios per strategy

## Implementation Notes

### Architecture Pattern
Follow the React strategy example (architecture doc lines 147-167):
```rust
// Default strategy base
trait AssemblyStrategy {
    async fn assemble(&self, target: &Chunk, budget: usize) -> Result<Vec<ContextItem>>;
}

// Language-specific strategies extend default
struct PythonAssemblyStrategy {
    default: DefaultAssemblyStrategy,
}

impl AssemblyStrategy for PythonAssemblyStrategy {
    async fn assemble(&self, target: &Chunk, budget: usize) -> Result<Vec<ContextItem>> {
        let mut items = self.default.assemble(target, budget).await?;

        // Add Python-specific context
        if let Some(init) = self.find_init_py(target).await? {
            items.push(self.format_chunk(init, "package_init"));
        }

        Ok(items)
    }
}
```

### Language Detection Logic
- Primary: file extension mapping (`.py` -> Python, `.rs` -> Rust)
- Secondary: chunk metadata `language` field
- Tertiary: content-based detection for ambiguous files
- Cache detection results per file

### Strategy Selection
- Configuration specifies strategy per language
- Runtime lookup: `config.strategies.get(language).unwrap_or("default")`
- Strategy registry pattern for dynamic loading

### Testing Strategy
- Unit tests per strategy in `crates/maproom/tests/context/strategy_test.rs`
- Mock chunk data for each language type
- Verify correct context items selected
- Verify budget constraints respected
- Integration tests with real Python/Rust code samples

## Dependencies
- **CONTEXT_ASM-2003**: React strategy provides template pattern (referenced in architecture doc)
- No external library dependencies beyond existing Maproom dependencies

## Risk Assessment
- **Risk**: Language detection may be ambiguous for polyglot files (e.g., Rust macros generating code)
  - **Mitigation**: Support explicit language override in configuration; default to safest strategy

- **Risk**: Strategy complexity may grow unbounded as more languages added
  - **Mitigation**: Keep default strategy simple and well-documented; language strategies only override what's necessary

- **Risk**: Budget allocation across strategy components may require tuning per language
  - **Mitigation**: Make budget ratios configurable; provide sensible defaults; document tuning process

## Files/Packages Affected

### New Files to Create
- `crates/maproom/src/context/strategy.rs` - AssemblyStrategy trait definition
- `crates/maproom/src/context/strategies/mod.rs` - Strategy module exports
- `crates/maproom/src/context/strategies/default.rs` - Default strategy implementation
- `crates/maproom/src/context/strategies/python.rs` - Python strategy implementation
- `crates/maproom/src/context/strategies/rust.rs` - Rust strategy implementation
- `crates/maproom/src/context/language_detector.rs` - Language detection logic
- `crates/maproom/tests/context/strategy_test.rs` - Strategy unit tests
- `crates/maproom/tests/context/language_detector_test.rs` - Language detection tests

### Files to Modify
- `crates/maproom/src/context/mod.rs` - Add strategy module exports
- `crates/maproom/src/context/assembler.rs` - Integrate strategy selection
- `crates/maproom/src/config.rs` - Add strategy configuration schema

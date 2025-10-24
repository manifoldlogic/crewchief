# Ticket: LANG_PARSE-3004: Go Integration and Optimization

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- performance-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Integrate Go language support with existing TypeScript/JavaScript and Python parsers, implementing cross-language edge detection and comprehensive performance optimization across all three languages. This ticket unifies the multi-language parsing pipeline and ensures production readiness.

## Background
This is the culmination of Phase 3 (Multi-Language Support) in the LANG_PARSE project. With Go parsing complete (LANG_PARSE-3003), Python complete (LANG_PARSE-1008), and Rust complete (LANG_PARSE-2004), this ticket integrates all languages into a unified system. The focus is on:

1. **Unified Language Support**: Consistent symbol normalization and handling across TypeScript/JavaScript, Python, Go, and Rust
2. **Cross-Language Edge Detection**: Detecting calls between languages (e.g., Go calling C, TypeScript calling native modules, Python FFI)
3. **Performance Optimization**: Query optimization, caching strategies, and parallel processing to ensure <5% performance regression
4. **Production Readiness**: Testing with large codebases (like Kubernetes) and comprehensive documentation

This work enables Maproom to handle complex, mixed-language repositories with excellent performance.

## Acceptance Criteria
- [ ] All four languages (TypeScript/JavaScript, Python, Go, Rust) integrated into unified pipeline
- [ ] Symbol normalization is consistent across all languages
- [ ] Cross-language edge detection works for common FFI patterns (Go/C, Python/C, TypeScript/native)
- [ ] <5% performance regression on TypeScript/JavaScript parsing compared to baseline
- [ ] Query performance optimized with caching and parallel processing
- [ ] Mixed-language repositories (e.g., Kubernetes, Node.js with native modules) index successfully
- [ ] Comprehensive benchmarks show acceptable performance across all languages
- [ ] Complete documentation for multi-language support
- [ ] Integration tests cover mixed-language scenarios

## Technical Requirements
- Unified symbol normalization scheme that works across TypeScript/JavaScript, Python, Go, and Rust
- Cross-language edge detection for:
  - Go CGo calls to C code
  - Python FFI (ctypes, CFFI) to native code
  - TypeScript/JavaScript native module imports
  - Rust FFI
- Query optimization:
  - Efficient SQL queries for multi-language symbol lookups
  - Index optimization for cross-language searches
  - Caching layer for frequently accessed symbols
- Parallel processing:
  - Concurrent parsing of files across languages
  - Thread-safe symbol table updates
  - Resource management for large codebases
- Performance benchmarks:
  - Baseline measurements for TypeScript/JavaScript
  - Comparative benchmarks for Python, Go, Rust
  - Mixed-language repository benchmarks (Kubernetes as test case)
- Documentation:
  - Multi-language support architecture
  - Cross-language edge detection design
  - Performance tuning guide
  - API reference for language-agnostic queries

## Implementation Notes

### Unified Language Support
- Create a common `Symbol` trait/interface that all language parsers implement
- Normalize symbol names using a consistent scheme:
  - TypeScript/JavaScript: module-qualified names (e.g., `module.Class.method`)
  - Python: dot-separated names (e.g., `package.module.Class.method`)
  - Go: package-qualified names (e.g., `package.Type.Method`)
  - Rust: path-qualified names (e.g., `crate::module::Type::method`)
- Update `crates/maproom/src/parser/` to use unified symbol representation

### Cross-Language Edge Detection
- Implement `crates/maproom/src/parser/cross_language.rs`:
  - Detect CGo directives in Go files (`import "C"`)
  - Detect Python FFI patterns (ctypes, CFFI, PyO3 attributes)
  - Detect TypeScript/JavaScript native requires (`require('bindings')`)
  - Detect Rust FFI (`extern "C"`, `#[no_mangle]`)
- Create edges in the symbol graph for cross-language calls
- Store language boundary metadata for analysis

### Performance Optimization
- Query optimization:
  - Add composite indexes for common multi-language queries
  - Use prepared statements with query plan caching
  - Implement materialized views for expensive cross-language joins
- Caching strategies:
  - LRU cache for frequently accessed symbols
  - Bloom filters for quick symbol existence checks
  - Incremental index updates to avoid full rescans
- Parallel processing:
  - Use rayon for parallel file processing
  - Language-specific parser pools to avoid initialization overhead
  - Batch database operations to reduce transaction overhead

### Testing with Kubernetes
- Clone Kubernetes repository (large, mixed Go/C codebase)
- Index entire codebase and measure:
  - Time to index
  - Memory usage during indexing
  - Query performance for common patterns
  - Cross-language edge detection accuracy
- Validate results against known Kubernetes architecture

### Performance Targets
- TypeScript/JavaScript: <5% regression vs. baseline (from Phase 1)
- Python: Similar performance to TypeScript/JavaScript
- Go: Competitive with TypeScript/JavaScript (within 20%)
- Rust: Competitive with TypeScript/JavaScript (within 20%)
- Mixed-language: <10% overhead vs. single-language

## Dependencies
- **LANG_PARSE-3003**: Go parser complete with full test coverage
- **LANG_PARSE-1008**: Python parser complete and integrated
- **LANG_PARSE-2004**: Rust parser complete and integrated
- Baseline performance metrics from Phase 1 TypeScript/JavaScript implementation
- PostgreSQL with sufficient resources for large codebase indexing

## Risk Assessment
- **Risk**: Performance regression when integrating all languages
  - **Mitigation**: Comprehensive benchmarking before and after integration; rollback capability if regression exceeds 5%; incremental optimization guided by profiling

- **Risk**: Cross-language edge detection may have false positives/negatives
  - **Mitigation**: Conservative detection rules; manual validation with known codebases (Kubernetes, Node.js); allow users to disable cross-language analysis if needed

- **Risk**: Kubernetes indexing may exceed memory/time budgets
  - **Mitigation**: Implement streaming indexing for large repos; add resource limits; provide progress reporting; allow resumable indexing

- **Risk**: Symbol normalization conflicts between languages
  - **Mitigation**: Use language prefixes in normalized names if needed; store original language-specific names alongside normalized names; comprehensive test suite

- **Risk**: Parallel processing may introduce race conditions
  - **Mitigation**: Thorough review of thread safety; use Rust's type system to enforce safety; extensive testing under concurrent load

## Files/Packages Affected
- `crates/maproom/src/parser/mod.rs` - Update for unified pipeline
- `crates/maproom/src/parser/cross_language.rs` - **NEW** - Cross-language edge detection
- `crates/maproom/src/parser/symbol.rs` - **NEW** or update - Unified symbol representation
- `crates/maproom/src/parser/typescript.rs` - Update for unified symbols
- `crates/maproom/src/parser/python.rs` - Update for unified symbols
- `crates/maproom/src/parser/go.rs` - Update for unified symbols
- `crates/maproom/src/parser/rust.rs` - Update for unified symbols
- `crates/maproom/src/db/schema.rs` - Update for cross-language edges
- `crates/maproom/src/db/queries.rs` - Query optimization
- `crates/maproom/src/indexer/parallel.rs` - **NEW** - Parallel processing
- `crates/maproom/src/cache/mod.rs` - **NEW** - Caching layer
- `crates/maproom/benches/multi_language_bench.rs` - **NEW** - Comprehensive benchmarks
- `crates/maproom/tests/integration/mixed_language_test.rs` - **NEW** - Mixed-language tests
- `crates/maproom/tests/integration/kubernetes_test.rs` - **NEW** - Kubernetes test case
- `crates/maproom/docs/multi_language_support.md` - **NEW** - Multi-language documentation
- `crates/maproom/docs/performance_tuning.md` - **NEW** - Performance guide
- `crates/maproom/migrations/` - New migrations for cross-language support

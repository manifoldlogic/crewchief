# Ticket: LANG_PARSE-2004: Rust Integration and Cross-Language Support

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- parser-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement comprehensive Rust language support in Maproom's parser system, including FFI detection for cross-language references, Cargo dependency tracking, performance optimization through parallel parsing, and incremental updates. This enables indexing of Rust projects with full semantic understanding of unsafe code blocks, FFI boundaries, and workspace dependencies.

## Background
Phase 2, Week 4 of the LANG_PARSE project focuses on completing Rust integration. While basic Rust parsing has been established in earlier phases, this ticket addresses advanced features critical for real-world Rust projects:

1. **Cross-language references**: Rust codebases often expose FFI (Foreign Function Interface) interfaces to C/C++ or are called from other languages. Detecting `extern` blocks and `unsafe` code boundaries is essential for understanding system integration points.

2. **Cargo dependency tracking**: Understanding project structure through Cargo.toml and workspace configurations enables better context assembly and dependency graph construction.

3. **Performance requirements**: Rust projects can be large (tokio, serde, async-std are test targets), requiring optimized parsing that exceeds 200 files/min with constrained memory usage (<100MB).

4. **Incremental updates**: Supporting fast re-indexing when Rust files change is critical for developer experience in watch mode.

This work builds upon LANG_PARSE-2003 (complete Rust parser) and enables Maproom to handle production Rust codebases effectively.

## Acceptance Criteria
- [ ] Cargo workspace indexed successfully with all member crates detected
- [ ] Parse rate exceeds 200 files/min for large Rust projects
- [ ] FFI bindings (`extern` blocks) detected and indexed with proper metadata
- [ ] Memory usage remains under 100MB for large projects (tokio, serde scale)
- [ ] `unsafe` code blocks identified and marked in index
- [ ] Cargo.toml dependencies parsed and stored in database
- [ ] Incremental updates work correctly for modified .rs files
- [ ] Integration tests pass with tokio, async-std, and serde projects
- [ ] Benchmark suite demonstrates performance targets met

## Technical Requirements

### FFI Detection
- Parse `extern "C"` and `extern "Rust"` blocks
- Identify function signatures exposed across language boundaries
- Track `#[no_mangle]` and `#[export_name]` attributes
- Mark unsafe blocks containing FFI calls
- Store FFI metadata in `symbols` table with appropriate tags

### Cargo Integration
- Parse Cargo.toml and Cargo.lock files
- Extract workspace members and dependency graph
- Store package metadata (name, version, features)
- Link source files to their owning crate/package
- Support workspace-relative path resolution

### Performance Optimization
- Implement parallel parsing using rayon for concurrent file processing
- Optimize tree-sitter query patterns for Rust grammar
- Use incremental parsing where possible (tree-sitter's built-in support)
- Profile and optimize hot paths in parser loop
- Implement streaming processing for large files

### Incremental Updates
- Track file hashes in database for change detection
- Only re-parse modified .rs files on update
- Invalidate dependent symbols when definitions change
- Update workspace metadata only when Cargo.toml changes
- Maintain referential integrity during partial updates

### Testing Requirements
- Integration tests with real-world projects:
  - tokio (async runtime, ~100k LOC)
  - serde (serialization, proc macros)
  - async-std (alternative async runtime)
- Benchmark suite measuring:
  - Files processed per minute
  - Peak memory usage during indexing
  - Incremental update latency
- Unit tests for FFI detection edge cases
- Unit tests for Cargo.toml parsing with various configurations

## Implementation Notes

### Architecture Components

**New Module: `crates/maproom/src/parser/rust/ffi.rs`**
```rust
// FFI detection logic
pub struct FfiAnalyzer {
    // Detect extern blocks, unsafe boundaries
}

pub struct FfiBinding {
    pub name: String,
    pub signature: String,
    pub abi: String, // "C", "Rust", "system", etc.
    pub is_unsafe: bool,
    pub location: SourceLocation,
}
```

**New Module: `crates/maproom/src/parser/rust/cargo.rs`**
```rust
// Cargo.toml parsing and workspace analysis
pub struct CargoWorkspace {
    pub root: PathBuf,
    pub members: Vec<CrateMember>,
    pub dependencies: DependencyGraph,
}

pub fn parse_cargo_toml(path: &Path) -> Result<CargoManifest>;
pub fn resolve_workspace(root: &Path) -> Result<CargoWorkspace>;
```

**Updated: `crates/maproom/src/parser/rust/mod.rs`**
- Integrate FFI analyzer into main parser pipeline
- Add parallel processing using `rayon::par_iter()`
- Implement incremental update logic
- Add performance instrumentation

**Benchmark Suite: `crates/maproom/benches/rust_parser_bench.rs`**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_tokio_indexing(c: &mut Criterion) {
    c.bench_function("index_tokio_project", |b| {
        b.iter(|| {
            // Index tokio source
        });
    });
}

criterion_group!(benches, bench_tokio_indexing);
criterion_main!(benches);
```

**Integration Tests: `crates/maproom/tests/integration/rust_projects_test.rs`**
- Clone test projects (tokio, serde, async-std) using git submodules or on-demand
- Verify indexing completes successfully
- Assert on expected symbol counts, FFI detections
- Measure performance against acceptance criteria

### Database Schema Updates

**Extend `symbols` table metadata:**
```sql
-- Add columns to track Rust-specific info
ALTER TABLE symbols ADD COLUMN is_unsafe BOOLEAN DEFAULT FALSE;
ALTER TABLE symbols ADD COLUMN ffi_abi TEXT; -- NULL for non-FFI, 'C'/'Rust'/etc for FFI
ALTER TABLE symbols ADD COLUMN crate_name TEXT; -- Cargo package name
```

**New table for dependencies:**
```sql
CREATE TABLE cargo_dependencies (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER REFERENCES repositories(id),
    crate_name TEXT NOT NULL,
    version TEXT,
    dependency_name TEXT NOT NULL,
    dependency_version TEXT,
    features TEXT[], -- enabled features
    is_dev BOOLEAN DEFAULT FALSE,
    is_build BOOLEAN DEFAULT FALSE
);
```

### Performance Optimization Strategy

1. **Parallel file processing**: Use `rayon` to process files concurrently
2. **Batch database inserts**: Accumulate symbols and insert in transactions
3. **Lazy Cargo.toml parsing**: Only parse when explicitly needed
4. **Query optimization**: Profile tree-sitter queries, minimize redundant captures
5. **Memory pooling**: Reuse parser instances across files

### Testing Strategy

**Unit Tests (per-module)**
- `ffi.rs`: Test extern block detection, unsafe analysis
- `cargo.rs`: Test Cargo.toml parsing with various configurations (workspaces, features, path deps)
- Main parser: Test incremental update logic, parallel processing

**Integration Tests**
- Full project indexing with real Rust codebases
- Verify symbol counts, relationships, FFI detections
- Test workspace support with multi-crate projects

**Benchmarks**
- Measure files/min throughput on large projects
- Track memory usage over time during indexing
- Compare incremental vs full re-index performance

**Manual Testing**
- Run against CrewChief's own Rust code (`crates/maproom`)
- Verify FFI detection for any C bindings
- Check Cargo workspace handling

## Dependencies
- **LANG_PARSE-2003** (complete Rust parser) - REQUIRED
  - Must be completed first, as this ticket builds on basic Rust parsing infrastructure
  - Assumes tree-sitter-rust integration is working
  - Assumes basic symbol extraction (functions, structs, traits) is functional

## Risk Assessment

- **Risk**: Performance targets (>200 files/min) may be difficult to achieve with complex projects
  - **Mitigation**: Profile early and often. Focus on parallel processing from the start. If needed, implement tiered indexing (fast pass for basic symbols, detailed pass for FFI/relationships). Consider query optimization in tree-sitter patterns.

- **Risk**: FFI detection may have edge cases (inline assembly, complex macros, proc macros)
  - **Mitigation**: Start with common patterns (extern blocks, no_mangle attributes). Document known limitations. Add test cases as edge cases are discovered. Proc macro expansion is out of scope for Phase 2.

- **Risk**: Large projects (tokio) may exceed 100MB memory constraint
  - **Mitigation**: Implement streaming processing, process files in batches. Use memory profiling tools (valgrind, heaptrack). Consider incremental processing with file-level boundaries to bound memory usage.

- **Risk**: Cargo workspace resolution may be complex (path dependencies, workspace inheritance)
  - **Mitigation**: Leverage existing Cargo metadata command (`cargo metadata --format-version=1`) instead of parsing manually. This provides authoritative workspace structure.

- **Risk**: Incremental updates may break referential integrity if not careful
  - **Mitigation**: Use database transactions. Test incremental updates thoroughly with integration tests. Consider maintaining a "version" counter per file to detect stale data.

## Files/Packages Affected

### New Files
- `crates/maproom/src/parser/rust/ffi.rs` - FFI detection logic
- `crates/maproom/src/parser/rust/cargo.rs` - Cargo.toml parsing and workspace resolution
- `crates/maproom/benches/rust_parser_bench.rs` - Performance benchmark suite
- `crates/maproom/tests/integration/rust_projects_test.rs` - Integration tests with real projects

### Modified Files
- `crates/maproom/src/parser/rust/mod.rs` - Main Rust parser integration
- `crates/maproom/src/parser/pipeline.rs` - Add parallel processing support
- `crates/maproom/src/db/schema.rs` - Database schema updates for Rust metadata
- `crates/maproom/migrations/` - New migration for Rust-specific columns and tables
- `crates/maproom/Cargo.toml` - Add dependencies: rayon, cargo_metadata crate

### Test Data
- Test projects (git submodules or downloaded):
  - tokio (~100k LOC)
  - serde (serialization framework)
  - async-std (async runtime)

### Documentation
- `crates/maproom/README.md` - Update with Rust support details
- `docs/rust-parsing.md` - New doc explaining FFI detection and Cargo integration (if applicable)

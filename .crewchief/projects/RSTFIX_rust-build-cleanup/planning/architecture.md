# Architecture: Rust Build Cleanup

## Approach

This is a cleanup project, not a feature project. The architecture is already established. The goal is surgical removal of dead code and fixing broken tests without changing behavior.

## Design Principles

### 1. Preserve Behavior
- Only remove code that is provably unused (compiler-verified)
- Don't refactor or restructure - only delete dead code
- Run full test suite after each change

### 2. Systematic Processing
- Process warnings by category (easier to batch)
- Fix unused imports first (safest)
- Then unused variables (may need `_` prefix if truly unused)
- Finally dead code (functions/structs that may need actual removal)

### 3. Dead Code Decision Tree

```
Is the code called anywhere?
├── Yes → Keep, investigate why warning exists
└── No → Is it intended for future use?
    ├── Yes → Add #[allow(dead_code)] with comment
    └── No → Remove entirely
```

## Modules Affected

### Low Risk (imports only)
- `ab_testing/logger.rs` - 1 unused import
- `context/cache.rs` - 2 unused imports
- `context/detectors/*.rs` - unused Context imports
- `context/graph.rs` - 1 unused import

### Medium Risk (unused variables)
- `context/relationships.rs` - unused EdgeType
- `search/fts.rs`, `search/vector.rs`, `search/graph.rs`, `search/signals.rs` - unused variables from SQLIMPL migration

### Higher Risk (dead code removal)
- Functions: `compute_edges`, `find_test_targets`, `insert_edges`, `is_route_chunk`, `is_test_chunk`
- Methods: `as_str`, `create_context_item`, `evict_lru_if_needed`
- Struct: `Edge`

## Test Fix Strategy

The failing test `test_invalid_config_rejected`:
1. Check if YAML deserialization accepts negative f32 values
2. Validation happens at `FusionWeights::validate()` which checks for negatives
3. Issue may be that `load_from_file` doesn't call validate, or error handling differs

## Technology Constraints

- Must compile on all target platforms (darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64)
- No changes to public API
- No changes to Cargo.toml dependencies
- Vendor code (`sqlite-vec`) is untouchable

## Verification

After each batch of changes:
```bash
cargo build --bin crewchief-maproom 2>&1 | grep "warning:" | grep -v "sqlite-vec" | wc -l
cargo test -p crewchief-maproom
```

Final verification:
```bash
cargo clippy -p crewchief-maproom 2>&1 | grep -v "sqlite-vec"
```

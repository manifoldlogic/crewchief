# Quality Strategy: edge extraction

## Testing Philosophy

**Confidence over Coverage:** Test critical paths that ensure edges are extracted correctly and performance is acceptable. Avoid testing trivial code or tree-sitter internals. Focus on integration points and accuracy validation.

**MVP Pragmatism:** Same-file accuracy is more important than cross-file. Ship with ≥70% overall accuracy, iterate to improve.

**Synthetic Test Repos:** Create small, focused test repositories with known call graphs to validate extraction accuracy.

## Test Types

### Unit Tests

**Scope:**
- TypeScript call expression extraction (`typescript.rs`)
- Symbol resolution (same-file lookup)
- Edge struct creation
- Helper functions (find_enclosing_chunk, extract_function_identifier)

**Tools:**
- Rust `#[test]` framework
- Tree-sitter test fixtures

**Coverage Target:**
- Critical paths: 100% (call extraction, symbol resolution)
- Helper functions: 80%
- Error handling: Happy path + common errors

**Example Tests:**
```rust
#[test]
fn test_extract_simple_call() {
    let source = r#"
        function foo() {}
        function bar() { foo(); }
    "#;
    let chunks = vec![
        ChunkWithId { id: 1, symbol_name: Some("foo"), ... },
        ChunkWithId { id: 2, symbol_name: Some("bar"), ... },
    ];
    let edges = extract_calls(source, &chunks).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].src_chunk_id, 2); // bar calls foo
    assert_eq!(edges[0].dst_chunk_id, 1);
}

#[test]
fn test_method_call_extraction() {
    // Test obj.method() pattern
}

#[test]
fn test_unresolved_call_skipped() {
    // Call to symbol not in chunks (cross-file) should be skipped in Phase 1
}

#[test]
fn test_parse_error_returns_empty() {
    let invalid_source = "function foo(";
    let result = extract_calls(invalid_source, &[]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}
```

### Integration Tests

**Scope:**
- End-to-end: scan repository → edges inserted into database
- Incremental updates: modify file → edges recomputed
- Language support: TypeScript, JavaScript, TSX, JSX
- Edge types: calls edges only (Phase 1)

**Approach:**
Create synthetic test repositories:
```
test_repos/typescript_calls/
├── src/
│   ├── utils.ts       # function add(a, b)
│   ├── math.ts        # function multiply(a, b) { return add(a, a); }
│   └── main.ts        # function main() { multiply(2, 3); }
```

**Test:**
```rust
#[tokio::test]
async fn test_scan_creates_edges() {
    let store = setup_test_db().await;
    scan_worktree(&store, "test", "main", Path::new("test_repos/typescript_calls"), "HEAD").await?;

    let edge_count = store.run(|conn| {
        conn.query_row("SELECT COUNT(*) FROM chunk_edges", [], |row| row.get(0))
    }).await?;

    assert!(edge_count >= 2, "Expected at least 2 edges (multiply→add, main→multiply)");

    // Validate specific edge
    let edges = store.get_edges_for_chunk(math_chunk_id).await?;
    assert!(edges.iter().any(|e| e.dst_chunk_id == utils_chunk_id));
}

#[tokio::test]
async fn test_incremental_update_recomputes_edges() {
    // 1. Initial scan
    // 2. Modify file (add new call)
    // 3. Trigger update
    // 4. Verify new edge created, old edges preserved
}
```

### Performance Tests

**Scope:**
- Measure scan time overhead
- Verify no memory leaks
- Validate batch insertion performance

**Approach:**
```rust
#[bench]
fn bench_edge_extraction_typical_file(b: &mut Bencher) {
    let source = load_fixture("typical_200_line_file.ts");
    let chunks = extract_chunks(&source, "ts");
    b.iter(|| {
        let edges = extract_edges(&source, "ts", &chunks);
        assert!(edges.is_ok());
    });
}

#[tokio::test]
async fn test_scan_performance_overhead() {
    let baseline_time = measure_scan_without_edges();
    let with_edges_time = measure_scan_with_edges();
    let overhead = (with_edges_time - baseline_time) / baseline_time;
    assert!(overhead < 0.3, "Overhead should be <30%, got {:.1}%", overhead * 100.0);
}
```

## Critical Paths

The following paths MUST be tested:

1. **Edge Extraction Pipeline**
   - scan_worktree() → extract_edges() → insert_edges() → chunk_edges table populated
   - Verify edges created for TypeScript/JavaScript files

2. **Incremental Updates**
   - File modification → EdgeUpdater::update_edges() → old edges deleted, new edges inserted
   - Verify no stale edges remain

3. **Symbol Resolution**
   - Same-file calls resolved correctly (≥85% accuracy)
   - Unresolved calls skipped gracefully (no errors)

4. **Error Handling**
   - Parse failures → log warning, continue scan
   - Database errors → fail scan with clear error
   - Invalid chunk IDs → skip edge creation

5. **Performance Budget**
   - Scan time overhead <30%
   - No memory leaks (run scan on large repo, monitor RSS)
   - Batch insertion <5ms for 30 edges

6. **Edge Type Coverage**
   - Calls edges created (src_chunk_id, dst_chunk_id, type="calls")
   - No other edge types in Phase 1

## Test Data Strategy

**Synthetic Repositories:**
Create minimal test repos with known call graphs:
- `typescript_calls/` - Simple call chains
- `typescript_methods/` - Object methods and class methods
- `typescript_complex/` - Nested calls, callbacks, higher-order functions
- `javascript_calls/` - ES6 modules and function calls

**Real Repository Sample:**
Use a subset of crewchief codebase (e.g., `packages/cli/src/`) to validate real-world accuracy.

**Fixtures:**
Store test files in `crates/maproom/tests/fixtures/edge_extraction/`

## Quality Gates

Before verification:
- [ ] All unit tests pass (`cargo test`)
- [ ] Integration tests pass (synthetic repos)
- [ ] Performance tests pass (<30% overhead)
- [ ] No linting errors (`cargo clippy`)
- [ ] No format errors (`cargo fmt --check`)
- [ ] Accuracy validation ≥70% on sample repository

## Accuracy Validation

**Manual Evaluation:**
1. Scan crewchief repository
2. Query edges for 20 random functions
3. Manually verify call graph is correct
4. Calculate precision/recall

**Acceptance:**
- Precision ≥70% (edges created are correct)
- Recall ≥60% (most actual calls captured)
- Same-file precision ≥85%

## Monitoring

**Metrics to track:**
- Edge count per repository
- Edges per file (distribution)
- Unresolved calls (trace logs)
- Parse failures (warning logs)
- Scan time overhead (before/after comparison)

**Logging:**
```rust
info!("Edge extraction: {} edges created for {} files", total_edges, files_processed);
debug!("File {}: {} calls, {} resolved, {} unresolved", file_path, total_calls, resolved, unresolved);
```

## Post-Deployment Validation

After Phase 1 deployment:
1. Rescan test repository
2. Query edge count (expect ≥10,000 edges)
3. Spot-check accuracy on known call chains
4. Monitor performance (scan time should not double)
5. Verify SRCHREL can use edges for quality scoring
